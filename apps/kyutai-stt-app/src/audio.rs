use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

pub struct AudioRecorder {
    _host: cpal::Host,
    device: cpal::Device,
    config: cpal::StreamConfig,
}

impl AudioRecorder {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow!("No input device available"))?;

        let default_cfg = device.default_input_config().map_err(|e| anyhow!("{e}"))?;
        Ok(Self {
            _host: host,
            device,
            config: default_cfg.config(),
        })
    }

    pub fn record_sync(&mut self, duration_seconds: u32) -> Result<(Vec<f32>, u32)> {
        let sample_rate = self.config.sample_rate.0;
        let channels = self.config.channels as usize;
        let total_samples = (sample_rate * duration_seconds) as usize;

        let audio_data = Arc::new(Mutex::new(Vec::with_capacity(total_samples)));
        let audio_data_clone = audio_data.clone();

        let stream = self
            .device
            .build_input_stream(
                &self.config,
                move |data: &[f32], _| {
                    if let Ok(mut buffer) = audio_data_clone.lock() {
                        buffer.extend_from_slice(data);
                    }
                },
                |err| eprintln!("Audio stream error: {}", err),
                None,
            )
            .map_err(|e| anyhow!("{e}"))?;

        stream.play().map_err(|e| anyhow!("{e}"))?;
        std::thread::sleep(std::time::Duration::from_secs(duration_seconds as u64));
        let _ = stream.pause();

        let recorded_data = audio_data
            .lock()
            .map_err(|_| anyhow!("lock poisoned"))?
            .clone();

        // Convert to mono if multi-channel
        let mono_data = if channels > 1 {
            recorded_data
                .chunks(channels)
                .map(|chunk| chunk.iter().copied().sum::<f32>() / channels as f32)
                .collect()
        } else {
            recorded_data
        };

        Ok((mono_data, sample_rate))
    }
}
