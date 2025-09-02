use crate::{AsrSegment, AsrStreamConfig, TtsConfig};

pub trait AsrStream {
    fn new(config: AsrStreamConfig) -> Self
    where
        Self: Sized;
    fn push_audio(&mut self, _pcm_s16le: &[i16]);
    fn poll(&mut self) -> Option<AsrSegment>;
    fn end(self) -> Option<AsrSegment>
    where
        Self: Sized,
    {
        None
    }

    /// Check if wake word has been detected
    fn is_wake_word_detected(&self) -> bool {
        false
    }

    /// Reset wake word detection state
    fn reset_wake_word(&mut self) {}
}

pub trait TtsEngine {
    fn new(config: TtsConfig) -> Self
    where
        Self: Sized;
    fn synthesize(&mut self, text: &str) -> Vec<i16>;
}
