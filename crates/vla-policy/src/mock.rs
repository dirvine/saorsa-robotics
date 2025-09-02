//! Mock VLA policy implementation for development and testing

use crate::traits::PolicyMetadata;
use crate::types::ActionType;
use crate::{Action, Observation, Policy, PolicyConfig, PolicyResult};
use async_trait::async_trait;
use rand::Rng;
use std::collections::HashMap;
use std::error::Error;

/// Mock policy implementation for development
pub struct MockPolicy {
    config: PolicyConfig,
    initialized: bool,
}

impl MockPolicy {
    pub fn new(config: PolicyConfig) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            config,
            initialized: true, // Initialize by default for testing
        })
    }
}

#[async_trait]
impl Policy for MockPolicy {
    async fn initialize(&mut self, config: PolicyConfig) -> Result<(), Box<dyn Error>> {
        self.config = config;
        self.initialized = true;
        tracing::info!("Mock VLA policy initialized");
        Ok(())
    }

    async fn predict(&self, observation: &Observation) -> Result<PolicyResult, Box<dyn Error>> {
        if !self.initialized {
            return Err("Policy not initialized".into());
        }

        let start_time = std::time::Instant::now();

        // Generate mock actions based on observation
        let mut actions = Vec::new();
        let mut rng = rand::thread_rng();

        // Create joint position action
        let joint_positions: Vec<f32> = (0..6).map(|_| rng.gen_range(-1.57..1.57)).collect();

        actions.push(Action {
            action_type: ActionType::JointPositions,
            values: joint_positions,
            confidence: rng.gen_range(0.7..0.95),
            timestamp: observation.timestamp,
        });

        // Sometimes add a gripper action
        if rng.gen_bool(0.3) {
            actions.push(Action {
                action_type: ActionType::Gripper,
                values: vec![rng.gen_range(0.0..1.0)],
                confidence: rng.gen_range(0.8..0.98),
                timestamp: observation.timestamp,
            });
        }

        let inference_time = start_time.elapsed().as_millis() as f64;

        Ok(PolicyResult {
            actions,
            metadata: HashMap::new(),
            inference_time_ms: inference_time,
        })
    }

    fn metadata(&self) -> PolicyMetadata {
        PolicyMetadata {
            name: "Mock VLA Policy".to_string(),
            version: "1.0.0".to_string(),
            model_type: "mock".to_string(),
            input_shape: vec![224, 224, 3],
            output_shape: vec![7],
            supported_actions: vec!["joint_positions".to_string(), "gripper".to_string()],
        }
    }

    async fn reset(&mut self) -> Result<(), Box<dyn Error>> {
        // Reset any internal state
        Ok(())
    }
}
