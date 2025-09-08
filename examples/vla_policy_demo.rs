//! VLA Policy Demo for Saorsa Robotics
//!
//! This example demonstrates how to use the VLA policy system
//! with the mock policy and basic skills.

use vla_policy::{create_policy, ActionType, Observation, PolicyConfig, SkillContext};
use vla_policy::skills::{PickSkill, PlaceSkill, ReachSkill};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🤖 Saorsa Robotics VLA Policy Demo");
    println!("====================================");

    // Initialize the VLA policy system
    vla_policy::init()?;

    // Create a mock policy configuration
    let config = PolicyConfig {
        model_type: "mock".to_string(),
        model_path: "mock".to_string(),
        action_heads: vec![
            vla_policy::ActionHead {
                name: "joint_positions".to_string(),
                action_type: ActionType::JointPositions,
                dimensions: 6,
                bounds: Some(vec![
                    (-3.14, 3.14), // Joint 1
                    (-1.57, 1.57), // Joint 2
                    (-3.14, 3.14), // Joint 3
                    (-1.57, 1.57), // Joint 4
                    (-3.14, 3.14), // Joint 5
                    (-1.57, 1.57), // Joint 6
                ]),
            },
            vla_policy::ActionHead {
                name: "gripper".to_string(),
                action_type: ActionType::Gripper,
                dimensions: 1,
                bounds: Some(vec![(0.0, 1.0)]),
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

    // Create an observation
    let observation = Observation {
        image: vec![128; 224 * 224 * 3], // Gray image for demo
        image_shape: (224, 224, 3),
        depth_u16: None,
        depth_shape: None,
        dof_mask: None,
        dataset_name: None,
        depth_u16: None,
        depth_shape: None,
        joint_positions: vec![0.0, 0.5, 0.0, 1.0, 0.0, 0.0],
        joint_velocities: vec![0.0; 6],
        ee_pose: Some(vec![0.3, 0.0, 0.2, 0.0, 0.0, 0.0]), // Current end-effector pose
        camera_t_base: None,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs_f64(),
    };

    // Get policy prediction
    let result = policy.predict(&observation).await?;
    println!("📊 Policy Prediction:");
    println!("   Actions: {}", result.actions.len());
    for (i, action) in result.actions.iter().enumerate() {
        println!("   Action {}: {:?} with confidence {:.2}",
                i, action.action_type, action.confidence);
        if action.values.len() <= 6 {
            println!("     Values: {:?}", action.values);
        } else {
            println!("     Values: [{} values]", action.values.len());
        }
    }
    println!("   Inference time: {:.2}ms", result.inference_time_ms);

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
        parameters: [
            ("target_pose".to_string(),
             serde_json::json!([0.4, 0.1, 0.3, 0.0, 0.0, 0.0]))
        ].into(),
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
