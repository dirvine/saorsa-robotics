//! Online Fine-Tuning (OFT) for model improvement
//!
//! This module provides online fine-tuning capabilities for VLA models
//! using collected robot interaction data.

use crate::{DataSample, ModelVersion, TrainingConfig, TrainingJob, TrainingStatus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Configuration for OFT training
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OFTConfig {
    /// Base model to fine-tune
    pub base_model_path: PathBuf,
    /// Output directory for fine-tuned models
    pub output_dir: PathBuf,
    /// Training hyperparameters
    pub training_config: TrainingConfig,
    /// Whether to use LoRA for efficient fine-tuning
    pub use_lora: bool,
    /// LoRA rank (if using LoRA)
    pub lora_rank: Option<usize>,
    /// Learning rate for LoRA
    pub lora_learning_rate: Option<f64>,
    /// Maximum sequence length
    pub max_seq_length: usize,
    /// Gradient accumulation steps
    pub gradient_accumulation_steps: usize,
    /// Save model every N steps
    pub save_steps: usize,
    /// Evaluation steps
    pub eval_steps: usize,
}

/// OFT trainer for online fine-tuning
pub struct OFTTrainer {
    config: OFTConfig,
    current_job: Arc<RwLock<Option<TrainingJob>>>,
    model_registry: Arc<dyn ModelRegistry>,
}

impl OFTTrainer {
    /// Create a new OFT trainer
    pub fn new(config: OFTConfig, model_registry: Arc<dyn ModelRegistry>) -> Self {
        Self {
            config,
            current_job: Arc::new(RwLock::new(None)),
            model_registry,
        }
    }

    /// Start an OFT training job
    pub async fn start_training(
        &self,
        dataset: Vec<DataSample>,
        job_name: String,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let job_id = format!("oft_{}", uuid::Uuid::new_v4().simple());

        let job = TrainingJob {
            id: job_id.clone(),
            model_version: job_name.clone(),
            status: TrainingStatus::Queued,
            started_at: None,
            completed_at: None,
            progress: 0.0,
            metrics: HashMap::new(),
            error_message: None,
        };

        *self.current_job.write().await = Some(job);

        // Spawn training task
        let config = self.config.clone();
        let model_registry = Arc::clone(&self.model_registry);
        let current_job = Arc::clone(&self.current_job);
        let dataset = dataset.clone();

        tokio::spawn(async move {
            if let Err(e) =
                Self::run_training_job(config, dataset, job_name, current_job, model_registry).await
            {
                tracing::error!("OFT training job failed: {}", e);
            }
        });

        Ok(job_id)
    }

    /// Get current training job status
    pub async fn get_job_status(&self, job_id: &str) -> Option<TrainingJob> {
        if let Some(ref job) = *self.current_job.read().await {
            if job.id == job_id {
                return Some(job.clone());
            }
        }
        None
    }

    /// Cancel current training job
    pub async fn cancel_training(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut job = self.current_job.write().await;
        if let Some(ref mut j) = *job {
            j.status = TrainingStatus::Cancelled;
            tracing::info!("Cancelled OFT training job: {}", j.id);
        }
        Ok(())
    }

    async fn run_training_job(
        config: OFTConfig,
        dataset: Vec<DataSample>,
        job_name: String,
        current_job: Arc<RwLock<Option<TrainingJob>>>,
        model_registry: Arc<dyn ModelRegistry>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Update job status to running
        {
            let mut job = current_job.write().await;
            if let Some(ref mut j) = *job {
                j.status = TrainingStatus::Running;
                j.started_at = Some(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_secs_f64(),
                );
            }
        }

        tracing::info!("Starting OFT training job: {}", job_name);

        // Prepare training data
        let training_data = Self::prepare_training_data(&dataset)?;

        // Simulate training process
        let total_steps = config.training_config.num_epochs
            * (training_data.len() / config.training_config.batch_size);
        let mut current_step = 0;

        for epoch in 1..=config.training_config.num_epochs {
            tracing::info!("OFT Epoch {}/{}", epoch, config.training_config.num_epochs);

            for batch in training_data.chunks(config.training_config.batch_size) {
                // Simulate training step
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                current_step += 1;
                let progress = current_step as f64 / total_steps as f64;

                // Update job progress
                {
                    let mut job = current_job.write().await;
                    if let Some(ref mut j) = *job {
                        j.progress = progress;
                        j.metrics.insert("current_epoch".to_string(), epoch as f64);
                        j.metrics
                            .insert("current_step".to_string(), current_step as f64);
                        j.metrics
                            .insert("train_loss".to_string(), (1.0 - progress * 0.5));
                        // Simulated loss
                    }
                }

                // Check if cancelled
                {
                    let job = current_job.read().await;
                    if let Some(ref j) = *job {
                        if matches!(j.status, TrainingStatus::Cancelled) {
                            return Ok(());
                        }
                    }
                }
            }
        }

        // Create new model version
        let new_version = Self::create_model_version(&config, &job_name, &dataset).await?;

        // Register new model
        model_registry.register_version(new_version).await?;

        // Update job status to completed
        {
            let mut job = current_job.write().await;
            if let Some(ref mut j) = *job {
                j.status = TrainingStatus::Completed;
                j.completed_at = Some(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_secs_f64(),
                );
                j.metrics.insert("final_loss".to_string(), 0.1);
                j.metrics.insert("validation_accuracy".to_string(), 0.95);
            }
        }

        tracing::info!("OFT training job completed: {}", job_name);
        Ok(())
    }

    fn prepare_training_data(
        dataset: &[DataSample],
    ) -> Result<Vec<OFTTrainingSample>, Box<dyn std::error::Error>> {
        let mut training_samples = Vec::new();

        for sample in dataset {
            // Convert DataSample to OFT training format
            let oft_sample = OFTTrainingSample {
                observation: sample.observation.clone(),
                action: sample.action.clone(),
                reward: sample
                    .reward
                    .as_ref()
                    .map(|r| r.total_reward)
                    .unwrap_or(0.0),
                is_intervention: sample.is_intervention,
            };
            training_samples.push(oft_sample);
        }

        Ok(training_samples)
    }

    async fn create_model_version(
        config: &OFTConfig,
        job_name: &str,
        dataset: &[DataSample],
    ) -> Result<ModelVersion, Box<dyn std::error::Error>> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs_f64();

        let version = format!("{}_{}", job_name, timestamp as u64);

        Ok(ModelVersion {
            id: uuid::Uuid::new_v4().to_string(),
            name: job_name.to_string(),
            version,
            training_config: config.training_config.clone(),
            metrics: {
                let mut metrics = HashMap::new();
                metrics.insert("dataset_size".to_string(), dataset.len() as f64);
                metrics.insert("training_time".to_string(), 100.0); // Placeholder
                metrics.insert("final_accuracy".to_string(), 0.95); // Placeholder
                metrics
            },
            created_at: timestamp,
            model_path: config
                .output_dir
                .join(format!("{}_final", job_name))
                .to_string_lossy()
                .to_string(),
            is_deployed: false,
            parent_version: None, // Would be set if doing incremental training
        })
    }
}

/// Training sample for OFT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OFTTrainingSample {
    /// Robot observation
    pub observation: vla_policy::Observation,
    /// Action taken
    pub action: vla_policy::Action,
    /// Reward received
    pub reward: f32,
    /// Whether this was from a human intervention
    pub is_intervention: bool,
}

/// Model registry trait for version management
#[async_trait::async_trait]
pub trait ModelRegistry: Send + Sync {
    /// Register a new model version
    async fn register_version(
        &self,
        version: ModelVersion,
    ) -> Result<(), Box<dyn std::error::Error>>;

    /// Get model version by ID
    async fn get_version(
        &self,
        id: &str,
    ) -> Result<Option<ModelVersion>, Box<dyn std::error::Error>>;

    /// List all model versions
    async fn list_versions(&self) -> Result<Vec<ModelVersion>, Box<dyn std::error::Error>>;

    /// Deploy a model version
    async fn deploy_version(&self, id: &str) -> Result<(), Box<dyn std::error::Error>>;

    /// Get currently deployed version
    async fn get_deployed_version(
        &self,
    ) -> Result<Option<ModelVersion>, Box<dyn std::error::Error>>;
}

/// Simple file-based model registry implementation
pub struct FileModelRegistry {
    registry_file: PathBuf,
    versions: Arc<RwLock<HashMap<String, ModelVersion>>>,
}

impl FileModelRegistry {
    /// Create a new file-based model registry
    pub fn new(registry_file: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let versions = if registry_file.exists() {
            let data = std::fs::read_to_string(&registry_file)?;
            serde_json::from_str(&data)?
        } else {
            HashMap::new()
        };

        Ok(Self {
            registry_file,
            versions: Arc::new(RwLock::new(versions)),
        })
    }
}

#[async_trait::async_trait]
impl ModelRegistry for FileModelRegistry {
    async fn register_version(
        &self,
        version: ModelVersion,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut versions = self.versions.write().await;
        versions.insert(version.id.clone(), version);

        // Save to file
        let data = serde_json::to_string_pretty(&*versions)?;
        std::fs::write(&self.registry_file, data)?;

        Ok(())
    }

    async fn get_version(
        &self,
        id: &str,
    ) -> Result<Option<ModelVersion>, Box<dyn std::error::Error>> {
        let versions = self.versions.read().await;
        Ok(versions.get(id).cloned())
    }

    async fn list_versions(&self) -> Result<Vec<ModelVersion>, Box<dyn std::error::Error>> {
        let versions = self.versions.read().await;
        Ok(versions.values().cloned().collect())
    }

    async fn deploy_version(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut versions = self.versions.write().await;

        // Mark all versions as not deployed
        for version in versions.values_mut() {
            version.is_deployed = false;
        }

        // Mark target version as deployed
        if let Some(version) = versions.get_mut(id) {
            version.is_deployed = true;
        } else {
            return Err(format!("Model version {} not found", id).into());
        }

        // Save to file
        let data = serde_json::to_string_pretty(&*versions)?;
        std::fs::write(&self.registry_file, data)?;

        Ok(())
    }

    async fn get_deployed_version(
        &self,
    ) -> Result<Option<ModelVersion>, Box<dyn std::error::Error>> {
        let versions = self.versions.read().await;
        for version in versions.values() {
            if version.is_deployed {
                return Ok(Some(version.clone()));
            }
        }
        Ok(None)
    }
}
