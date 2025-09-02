//! Named entity recognition for commands

use serde::{Deserialize, Serialize};

/// An extracted entity from a command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// Type of entity
    pub entity_type: EntityType,
    /// Extracted value as string
    pub value: String,
    /// Confidence in this entity extraction
    pub confidence: f32,
}

/// Types of entities that can be extracted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityType {
    /// Numeric measurement (distance, angle, etc.)
    Measurement,
    /// Direction (up, down, left, right, etc.)
    Direction,
    /// Unit of measurement (cm, mm, degrees, etc.)
    Unit,
    /// Joint identifier
    JointId,
    /// Speed specification
    Speed,
    /// Object or target
    Object,
}

/// Directions for motion
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

impl Entity {
    /// Create a new entity
    pub fn new(entity_type: EntityType, value: String, confidence: f32) -> Self {
        Self {
            entity_type,
            value,
            confidence,
        }
    }

    /// Try to parse the value as a number
    pub fn as_number(&self) -> Option<f32> {
        self.value.parse::<f32>().ok()
    }

    /// Get the entity as a direction if applicable
    pub fn as_direction(&self) -> Option<Direction> {
        match self.value.to_lowercase().as_str() {
            "up" => Some(Direction::Up),
            "down" => Some(Direction::Down),
            "left" => Some(Direction::Left),
            "right" => Some(Direction::Right),
            "forward" => Some(Direction::Forward),
            "backward" | "back" => Some(Direction::Backward),
            "clockwise" | "cw" => Some(Direction::Clockwise),
            "counterclockwise" | "ccw" => Some(Direction::CounterClockwise),
            _ => None,
        }
    }

    /// Get the entity as a unit if applicable
    pub fn as_unit(&self) -> Option<Unit> {
        match self.value.to_lowercase().as_str() {
            "mm" | "millimeters" => Some(Unit::Millimeters),
            "cm" | "centimeters" => Some(Unit::Centimeters),
            "m" | "meters" => Some(Unit::Meters),
            "in" | "inches" => Some(Unit::Inches),
            "degrees" | "deg" | "°" => Some(Unit::Degrees),
            "radians" | "rad" => Some(Unit::Radians),
            _ => None,
        }
    }
}

impl Unit {
    /// Convert a value from this unit to meters
    pub fn to_meters(&self, value: f32) -> f32 {
        match self {
            Unit::Millimeters => value * 0.001,
            Unit::Centimeters => value * 0.01,
            Unit::Meters => value,
            Unit::Inches => value * 0.0254,
            Unit::Degrees => value.to_radians(), // For angular units, convert to radians
            Unit::Radians => value,
        }
    }

    /// Convert a value from this unit to radians (for angles)
    pub fn to_radians(&self, value: f32) -> f32 {
        match self {
            Unit::Degrees => value.to_radians(),
            Unit::Radians => value,
            _ => value, // For linear units, return as-is
        }
    }
}

/// Entity extractor for finding entities in text
pub struct EntityExtractor {
    // Could add more sophisticated NLP models here
}

impl EntityExtractor {
    /// Create a new entity extractor
    pub fn new() -> Self {
        Self {}
    }

    /// Extract entities from text
    pub fn extract(&self, text: &str) -> Vec<Entity> {
        let mut entities = Vec::new();

        // Simple regex-based extraction
        self.extract_measurements(text, &mut entities);
        self.extract_directions(text, &mut entities);
        self.extract_units(text, &mut entities);

        entities
    }

    fn extract_measurements(&self, text: &str, entities: &mut Vec<Entity>) {
        use regex::Regex;
        use std::sync::OnceLock;

        static NUMBER_REGEX: OnceLock<Regex> = OnceLock::new();
        let number_regex = NUMBER_REGEX.get_or_init(|| {
            Regex::new(r"(\d+(?:\.\d+)?)").expect("Invalid regex pattern - this is a bug")
        });

        for capture in number_regex.captures_iter(text) {
            if let Some(mat) = capture.get(1) {
                entities.push(Entity::new(
                    EntityType::Measurement,
                    mat.as_str().to_string(),
                    0.9,
                ));
            }
        }
    }

    fn extract_directions(&self, text: &str, entities: &mut Vec<Entity>) {
        let directions = ["up", "down", "left", "right", "forward", "backward", "back"];

        for direction in directions {
            if text.to_lowercase().contains(direction) {
                entities.push(Entity::new(
                    EntityType::Direction,
                    direction.to_string(),
                    0.8,
                ));
            }
        }
    }

    fn extract_units(&self, text: &str, entities: &mut Vec<Entity>) {
        let units = [
            "cm", "mm", "m", "inches", "degrees", "deg", "°", "radians", "rad",
        ];

        for unit in units {
            if text.to_lowercase().contains(unit) {
                entities.push(Entity::new(EntityType::Unit, unit.to_string(), 0.9));
            }
        }
    }
}
