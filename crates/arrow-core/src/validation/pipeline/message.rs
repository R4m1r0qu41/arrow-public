use crate::{registry::loader::Registries, types::message::ArrowMessage};

use super::{semantic::validate_semantic, ValidationResult, Violation};

pub(super) fn run_message(msg: &ArrowMessage, registries: &Registries) -> ValidationResult {
    let mut violations = Vec::new();

    // Step 1 — MSG-004: field registry lookups
    if !registries.classes.contains_key(&msg.class.as_u8()) {
        violations.push(Violation::new(
            "MSG-004",
            Some("class"),
            format!("class {} not registered", msg.class.as_u8()),
        ));
    }
    if !registries.states.contains_key(msg.state.as_str()) {
        violations.push(Violation::new(
            "MSG-004",
            Some("state"),
            format!("state `{}` not registered", msg.state.as_str()),
        ));
    }
    if !registries.actions.contains_key(msg.action.as_str()) {
        violations.push(Violation::new(
            "MSG-004",
            Some("action"),
            format!("action `{}` not registered", msg.action.as_str()),
        ));
    }
    if !registries.confidence.contains_key(msg.confidence.as_str()) {
        violations.push(Violation::new(
            "MSG-004",
            Some("confidence"),
            format!("confidence `{}` not registered", msg.confidence.as_str()),
        ));
    }
    if violations.iter().any(Violation::is_error) {
        return ValidationResult::from_violations(violations);
    }

    // Step 2 — ORG-003: origin registered
    if !registries.origins.contains_key(msg.origin.as_str()) {
        violations.push(Violation::new(
            "ORG-003",
            Some("origin"),
            format!("origin `{}` not registered", msg.origin.as_str()),
        ));
        return ValidationResult::from_violations(violations);
    }

    // Step 4 — verification_regime declared
    if let Some(entry) = registries.origins.get(msg.origin.as_str()) {
        if entry.verification_regime.trim().is_empty() {
            violations.push(Violation::new(
                "SEM-015",
                Some("verification_regime"),
                format!(
                    "origin `{}` has no verification_regime",
                    msg.origin.as_str()
                ),
            ));
            return ValidationResult::from_violations(violations);
        }
    }

    // Step 5 — Semantic constraints (SEM-001 to SEM-014)
    violations.extend(validate_semantic(msg, registries));

    ValidationResult::from_violations(violations)
}
