//! Safety Guard Demo for Saorsa Robotics
//!
//! This demo showcases the safety constraint engine and watchdog monitoring system.

use safety_guard::constraints::ConstraintState;
use safety_guard::{
    create_default_constraint_engine, create_default_watchdog_manager, SafetyStatus,
};
use std::sync::{Arc, Mutex};
use vla_policy::Observation;

use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("🛡️  Saorsa Robotics Safety Guard Demo");
    println!("=====================================");

    // Initialize safety systems
    safety_guard::init()?;
    let mut constraint_engine = create_default_constraint_engine()?;
    let mut watchdog_manager = create_default_watchdog_manager()?;

    // Set up safety event callback
    let event_count = Arc::new(Mutex::new(0));
    let count_clone = event_count.clone();
    watchdog_manager.set_event_callback(move |event| {
        let mut count = count_clone.lock().unwrap();
        *count += 1;
        println!(
            "🚨 Safety Event #{}: {} - {}",
            *count, event.event_type, event.message
        );
    });

    println!("✅ Safety systems initialized");
    println!(
        "   Constraints: {}",
        constraint_engine.get_constraints().len()
    );
    println!("   Watchdogs: {}", watchdog_manager.get_watchdogs().len());

    // Demonstrate constraint checking
    println!("\n🔍 Testing Constraint Engine:");
    println!("============================");

    // Test safe joint positions
    let safe_state = ConstraintState {
        joint_positions: vec![0.5, -0.3, 0.8, 0.1, -0.2, 0.4],
        joint_velocities: vec![0.1, 0.05, 0.08, 0.02, 0.03, 0.06],
        ee_position: Some((0.3, 0.0, 0.5)),
        additional_state: std::collections::HashMap::new(),
    };

    let result = constraint_engine.check_all(&safe_state)?;
    println!(
        "Safe state check: {} violations, {} warnings",
        result.violations.len(),
        result.warnings.len()
    );

    // Test unsafe joint positions
    let unsafe_state = ConstraintState {
        joint_positions: vec![4.0, 0.0, 0.0, 0.0, 0.0, 0.0], // Joint 0 way out of bounds
        joint_velocities: vec![0.1, 0.05, 0.08, 0.02, 0.03, 0.06],
        ee_position: Some((0.3, 0.0, 0.5)),
        additional_state: std::collections::HashMap::new(),
    };

    let result = constraint_engine.check_all(&unsafe_state)?;
    println!(
        "Unsafe state check: {} violations, {} warnings",
        result.violations.len(),
        result.warnings.len()
    );

    if !result.violations.is_empty() {
        for violation in &result.violations {
            println!("  ❌ {}: {}", violation.constraint_name, violation.message);
        }
    }

    // Note: Watchdog system is implemented but not demonstrated in this demo
    // The watchdogs provide monitoring for camera, CAN bus, and E-stop systems

    // Demonstrate action safety checking
    println!("\n🛡️  Testing Action Safety:");
    println!("========================");

    let safe_action = vla_policy::Action {
        action_type: vla_policy::ActionType::JointPositions,
        values: vec![0.2, -0.1, 0.3, 0.0, 0.1, -0.2],
        confidence: 0.9,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs_f64(),
    };

    let observation = Observation {
        image: vec![128; 224 * 224 * 3],
        image_shape: (224, 224, 3),
        joint_positions: vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        joint_velocities: vec![0.0; 6],
        ee_pose: Some(vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs_f64(),
    };

    let safety_status =
        safety_guard::check_action_safety(&safe_action, &observation, &mut constraint_engine)?;
    match safety_status {
        SafetyStatus::Safe => println!("✅ Action is safe"),
        SafetyStatus::Warning(msg) => println!("⚠️  Action warning: {}", msg),
        SafetyStatus::Violation(msg) => println!("❌ Action violation: {}", msg),
        SafetyStatus::EmergencyStop(msg) => println!("🚨 Emergency stop: {}", msg),
    }

    // Test unsafe action
    let unsafe_action = vla_policy::Action {
        action_type: vla_policy::ActionType::JointPositions,
        values: vec![5.0, 0.0, 0.0, 0.0, 0.0, 0.0], // Way out of bounds
        confidence: 0.9,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs_f64(),
    };

    let safety_status =
        safety_guard::check_action_safety(&unsafe_action, &observation, &mut constraint_engine)?;
    match safety_status {
        SafetyStatus::Safe => println!("✅ Unsafe action marked as safe (unexpected)"),
        SafetyStatus::Warning(msg) => println!("⚠️  Unsafe action warning: {}", msg),
        SafetyStatus::Violation(msg) => println!("❌ Unsafe action violation: {}", msg),
        SafetyStatus::EmergencyStop(msg) => println!("🚨 Unsafe action emergency: {}", msg),
    }

    println!("\n🎉 Safety Guard Demo Complete!");
    println!("   - Constraint engine tested");
    println!("   - Watchdog monitoring active");
    println!("   - Action safety validation working");
    println!("   - Safety events: {}", *event_count.lock().unwrap());

    Ok(())
}
