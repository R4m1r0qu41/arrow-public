use crate::{
    registry::loader::Registries, types::message::ArrowMessage, validation::common::confidence_rank,
};

use super::Violation;

pub(super) fn validate_semantic(msg: &ArrowMessage, registries: &Registries) -> Vec<Violation> {
    let mut v = Vec::new();

    // SEM-005 — Class 6 (CLOSE) MUST use Action CL.
    // This is SEM-005's single pinned obligation (rule-surface slug
    // `class6_requires_cl`). The finality-1 requirement for closures is not a
    // separate SEM-005 obligation: it is derived — SEM-005 forces Action CL,
    // and SEM-013 rejects any executing Action (incl. CL) at Finality=0.
    if msg.class.as_u8() == 6 && msg.action.as_str() != "CL" {
        v.push(Violation::new(
            "SEM-005",
            Some("action"),
            format!(
                "Class 6 (CLOSE) MUST use Action CL; got `{}`",
                msg.action.as_str()
            ),
        ));
    }

    // SEM-020 (v1.6) — Closure Class Binding: Action CL MUST use Class 6.
    // Converse of SEM-005. The pair is redundant-and-checked: both fields must
    // agree before a consumer may treat an event as lifecycle-terminal, so a
    // single-field corruption cannot present terminal semantics on a
    // non-terminal class.
    if msg.action.as_str() == "CL" && msg.class.as_u8() != 6 {
        v.push(Violation::new(
            "SEM-020",
            Some("action"),
            format!(
                "Action CL requires Class 6 (CLOSE); got Class {}",
                msg.class.as_u8()
            ),
        ));
    }

    // SEM-006 — Class 3 (CONDITION) MUST use a non-executing action.
    if msg.class.as_u8() == 3 {
        let action = msg.action.as_str();
        if action != "BL" && action != "HL" && action != "RW" {
            v.push(Violation::new(
                "SEM-006",
                Some("action"),
                format!("Class 3 (CONDITION) MUST use Action BL, HL, or RW; got `{action}`"),
            ));
        }
    }

    // SEM-001 — Finality=1 requires State FN or RV. A claim of settled
    // finality on a state that is by definition unsettled is incoherent.
    if msg.finality == 1 {
        let st = msg.state.as_str();
        if st != "FN" && st != "RV" {
            v.push(Violation::new(
                "SEM-001",
                Some("state"),
                format!("Finality=1 requires State FN or RV; got `{st}`"),
            ));
        }
    }

    // SEM-002/003/004 — PD, TL, and CD are non-final states: Finality MUST
    // be 0. Kept as three rule IDs (not folded into SEM-001) because the
    // spec assigns each state its own permanent rule slot.
    if msg.finality == 1 {
        let (rule, name) = match msg.state.as_str() {
            "PD" => ("SEM-002", "PD (Pending)"),
            "TL" => ("SEM-003", "TL (Timelocked)"),
            "CD" => ("SEM-004", "CD (Conditional)"),
            _ => ("", ""),
        };
        if !rule.is_empty() {
            v.push(Violation::new(
                rule,
                Some("finality"),
                format!("State {name} requires Finality=0"),
            ));
        }
    }

    // SEM-007 — Class 5 (TRANSFER) MUST use Action TR or PR. A transfer that
    // blocks, holds, reviews, or closes is not a transfer.
    if msg.class.as_u8() == 5 {
        let action = msg.action.as_str();
        if action != "TR" && action != "PR" {
            v.push(Violation::new(
                "SEM-007",
                Some("action"),
                format!("Class 5 (TRANSFER) MUST use Action TR or PR; got `{action}`"),
            ));
        }
    }

    // TRG-001 (class direction) — Action TR is permitted in Classes
    // {2 INSTRUCT, 5 TRANSFER} only (RFC-010 Part 1 / RFC-008 §2.3). The
    // envelope clauses of TRG-001 are event-level; this clause is pure
    // six-field and enumerable.
    if msg.action.as_str() == "TR" && msg.class.as_u8() != 2 && msg.class.as_u8() != 5 {
        v.push(Violation::new(
            "TRG-001",
            Some("action"),
            format!(
                "Action TR is permitted in Classes 2 (INSTRUCT) and 5 (TRANSFER) only; got Class {}",
                msg.class.as_u8()
            ),
        ));
    }

    // SEM-009 — confidence E must not pair with Action PR or TR
    if msg.confidence.as_str() == "E"
        && (msg.action.as_str() == "PR" || msg.action.as_str() == "TR")
    {
        v.push(Violation::new(
            "SEM-009",
            Some("confidence"),
            "confidence E must not pair with Action PR or TR",
        ));
    }

    // SEM-013 — FN+0 Executing-Action Ceiling (v1.5.1).
    // Finality=0 events MUST NOT carry an executing Action ({PR, TR, CL}).
    // Origin-agnostic, protocol-wide.
    if msg.finality == 0 {
        let action = msg.action.as_str();
        if action == "PR" || action == "TR" || action == "CL" {
            v.push(Violation::new(
                "SEM-013",
                Some("action"),
                format!(
                    "Finality=0 (FN+0) MUST NOT carry executing Action; got `{action}` (permitted at Finality=0: BL, HL, RW)"
                ),
            ));
        }
    }

    // SEM-012 — class 6 (CLOSE) must have State FN
    if msg.class.as_u8() == 6 && msg.state.as_str() != "FN" {
        v.push(Violation::new(
            "SEM-012",
            Some("state"),
            format!(
                "class 6 (CLOSE) requires State FN — got `{}`",
                msg.state.as_str()
            ),
        ));
    }

    // Step 10 — SEM-015: confidence ceiling vs verification regime.
    // Runs after bridge rules (Steps 3a–9) because bridge rules may downgrade
    // confidence before this ceiling check.
    //
    // Conditional (pre-settlement) ceiling: regimes that declare
    // `pre_settlement_max_confidence` (e.g. optimistic_settled) cap events at
    // that lower ceiling while Finality=0. The discriminator is the message's
    // own Finality bit — deterministic, event-carried, no clock input. A
    // Finality=0 event under such a regime is by definition still inside its
    // settlement window (FB-001 separately forces it to declare the basis).
    if let Some(origin_entry) = registries.origins.get(msg.origin.as_str()) {
        if let Some(regime_entry) = registries
            .verification_regimes
            .get(origin_entry.verification_regime.as_str())
        {
            let pre_settlement =
                msg.finality == 0 && regime_entry.pre_settlement_max_confidence.is_some();
            let ceiling = if pre_settlement {
                regime_entry
                    .pre_settlement_max_confidence
                    .as_deref()
                    .unwrap()
            } else {
                regime_entry.max_confidence.as_str()
            };
            let conf_rank = confidence_rank(msg.confidence.as_str());
            let ceiling_rank = confidence_rank(ceiling);
            if conf_rank > ceiling_rank {
                v.push(Violation::new(
                    "SEM-015",
                    Some("confidence"),
                    format!(
                        "confidence `{}` exceeds regime `{}` {}ceiling `{}`",
                        msg.confidence.as_str(),
                        origin_entry.verification_regime,
                        if pre_settlement {
                            "pre-settlement (Finality=0) "
                        } else {
                            ""
                        },
                        ceiling
                    ),
                ));
            }
        }
    }

    v
}
