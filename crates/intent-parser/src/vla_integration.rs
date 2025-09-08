//! VLA Policy integration for intent execution

use crate::{ActionType, ActionUnit, JointCommand, MotionCommand, RobotAction};
use std::sync::Arc;
use vla_policy::{Action, ActionType as VlaActionType, Observation, Policy, PolicyConfig};

/// VLA Policy executor for robot actions
pub struct VlaPolicyExecutor {
    policy: Option<Arc<dyn Policy>>,
    config: PolicyConfig,
}

impl VlaPolicyExecutor {
    /// Create a new VLA policy executor
    pub fn new(config: PolicyConfig) -> Self {
        Self {
            policy: None,
            config,
        }
    }

    /// Set the policy to use for execution
    pub fn set_policy(&mut self, policy: Arc<dyn Policy>) {
        self.policy = Some(policy);
    }

    /// Execute a robot action using VLA policy
    pub async fn execute_action(
        &self,
        action: RobotAction,
        current_observation: Observation,
    ) -> Result<VlaExecutionResult, Box<dyn std::error::Error>> {
        match action.action_type {
            ActionType::Motion(motion) => self.execute_motion(motion, current_observation).await,
            ActionType::Joint(joint) => self.execute_joint(joint, current_observation).await,
            ActionType::Stop => self.execute_stop(current_observation).await,
            ActionType::Home => self.execute_home(current_observation).await,
            ActionType::Skill(skill_name) => {
                self.execute_skill(skill_name, current_observation).await
            }
        }
    }

    async fn execute_motion(
        &self,
        motion: MotionCommand,
        observation: Observation,
    ) -> Result<VlaExecutionResult, Box<dyn std::error::Error>> {
        // Convert motion command to VLA action
        let vla_action = self.motion_to_vla_action(motion)?;

        // If we have a policy, use it for refinement
        if let Some(ref policy) = self.policy {
            let policy_result = policy.predict(&observation).await?;

            // Use policy result to refine the action
            let refined_action = self.refine_action_with_policy(vla_action, policy_result)?;
            Ok(VlaExecutionResult::new(refined_action))
        } else {
            // No policy available, use direct action
            Ok(VlaExecutionResult::new(vla_action))
        }
    }

    async fn execute_joint(
        &self,
        joint: JointCommand,
        observation: Observation,
    ) -> Result<VlaExecutionResult, Box<dyn std::error::Error>> {
        // Convert joint command to VLA action
        let vla_action = self.joint_to_vla_action(joint)?;

        // Use policy if available
        if let Some(ref policy) = self.policy {
            let policy_result = policy.predict(&observation).await?;
            let refined_action = self.refine_action_with_policy(vla_action, policy_result)?;
            Ok(VlaExecutionResult::new(refined_action))
        } else {
            Ok(VlaExecutionResult::new(vla_action))
        }
    }

    async fn execute_stop(
        &self,
        _observation: Observation,
    ) -> Result<VlaExecutionResult, Box<dyn std::error::Error>> {
        // Create emergency stop action
        let stop_action = Action {
            action_type: VlaActionType::JointVelocities,
            values: vec![0.0; 6], // Zero velocities for all joints
            confidence: 1.0,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs_f64(),
        };

        Ok(VlaExecutionResult::new(stop_action))
    }

    async fn execute_home(
        &self,
        observation: Observation,
    ) -> Result<VlaExecutionResult, Box<dyn std::error::Error>> {
        // Create home position action
        let home_positions = vec![0.0, -1.57, 1.57, 0.0, 0.0, 0.0]; // Example home pose

        let home_action = Action {
            action_type: VlaActionType::JointPositions,
            values: home_positions,
            confidence: 0.9,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs_f64(),
        };

        // Use policy if available for refinement
        if let Some(ref policy) = self.policy {
            let policy_result = policy.predict(&observation).await?;
            let refined_action = self.refine_action_with_policy(home_action, policy_result)?;
            Ok(VlaExecutionResult::new(refined_action))
        } else {
            Ok(VlaExecutionResult::new(home_action))
        }
    }

    async fn execute_skill(
        &self,
        skill_name: String,
        _observation: Observation,
    ) -> Result<VlaExecutionResult, Box<dyn std::error::Error>> {
        // For now, create a placeholder action
        // In a real implementation, this would look up the skill and execute it
        let skill_action = Action {
            action_type: VlaActionType::EndEffectorDelta,
            values: vec![0.0, 0.0, 0.1, 0.0, 0.0, 0.0], // Small upward motion
            confidence: 0.8,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs_f64(),
        };

        tracing::info!("Executing skill: {}", skill_name);
        Ok(VlaExecutionResult::new(skill_action))
    }

    fn motion_to_vla_action(
        &self,
        motion: MotionCommand,
    ) -> Result<Action, Box<dyn std::error::Error>> {
        // Convert motion command to end-effector delta
        let (dx, dy, dz) = motion.direction.to_vector(motion.distance);

        // Convert units to meters
        let scale = match motion.unit {
            ActionUnit::Millimeters => 0.001,
            ActionUnit::Centimeters => 0.01,
            ActionUnit::Meters => 1.0,
            ActionUnit::Inches => 0.0254,
            ActionUnit::Degrees | ActionUnit::Radians => {
                return Err("Motion commands should use linear units".into());
            }
        };

        let values = vec![
            dx * scale,
            dy * scale,
            dz * scale,
            0.0, // No rotation for simple motion
            0.0,
            0.0,
        ];

        Ok(Action {
            action_type: VlaActionType::EndEffectorDelta,
            values,
            confidence: 0.9,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs_f64(),
        })
    }

    fn joint_to_vla_action(
        &self,
        joint: JointCommand,
    ) -> Result<Action, Box<dyn std::error::Error>> {
        // Convert joint command to joint position action
        let position_rad = match joint.unit {
            ActionUnit::Degrees => joint.position.to_radians(),
            ActionUnit::Radians => joint.position,
            _ => return Err("Joint positions must be in degrees or radians".into()),
        };

        // Create action for specific joint
        let mut values = vec![0.0; 6]; // Assume 6-DOF arm
        let joint_idx = joint.joint_id as usize;
        if joint_idx < values.len() {
            values[joint_idx] = position_rad;
        } else {
            return Err(format!("Joint ID {} out of range", joint.joint_id).into());
        }

        Ok(Action {
            action_type: VlaActionType::JointPositions,
            values,
            confidence: 0.9,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs_f64(),
        })
    }

    fn refine_action_with_policy(
        &self,
        base_action: Action,
        policy_result: vla_policy::PolicyResult,
    ) -> Result<Action, Box<dyn std::error::Error>> {
        // Simple refinement: blend base action with policy prediction
        if policy_result.actions.is_empty() {
            return Ok(base_action);
        }

        let policy_action = &policy_result.actions[0];

        // Only refine if action types match
        if std::mem::discriminant(&base_action.action_type)
            == std::mem::discriminant(&policy_action.action_type)
        {
            // Blend actions (70% base, 30% policy)
            let mut refined_values = Vec::new();
            for (base_val, policy_val) in base_action.values.iter().zip(&policy_action.values) {
                refined_values.push(base_val * 0.7 + policy_val * 0.3);
            }

            Ok(Action {
                action_type: base_action.action_type,
                values: refined_values,
                confidence: (base_action.confidence + policy_action.confidence) / 2.0,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_secs_f64(),
            })
        } else {
            Ok(base_action)
        }
    }
}

/// Result of VLA action execution
#[derive(Debug, Clone)]
pub struct VlaExecutionResult {
    /// The action that was executed
    pub action: Action,
    /// Execution metadata
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
    /// Execution time in milliseconds
    pub execution_time_ms: f64,
}

impl VlaExecutionResult {
    /// Create a new execution result
    pub fn new(action: Action) -> Self {
        Self {
            action,
            metadata: std::collections::HashMap::new(),
            execution_time_ms: 0.0,
        }
    }

    /// Add metadata to the result
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Set execution time
    pub fn with_execution_time(mut self, time_ms: f64) -> Self {
        self.execution_time_ms = time_ms;
        self
    }
}
