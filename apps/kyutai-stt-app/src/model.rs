use crate::decoder::SttDecoder;
use crate::mimi::MimiEncoder;
use anyhow::{anyhow, Result};
use hf_hub::api::sync::Api;
use hound;
use reqwest::blocking as reqwest_blocking;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tokenizers::Tokenizer;

/// Kyutai STT Model implementation
/// This mirrors the example structure found in ../stt and is ready
/// to be replaced by a real Candle-backed model loader + inference.
pub struct SttModel {
    loaded: bool,
    cache_dir: PathBuf,
    // Model files
    repo_id: String,
    config: Option<ModelConfig>,
    mimi_path: Option<PathBuf>,
    weights_path: Option<PathBuf>,
    tokenizer_path: Option<PathBuf>,
    mimi: Option<MimiEncoder>,
    decoder: Option<SttDecoder>,
    tokenizer: Option<Tokenizer>,
}

impl SttModel {
    pub fn new() -> Result<Self> {
        println!("🎯 Initializing Kyutai STT model framework");
        // Derive a cache dir for model-related files (not used heavily yet)
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join("huggingface");
        Ok(Self {
            loaded: false,
            cache_dir,
            repo_id: "kyutai/stt-2.6b-en".into(),
            config: None,
            mimi_path: None,
            weights_path: None,
            tokenizer_path: None,
            mimi: None,
            decoder: None,
            tokenizer: None,
        })
    }

    pub async fn load(&mut self) -> Result<()> {
        println!("🔄 Loading Kyutai STT model metadata from Hugging Face...");
        println!("📥 This is a placeholder - full model loading would:");
        println!("   • Download Kyutai STT model files (2.6B parameters)");
        println!("   • Initialize Mimi audio encoder");
        println!("   • Set up Transformer text decoder");
        println!("   • Load tokenizers and preprocessing pipeline");
        println!("   • Optimize for MPS (Apple Silicon)");

        // Simulate loading time
        // Download required files
        let api = Api::new()?;
        let repo_ref = api.model(self.repo_id.clone());
        let config_path = repo_ref.get("config.json")?;
        let readme_path = repo_ref.get("README.md")?;
        let _ = readme_path; // not used
        let config_str = fs::read_to_string(&config_path)?;
        let cfg: ModelConfig = serde_json::from_str(&config_str)?;
        self.config = Some(cfg.clone());
        // Tokenizer and Mimi
        self.tokenizer_path = Some(repo_ref.get(&cfg.tokenizer_name)?);
        self.mimi_path = Some(repo_ref.get(&cfg.mimi_name)?);
        // Model weights - this is large; fetch explicitly (user can pre-cache)
        self.weights_path = Some(repo_ref.get("model.safetensors")?);
        println!("✅ Downloaded model assets");
        // Initialize Mimi and Decoder
        if let Some(ref mp) = self.mimi_path {
            let mut enc = MimiEncoder::load(mp)?;
            let _ = enc.configure(self.config.as_ref());
            self.mimi = Some(enc);
        }
        if let (Some(ref wp), Some(ref cfg)) = (self.weights_path.as_ref(), self.config.as_ref()) {
            self.decoder = Some(SttDecoder::load(wp, cfg)?);
        }
        // Try load tokenizer if JSON; SentencePiece .model not yet supported here
        if let Some(tokp) = self.tokenizer_path.as_ref() {
            if tokp.extension().and_then(|s| s.to_str()) == Some("json") {
                match Tokenizer::from_file(tokp) {
                    Ok(t) => {
                        self.tokenizer = Some(t);
                    }
                    Err(e) => {
                        eprintln!("tokenizer load failed: {e}");
                    }
                }
            } else {
                eprintln!(
                    "note: tokenizer '{}' is not a JSON tokenizer; local decode pending",
                    tokp.display()
                );
            }
        }

        self.loaded = true;
        println!("✅ Kyutai STT model ready!\n📊 Framework ready for real model integration");
        Ok(())
    }

    pub fn analyze(&self) {
        if let Some(ref cfg) = self.config {
            println!(
                "config: dim={} heads={} layers={} text_card={}",
                cfg.dim, cfg.num_heads, cfg.num_layers, cfg.text_card
            );
        }
        if let Some(ref m) = self.mimi {
            println!("mimi: tensors={}", m.tensors);
            m.graph.print_groups();
            m.graph.print_summary();
        }
        if let Some(ref d) = self.decoder {
            println!("decoder: tensors={}", d.tensors);
            d.print_groups();
            d.print_summary();
        }
    }

    pub fn dump_weights(&self) -> Result<()> {
        if let Some(ref m) = self.mimi {
            let _ = m.dump_tensors();
        }
        if let Some(ref d) = self.decoder {
            let _ = d.dump_tensors();
        }
        Ok(())
    }

    pub async fn transcribe(&mut self, audio_data: &[f32], sample_rate_hz: u32) -> Result<String> {
        if !self.loaded {
            return Err(anyhow!("Model not loaded"));
        }
        // If HF token is configured, use HuggingFace Inference API as a stop-gap.
        // This provides real transcripts of the exact model while local Candle integration is wired.
        if let Ok(token) = env::var("HUGGINGFACEHUB_API_TOKEN") {
            let url = env::var("HF_INFERENCE_URL").unwrap_or_else(|_| {
                "https://api-inference.huggingface.co/models/kyutai/stt-2.6b-en".to_string()
            });
            // HF accepts raw audio; we write WAV at 24k to match model expectation
            let audio_24k = if sample_rate_hz == 24000 {
                audio_data.to_vec()
            } else {
                resample_linear(audio_data, sample_rate_hz, 24000)
            };
            match self.call_hf_inference(&url, &token, &audio_24k) {
                Ok(text) => return Ok(text),
                Err(e) => eprintln!("HF inference fallback failed: {e}"),
            }
        }
        // Otherwise, begin local path: resample → frame → mimi encode → transformer decode (WIP)
        let mono_24k = if sample_rate_hz == 24000 {
            audio_data.to_vec()
        } else {
            resample_linear(audio_data, sample_rate_hz, 24000)
        };
        let frames = frame_audio(&mono_24k, 24000, 0.08); // 80ms frames
        let mimi_tokens = if let Some(ref enc) = self.mimi {
            enc.encode_frames(&frames, 32)?
        } else {
            self.encode_mimi_stub(&frames, 32)?
        };
        let text = self.decode_transformer_stub(&mimi_tokens)?; // TODO: real transformer
        if text.is_empty() {
            Err(anyhow!("Local Candle inference not yet implemented (Mimi+Transformer). Use HF token for temporary fallback."))
        } else {
            Ok(text)
        }
    }
}

impl SttModel {
    fn call_hf_inference(&self, url: &str, token: &str, audio_data: &[f32]) -> Result<String> {
        // Write a temporary WAV (16-bit PCM, 24kHz mono)
        let tmp = tempfile::NamedTempFile::new()?;
        let path = tmp.path();
        self.write_wav(path, audio_data, 24000)?;
        let bytes = fs::read(path)?;

        let client = reqwest_blocking::Client::new();
        let resp = client
            .post(url)
            .header(AUTHORIZATION, format!("Bearer {}", token))
            .header(CONTENT_TYPE, "application/octet-stream")
            .body(bytes)
            .send()?;
        if !resp.status().is_success() {
            return Err(anyhow!("HF inference error: {}", resp.status()));
        }
        // Response for audio tasks is often JSON with text field; handle simple cases
        let text = resp.text()?;
        // Try parse JSON {"text": "..."}
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(s) = v.get("text").and_then(|t| t.as_str()) {
                return Ok(s.to_string());
            }
        }
        // Otherwise return raw
        Ok(text)
    }

    fn write_wav(&self, path: &Path, audio_data: &[f32], sample_rate: u32) -> Result<()> {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(path, spec)?;
        for &s in audio_data {
            let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
            w.write_sample(v)?;
        }
        w.finalize()?;
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct ModelConfig {
    pub dim: usize,
    pub num_heads: usize,
    pub num_layers: usize,
    pub text_card: usize,
    pub existing_text_padding_id: Option<usize>,
    pub context: usize,
    pub positional_embedding: String,
    pub norm: String,
    pub gating: String,
    pub mimi_name: String,
    pub tokenizer_name: String,
}

fn resample_linear(samples: &[f32], sr_in: u32, sr_out: u32) -> Vec<f32> {
    if sr_in == sr_out || samples.is_empty() {
        return samples.to_vec();
    }
    let ratio = sr_out as f32 / sr_in as f32;
    let out_len = (samples.len() as f32 * ratio) as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let pos = i as f32 / ratio;
        let i0 = pos.floor() as usize;
        let i1 = (i0 + 1).min(samples.len() - 1);
        let t = pos - i0 as f32;
        let v = samples[i0] * (1.0 - t) + samples[i1] * t;
        out.push(v);
    }
    out
}

fn frame_audio(samples: &[f32], sr: u32, frame_s: f32) -> Vec<&[f32]> {
    let frame_len = (sr as f32 * frame_s) as usize;
    if frame_len == 0 {
        return vec![];
    }
    let mut frames = Vec::new();
    let mut i = 0;
    while i + frame_len <= samples.len() {
        frames.push(&samples[i..i + frame_len]);
        i += frame_len;
    }
    frames
}

impl SttModel {
    fn encode_mimi_stub(
        &self,
        frames: &Vec<&[f32]>,
        tokens_per_frame: usize,
    ) -> Result<Vec<Vec<usize>>> {
        // TODO: Implement Mimi encoder using mimi-pytorch-*.safetensors
        Ok(frames
            .iter()
            .map(|_| vec![0usize; tokens_per_frame])
            .collect())
    }

    fn decode_transformer_stub(&self, _audio_tokens: &Vec<Vec<usize>>) -> Result<String> {
        // TODO: Implement transformer decode using model.safetensors
        Ok(String::new())
    }
}
