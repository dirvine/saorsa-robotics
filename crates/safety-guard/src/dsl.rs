use evalexpr::*;
use std::collections::HashMap;

/// Safety constraints DSL for expressing complex safety rules
pub struct SafetyDSL {
    expressions: HashMap<String, ConstraintExpression>,
}

impl SafetyDSL {
    pub fn new() -> Self {
        Self {
            expressions: HashMap::new(),
        }
    }

    /// Add a constraint expression
    pub fn add_expression(
        &mut self,
        name: &str,
        expression: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let parsed = evalexpr::build_operator_tree(expression)?;
        let constraint_expr = ConstraintExpression {
            name: name.to_string(),
            expression: expression.to_string(),
            compiled: parsed,
        };

        self.expressions.insert(name.to_string(), constraint_expr);
        Ok(())
    }

    /// Evaluate a constraint expression
    pub fn evaluate(
        &self,
        name: &str,
        context: &HashMapContext,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        if let Some(expr) = self.expressions.get(name) {
            let result = expr.compiled.eval_with_context(context)?;
            match result {
                Value::Boolean(b) => Ok(b),
                _ => Err(format!("Expression '{}' did not evaluate to boolean", name).into()),
            }
        } else {
            Err(format!("Expression '{}' not found", name).into())
        }
    }

    /// Get all expression names
    pub fn get_expressions(&self) -> Vec<String> {
        self.expressions.keys().cloned().collect()
    }

    /// Remove an expression
    pub fn remove_expression(&mut self, name: &str) -> bool {
        self.expressions.remove(name).is_some()
    }

    /// Validate an expression syntax
    pub fn validate_expression(expression: &str) -> Result<(), Box<dyn std::error::Error>> {
        evalexpr::build_operator_tree(expression)?;
        Ok(())
    }
}

/// Compiled constraint expression
#[derive(Debug, Clone)]
pub struct ConstraintExpression {
    pub name: String,
    pub expression: String,
    pub compiled: Node,
}

/// Predefined safety expressions
#[allow(dead_code)]
pub mod expressions {
    pub const JOINT_POSITION_LIMIT: &str =
        "joint_0 >= -3.14 && joint_0 <= 3.14 && joint_1 >= -1.57 && joint_1 <= 1.57";
    pub const WORKSPACE_LIMIT: &str =
        "ee_0 >= -1.0 && ee_0 <= 1.0 && ee_1 >= -1.0 && ee_1 <= 1.0 && ee_2 >= 0.0 && ee_2 <= 1.5";
    pub const VELOCITY_LIMIT: &str = "joint_0_vel <= 2.0 && joint_1_vel <= 2.0";
    pub const COLLISION_AVOIDANCE: &str = "ee_2 > 0.1"; // Keep end-effector above ground
}

/// Helper functions for creating common safety expressions
pub mod helpers {

    pub fn joint_position_limit(joint_indices: &[usize], min: f32, max: f32) -> String {
        let mut conditions = Vec::new();
        for &idx in joint_indices {
            conditions.push(format!(
                "joint_{} >= {} && joint_{} <= {}",
                idx, min, idx, max
            ));
        }
        conditions.join(" && ")
    }

    pub fn workspace_box(
        min_x: f32,
        max_x: f32,
        min_y: f32,
        max_y: f32,
        min_z: f32,
        max_z: f32,
    ) -> String {
        format!(
            "ee_0 >= {} && ee_0 <= {} && ee_1 >= {} && ee_1 <= {} && ee_2 >= {} && ee_2 <= {}",
            min_x, max_x, min_y, max_y, min_z, max_z
        )
    }

    pub fn velocity_limit(joint_indices: &[usize], max_vel: f32) -> String {
        let mut conditions = Vec::new();
        for &idx in joint_indices {
            conditions.push(format!("joint_{}_vel <= {}", idx, max_vel));
        }
        conditions.join(" && ")
    }

    pub fn acceleration_limit(joint_indices: &[usize], max_acc: f32) -> String {
        let mut conditions = Vec::new();
        for &idx in joint_indices {
            conditions.push(format!("joint_{}_acc <= {}", idx, max_acc));
        }
        conditions.join(" && ")
    }

    pub fn collision_sphere(center: [f32; 3], radius: f32) -> String {
        format!(
            "sqrt((ee_0 - {})^2 + (ee_1 - {})^2 + (ee_2 - {})^2) > {}",
            center[0], center[1], center[2], radius
        )
    }
}

/// Example usage and testing
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_expression() {
        let mut dsl = SafetyDSL::new();
        dsl.add_expression("joint_limit", "joint_0 >= -1.57 && joint_0 <= 1.57")
            .unwrap();

        let mut context = HashMapContext::new();
        context
            .set_value("joint_0".to_string(), Value::Float(0.5))
            .unwrap();

        let result = dsl.evaluate("joint_limit", &context).unwrap();
        assert!(result);
    }

    #[test]
    fn test_workspace_limit() {
        let mut dsl = SafetyDSL::new();
        dsl.add_expression("workspace", expressions::WORKSPACE_LIMIT)
            .unwrap();

        let mut context = HashMapContext::new();
        context
            .set_value("ee_0".to_string(), Value::Float(0.5))
            .unwrap();
        context
            .set_value("ee_1".to_string(), Value::Float(0.0))
            .unwrap();
        context
            .set_value("ee_2".to_string(), Value::Float(0.8))
            .unwrap();

        let result = dsl.evaluate("workspace", &context).unwrap();
        assert!(result);
    }

    #[test]
    fn test_helpers() {
        let joint_expr = helpers::joint_position_limit(&[0, 1], -1.57, 1.57);
        assert_eq!(
            joint_expr,
            "joint_0 >= -1.57 && joint_0 <= 1.57 && joint_1 >= -1.57 && joint_1 <= 1.57"
        );

        let workspace_expr = helpers::workspace_box(-1.0, 1.0, -1.0, 1.0, 0.0, 1.5);
        assert!(workspace_expr.contains("ee_0 >= -1"));
    }
}
