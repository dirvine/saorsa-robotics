#![allow(dead_code)]
//! Intent Parser for Voice Commands
//!
//! This crate provides natural language processing for robot voice commands,
//! converting speech transcriptions into structured robot actions.

mod actions;
mod entities;
mod parser;
mod vla_integration;

pub use actions::{
    ActionType, Direction as ActionDirection, JointCommand, MotionCommand, RobotAction,
    Unit as ActionUnit,
};
pub use entities::{Direction as EntityDirection, Entity, EntityType, Unit as EntityUnit};
pub use parser::{IntentParser, IntentType, ParseResult};
pub use vla_integration::{VlaExecutionResult, VlaPolicyExecutor};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for intent parsing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentConfig {
    /// Confidence threshold for accepting parsed intents
    pub confidence_threshold: f32,
    /// Maximum number of alternative interpretations
    pub max_alternatives: usize,
    /// Supported languages
    pub languages: Vec<String>,
    /// Custom command patterns
    pub custom_patterns: HashMap<String, String>,
}

impl Default for IntentConfig {
    fn default() -> Self {
        Self {
            confidence_threshold: 0.7,
            max_alternatives: 3,
            languages: vec!["en".to_string()],
            custom_patterns: HashMap::new(),
        }
    }
}

/// Initialize the intent parser system
pub fn init() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Initializing Intent Parser system");
    Ok(())
}

/// Create a new intent parser with default configuration
pub fn create_parser() -> Result<IntentParser, Box<dyn std::error::Error>> {
    let config = IntentConfig::default();
    IntentParser::new(config)
}

/// Parse a voice command and return structured actions
pub fn parse_command(text: &str) -> Result<ParseResult, Box<dyn std::error::Error>> {
    let mut parser = create_parser()?;
    parser.parse(text)
}

/// Quick test function for voice commands
pub fn test_command(text: &str) -> Result<String, Box<dyn std::error::Error>> {
    let result = parse_command(text)?;
    Ok(format!(
        "Parsed: {:?} (confidence: {:.2})",
        result.intent, result.confidence
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_commands() {
        let test_cases = vec![
            "raise arm 15 cm",
            "lower the arm by 10 centimeters",
            "move arm up 20 cm",
            "extend arm forward 30 cm",
            "rotate arm left 45 degrees",
            "stop the arm",
            "go to home position",
        ];

        for command in test_cases {
            match parse_command(command) {
                Ok(result) => {
                    println!("✓ '{}' -> {:?}", command, result.intent);
                    assert!(result.confidence > 0.0);
                }
                Err(e) => {
                    println!("✗ '{}' -> Error: {}", command, e);
                }
            }
        }
    }
}
