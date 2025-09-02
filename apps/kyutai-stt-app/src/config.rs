use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub hotkey: String,
    pub recording_duration: u32,
    pub model_path: String,
    pub sample_rate: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hotkey: "F12".to_string(),
            recording_duration: 5,
            model_path: "./models/kyutai-stt".to_string(),
            sample_rate: 24000,
        }
    }
}

impl Config {
    pub fn load(path: &str) -> Result<Self> {
        if std::path::Path::new(path).exists() {
            let contents = fs::read_to_string(path)?;
            Ok(serde_json::from_str(&contents)?)
        } else {
            let config = Self::default();
            config.save(path)?;
            Ok(config)
        }
    }

    pub fn save(&self, path: &str) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }
}
