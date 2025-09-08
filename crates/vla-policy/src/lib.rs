//! vla-policy: Vision-Language-Action policy integration
//!
//! This crate provides interfaces for integrating VLA models (OpenVLA, MolmoAct)
//! with robotics control systems. It supports both Python-based inference and
//! native Rust implementations.

mod types;
pub use types::{
    Action, ActionHead, ActionType, DeviceConfig, NormalizationConfig, Observation, PolicyConfig,
    PolicyResult, SkillContext, SkillResult,
};

mod traits;
pub use traits::{Policy, Skill};

pub mod skills;
pub use skills::{PickSkill, PlaceSkill, ReachSkill};

#[cfg(feature = "mock")]
pub mod mock;

#[cfg(feature = "openvla")]
pub mod openvla;

#[cfg(feature = "molmoact")]
pub mod molmoact;

#[cfg(feature = "wallx-http")]
pub mod wallx_http;

/// Initialize the VLA policy system
pub fn init() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Initializing VLA Policy system");
    Ok(())
}

/// Create a policy instance based on configuration
pub fn create_policy(
    config: PolicyConfig,
) -> Result<std::sync::Arc<dyn Policy>, Box<dyn std::error::Error>> {
    match config.model_type.as_str() {
        #[cfg(feature = "mock")]
        "mock" => {
            let policy = mock::MockPolicy::new(config)?;
            Ok(std::sync::Arc::new(policy))
        }
        #[cfg(feature = "openvla")]
        "openvla" => {
            let policy = openvla::OpenVlaPolicy::new(config)?;
            Ok(std::sync::Arc::new(policy))
        }
        #[cfg(feature = "molmoact")]
        "molmoact" => {
            let policy = molmoact::MolmoActPolicy::new(config)?;
            Ok(std::sync::Arc::new(policy))
        }
        #[cfg(feature = "wallx-http")]
        "wallx_http" => {
            let policy = wallx_http::WallxHttpPolicy::new(config)?;
            Ok(std::sync::Arc::new(policy))
        }
        _ => Err(format!("Unsupported policy type: {}", config.model_type).into()),
    }
}
