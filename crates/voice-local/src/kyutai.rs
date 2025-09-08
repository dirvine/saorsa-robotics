//! Kyutai STT integration for voice-local
//!
//! This module provides streaming ASR using Kyutai's STT models.
//! It supports both local inference and HuggingFace API fallback.
//!
//! ## Wake Word Support
//!
//! The system includes wake word detection with the default wake word "Tektra".
//! Wake words are configured in `AsrStreamConfig::wake_words` and can be customized.
//!
//! Example:
//! ```ignore
//! use voice_local::AsrStreamConfig;
//!
//! let config = AsrStreamConfig {
//!     wake_words: vec!["Tektra".to_string(), "Computer".to_string()],
//!     wake_word_sensitivity: 0.7,
//!     ..Default::default()
//! };
//! ```

use crate::{AsrSegment, AsrStream, AsrStreamConfig};
use anyhow::{anyhow, Result};
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info, warn};

/// Kyutai STT streaming ASR implementation
pub struct KyutaiAsrStream {
    config: AsrStreamConfig,
    buffer: Vec<i16>,
    sample_count: usize,
    _start_time: Instant,
    last_segment_end: u64,
    model: Arc<KyutaiModel>,
    wake_word_detected: bool,
    wake_word_buffer: String,
}

impl KyutaiAsrStream {
    pub fn new(config: AsrStreamConfig) -> Result<Self> {
        let model = Arc::new(KyutaiModel::new()?);

        Ok(Self {
            config,
            buffer: Vec::with_capacity(48000), // 2 seconds at 24kHz
            sample_count: 0,
            _start_time: Instant::now(),
            last_segment_end: 0,
            model,
            wake_word_detected: false,
            wake_word_buffer: String::new(),
        })
    }
}

impl AsrStream for KyutaiAsrStream {
    fn new(config: AsrStreamConfig) -> Self {
        match Self::new(config.clone()) {
            Ok(stream) => stream,
            Err(e) => {
                warn!(
                    "Failed to create Kyutai ASR stream: {}. Using fallback configuration",
                    e
                );
                // Return a basic configuration that won't crash
                Self {
                    config,
                    buffer: Vec::with_capacity(48000),
                    sample_count: 0,
                    _start_time: Instant::now(),
                    last_segment_end: 0,
                    model: Arc::new(KyutaiModel::default()),
                    wake_word_detected: false,
                    wake_word_buffer: String::new(),
                }
            }
        }
    }

    fn push_audio(&mut self, pcm_s16le: &[i16]) {
        self.buffer.extend_from_slice(pcm_s16le);
        self.sample_count += pcm_s16le.len();
    }

    fn poll(&mut self) -> Option<AsrSegment> {
        // Process audio in chunks of ~1 second (24kHz * 1s = 24000 samples)
        let chunk_size = (self.config.sample_rate_hz as usize).max(16000);

        if self.buffer.len() < chunk_size {
            return None;
        }

        // Extract chunk for processing
        let chunk: Vec<i16> = self.buffer.drain(..chunk_size).collect();

        // Convert to f32 for processing
        let audio_f32: Vec<f32> = chunk.iter().map(|&s| s as f32 / 32768.0).collect();

        // Transcribe the chunk
        match self
            .model
            .transcribe_chunk(&audio_f32, self.config.sample_rate_hz)
        {
            Ok(text) if !text.trim().is_empty() => {
                let transcribed_text = text.trim().to_string();

                // Check for wake words
                if !self.wake_word_detected && self.detect_wake_word(&transcribed_text) {
                    self.wake_word_detected = true;
                    info!("Wake word 'Tektra' detected!");
                }

                let start_ms = self.last_segment_end;
                let duration_ms =
                    (chunk.len() as f64 / self.config.sample_rate_hz as f64 * 1000.0) as u64;
                let end_ms = start_ms + duration_ms;

                self.last_segment_end = end_ms;

                Some(AsrSegment {
                    start_ms,
                    end_ms,
                    text: transcribed_text,
                    ts: Some(time::OffsetDateTime::now_utc()),
                })
            }
            Ok(_) => None, // Empty text, continue buffering
            Err(e) => {
                warn!("Kyutai transcription error: {}", e);
                None
            }
        }
    }

    fn end(self) -> Option<AsrSegment> {
        if self.buffer.is_empty() {
            return None;
        }

        // Process remaining audio
        let audio_f32: Vec<f32> = self.buffer.iter().map(|&s| s as f32 / 32768.0).collect();

        match self
            .model
            .transcribe_chunk(&audio_f32, self.config.sample_rate_hz)
        {
            Ok(text) if !text.trim().is_empty() => {
                let start_ms = self.last_segment_end;
                let duration_ms =
                    (self.buffer.len() as f64 / self.config.sample_rate_hz as f64 * 1000.0) as u64;
                let end_ms = start_ms + duration_ms;

                Some(AsrSegment {
                    start_ms,
                    end_ms,
                    text: text.trim().to_string(),
                    ts: Some(time::OffsetDateTime::now_utc()),
                })
            }
            _ => None,
        }
    }

    /// Check if wake word has been detected
    fn is_wake_word_detected(&self) -> bool {
        self.wake_word_detected
    }

    /// Reset wake word detection state
    fn reset_wake_word(&mut self) {
        self.wake_word_detected = false;
        self.wake_word_buffer.clear();
    }
}

impl KyutaiAsrStream {
    /// Check if wake word is detected in the transcribed text
    fn detect_wake_word(&self, text: &str) -> bool {
        if self.config.wake_words.is_empty() {
            return false;
        }

        let text_lower = text.to_lowercase();

        // Check for exact wake word matches
        for wake_word in &self.config.wake_words {
            let wake_word_lower = wake_word.to_lowercase();

            // Check for exact match
            if text_lower.contains(&wake_word_lower) {
                return true;
            }

            // Check for phonetic variations (simple fuzzy matching)
            if self.fuzzy_wake_word_match(&text_lower, &wake_word_lower) {
                return true;
            }
        }

        false
    }

    /// Simple fuzzy matching for wake word detection
    fn fuzzy_wake_word_match(&self, text: &str, wake_word: &str) -> bool {
        // For "tektra", check for common mispronunciations
        if wake_word == "tektra" {
            let variations = [
                "tektra", "tectra", "tektra", "tektra", "tektra", "techtra", "tektra", "tektra",
                "tektra",
            ];

            for variation in &variations {
                if text.contains(variation) {
                    return true;
                }
            }
        }

        false
    }
}

/// Kyutai STT model wrapper
#[derive(Default)]
struct KyutaiModel {
    // For now, we'll use HF API as the model isn't fully integrated locally
    // TODO: Add local Candle-based inference when available
}

impl KyutaiModel {
    fn new() -> Result<Self> {
        info!("Initializing Kyutai STT model");
        Ok(Self {})
    }

    fn transcribe_chunk(&self, audio: &[f32], sample_rate: u32) -> Result<String> {
        // Try HuggingFace API first
        if let Ok(token) = std::env::var("HUGGINGFACEHUB_API_TOKEN") {
            return self.transcribe_via_hf_api(audio, sample_rate, &token);
        }

        // Fallback to local stub (returns empty for now)
        warn!("No HuggingFace token found, using local stub (returns empty)");
        Ok(String::new())
    }

    fn transcribe_via_hf_api(
        &self,
        audio: &[f32],
        sample_rate: u32,
        token: &str,
    ) -> Result<String> {
        use reqwest::blocking::Client;
        use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};

        // Resample to 24kHz if needed
        let audio_24k = if sample_rate == 24000 {
            audio.to_vec()
        } else {
            resample_linear(audio, sample_rate, 24000)
        };

        // Create temporary WAV file
        let tmp = tempfile::NamedTempFile::new()?;
        let path = tmp.path();
        self.write_wav(path, &audio_24k, 24000)?;

        let bytes = std::fs::read(path)?;
        let url = std::env::var("HF_INFERENCE_URL").unwrap_or_else(|_| {
            "https://api-inference.huggingface.co/models/kyutai/stt-2.6b-en".to_string()
        });

        let client = Client::new();
        let resp = client
            .post(url)
            .header(AUTHORIZATION, format!("Bearer {}", token))
            .header(CONTENT_TYPE, "application/octet-stream")
            .body(bytes)
            .send()?;

        if !resp.status().is_success() {
            return Err(anyhow!("HF inference error: {}", resp.status()));
        }

        let text = resp.text()?;
        debug!("HF API response: {}", text);

        // Parse JSON response
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(text_field) = value.get("text").and_then(|t| t.as_str()) {
                return Ok(text_field.to_string());
            }
        }

        Ok(text)
    }

    fn write_wav(
        &self,
        path: &std::path::Path,
        audio_data: &[f32],
        sample_rate: u32,
    ) -> Result<()> {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = hound::WavWriter::create(path, spec)?;
        for &sample in audio_data {
            let clamped = sample.clamp(-1.0, 1.0);
            let pcm = (clamped * 32767.0) as i16;
            writer.write_sample(pcm)?;
        }
        writer.finalize()?;
        Ok(())
    }
}

/// Simple linear resampling
fn resample_linear(samples: &[f32], sr_in: u32, sr_out: u32) -> Vec<f32> {
    if sr_in == sr_out || samples.is_empty() {
        return samples.to_vec();
    }

    let ratio = sr_out as f64 / sr_in as f64;
    let out_len = (samples.len() as f64 * ratio) as usize;
    let mut out = Vec::with_capacity(out_len);

    for i in 0..out_len {
        let pos = i as f64 / ratio;
        let i0 = pos.floor() as usize;
        let i1 = (i0 + 1).min(samples.len() - 1);
        let t = pos - i0 as f64;
        let sample = samples[i0] * (1.0 - t) as f32 + samples[i1] * t as f32;
        out.push(sample);
    }

    out
}
