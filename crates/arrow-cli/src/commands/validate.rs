use anyhow::{Context, Result};
use arrow_core::{
    registry::loader::Registries,
    types::event::ArrowEvent,
    validation::pipeline::{LifecycleContext, ValidationPipeline, ViolationSeverity},
};
use std::path::{Path, PathBuf};

pub fn run(path: &Path, json_output: bool, context_files: &[PathBuf]) -> Result<bool> {
    let raw =
        std::fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
    let event: ArrowEvent = serde_json::from_str(&raw)
        .with_context(|| format!("cannot parse {} as ArrowEvent", path.display()))?;
    let registries = Registries::load_embedded().context("failed to load embedded registries")?;
    let pipeline = ValidationPipeline::new(&registries);
    let result = if context_files.is_empty() {
        pipeline.validate_event(&event)
    } else {
        let mut ctx = LifecycleContext::default();
        for cf in context_files {
            let craw = std::fs::read_to_string(cf)
                .with_context(|| format!("cannot read context file {}", cf.display()))?;
            let prior: ArrowEvent = serde_json::from_str(&craw)
                .with_context(|| format!("cannot parse {} as ArrowEvent", cf.display()))?;
            ctx.processed_event_ids.insert(prior.event_id.clone());
            ctx.lifecycle_events
                .entry(prior.lifecycle_id.clone())
                .or_default()
                .push(prior);
        }
        pipeline.validate_event_with_context(&event, &ctx)
    };

    if json_output {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        let status = if result.passed { "PASSED" } else { "FAILED" };
        println!("[{status}]  {}", path.display());
        if let Some(fp) = &result.forced_action {
            println!("  forced_action: {fp}");
        }
        for v in &result.violations {
            let sev = if v.severity == ViolationSeverity::Error {
                "ERROR"
            } else {
                "WARN "
            };
            let field = v.field.as_deref().unwrap_or("—");
            println!("  [{sev}] {} | field: {field} | {}", v.rule_id, v.message);
        }
    }
    Ok(result.passed)
}
