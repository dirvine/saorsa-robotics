use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use tokio::sync::Mutex;

mod audio;
mod config;
mod decoder;
mod keyboard;
mod mimi;
mod model;

use audio::AudioRecorder;
use config::Config;
use keyboard::KeyboardListener;
use model::SttModel;

#[derive(Parser)]
#[command(name = "kyutai-stt-app")]
#[command(about = "Real-time speech-to-text with global hotkey (Kyutai scaffold)")]
struct Args {
    /// Configuration file path
    #[arg(short, long, default_value = "config.json")]
    config: String,

    /// Hotkey to trigger recording (e.g., "F12", "Space")
    #[arg(short, long)]
    hotkey: Option<String>,

    /// Recording duration in seconds
    #[arg(short, long, default_value = "5")]
    duration: u32,

    /// Dump Mimi/decoder safetensors keys and exit
    #[arg(long, action = clap::ArgAction::SetTrue)]
    dump_weights: bool,

    /// Analyze model config + tensor groupings and exit
    #[arg(long, action = clap::ArgAction::SetTrue)]
    analyze_model: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    // Load configuration
    let config = Config::load(&args.config)?;

    // Override config with command line args
    let hotkey = args.hotkey.unwrap_or(config.hotkey);
    let duration = args.duration;

    println!("Kyutai STT App");
    println!("Hotkey: {}", hotkey);
    println!("Recording duration: {}s", duration);
    println!("Press {} to start recording...", hotkey);

    // Initialize components
    let model = Arc::new(Mutex::new(SttModel::new()?));
    let recorder = Arc::new(Mutex::new(AudioRecorder::new()?));
    let keyboard_listener = KeyboardListener::new(hotkey)?;

    // Load the model
    {
        let mut model_lock = model.lock().await;
        println!("Loading Kyutai STT model (scaffold)...");
        model_lock.load().await?;
        println!("Model ready.");
        if args.dump_weights {
            println!("Dumping Mimi/decoder weights...");
            model_lock.dump_weights()?;
            return Ok(());
        }
        if args.analyze_model {
            println!("Analyzing model...");
            model_lock.analyze();
            return Ok(());
        }
    }

    // Start keyboard listener
    let model_clone = model.clone();
    let recorder_clone = recorder.clone();

    let _ = keyboard_listener.start(move || {
        let model = model_clone.clone();
        let recorder = recorder_clone.clone();

        // Run synchronously in a background thread
        let _ = std::thread::spawn(move || {
            if let Ok(rt) = tokio::runtime::Runtime::new() {
                rt.block_on(async {
                    if let Err(e) = handle_recording(model, recorder, duration).await {
                        eprintln!("Recording error: {}", e);
                    }
                });
            } else {
                eprintln!("Failed to create runtime");
            }
        });
    });

    // Keep the main thread alive until Ctrl-C
    let _ = tokio::signal::ctrl_c().await;
    println!("Shutting down...");

    Ok(())
}

async fn handle_recording(
    model: Arc<Mutex<SttModel>>,
    recorder: Arc<Mutex<AudioRecorder>>,
    duration: u32,
) -> Result<()> {
    println!("Recording {}s... Speak now!", duration);

    // Record audio (synchronous)
    let (audio_data, sample_rate) = {
        let mut rec = recorder.lock().await;
        rec.record_sync(duration)?
    };

    println!("Transcribing...");

    // Transcribe
    let transcription = {
        let mut model_lock = model.lock().await;
        model_lock.transcribe(&audio_data, sample_rate).await?
    };

    if !transcription.is_empty() {
        println!("Result: {}", transcription);
    } else {
        println!("No speech detected");
    }

    Ok(())
}
