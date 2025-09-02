use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsrStreamConfig {
    pub language: Option<String>,
    pub sample_rate_hz: u32,
    #[serde(default)]
    pub wake_words: Vec<String>,
    #[serde(default = "default_sensitivity")]
    pub wake_word_sensitivity: f32,
}

fn default_sensitivity() -> f32 {
    0.7
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsrSegment {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
    pub ts: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    pub voice: Option<String>,
    pub sample_rate_hz: u32,
}
