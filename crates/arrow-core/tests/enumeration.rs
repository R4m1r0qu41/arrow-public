//! Exhaustive six-field enumeration — total-truth properties.
//!
//! Enumerates the ENTIRE message tuple space (every registered Class ×
//! Origin × State × Action × {0,1} Finality × Confidence) and asserts the
//! spec's message-level MUSTs as theorems over the real validator — total
//! truths, not sampled tests.
//!
//! Scope: message-level (`validate_message`) rules only. Rules keyed on
//! event/context fields (FB-001, the LEG family, SEM-014..019, BRG-*,
//! LFC-*, CHN-001, and the SEM-015 event extension) are NOT covered here;
//! they have their own fixtures. SEM-008 is a SHOULD, not a total truth,
//! and is deliberately absent.
//!
//! The oracle is independent of the implementation: each theorem restates
//! the spec sentence (docs/protocol/validation-rules.md, Semantic
//! Constraints table) using only registry data and a locally-defined
//! confidence order — it never calls back into validator internals.

use arrow_core::{
    parser::message::parse, registry::loader::Registries, validation::pipeline::ValidationPipeline,
};

/// Spec confidence order (A strongest … E weakest), defined locally so the
/// theorem does not borrow the implementation's ranking.
fn rank(c: &str) -> u8 {
    match c {
        "A" => 5,
        "B" => 4,
        "C" => 3,
        "D" => 2,
        "E" => 1,
        _ => 0,
    }
}

const EXECUTING: [&str; 3] = ["PR", "TR", "CL"];

#[test]
fn six_field_total_truths() {
    let registries = Registries::load_embedded().expect("embedded registries must load");
    let pipeline = ValidationPipeline::new(&registries);

    let mut classes: Vec<u8> = registries.classes.keys().copied().collect();
    let mut origins: Vec<&str> = registries.origins.keys().map(|s| s.as_str()).collect();
    let mut states: Vec<&str> = registries.states.keys().map(|s| s.as_str()).collect();
    let mut actions: Vec<&str> = registries.actions.keys().map(|s| s.as_str()).collect();
    let mut confidences: Vec<&str> = registries.confidence.keys().map(|s| s.as_str()).collect();
    classes.sort_unstable();
    origins.sort_unstable();
    states.sort_unstable();
    actions.sort_unstable();
    confidences.sort_unstable();

    let expected_total =
        classes.len() * origins.len() * states.len() * actions.len() * 2 * confidences.len();

    let mut total = 0usize;
    let mut passed = 0usize;
    let mut failures: Vec<String> = Vec::new();
    fn check(failures: &mut Vec<String>, cond: bool, code: &str, theorem: &str) {
        if !cond {
            failures.push(format!("{code} violates: {theorem}"));
        }
    }

    for &class in &classes {
        for &origin in &origins {
            for &state in &states {
                for &action in &actions {
                    for finality in [0u8, 1u8] {
                        for &conf in &confidences {
                            let code =
                                format!("{class}:{origin}:{state}:{action}:{finality}:{conf}");
                            total += 1;

                            let msg = match parse(&code) {
                                Ok(m) => m,
                                Err(e) => {
                                    failures.push(format!(
                                        "{code} failed to parse a registry-valid tuple: {e}"
                                    ));
                                    continue;
                                }
                            };
                            let result = pipeline.validate_message(&msg);

                            // Structural: a rejection must be explained by at
                            // least one Error-severity violation.
                            if !result.passed {
                                check(
                                    &mut failures,
                                    result
                                        .violations
                                        .iter()
                                        .any(|v| format!("{:?}", v.severity) == "Error"),
                                    &code,
                                    "rejected without any Error violation",
                                );
                                continue;
                            }
                            passed += 1;

                            // T1 / SEM-013: no executing Action passes with Finality 0.
                            check(
                                &mut failures,
                                !(finality == 0 && EXECUTING.contains(&action)),
                                &code,
                                "SEM-013: executing Action passed at Finality=0",
                            );

                            // T2 / SEM-015: Confidence never exceeds the regime's
                            // effective ceiling (pre-settlement ceiling at Finality=0
                            // where the regime declares one).
                            let regime = &registries.origins[origin].verification_regime;
                            let entry = &registries.verification_regimes[regime];
                            let ceiling = if finality == 0 {
                                entry
                                    .pre_settlement_max_confidence
                                    .as_deref()
                                    .unwrap_or(&entry.max_confidence)
                            } else {
                                &entry.max_confidence
                            };
                            check(
                                &mut failures,
                                rank(conf) <= rank(ceiling),
                                &code,
                                "SEM-015: Confidence above the regime's effective ceiling passed",
                            );

                            // T3 / SEM-005+012+013 (derived closure shape): a passing
                            // Class 6 is exactly 6:*:FN:CL:1:*.
                            check(
                                &mut failures,
                                class != 6 || (action == "CL" && state == "FN" && finality == 1),
                                &code,
                                "closure shape: Class 6 passed without FN/CL/Finality-1",
                            );

                            // T4 / SEM-020: Action CL only passes on Class 6.
                            check(
                                &mut failures,
                                action != "CL" || class == 6,
                                &code,
                                "SEM-020: Action CL passed outside Class 6",
                            );

                            // T5 / SEM-006: Class 3 passes only with BL/HL/RW.
                            check(
                                &mut failures,
                                class != 3 || ["BL", "HL", "RW"].contains(&action),
                                &code,
                                "SEM-006: Class 3 passed with an executing Action",
                            );

                            // T6 / SEM-007: Class 5 passes only with TR/PR.
                            check(
                                &mut failures,
                                class != 5 || ["TR", "PR"].contains(&action),
                                &code,
                                "SEM-007: Class 5 passed with a non-transfer Action",
                            );

                            // T7 / SEM-009: Confidence E never passes with PR/TR.
                            check(
                                &mut failures,
                                conf != "E" || !["PR", "TR"].contains(&action),
                                &code,
                                "SEM-009: Confidence E passed with PR/TR",
                            );

                            // T8 / SEM-001: Finality 1 passes only in FN or RV.
                            check(
                                &mut failures,
                                finality != 1 || ["FN", "RV"].contains(&state),
                                &code,
                                "SEM-001: Finality=1 passed outside FN/RV",
                            );

                            // T10 / TRG-001 (class clause): Action TR passes only
                            // in Classes 2 (INSTRUCT) and 5 (TRANSFER). The
                            // envelope clauses are event-level and fixture-covered.
                            check(
                                &mut failures,
                                action != "TR" || class == 2 || class == 5,
                                &code,
                                "TRG-001: Action TR passed outside Classes {2,5}",
                            );

                            // T9 / SEM-002/003/004: non-final states pass only at
                            // Finality 0. (Implied by T8 over the current state
                            // registry; asserted separately because the spec gives
                            // each state its own permanent rule slot.)
                            check(
                                &mut failures,
                                !["PD", "TL", "CD"].contains(&state) || finality == 0,
                                &code,
                                "SEM-002/003/004: non-final state passed at Finality=1",
                            );
                        }
                    }
                }
            }
        }
    }

    assert_eq!(
        total, expected_total,
        "enumeration did not cover the full registry-defined tuple space"
    );
    assert!(
        failures.is_empty(),
        "{} of {} tuples violate total-truth properties ({} passed overall):\n{}",
        failures.len(),
        total,
        passed,
        failures[..failures.len().min(40)].join("\n")
    );
    println!(
        "enumerated {total} tuples ({passed} pass / {} reject) — all total-truth properties hold",
        total - passed
    );
}
