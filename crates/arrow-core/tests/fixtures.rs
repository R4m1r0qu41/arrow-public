//! Data-driven regression suite for Arrow Protocol event fixtures.
//!
//! Driven by `tests/fixtures/index.json` at the workspace root.
//! Each fixture entry names a JSON file containing a full `ArrowEvent` and
//! declares the expected validation outcome, expected violation rule IDs,
//! and an optional expected reason-code substring.
//!
//! Run with:
//!   cargo test -p arrow-core --test fixtures

use std::path::{Path, PathBuf};

use arrow_core::{
    registry::loader::Registries,
    types::event::ArrowEvent,
    validation::pipeline::{LifecycleContext, ValidationPipeline},
};
use serde::Deserialize;

// ── Index schema ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct FixtureIndex {
    fixtures: Vec<FixtureSpec>,
}

/// One entry in `index.json`.
#[derive(Deserialize)]
struct FixtureSpec {
    /// Short identifier used in failure messages (e.g. "F-01").
    id: String,
    /// Filename relative to the fixtures directory (e.g. "F-01-pass-native-canonical.json").
    file: String,
    /// `"pass"` or `"reject"`.
    expected_result: String,
    /// Rule IDs that must appear in `result.violations` (e.g. ["BRG-001"]).
    #[serde(default)]
    expected_violations: Vec<String>,
    /// If present, at least one violation message must contain this substring
    /// (e.g. "BRIDGE_TRUST_REVOKED").
    #[serde(default)]
    expected_reason_code: Option<String>,
    /// Context-bearing fixtures (LFC rules): files whose events are loaded
    /// into a LifecycleContext BEFORE validating this fixture. Context files
    /// are prior lifecycle history, not test cases themselves.
    #[serde(default)]
    context_files: Vec<String>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn fixtures_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR = crates/arrow-core/
    // fixtures live two levels up at tests/fixtures/
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures")
}

// ── Suite ─────────────────────────────────────────────────────────────────────

#[test]
fn fixture_regression_suite() {
    let dir = fixtures_dir();

    // Load index
    let index_path = dir.join("index.json");
    let index_raw = std::fs::read_to_string(&index_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", index_path.display()));
    let index: FixtureIndex =
        serde_json::from_str(&index_raw).unwrap_or_else(|e| panic!("cannot parse index.json: {e}"));

    // Shared pipeline backed by the embedded (compiled-in) registry snapshot.
    let registries = Registries::load_embedded().expect("embedded registries must load");
    let pipeline = ValidationPipeline::new(&registries);

    let mut failures: Vec<String> = Vec::new();

    for spec in &index.fixtures {
        let fixture_path = dir.join(&spec.file);

        // Read file
        let raw = match std::fs::read_to_string(&fixture_path) {
            Ok(s) => s,
            Err(e) => {
                failures.push(format!(
                    "[{}] cannot read {}: {e}",
                    spec.id,
                    fixture_path.display()
                ));
                continue;
            }
        };

        // Deserialize event
        let event: ArrowEvent = match serde_json::from_str(&raw) {
            Ok(e) => e,
            Err(e) => {
                failures.push(format!(
                    "[{}] cannot deserialize {}: {e}",
                    spec.id, spec.file
                ));
                continue;
            }
        };

        // Validate — with lifecycle context when the spec declares prior events.
        let result = if spec.context_files.is_empty() {
            pipeline.validate_event(&event)
        } else {
            let mut ctx = LifecycleContext::default();
            let mut ctx_err = false;
            for cf in &spec.context_files {
                let cp = dir.join(cf);
                let prior: ArrowEvent = match std::fs::read_to_string(&cp)
                    .map_err(|e| e.to_string())
                    .and_then(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
                {
                    Ok(e) => e,
                    Err(e) => {
                        failures.push(format!("[{}] bad context file {cf}: {e}", spec.id));
                        ctx_err = true;
                        break;
                    }
                };
                ctx.processed_event_ids.insert(prior.event_id.clone());
                ctx.lifecycle_events
                    .entry(prior.lifecycle_id.clone())
                    .or_default()
                    .push(prior);
            }
            if ctx_err {
                continue;
            }
            pipeline.validate_event_with_context(&event, &ctx)
        };

        let violation_ids: Vec<&str> = result
            .violations
            .iter()
            .map(|v| v.rule_id.as_str())
            .collect();

        // Check pass/reject outcome
        let expected_pass = spec.expected_result == "pass";
        if result.passed != expected_pass {
            failures.push(format!(
                "[{}] expected `{}`, got `{}` — violations: {:?}",
                spec.id,
                spec.expected_result,
                if result.passed { "pass" } else { "reject" },
                violation_ids
            ));
            // Don't check sub-assertions when the top-level outcome is already wrong.
            continue;
        }

        // Check each expected violation rule ID is present
        for rule_id in &spec.expected_violations {
            if !violation_ids.contains(&rule_id.as_str()) {
                failures.push(format!(
                    "[{}] expected violation `{rule_id}` was not fired — violations: {:?}",
                    spec.id, violation_ids
                ));
            }
        }

        // Check expected reason code appears in at least one violation message
        if let Some(code) = &spec.expected_reason_code {
            let found = result
                .violations
                .iter()
                .any(|v| v.message.contains(code.as_str()));
            if !found {
                failures.push(format!(
                    "[{}] reason code `{code}` not found in any violation message — messages: {:?}",
                    spec.id,
                    result
                        .violations
                        .iter()
                        .map(|v| v.message.as_str())
                        .collect::<Vec<_>>()
                ));
            }
        }
    }

    // Single combined failure report
    if !failures.is_empty() {
        panic!(
            "\n{} fixture(s) failed:\n  {}",
            failures.len(),
            failures.join("\n  ")
        );
    }
}
