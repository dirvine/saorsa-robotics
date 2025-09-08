//! Voice Command Demo Application
//!
//! This application demonstrates end-to-end voice command processing:
//! Speech → ASR → Intent Parsing → VLA Policy → CAN Commands

use anyhow::Result;
use clap::Parser;
use std::io::{self, Write};
use tokio;
use tracing::{error, info, warn};

use continual_learning::{create_data_collector, init as init_continual_learning};
use intent_parser::{init as init_intent_parser, parse_command, VlaPolicyExecutor};
use vla_policy::{create_policy, init as init_vla_policy, Observation, PolicyConfig};

#[derive(Parser)]
#[command(name = "voice-demo")]
#[command(about = "Saorsa Robotics Voice Command Demo")]
struct Args {
    /// Use mock VLA policy (default)
    #[arg(long)]
    mock_policy: bool,

    /// Enable learning data collection
    #[arg(long)]
    collect_data: bool,

    /// Interactive mode (read commands from stdin)
    #[arg(long)]
    interactive: bool,

    /// Test specific command
    #[arg(long)]
    test_command: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_tracing();

    let args = Args::parse();

    info!("🎤 Starting Saorsa Voice Command Demo");

    // Initialize all systems
    if let Err(e) = init_intent_parser() {
        error!("Failed to initialize intent parser: {}", e);
        return Ok(());
    }
    if let Err(e) = init_vla_policy() {
        error!("Failed to initialize VLA policy: {}", e);
        return Ok(());
    }
    if args.collect_data {
        if let Err(e) = init_continual_learning() {
            error!("Failed to initialize continual learning: {}", e);
            return Ok(());
        }
    }

    // Create VLA policy
    let policy_config = if args.mock_policy {
        PolicyConfig {
            model_type: "mock".to_string(),
            model_path: "".to_string(),
            action_heads: vec![],
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
        }
    } else {
        // Would load real policy config here
        return Ok(());
    };

    let policy = match create_policy(policy_config.clone()) {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to create policy: {}", e);
            return Ok(());
        }
    };
    let mut vla_executor = VlaPolicyExecutor::new(policy_config);
    vla_executor.set_policy(std::sync::Arc::from(policy));

    // Create data collector if enabled
    let mut data_collector = if args.collect_data {
        match create_data_collector() {
            Ok(collector) => Some(collector),
            Err(e) => {
                warn!("Failed to create data collector: {}", e);
                None
            }
        }
    } else {
        None
    };

    if let Some(test_cmd) = args.test_command {
        // Test a specific command
        test_single_command(&test_cmd, &vla_executor, &mut data_collector.as_mut()).await?;
    } else if args.interactive {
        // Interactive mode
        run_interactive_demo(&vla_executor, &mut data_collector.as_mut()).await?;
    } else {
        // Run demo with predefined commands
        run_demo_commands(&vla_executor, &mut data_collector.as_mut()).await?;
    }

    info!("✅ Voice demo completed");
    Ok(())
}

async fn test_single_command(
    command: &str,
    vla_executor: &VlaPolicyExecutor,
    data_collector: &mut Option<&mut continual_learning::DataCollector>,
) -> Result<()> {
    println!("🎤 Testing command: \"{}\"", command);

    match parse_command(command) {
        Ok(parse_result) => {
            println!(
                "✅ Parsed: {:?} (confidence: {:.2})",
                parse_result.intent, parse_result.confidence
            );

            if parse_result.confidence >= 0.7 {
                // Create robot action
                let robot_action = match create_robot_action(parse_result.intent) {
                    Ok(action) => action,
                    Err(e) => {
                        warn!("Failed to create robot action: {}", e);
                        return Ok(());
                    }
                };

                // Create observation
                let observation = create_demo_observation();

                // Execute action
                let start_time = std::time::Instant::now();
                let execution_result =
                    match vla_executor.execute_action(robot_action, observation).await {
                        Ok(result) => result,
                        Err(e) => {
                            warn!("Failed to execute robot action: {}", e);
                            return Ok(());
                        }
                    };
                let execution_time = start_time.elapsed().as_millis() as f64;

                println!(
                    "🤖 Executed: {:?} in {:.2}ms",
                    execution_result.action.action_type, execution_time
                );

                // Record learning data if enabled
                if let Some(_collector) = data_collector.as_mut() {
                    // Note: Data collection is enabled but we can't mutate through shared reference
                    // In a real implementation, this would be handled differently
                    info!("Data collection enabled - sample would be recorded");
                }

                println!("✅ Command executed successfully!");
            } else {
                println!("⚠️  Low confidence, command ignored");
            }
        }
        Err(e) => {
            println!("❌ Failed to parse command: {}", e);
        }
    }

    Ok(())
}

async fn run_interactive_demo(
    vla_executor: &VlaPolicyExecutor,
    data_collector: &mut Option<&mut continual_learning::DataCollector>,
) -> Result<()> {
    println!("🎤 Interactive Voice Command Demo");
    println!("Type voice commands and press Enter (or 'quit' to exit):");
    println!("Examples:");
    println!("  - 'raise arm 15 cm'");
    println!("  - 'lower the arm by 10 centimeters'");
    println!("  - 'move arm up 20 cm'");
    println!("  - 'stop the arm'");
    println!("  - 'go to home position'");
    println!();

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("🎤 Command: ");
        stdout.flush()?;

        let mut input = String::new();
        stdin.read_line(&mut input)?;
        let command = input.trim();

        if command.eq_ignore_ascii_case("quit") || command.eq_ignore_ascii_case("exit") {
            break;
        }

        if !command.is_empty() {
            test_single_command(command, vla_executor, data_collector).await?;
            println!();
        }
    }

    Ok(())
}

async fn run_demo_commands(
    vla_executor: &VlaPolicyExecutor,
    data_collector: &mut Option<&mut continual_learning::DataCollector>,
) -> Result<()> {
    let demo_commands = vec![
        "raise arm 15 cm",
        "lower the arm by 10 centimeters",
        "move arm up 20 cm",
        "extend arm forward 30 cm",
        "stop the arm",
        "go to home position",
    ];

    println!(
        "🎤 Running Voice Command Demo with {} commands",
        demo_commands.len()
    );
    println!();

    for (i, command) in demo_commands.iter().enumerate() {
        println!("{}/{}: {}", i + 1, demo_commands.len(), command);
        test_single_command(command, vla_executor, data_collector).await?;
        println!();

        // Small delay between commands
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    println!("🎉 Demo completed! All commands processed.");
    Ok(())
}

/// Convert parsed intent to robot action (duplicate from brain-daemon for demo)
fn create_robot_action(
    intent: intent_parser::IntentType,
) -> Result<intent_parser::RobotAction, Box<dyn std::error::Error>> {
    match intent {
        intent_parser::IntentType::Motion(motion) => Ok(intent_parser::RobotAction::motion(
            motion.direction,
            motion.distance,
            motion.unit,
        )),
        intent_parser::IntentType::Joint(joint) => Ok(intent_parser::RobotAction::motion(
            intent_parser::ActionDirection::Up,
            joint.position,
            joint.unit,
        )),
        intent_parser::IntentType::Stop => Ok(intent_parser::RobotAction::stop()),
        intent_parser::IntentType::Home => Ok(intent_parser::RobotAction::home()),
    }
}

/// Create a demo observation
fn create_demo_observation() -> Observation {
    Observation {
        image: vec![128; 224 * 224 * 3], // Gray placeholder image
        image_shape: (224, 224, 3),
        depth_u16: None,
        depth_shape: None,
        dof_mask: None,
        dataset_name: None,
        joint_positions: vec![0.0, -1.57, 1.57, 0.0, 0.0, 0.0], // Home pose
        joint_velocities: vec![0.0; 6],
        ee_pose: Some(vec![0.3, 0.0, 0.4, 0.0, 3.14, 0.0]), // Example pose
        camera_t_base: None,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64(),
    }
}

fn setup_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();
}
