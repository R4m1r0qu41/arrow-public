mod event;
mod message;
mod result;
mod semantic;

pub use result::{LifecycleContext, ValidationResult, Violation, ViolationSeverity};

use event::run_event;
use message::run_message;

use crate::{
    parser::message::parse,
    registry::loader::Registries,
    types::{event::ArrowEvent, message::ArrowMessage},
};

// ── Pipeline ──────────────────────────────────────────────────────────────

pub struct ValidationPipeline<'a> {
    registries: &'a Registries,
}

impl<'a> ValidationPipeline<'a> {
    pub fn new(registries: &'a Registries) -> Self {
        Self { registries }
    }

    pub fn validate_message(&self, msg: &ArrowMessage) -> ValidationResult {
        run_message(msg, self.registries)
    }

    pub fn validate_event(&self, event: &ArrowEvent) -> ValidationResult {
        let msg = match parse(&event.arrow_code) {
            Ok(m) => m,
            Err(e) => {
                return ValidationResult::from_violations(vec![Violation::new(
                    e.rule_id(),
                    Some("arrow_code"),
                    e.to_string(),
                )]);
            }
        };
        run_event(&msg, event, None, self.registries)
    }

    pub fn validate_event_with_context(
        &self,
        event: &ArrowEvent,
        context: &LifecycleContext,
    ) -> ValidationResult {
        let msg = match parse(&event.arrow_code) {
            Ok(m) => m,
            Err(e) => {
                return ValidationResult::from_violations(vec![Violation::new(
                    e.rule_id(),
                    Some("arrow_code"),
                    e.to_string(),
                )]);
            }
        };
        run_event(&msg, event, Some(context), self.registries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::event::LegJurisdiction;
    use crate::types::registry::{
        ActionEntry, ClassEntry, ConfidenceEntry, FinalityBasisEntry, LegSupraJurisdictionEntry,
        OriginEntry, StateEntry, VerificationRegimeEntry,
    };

    /// Minimal registries populated enough to pass MSG-004, ORG-003, SEM-015, and reach FB-001.
    fn minimal_registries_for_fb001() -> Registries {
        let mut r = Registries::empty();
        r.classes.insert(
            3,
            ClassEntry {
                code: 3,
                name: "Restrict".into(),
                description: "".into(),
            },
        );
        r.states.insert(
            "FN".into(),
            StateEntry {
                code: "FN".into(),
                name: "Final".into(),
                description: "".into(),
            },
        );
        r.actions.insert(
            "BL".into(),
            ActionEntry {
                code: "BL".into(),
                name: "Block".into(),
                description: "".into(),
            },
        );
        r.confidence.insert(
            "C".into(),
            ConfidenceEntry {
                code: "C".into(),
                name: "C".into(),
                description: "".into(),
                allows_action: false,
            },
        );
        r.verification_regimes.insert(
            "legal_attestation".into(),
            VerificationRegimeEntry {
                code: "legal_attestation".into(),
                name: "Legal Attestation".into(),
                description: "".into(),
                max_confidence: "C".into(),
                classification_bound: false,
                pre_settlement_max_confidence: None,
                closure_discipline: None,
            },
        );
        r.origins.insert(
            "LEG".into(),
            OriginEntry {
                code: "LEG".into(),
                name: "Legal".into(),
                description: "".into(),
                verification_regime: "legal_attestation".into(),
                origin_type: "legal".into(),
            },
        );
        r.finality_basis.insert(
            "probabilistic".into(),
            FinalityBasisEntry {
                code: "probabilistic".into(),
                name: "Probabilistic Finality".into(),
                description: "".into(),
                requires_finality_eta: false,
            },
        );
        r
    }

    #[test]
    fn fb001_fires_when_finality0_and_basis_absent() {
        let registries = minimal_registries_for_fb001();
        let pipeline = ValidationPipeline::new(&registries);
        let event = ArrowEvent {
            arrow_code: "3:LEG:FN:BL:0:C".into(),
            event_id: "e1".into(),
            lifecycle_id: "lc1".into(),
            timestamp: "2026-01-01T00:00:00Z".into(),
            object_type: "".into(),
            object_id: "obj1".into(),
            finality_basis: None,
            ..ArrowEvent::default()
        };
        let result = pipeline.validate_event(&event);
        assert!(
            result.violations.iter().any(|v| v.rule_id == "FB-001"),
            "FB-001 should fire when finality=0 and finality_basis is absent"
        );
    }

    #[test]
    fn fb001_fires_when_finality0_and_basis_unregistered() {
        let registries = minimal_registries_for_fb001();
        let pipeline = ValidationPipeline::new(&registries);
        let event = ArrowEvent {
            arrow_code: "3:LEG:FN:BL:0:C".into(),
            event_id: "e2".into(),
            lifecycle_id: "lc1".into(),
            timestamp: "2026-01-01T00:00:00Z".into(),
            object_type: "".into(),
            object_id: "obj1".into(),
            finality_basis: Some("not_a_real_basis".into()),
            ..ArrowEvent::default()
        };
        let result = pipeline.validate_event(&event);
        assert!(
            result.violations.iter().any(|v| v.rule_id == "FB-001"),
            "FB-001 should fire when finality_basis value is not registered"
        );
    }

    #[test]
    fn fb001_passes_when_finality0_and_basis_registered() {
        let registries = minimal_registries_for_fb001();
        let pipeline = ValidationPipeline::new(&registries);
        let event = ArrowEvent {
            arrow_code: "3:LEG:FN:BL:0:C".into(),
            event_id: "e3".into(),
            lifecycle_id: "lc1".into(),
            timestamp: "2026-01-01T00:00:00Z".into(),
            object_type: "".into(),
            object_id: "obj1".into(),
            finality_basis: Some("probabilistic".into()),
            ..ArrowEvent::default()
        };
        let result = pipeline.validate_event(&event);
        assert!(
            !result.violations.iter().any(|v| v.rule_id == "FB-001"),
            "FB-001 should not fire when finality_basis is registered"
        );
    }

    #[test]
    fn fb001_skipped_when_finality1() {
        let registries = minimal_registries_for_fb001();
        let pipeline = ValidationPipeline::new(&registries);
        let event = ArrowEvent {
            arrow_code: "3:LEG:FN:BL:1:C".into(),
            event_id: "e4".into(),
            lifecycle_id: "lc1".into(),
            timestamp: "2026-01-01T00:00:00Z".into(),
            object_type: "".into(),
            object_id: "obj1".into(),
            finality_basis: None,
            ..ArrowEvent::default()
        };
        let result = pipeline.validate_event(&event);
        assert!(
            !result.violations.iter().any(|v| v.rule_id == "FB-001"),
            "FB-001 should not fire when finality=1"
        );
    }

    // ── v1.5.1 SEM-013, LEG-006/007/008, MSG-006 ────────────────────────────

    /// Registries with enough breadth to test SEM-013 and the LEG-* derivation rules.
    /// Includes both LEG (legal_attestation) and ARB (optimistic_settled) origins, and
    /// the action / state / class / confidence vocabulary needed for v1.5.1 examples.
    fn registries_for_v1_6_tests() -> Registries {
        let mut r = minimal_registries_for_fb001();
        for code in [1u8, 2, 5, 6] {
            r.classes.insert(
                code,
                ClassEntry {
                    code,
                    name: format!("Class {code}"),
                    description: "".into(),
                },
            );
        }
        for s in ["PD", "CD", "RV", "TL"] {
            r.states.insert(
                s.into(),
                StateEntry {
                    code: s.into(),
                    name: s.into(),
                    description: "".into(),
                },
            );
        }
        for p in ["PR", "TR", "CL", "HL", "RW"] {
            r.actions.insert(
                p.into(),
                ActionEntry {
                    code: p.into(),
                    name: p.into(),
                    description: "".into(),
                },
            );
        }
        for c in ["A", "B", "D", "E"] {
            r.confidence.insert(
                c.into(),
                ConfidenceEntry {
                    code: c.into(),
                    name: c.into(),
                    description: "".into(),
                    allows_action: c != "E",
                },
            );
        }
        r.verification_regimes.insert(
            "optimistic_settled".into(),
            VerificationRegimeEntry {
                code: "optimistic_settled".into(),
                name: "Optimistic Settled".into(),
                description: "".into(),
                max_confidence: "A".into(),
                classification_bound: false,
                pre_settlement_max_confidence: Some("B".into()),
                closure_discipline: None,
            },
        );
        r.origins.insert(
            "ARB".into(),
            OriginEntry {
                code: "ARB".into(),
                name: "Arbitrum".into(),
                description: "".into(),
                verification_regime: "optimistic_settled".into(),
                origin_type: "chain".into(),
            },
        );
        r.leg_supra_jurisdictions.insert(
            "UNSC".into(),
            LegSupraJurisdictionEntry {
                code: "UNSC".into(),
                name: "United Nations Security Council".into(),
                enforcement_basis: "UN Charter Chapter VII".into(),
                enforcement_type: "mandatory_global".into(),
                geographic_scope: "global_un_members".into(),
                status: "active".into(),
                added_version: "0.1".into(),
            },
        );
        r
    }

    fn leg_event(arrow_code: &str, subtype: &str, reason_code: &str) -> ArrowEvent {
        ArrowEvent {
            arrow_code: arrow_code.into(),
            event_id: "e".into(),
            lifecycle_id: "lc".into(),
            timestamp: "2026-05-31T00:00:00Z".into(),
            object_type: "".into(),
            object_id: "obj".into(),
            leg_subtype: Some(subtype.into()),
            leg_jurisdiction: Some(LegJurisdiction {
                country: Some("MX".into()),
                supra: None,
            }),
            reason_code: reason_code.into(),
            finality_basis: Some("probabilistic".into()),
            ..ArrowEvent::default()
        }
    }

    #[test]
    fn sem013_rejects_executing_action_at_finality0() {
        let registries = registries_for_v1_6_tests();
        let pipeline = ValidationPipeline::new(&registries);
        for raw in ["5:ARB:FN:TR:0:A", "2:ARB:FN:PR:0:A", "6:ARB:FN:CL:0:A"] {
            let msg = parse(raw).unwrap();
            let result = pipeline.validate_message(&msg);
            assert!(
                result.violations.iter().any(|v| v.rule_id == "SEM-013"),
                "SEM-013 should fire on {raw}: {:?}",
                result.violations
            );
            assert!(
                !result.violations.iter().any(|v| v.rule_id == "SEM-006"),
                "SEM-013 independence check should not rely on SEM-006 for {raw}: {:?}",
                result.violations
            );
        }
    }

    #[test]
    fn sem006_accepts_class3_non_executing_actions() {
        let registries = registries_for_v1_6_tests();
        let pipeline = ValidationPipeline::new(&registries);
        for raw in [
            "3:ARB:FN:BL:1:A",
            "3:ARB:FN:HL:1:A",
            "3:ARB:FN:RW:1:A",
            // Finality=0 case sits at Confidence B: optimistic_settled's
            // pre-settlement SEM-015 ceiling caps pending events at B, and
            // this test isolates SEM-006, not the ceiling.
            "3:ARB:FN:BL:0:B",
        ] {
            let msg = parse(raw).unwrap();
            let result = pipeline.validate_message(&msg);
            assert!(
                result.passed,
                "Class 3 non-executing action should pass for {raw}: {:?}",
                result.violations
            );
        }
    }

    #[test]
    fn sem006_rejects_class3_executing_actions() {
        let registries = registries_for_v1_6_tests();
        let pipeline = ValidationPipeline::new(&registries);
        for raw in ["3:ARB:FN:PR:1:A", "3:ARB:FN:TR:1:A", "3:ARB:FN:CL:1:A"] {
            let msg = parse(raw).unwrap();
            let result = pipeline.validate_message(&msg);
            assert!(
                result
                    .violations
                    .iter()
                    .any(|v| { v.rule_id == "SEM-006" && v.message.contains(msg.action.as_str()) }),
                "SEM-006 should name the invalid Action for {raw}: {:?}",
                result.violations
            );
        }
    }

    #[test]
    fn sem013_allows_non_executing_action_at_finality0() {
        let registries = registries_for_v1_6_tests();
        let pipeline = ValidationPipeline::new(&registries);
        for raw in ["1:ARB:FN:HL:0:A", "1:ARB:FN:RW:0:A", "3:ARB:CD:BL:0:A"] {
            let msg = parse(raw).unwrap();
            let result = pipeline.validate_message(&msg);
            assert!(
                !result.violations.iter().any(|v| v.rule_id == "SEM-013"),
                "SEM-013 should not fire on {raw}: {:?}",
                result.violations
            );
        }
    }

    #[test]
    fn sem013_skipped_at_finality1() {
        let registries = registries_for_v1_6_tests();
        let pipeline = ValidationPipeline::new(&registries);
        let msg = parse("5:ARB:FN:TR:1:A").unwrap();
        let result = pipeline.validate_message(&msg);
        assert!(
            !result.violations.iter().any(|v| v.rule_id == "SEM-013"),
            "SEM-013 should not fire at Finality=1"
        );
    }

    #[test]
    fn msg006_fires_when_leg_subtype_absent() {
        let registries = registries_for_v1_6_tests();
        let pipeline = ValidationPipeline::new(&registries);
        let event = ArrowEvent {
            arrow_code: "2:LEG:FN:PR:1:C".into(),
            event_id: "e".into(),
            lifecycle_id: "lc".into(),
            timestamp: "2026-05-31T00:00:00Z".into(),
            object_type: "".into(),
            object_id: "obj".into(),
            leg_subtype: None,
            ..ArrowEvent::default()
        };
        let result = pipeline.validate_event(&event);
        assert!(
            result.violations.iter().any(|v| v.rule_id == "MSG-006"),
            "MSG-006 should fire when leg_subtype is absent: {:?}",
            result.violations
        );
    }

    #[test]
    fn msg006_fires_when_leg_subtype_unregistered() {
        let registries = registries_for_v1_6_tests();
        let pipeline = ValidationPipeline::new(&registries);
        let event = leg_event("2:LEG:FN:PR:1:C", "INVALID_SUBTYPE", "CO");
        let result = pipeline.validate_event(&event);
        assert!(
            result.violations.iter().any(|v| v.rule_id == "MSG-006"),
            "MSG-006 should fire on unregistered leg_subtype: {:?}",
            result.violations
        );
    }

    #[test]
    fn leg006_rejects_action_outside_jud_range() {
        // JUD + Finality=1 permits {PR, BL, TR, CL}. RW is outside the range.
        let registries = registries_for_v1_6_tests();
        let pipeline = ValidationPipeline::new(&registries);
        let event = leg_event("2:LEG:FN:RW:1:C", "JUD", "CO");
        let result = pipeline.validate_event(&event);
        assert!(
            result.violations.iter().any(|v| v.rule_id == "LEG-006"),
            "LEG-006 should fire when Action is outside JUD permitted set: {:?}",
            result.violations
        );
    }

    #[test]
    fn leg006_permits_jud_pr_at_finality1() {
        let registries = registries_for_v1_6_tests();
        let pipeline = ValidationPipeline::new(&registries);
        let event = leg_event("2:LEG:FN:PR:1:C", "JUD", "CO");
        let result = pipeline.validate_event(&event);
        assert!(
            !result.violations.iter().any(|v| v.rule_id == "LEG-006"),
            "LEG-006 should pass for JUD + PR + Finality=1: {:?}",
            result.violations
        );
    }

    #[test]
    fn leg006_prec_only_permits_rw() {
        // PREC + Finality=1 permits only {RW}.
        let registries = registries_for_v1_6_tests();
        let pipeline = ValidationPipeline::new(&registries);
        let event = leg_event("2:LEG:FN:PR:1:C", "PREC", "CO");
        let result = pipeline.validate_event(&event);
        assert!(
            result.violations.iter().any(|v| v.rule_id == "LEG-006"),
            "LEG-006 should reject PR for PREC subtype: {:?}",
            result.violations
        );
    }

    #[test]
    fn leg007_forces_bl_on_supra_sanctions() {
        // SUPRA + reason_code=SA + Finality=1 + Action=PR → LEG-007 fires + forced_action=BL.
        let registries = registries_for_v1_6_tests();
        let pipeline = ValidationPipeline::new(&registries);
        let event = leg_event("2:LEG:FN:PR:1:C", "SUPRA", "SA");
        let result = pipeline.validate_event(&event);
        assert!(
            result.violations.iter().any(|v| v.rule_id == "LEG-007"),
            "LEG-007 should fire on SUPRA+SA+Finality=1 with non-BL Action: {:?}",
            result.violations
        );
        assert_eq!(
            result.forced_action.as_deref(),
            Some("BL"),
            "forced_action should be set to BL"
        );
        // LEG-006 range check should be skipped when LEG-007 forces BL.
        assert!(
            !result.violations.iter().any(|v| v.rule_id == "LEG-006"),
            "LEG-006 should not double-fire when LEG-007 has already forced BL"
        );
    }

    #[test]
    fn leg007_silent_when_supra_sa_already_bl() {
        let registries = registries_for_v1_6_tests();
        let pipeline = ValidationPipeline::new(&registries);
        let event = leg_event("2:LEG:FN:BL:1:C", "SUPRA", "SA");
        let result = pipeline.validate_event(&event);
        assert!(
            !result.violations.iter().any(|v| v.rule_id == "LEG-007"),
            "LEG-007 should not fire when claimed Action already equals BL: {:?}",
            result.violations
        );
        assert_eq!(result.forced_action.as_deref(), Some("BL"));
    }

    #[test]
    fn leg008_warns_on_leg_text_finality1() {
        let registries = registries_for_v1_6_tests();
        let pipeline = ValidationPipeline::new(&registries);
        let event = leg_event("2:LEG:FN:HL:1:C", "LEG_TEXT", "CO");
        let result = pipeline.validate_event(&event);
        let warning = result
            .violations
            .iter()
            .find(|v| v.rule_id == "LEG-008")
            .expect("LEG-008 warning expected");
        assert_eq!(warning.severity, ViolationSeverity::Warning);
        // Warnings are non-fatal: event still passes.
        assert!(
            result.passed,
            "LEG-008 is a warning and must not cause rejection: {:?}",
            result.violations
        );
    }

    #[test]
    fn leg009_requires_leg_jurisdiction() {
        let registries = registries_for_v1_6_tests();
        let pipeline = ValidationPipeline::new(&registries);
        let mut event = leg_event("2:LEG:FN:PR:1:C", "JUD", "CO");
        event.leg_jurisdiction = None;
        let result = pipeline.validate_event(&event);
        assert!(
            result.violations.iter().any(|v| v.rule_id == "LEG-009"),
            "LEG-009 should fire when leg_jurisdiction is absent: {:?}",
            result.violations
        );
    }

    #[test]
    fn leg010_rejects_unregistered_supra() {
        let registries = registries_for_v1_6_tests();
        let pipeline = ValidationPipeline::new(&registries);
        let mut event = leg_event("2:LEG:FN:HL:1:C", "SUPRA", "CO");
        event.leg_jurisdiction = Some(LegJurisdiction {
            country: None,
            supra: Some("ATLANTIS".into()),
        });
        let result = pipeline.validate_event(&event);
        assert!(
            result.violations.iter().any(|v| v.rule_id == "LEG-010"),
            "LEG-010 should reject unregistered supra: {:?}",
            result.violations
        );
    }

    #[test]
    fn leg010_accepts_registered_active_supra() {
        let registries = registries_for_v1_6_tests();
        let pipeline = ValidationPipeline::new(&registries);
        let mut event = leg_event("2:LEG:FN:HL:1:C", "SUPRA", "CO");
        event.leg_jurisdiction = Some(LegJurisdiction {
            country: None,
            supra: Some("UNSC".into()),
        });
        let result = pipeline.validate_event(&event);
        assert!(
            !result.violations.iter().any(|v| v.rule_id == "LEG-010"),
            "LEG-010 should accept registered active supra: {:?}",
            result.violations
        );
    }

    #[test]
    fn leg005_blocks_confidence_d_for_jud() {
        let registries = registries_for_v1_6_tests();
        let pipeline = ValidationPipeline::new(&registries);
        let event = leg_event("2:LEG:FN:PR:1:D", "JUD", "CO");
        let result = pipeline.validate_event(&event);
        assert!(
            result.violations.iter().any(|v| v.rule_id == "LEG-005"),
            "LEG-005 should fire on Confidence=D + JUD: {:?}",
            result.violations
        );
    }
}
