use crate::types::{SkillContext, SkillResult};
use crate::{Action, Observation, PolicyConfig, PolicyResult};
use async_trait::async_trait;
use std::error::Error;

/// Core policy trait for VLA models
#[async_trait]
pub trait Policy: Send + Sync {
    /// Initialize the policy with configuration
    async fn initialize(&mut self, config: PolicyConfig) -> Result<(), Box<dyn Error>>;

    /// Run inference on an observation
    async fn predict(&self, observation: &Observation) -> Result<PolicyResult, Box<dyn Error>>;

    /// Get policy metadata
    fn metadata(&self) -> PolicyMetadata;

    /// Reset policy state (if stateful)
    async fn reset(&mut self) -> Result<(), Box<dyn Error>>;
}

/// Metadata about a policy
#[derive(Debug, Clone)]
pub struct PolicyMetadata {
    pub name: String,
    pub version: String,
    pub model_type: String,
    pub input_shape: Vec<usize>,
    pub output_shape: Vec<usize>,
    pub supported_actions: Vec<String>,
}

/// Skill trait for high-level robot behaviors
#[async_trait]
pub trait Skill: Send + Sync {
    /// Execute a skill with given context
    async fn execute(
        &self,
        context: &SkillContext,
        policy: &dyn Policy,
    ) -> Result<SkillResult, Box<dyn Error>>;

    /// Check if skill can be executed in current state
    async fn can_execute(&self, context: &SkillContext) -> Result<bool, Box<dyn Error>>;

    /// Get skill metadata
    fn metadata(&self) -> SkillMetadata;

    /// Validate skill parameters
    fn validate_parameters(
        &self,
        parameters: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<(), Box<dyn Error>>;
}

/// Metadata about a skill
#[derive(Debug, Clone)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    pub parameters: Vec<SkillParameter>,
    pub estimated_duration_s: f64,
}

/// Skill parameter definition
#[derive(Debug, Clone)]
pub struct SkillParameter {
    pub name: String,
    pub param_type: String,
    pub description: String,
    pub required: bool,
    pub default_value: Option<serde_json::Value>,
}

/// Action head trait for different output types
pub trait ActionHeadTrait: Send + Sync {
    /// Convert raw policy outputs to actions
    fn process_outputs(
        &self,
        raw_outputs: &[f32],
        confidence: f32,
    ) -> Result<Vec<Action>, Box<dyn Error>>;

    /// Get action head metadata
    fn metadata(&self) -> ActionHeadMetadata;
}

/// Metadata for action heads
#[derive(Debug, Clone)]
pub struct ActionHeadMetadata {
    pub name: String,
    pub action_type: String,
    pub dimensions: usize,
    pub bounds: Option<Vec<(f32, f32)>>,
}
