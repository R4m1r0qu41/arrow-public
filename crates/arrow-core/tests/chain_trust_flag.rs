// Phase 0 Step 0.1: migrated from tests/integration/ to flat layout.
// Schema fixes for chain_registry v2.0:
//   - Registry key is "BTC" (event instance IDs like "BTC-1" resolve to it)
//   - chain_trust_flag is now nested under protocol_required
//   - chain_status → status; chain_status_reason/effective_from removed from v2.0 struct
//   - ValidationResult.forced_action is Option<String>, not an Action wrapper
use arrow_core::{
    registry::loader::Registries, types::event::ArrowEvent,
    validation::pipeline::ValidationPipeline,
};

#[test]
fn sem_016_forces_bl_when_chain_untrusted() {
    let mut registries = Registries::load_embedded().unwrap();

    // chain_registry v2.0 key is the 3-char code "BTC".
    let entry = registries.chain_registry.get_mut("BTC").unwrap();
    entry.protocol_required.chain_trust_flag = 0;
    entry.status = "compromised".to_owned();
    // chain_status_reason and chain_status_effective_from were v1.x flat fields;
    // removed in v2.0. The trust flag alone gates SEM-016.

    let raw_event = serde_json::json!({
        "event_id": "ARR-20260518-SEM016-TEST",
        "lifecycle_id": "LFC-TEST-001",
        "related_events": [],
        "arrow_code": "1:BTC:FN:PR:1:A",
        "object_type": "asset",
        "object_id": "btc:tx:test",
        "reason_code": "DEP_CONFIRMED",
        "timestamp": "2026-05-18T00:00:00Z",
        // Instance-style chain IDs are accepted and resolved through their canonical prefix.
        "chain_id": "BTC-1"
    });

    let event: ArrowEvent = serde_json::from_value(raw_event).unwrap();
    let result = ValidationPipeline::new(&registries).validate_event(&event);

    assert!(!result.passed);
    // forced_action is Option<String> in ValidationResult (not an Action wrapper).
    assert_eq!(result.forced_action, Some("BL".to_string()));
    assert_eq!(result.violations[0].rule_id, "SEM-016");
    assert_eq!(result.violations.len(), 1);
}
