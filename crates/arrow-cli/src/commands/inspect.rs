use anyhow::{Context, Result};
use arrow_core::{parser::message::parse, registry::loader::Registries};

pub fn run(arrow_code: &str) -> Result<()> {
    let msg = parse(arrow_code).with_context(|| format!("parse error for `{arrow_code}`"))?;

    let registries = Registries::load_embedded().context("failed to load embedded registries")?;

    let class_name = registries
        .classes
        .get(&msg.class.as_u8())
        .map(|e| e.name.as_str())
        .unwrap_or("unknown");
    let origin_name = registries
        .origins
        .get(msg.origin.as_str())
        .map(|e| e.name.as_str())
        .unwrap_or("unknown");
    let state_name = registries
        .states
        .get(msg.state.as_str())
        .map(|e| e.name.as_str())
        .unwrap_or("unknown");
    let action_name = registries
        .actions
        .get(msg.action.as_str())
        .map(|e| e.name.as_str())
        .unwrap_or("unknown");
    let conf_name = registries
        .confidence
        .get(msg.confidence.as_str())
        .map(|e| e.name.as_str())
        .unwrap_or("unknown");
    let finality_label = if msg.finality == 1 {
        "Final (1)"
    } else {
        "Pending (0)"
    };

    println!("Arrow Code  : {}", msg.raw);
    println!("─────────────────────────────────────────");
    println!("  Class      : {} — {class_name}", msg.class.as_u8());
    println!("  Origin     : {} — {origin_name}", msg.origin);
    println!("  State      : {} — {state_name}", msg.state.as_str());
    println!("  Action    : {} — {action_name}", msg.action.as_str());
    println!("  Finality   : {finality_label}");
    println!("  Confidence : {} — {conf_name}", msg.confidence.as_str());
    Ok(())
}
