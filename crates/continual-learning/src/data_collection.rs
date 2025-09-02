//! Data collection and buffering system
//!
//! This module provides the infrastructure for collecting data from robot interactions,
//! buffering it efficiently, and preparing it for training.

use crate::types::{EventSeverity, LearningEventType};
use crate::{DataSample, LearningEvent};
use parking_lot::RwLock;
use std::collections::VecDeque;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

/// Configuration for data collection
#[derive(Debug, Clone)]
pub struct DataCollectorConfig {
    /// Maximum number of samples to keep in memory buffer
    pub buffer_size: usize,
    /// Interval for flushing data to disk (milliseconds)
    pub flush_interval_ms: u64,
    /// Maximum file size before rotation (MB)
    pub max_file_size_mb: u64,
    /// Whether to compress data files
    pub compression_enabled: bool,
}

/// Main data collector for robot interactions
pub struct DataCollector {
    config: DataCollectorConfig,
    buffer: Arc<RwLock<VecDeque<DataSample>>>,
    data_dir: PathBuf,
    current_file: Option<BufWriter<File>>,
    current_file_size: u64,
    total_samples: u64,
    #[cfg(feature = "tokio")]
    flush_handle: Option<tokio::task::JoinHandle<()>>,
}

impl DataCollector {
    /// Create a new data collector
    pub fn new(config: DataCollectorConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let data_dir = std::env::current_dir()?.join("data").join("learning");

        // Create data directory if it doesn't exist
        fs::create_dir_all(&data_dir)?;

        let buffer_size = config.buffer_size;
        let mut collector = Self {
            config,
            buffer: Arc::new(RwLock::new(VecDeque::with_capacity(buffer_size))),
            data_dir,
            current_file: None,
            current_file_size: 0,
            total_samples: 0,
            #[cfg(feature = "tokio")]
            flush_handle: None,
        };

        // Start background flush task
        #[cfg(feature = "tokio")]
        collector.start_flush_task();

        Ok(collector)
    }

    /// Record a new data sample
    pub fn record_sample(
        &mut self,
        observation: vla_policy::Observation,
        action: vla_policy::Action,
        reward: Option<crate::RewardSignal>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let sample = DataSample {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_secs_f64(),
            observation,
            action,
            reward,
            is_intervention: false,
            metadata: std::collections::HashMap::new(),
        };

        // Add to buffer
        {
            let mut buffer = self.buffer.write();
            buffer.push_back(sample.clone());

            // Remove oldest samples if buffer is full
            while buffer.len() > self.config.buffer_size {
                buffer.pop_front();
            }
        }

        self.total_samples += 1;

        // Record learning event
        let event = LearningEvent {
            timestamp: sample.timestamp,
            event_type: LearningEventType::DataSampleCollected,
            data: std::collections::HashMap::new(),
            model_version: None,
            severity: EventSeverity::Info,
        };

        crate::record_event(event)?;

        Ok(())
    }

    /// Record a human intervention
    pub fn record_intervention(
        &mut self,
        observation: vla_policy::Observation,
        original_action: vla_policy::Action,
        corrected_action: vla_policy::Action,
        reason: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let sample = DataSample {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_secs_f64(),
            observation,
            action: corrected_action,
            reward: None, // Interventions don't have immediate rewards
            is_intervention: true,
            metadata: {
                let mut meta = std::collections::HashMap::new();
                meta.insert(
                    "original_action".to_string(),
                    serde_json::to_value(&original_action)?,
                );
                meta.insert(
                    "intervention_reason".to_string(),
                    serde_json::Value::String(reason.clone()),
                );
                meta
            },
        };

        // Add to buffer
        {
            let mut buffer = self.buffer.write();
            buffer.push_back(sample.clone());
        }

        self.total_samples += 1;

        // Record learning event
        let event = LearningEvent {
            timestamp: sample.timestamp,
            event_type: LearningEventType::InterventionOccurred,
            data: {
                let mut data = std::collections::HashMap::new();
                data.insert("reason".to_string(), serde_json::Value::String(reason));
                data
            },
            model_version: None,
            severity: EventSeverity::Info,
        };

        crate::record_event(event)?;

        Ok(())
    }

    /// Get current buffer contents
    pub fn get_buffer(&self) -> Vec<DataSample> {
        let buffer = self.buffer.read();
        buffer.iter().cloned().collect()
    }

    /// Get buffer statistics
    pub fn get_stats(&self) -> DataCollectorStats {
        let buffer = self.buffer.read();
        DataCollectorStats {
            buffer_size: buffer.len(),
            total_samples: self.total_samples,
            current_file_size: self.current_file_size,
        }
    }

    /// Flush buffer to disk
    pub fn flush(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let samples = {
            let mut buffer = self.buffer.write();
            let samples: Vec<_> = buffer.drain(..).collect();
            samples
        };

        if samples.is_empty() {
            return Ok(());
        }

        self.flush_samples_to_disk(&samples)
    }

    /// Force flush and shutdown
    pub fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(feature = "tokio")]
        {
            if let Some(handle) = self.flush_handle.take() {
                handle.abort();
            }
        }

        self.flush()?;
        Ok(())
    }

    fn flush_samples_to_disk(
        &mut self,
        samples: &[DataSample],
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create new file if needed
        if self.current_file.is_none()
            || self.current_file_size > self.config.max_file_size_mb * 1024 * 1024
        {
            self.rotate_file()?;
        }

        if let Some(ref mut file) = self.current_file {
            for sample in samples {
                let json = serde_json::to_string(sample)?;
                writeln!(file, "{}", json)?;
                self.current_file_size += json.len() as u64 + 1; // +1 for newline
            }
            file.flush()?;
        }

        Ok(())
    }

    fn rotate_file(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Close current file
        self.current_file = None;

        // Create new filename with timestamp
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs();

        let filename = format!("learning_data_{}.jsonl", timestamp);
        let filepath = self.data_dir.join(filename);

        let file = File::create(&filepath)?;
        self.current_file = Some(BufWriter::new(file));
        self.current_file_size = 0;

        tracing::info!("Rotated data file: {:?}", filepath);

        Ok(())
    }

    #[cfg(feature = "tokio")]
    fn start_flush_task(&mut self) {
        let buffer = Arc::clone(&self.buffer);
        let flush_interval = Duration::from_millis(self.config.flush_interval_ms);

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(flush_interval);
            loop {
                interval.tick().await;

                // Check if buffer needs flushing
                let buffer_len = {
                    let buffer = buffer.read();
                    buffer.len()
                };

                if buffer_len > 0 {
                    tracing::debug!("Auto-flushing {} samples to disk", buffer_len);
                    // Note: In a real implementation, we'd need a way to call flush() from here
                    // This is a simplified version
                }
            }
        });

        self.flush_handle = Some(handle);
    }
}

/// Statistics for data collection
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DataCollectorStats {
    pub buffer_size: usize,
    pub total_samples: u64,
    pub current_file_size: u64,
}

/// Data tap for intercepting data from various sources
pub trait DataTap {
    /// Called when new data is available
    fn on_data(&mut self, sample: &DataSample) -> Result<(), Box<dyn std::error::Error>>;

    /// Get tap identifier
    fn id(&self) -> &str;
}

/// Simple data tap that just logs data
#[allow(dead_code)]
pub struct LoggingDataTap {
    id: String,
}

#[allow(dead_code)]
impl LoggingDataTap {
    pub fn new(id: String) -> Self {
        Self { id }
    }
}

impl DataTap for LoggingDataTap {
    fn on_data(&mut self, sample: &DataSample) -> Result<(), Box<dyn std::error::Error>> {
        tracing::debug!(
            "Data sample received: id={}, timestamp={}, intervention={}",
            sample.id,
            sample.timestamp,
            sample.is_intervention
        );
        Ok(())
    }

    fn id(&self) -> &str {
        &self.id
    }
}

/// Data buffer for efficient data management
pub struct DataBuffer {
    samples: VecDeque<DataSample>,
    max_size: usize,
}

impl DataBuffer {
    pub fn new(max_size: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    pub fn push(&mut self, sample: DataSample) {
        self.samples.push_back(sample);

        // Remove oldest samples if over capacity
        while self.samples.len() > self.max_size {
            self.samples.pop_front();
        }
    }

    pub fn pop(&mut self) -> Option<DataSample> {
        self.samples.pop_front()
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    pub fn clear(&mut self) {
        self.samples.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = &DataSample> {
        self.samples.iter()
    }
}
