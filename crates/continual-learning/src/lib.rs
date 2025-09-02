//! continual-learning: Data collection, reward modeling, and continual learning for robotics
//!
//! This crate provides the infrastructure for continual learning in robotic systems including:
//! - Data collection and buffering from robot interactions
//! - Reward modeling for reinforcement learning
//! - Online Fine-Tuning (OFT) for model improvement
//! - Model registry and deployment management
//! - Learning from human interventions

mod types;
pub use types::{
    DataSample, LearningEvent, ModelVersion, RewardSignal, RewardType, TrainingConfig,
};

mod data_collection;
pub use data_collection::{DataBuffer, DataCollector, DataTap};

mod reward_modeling;
pub use reward_modeling::{RewardConfig, RewardModel, RewardPredictor};

#[cfg(feature = "oft")]
pub mod oft;
#[cfg(feature = "oft")]
pub use oft::{FileModelRegistry, OFTTrainer, OFTTrainingSample};

#[cfg(feature = "model-registry")]
pub mod model_registry;
#[cfg(feature = "model-registry")]
pub use model_registry::{
    DeploymentConfig, DeploymentManager, DeploymentType, ModelPromotionWorkflow,
    ModelRegistryClient,
};

#[cfg(feature = "intervention-learning")]
pub mod intervention_learning;
#[cfg(feature = "intervention-learning")]
pub use intervention_learning::{
    ConstraintLearner, ConstraintType, InterventionLearner, InterventionStats, LearnedConstraint,
};

/// Initialize the continual learning system
pub fn init() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Initializing Continual Learning system");
    Ok(())
}

/// Create a data collector with default configuration
#[cfg(feature = "data-collection")]
pub fn create_data_collector() -> Result<DataCollector, Box<dyn std::error::Error>> {
    let config = data_collection::DataCollectorConfig {
        buffer_size: 10000,
        flush_interval_ms: 5000,
        max_file_size_mb: 100,
        compression_enabled: true,
    };

    DataCollector::new(config)
}

/// Create a reward model with default configuration
#[cfg(feature = "reward-modeling")]
pub fn create_reward_model() -> Result<RewardModel, Box<dyn std::error::Error>> {
    let config = reward_modeling::RewardModelConfig {
        learning_rate: 0.001,
        batch_size: 32,
        hidden_dims: vec![128, 64],
        num_epochs: 100,
        validation_split: 0.2,
        model_type: reward_modeling::RewardModelType::Linear,
    };

    RewardModel::new(config)
}

/// Create a model registry client
#[cfg(feature = "model-registry")]
pub fn create_model_registry(
    base_url: &str,
) -> Result<model_registry::ModelRegistryClient, Box<dyn std::error::Error>> {
    model_registry::ModelRegistryClient::new(base_url)
}

/// Record a learning event
pub fn record_event(event: LearningEvent) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Learning event recorded: {:?}", event.event_type);
    // In a real implementation, this would be sent to a logging/monitoring system
    Ok(())
}

/// Get learning statistics
pub fn get_learning_stats() -> Result<LearningStats, Box<dyn std::error::Error>> {
    // Placeholder for learning statistics
    Ok(LearningStats {
        total_samples: 0,
        total_rewards: 0,
        models_trained: 0,
        interventions_processed: 0,
    })
}

/// Learning system statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LearningStats {
    pub total_samples: u64,
    pub total_rewards: u64,
    pub models_trained: u32,
    pub interventions_processed: u32,
}
