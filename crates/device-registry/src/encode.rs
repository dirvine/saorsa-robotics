use crate::{DeviceDescriptor, JointMap};
use anyhow::Context;
use can_transport::{CanFrame, CanId};

#[derive(Clone, Copy, Debug)]
pub enum JointCommand {
    TorqueNm(f32),
    Position(f32),
    Velocity(f32),
}

pub fn build_frames_for_joint(
    desc: &DeviceDescriptor,
    joint_name: &str,
    cmd: JointCommand,
) -> anyhow::Result<Vec<CanFrame>> {
    let joint = desc
        .joints
        .iter()
        .find(|j| j.name == joint_name)
        .with_context(|| format!("joint not found: {joint_name}"))?;
    build_for_map(desc, joint, &joint.r#map, &cmd)
}

fn build_for_map(
    desc: &DeviceDescriptor,
    _joint: &crate::types::Joint,
    map: &JointMap,
    cmd: &JointCommand,
) -> anyhow::Result<Vec<CanFrame>> {
    let mut out = Vec::new();
    for frame in &map.frames {
        let id = parse_id(&frame.id).with_context(|| format!("invalid id: {}", frame.id))?;
        match frame.fmt.as_str() {
            // T-Motor AK series (MIT 8-byte packed command): p(16) v(12) kp(12) kd(12) t(12)
            "tmotor_cmd" => {
                // Ranges: prefer descriptor limits; fallback to conservative defaults
                let (p_min, p_max) = joint_pos_range_rad(map, desc);
                let (v_min, v_max) = joint_vel_range_rad(map, desc);
                let (t_min, t_max) = joint_torque_range(map, desc);

                let pd = map.pd.as_ref();
                let default_kp = pd.and_then(|g| g.kp).unwrap_or(0.0);
                let default_kd = pd.and_then(|g| g.kd).unwrap_or(0.0);
                let (p, v, kp, kd, t) = match *cmd {
                    JointCommand::TorqueNm(tq) => (0.0, 0.0, 0.0, 0.0, tq),
                    JointCommand::Position(pos) => (pos, 0.0, default_kp, default_kd, 0.0),
                    JointCommand::Velocity(vel) => (0.0, vel, default_kp, default_kd, 0.0),
                };

                let p_u16 = map_to_uint(p, p_min, p_max, 16) as u16;
                let v_u12 = map_to_uint(v, v_min, v_max, 12) as u16;
                let kp_u12 = map_to_uint(kp, 0.0, 500.0, 12) as u16; // gain ranges
                let kd_u12 = map_to_uint(kd, 0.0, 5.0, 12) as u16;
                let t_u12 = map_to_uint(t, t_min, t_max, 12) as u16;

                let mut data = [0u8; 8];
                // Pack bits: p[15:8], p[7:0], v[11:4], v[3:0]|kp[11:8], kp[7:0], kd[11:4], kd[3:0]|t[11:8], t[7:0]
                data[0] = (p_u16 >> 8) as u8;
                data[1] = (p_u16 & 0xFF) as u8;
                data[2] = (v_u12 >> 4) as u8;
                data[3] = ((v_u12 & 0x0F) as u8) << 4 | ((kp_u12 >> 8) as u8 & 0x0F);
                data[4] = (kp_u12 & 0xFF) as u8;
                data[5] = (kd_u12 >> 4) as u8;
                data[6] = ((kd_u12 & 0x0F) as u8) << 4 | ((t_u12 >> 8) as u8 & 0x0F);
                data[7] = (t_u12 & 0xFF) as u8;

                let cf = CanFrame {
                    id,
                    len: 8,
                    data,
                    rtr: false,
                    timestamp: None,
                };
                out.push(cf);
            }
            // ODrive Set Input Pos: 8 bytes: pos f32 LE + 4 bytes feed-forward zeros
            "odrive_set_pos" => {
                let mut data = [0u8; 8];
                let pos = match cmd {
                    JointCommand::Position(v) => *v,
                    _ => 0.0,
                };
                data[..4].copy_from_slice(&pos.to_le_bytes());
                let cf = CanFrame {
                    id,
                    len: 8,
                    data,
                    rtr: false,
                    timestamp: None,
                };
                out.push(cf);
            }
            other => {
                anyhow::bail!("unsupported frame fmt: {other}");
            }
        }
    }
    Ok(out)
}

fn parse_id(s: &str) -> Option<CanId> {
    let t = s.trim();
    if let Some(hex) = t.strip_prefix("0x") {
        let val = u32::from_str_radix(hex, 16).ok()?;
        if val <= 0x7FF {
            CanId::standard(val as u16)
        } else if val <= 0x1FFF_FFFF {
            CanId::extended(val)
        } else {
            None
        }
    } else {
        // decimal
        let val = t.parse::<u32>().ok()?;
        if val <= 0x7FF {
            CanId::standard(val as u16)
        } else if val <= 0x1FFF_FFFF {
            CanId::extended(val)
        } else {
            None
        }
    }
}

fn map_to_uint(x: f32, min: f32, max: f32, nbits: u32) -> u32 {
    let hi = ((1u64 << nbits) - 1) as f32;
    if !(min < max) {
        return 0;
    }
    let mut y = (x - min) as f32 / (max - min) as f32;
    if y.is_nan() {
        y = 0.0;
    }
    if y < 0.0 {
        y = 0.0;
    }
    if y > 1.0 {
        y = 1.0;
    }
    (y * hi).round() as u32
}

pub(crate) fn joint_pos_range_rad(_map: &JointMap, desc: &DeviceDescriptor) -> (f32, f32) {
    // Use joint limits if present for the first matching joint; conservative fallback otherwise
    // Note: In a more complete implementation, consider per-joint overrides.
    let mut out = None;
    for j in &desc.joints {
        if let Some((lo, hi)) = j.limits.pos_deg {
            let lo_r = (lo as f32).to_radians();
            let hi_r = (hi as f32).to_radians();
            out = Some((lo_r.min(hi_r), lo_r.max(hi_r)));
            break;
        }
    }
    out.unwrap_or((-12.5_f32.to_radians(), 12.5_f32.to_radians()))
}

pub(crate) fn joint_vel_range_rad(_map: &JointMap, desc: &DeviceDescriptor) -> (f32, f32) {
    let mut out = None;
    for j in &desc.joints {
        if let Some(vel) = j.limits.vel_dps {
            let v = (vel as f32).to_radians();
            out = Some((-v, v));
            break;
        }
    }
    out.unwrap_or((-45.0_f32.to_radians(), 45.0_f32.to_radians()))
}

pub(crate) fn joint_torque_range(_map: &JointMap, desc: &DeviceDescriptor) -> (f32, f32) {
    let mut out = None;
    for j in &desc.joints {
        if let Some(t) = j.limits.torque_nm {
            let t = t as f32;
            out = Some((-t, t));
            break;
        }
    }
    out.unwrap_or((-18.0, 18.0))
}
