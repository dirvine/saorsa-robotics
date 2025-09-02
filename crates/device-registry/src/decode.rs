use crate::{DeviceDescriptor, TelemetryMap};
use can_transport::CanId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TelemetryValue {
    F64(f64),
    I64(i64),
    Bool(bool),
    Text(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryRecord {
    pub id: String,
    pub fmt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ts: Option<String>,
    pub fields: HashMap<String, TelemetryValue>,
}

pub fn decode_by_id(
    desc: &DeviceDescriptor,
    id: CanId,
    data: &[u8],
    ts: Option<OffsetDateTime>,
) -> Option<TelemetryRecord> {
    let telem = match find_telem(desc, id) {
        Some(t) => t,
        None => return None,
    };
    decode_fmt(desc, &telem.fmt, data, ts)
}

fn find_telem<'a>(desc: &'a DeviceDescriptor, id: CanId) -> Option<&'a TelemetryMap> {
    for t in &desc.telemetry {
        if let Some(tid) = parse_id(&t.id) {
            if tid == id {
                return Some(t);
            }
        }
    }
    None
}

pub fn decode_fmt(
    desc: &DeviceDescriptor,
    fmt: &str,
    data: &[u8],
    ts: Option<OffsetDateTime>,
) -> Option<TelemetryRecord> {
    match fmt {
        // T-Motor AK feedback: p(16) v(12) t(12)
        "tmotor_state" => decode_tmotor_state(desc, data, ts),
        // ODrive: pos f32 LE, vel f32 LE
        "odrive_get_state" => decode_odrive_state(data, ts),
        _ => None,
    }
}

fn decode_tmotor_state(
    desc: &DeviceDescriptor,
    data: &[u8],
    ts: Option<OffsetDateTime>,
) -> Option<TelemetryRecord> {
    if data.len() < 8 {
        return None;
    }
    let p_u16 = u16::from(data[0]) << 8 | u16::from(data[1]);
    let v_u12 = (u16::from(data[2]) << 4) | (u16::from(data[3]) >> 4);
    let t_u12 = (u16::from(data[6] & 0x0F) << 8) | u16::from(data[7]);

    let (p_min, p_max) = super::encode::joint_pos_range_rad(&Default::default(), desc);
    let (v_min, v_max) = super::encode::joint_vel_range_rad(&Default::default(), desc);
    let (t_min, t_max) = super::encode::joint_torque_range(&Default::default(), desc);

    let p = unmap_from_uint(p_u16 as u32, p_min, p_max, 16) as f64;
    let v = unmap_from_uint(v_u12 as u32, v_min, v_max, 12) as f64;
    let t = unmap_from_uint(t_u12 as u32, t_min, t_max, 12) as f64;

    let mut fields = HashMap::new();
    fields.insert("pos_rad".into(), TelemetryValue::F64(p));
    fields.insert("vel_rad_s".into(), TelemetryValue::F64(v));
    fields.insert("torque_nm".into(), TelemetryValue::F64(t));

    Some(TelemetryRecord {
        id: "tmotor_state".into(),
        fmt: "tmotor_state".into(),
        ts: ts.and_then(|t| {
            t.format(&time::format_description::well_known::Rfc3339)
                .ok()
        }),
        fields,
    })
}

fn decode_odrive_state(data: &[u8], ts: Option<OffsetDateTime>) -> Option<TelemetryRecord> {
    if data.len() < 8 {
        return None;
    }
    let mut b0 = [0u8; 4];
    let mut b1 = [0u8; 4];
    b0.copy_from_slice(&data[0..4]);
    b1.copy_from_slice(&data[4..8]);
    let pos = f32::from_le_bytes(b0) as f64;
    let vel = f32::from_le_bytes(b1) as f64;

    let mut fields = HashMap::new();
    fields.insert("pos".into(), TelemetryValue::F64(pos));
    fields.insert("vel".into(), TelemetryValue::F64(vel));

    Some(TelemetryRecord {
        id: "odrive_get_state".into(),
        fmt: "odrive_get_state".into(),
        ts: ts.and_then(|t| {
            t.format(&time::format_description::well_known::Rfc3339)
                .ok()
        }),
        fields,
    })
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

fn unmap_from_uint(u: u32, min: f32, max: f32, nbits: u32) -> f32 {
    if !(min < max) {
        return min;
    }
    let hi = ((1u64 << nbits) - 1) as f32;
    let y = (u as f32) / hi;
    min + y * (max - min)
}
