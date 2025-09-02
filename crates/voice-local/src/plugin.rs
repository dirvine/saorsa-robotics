#[cfg(feature = "kyutai_moshi")]
use crate::KyutaiAsrStream;
use crate::{AsrStream, AsrStreamConfig, MockAsr};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum AsrBackendKind {
    Mock,
    WhisperCpp,
    FasterWhisper,
    Vosk,
    KyutaiMoshi,
}

pub fn new_asr_backend(
    kind: AsrBackendKind,
    cfg: AsrStreamConfig,
) -> Result<Box<dyn AsrStream + Send>, String> {
    match kind {
        AsrBackendKind::Mock => Ok(Box::new(MockAsr::new(cfg))),
        AsrBackendKind::WhisperCpp => Err("whisper_cpp backend not yet integrated".into()),
        AsrBackendKind::FasterWhisper => Err("faster_whisper backend not yet integrated".into()),
        AsrBackendKind::Vosk => Err("vosk backend not yet integrated".into()),
        AsrBackendKind::KyutaiMoshi => {
            #[cfg(feature = "kyutai_moshi")]
            {
                KyutaiAsrStream::new(cfg)
                    .map(|s| Box::new(s) as Box<dyn AsrStream + Send>)
                    .map_err(|e| e.to_string())
            }
            #[cfg(not(feature = "kyutai_moshi"))]
            {
                Err("kyutai_moshi feature not enabled".into())
            }
        }
    }
}
