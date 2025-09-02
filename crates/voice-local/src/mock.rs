use crate::{AsrSegment, AsrStream, AsrStreamConfig, TtsConfig, TtsEngine};
use time::OffsetDateTime;

pub struct MockAsr {
    _cfg: AsrStreamConfig,
    counter: u64,
}

impl AsrStream for MockAsr {
    fn new(config: AsrStreamConfig) -> Self
    where
        Self: Sized,
    {
        Self {
            _cfg: config,
            counter: 0,
        }
    }

    fn push_audio(&mut self, _pcm_s16le: &[i16]) {
        // ignore in mock
    }

    fn poll(&mut self) -> Option<AsrSegment> {
        // Return a fake segment every call up to 3
        if self.counter >= 3 {
            return None;
        }
        let idx = self.counter;
        self.counter += 1;
        Some(AsrSegment {
            start_ms: idx * 1000,
            end_ms: (idx + 1) * 1000,
            text: format!("mock utterance {}", idx + 1),
            ts: Some(OffsetDateTime::now_utc()),
        })
    }
}

pub struct MockTts {
    cfg: TtsConfig,
}

impl TtsEngine for MockTts {
    fn new(config: TtsConfig) -> Self
    where
        Self: Sized,
    {
        Self { cfg: config }
    }

    fn synthesize(&mut self, text: &str) -> Vec<i16> {
        // Produce a short 440Hz sine placeholder in S16LE based on text length
        let sr = self.cfg.sample_rate_hz.max(8000);
        let dur_s = (text.len() as f32 / 10.0).clamp(0.2, 1.0);
        let frames = (sr as f32 * dur_s) as usize;
        let mut out = Vec::with_capacity(frames);
        let freq = 440.0_f32;
        for n in 0..frames {
            let t = n as f32 / sr as f32;
            let s = (2.0 * std::f32::consts::PI * freq * t).sin();
            let v = (s * 3000.0) as i16;
            out.push(v);
        }
        out
    }
}
