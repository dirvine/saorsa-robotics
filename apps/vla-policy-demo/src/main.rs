//! VLA Policy Demo for Saorsa Robotics
//!
//! Demonstrates a depth-aware grasp pipeline (RGB(+D) → grasp pose →
//! policy actions → safety guard → optional CAN send), plus basic skills.

use can_transport::{CanBus, MockBus};
use clap::Parser;
use device_registry::{
    build_frames_for_joint, load_descriptors_dir, DeviceDescriptor, JointCommand,
};
use safety_guard::{check_action_safety, create_default_constraint_engine, SafetyStatus};
use vla_policy::skills::{PickSkill, PlaceSkill, ReachSkill};
use vla_policy::{create_policy, ActionType, Observation, PolicyConfig, Skill, SkillContext};

#[derive(Parser, Debug)]
#[command(name = "vla-policy-demo", about = "Depth pick demo with mock policy")]
struct Args {
    /// ROI as x,y,w,h in pixels (optional)
    #[arg(long)]
    roi: Option<String>,
    /// Camera intrinsics as fx,fy,cx,cy (optional; defaults to 600,600,112,112 for 224x224)
    #[arg(long)]
    intr: Option<String>,
    /// AprilTag demo: path to image (grayscale or color)
    #[arg(long)]
    tag_image: Option<String>,
    /// AprilTag size in meters (default 0.05)
    #[arg(long, default_value_t = 0.05)]
    tag_size_m: f64,
    /// Transform output grasp to base frame using camera_T_base if provided
    #[arg(long, default_value_t = false)]
    to_base: bool,
    /// camera_T_base as 16 comma-separated numbers (row-major)
    #[arg(long)]
    cam_t_base: Option<String>,
    /// Device descriptors directory (for send demonstration)
    #[arg(long, default_value = "configs/devices")]
    desc_dir: String,
    /// Actually send frames on mock CAN bus (mock0) if true; otherwise just print
    #[arg(long, default_value_t = false)]
    send: bool,
}

fn parse_roi(s: &str) -> Option<(i32, i32, i32, i32)> {
    let parts: Vec<_> = s.split(',').collect();
    if parts.len() != 4 {
        return None;
    }
    let px: Result<Vec<i32>, _> = parts.iter().map(|p| p.trim().parse::<i32>()).collect();
    px.ok().and_then(|v| {
        v.get(0)
            .zip(v.get(1))
            .zip(v.get(2))
            .zip(v.get(3))
            .map(|(((x, y), w), h)| (*x, *y, *w, *h))
    })
}

fn parse_intr(s: &str) -> Option<(f64, f64, f64, f64)> {
    let parts: Vec<_> = s.split(',').collect();
    if parts.len() != 4 {
        return None;
    }
    let pf: Result<Vec<f64>, _> = parts.iter().map(|p| p.trim().parse::<f64>()).collect();
    pf.ok().and_then(|v| {
        v.get(0)
            .zip(v.get(1))
            .zip(v.get(2))
            .zip(v.get(3))
            .map(|(((fx, fy), cx), cy)| (*fx, *fy, *cx, *cy))
    })
}

fn parse_mat4_row_major(s: &str) -> Option<[f32; 16]> {
    let vals: Result<Vec<f32>, _> = s.split(',').map(|p| p.trim().parse::<f32>()).collect();
    match vals {
        Ok(v) if v.len() == 16 => {
            let mut out = [0.0f32; 16];
            for (i, x) in v.into_iter().enumerate() {
                out[i] = x;
            }
            Some(out)
        }
        _ => None,
    }
}

fn joint_name_for_index<'a>(desc: &'a DeviceDescriptor, idx: usize) -> &'a str {
    if let Some(j) = desc.joints.get(idx) {
        return j.name.as_str();
    }
    if let Some(j) = desc.joints.first() {
        return j.name.as_str();
    }
    // Fallback for demos if descriptor has no joints
    "axis0"
}

struct DefaultKinematics<'a> {
    desc: Option<&'a DeviceDescriptor>,
}

impl<'a> DefaultKinematics<'a> {
    fn new(desc: Option<&'a DeviceDescriptor>) -> Self {
        Self { desc }
    }

    fn clamp_pos(&self, idx: usize, val: f32) -> f32 {
        if let Some(desc) = self.desc {
            if let Some(j) = desc.joints.get(idx) {
                if let Some((lo, hi)) = j.limits.pos_deg {
                    let lo_r = (lo as f32).to_radians();
                    let hi_r = (hi as f32).to_radians();
                    return val.max(lo_r.min(hi_r)).min(lo_r.max(hi_r));
                }
            }
        }
        val.max(-3.14).min(3.14)
    }

    fn ee_delta_to_joint_positions(
        &self,
        curr: &[f32],
        _ee_pose: &[f32],
        delta: &[f32],
    ) -> Vec<f32> {
        let gains = [1.0f32, 1.0, 1.0, 0.5, 0.5, 0.5];
        let mut out = curr.to_vec();
        for i in 0..out.len().min(6) {
            let step = if let Some(d) = delta.get(i) { *d } else { 0.0 };
            out[i] = self.clamp_pos(i, out[i] + gains[i] * step);
        }
        out
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🤖 Saorsa Robotics VLA Policy Demo");
    println!("====================================");
    let args = Args::parse();

    // Initialize the VLA policy system
    vla_policy::init()?;

    // Create a mock policy configuration
    let config = PolicyConfig {
        model_type: "mock".to_string(),
        model_path: "mock".to_string(),
        action_heads: vec![
            vla_policy::ActionHead {
                name: "ee_delta".to_string(),
                action_type: ActionType::EndEffectorDelta,
                dimensions: 6,
                bounds: None,
            },
            vla_policy::ActionHead {
                name: "gripper".to_string(),
                action_type: ActionType::Gripper,
                dimensions: 1,
                bounds: Some(vec![(0.0, 1.0)]),
            },
            vla_policy::ActionHead {
                name: "joint_positions".to_string(),
                action_type: ActionType::JointPositions,
                dimensions: 6,
                bounds: None,
            },
        ],
        image_size: (224, 224),
        normalization: vla_policy::NormalizationConfig {
            image_mean: vec![0.485, 0.456, 0.406],
            image_std: vec![0.229, 0.224, 0.225],
            joint_mean: None,
            joint_std: None,
        },
        device: vla_policy::DeviceConfig {
            device_type: "cpu".to_string(),
            device_id: None,
        },
        metadata: std::collections::HashMap::new(),
    };

    // Create the policy
    let policy = create_policy(config)?;
    // Note: Policy is already initialized during creation
    // policy.initialize(vla_policy::PolicyConfig {
    //     model_type: "mock".to_string(),
    //     model_path: "mock".to_string(),
    //     action_heads: vec![],
    //     image_size: (224, 224),
    //     normalization: vla_policy::NormalizationConfig {
    //         image_mean: vec![0.485, 0.456, 0.406],
    //         image_std: vec![0.229, 0.224, 0.225],
    //         joint_mean: None,
    //         joint_std: None,
    //     },
    //     device: vla_policy::DeviceConfig {
    //         device_type: "cpu".to_string(),
    //         device_id: None,
    //     },
    // }).await?;

    // Build a synthetic observation with optional ROI/depth for demo
    let observation = Observation {
        image: vec![128; 224 * 224 * 3], // Gray image for demo
        image_shape: (224, 224, 3),
        depth_u16: Some(vec![800u16; 224 * 224]), // 0.8m plane
        depth_shape: Some((224, 224)),
        dof_mask: None,
        dataset_name: None,
        joint_positions: vec![0.0, 0.5, 0.0, 1.0, 0.0, 0.0],
        joint_velocities: vec![0.0; 6],
        ee_pose: Some(vec![0.3, 0.0, 0.2, 0.0, 0.0, 0.0]), // Current end-effector pose
        camera_t_base: None,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs_f64(),
    };

    // If we have ROI + intr, compute a grasp pose
    if let (Some(roi_str), true) = (&args.roi, observation.depth_u16.is_some()) {
        let roi = parse_roi(roi_str).unwrap_or((92, 92, 40, 40));
        let (fx, fy, cx, cy) = args
            .intr
            .map(|s| parse_intr(&s).unwrap_or((600.0, 600.0, 112.0, 112.0)))
            .unwrap_or((600.0, 600.0, 112.0, 112.0));
        let intr = vision_stereo::tags::CameraIntrinsics { fx, fy, cx, cy };
        if let Some(depth) = observation.depth_u16.as_ref() {
            let (h, w) = observation.depth_shape.unwrap_or((224, 224));
            match vision_stereo::grasp::grasp_from_roi(
                depth,
                w as usize,
                h as usize,
                &intr,
                roi,
                args.to_base,
                args.cam_t_base.as_deref().and_then(parse_mat4_row_major),
            ) {
                Ok(p) => {
                    println!(
                        "📐 Grasp (camera frame): t=({:.3},{:.3},{:.3})",
                        p.t[0], p.t[1], p.t[2]
                    );
                }
                Err(e) => {
                    println!("⚠️  ROI grasp estimation failed: {e}");
                }
            }
        }
    }

    // AprilTag path: requires opencv+apriltag features at compile time
    if let Some(tag_path) = args.tag_image.as_deref() {
        #[cfg(all(feature = "vision-opencv", feature = "apriltag"))]
        {
            let (fx, fy, cx, cy) = args
                .intr
                .as_deref()
                .and_then(parse_intr)
                .unwrap_or((600.0, 600.0, 112.0, 112.0));
            let intr = vision_stereo::tags::CameraIntrinsics { fx, fy, cx, cy };
            match vision_stereo::io::read_gray8(tag_path) {
                Ok((gray, w, h)) => {
                    match vision_stereo::tags::estimate_tag_pose_from_image(
                        &gray,
                        w,
                        h,
                        &intr,
                        args.tag_size_m,
                        None,
                    ) {
                        Ok(poses) if !poses.is_empty() => {
                            let p0 = &poses[0];
                            match vision_stereo::grasp::grasp_from_tag(
                                p0.r,
                                p0.t,
                                0.10,
                                args.to_base,
                                args.cam_t_base.as_deref().and_then(parse_mat4_row_major),
                            ) {
                                Ok(g) => {
                                    println!(
                                        "🏷️  AprilTag grasp t=({:.3},{:.3},{:.3})",
                                        g.t[0], g.t[1], g.t[2]
                                    );
                                }
                                Err(e) => println!("⚠️  Tag grasp failed: {e}"),
                            }
                        }
                        Ok(_) => println!("ℹ️  No AprilTags detected."),
                        Err(e) => println!("⚠️  Tag pose estimation failed: {e}"),
                    }
                }
                Err(e) => println!("⚠️  Failed to read tag image: {e}"),
            }
        }
        #[cfg(not(all(feature = "vision-opencv", feature = "apriltag")))]
        {
            let _ = tag_path; // silence unused warning when features are not enabled
            println!(
                "AprilTag mode requires: --features vla-policy-demo/vision-opencv,vla-policy-demo/apriltag"
            );
        }
    }

    // Get policy prediction (EndEffectorDelta + Gripper, and maybe JointPositions)
    let result = policy.predict(&observation).await?;
    println!("📊 Policy Prediction:");
    println!("   Actions: {}", result.actions.len());
    for (i, action) in result.actions.iter().enumerate() {
        println!(
            "   Action {}: {:?} with confidence {:.2}",
            i, action.action_type, action.confidence
        );
        if action.values.len() <= 6 {
            println!("     Values: {:?}", action.values);
        } else {
            println!("     Values: [{} values]", action.values.len());
        }
    }
    println!("   Inference time: {:.2}ms", result.inference_time_ms);

    // Run actions through safety-guard and (optionally) send via CAN
    let mut engine = create_default_constraint_engine()?;
    let mut bus = if args.send {
        Some(MockBus::open("mock0")?)
    } else {
        None
    };
    let reg =
        load_descriptors_dir(&args.desc_dir).unwrap_or_else(|_| device_registry::DeviceRegistry {
            devices: std::collections::HashMap::new(),
        });
    let maybe_desc = reg.devices.values().next();
    let kin = DefaultKinematics::new(maybe_desc);
    for action in &result.actions {
        match check_action_safety(action, &observation, &mut engine)? {
            SafetyStatus::Safe => {
                println!("✅ Safe action: {:?}", action.action_type);
                match action.action_type {
                    ActionType::JointPositions => {
                        if let Some(desc) = maybe_desc {
                            for (i, v) in action.values.iter().enumerate() {
                                let joint_name = joint_name_for_index(desc, i);
                                if let Ok(frames) = build_frames_for_joint(
                                    desc,
                                    joint_name,
                                    JointCommand::Position(*v),
                                ) {
                                    println!(
                                        "🖨️  Joint {} → {:.3} rad ({} frame(s))",
                                        i,
                                        v,
                                        frames.len()
                                    );
                                    if let Some(b) = bus.as_mut() {
                                        for f in &frames {
                                            let _ = b.send(f);
                                        }
                                    }
                                }
                            }
                        } else {
                            println!("ℹ️  No device descriptors found; printing only.");
                            println!("   Target joints: {:?}", action.values);
                        }
                    }
                    ActionType::EndEffectorDelta => {
                        if let Some(ee_pose) = &observation.ee_pose {
                            let new_j = kin.ee_delta_to_joint_positions(
                                &observation.joint_positions,
                                ee_pose,
                                &action.values,
                            );
                            // Safety-check converted joint action
                            let jp_action = vla_policy::Action {
                                action_type: ActionType::JointPositions,
                                values: new_j.clone(),
                                confidence: action.confidence,
                                timestamp: action.timestamp,
                            };
                            match check_action_safety(&jp_action, &observation, &mut engine)? {
                                SafetyStatus::Safe => {
                                    if let Some(desc) = maybe_desc {
                                        for (i, v) in new_j.iter().enumerate() {
                                            let joint_name = joint_name_for_index(desc, i);
                                            if let Ok(frames) = build_frames_for_joint(
                                                desc,
                                                joint_name,
                                                JointCommand::Position(*v),
                                            ) {
                                                println!(
                                                    "🖨️  IK joint {} → {:.3} rad ({} frame(s))",
                                                    i,
                                                    v,
                                                    frames.len()
                                                );
                                                if let Some(b) = bus.as_mut() {
                                                    for f in &frames {
                                                        let _ = b.send(f);
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        println!("IK joints: {:?}", new_j);
                                    }
                                }
                                other => println!("🚫 IK safety blocked: {:?}", other),
                            }
                        }
                    }
                    _ => {}
                }
            }
            other => {
                println!(
                    "🚫 Safety gate blocked action: {:?} -> {:?}",
                    action.action_type, other
                );
            }
        }
    }

    // Demonstrate skills
    println!("\n🎯 Skill Demonstrations:");
    println!("========================");

    // Reach skill
    let reach_skill = ReachSkill::new();
    println!("📍 Reach Skill Metadata:");
    let metadata = reach_skill.metadata();
    println!("   Name: {}", metadata.name);
    println!("   Description: {}", metadata.description);
    println!("   Parameters: {}", metadata.parameters.len());

    // Create a skill context for reaching
    let reach_context = SkillContext {
        goal: "Move to target position".to_string(),
        parameters: [(
            "target_pose".to_string(),
            serde_json::json!([0.4, 0.1, 0.3, 0.0, 0.0, 0.0]),
        )]
        .into(),
        timeout_s: 10.0,
        max_retries: 5,
    };

    // Check if skill can execute
    let can_execute = reach_skill.can_execute(&reach_context).await?;
    println!("   Can execute: {}", can_execute);

    // Pick skill
    let pick_skill = PickSkill::new();
    println!("\n👐 Pick Skill Metadata:");
    let pick_metadata = pick_skill.metadata();
    println!("   Name: {}", pick_metadata.name);
    println!("   Description: {}", pick_metadata.description);

    // Place skill
    let place_skill = PlaceSkill::new();
    println!("\n📦 Place Skill Metadata:");
    let place_metadata = place_skill.metadata();
    println!("   Name: {}", place_metadata.name);
    println!("   Description: {}", place_metadata.description);

    println!("\n✅ VLA Policy Demo Complete!");
    println!("   - Policy system initialized");
    println!("   - Mock predictions working");
    println!("   - Skills framework ready");
    println!("   - Ready for OpenVLA integration");

    Ok(())
}
