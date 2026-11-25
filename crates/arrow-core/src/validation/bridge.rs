use super::common::confidence_rank;
use super::pipeline::Violation;
/// Bridge Registry rules (BRG-001 to BRG-006) and cross-domain semantic
/// consistency rules (SEM-017, MSG-005, SEM-019) introduced in Arrow Protocol
/// v1.4 (RFC-002 — Verus-Ethereum Bridge Exploit, 2026-05-18).
///
/// Called from `pipeline::run_event` after SEM-016 (chain trust flag).
/// All rules in this module are protocol-level and non-overridable at the
/// deployment policy layer.
use crate::{
    registry::loader::Registries,
    types::{event::ArrowEvent, message::ArrowMessage},
};

// ── Result type ───────────────────────────────────────────────────────────

/// Result returned by the bridge validation pass.
/// `forced_action` is set to `Some("BL")` when BRG-001 fires — the caller
/// must propagate this into the ValidationResult the same way SEM-016 does.
#[derive(Debug, Default)]
pub struct BridgeValidationResult {
    pub violations: Vec<Violation>,
    pub forced_action: Option<String>,
}

impl BridgeValidationResult {
    fn ok() -> Self {
        Self::default()
    }
    fn hard_block(violations: Vec<Violation>) -> Self {
        Self {
            violations,
            forced_action: Some("BL".to_string()),
        }
    }
}

// ── Public entry point ────────────────────────────────────────────────────

/// Entry point for all Bridge Registry and cross-domain semantic consistency
/// rules. Called from `pipeline::run_event` after SEM-016.
///
/// If `event.bridge_id` is None, returns immediately with no violations.
pub fn validate_bridge_rules(
    msg: &ArrowMessage,
    event: &ArrowEvent,
    registries: &Registries,
) -> BridgeValidationResult {
    let bridge_id = match &event.bridge_id {
        Some(id) if !id.is_empty() => id.as_str(),
        _ => return BridgeValidationResult::ok(),
    };

    // Resolve bridge registry entry — reject immediately if unregistered.
    let bridge = match registries.bridge_registry.get(bridge_id) {
        Some(b) => b,
        None => {
            return BridgeValidationResult {
                violations: vec![Violation::new(
                    "BRG-000",
                    Some("bridge_id"),
                    format!(
                        "bridge_id `{bridge_id}` is not registered in the Bridge Registry; \
                         events referencing unknown bridges MUST be rejected"
                    ),
                )],
                forced_action: None,
            };
        }
    };

    // ── BRG-001 — Bridge Trust Flag Enforcement ───────────────────────────
    // bridge_trust_flag == 0 forces Action BL regardless of all other fields.
    // Non-overridable. Parallel to SEM-016 for chain_trust_flag.
    if bridge.bridge_trust_flag == 0 {
        let mut violations = Vec::new();

        violations.push(Violation::new(
            "BRG-001",
            Some("bridge_id"),
            format!(
                "bridge `{bridge_id}` is disabled (bridge_trust_flag=0, status=`{}`{}); \
                 all events from this bridge are forced to Action BL — non-overridable \
                 (reason code: BRIDGE_TRUST_REVOKED)",
                bridge.bridge_status,
                bridge
                    .bridge_status_reason
                    .as_deref()
                    .map(|r| format!(", reason: {r}"))
                    .unwrap_or_default()
            ),
        ));

        // Reject non-BL action explicitly.
        if msg.action.as_str() != "BL" {
            violations.push(Violation::new(
                "BRG-001",
                Some("action"),
                format!(
                    "action `{}` rejected; bridge `{bridge_id}` has bridge_trust_flag=0 — \
                     only Action BL is permitted",
                    msg.action.as_str()
                ),
            ));
        }

        return BridgeValidationResult::hard_block(violations);
    }

    // ── Active bridge — run remaining rules ───────────────────────────────
    let mut violations = Vec::new();

    // SEM-017 — Proof Classification Binding
    violations.extend(validate_sem017(msg, event, bridge, registries));

    // MSG-005 — Classification Binding Method Registry Membership
    violations.extend(validate_msg005(event, registries));

    // SEM-019 — Economic Consistency Binding
    violations.extend(validate_sem019(msg, event));

    // BRG-003 — Classification audit reference required above Confidence C
    let conf_rank = confidence_rank(msg.confidence.as_str());
    let cp = &bridge.classification_profile;
    if cp.classification_verified
        && conf_rank > confidence_rank("C")
        && cp.classification_audit_ref.is_none()
    {
        violations.push(Violation::new(
            "BRG-003",
            Some("bridge_id"),
            format!(
                "bridge `{bridge_id}` declares classification_verified=true and claims \
                 Confidence {} (above C) but provides no classification_audit_ref; \
                 self-certification is insufficient per BRG-003",
                msg.confidence.as_str()
            ),
        ));
    }

    // BRG-006 — Confidence subprofile required when Confidence above D
    if conf_rank > confidence_rank("D") && cp.classification_binding_method.is_none() {
        violations.push(Violation::new(
            "BRG-006",
            Some("confidence"),
            format!(
                "bridge `{bridge_id}` claims Confidence {} (above D) but has no \
                 classification_binding_method declared; events from bridges without a \
                 declared confidence subprofile are capped at D (BRG-006)",
                msg.confidence.as_str()
            ),
        ));
    }

    BridgeValidationResult {
        violations,
        forced_action: None,
    }
}

// ── SEM-017 — Proof Classification Binding ────────────────────────────────

fn validate_sem017(
    msg: &ArrowMessage,
    event: &ArrowEvent,
    bridge: &crate::types::registry::BridgeRegistryEntry,
    registries: &Registries,
) -> Vec<Violation> {
    let mut v = Vec::new();
    let regime = bridge.verification_regime.as_str();

    // SEM-017 is exempt for native_canonical and optimistic_settled — classification
    // is enforced by the chain's own type system in those regimes.
    // "chain_native" alias accepted for bridge registry entries predating v1.5 rename.
    if matches!(
        regime,
        "native_canonical" | "chain_native" | "optimistic_settled"
    ) {
        return v;
    }

    // proof_semantics is required for all non-canonical bridge events.
    let ps = match &event.proof_semantics {
        Some(ps) => ps,
        None => {
            v.push(Violation::new(
                "SEM-017",
                Some("proof_semantics"),
                format!(
                    "proof_semantics is required for bridge events under regime `{regime}`; \
                     a proof of source-domain data existence is not a proof of its \
                     classification (SEM-017 — PROOF_TYPE_UNBOUND)"
                ),
            ));
            return v;
        }
    };

    let conf_rank = confidence_rank(msg.confidence.as_str());

    // classification_verified=false caps Confidence at C.
    if !ps.classification_verified && conf_rank > confidence_rank("C") {
        v.push(Violation::new(
            "SEM-017",
            Some("confidence"),
            format!(
                "classification_verified=false; Confidence MUST NOT exceed C — got `{}`; \
                 reduce Confidence or establish classification binding \
                 (SEM-017 — PROOF_TYPE_UNBOUND)",
                msg.confidence.as_str()
            ),
        ));
    }

    // Regime is not classification-bound by default — Confidence A or B requires
    // explicit classification binding in the Bridge Registry.
    let regime_entry = registries.verification_regimes.get(regime);
    let classification_bound = regime_entry
        .map(|r| r.classification_bound)
        .unwrap_or(false);

    if !classification_bound && conf_rank >= confidence_rank("B") {
        // A or B claimed under a non-classification-bound regime without the
        // bridge explicitly declaring classification binding.
        let bridge_overrides = bridge.classification_profile.classification_verified
            && bridge
                .classification_profile
                .classification_audit_ref
                .is_some();

        if !bridge_overrides {
            v.push(Violation::new(
                "SEM-017",
                Some("confidence"),
                format!(
                    "verification regime `{regime}` is not classification-bound by default; \
                     Confidence A or B requires explicit classification_verified=true and \
                     classification_audit_ref in the Bridge Registry entry \
                     (SEM-017 — CLASSIFICATION_REGIME_CEILING)"
                ),
            ));
        }
    }

    // Consistency check: proof_semantics says verified but bridge registry says not.
    if ps.classification_verified && !bridge.classification_profile.classification_verified {
        v.push(Violation::new(
            "SEM-017",
            Some("proof_semantics.classification_verified"),
            format!(
                "proof_semantics.classification_verified=true but bridge `{}` \
                 classification_profile.classification_verified=false; \
                 the Bridge Registry entry must be updated before events can claim \
                 classification binding",
                bridge.bridge_id
            ),
        ));
    }

    v
}

// ── MSG-005 — Classification Binding Method Registry Membership ────────────

fn validate_msg005(event: &ArrowEvent, registries: &Registries) -> Vec<Violation> {
    let mut v = Vec::new();

    let ps = match &event.proof_semantics {
        Some(ps) => ps,
        None => return v,
    };

    let method = ps.classification_binding_method.as_str();
    if method.is_empty() {
        // Empty method is allowed only when proof_semantics.classification_verified=false
        // and the empty string is intentional. SEM-017 already constrains the ceiling.
        return v;
    }

    if !registries
        .classification_binding_methods
        .contains_key(method)
    {
        v.push(Violation::new(
            "MSG-005",
            Some("proof_semantics.classification_binding_method"),
            format!(
                "classification_binding_method `{method}` is not a registered entry in \
                 classification_binding_methods.json; freeform strings are not permitted \
                 (MSG-005)"
            ),
        ));
    }

    v
}

// ── SEM-019 — Economic Consistency Binding ────────────────────────────────

fn validate_sem019(msg: &ArrowMessage, event: &ArrowEvent) -> Vec<Violation> {
    let mut v = Vec::new();
    let class = msg.class.as_u8();
    let conf_rank = confidence_rank(msg.confidence.as_str());

    // Applies only to Class 5 (TRANSFER) or Class 6 (CLOSE) claiming Confidence A or B.
    let applies = (class == 5 || class == 6) && conf_rank >= confidence_rank("B");
    if !applies {
        return v;
    }

    let ec = match &event.economic_consistency {
        Some(ec) => ec,
        None => {
            v.push(Violation::new(
                "SEM-019",
                Some("economic_consistency"),
                format!(
                    "economic_consistency object is required for Class {class} events claiming \
                     Confidence {} through a bridge; source-side value and destination-side \
                     payout must be proven consistent (SEM-019 — ECONOMIC_CONSISTENCY_UNBOUND)",
                    msg.confidence.as_str()
                ),
            ));
            return v;
        }
    };

    // consistency_verified=false caps at C.
    if !ec.consistency_verified && conf_rank >= confidence_rank("B") {
        v.push(Violation::new(
            "SEM-019",
            Some("economic_consistency.consistency_verified"),
            format!(
                "economic_consistency.consistency_verified=false; Confidence {} not \
                 permitted for Class {class} bridge events — reduce to C or below, or \
                 establish economic consistency (SEM-019 — ECONOMIC_CONSISTENCY_UNBOUND)",
                msg.confidence.as_str()
            ),
        ));
    }

    // consistency_verified=true at A or B requires audit ref.
    if ec.consistency_verified
        && conf_rank >= confidence_rank("B")
        && event.economic_consistency_audit_ref.is_none()
    {
        v.push(Violation::new(
            "SEM-019",
            Some("economic_consistency_audit_ref"),
            format!(
                "economic_consistency_audit_ref is required when \
                 consistency_verified=true and Confidence is {} \
                 (SEM-019)",
                msg.confidence.as_str()
            ),
        ));
    }

    v
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        parser::message::parse,
        registry::loader::Registries,
        types::{
            event::{ArrowEvent, EconomicConsistency, ProofSemantics},
            registry::{
                BridgeClassificationProfile, BridgeEconomicConsistencyProfile, BridgeRegistryEntry,
                ClassificationBindingMethodEntry, VerificationRegimeEntry,
            },
        },
    };

    // ── Test fixture builders ─────────────────────────────────────────────

    fn disabled_bridge() -> BridgeRegistryEntry {
        BridgeRegistryEntry {
            bridge_id: "VRSC-ETH-001".into(),
            bridge_name: "Verus-Ethereum Bridge (disabled)".into(),
            description: None,
            source_chain_id: "VRSC-1".into(),
            destination_chain_id: "ETH-1".into(),
            bridge_contract_address: None,
            verification_regime: "bonded_quorum".into(),
            bridge_trust_flag: 0,
            bridge_status: "disabled".into(),
            bridge_status_reason: Some("semantic classification exploit 2026-05-18".into()),
            bridge_status_effective_from: Some("2026-05-18T00:00:00Z".into()),
            classification_profile: BridgeClassificationProfile {
                classification_verified: false,
                classification_confidence_ceiling: Some("C".into()),
                classification_binding_method: Some("context_inference".into()),
                classification_audit_ref: None,
                requires_subprofile: false,
                classification_notes: None,
            },
            economic_consistency_profile: BridgeEconomicConsistencyProfile {
                economic_consistency_required: true,
                economic_consistency_verified: false,
                economic_consistency_audit_ref: None,
                economic_consistency_notes: None,
            },
            restoration_confidence_ceiling: None,
            restoration_ceiling_effective_until: None,
            restoration_conditions_met: false,
        }
    }

    fn active_unclassified_bridge() -> BridgeRegistryEntry {
        BridgeRegistryEntry {
            bridge_id: "TEST-ACTIVE-001".into(),
            bridge_name: "Test Active Bridge".into(),
            description: None,
            source_chain_id: "VRSC-1".into(),
            destination_chain_id: "ETH-1".into(),
            bridge_contract_address: None,
            verification_regime: "bonded_quorum".into(),
            bridge_trust_flag: 1,
            bridge_status: "active".into(),
            bridge_status_reason: None,
            bridge_status_effective_from: None,
            classification_profile: BridgeClassificationProfile {
                classification_verified: false,
                classification_confidence_ceiling: Some("C".into()),
                classification_binding_method: Some("context_inference".into()),
                classification_audit_ref: None,
                requires_subprofile: false,
                classification_notes: None,
            },
            economic_consistency_profile: BridgeEconomicConsistencyProfile {
                economic_consistency_required: false,
                economic_consistency_verified: false,
                economic_consistency_audit_ref: None,
                economic_consistency_notes: None,
            },
            restoration_confidence_ceiling: None,
            restoration_ceiling_effective_until: None,
            restoration_conditions_met: false,
        }
    }

    fn bonded_quorum_regime() -> VerificationRegimeEntry {
        VerificationRegimeEntry {
            code: "bonded_quorum".into(),
            name: "Bonded Quorum".into(),
            description: String::new(),
            max_confidence: "B".into(),
            classification_bound: false,
            pre_settlement_max_confidence: None,
            closure_discipline: None,
        }
    }

    fn context_inference_method() -> ClassificationBindingMethodEntry {
        ClassificationBindingMethodEntry {
            method_id: "context_inference".into(),
            name: "Context Inference".into(),
            description: String::new(),
            max_confidence_ceiling: "C".into(),
            requires_audit_ref: false,
            status: "active".into(),
            added_version: "1.4.0".into(),
        }
    }

    fn regs_with_disabled_bridge() -> Registries {
        let bridge = disabled_bridge();
        let mut regs = Registries::empty();
        regs.bridge_registry
            .insert(bridge.bridge_id.clone(), bridge);
        regs
    }

    fn regs_with_active_bridge() -> Registries {
        let bridge = active_unclassified_bridge();
        let mut regs = Registries::empty();
        regs.bridge_registry
            .insert(bridge.bridge_id.clone(), bridge);
        regs.verification_regimes
            .insert("bonded_quorum".into(), bonded_quorum_regime());
        regs.classification_binding_methods
            .insert("context_inference".into(), context_inference_method());
        regs
    }

    // ── BRG-001 ───────────────────────────────────────────────────────────

    /// RFC-002 Demo Scenario 1 — disabled bridge forces BL, rejects PR.
    #[test]
    fn brg001_disabled_bridge_pr_action_rejected() {
        let msg = parse("5:ORC:FN:PR:1:B").unwrap();
        let event = ArrowEvent {
            arrow_code: "5:ORC:FN:PR:1:B".into(),
            event_id: "E-001".into(),
            lifecycle_id: "L-001".into(),
            timestamp: "2026-05-24T00:00:00Z".into(),
            bridge_id: Some("VRSC-ETH-001".into()),
            ..ArrowEvent::default()
        };
        let regs = regs_with_disabled_bridge();

        let result = validate_bridge_rules(&msg, &event, &regs);

        assert_eq!(
            result.forced_action,
            Some("BL".to_string()),
            "BRG-001 must force Action BL on disabled bridge"
        );
        assert!(
            result.violations.iter().any(|v| v.rule_id == "BRG-001"),
            "BRG-001 violation must be present"
        );
        // Two violations: one for bridge disabled, one for non-BL action.
        assert_eq!(
            result
                .violations
                .iter()
                .filter(|v| v.rule_id == "BRG-001")
                .count(),
            2
        );
    }

    /// BRG-001 fires even when the submitted action is already BL.
    /// The violation records the disabled bridge; forced_action=BL is redundant but correct.
    #[test]
    fn brg001_disabled_bridge_bl_action_still_violates() {
        let msg = parse("3:ORC:FN:BL:0:E").unwrap();
        let event = ArrowEvent {
            arrow_code: "3:ORC:FN:BL:0:E".into(),
            event_id: "E-002".into(),
            lifecycle_id: "L-001".into(),
            timestamp: "2026-05-24T00:00:00Z".into(),
            bridge_id: Some("VRSC-ETH-001".into()),
            ..ArrowEvent::default()
        };
        let regs = regs_with_disabled_bridge();

        let result = validate_bridge_rules(&msg, &event, &regs);

        // forced_action is still BL
        assert_eq!(result.forced_action, Some("BL".to_string()));
        // One violation: bridge disabled. No second violation because action is already BL.
        let brg001_count = result
            .violations
            .iter()
            .filter(|v| v.rule_id == "BRG-001")
            .count();
        assert_eq!(brg001_count, 1);
    }

    /// No bridge_id — bridge validation skips entirely, no violations.
    #[test]
    fn no_bridge_id_skips_all_bridge_rules() {
        let msg = parse("2:CST:FN:PR:1:C").unwrap();
        let event = ArrowEvent {
            arrow_code: "2:CST:FN:PR:1:C".into(),
            event_id: "E-003".into(),
            lifecycle_id: "L-001".into(),
            timestamp: "2026-05-24T00:00:00Z".into(),
            bridge_id: None,
            ..ArrowEvent::default()
        };
        let regs = regs_with_active_bridge();

        let result = validate_bridge_rules(&msg, &event, &regs);

        assert!(result.violations.is_empty());
        assert!(result.forced_action.is_none());
    }

    /// Unknown bridge_id — BRG-000 rejection.
    #[test]
    fn unknown_bridge_id_rejected() {
        let msg = parse("2:ORC:FN:PR:0:C").unwrap();
        let event = ArrowEvent {
            arrow_code: "2:ORC:FN:PR:0:C".into(),
            event_id: "E-004".into(),
            lifecycle_id: "L-001".into(),
            timestamp: "2026-05-24T00:00:00Z".into(),
            bridge_id: Some("DOES-NOT-EXIST".into()),
            ..ArrowEvent::default()
        };
        let regs = Registries::empty();

        let result = validate_bridge_rules(&msg, &event, &regs);

        assert!(result.violations.iter().any(|v| v.rule_id == "BRG-000"));
        assert!(result.forced_action.is_none());
    }

    // ── SEM-017 ───────────────────────────────────────────────────────────

    /// RFC-002 Demo Scenario 2 — classification_verified=false caps at C.
    /// Confidence A submitted → SEM-017 fires.
    #[test]
    fn sem017_unverified_classification_blocks_confidence_a() {
        let msg = parse("2:ORC:FN:PR:0:A").unwrap();
        let event = ArrowEvent {
            arrow_code: "2:ORC:FN:PR:0:A".into(),
            event_id: "E-005".into(),
            lifecycle_id: "L-001".into(),
            timestamp: "2026-05-24T00:00:00Z".into(),
            bridge_id: Some("TEST-ACTIVE-001".into()),
            proof_semantics: Some(ProofSemantics {
                proven_content: "0xdeadbeef".into(),
                source_claim_type: "supplemental_export_output".into(),
                destination_claim_type: "primary_export_instruction".into(),
                classification_binding_method: "context_inference".into(),
                classification_verified: false, // <-- not verified
            }),
            ..ArrowEvent::default()
        };
        let regs = regs_with_active_bridge();

        let result = validate_bridge_rules(&msg, &event, &regs);

        assert!(
            result.violations.iter().any(|v| v.rule_id == "SEM-017"),
            "SEM-017 must fire when classification_verified=false and Confidence=A"
        );
    }

    /// Confidence C with classification_verified=false — permitted.
    #[test]
    fn sem017_unverified_classification_permits_confidence_c() {
        let msg = parse("2:ORC:FN:PR:0:C").unwrap();
        let event = ArrowEvent {
            arrow_code: "2:ORC:FN:PR:0:C".into(),
            event_id: "E-006".into(),
            lifecycle_id: "L-001".into(),
            timestamp: "2026-05-24T00:00:00Z".into(),
            bridge_id: Some("TEST-ACTIVE-001".into()),
            proof_semantics: Some(ProofSemantics {
                proven_content: "0xdeadbeef".into(),
                source_claim_type: "supplemental_export_output".into(),
                destination_claim_type: "primary_export_instruction".into(),
                classification_binding_method: "context_inference".into(),
                classification_verified: false,
            }),
            ..ArrowEvent::default()
        };
        let regs = regs_with_active_bridge();

        let result = validate_bridge_rules(&msg, &event, &regs);

        assert!(
            !result.violations.iter().any(|v| v.rule_id == "SEM-017" && v.field.as_deref() == Some("confidence")),
            "SEM-017 confidence cap must NOT fire when Confidence=C with classification_verified=false"
        );
    }

    /// Missing proof_semantics on non-canonical bridge event → SEM-017.
    #[test]
    fn sem017_missing_proof_semantics_fires() {
        let msg = parse("2:ORC:FN:PR:0:C").unwrap();
        let event = ArrowEvent {
            arrow_code: "2:ORC:FN:PR:0:C".into(),
            event_id: "E-007".into(),
            lifecycle_id: "L-001".into(),
            timestamp: "2026-05-24T00:00:00Z".into(),
            bridge_id: Some("TEST-ACTIVE-001".into()),
            proof_semantics: None, // <-- absent
            ..ArrowEvent::default()
        };
        let regs = regs_with_active_bridge();

        let result = validate_bridge_rules(&msg, &event, &regs);

        assert!(result.violations.iter().any(|v| v.rule_id == "SEM-017"));
    }

    // ── MSG-005 ───────────────────────────────────────────────────────────

    /// Unknown classification_binding_method → MSG-005 rejection.
    #[test]
    fn msg005_unregistered_method_rejected() {
        let msg = parse("2:ORC:FN:PR:0:C").unwrap();
        let event = ArrowEvent {
            arrow_code: "2:ORC:FN:PR:0:C".into(),
            event_id: "E-008".into(),
            lifecycle_id: "L-001".into(),
            timestamp: "2026-05-24T00:00:00Z".into(),
            bridge_id: Some("TEST-ACTIVE-001".into()),
            proof_semantics: Some(ProofSemantics {
                proven_content: "0xabc".into(),
                source_claim_type: "export".into(),
                destination_claim_type: "import".into(),
                classification_binding_method: "made_up_method".into(), // <-- not registered
                classification_verified: false,
            }),
            ..ArrowEvent::default()
        };
        let regs = regs_with_active_bridge();

        let result = validate_bridge_rules(&msg, &event, &regs);

        assert!(
            result.violations.iter().any(|v| v.rule_id == "MSG-005"),
            "MSG-005 must reject unregistered classification_binding_method"
        );
    }

    /// Registered classification_binding_method → MSG-005 passes.
    #[test]
    fn msg005_registered_method_passes() {
        let msg = parse("2:ORC:FN:PR:0:C").unwrap();
        let event = ArrowEvent {
            arrow_code: "2:ORC:FN:PR:0:C".into(),
            event_id: "E-009".into(),
            lifecycle_id: "L-001".into(),
            timestamp: "2026-05-24T00:00:00Z".into(),
            bridge_id: Some("TEST-ACTIVE-001".into()),
            proof_semantics: Some(ProofSemantics {
                proven_content: "0xabc".into(),
                source_claim_type: "export".into(),
                destination_claim_type: "import".into(),
                classification_binding_method: "context_inference".into(), // registered
                classification_verified: false,
            }),
            ..ArrowEvent::default()
        };
        let regs = regs_with_active_bridge();

        let result = validate_bridge_rules(&msg, &event, &regs);

        assert!(
            !result.violations.iter().any(|v| v.rule_id == "MSG-005"),
            "MSG-005 must not fire for registered method"
        );
    }

    // ── SEM-019 ───────────────────────────────────────────────────────────

    /// Class 5, Confidence B, missing economic_consistency → SEM-019.
    #[test]
    fn sem019_missing_economic_consistency_on_execute_b() {
        let msg = parse("5:ORC:FN:TR:1:B").unwrap();
        let event = ArrowEvent {
            arrow_code: "5:ORC:FN:TR:1:B".into(),
            event_id: "E-010".into(),
            lifecycle_id: "L-001".into(),
            timestamp: "2026-05-24T00:00:00Z".into(),
            bridge_id: Some("TEST-ACTIVE-001".into()),
            proof_semantics: Some(ProofSemantics {
                proven_content: "0xabc".into(),
                source_claim_type: "export".into(),
                destination_claim_type: "import".into(),
                classification_binding_method: "context_inference".into(),
                classification_verified: false,
            }),
            economic_consistency: None, // <-- absent on Class 5 / Confidence B
            ..ArrowEvent::default()
        };
        let regs = regs_with_active_bridge();

        let result = validate_bridge_rules(&msg, &event, &regs);

        assert!(
            result.violations.iter().any(|v| v.rule_id == "SEM-019"),
            "SEM-019 must fire when economic_consistency absent on Class 5 / Confidence B"
        );
    }

    /// Class 5, Confidence C — SEM-019 does not apply (below B threshold).
    #[test]
    fn sem019_class5_confidence_c_not_required() {
        let msg = parse("5:ORC:FN:TR:0:C").unwrap();
        let event = ArrowEvent {
            arrow_code: "5:ORC:FN:TR:0:C".into(),
            event_id: "E-011".into(),
            lifecycle_id: "L-001".into(),
            timestamp: "2026-05-24T00:00:00Z".into(),
            bridge_id: Some("TEST-ACTIVE-001".into()),
            proof_semantics: Some(ProofSemantics {
                proven_content: "0xabc".into(),
                source_claim_type: "export".into(),
                destination_claim_type: "import".into(),
                classification_binding_method: "context_inference".into(),
                classification_verified: false,
            }),
            economic_consistency: None, // absent — should be OK at Confidence C
            ..ArrowEvent::default()
        };
        let regs = regs_with_active_bridge();

        let result = validate_bridge_rules(&msg, &event, &regs);

        assert!(
            !result.violations.iter().any(|v| v.rule_id == "SEM-019"),
            "SEM-019 must not fire at Confidence C"
        );
    }

    /// consistency_verified=true but no audit_ref → SEM-019.
    #[test]
    fn sem019_verified_without_audit_ref_fails() {
        let msg = parse("5:ORC:FN:TR:1:B").unwrap();
        let event = ArrowEvent {
            arrow_code: "5:ORC:FN:TR:1:B".into(),
            event_id: "E-012".into(),
            lifecycle_id: "L-001".into(),
            timestamp: "2026-05-24T00:00:00Z".into(),
            bridge_id: Some("TEST-ACTIVE-001".into()),
            proof_semantics: Some(ProofSemantics {
                proven_content: "0xabc".into(),
                source_claim_type: "export".into(),
                destination_claim_type: "import".into(),
                classification_binding_method: "context_inference".into(),
                classification_verified: false,
            }),
            economic_consistency: Some(EconomicConsistency {
                source_value: "100.00".into(),
                destination_value: "100.00".into(),
                consistency_verified: true, // verified
            }),
            economic_consistency_audit_ref: None, // <-- but no audit ref
            ..ArrowEvent::default()
        };
        let regs = regs_with_active_bridge();

        let result = validate_bridge_rules(&msg, &event, &regs);

        assert!(
            result.violations.iter().any(|v| v.rule_id == "SEM-019"
                && v.field.as_deref() == Some("economic_consistency_audit_ref")),
            "SEM-019 must require audit_ref when consistency_verified=true at Confidence B"
        );
    }

    /// Full valid bridge event at Confidence C — all rules pass.
    #[test]
    fn valid_bridge_event_confidence_c_passes_all_rules() {
        let msg = parse("2:ORC:FN:PR:0:C").unwrap();
        let event = ArrowEvent {
            arrow_code: "2:ORC:FN:PR:0:C".into(),
            event_id: "E-013".into(),
            lifecycle_id: "L-001".into(),
            timestamp: "2026-05-24T00:00:00Z".into(),
            bridge_id: Some("TEST-ACTIVE-001".into()),
            proof_semantics: Some(ProofSemantics {
                proven_content: "0xabc".into(),
                source_claim_type: "supplemental_export_output".into(),
                destination_claim_type: "supplemental_export_output".into(),
                classification_binding_method: "context_inference".into(),
                classification_verified: false,
            }),
            economic_consistency: None, // not required at Confidence C for Class 2
            ..ArrowEvent::default()
        };
        let regs = regs_with_active_bridge();

        let result = validate_bridge_rules(&msg, &event, &regs);

        // Should have no SEM-017/018/019/BRG-001/003/006 violations
        let fatal_rules = [
            "BRG-001", "BRG-003", "BRG-006", "SEM-017", "MSG-005", "SEM-019",
        ];
        let fatal_violations: Vec<_> = result
            .violations
            .iter()
            .filter(|v| fatal_rules.contains(&v.rule_id.as_str()))
            .collect();
        assert!(
            fatal_violations.is_empty(),
            "No fatal bridge violations expected for valid Confidence C event: {:?}",
            fatal_violations
        );
        assert!(result.forced_action.is_none());
    }
}
