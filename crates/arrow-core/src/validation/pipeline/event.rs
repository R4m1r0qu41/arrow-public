use crate::{
    registry::loader::Registries,
    types::{event::ArrowEvent, message::ArrowMessage, registry::ChainRegistryEntry},
    validation::bridge::validate_bridge_rules,
};

use super::{message::run_message, LifecycleContext, ValidationResult, Violation};

// ── Event-level validation (Steps 3, 3a–3b, 6–9, 11–12) ─────────────────

pub(super) fn run_event(
    msg: &ArrowMessage,
    event: &ArrowEvent,
    context: Option<&LifecycleContext>,
    registries: &Registries,
) -> ValidationResult {
    // Message-level validation first.
    let mut result = run_message(msg, registries);
    if !result.passed {
        return result;
    }

    // Step 3 — SEM-016: chain trust flag override.
    // chain_trust_flag == 0 forces Action BL — non-overridable.
    if let Some(chain_id) = &event.chain_id {
        if let Some((registry_key, chain_entry)) = resolve_chain_entry(registries, chain_id) {
            if chain_entry.protocol_required.chain_trust_flag == 0 {
                return ValidationResult {
                    passed: false,
                    violations: vec![Violation::new(
                        "SEM-016",
                        Some("action"),
                        format!(
                            "chain `{chain_id}` resolves to registry entry `{registry_key}` \
                             with chain_trust_flag=0 (status: {}); \
                             action forced to BL — non-overridable",
                            chain_entry.status
                        ),
                    )],
                    forced_action: Some("BL".to_string()),
                };
            }
        } else {
            result.violations.push(Violation::new(
                "CHN-001",
                Some("chain_id"),
                format!("chain_id `{chain_id}` is not registered in the Chain Registry"),
            ));
        }
    }

    // Steps 3a–3b, 6–9 — Bridge Registry rules (BRG-001–006) and
    // cross-domain semantic consistency rules (SEM-017, MSG-005, SEM-019).
    // Called only when event.bridge_id is Some; returns immediately otherwise.
    {
        let bridge_result = validate_bridge_rules(msg, event, registries);

        // BRG-001 hard block: forced_action signals a non-overridable BL override.
        if bridge_result.forced_action.is_some() {
            return ValidationResult {
                passed: false,
                violations: bridge_result.violations,
                forced_action: bridge_result.forced_action,
            };
        }

        // Accumulate non-blocking bridge violations (BRG-003, BRG-006, SEM-017, MSG-005, SEM-019).
        result.violations.extend(bridge_result.violations);
    }

    // FB-001 — finality_basis required when finality == 0.
    if msg.finality == 0 {
        match &event.finality_basis {
            None => {
                result.violations.push(Violation::new(
                    "FB-001",
                    Some("finality_basis"),
                    "finality_basis is required when finality == 0",
                ));
            }
            Some(basis) => match registries.finality_basis.get(basis.as_str()) {
                None => {
                    result.violations.push(Violation::new(
                        "FB-001",
                        Some("finality_basis"),
                        format!("finality_basis `{basis}` is not a registered value"),
                    ));
                }
                // Spec: if the registered basis sets requires_finality_eta,
                // finality_eta MUST also be present. Without it a pending
                // window is undated — the event carries no reproducible
                // evidence of when settlement is due.
                Some(entry) if entry.requires_finality_eta && event.finality_eta.is_none() => {
                    result.violations.push(Violation::new(
                        "FB-001",
                        Some("finality_eta"),
                        format!("finality_basis `{basis}` requires finality_eta, which is absent"),
                    ));
                }
                Some(_) => {}
            },
        }
    }

    // SEM-015 (event extension) — settled-claim consistency for conditional-
    // ceiling regimes. A Finality=1 event claiming confidence above the
    // regime's pre-settlement ceiling asserts its window has closed. When the
    // event itself carries a dated window close (finality_eta as unix_ts), the
    // event's own timestamp must not precede it. Both operands are event-
    // carried: validation stays V(event, registry_snapshot) — deterministic,
    // reproducible, no ambient clock.
    if msg.finality == 1 {
        if let Some(regime_entry) = registries.origins.get(msg.origin.as_str()).and_then(|o| {
            registries
                .verification_regimes
                .get(o.verification_regime.as_str())
        }) {
            if let Some(pre_ceiling) = regime_entry.pre_settlement_max_confidence.as_deref() {
                use crate::validation::common::confidence_rank;
                let claims_settled_confidence =
                    confidence_rank(msg.confidence.as_str()) > confidence_rank(pre_ceiling);
                if claims_settled_confidence
                    && event.finality_eta_type.as_deref() == Some("unix_ts")
                {
                    if let Some(eta) = event.finality_eta {
                        match chrono::DateTime::parse_from_rfc3339(&event.timestamp) {
                            Ok(ts) => {
                                if (ts.timestamp() as i128) < eta as i128 {
                                    result.violations.push(Violation::new(
                                        "SEM-015",
                                        Some("confidence"),
                                        format!(
                                            "confidence `{}` claims settled ceiling but event timestamp `{}` precedes finality_eta window close ({eta})",
                                            msg.confidence.as_str(),
                                            event.timestamp
                                        ),
                                    ));
                                }
                            }
                            // Malformed input rejects, it never skips: this
                            // check is the only thing standing between a
                            // settled-confidence claim and its own dated
                            // window, so an unparseable timestamp cannot be
                            // allowed to disarm it (AGENTS.md: prefer
                            // explicit rejection of malformed input).
                            Err(_) => {
                                result.violations.push(Violation::new(
                                    "SEM-015",
                                    Some("timestamp"),
                                    format!(
                                        "event timestamp `{}` is not valid RFC 3339; settled-claim consistency against finality_eta cannot be verified",
                                        event.timestamp
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    // TRG-001 (envelope clauses) — Action TR must name its target, object,
    // and authorizing instrument (RFC-010 Part 1; class clause lives in
    // validate_semantic). The gate checks SHAPE only — presence, non-empty
    // refs, authorizing_events non-empty and a subset of related_events.
    // Resolvability of target_ref is consumer-side (Tarquin LC-014) per the
    // architect's 2026-07-07 split: V stays free of consumer scope tables.
    if msg.action.as_str() == "TR" {
        match &event.action_target {
            None => {
                result.violations.push(Violation::new(
                    "TRG-001",
                    Some("action_target"),
                    "Action TR requires an action_target envelope (target_ref, object_ref, authorizing_events)",
                ));
            }
            Some(at) => {
                if at.target_ref.trim().is_empty() {
                    result.violations.push(Violation::new(
                        "TRG-001",
                        Some("action_target"),
                        "action_target.target_ref must be non-empty",
                    ));
                }
                if at.object_ref.trim().is_empty() {
                    result.violations.push(Violation::new(
                        "TRG-001",
                        Some("action_target"),
                        "action_target.object_ref must be non-empty",
                    ));
                }
                if at.authorizing_events.is_empty() {
                    result.violations.push(Violation::new(
                        "TRG-001",
                        Some("action_target"),
                        "action_target.authorizing_events must name at least one authorizing event",
                    ));
                } else {
                    for ae in &at.authorizing_events {
                        if !event.related_events.contains(ae) {
                            result.violations.push(Violation::new(
                                "TRG-001",
                                Some("action_target"),
                                format!("authorizing event `{ae}` is not listed in related_events"),
                            ));
                        }
                    }
                }
            }
        }
    }

    // LFC-008 — Closure Expectation Declaration (RFC-010 Part 2A, Universal
    // Eventual Closure). A Class 1 settlement observation (State FN,
    // Finality=1) under a BOUNDED closure discipline must declare when its
    // lifecycle is due to close. An event that fails to close on schedule is
    // possibly under attack; the declaration is what makes "overdue" a
    // computable, passively-monitored condition (consumer side: LC-016).
    // Scope: Class 1 FN only — reversals (State RV) restart the clock via
    // their own lifecycle discipline, closures (Class 6) ARE the close, and
    // attestation_bound / legal_exhaustion regimes carry no fixed deadline.
    if msg.class.as_u8() == 1 && msg.state.as_str() == "FN" && msg.finality == 1 {
        if let Some(regime_entry) = registries.origins.get(msg.origin.as_str()).and_then(|o| {
            registries
                .verification_regimes
                .get(o.verification_regime.as_str())
        }) {
            let bounded = matches!(
                regime_entry.closure_discipline.as_deref(),
                Some("depth_exhaustion") | Some("window_exhaustion")
            );
            if bounded {
                match (&event.closure_eta, &event.closure_eta_type) {
                    (Some(_), Some(_)) => {}
                    _ => {
                        result.violations.push(Violation::new(
                            "LFC-008",
                            Some("closure_eta"),
                            format!(
                                "closure_eta and closure_eta_type are required for settled Class 1 events under bounded closure discipline `{}`",
                                regime_entry.closure_discipline.as_deref().unwrap_or("")
                            ),
                        ));
                    }
                }
            }
        }
    }

    // Step 5b — LEG Action Derivation Gate (v1.5.1-draft).
    //
    // Renumbering vs. v1.5:
    //   v1.5 LEG-006 (leg_subtype controlled vocab) → v1.5.1 MSG-006 (schema constraint).
    //   v1.5 LEG-007 → v1.5.1 LEG-006 (action derivation).
    //   v1.5 LEG-008 → v1.5.1 LEG-007 (SUPRA + SA force-BL).
    //   v1.5 LEG-009 → v1.5.1 LEG-008 (LEG_TEXT/PREC + Finality=1 warning).
    //
    // FN+0 (Finality=0) protocol-wide executing-action ceiling is handled by SEM-013 in
    // validate_semantic(), so LEG-006 enumerates only the Finality=1 permitted-action table.
    if msg.origin == "LEG" {
        const LEG_SUBTYPES: &[&str] = &["JUD", "ADM", "SUPRA", "LEG_TEXT", "PREC"];

        // LEG-009: leg_jurisdiction MUST be present and carry country or supra.
        let has_leg_jurisdiction = event
            .leg_jurisdiction
            .as_ref()
            .map(|j| j.country.is_some() || j.supra.is_some())
            .unwrap_or(false);
        if !has_leg_jurisdiction {
            result.violations.push(Violation::new(
                "LEG-009",
                Some("leg_jurisdiction"),
                "leg_jurisdiction is required when origin = LEG and must include country or supra",
            ));
        }

        // MSG-006: leg_subtype MUST be present and from controlled vocabulary.
        let subtype: Option<&str> = match &event.leg_subtype {
            None => {
                result.violations.push(Violation::new(
                    "MSG-006",
                    Some("leg_subtype"),
                    "leg_subtype is required when origin = LEG",
                ));
                None
            }
            Some(st) if !LEG_SUBTYPES.contains(&st.as_str()) => {
                result.violations.push(Violation::new(
                    "MSG-006",
                    Some("leg_subtype"),
                    format!("leg_subtype `{st}` is not a registered value"),
                ));
                None
            }
            Some(st) => Some(st.as_str()),
        };

        // LEG-004: Confidence=E on LEG origin requires Action=RW.
        if msg.confidence.as_str() == "E" && msg.action.as_str() != "RW" {
            result.violations.push(Violation::new(
                "LEG-004",
                Some("action"),
                format!(
                    "origin=LEG with Confidence=E requires Action=RW; got `{}`",
                    msg.action.as_str()
                ),
            ));
        }

        // LEG-005: Confidence=D on LEG origin requires subtype ∈ {LEG_TEXT, PREC}.
        if msg.confidence.as_str() == "D" {
            if let Some(st) = subtype {
                if st != "LEG_TEXT" && st != "PREC" {
                    result.violations.push(Violation::new(
                        "LEG-005",
                        Some("leg_subtype"),
                        format!(
                            "origin=LEG with Confidence=D requires leg_subtype ∈ {{LEG_TEXT, PREC}}; got `{st}`"
                        ),
                    ));
                }
            }
        }

        // LEG-007: SUPRA + reason_code=SA + Finality=1 forces Action=BL (non-overridable).
        // Runs before LEG-006 so the force-BL branch normalizes Action before the range check.
        let leg_007_forced_bl =
            matches!(subtype, Some("SUPRA")) && msg.finality == 1 && event.reason_code == "SA";
        if leg_007_forced_bl {
            if msg.action.as_str() != "BL" {
                result.violations.push(Violation::new(
                    "LEG-007",
                    Some("action"),
                    format!(
                        "SUPRA + reason_code=SA + Finality=1 forces Action=BL; got `{}` (reason_code LEG_SUPRA_SANCTIONS_FORCE)",
                        msg.action.as_str()
                    ),
                ));
            }
            result.forced_action = Some("BL".to_string());
        }

        // LEG-006: Action derivation gate (Finality=1 permitted-action table).
        // Finality=0 row is covered by SEM-013 protocol-wide. Skip when LEG-007 has
        // already forced BL.
        if !leg_007_forced_bl && msg.finality == 1 {
            if let Some(st) = subtype {
                let permitted: &[&str] = match st {
                    "JUD" => &["PR", "BL", "TR", "CL"],
                    "ADM" => &["PR", "BL", "HL", "CL"],
                    "SUPRA" => &["PR", "HL"],
                    "LEG_TEXT" => &["HL", "RW"],
                    "PREC" => &["RW"],
                    _ => &[],
                };
                if !permitted.contains(&msg.action.as_str()) {
                    result.violations.push(Violation::new(
                        "LEG-006",
                        Some("action"),
                        format!(
                            "Action `{}` is not in the permitted set {:?} for leg_subtype=`{st}` at Finality=1 (reason_code LEG_ACTION_RANGE_VIOLATION)",
                            msg.action.as_str(),
                            permitted
                        ),
                    ));
                }
            }
        }

        // LEG-008: warn when LEG_TEXT/PREC carry Finality=1.
        if matches!(subtype, Some("LEG_TEXT") | Some("PREC")) && msg.finality == 1 {
            result.violations.push(Violation::warning(
                "LEG-008",
                Some("finality"),
                format!(
                    "leg_subtype `{}` with Finality=1 is unusual — SHOULD reference a superseded norm in related_events",
                    subtype.unwrap()
                ),
            ));
        }

        // LEG-010: if supra is present, it MUST be registered and active.
        if let Some(supra) = event
            .leg_jurisdiction
            .as_ref()
            .and_then(|j| j.supra.as_deref())
        {
            match registries.leg_supra_jurisdictions.get(supra) {
                Some(entry) if entry.status == "active" => {}
                Some(entry) => result.violations.push(Violation::new(
                    "LEG-010",
                    Some("leg_jurisdiction.supra"),
                    format!(
                        "leg_jurisdiction.supra `{supra}` is registered but not active (status `{}`)",
                        entry.status
                    ),
                )),
                None => result.violations.push(Violation::new(
                    "LEG-010",
                    Some("leg_jurisdiction.supra"),
                    format!(
                        "leg_jurisdiction.supra `{supra}` is not a registered supra jurisdiction"
                    ),
                )),
            }
        }
    }

    // Lifecycle rules (LFC-001, LFC-003, LFC-007).
    if let Some(ctx) = context {
        // LFC-007 — Post-Closure Lifecycle Discipline (RFC-010 Part 2).
        // If this event's lifecycle already contains a closure (Class 6 —
        // necessarily 6:*:FN:CL:1:* by SEM-005/012/013/020), only the two
        // SEM-014 reopening shapes are valid, and each must cite the closure
        // event_id SPECIFICALLY in related_events (architect, 2026-07-07):
        //   reversal      = Class 1, State RV
        //   supersession  = Class 4 (CONTROL); subtype clause dormant until
        //                   SEM-021 / control_subtypes promotion.
        if let Some(prior) = ctx.lifecycle_events.get(&event.lifecycle_id) {
            let closure = prior
                .iter()
                .find(|e| crate::validation::reopening::is_closure(e));
            if let Some(cl_event) = closure {
                // Shared predicate (validation::reopening) — the same shapes
                // and interim origin constraint the consumer side uses, so
                // gate and consumer cannot drift when SEM-021 promotes the
                // subtype clause.
                if !crate::validation::reopening::is_valid_reopening(event, cl_event) {
                    result.violations.push(Violation::new(
                        "LFC-007",
                        Some("lifecycle_id"),
                        format!(
                            "lifecycle `{}` is closed by event `{}`; after closure only a Class 1 State RV reversal or a Class 4 (CONTROL) supersession citing the closure in related_events — sharing the closure's Origin, or a supersession from PCY — is valid (reason_code POST_CLOSURE_VIOLATION)",
                            event.lifecycle_id, cl_event.event_id
                        ),
                    ));
                }
            }
        }

        if ctx.processed_event_ids.contains(&event.event_id) {
            result.violations.push(Violation::new(
                "LFC-001",
                Some("event_id"),
                format!(
                    "event_id `{}` is not globally unique — rejected as replay attempt",
                    event.event_id
                ),
            ));
        }
        for rel in &event.related_events {
            if !ctx.processed_event_ids.contains(rel.as_str()) {
                result.violations.push(Violation::new(
                    "LFC-003",
                    Some("related_events"),
                    format!("related event `{rel}` not found in lifecycle context"),
                ));
            }
        }
    }

    let passed = !result.violations.iter().any(Violation::is_error);
    ValidationResult { passed, ..result }
}

fn resolve_chain_entry<'a>(
    registries: &'a Registries,
    chain_id: &'a str,
) -> Option<(&'a str, &'a ChainRegistryEntry)> {
    if let Some(entry) = registries.chain_registry.get(chain_id) {
        return Some((chain_id, entry));
    }

    let (prefix, _) = chain_id.split_once('-')?;
    registries
        .chain_registry
        .get_key_value(prefix)
        .map(|(key, entry)| (key.as_str(), entry))
}
