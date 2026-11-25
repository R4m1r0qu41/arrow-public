use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::types::event::ArrowEvent;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ViolationSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Violation {
    pub rule_id: String,
    pub field: Option<String>,
    pub message: String,
    pub severity: ViolationSeverity,
}

impl Violation {
    pub fn new(
        rule_id: &str,
        field: impl Into<Option<&'static str>>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            rule_id: rule_id.to_owned(),
            field: field.into().map(|s| s.to_owned()),
            message: message.into(),
            severity: ViolationSeverity::Error,
        }
    }

    pub fn new_owned(
        rule_id: &str,
        field: impl Into<Option<String>>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            rule_id: rule_id.to_owned(),
            field: field.into(),
            message: message.into(),
            severity: ViolationSeverity::Error,
        }
    }

    pub fn warning(
        rule_id: &str,
        field: impl Into<Option<&'static str>>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            rule_id: rule_id.to_owned(),
            field: field.into().map(|s| s.to_owned()),
            message: message.into(),
            severity: ViolationSeverity::Warning,
        }
    }

    pub fn is_error(&self) -> bool {
        self.severity == ViolationSeverity::Error
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub passed: bool,
    pub violations: Vec<Violation>,
    pub forced_action: Option<String>,
}

impl ValidationResult {
    pub fn ok() -> Self {
        Self {
            passed: true,
            violations: Vec::new(),
            forced_action: None,
        }
    }

    pub fn from_violations(violations: Vec<Violation>) -> Self {
        let passed = !violations.iter().any(Violation::is_error);
        Self {
            passed,
            violations,
            forced_action: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LifecycleContext {
    pub processed_event_ids: HashSet<String>,
    pub lifecycle_events: HashMap<String, Vec<ArrowEvent>>,
}
