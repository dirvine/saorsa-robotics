//! Wake word detection demo for Saorsa Robotics
//!
//! This example demonstrates how to use the wake word "Tektra" with the Kyutai STT system.

use voice_local::{AsrStreamConfig, plugin::new_asr_backend, plugin::AsrBackendKind};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Saorsa Robotics Wake Word Demo");
    println!("Say 'Tektra' to activate voice commands!");

    // Configure ASR with wake word
    let config = AsrStreamConfig {
        language: Some("en".to_string()),
        sample_rate_hz: 24000,
        wake_words: vec!["Tektra".to_string()],
        wake_word_sensitivity: 0.7,
    };

    // Create ASR backend
    let mut asr_stream = match new_asr_backend(AsrBackendKind::KyutaiMoshi, config) {
        Ok(stream) => {
            println!("✅ ASR backend initialized with wake word 'Tektra'");
            stream
        }
        Err(e) => {
            eprintln!("❌ Failed to initialize ASR backend: {}", e);
            return Err(e.into());
        }
    };

    // Simulate some audio input (in a real scenario, this would come from microphone)
    let sample_audio = vec![0i16; 48000]; // 2 seconds of silence for demo

    println!("🎤 Processing audio...");

    // Push audio data
    asr_stream.push_audio(&sample_audio);

    // Poll for results
    while let Some(segment) = asr_stream.poll() {
        println!("📝 Transcription: {}", segment.text);

        // Check for wake word
        if asr_stream.is_wake_word_detected() {
            println!("🎯 Wake word 'Tektra' detected! Voice commands activated!");
            println!("🤖 Ready to process commands...");

            // Reset wake word state for next detection
            asr_stream.reset_wake_word();
        }
    }

    // Process any remaining audio
    if let Some(segment) = asr_stream.end() {
        println!("📝 Final transcription: {}", segment.text);
    }

    println!("✅ Demo completed!");
    Ok(())
}