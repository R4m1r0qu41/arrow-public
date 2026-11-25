use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── v1.4 sub-objects ──────────────────────────────────────────────────────

/// Proof semantics block for cross-domain classification binding.
/// Required on bridge-origin events under non-canonical regimes.
/// Part of the SEM-017 Proof Classification Binding rule.
///
/// All fields are required when this object is present.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProofSemantics {
    /// The data, hash, state element, message, output, event, or object
    /// that exists in the source domain and is covered by proof_ref.
    pub proven_content: String,
    /// The source-domain classification of the proven object under the
    /// source protocol, registry, contract, or attestation framework.
    /// Example: "supplemental_export_output", "primary_export_instruction".
    pub source_claim_type: String,
    /// The destination-domain type or operational meaning asserted by this
    /// Arrow event. Must match destination_claim_type vs source_claim_type.
    pub destination_claim_type: String,
    /// Reference to a registered entry in classification_binding_methods.json.
    /// Declares the mechanism by which the classification is proven or inferred.
    /// Freeform strings are rejected by MSG-005.
    pub classification_binding_method: String,
    /// Whether source-domain classification of the proven object has been
    /// verified under the declared binding method.
    /// If false: Confidence MUST NOT exceed C (SEM-017).
    pub classification_verified: bool,
}

/// Economic consistency block for value-releasing bridge events.
/// Required on Class 5/6 bridge events claiming Confidence A or B.
/// Part of the SEM-019 Economic Consistency Binding rule.
///
/// All fields are required when this object is present.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EconomicConsistency {
    /// Decimal string. The value demonstrated at the source domain
    /// by the underlying proof.
    pub source_value: String,
    /// Decimal string. The value claimed at the destination domain
    /// by this Arrow event.
    pub destination_value: String,
    /// Whether source-side value, authority, and destination-side payout
    /// are economically consistent under the applicable bridge regime.
    /// If false: event MUST NOT claim Confidence A or B (SEM-019).
    pub consistency_verified: bool,
}

/// Structured jurisdiction for LEG-origin events (v1.5).
/// `local` was removed per §F-9.2 — use extensions.legal_instrument.local_forum instead.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LegJurisdiction {
    /// ISO 3166-1 alpha-2 country code (e.g. "MX", "US", "DE").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    /// Supra-national body code from leg_supra_jurisdictions.json (e.g. "EU", "FATF").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supra: Option<String>,
}

// ── Arrow Event Object ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArrowEvent {
    // ── Core fields (required) ──────────────────────────────────────────
    pub arrow_code: String,
    pub event_id: String,
    pub lifecycle_id: String,
    pub timestamp: String,
    pub object_type: String,
    pub object_id: String,

    // ── Optional / conditional core fields ─────────────────────────────
    #[serde(default)]
    pub subject_id: Option<String>,
    #[serde(default)]
    pub subject_type: Option<String>,
    #[serde(default)]
    pub proof_type: Option<String>,
    #[serde(default)]
    pub proof_ref: Option<String>,
    #[serde(default)]
    pub verification_regime: Option<String>,
    #[serde(default)]
    pub jurisdiction: Option<String>,
    #[serde(default)]
    pub reason_code: String,
    #[serde(default)]
    pub related_events: Vec<String>,
    #[serde(default)]
    pub chain_id: Option<String>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
    #[serde(default)]
    pub extensions: Option<HashMap<String, serde_json::Value>>,

    // ── v1.4 fields (RFC-002) ───────────────────────────────────────────
    /// Reference to a bridge_id in bridge_registry.json.
    /// Required when this event is routed through a registered bridge.
    /// Presence triggers BRG-001 through BRG-006 evaluation.
    #[serde(default)]
    pub bridge_id: Option<String>,

    /// Proof classification binding metadata.
    /// Required when bridge_id is present and the bridge's verification_regime
    /// is not native_canonical or optimistic_settled (SEM-017).
    #[serde(default)]
    pub proof_semantics: Option<ProofSemantics>,

    /// Economic consistency binding metadata.
    /// Required for Class 5 (TRANSFER) or Class 6 (CLOSE) events claiming
    /// Confidence A or B where bridge_id is present (SEM-019).
    #[serde(default)]
    pub economic_consistency: Option<EconomicConsistency>,

    /// Reference to the independent audit, formal verification, or governance
    /// review record establishing economic consistency.
    /// Required when economic_consistency.consistency_verified=true and
    /// Confidence A or B is claimed (SEM-019).
    #[serde(default)]
    pub economic_consistency_audit_ref: Option<String>,

    // ── v1.5 Tier-3 finality fields ────────────────────────────────────────
    /// Registered entry in finality_basis.json.
    /// REQUIRED when finality == 0 (FB-001). Declares the mechanism by which
    /// this pending-finality event will eventually settle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finality_basis: Option<String>,

    /// Expected settlement time. Semantics depend on finality_basis:
    /// Unix timestamp for time-bound windows; block count for probabilistic.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finality_eta: Option<u64>,

    /// Discriminates the unit of finality_eta. Values: "unix_ts", "block_count",
    /// "iso_date". Presence implies finality_eta is also present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finality_eta_type: Option<String>,

    // ── RFC-010 lifecycle-gate fields ──────────────────────────────────────
    /// TRG-001: REQUIRED when Action == TR. Names the workflow target,
    /// object, and authorizing instrument of the trigger; the gate checks
    /// shape, the consumer (Tarquin LC-014) checks resolvability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_target: Option<ActionTarget>,

    /// LFC-008: REQUIRED on Class 1 FN Finality=1 events under a bounded
    /// closure discipline (depth_exhaustion / window_exhaustion). Declares
    /// when this lifecycle is due to CLOSE (Class 6). Same unit semantics
    /// as finality_eta; 0 block_count means "closure due immediately".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closure_eta: Option<u64>,

    /// Discriminates the unit of closure_eta ("unix_ts", "block_count",
    /// "iso_date"). Presence implies closure_eta is also present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closure_eta_type: Option<String>,

    // ── LEG fields ─────────────────────────────────────────────────────────
    /// Legal event subtype. Required when origin == "LEG" (MSG-006).
    /// Controlled vocabulary: JUD, ADM, SUPRA, LEG_TEXT, PREC.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leg_subtype: Option<String>,

    /// Jurisdiction for LEG-origin events.
    /// country = ISO 3166-1 alpha-2 code; supra = supra-national body code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leg_jurisdiction: Option<LegJurisdiction>,
}

/// TRG-001 action-target envelope. Binds to the Action namespace (field 4):
/// a TR event must name what it triggers, on what, and on whose authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ActionTarget {
    /// Recipient / custodian / workflow target — explicit; resolvability is
    /// checked consumer-side (Tarquin LC-014), shape is checked at the gate.
    pub target_ref: String,
    /// Asset / right / control surface being acted on.
    pub object_ref: String,
    /// Event IDs that specifically authorize this trigger. Must be a
    /// non-empty subset of the event's `related_events`.
    #[serde(default)]
    pub authorizing_events: Vec<String>,
}
