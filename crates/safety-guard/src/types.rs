use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

/// Safety constraint definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConstraint {
    pub name: String,
    pub constraint_type: ConstraintType,
    pub enabled: bool,
    pub severity: ViolationSeverity,
    pub description: String,
}

/// Types of safety constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstraintType {
    JointPosition {
        joint_index: usize,
        min: f32,
        max: f32,
    },
    JointVelocity {
        joint_index: usize,
        max: f32,
    },
    JointTorque {
        joint_index: usize,
        max: f32,
    },
    WorkspaceBounds {
        min_x: f32,
        max_x: f32,
        min_y: f32,
        max_y: f32,
        min_z: f32,
        max_z: f32,
    },
    EndEffectorBounds {
        max_reach: f32,
        min_height: f32,
    },
    CollisionAvoidance {
        enabled: bool,
    },
}

/// Severity levels for safety violations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ViolationSeverity {
    Warning,
    Error,
    Critical,
    Emergency,
}

/// Safety violation report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyViolation {
    pub timestamp: SystemTime,
    pub constraint_name: String,
    pub severity: ViolationSeverity,
    pub message: String,
    pub violated_value: Option<f32>,
    pub expected_range: Option<(f32, f32)>,
    pub context: std::collections::HashMap<String, serde_json::Value>,
}

/// Result of a safety check
#[derive(Debug, Clone)]
pub struct SafetyCheckResult {
    pub is_safe: bool,
    pub violations: Vec<SafetyViolation>,
    pub warnings: Vec<SafetyViolation>,
    pub check_duration: Duration,
}

/// Watchdog status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchdogStatus {
    pub name: String,
    pub healthy: bool,
    pub last_check: SystemTime,
    pub last_error: Option<String>,
    pub timeout_duration: Duration,
    pub consecutive_failures: u32,
}

/// Safety event for logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyEvent {
    pub timestamp: SystemTime,
    pub event_type: SafetyEventType,
    pub message: String,
    pub severity: ViolationSeverity,
    pub context: std::collections::HashMap<String, serde_json::Value>,
}

/// Types of safety events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SafetyEventType {
    ViolationDetected,
    WatchdogFailure,
    EmergencyStop,
    SafetyOverride,
    SystemRecovery,
    ConstraintUpdated,
}

impl std::fmt::Display for SafetyEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SafetyEventType::ViolationDetected => write!(f, "ViolationDetected"),
            SafetyEventType::WatchdogFailure => write!(f, "WatchdogFailure"),
            SafetyEventType::EmergencyStop => write!(f, "EmergencyStop"),
            SafetyEventType::SafetyOverride => write!(f, "SafetyOverride"),
            SafetyEventType::SystemRecovery => write!(f, "SystemRecovery"),
            SafetyEventType::ConstraintUpdated => write!(f, "ConstraintUpdated"),
        }
    }
}

/// Safety configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    pub enabled: bool,
    pub emergency_stop_enabled: bool,
    pub auto_recovery_enabled: bool,
    pub log_violations: bool,
    pub constraints: Vec<SafetyConstraint>,
    pub watchdog_configs: Vec<WatchdogConfig>,
}

/// Watchdog configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchdogConfig {
    pub name: String,
    pub watchdog_type: WatchdogType,
    pub timeout_ms: u64,
    pub enabled: bool,
}

/// Types of watchdogs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WatchdogType {
    Camera,
    CanBus,
    JointController,
    EStop,
    Network,
    Custom(String),
}

/// Safety statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyStats {
    pub total_checks: u64,
    pub violations_detected: u64,
    pub emergency_stops: u64,
    pub watchdog_failures: u64,
    pub average_check_time_ms: f64,
    pub uptime_seconds: u64,
}
