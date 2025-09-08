#![allow(dead_code)]
//! safety-guard: Safety constraints and monitoring for robotics
//!
//! This crate provides a comprehensive safety system for robotic operations including:
//! - Safety constraints DSL for joints, end-effector, and workspace limits
//! - Watchdog monitoring for system health (camera, CAN, E-stop)
//! - Safety violation detection and response
//! - Audit logging of safety events

mod types;
pub use types::{SafetyCheckResult, SafetyConstraint, SafetyViolation, WatchdogStatus};

pub mod constraints;
pub use constraints::{create_default_constraint_engine, ConstraintEngine, ConstraintState};

mod watchdogs;
pub use watchdogs::{CameraWatchdog, CanWatchdog, EStopWatchdog, Watchdog, WatchdogManager};

mod dsl;
pub use dsl::{ConstraintExpression, SafetyDSL};

// Async watchdogs module will be added later
// #[cfg(feature = "watchdogs")]
// mod async_watchdogs;
// #[cfg(feature = "watchdogs")]
// pub use async_watchdogs::{AsyncWatchdog, WatchdogEvent};

/// Initialize the safety system
pub fn init() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Initializing Safety Guard system");
    Ok(())
}

/// Create a watchdog manager with default watchdogs
#[cfg(feature = "watchdogs")]
pub fn create_default_watchdog_manager() -> Result<WatchdogManager, Box<dyn std::error::Error>> {
    let mut manager = WatchdogManager::new();

    // Add camera watchdog (30 FPS minimum)
    manager.add_watchdog(Box::new(CameraWatchdog::new(
        30.0,
        std::time::Duration::from_secs(5),
    )))?;

    // Add CAN watchdog (100ms timeout)
    manager.add_watchdog(Box::new(CanWatchdog::new(
        std::time::Duration::from_millis(100),
    )))?;

    // Add E-stop watchdog if GPIO is available
    #[cfg(feature = "gpio")]
    {
        use std::sync::Arc;
        use tokio::sync::Mutex;
        let e_stop_watchdog = EStopWatchdog::new_with_gpio(Arc::new(Mutex::new(false)))?;
        manager.add_watchdog(Box::new(e_stop_watchdog))?;
    }

    Ok(manager)
}

/// Safety check result
#[derive(Debug, Clone)]
pub enum SafetyStatus {
    Safe,
    Warning(String),
    Violation(String),
    EmergencyStop(String),
}

/// Check if a planned action is safe
pub fn check_action_safety(
    action: &vla_policy::Action,
    current_state: &vla_policy::Observation,
    constraint_engine: &mut ConstraintEngine,
) -> Result<SafetyStatus, Box<dyn std::error::Error>> {
    use std::collections::HashMap;

    // Create constraint state from current observation
    let mut state = ConstraintState {
        joint_positions: Vec::new(),
        joint_velocities: Vec::new(),
        ee_position: None,
        additional_state: HashMap::new(),
    };

    match &action.action_type {
        vla_policy::ActionType::JointPositions => {
            // Use the action values as the new joint positions to check
            state.joint_positions = action.values.clone();
            state.joint_velocities = current_state.joint_velocities.clone();
        }
        vla_policy::ActionType::EndEffectorDelta => {
            if let Some(ee_pose) = &current_state.ee_pose {
                // Calculate new EE pose after applying delta
                let new_x = ee_pose[0] + action.values[0];
                let new_y = ee_pose[1] + action.values[1];
                let new_z = ee_pose[2] + action.values[2];

                state.ee_position = Some((new_x, new_y, new_z));
                state.joint_positions = current_state.joint_positions.clone();
                state.joint_velocities = current_state.joint_velocities.clone();
            }
        }
        _ => {
            // For other action types, use current state
            state.joint_positions = current_state.joint_positions.clone();
            state.joint_velocities = current_state.joint_velocities.clone();
            if let Some(ee_pose) = &current_state.ee_pose {
                if ee_pose.len() >= 3 {
                    state.ee_position = Some((ee_pose[0], ee_pose[1], ee_pose[2]));
                }
            }
        }
    }

    // Check constraints
    let result = constraint_engine.check_all(&state)?;

    if !result.is_safe {
        // Return the first violation as an error
        if let Some(violation) = result.violations.first() {
            return Ok(SafetyStatus::Violation(violation.message.clone()));
        }
    }

    // Check for warnings
    if let Some(warning) = result.warnings.first() {
        return Ok(SafetyStatus::Warning(warning.message.clone()));
    }

    Ok(SafetyStatus::Safe)
}
