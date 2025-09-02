//! Intervention learning from human corrections
//!
//! This module provides capabilities for learning from human interventions,
//! including constraint learning and preference-based reward modeling.

use crate::{
    DataSample, EventSeverity, InterventionData, LearningEvent, LearningEventType, RewardSignal,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for intervention learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterventionLearningConfig {
    /// Learning rate for intervention model
    pub learning_rate: f64,
    /// Batch size for training
    pub batch_size: usize,
    /// Number of training epochs
    pub num_epochs: usize,
    /// Intervention model type
    pub model_type: InterventionModelType,
    /// Whether to use constraint learning
    pub use_constraint_learning: bool,
    /// Constraint violation penalty
    pub constraint_penalty: f32,
    /// Intervention confidence threshold
    pub confidence_threshold: f32,
}

/// Types of intervention learning models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterventionModelType {
    /// Simple correction model
    CorrectionBased,
    /// Preference-based learning
    PreferenceBased,
    /// Constraint-based learning
    ConstraintBased,
    /// Hybrid approach
    Hybrid,
}

/// Intervention learning model
pub struct InterventionLearner {
    config: InterventionLearningConfig,
    intervention_history: Vec<InterventionData>,
    learned_constraints: Vec<LearnedConstraint>,
    correction_model: Option<CorrectionModel>,
    is_trained: bool,
}

impl InterventionLearner {
    /// Create a new intervention learner
    pub fn new(config: InterventionLearningConfig) -> Self {
        Self {
            config,
            intervention_history: Vec::new(),
            learned_constraints: Vec::new(),
            correction_model: None,
            is_trained: false,
        }
    }

    /// Record a human intervention
    pub fn record_intervention(
        &mut self,
        intervention: InterventionData,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.intervention_history.push(intervention.clone());

        // Record learning event
        let event = LearningEvent {
            timestamp: intervention.timestamp,
            event_type: LearningEventType::InterventionOccurred,
            data: {
                let mut data = HashMap::new();
                data.insert(
                    "reason".to_string(),
                    serde_json::Value::String(intervention.reason.clone()),
                );
                data.insert(
                    "intervention_type".to_string(),
                    serde_json::to_value(&intervention.intervention_type)?,
                );
                data
            },
            model_version: None,
            severity: EventSeverity::Info,
        };

        crate::record_event(event)?;

        tracing::info!("Recorded intervention: {}", intervention.reason);
        Ok(())
    }

    /// Learn from collected interventions
    pub fn learn_from_interventions(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.intervention_history.is_empty() {
            return Err("No interventions available for learning".into());
        }

        tracing::info!(
            "Learning from {} interventions",
            self.intervention_history.len()
        );

        match self.config.model_type {
            InterventionModelType::CorrectionBased => self.learn_correction_model()?,
            InterventionModelType::PreferenceBased => self.learn_preference_model()?,
            InterventionModelType::ConstraintBased => self.learn_constraint_model()?,
            InterventionModelType::Hybrid => self.learn_hybrid_model()?,
        }

        if self.config.use_constraint_learning {
            self.extract_constraints()?;
        }

        self.is_trained = true;

        tracing::info!("Intervention learning completed");
        Ok(())
    }

    /// Predict corrected action for given observation and original action
    pub fn predict_correction(
        &self,
        observation: &vla_policy::Observation,
        original_action: &vla_policy::Action,
    ) -> Result<Option<vla_policy::Action>, Box<dyn std::error::Error>> {
        if !self.is_trained {
            return Ok(None);
        }

        // Check constraints first
        if self.config.use_constraint_learning {
            if let Some(constraint) = self.check_constraint_violation(observation, original_action)
            {
                tracing::warn!("Constraint violation detected: {}", constraint.description);
                return Ok(Some(
                    self.apply_constraint_correction(original_action, &constraint),
                ));
            }
        }

        // Use correction model if available
        if let Some(ref model) = self.correction_model {
            let corrected = model.predict_correction(observation, original_action)?;
            return Ok(Some(corrected));
        }

        Ok(None)
    }

    /// Get learned constraints
    pub fn get_learned_constraints(&self) -> &[LearnedConstraint] {
        &self.learned_constraints
    }

    /// Get intervention statistics
    pub fn get_intervention_stats(&self) -> InterventionStats {
        let mut type_counts = HashMap::new();
        let mut reason_counts = HashMap::new();

        for intervention in &self.intervention_history {
            *type_counts
                .entry(intervention.intervention_type.clone())
                .or_insert(0) += 1;
            *reason_counts
                .entry(intervention.reason.clone())
                .or_insert(0) += 1;
        }

        InterventionStats {
            total_interventions: self.intervention_history.len(),
            type_distribution: type_counts,
            reason_distribution: reason_counts,
            constraints_learned: self.learned_constraints.len(),
        }
    }

    fn learn_correction_model(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let model = CorrectionModel::train(&self.intervention_history, &self.config)?;
        self.correction_model = Some(model);
        Ok(())
    }

    fn learn_preference_model(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Implement preference-based learning
        // This would learn from comparisons between original and corrected actions
        tracing::info!("Learning preference-based intervention model");
        Ok(())
    }

    fn learn_constraint_model(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Implement constraint-based learning
        tracing::info!("Learning constraint-based intervention model");
        Ok(())
    }

    fn learn_hybrid_model(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Combine multiple learning approaches
        tracing::info!("Learning hybrid intervention model");
        self.learn_correction_model()?;
        self.learn_preference_model()?;
        Ok(())
    }

    fn extract_constraints(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut constraints = Vec::new();

        // Analyze intervention patterns to extract constraints
        let mut action_patterns = HashMap::new();

        for intervention in &self.intervention_history {
            let key = self.get_action_pattern_key(&intervention.original_action);
            let entry = action_patterns.entry(key).or_insert_with(Vec::new);
            entry.push(intervention.clone());
        }

        // Create constraints from frequent intervention patterns
        for (pattern, interventions) in action_patterns {
            if interventions.len() >= 3 {
                // Require at least 3 interventions for same pattern
                let constraint = self.create_constraint_from_pattern(&pattern, &interventions);
                constraints.push(constraint);
            }
        }

        self.learned_constraints = constraints;
        tracing::info!(
            "Extracted {} constraints from intervention patterns",
            self.learned_constraints.len()
        );

        Ok(())
    }

    fn check_constraint_violation(
        &self,
        observation: &vla_policy::Observation,
        action: &vla_policy::Action,
    ) -> Option<&LearnedConstraint> {
        for constraint in &self.learned_constraints {
            if constraint.is_violated(observation, action) {
                return Some(constraint);
            }
        }
        None
    }

    fn apply_constraint_correction(
        &self,
        original_action: &vla_policy::Action,
        constraint: &LearnedConstraint,
    ) -> vla_policy::Action {
        // Apply constraint-based correction
        match constraint.constraint_type {
            ConstraintType::JointLimit => {
                self.apply_joint_limit_correction(original_action, constraint)
            }
            ConstraintType::WorkspaceLimit => {
                self.apply_workspace_limit_correction(original_action, constraint)
            }
            ConstraintType::VelocityLimit => {
                self.apply_velocity_limit_correction(original_action, constraint)
            }
            ConstraintType::SafetyZone => {
                self.apply_safety_zone_correction(original_action, constraint)
            }
        }
    }

    fn get_action_pattern_key(&self, action: &vla_policy::Action) -> String {
        // Create a simplified key for action pattern matching
        // In a real implementation, this would analyze action structure
        format!("action_type_{}", action.get_action_type())
    }

    fn create_constraint_from_pattern(
        &self,
        pattern: &str,
        interventions: &[InterventionData],
    ) -> LearnedConstraint {
        // Analyze interventions to determine constraint type and parameters
        let most_common_reason = self.find_most_common_reason(interventions);

        LearnedConstraint {
            id: uuid::Uuid::new_v4().to_string(),
            description: format!(
                "Constraint learned from {} interventions: {}",
                interventions.len(),
                most_common_reason
            ),
            constraint_type: self.infer_constraint_type(&most_common_reason),
            parameters: self.extract_constraint_parameters(interventions),
            confidence: interventions.len() as f32 / self.intervention_history.len() as f32,
            created_from_interventions: interventions.len(),
            last_updated: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64(),
        }
    }

    fn find_most_common_reason(&self, interventions: &[InterventionData]) -> String {
        let mut reason_counts = HashMap::new();
        for intervention in interventions {
            *reason_counts
                .entry(intervention.reason.clone())
                .or_insert(0) += 1;
        }

        reason_counts
            .into_iter()
            .max_by_key(|&(_, count)| count)
            .map(|(reason, _)| reason)
            .unwrap_or_else(|| "Unknown".to_string())
    }

    fn infer_constraint_type(&self, reason: &str) -> ConstraintType {
        if reason.to_lowercase().contains("joint") || reason.to_lowercase().contains("limit") {
            ConstraintType::JointLimit
        } else if reason.to_lowercase().contains("workspace")
            || reason.to_lowercase().contains("boundary")
        {
            ConstraintType::WorkspaceLimit
        } else if reason.to_lowercase().contains("velocity")
            || reason.to_lowercase().contains("speed")
        {
            ConstraintType::VelocityLimit
        } else if reason.to_lowercase().contains("safety")
            || reason.to_lowercase().contains("danger")
        {
            ConstraintType::SafetyZone
        } else {
            ConstraintType::SafetyZone // Default to safety zone
        }
    }

    fn extract_constraint_parameters(
        &self,
        interventions: &[InterventionData],
    ) -> HashMap<String, serde_json::Value> {
        // Extract parameters from intervention data
        let mut params = HashMap::new();

        // Simple parameter extraction - in practice this would be more sophisticated
        if let Some(first) = interventions.first() {
            params.insert(
                "original_action_sample".to_string(),
                serde_json::to_value(&first.original_action).unwrap_or_default(),
            );
            params.insert(
                "corrected_action_sample".to_string(),
                serde_json::to_value(&first.corrected_action).unwrap_or_default(),
            );
        }

        params.insert(
            "intervention_count".to_string(),
            serde_json::Value::Number(interventions.len().into()),
        );

        params
    }

    // Placeholder correction methods - would be implemented based on specific action types
    fn apply_joint_limit_correction(
        &self,
        _action: &vla_policy::Action,
        _constraint: &LearnedConstraint,
    ) -> vla_policy::Action {
        // Apply joint limit corrections
        todo!("Implement joint limit correction")
    }

    fn apply_workspace_limit_correction(
        &self,
        _action: &vla_policy::Action,
        _constraint: &LearnedConstraint,
    ) -> vla_policy::Action {
        // Apply workspace limit corrections
        todo!("Implement workspace limit correction")
    }

    fn apply_velocity_limit_correction(
        &self,
        _action: &vla_policy::Action,
        _constraint: &LearnedConstraint,
    ) -> vla_policy::Action {
        // Apply velocity limit corrections
        todo!("Implement velocity limit correction")
    }

    fn apply_safety_zone_correction(
        &self,
        _action: &vla_policy::Action,
        _constraint: &LearnedConstraint,
    ) -> vla_policy::Action {
        // Apply safety zone corrections
        todo!("Implement safety zone correction")
    }
}

/// Simple correction model
struct CorrectionModel {
    corrections: HashMap<String, vla_policy::Action>,
}

impl CorrectionModel {
    fn train(
        interventions: &[InterventionData],
        config: &InterventionLearningConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut corrections = HashMap::new();

        for intervention in interventions {
            let key = Self::get_correction_key(&intervention.original_action);
            corrections.insert(key, intervention.corrected_action.clone());
        }

        Ok(Self { corrections })
    }

    fn predict_correction(
        &self,
        observation: &vla_policy::Observation,
        original_action: &vla_policy::Action,
    ) -> Result<vla_policy::Action, Box<dyn std::error::Error>> {
        let key = Self::get_correction_key(original_action);

        if let Some(corrected) = self.corrections.get(&key) {
            Ok(corrected.clone())
        } else {
            Err("No correction available for this action pattern".into())
        }
    }

    fn get_correction_key(action: &vla_policy::Action) -> String {
        // Create a key for action pattern matching
        format!("correction_{}", action.get_action_type())
    }
}

/// Learned constraint from interventions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedConstraint {
    /// Unique constraint identifier
    pub id: String,
    /// Human-readable description
    pub description: String,
    /// Type of constraint
    pub constraint_type: ConstraintType,
    /// Constraint parameters
    pub parameters: HashMap<String, serde_json::Value>,
    /// Confidence in this constraint (0.0 to 1.0)
    pub confidence: f32,
    /// Number of interventions this constraint was learned from
    pub created_from_interventions: usize,
    /// Last update timestamp
    pub last_updated: f64,
}

impl LearnedConstraint {
    /// Check if this constraint is violated by the given observation-action pair
    pub fn is_violated(
        &self,
        observation: &vla_policy::Observation,
        action: &vla_policy::Action,
    ) -> bool {
        // Placeholder constraint checking logic
        // In practice, this would implement specific constraint validation
        match self.constraint_type {
            ConstraintType::JointLimit => self.check_joint_limits(action),
            ConstraintType::WorkspaceLimit => self.check_workspace_limits(observation, action),
            ConstraintType::VelocityLimit => self.check_velocity_limits(action),
            ConstraintType::SafetyZone => self.check_safety_zone(observation, action),
        }
    }

    fn check_joint_limits(&self, _action: &vla_policy::Action) -> bool {
        // Check if action violates joint limits
        false // Placeholder
    }

    fn check_workspace_limits(
        &self,
        _observation: &vla_policy::Observation,
        _action: &vla_policy::Action,
    ) -> bool {
        // Check if action would move end-effector outside workspace
        false // Placeholder
    }

    fn check_velocity_limits(&self, _action: &vla_policy::Action) -> bool {
        // Check if action exceeds velocity limits
        false // Placeholder
    }

    fn check_safety_zone(
        &self,
        _observation: &vla_policy::Observation,
        _action: &vla_policy::Action,
    ) -> bool {
        // Check if action violates safety zones
        false // Placeholder
    }
}

/// Types of learned constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstraintType {
    /// Joint angle/position limits
    JointLimit,
    /// Workspace boundaries
    WorkspaceLimit,
    /// Velocity/acceleration limits
    VelocityLimit,
    /// Safety exclusion zones
    SafetyZone,
}

/// Statistics about interventions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterventionStats {
    /// Total number of interventions
    pub total_interventions: usize,
    /// Distribution of intervention types
    pub type_distribution: HashMap<crate::InterventionType, usize>,
    /// Distribution of intervention reasons
    pub reason_distribution: HashMap<String, usize>,
    /// Number of constraints learned
    pub constraints_learned: usize,
}

/// Constraint learning from interventions
pub struct ConstraintLearner {
    constraints: Vec<LearnedConstraint>,
    learning_config: ConstraintLearningConfig,
}

impl ConstraintLearner {
    /// Create a new constraint learner
    pub fn new(config: ConstraintLearningConfig) -> Self {
        Self {
            constraints: Vec::new(),
            learning_config: config,
        }
    }

    /// Learn constraints from intervention data
    pub fn learn_constraints(
        &mut self,
        interventions: &[InterventionData],
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Analyze intervention patterns
        let patterns = self.analyze_intervention_patterns(interventions);

        // Extract constraints from patterns
        for pattern in patterns {
            if pattern.frequency >= self.learning_config.min_pattern_frequency {
                let constraint = self.create_constraint_from_pattern(&pattern);
                self.constraints.push(constraint);
            }
        }

        tracing::info!(
            "Learned {} constraints from {} interventions",
            self.constraints.len(),
            interventions.len()
        );

        Ok(())
    }

    /// Get learned constraints
    pub fn get_constraints(&self) -> &[LearnedConstraint] {
        &self.constraints
    }

    fn analyze_intervention_patterns(
        &self,
        interventions: &[InterventionData],
    ) -> Vec<InterventionPattern> {
        let mut patterns = HashMap::new();

        for intervention in interventions {
            let pattern_key = self.get_pattern_key(intervention);
            let pattern = patterns
                .entry(pattern_key)
                .or_insert_with(|| InterventionPattern {
                    key: pattern_key.clone(),
                    interventions: Vec::new(),
                    frequency: 0,
                    common_reason: String::new(),
                });

            pattern.interventions.push(intervention.clone());
            pattern.frequency += 1;
        }

        // Update common reasons
        for pattern in patterns.values_mut() {
            pattern.common_reason = self.find_common_reason(&pattern.interventions);
        }

        patterns.into_iter().map(|(_, p)| p).collect()
    }

    fn get_pattern_key(&self, intervention: &InterventionData) -> String {
        // Create a pattern key based on intervention characteristics
        format!(
            "{}_{}",
            intervention.intervention_type.as_str(),
            intervention.reason.to_lowercase().replace(" ", "_")
        )
    }

    fn find_common_reason(&self, interventions: &[InterventionData]) -> String {
        let mut reason_counts = HashMap::new();
        for intervention in interventions {
            *reason_counts
                .entry(intervention.reason.clone())
                .or_insert(0) += 1;
        }

        reason_counts
            .into_iter()
            .max_by_key(|&(_, count)| count)
            .map(|(reason, _)| reason)
            .unwrap_or_else(|| "Unknown".to_string())
    }

    fn create_constraint_from_pattern(&self, pattern: &InterventionPattern) -> LearnedConstraint {
        LearnedConstraint {
            id: uuid::Uuid::new_v4().to_string(),
            description: format!("Learned constraint: {}", pattern.common_reason),
            constraint_type: self.infer_constraint_type(&pattern.common_reason),
            parameters: HashMap::new(),
            confidence: pattern.frequency as f32 / 10.0, // Simple confidence calculation
            created_from_interventions: pattern.frequency,
            last_updated: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64(),
        }
    }

    fn infer_constraint_type(&self, reason: &str) -> ConstraintType {
        if reason.to_lowercase().contains("joint") {
            ConstraintType::JointLimit
        } else if reason.to_lowercase().contains("workspace") {
            ConstraintType::WorkspaceLimit
        } else if reason.to_lowercase().contains("velocity")
            || reason.to_lowercase().contains("speed")
        {
            ConstraintType::VelocityLimit
        } else {
            ConstraintType::SafetyZone
        }
    }
}

/// Configuration for constraint learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintLearningConfig {
    /// Minimum frequency for pattern to be considered
    pub min_pattern_frequency: usize,
    /// Maximum number of constraints to learn
    pub max_constraints: usize,
    /// Confidence threshold for constraints
    pub confidence_threshold: f32,
}

/// Pattern of interventions
struct InterventionPattern {
    key: String,
    interventions: Vec<InterventionData>,
    frequency: usize,
    common_reason: String,
}
