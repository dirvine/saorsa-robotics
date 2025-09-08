use anyhow::Result;
use clap::Parser;
use safety_guard::{create_default_constraint_engine, create_default_watchdog_manager};
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info, warn};
use voice_local::{plugin::new_asr_backend, plugin::AsrBackendKind, AsrStreamConfig};

use continual_learning::{create_data_collector, init as init_continual_learning};
use intent_parser::{init as init_intent_parser, parse_command, RobotAction, VlaPolicyExecutor};
use vla_policy::{create_policy, init as init_vla_policy, Observation, PolicyConfig};

#[derive(Parser)]
#[command(name = "brain-daemon")]
#[command(about = "Saorsa Robotics brain daemon with voice processing")]
struct Args {
    /// ASR backend to use
    #[arg(long, default_value = "kyutai_moshi")]
    asr_backend: String,

    /// Sample rate for audio processing
    #[arg(long, default_value = "24000")]
    sample_rate: u32,

    /// Language for ASR
    #[arg(long, default_value = "en")]
    language: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_tracing();

    let args = Args::parse();

    info!("Saorsa brain-daemon starting");
    info!("ASR Backend: {}", args.asr_backend);
    info!("Sample Rate: {}Hz", args.sample_rate);

    // Initialize safety systems
    let constraint_engine = create_default_constraint_engine()
        .map_err(|e| anyhow::anyhow!("Failed to create constraint engine: {}", e))?;
    let mut watchdog_manager = create_default_watchdog_manager()
        .map_err(|e| anyhow::anyhow!("Failed to create watchdog manager: {}", e))?;

    // Initialize continual learning system
    init_continual_learning()
        .map_err(|e| anyhow::anyhow!("Failed to init continual learning: {}", e))?;
    let data_collector = create_data_collector()
        .map_err(|e| anyhow::anyhow!("Failed to create data collector: {}", e))?;

    // Initialize intent parser
    init_intent_parser()
        .map_err(|e| anyhow::anyhow!("Failed to init intent parser: {}", e))?;

    // Initialize VLA policy system
    init_vla_policy()
        .map_err(|e| anyhow::anyhow!("Failed to init VLA policy: {}", e))?;

    // Create VLA policy executor (using mock policy for now)
    let policy_config = PolicyConfig {
        model_type: "mock".to_string(),
        model_path: "".to_string(),
        action_heads: vec![], // Will be configured by mock policy
        image_size: (224, 224),
        normalization: vla_policy::NormalizationConfig {
            image_mean: vec![0.485, 0.456, 0.406],
            image_std: vec![0.229, 0.224, 0.225],
            joint_mean: None,
            joint_std: None,
        },
        device: vla_policy::DeviceConfig {
            device_type: "cpu".to_string(),
            device_id: None,
        },
        metadata: std::collections::HashMap::new(),
    };

    let policy = create_policy(policy_config.clone())
        .map_err(|e| anyhow::anyhow!("Failed to create policy: {}", e))?;
    let mut vla_executor = VlaPolicyExecutor::new(policy_config);
    vla_executor.set_policy(policy);

    // Set up safety event callback
    let safety_events = Arc::new(Mutex::new(Vec::new()));
    let events_clone = safety_events.clone();
    watchdog_manager.set_event_callback(move |event| {
        info!("Safety Event: {} - {}", event.event_type, event.message);
        if let Ok(mut events) = events_clone.lock() {
            events.push(event);
        }
    });

    info!(
        "Safety systems initialized with {} constraints and {} watchdogs",
        constraint_engine.get_constraints().len(),
        watchdog_manager.get_watchdogs().len()
    );

    info!("Continual learning system initialized");

    // Parse ASR backend
    let backend_kind = match args.asr_backend.as_str() {
        "kyutai_moshi" => AsrBackendKind::KyutaiMoshi,
        "mock" => AsrBackendKind::Mock,
        _ => {
            error!("Unknown ASR backend: {}", args.asr_backend);
            return Err(anyhow::anyhow!("Unknown ASR backend"));
        }
    };

    // Create ASR configuration with wake word
    let asr_config = AsrStreamConfig {
        language: args.language,
        sample_rate_hz: args.sample_rate,
        wake_words: vec!["Tektra".to_string()],
        wake_word_sensitivity: 0.7,
    };

    // Initialize ASR backend
    let asr_stream = match new_asr_backend(backend_kind, asr_config) {
        Ok(stream) => {
            info!("ASR backend initialized successfully");
            stream
        }
        Err(e) => {
            error!("Failed to initialize ASR backend: {}", e);
            return Err(anyhow::anyhow!("ASR initialization failed: {}", e));
        }
    };

    // Start microphone input
    #[cfg(feature = "audio")]
    {
        info!("Starting microphone input...");
        start_voice_processing(
            asr_stream,
            data_collector,
            &mut vla_executor,
            &mut watchdog_manager,
        )
        .await?;
    }

    #[cfg(not(feature = "audio"))]
    {
        warn!("Audio feature not enabled, running in mock mode");
        // In mock mode, just keep the daemon running
        let _ = tokio::signal::ctrl_c().await;
    }

    info!("Brain daemon shutting down");
    Ok(())
}

#[cfg(feature = "audio")]
async fn start_voice_processing(
    mut asr_stream: Box<dyn voice_local::AsrStream + Send>,
    _data_collector: continual_learning::DataCollector,
    _vla_executor: &mut VlaPolicyExecutor,
    _watchdog_manager: &mut safety_guard::WatchdogManager,
) -> Result<()> {
    use voice_local::mic;

    // Start microphone streaming
    let (_stream, config, rx) = mic::start_default_input_i16()?;

    info!(
        "Microphone started: {}Hz, {} channels",
        config.sample_rate_hz, config.channels
    );

    // Spawn audio processing task
    tokio::spawn(async move {
        // Note: For now, we'll skip the VLA executor and watchdog manager usage
        // to avoid lifetime issues. This can be fixed later with proper ownership.
        loop {
            match rx.recv() {
                Ok(audio_chunk) => {
                    // Push audio to ASR stream
                    asr_stream.push_audio(&audio_chunk);

                    // Poll for transcription results
                    while let Some(segment) = asr_stream.poll() {
                        info!("Transcription: {}", segment.text);

                        // Check for wake word detection
                        if asr_stream.is_wake_word_detected() {
                            info!("🎯 Wake word 'Tektra' detected! Activating voice commands...");
                            asr_stream.reset_wake_word();
                        }

                        // Try to parse the transcription as a command
                        match parse_command(&segment.text) {
                            Ok(parse_result) => {
                                if parse_result.confidence >= 0.7 {
                                    info!(
                                        "🎯 Parsed command: {:?} (confidence: {:.2})",
                                        parse_result.intent, parse_result.confidence
                                    );

                                    // Convert intent to robot action
                                    match create_robot_action(parse_result.intent) {
                                        Ok(robot_action) => {
                                            info!(
                                                "🤖 Executing action: {:?}",
                                                robot_action.action_type
                                            );

                                            // Create current observation (placeholder)
                                            let _current_observation = create_current_observation();

                                            // Execute the action using VLA policy
                                            // TODO: Fix lifetime issues with async spawn
                                            // if let Err(e) = execute_robot_action(robot_action, &vla_executor, current_observation).await {
                                            //     warn!("Failed to execute robot action: {}", e);
                                            // }
                                            info!("Action execution simulated (lifetime issue pending fix)");
                                        }
                                        Err(e) => {
                                            warn!("Failed to create robot action: {}", e);
                                        }
                                    }
                                } else {
                                    info!(
                                        "🤔 Low confidence command ({}), ignoring",
                                        parse_result.confidence
                                    );
                                }
                            }
                            Err(e) => {
                                // Not a parseable command, just log it
                                debug!("Not a robot command: {}", e);
                            }
                        }

                        // Update safety watchdogs
                        // TODO: Record camera frame - helpers not exported
                        // safety_guard::watchdogs::helpers::record_camera_frame(&mut watchdog_manager);
                    }

                    // Periodic safety checks
                    // TODO: Enable safety checks when helpers are exported
                    // if let Ok(statuses) = watchdog_manager.check_all() {
                    //     for status in statuses {
                    //         if !status.healthy {
                    //             warn!("Watchdog '{}' unhealthy: {}", status.name, status.last_error.as_deref().unwrap_or("Unknown"));
                    //         }
                    //     }
                    // }
                }
                Err(e) => {
                    error!("Audio receive error: {}", e);
                    break;
                }
            }
        }

        // Note: end() method not available on trait objects due to Sized requirement
        // Final processing would need to be handled differently
    });

    Ok(())
}

/// Create a current observation for VLA policy
fn create_current_observation() -> Observation {
    // Create a placeholder observation
    // In a real implementation, this would come from camera and joint sensors
    Observation {
        image: vec![128; 224 * 224 * 3], // Gray placeholder image
        image_shape: (224, 224, 3),
        depth_u16: None,
        depth_shape: None,
        dof_mask: None,
        dataset_name: None,
        joint_positions: vec![0.0; 6], // Assume 6-DOF arm
        joint_velocities: vec![0.0; 6],
        ee_pose: Some(vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0]), // Placeholder pose
        camera_T_base: None,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64(),
    }
}

/// Convert parsed intent to robot action
fn create_robot_action(
    intent: intent_parser::IntentType,
) -> Result<RobotAction, Box<dyn std::error::Error>> {
    match intent {
        intent_parser::IntentType::Motion(motion) => Ok(RobotAction::motion(
            motion.direction,
            motion.distance,
            motion.unit,
        )),
        intent_parser::IntentType::Joint(joint) => {
            // For now, convert joint commands to motion commands
            // In a real implementation, this would use the device registry
            Ok(RobotAction::motion(
                intent_parser::ActionDirection::Up, // Placeholder
                joint.position,
                joint.unit,
            ))
        }
        intent_parser::IntentType::Stop => Ok(RobotAction::stop()),
        intent_parser::IntentType::Home => Ok(RobotAction::home()),
    }
}

/// Execute a robot action using VLA policy
async fn execute_robot_action(
    action: RobotAction,
    vla_executor: &VlaPolicyExecutor,
    observation: Observation,
) -> Result<(), Box<dyn std::error::Error>> {
    // Execute action using VLA policy executor
    let execution_result = vla_executor.execute_action(action, observation).await?;

    info!(
        "VLA Action executed: {:?} (confidence: {:.2})",
        execution_result.action.action_type, execution_result.action.confidence
    );

    // Here we would send the action to the device registry/CAN transport
    // For now, simulate execution
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Record execution in learning data
    info!(
        "Action execution completed in {:.2}ms",
        execution_result.execution_time_ms
    );

    Ok(())
}

fn setup_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();
}
