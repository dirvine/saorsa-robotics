//! HTTP client policy for a WALL‑OSS (or compatible) inference shim.

use crate::{Action, ActionType, Observation, Policy, PolicyConfig, PolicyResult};
use async_trait::async_trait;
use std::collections::HashMap;

pub struct WallxHttpPolicy {
    config: PolicyConfig,
    endpoint: String,
    client: reqwest::Client,
}

impl WallxHttpPolicy {
    pub fn new(config: PolicyConfig) -> Result<Self, Box<dyn std::error::Error>> {
        // Endpoint resolution: prefer config.model_path, else metadata["endpoint"].
        let endpoint = if !config.model_path.is_empty() {
            config.model_path.clone()
        } else {
            config
                .metadata
                .get("endpoint")
                .and_then(|v| v.as_str())
                .unwrap_or("http://127.0.0.1:9009/infer")
                .to_string()
        };
        let client = reqwest::Client::builder().build()?;
        Ok(Self {
            config,
            endpoint,
            client,
        })
    }
}

#[async_trait]
impl Policy for WallxHttpPolicy {
    async fn initialize(&mut self, config: PolicyConfig) -> Result<(), Box<dyn std::error::Error>> {
        self.config = config;
        Ok(())
    }

    async fn predict(
        &self,
        observation: &Observation,
    ) -> Result<PolicyResult, Box<dyn std::error::Error>> {
        #[derive(serde::Serialize)]
        struct ObsReq<'a> {
            image_shape: (u32, u32, u32),
            joint_positions: &'a [f32],
            joint_velocities: &'a [f32],
            ee_pose: Option<&'a [f32]>,
            depth_shape: Option<(u32, u32)>,
            dof_mask: Option<&'a [u8]>,
            dataset_name: Option<&'a str>,
        }

        let req = ObsReq {
            image_shape: observation.image_shape,
            joint_positions: &observation.joint_positions,
            joint_velocities: &observation.joint_velocities,
            ee_pose: observation.ee_pose.as_deref(),
            depth_shape: observation.depth_shape,
            dof_mask: observation.dof_mask.as_deref(),
            dataset_name: observation.dataset_name.as_deref(),
        };

        let start = std::time::Instant::now();
        let resp = self.client.post(&self.endpoint).json(&req).send().await?;

        if !resp.status().is_success() {
            return Err(format!("wallx_http: HTTP {}", resp.status()).into());
        }

        // Expected response: { actions: [{action_type, values, confidence}], inference_time_ms, metadata }
        #[derive(serde::Deserialize)]
        struct ActResp {
            action_type: String,
            values: Vec<f32>,
            confidence: f32,
            #[allow(dead_code)]
            timestamp: Option<f64>,
        }
        #[derive(serde::Deserialize)]
        struct RespBody {
            actions: Vec<ActResp>,
            inference_time_ms: Option<f64>,
            metadata: Option<HashMap<String, serde_json::Value>>,
        }

        let body: RespBody = resp.json().await?;
        let mut actions = Vec::new();
        for a in body.actions {
            let at = match a.action_type.as_str() {
                "EndEffectorDelta" | "ee_delta" => ActionType::EndEffectorDelta,
                "Gripper" | "gripper" => ActionType::Gripper,
                "JointPositions" | "joints" => ActionType::JointPositions,
                "JointVelocities" => ActionType::JointVelocities,
                "JointTorques" => ActionType::JointTorques,
                other => {
                    tracing::warn!("unknown action_type from server: {}", other);
                    ActionType::JointPositions
                }
            };
            actions.push(Action {
                action_type: at,
                values: a.values,
                confidence: a.confidence,
                timestamp: observation.timestamp,
            });
        }

        let dt_remote = body.inference_time_ms.unwrap_or_default();
        let dt_total = start.elapsed().as_millis() as f64;
        Ok(PolicyResult {
            actions,
            metadata: body.metadata.unwrap_or_default(),
            inference_time_ms: dt_remote.max(dt_total),
        })
    }

    fn metadata(&self) -> crate::traits::PolicyMetadata {
        crate::traits::PolicyMetadata {
            name: "WALL‑X HTTP Policy".to_string(),
            version: "0.1.0".to_string(),
            model_type: "wallx_http".to_string(),
            input_shape: vec![
                self.config.image_size.0 as usize,
                self.config.image_size.1 as usize,
                3,
            ],
            output_shape: vec![7],
            supported_actions: vec![
                "EndEffectorDelta".into(),
                "Gripper".into(),
                "JointPositions".into(),
            ],
        }
    }

    async fn reset(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}
