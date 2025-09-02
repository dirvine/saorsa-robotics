//! voice-local: local ASR/TTS traits with a mock backend

mod types;
pub use types::{AsrSegment, AsrStreamConfig, TtsConfig};

mod traits;
pub use traits::{AsrStream, TtsEngine};

#[cfg(feature = "mock")]
mod mock;
#[cfg(feature = "mock")]
pub use mock::{MockAsr, MockTts};

#[cfg(feature = "kyutai_moshi")]
mod kyutai;
#[cfg(feature = "kyutai_moshi")]
pub use kyutai::KyutaiAsrStream;

#[cfg(feature = "audio")]
pub mod mic;

pub mod plugin;
