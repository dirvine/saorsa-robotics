//! Command parser for natural language robot instructions

use crate::actions::{Direction, Unit};
use crate::{
    ActionType, Entity, EntityType, IntentConfig, JointCommand, MotionCommand, RobotAction,
};

/// Result of parsing a command
#[derive(Debug, Clone)]
pub struct ParseResult {
    /// The parsed intent
    pub intent: IntentType,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Original text
    pub text: String,
    /// Extracted entities
    pub entities: Vec<Entity>,
}

/// Types of intents that can be parsed
#[derive(Debug, Clone)]
pub enum IntentType {
    /// Motion command (raise, lower, extend, etc.)
    Motion(MotionCommand),
    /// Joint-specific command
    Joint(JointCommand),
    /// Stop all motion
    Stop,
    /// Go to home position
    Home,
}
use regex::Regex;
use std::collections::HashMap;

/// Main intent parser
pub struct IntentParser {
    config: IntentConfig,
    patterns: HashMap<String, Regex>,
}

impl IntentParser {
    /// Create a new intent parser
    pub fn new(config: IntentConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let mut patterns = HashMap::new();

        // Motion commands
        patterns.insert("raise_arm".to_string(), Regex::new(r"(?i)(raise|lift|move up|go up).*arm.*(\d+(?:\.\d+)?)\s*(cm|centimeters?|mm|millimeters?|inches?|in)")?);
        patterns.insert("lower_arm".to_string(), Regex::new(r"(?i)(lower|drop|move down|go down).*arm.*(\d+(?:\.\d+)?)\s*(cm|centimeters?|mm|millimeters?|inches?|in)")?);
        patterns.insert("extend_arm".to_string(), Regex::new(r"(?i)(extend|move forward|go forward).*arm.*(\d+(?:\.\d+)?)\s*(cm|centimeters?|mm|millimeters?|inches?|in)")?);
        patterns.insert("retract_arm".to_string(), Regex::new(r"(?i)(retract|move back|go back).*arm.*(\d+(?:\.\d+)?)\s*(cm|centimeters?|mm|millimeters?|inches?|in)")?);

        // Rotation commands
        patterns.insert(
            "rotate_left".to_string(),
            Regex::new(r"(?i)(rotate|turn).*left.*(\d+(?:\.\d+)?)\s*(degrees?|deg|°)")?,
        );
        patterns.insert(
            "rotate_right".to_string(),
            Regex::new(r"(?i)(rotate|turn).*right.*(\d+(?:\.\d+)?)\s*(degrees?|deg|°)")?,
        );

        // Stop commands
        patterns.insert(
            "stop".to_string(),
            Regex::new(r"(?i)(stop|halt|freeze|emergency stop)")?,
        );

        // Home position
        patterns.insert(
            "home".to_string(),
            Regex::new(r"(?i)(go to|move to).*home.*position")?,
        );

        // Joint-specific commands
        patterns.insert(
            "joint_move".to_string(),
            Regex::new(
                r"(?i)move.*joint.*(\d+).*to.*(\d+(?:\.\d+)?)\s*(degrees?|deg|°|radians?|rad)",
            )?,
        );

        Ok(Self { config, patterns })
    }

    /// Parse a text command into structured intent
    pub fn parse(&mut self, text: &str) -> Result<ParseResult, Box<dyn std::error::Error>> {
        let text = text.trim().to_lowercase();

        // Try each pattern
        for (pattern_name, regex) in &self.patterns {
            if let Some(captures) = regex.captures(&text) {
                let intent = self.build_intent(pattern_name, &captures)?;
                let confidence = self.calculate_confidence(pattern_name, &text);

                if confidence >= self.config.confidence_threshold {
                    return Ok(ParseResult {
                        intent,
                        confidence,
                        text: text.clone(),
                        entities: self.extract_entities(&captures),
                    });
                }
            }
        }

        // Fallback: try to extract basic motion commands
        if let Some(intent) = self.parse_fallback(&text) {
            return Ok(ParseResult {
                intent,
                confidence: 0.5, // Lower confidence for fallback
                text: text.clone(),
                entities: Vec::new(),
            });
        }

        Err(format!("Could not parse command: {}", text).into())
    }

    fn build_intent(
        &self,
        pattern_name: &str,
        captures: &regex::Captures,
    ) -> Result<IntentType, Box<dyn std::error::Error>> {
        match pattern_name {
            "raise_arm" | "lower_arm" | "extend_arm" | "retract_arm" => {
                let distance = captures
                    .get(2)
                    .and_then(|m| m.as_str().parse::<f32>().ok())
                    .unwrap_or(0.0);

                let unit = self.parse_unit(captures.get(3).map(|m| m.as_str()).unwrap_or("cm"));
                let direction = self.parse_direction(pattern_name);

                let motion = MotionCommand {
                    direction,
                    distance,
                    unit,
                    speed: None, // Default speed
                };

                Ok(IntentType::Motion(motion))
            }

            "rotate_left" | "rotate_right" => {
                let angle = captures
                    .get(2)
                    .and_then(|m| m.as_str().parse::<f32>().ok())
                    .unwrap_or(0.0);

                let direction = if pattern_name == "rotate_left" {
                    Direction::Left
                } else {
                    Direction::Right
                };

                let motion = MotionCommand {
                    direction,
                    distance: angle,
                    unit: Unit::Degrees,
                    speed: None,
                };

                Ok(IntentType::Motion(motion))
            }

            "stop" => Ok(IntentType::Stop),

            "home" => Ok(IntentType::Home),

            "joint_move" => {
                let joint_id = captures
                    .get(1)
                    .and_then(|m| m.as_str().parse::<u32>().ok())
                    .unwrap_or(0);

                let position = captures
                    .get(2)
                    .and_then(|m| m.as_str().parse::<f32>().ok())
                    .unwrap_or(0.0);

                let unit =
                    self.parse_unit(captures.get(3).map(|m| m.as_str()).unwrap_or("degrees"));

                let joint_cmd = JointCommand {
                    joint_id,
                    position,
                    unit,
                    speed: None,
                };

                Ok(IntentType::Joint(joint_cmd))
            }

            _ => Err(format!("Unknown pattern: {}", pattern_name).into()),
        }
    }

    fn parse_fallback(&self, text: &str) -> Option<IntentType> {
        // Simple keyword-based fallback parsing
        if text.contains("stop") || text.contains("halt") {
            Some(IntentType::Stop)
        } else if text.contains("home") {
            Some(IntentType::Home)
        } else {
            None
        }
    }

    fn parse_unit(&self, unit_str: &str) -> Unit {
        match unit_str.to_lowercase().as_str() {
            "mm" | "millimeters" => Unit::Millimeters,
            "in" | "inches" => Unit::Inches,
            "degrees" | "deg" | "°" => Unit::Degrees,
            "radians" | "rad" => Unit::Radians,
            _ => Unit::Centimeters,
        }
    }

    fn parse_direction(&self, pattern_name: &str) -> Direction {
        match pattern_name {
            "raise_arm" => Direction::Up,
            "lower_arm" => Direction::Down,
            "extend_arm" => Direction::Forward,
            "retract_arm" => Direction::Backward,
            _ => Direction::Forward,
        }
    }

    fn calculate_confidence(&self, pattern_name: &str, text: &str) -> f32 {
        // Simple confidence calculation based on pattern specificity
        let base_confidence = match pattern_name {
            "stop" | "home" => 0.9,
            "raise_arm" | "lower_arm" | "extend_arm" | "retract_arm" => 0.8,
            "rotate_left" | "rotate_right" => 0.8,
            "joint_move" => 0.7,
            _ => 0.5,
        };

        // Boost confidence if text is short and direct
        if text.len() < 20 {
            base_confidence + 0.1
        } else {
            base_confidence
        }
    }

    fn extract_entities(&self, captures: &regex::Captures) -> Vec<Entity> {
        let mut entities = Vec::new();

        // Extract numbers as measurements
        for (i, capture) in captures.iter().enumerate() {
            if let Some(mat) = capture {
                if i > 0 {
                    // Skip the full match
                    if let Ok(_value) = mat.as_str().parse::<f32>() {
                        entities.push(Entity {
                            entity_type: EntityType::Measurement,
                            value: mat.as_str().to_string(),
                            confidence: 0.9,
                        });
                    }
                }
            }
        }

        entities
    }
}
