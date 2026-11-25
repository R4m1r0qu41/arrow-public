//! Pure mapping from a structured legal attestation to an Arrow LEG event.
//!
//! This module has **no network dependency** — it operates on the plain
//! [`LegalAttestation`] struct so the attestation→Arrow-code logic is fully
//! unit-testable. It is the deterministic heart of the Legal-attestation Origin
//! Adapter, the off-chain analogue of [`arrow_origin_eth::mapping`].
//!
//! ## What it does
//!
//! A LexExe-style engine evaluates a legal context and hands this adapter a
//! structured attestation: the authority subtype (`leg_subtype`), the jurisdiction
//! (`country` / `supra`), finality, confidence, and a verification regime. The
//! adapter does **not** invent legal facts. It does two honest things:
//!
//! 1. **Derives the canonical Action** from `leg_subtype + Finality + reason_code`
//!    using the same LEG Action Derivation table the validator enforces
//!    (`arrow-protocol-v1.5.1` §LEG-006/LEG-007). The minted event therefore lands
//!    inside the permitted Action range by construction, and a supranational
//!    sanctions instrument (`SUPRA` + `reason_code = SA` + `Finality = 1`) is minted
//!    as a non-overridable `BL` (LEG-007), never `PR`.
//! 2. **Emits a protocol-valid [`ArrowEvent`]** carrying `leg_subtype`,
//!    `leg_jurisdiction`, the derived flat `jurisdiction`, the verification regime,
//!    and — when `Finality = 0` — a `finality_basis` (FB-001).
//!
//! It classifies and records; it never determines applicable law or authorizes a
//! downstream action. The decision to act on a minted LEG event is delegated, by
//! design, to the fail-closed human-in-the-loop consumer (`arrow-consume-leg`).

use arrow_core::types::event::{ArrowEvent, LegJurisdiction};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The five LEG authority subtypes (mirrors the validator's controlled vocabulary).
pub const LEG_SUBTYPES: &[&str] = &["JUD", "ADM", "SUPRA", "LEG_TEXT", "PREC"];

/// A structured legal attestation handed to the Origin Adapter.
///
/// This is the LexExe→Arrow boundary object: a machine-readable record of an
/// asserted legal fact. Deserialized from an operator-supplied JSON file by the
/// binary; constructed directly in unit tests and by the helper constructors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegalAttestation {
    /// Authority subtype: one of [`LEG_SUBTYPES`].
    pub leg_subtype: String,
    /// ISO 3166-1 alpha-2 country code (e.g. `MX`). At least one of `country` or
    /// `supra` must be present for LEG-009.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    /// Supranational body code (e.g. `UNSC`, `EU`). Registered against
    /// `registries/leg_supra_jurisdictions.json` by LEG-010.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supra: Option<String>,
    /// Short legal reason code (e.g. `SA` sanctions, `CO` compliance, `LR`
    /// legislative reference). `SA` on a final `SUPRA` instrument forces `BL`.
    pub reason_code: String,
    /// Arrow Finality flag: `1` final, `0` non-final / interim / under appeal.
    pub finality: u8,
    /// Confidence grade `A`–`E`. `A` is not available for any LEG subtype.
    pub confidence: String,
    /// Arrow Class (1–6). Defaults to `2` (INSTRUCT) for a legal assertion.
    #[serde(default = "default_class")]
    pub class: u8,
    /// Arrow State code (`FN`, `PD`, `CD`, …). Defaults from `finality`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    /// IDENTITY/DID reference of the asserting legal authority or oracle.
    pub asserted_by: String,
    /// Case / decision / instrument identifier (becomes `subject_id`).
    pub case_ref: String,
    /// Stable object identifier for the legal assertion (becomes `object_id`).
    pub object_id: String,
    /// Structured legal proof type (e.g. `notarial_attestation`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proof_type: Option<String>,
    /// Verifiable reference to the attestation record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proof_ref: Option<String>,
    /// Verification regime: `legal_attestation` (manual C ceiling),
    /// `structured_legal_attestation` or `multiple_legal_attestation` (B
    /// ceilings), or `bonded_quorum` for ORC-style oracle-network attestations.
    pub verification_regime: String,
    /// Registered `finality_basis` code. Required when `finality = 0` (FB-001);
    /// defaults to `legal_appeal` when omitted on a non-final attestation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finality_basis: Option<String>,
    /// Optional neutral description (no legal advice, no enforceability claim).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

fn default_class() -> u8 {
    2
}

/// Outcome of deriving a Action: the canonical value plus whether it was a
/// non-overridable force (LEG-007 sanctions path).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionDerivation {
    pub action: &'static str,
    pub forced: bool,
}

/// Derive the canonical Action for a LEG attestation, mirroring the validator's
/// LEG-006/LEG-007 derivation table.
///
/// The chosen value is always inside the permitted range for the
/// `subtype + Finality` pair, so an event minted from it passes the LEG Action
/// Derivation Gate. For each subtype the most action-capable permitted value is
/// selected; `SUPRA + SA + Finality = 1` is forced to `BL` (LEG-007).
pub fn derive_action(subtype: &str, finality: u8, reason_code: &str) -> ActionDerivation {
    // LEG-007 — supranational sanctions force-block, non-overridable.
    if subtype == "SUPRA" && finality == 1 && reason_code == "SA" {
        return ActionDerivation {
            action: "BL",
            forced: true,
        };
    }
    // LEG-006 — permitted-range table; pick the canonical representative.
    let action = match (subtype, finality) {
        ("JUD", 1) => "PR",
        ("JUD", 0) => "HL",
        ("ADM", 1) => "PR",
        ("ADM", 0) => "HL",
        ("SUPRA", 1) => "PR",
        ("SUPRA", 0) => "HL",
        ("LEG_TEXT", _) => "HL",
        ("PREC", _) => "RW",
        // Unknown subtype: fail safe to review-only. The validator's MSG-006 will
        // reject the unknown subtype; the Action here is irrelevant but inert.
        _ => "RW",
    };
    ActionDerivation {
        action,
        forced: false,
    }
}

/// Default Arrow State for a given finality when the attestation omits one.
fn default_state(finality: u8) -> &'static str {
    if finality == 1 {
        "FN"
    } else {
        "PD"
    }
}

/// Map a structured legal attestation to a protocol-valid [`ArrowEvent`].
///
/// The returned event satisfies MSG-006 (`leg_subtype`), the LEG Action Derivation
/// Gate (LEG-004 through LEG-008), and FB-001 (`finality_basis` when
/// `Finality = 0`) and the LEG-009/LEG-010 jurisdiction rules.
pub fn map_to_arrow(att: &LegalAttestation) -> ArrowEvent {
    let derivation = derive_action(&att.leg_subtype, att.finality, &att.reason_code);
    let state = att
        .state
        .clone()
        .unwrap_or_else(|| default_state(att.finality).to_string());

    let arrow_code = format!(
        "{}:LEG:{}:{}:{}:{}",
        att.class, state, derivation.action, att.finality, att.confidence
    );

    // Flat `jurisdiction` projection: country if present, else supra. Keeps LEG-002
    // and all existing flat-jurisdiction rules operating unchanged (addendum §3.4).
    let flat_jurisdiction = att.country.clone().or_else(|| att.supra.clone());

    let leg_jurisdiction = if att.country.is_some() || att.supra.is_some() {
        Some(LegJurisdiction {
            country: att.country.clone(),
            supra: att.supra.clone(),
        })
    } else {
        None
    };

    let lifecycle_id = format!("leg:{}", att.case_ref);
    let event_id = format!("leg-evt-{}", Uuid::new_v4());

    let mut metadata = serde_json::Map::new();
    metadata.insert(
        "asserted_by".to_string(),
        serde_json::Value::String(att.asserted_by.clone()),
    );
    if let Some(j) = &flat_jurisdiction {
        metadata.insert(
            "jurisdiction".to_string(),
            serde_json::Value::String(j.clone()),
        );
    }
    if derivation.forced {
        metadata.insert(
            "action_forced".to_string(),
            serde_json::Value::String("LEG-007: supranational sanctions force-block".to_string()),
        );
    }
    if let Some(d) = &att.description {
        metadata.insert(
            "description".to_string(),
            serde_json::Value::String(d.clone()),
        );
    }

    let mut event = ArrowEvent {
        arrow_code,
        event_id,
        lifecycle_id,
        timestamp: Utc::now().to_rfc3339(),
        object_type: "legal_assertion".to_string(),
        object_id: att.object_id.clone(),
        subject_id: Some(att.case_ref.clone()),
        subject_type: Some("legal_case_reference".to_string()),
        proof_type: att.proof_type.clone(),
        proof_ref: att.proof_ref.clone(),
        verification_regime: Some(att.verification_regime.clone()),
        jurisdiction: flat_jurisdiction,
        reason_code: att.reason_code.clone(),
        metadata: Some(serde_json::Value::Object(metadata)),
        leg_subtype: Some(att.leg_subtype.clone()),
        leg_jurisdiction,
        ..Default::default()
    };

    // FB-001 — a non-final attestation MUST declare how it will eventually settle.
    if att.finality == 0 {
        event.finality_basis = Some(
            att.finality_basis
                .clone()
                .unwrap_or_else(|| "legal_appeal".to_string()),
        );
    } else {
        event.finality_basis = att.finality_basis.clone();
    }

    event
}

impl LegalAttestation {
    /// A final judicial order (`JUD`, `Finality = 1` → derived Action `PR`).
    pub fn final_judicial_order(
        country: impl Into<String>,
        case_ref: impl Into<String>,
        asserted_by: impl Into<String>,
    ) -> Self {
        let case_ref = case_ref.into();
        Self {
            leg_subtype: "JUD".to_string(),
            country: Some(country.into()),
            supra: None,
            reason_code: "CO".to_string(),
            finality: 1,
            confidence: "C".to_string(),
            class: default_class(),
            state: None,
            asserted_by: asserted_by.into(),
            object_id: format!("obj-{case_ref}"),
            case_ref,
            proof_type: Some("notarial_attestation".to_string()),
            proof_ref: None,
            verification_regime: "legal_attestation".to_string(),
            finality_basis: None,
            description: None,
        }
    }

    /// A supranational sanctions instrument (`SUPRA` + `SA` + `Finality = 1` →
    /// forced Action `BL`, LEG-007).
    pub fn supranational_sanctions(
        supra: impl Into<String>,
        case_ref: impl Into<String>,
        asserted_by: impl Into<String>,
    ) -> Self {
        let case_ref = case_ref.into();
        Self {
            leg_subtype: "SUPRA".to_string(),
            country: None,
            supra: Some(supra.into()),
            reason_code: "SA".to_string(),
            finality: 1,
            confidence: "C".to_string(),
            // Class 2 (INSTRUCT): SUPRA sanctions are legal instructions. Class 3
            // CONDITION events are reserved for non-executing actions per SEM-006.
            class: 2,
            state: None,
            asserted_by: asserted_by.into(),
            object_id: format!("obj-{case_ref}"),
            case_ref,
            proof_type: Some("notarial_attestation".to_string()),
            proof_ref: None,
            verification_regime: "legal_attestation".to_string(),
            finality_basis: None,
            description: None,
        }
    }

    /// A legislative / statutory norm (`LEG_TEXT` → derived Action `HL`).
    pub fn legislative_text(
        country: impl Into<String>,
        case_ref: impl Into<String>,
        asserted_by: impl Into<String>,
    ) -> Self {
        let case_ref = case_ref.into();
        Self {
            leg_subtype: "LEG_TEXT".to_string(),
            country: Some(country.into()),
            supra: None,
            reason_code: "LR".to_string(),
            finality: 0,
            confidence: "C".to_string(),
            class: 1,
            state: None,
            asserted_by: asserted_by.into(),
            object_id: format!("obj-{case_ref}"),
            case_ref,
            proof_type: None,
            proof_ref: None,
            verification_regime: "legal_attestation".to_string(),
            finality_basis: Some("legal_appeal".to_string()),
            description: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_core::registry::loader::Registries;
    use arrow_core::validation::pipeline::ValidationPipeline;

    fn assert_validates(event: &ArrowEvent) {
        let registries = Registries::load_embedded().expect("load embedded registries");
        let pipeline = ValidationPipeline::new(&registries);
        let result = pipeline.validate_event(event);
        let errors: Vec<_> = result.violations.iter().filter(|v| v.is_error()).collect();
        assert!(
            result.passed && errors.is_empty(),
            "event failed validation: arrow_code={}, errors={:#?}",
            event.arrow_code,
            errors
        );
    }

    #[test]
    fn final_judicial_order_mints_proceed_and_validates() {
        let att =
            LegalAttestation::final_judicial_order("MX", "case-2026-001", "did:arrow:oracle:mx-1");
        let ev = map_to_arrow(&att);
        assert_eq!(ev.arrow_code, "2:LEG:FN:PR:1:C");
        assert_eq!(ev.leg_subtype.as_deref(), Some("JUD"));
        assert_eq!(ev.jurisdiction.as_deref(), Some("MX"));
        assert_validates(&ev);
    }

    #[test]
    fn supranational_sanctions_is_forced_to_block() {
        let att = LegalAttestation::supranational_sanctions(
            "UNSC",
            "unscr-2026-42",
            "did:arrow:oracle:un-1",
        );
        let derivation = derive_action(&att.leg_subtype, att.finality, &att.reason_code);
        assert_eq!(derivation.action, "BL");
        assert!(derivation.forced);
        let ev = map_to_arrow(&att);
        // Forced BL coincides with the minted claim, so the event is valid (LEG-007
        // does not reject a claim that already equals the forced output).
        assert_eq!(ev.arrow_code, "2:LEG:FN:BL:1:C");
        assert_validates(&ev);
    }

    #[test]
    fn legislative_text_is_non_final_hold_with_finality_basis() {
        let att =
            LegalAttestation::legislative_text("MX", "ley-fintech-2026", "did:arrow:oracle:mx-2");
        let ev = map_to_arrow(&att);
        assert_eq!(ev.arrow_code, "1:LEG:PD:HL:0:C");
        // FB-001: a non-final attestation declares its settlement basis.
        assert_eq!(ev.finality_basis.as_deref(), Some("legal_appeal"));
        assert_validates(&ev);
    }

    #[test]
    fn derive_action_covers_the_table() {
        assert_eq!(derive_action("JUD", 1, "CO").action, "PR");
        assert_eq!(derive_action("JUD", 0, "CO").action, "HL");
        assert_eq!(derive_action("ADM", 1, "CO").action, "PR");
        assert_eq!(derive_action("SUPRA", 1, "CO").action, "PR");
        assert_eq!(derive_action("SUPRA", 0, "SA").action, "HL");
        assert_eq!(derive_action("LEG_TEXT", 1, "CO").action, "HL");
        assert_eq!(derive_action("PREC", 1, "CO").action, "RW");
        let forced = derive_action("SUPRA", 1, "SA");
        assert_eq!(forced.action, "BL");
        assert!(forced.forced);
    }
}
