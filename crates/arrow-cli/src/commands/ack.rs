use std::path::Path;

use anyhow::{Context, Result};
use arrow_core::validation::pipeline::ViolationSeverity;
use arrow_transport::envelope::{AckEnvelope, AckResult};

pub fn run(path: &Path) -> Result<()> {
    print!("{}", inspect(path)?);
    Ok(())
}

pub fn inspect(path: &Path) -> Result<String> {
    let ack: AckEnvelope = serde_json::from_str(
        &std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?,
    )
    .with_context(|| format!("parsing AckEnvelope from {}", path.display()))?;
    Ok(format_ack(&ack))
}

pub fn format_ack(ack: &AckEnvelope) -> String {
    let mut out = String::new();
    out.push_str(&format!("Ack:        {}\n", ack.ack_id));
    out.push_str(&format!("Result:     {}\n", result_label(&ack.result)));
    out.push_str(&format!(
        "Re: env:    {}\n",
        ack.ref_envelope_id.as_deref().unwrap_or("(none)")
    ));
    out.push_str(&format!(
        "Hash:       {}\n",
        ack.ref_envelope_hash.as_deref().unwrap_or("(none)")
    ));

    match &ack.validation_result {
        Some(result) => {
            out.push_str(&format!("Violations: {}\n", result.violations.len()));
            for violation in &result.violations {
                let severity = match violation.severity {
                    ViolationSeverity::Error => "ERROR",
                    ViolationSeverity::Warning => "WARN",
                };
                let field = violation.field.as_deref().unwrap_or("-");
                out.push_str(&format!(
                    "  [{severity}] {} ({field}): {}\n",
                    violation.rule_id, violation.message
                ));
                out.push_str(&format!("          -> {}\n", narrative(&violation.rule_id)));
            }
        }
        None => out.push_str("Violations: 0\n"),
    }

    if let Some(details) = &ack.details {
        out.push_str(&format!("Details:    {details}\n"));
    }

    out
}

fn result_label(result: &AckResult) -> &'static str {
    match result {
        AckResult::Accepted => "ACCEPTED",
        AckResult::Rejected => "REJECTED",
    }
}

fn narrative(rule_id: &str) -> &'static str {
    match rule_id {
        "BRG-001" => "Bridge execution is disabled or constrained by the bridge registry.",
        "FB-001" => "Finality 0 requires a registered finality_basis.",
        "SEM-001" => "Finality 1 requires a settled state (FN or RV).",
        "SEM-002" => "A pending state cannot claim finality.",
        "SEM-003" => "A timelocked state cannot claim finality.",
        "SEM-004" => "A conditional state cannot claim finality.",
        "SEM-007" => "Class 5 transfers may only use Action TR or PR.",
        "TRG-001" => "A trigger must name its target, object, and authorizing instrument.",
        "LFC-007" => "A closed lifecycle admits only formal reversal or supersession, each citing the closure.",
        "LFC-008" => "A settled event must declare when its lifecycle is due to close.",
        "LEG-004" => "LEG origin Confidence E requires Action RW.",
        "LEG-006" => "Judicial or precedent legal events are constrained by finality and action.",
        "MSG-006" => "A LEG event must carry a registered legal subtype.",
        "ORG-003" => "The event origin must be registered.",
        "SEM-005" => "Class 6 CLOSE messages require Action CL.",
        "SEM-006" => "Class 3 messages may only use non-executing actions.",
        "SEM-012" => "Class 6 CLOSE messages require state FN.",
        "SEM-015" => "Confidence exceeds the maximum allowed by the verification regime.",
        "SEM-017" => "Bridge confidence requires explicit proof semantics.",
        _ => "No narrative on file; see docs/protocol/validation-rules.md.",
    }
}
