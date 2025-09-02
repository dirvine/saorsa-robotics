use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Observation from the environment (vision + state)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    /// RGB image from camera (HWC format)
    pub image: Vec<u8>,
    /// Image dimensions (height, width, channels)
    pub image_shape: (u32, u32, u32),
    /// Joint positions (radians)
    pub joint_positions: Vec<f32>,
    /// Joint velocities (rad/s)
    pub joint_velocities: Vec<f32>,
    /// End-effector pose (x, y, z, rx, ry, rz)
    pub ee_pose: Option<Vec<f32>>,
    /// Timestamp
    pub timestamp: f64,
}

impl Default for Observation {
    fn default() -> Self {
        Self {
            image: vec![0; 640 * 480 * 3], // Default VGA RGB image
            image_shape: (480, 640, 3),
            joint_positions: vec![0.0; 6], // Default 6-DOF
            joint_velocities: vec![0.0; 6],
            ee_pose: None,
            timestamp: 0.0,
        }
    }
}

/// Action output from policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    /// Action type (joint, ee_delta, gripper)
    pub action_type: ActionType,
    /// Action values
    pub values: Vec<f32>,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Timestamp
    pub timestamp: f64,
}

impl Default for Action {
    fn default() -> Self {
        Self {
            action_type: ActionType::JointPositions,
            values: vec![0.0; 6], // Default 6-DOF
            confidence: 0.0,
            timestamp: 0.0,
        }
    }
}

/// Types of actions the policy can output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    /// Joint position targets (radians)
    JointPositions,
    /// Joint velocity targets (rad/s)
    JointVelocities,
    /// Joint torque targets (Nm)
    JointTorques,
    /// End-effector delta pose (dx, dy, dz, drx, dry, drz)
    EndEffectorDelta,
    /// Gripper command (0.0 = open, 1.0 = closed)
    Gripper,
}

/// Action head configuration for different output types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionHead {
    pub name: String,
    pub action_type: ActionType,
    pub dimensions: usize,
    pub bounds: Option<Vec<(f32, f32)>>,
}

/// Policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    /// Model type ("openvla", "molmoact")
    pub model_type: String,
    /// Model path or HuggingFace repo ID
    pub model_path: String,
    /// Action heads configuration
    pub action_heads: Vec<ActionHead>,
    /// Image size for model input
    pub image_size: (u32, u32),
    /// Normalization parameters
    pub normalization: NormalizationConfig,
    /// Device configuration
    pub device: DeviceConfig,
    /// Additional metadata for specific implementations
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Normalization parameters for inputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizationConfig {
    pub image_mean: Vec<f32>,
    pub image_std: Vec<f32>,
    pub joint_mean: Option<Vec<f32>>,
    pub joint_std: Option<Vec<f32>>,
}

/// Device configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub device_type: String, // "cpu", "cuda", "mps"
    pub device_id: Option<usize>,
}

/// Result from policy inference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyResult {
    pub actions: Vec<Action>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub inference_time_ms: f64,
}

/// Skill execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillContext {
    pub goal: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub timeout_s: f64,
    pub max_retries: usize,
}

/// Skill execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillResult {
    pub success: bool,
    pub message: String,
    pub execution_time_s: f64,
    pub actions_executed: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_observation_default() {
        let obs = Observation::default();
        assert_eq!(obs.joint_positions.len(), 6);
        assert_eq!(obs.joint_velocities.len(), 6);
        assert_eq!(obs.ee_pose, None);
        assert_eq!(obs.timestamp, 0.0);
        assert_eq!(obs.image.len(), 640 * 480 * 3);
        assert_eq!(obs.image_shape, (480, 640, 3));
    }

    #[test]
    fn test_action_creation() {
        let action = Action {
            action_type: ActionType::JointPositions,
            values: vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6],
            confidence: 0.95,
            timestamp: 1234.5,
        };
        assert_eq!(action.values.len(), 6);
        assert_eq!(action.confidence, 0.95);
        assert_eq!(action.timestamp, 1234.5);
    }

    #[test]
    fn test_action_type_variants() {
        let types = vec![
            ActionType::JointPositions,
            ActionType::JointVelocities,
            ActionType::JointTorques,
            ActionType::EndEffectorDelta,
            ActionType::Gripper,
        ];
        // Ensure all variants can be created
        assert_eq!(types.len(), 5);
    }

    #[test]
    fn test_policy_config() {
        let config = PolicyConfig {
            model_type: "openvla".to_string(),
            model_path: "test_model".to_string(),
            action_heads: vec![],
            image_size: (224, 224),
            normalization: NormalizationConfig {
                image_mean: vec![0.485, 0.456, 0.406],
                image_std: vec![0.229, 0.224, 0.225],
                joint_mean: None,
                joint_std: None,
            },
            device: DeviceConfig {
                device_type: "cuda".to_string(),
                device_id: Some(0),
            },
            metadata: HashMap::new(),
        };
        assert_eq!(config.model_type, "openvla");
        assert_eq!(config.device.device_type, "cuda");
        assert_eq!(config.image_size, (224, 224));
    }

    #[test]
    fn test_policy_result() {
        let result = PolicyResult {
            actions: vec![Action {
                action_type: ActionType::JointPositions,
                values: vec![0.0; 6],
                confidence: 0.95,
                timestamp: 0.0,
            }],
            metadata: HashMap::new(),
            inference_time_ms: 12.5,
        };
        assert_eq!(result.actions.len(), 1);
        assert_eq!(result.actions[0].confidence, 0.95);
        assert!(result.metadata.is_empty());
    }

    #[test]
    fn test_skill_context() {
        let mut params = HashMap::new();
        params.insert("speed".to_string(), serde_json::json!(0.5));
        
        let context = SkillContext {
            goal: "pick_object".to_string(),
            parameters: params.clone(),
            timeout_s: 30.0,
            max_retries: 3,
        };
        
        assert_eq!(context.goal, "pick_object");
        assert_eq!(context.parameters.len(), 1);
        assert_eq!(context.timeout_s, 30.0);
        assert_eq!(context.max_retries, 3);
    }

    #[test]
    fn test_skill_result() {
        let result = SkillResult {
            success: true,
            message: "Skill executed successfully".to_string(),
            execution_time_s: 5.5,
            actions_executed: 42,
        };
        
        assert!(result.success);
        assert_eq!(result.message, "Skill executed successfully");
        assert_eq!(result.execution_time_s, 5.5);
        assert_eq!(result.actions_executed, 42);
    }

    #[test]
    fn test_observation_with_ee_pose() {
        let mut obs = Observation::default();
        obs.ee_pose = Some(vec![0.1, 0.2, 0.3, 0.0, 0.0, 0.0]);
        
        assert!(obs.ee_pose.is_some());
        if let Some(pose) = obs.ee_pose {
            assert_eq!(pose.len(), 6);
            assert_eq!(pose[0], 0.1);
            assert_eq!(pose[1], 0.2);
            assert_eq!(pose[2], 0.3);
        }
    }

    #[test]
    fn test_action_serialization() {
        let action = Action {
            action_type: ActionType::Gripper,
            values: vec![1.0],
            confidence: 0.9,
            timestamp: 999.0,
        };
        
        let json = serde_json::to_string(&action).unwrap();
        let deserialized: Action = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.values, action.values);
        assert_eq!(deserialized.confidence, action.confidence);
        assert_eq!(deserialized.timestamp, action.timestamp);
    }

    #[test]
    fn test_policy_result_with_metadata() {
        let mut metadata = HashMap::new();
        metadata.insert("model_version".to_string(), serde_json::json!("1.0.0"));
        metadata.insert("temperature".to_string(), serde_json::json!(0.7));
        
        let result = PolicyResult {
            actions: vec![Action::default()],
            metadata: metadata.clone(),
            inference_time_ms: 20.0,
        };
        
        assert_eq!(result.metadata.len(), 2);
        assert!(result.metadata.contains_key("model_version"));
        assert!(result.metadata.contains_key("temperature"));
    }
}
