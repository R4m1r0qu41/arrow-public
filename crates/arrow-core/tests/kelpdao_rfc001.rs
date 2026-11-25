// Phase 0 Step 0.1: migrated from tests/integration/ to flat layout.
// v1.5 compat patches applied (see comments inline):
//   - include_str! path corrected (one fewer ../ after moving up one directory level)
//   - RFC-001 events reference chain_id "ARB-1" (pre-v2.0 format); chain_registry
//     v2.0 uses "ARB". Event validation resolves instance IDs by canonical prefix.
//   - RFC-001's LEG event predates the v1.5 LEG schema/jurisdiction requirements.
//     leg_subtype and leg_jurisdiction are set on LEG-origin events before validation so the
//     lifecycle assertion (the purpose of this test) is not obscured by a v1.5
//     schema requirement that didn't exist when the RFC was authored.
use arrow_core::{
    registry::loader::Registries,
    types::event::{ArrowEvent, LegJurisdiction},
    validation::pipeline::{LifecycleContext, ValidationPipeline},
};

fn extract_json_events() -> Vec<ArrowEvent> {
    // Path is three levels up from crates/arrow-core/tests/ to repo root.
    let md = include_str!("../../../rfc/RFC-001-KelpDAO.md");
    md.split("```json")
        .skip(1)
        .filter_map(|chunk| chunk.split("```").next())
        .map(|json| {
            serde_json::from_str::<ArrowEvent>(json.trim())
                .expect("RFC-001 event JSON must deserialize")
        })
        .collect()
}

#[test]
fn kelpdao_rfc001_four_event_lifecycle_passes() {
    let registries = Registries::load_embedded().unwrap();

    let pipeline = ValidationPipeline::new(&registries);
    let events = extract_json_events();
    assert_eq!(events.len(), 4);

    let mut ctx = LifecycleContext::default();
    for mut event in events {
        // RFC-001's LEG-origin event predates the v1.5/v1.5.1 LEG structured fields.
        // Fill the minimum valid JUD jurisdiction shape so this lifecycle test is not
        // blocked by rules that postdate the RFC.
        if event.arrow_code.contains(":LEG:") {
            if event.leg_subtype.is_none() {
                event.leg_subtype = Some("JUD".to_string());
            }
            if event.leg_jurisdiction.is_none() {
                event.jurisdiction = Some("MX".to_string());
                event.leg_jurisdiction = Some(LegJurisdiction {
                    country: Some("MX".to_string()),
                    supra: None,
                });
            }
        }

        // RFC-001's TR event (E3) predates TRG-001 (RFC-010, 2026-07). Fill a
        // minimal valid action_target from its own related_events so this
        // lifecycle test is not blocked by a rule that postdates the RFC —
        // same compat treatment as the LEG structured fields above.
        if event.arrow_code.contains(":TR:") && event.action_target.is_none() {
            event.action_target = Some(arrow_core::types::event::ActionTarget {
                target_ref: "kelpdao:withdrawal-workflow".to_string(),
                object_ref: event.object_id.clone(),
                authorizing_events: event.related_events.clone(),
            });
        }

        let result = pipeline.validate_event_with_context(&event, &ctx);
        assert!(result.passed, "{}: {:?}", event.event_id, result.violations);
        for related in &event.related_events {
            assert!(
                ctx.processed_event_ids.contains(related),
                "{} missing related {}",
                event.event_id,
                related
            );
        }
        ctx.processed_event_ids.insert(event.event_id.clone());
        ctx.lifecycle_events
            .entry(event.lifecycle_id.clone())
            .or_default()
            .push(event);
    }
}
