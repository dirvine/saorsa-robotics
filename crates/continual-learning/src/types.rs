use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A single data sample from robot interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSample {
    /// Unique identifier for this sample
    pub id: Uuid,
    /// Timestamp when sample was collected
    pub timestamp: f64,
    /// Robot observation (from VLA policy)
    pub observation: vla_policy::Observation,
    /// Action taken by the policy
    pub action: vla_policy::Action,
    /// Reward signal for this action
    pub reward: Option<RewardSignal>,
    /// Whether this was a human intervention
    pub is_intervention: bool,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Reward signal for reinforcement learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardSignal {
    /// Total reward value
    pub total_reward: f32,
    /// Breakdown of reward components
    pub components: HashMap<String, f32>,
    /// Reward type (sparse/dense/shaped)
    pub reward_type: RewardType,
    /// Whether this is a terminal reward
    pub is_terminal: bool,
    /// Discount factor for future rewards
    pub discount_factor: f32,
}

/// Types of reward signals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RewardType {
    /// Sparse rewards (only at task completion)
    Sparse,
    /// Dense rewards (every timestep)
    Dense,
    /// Shaped rewards (task progress indicators)
    Shaped,
}

/// Learning event for monitoring and analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningEvent {
    /// Event timestamp
    pub timestamp: f64,
    /// Event type
    pub event_type: LearningEventType,
    /// Event data
    pub data: HashMap<String, serde_json::Value>,
    /// Associated model version (if applicable)
    pub model_version: Option<String>,
    /// Event severity
    pub severity: EventSeverity,
}

/// Types of learning events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LearningEventType {
    /// New data sample collected
    DataSampleCollected,
    /// Model training started
    TrainingStarted,
    /// Model training completed
    TrainingCompleted,
    /// Model deployment
    ModelDeployed,
    /// Human intervention occurred
    InterventionOccurred,
    /// Safety violation during learning
    SafetyViolation,
    /// Performance improvement detected
    PerformanceImprovement,
    /// Learning system error
    SystemError,
}

/// Event severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventSeverity {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

/// Model version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelVersion {
    /// Unique version identifier
    pub id: String,
    /// Model name
    pub name: String,
    /// Version number (semantic versioning)
    pub version: String,
    /// Training configuration used
    pub training_config: TrainingConfig,
    /// Performance metrics
    pub metrics: HashMap<String, f64>,
    /// Creation timestamp
    pub created_at: f64,
    /// Model file path or URL
    pub model_path: String,
    /// Whether this version is currently deployed
    pub is_deployed: bool,
    /// Parent version (for incremental training)
    pub parent_version: Option<String>,
}

/// Training configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingConfig {
    /// Learning rate
    pub learning_rate: f64,
    /// Batch size
    pub batch_size: usize,
    /// Number of training epochs
    pub num_epochs: usize,
    /// Optimizer type
    pub optimizer: String,
    /// Loss function
    pub loss_function: String,
    /// Dataset size used for training
    pub dataset_size: usize,
    /// Validation split ratio
    pub validation_split: f64,
    /// Random seed for reproducibility
    pub random_seed: Option<u64>,
    /// Additional hyperparameters
    pub hyperparameters: HashMap<String, serde_json::Value>,
}

/// Training job status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrainingStatus {
    /// Job is queued
    Queued,
    /// Training is in progress
    Running,
    /// Training completed successfully
    Completed,
    /// Training failed
    Failed,
    /// Training was cancelled
    Cancelled,
}

/// Training job information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingJob {
    /// Unique job identifier
    pub id: String,
    /// Model version being trained
    pub model_version: String,
    /// Training status
    pub status: TrainingStatus,
    /// Start timestamp
    pub started_at: Option<f64>,
    /// Completion timestamp
    pub completed_at: Option<f64>,
    /// Progress (0.0 to 1.0)
    pub progress: f64,
    /// Current training metrics
    pub metrics: HashMap<String, f64>,
    /// Error message (if failed)
    pub error_message: Option<String>,
}

/// Intervention data from human corrections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterventionData {
    /// Unique intervention identifier
    pub id: Uuid,
    /// Timestamp of intervention
    pub timestamp: f64,
    /// Original observation
    pub original_observation: vla_policy::Observation,
    /// Original action (what the model wanted to do)
    pub original_action: vla_policy::Action,
    /// Corrected action (what the human did instead)
    pub corrected_action: vla_policy::Action,
    /// Reason for intervention
    pub reason: String,
    /// Intervention type
    pub intervention_type: InterventionType,
    /// Additional context
    pub context: HashMap<String, serde_json::Value>,
}

/// Types of human interventions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterventionType {
    /// Safety correction (prevented dangerous action)
    SafetyCorrection,
    /// Task correction (improved task performance)
    TaskCorrection,
    /// Demonstration (teaching new behavior)
    Demonstration,
    /// Recovery (helping robot out of stuck state)
    Recovery,
}

/// Dataset for training
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dataset {
    /// Dataset identifier
    pub id: String,
    /// Dataset name
    pub name: String,
    /// Number of samples
    pub num_samples: usize,
    /// Data sources included
    pub sources: Vec<String>,
    /// Creation timestamp
    pub created_at: f64,
    /// Dataset statistics
    pub statistics: DatasetStats,
}

/// Dataset statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetStats {
    /// Average reward
    pub avg_reward: f64,
    /// Reward standard deviation
    pub reward_std: f64,
    /// Number of interventions
    pub num_interventions: usize,
    /// Task success rate
    pub success_rate: f64,
    /// Average episode length
    pub avg_episode_length: f64,
}
