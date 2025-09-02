//! Robot action definitions

use serde::{Deserialize, Serialize};

/// A robot action that can be executed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobotAction {
    /// Type of action
    pub action_type: ActionType,
    /// Priority level (higher = more important)
    pub priority: u8,
    /// Whether this action requires confirmation
    pub requires_confirmation: bool,
    /// Safety constraints for this action
    pub safety_constraints: Vec<SafetyConstraint>,
    /// Execution timeout in seconds
    pub timeout_seconds: Option<f32>,
}

/// Types of robot actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    /// Motion command in Cartesian space
    Motion(MotionCommand),
    /// Joint-specific command
    Joint(JointCommand),
    /// Stop all motion immediately
    Stop,
    /// Move to home position
    Home,
    /// Complex skill execution
    Skill(String),
}

/// Motion command in Cartesian space
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionCommand {
    /// Direction of motion
    pub direction: Direction,
    /// Distance/angle to move
    pub distance: f32,
    /// Unit of measurement
    pub unit: Unit,
    /// Speed (optional, uses default if None)
    pub speed: Option<f32>,
}

/// Joint-specific command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JointCommand {
    /// Joint ID (0-based)
    pub joint_id: u32,
    /// Target position
    pub position: f32,
    /// Unit of position
    pub unit: Unit,
    /// Speed (optional)
    pub speed: Option<f32>,
}

/// Directions for motion commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
    Forward,
    Backward,
    Clockwise,
    CounterClockwise,
}

impl Direction {
    /// Convert direction to a vector (x, y, z)
    pub fn to_vector(&self, magnitude: f32) -> (f32, f32, f32) {
        match self {
            Direction::Up => (0.0, 0.0, magnitude),
            Direction::Down => (0.0, 0.0, -magnitude),
            Direction::Left => (-magnitude, 0.0, 0.0),
            Direction::Right => (magnitude, 0.0, 0.0),
            Direction::Forward => (0.0, magnitude, 0.0),
            Direction::Backward => (0.0, -magnitude, 0.0),
            Direction::Clockwise => (0.0, 0.0, 0.0), // Rotation around Z
            Direction::CounterClockwise => (0.0, 0.0, 0.0), // Rotation around Z
        }
    }
}

/// Units of measurement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Unit {
    Millimeters,
    Centimeters,
    Meters,
    Inches,
    Degrees,
    Radians,
}

/// Safety constraints for actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConstraint {
    /// Type of constraint
    pub constraint_type: ConstraintType,
    /// Constraint value
    pub value: f32,
    /// Unit for the constraint value
    pub unit: Unit,
}

/// Types of safety constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstraintType {
    /// Maximum velocity
    MaxVelocity,
    /// Maximum acceleration
    MaxAcceleration,
    /// Maximum joint position
    MaxJointPosition,
    /// Minimum joint position
    MinJointPosition,
    /// Workspace boundary
    WorkspaceBoundary,
    /// Collision avoidance
    CollisionAvoidance,
}

impl RobotAction {
    /// Create a motion action
    pub fn motion(direction: Direction, distance: f32, unit: Unit) -> Self {
        Self {
            action_type: ActionType::Motion(MotionCommand {
                direction,
                distance,
                unit,
                speed: None,
            }),
            priority: 1,
            requires_confirmation: false,
            safety_constraints: Vec::new(),
            timeout_seconds: Some(10.0),
        }
    }

    /// Create a stop action
    pub fn stop() -> Self {
        Self {
            action_type: ActionType::Stop,
            priority: 10, // High priority
            requires_confirmation: false,
            safety_constraints: Vec::new(),
            timeout_seconds: Some(1.0),
        }
    }

    /// Create a home action
    pub fn home() -> Self {
        Self {
            action_type: ActionType::Home,
            priority: 1,
            requires_confirmation: true, // Require confirmation for home moves
            safety_constraints: Vec::new(),
            timeout_seconds: Some(30.0),
        }
    }

    /// Add a safety constraint
    pub fn with_constraint(mut self, constraint: SafetyConstraint) -> Self {
        self.safety_constraints.push(constraint);
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Require confirmation
    pub fn with_confirmation(mut self, required: bool) -> Self {
        self.requires_confirmation = required;
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, seconds: f32) -> Self {
        self.timeout_seconds = Some(seconds);
        self
    }
}

impl MotionCommand {
    /// Convert motion command to VLA policy action
    pub fn to_vla_action(&self) -> Result<vla_policy::Action, Box<dyn std::error::Error>> {
        // Convert direction and distance to end-effector delta
        let (_dx, _dy, _dz) = match self.direction {
            Direction::Up => (0.0, 0.0, self.distance),
            Direction::Down => (0.0, 0.0, -self.distance),
            Direction::Left => (-self.distance, 0.0, 0.0),
            Direction::Right => (self.distance, 0.0, 0.0),
            Direction::Forward => (0.0, self.distance, 0.0),
            Direction::Backward => (0.0, -self.distance, 0.0),
            Direction::Clockwise => todo!("Implement rotation"),
            Direction::CounterClockwise => todo!("Implement rotation"),
        };

        // Convert units to meters
        let _scale = match self.unit {
            Unit::Millimeters => 0.001,
            Unit::Centimeters => 0.01,
            Unit::Meters => 1.0,
            Unit::Inches => 0.0254,
            Unit::Degrees | Unit::Radians => {
                return Err("Rotation commands should use Joint action type".into());
            }
        };

        // Create VLA action with end-effector delta
        // This is a placeholder - actual implementation would depend on VLA policy structure
        Ok(vla_policy::Action::default())
    }
}

impl JointCommand {
    /// Convert joint command to device registry commands
    pub fn to_device_commands(
        &self,
    ) -> Result<Vec<device_registry::JointCommand>, Box<dyn std::error::Error>> {
        let position_rad = match self.unit {
            Unit::Degrees => self.position.to_radians(),
            Unit::Radians => self.position,
            _ => return Err("Joint positions must be in degrees or radians".into()),
        };

        let cmd = device_registry::JointCommand::Position(position_rad);
        Ok(vec![cmd])
    }
}
