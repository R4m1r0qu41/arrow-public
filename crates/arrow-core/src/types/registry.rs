use serde::Deserialize;

// ── Existing types (unchanged) ─────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct OriginEntry {
    pub code: String,
    pub name: String,
    pub description: String,
    pub verification_regime: String,
    #[serde(default)]
    pub origin_type: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClassEntry {
    pub code: u8,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StateEntry {
    pub code: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ActionEntry {
    pub code: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfidenceEntry {
    pub code: String,
    pub name: String,
    pub description: String,
    pub allows_action: bool,
}

/// Updated in v1.4: added `classification_bound` field.
/// Defaults to false for backward compatibility with v1.3.x registry files
/// that do not carry the field.
#[derive(Debug, Clone, Deserialize)]
pub struct VerificationRegimeEntry {
    pub code: String,
    pub name: String,
    pub description: String,
    pub max_confidence: String,
    /// Whether this regime, by default, verifies source-domain output type
    /// classification as part of its proof. Introduced in v1.4 for SEM-017.
    /// Defaults to false — bonded_quorum, institutional_attestation, and
    /// legal_attestation do not classify by default. native_canonical and
    /// optimistic_settled set this true in their registry entries.
    #[serde(default)]
    pub classification_bound: bool,
    /// Conditional SEM-015 ceiling applied while Finality=0 (pre-settlement).
    /// Set for regimes whose full `max_confidence` is only earned after a
    /// settlement window elapses (optimistic_settled: A settled / B pending).
    /// Absent means the flat `max_confidence` ceiling applies at any Finality.
    #[serde(default)]
    pub pre_settlement_max_confidence: Option<String>,
    /// RFC-010 Part 2A (Universal Eventual Closure): how lifecycles under
    /// this regime reach closure. `depth_exhaustion` / `window_exhaustion`
    /// are bounded disciplines — Class 1 FN Finality=1 events under them
    /// MUST declare `closure_eta` (LFC-008). `attestation_bound` and
    /// `legal_exhaustion` carry no fixed deadline and are exempt.
    #[serde(default)]
    pub closure_discipline: Option<String>,
}

/// v2.0 chain registry — protocol_required block.
#[derive(Debug, Clone, Deserialize)]
pub struct ChainProtocolRequired {
    pub finality_model: String,
    #[serde(default)]
    pub finality_basis_default: Option<String>,
    #[serde(default)]
    pub challenge_window_seconds: Option<u64>,
    #[serde(default)]
    pub reorg_safety_confirmations: Option<u32>,
    /// Binary trust flag. 0 = chain revoked (forces BL). 1 = trusted.
    pub chain_trust_flag: u8,
    /// Network layer: 1 = L1, 2 = L2.
    pub layer: u8,
    #[serde(default)]
    pub settlement_chain: Option<String>,
}

/// RFC-007 CHN-005 — structured disclosure for a single governance-capability field.
/// The `status` field carries the disclosure vocabulary:
/// "documented" | "demonstrated" | "inferred" | "unknown".
/// Aligns governance-capability disclosure with Arrow provenance discipline.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct CapabilityDisclosure {
    /// Disclosure status: documented | demonstrated | inferred | unknown.
    #[serde(default)]
    pub status: Option<String>,
    /// Source reference URL or identifier.
    #[serde(default)]
    pub source_ref: Option<String>,
    /// Date of observation (ISO 8601 date string).
    #[serde(default)]
    pub observed_at: Option<String>,
}

/// RFC-007 CHN-005 — advisory governance-capability block for Chain Registry entries.
/// Advisory only. MUST NOT affect Action (CHN-006). Never used to derive
/// finality_basis, finality_eta, or chain_trust_flag (PRV-003 coherence).
/// All fields optional; existing entries deserialize unchanged.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct GovernanceCapability {
    /// Whether an emergency-upgrade authority exists for this chain.
    #[serde(default)]
    pub emergency_upgrade_authority: Option<bool>,
    /// Multisignature threshold required for emergency action (e.g., "9-of-12").
    #[serde(default)]
    pub multisig_threshold: Option<String>,
    /// Delay in seconds before an upgrade takes effect (0 = no delay).
    #[serde(default)]
    pub upgrade_delay_seconds: Option<u64>,
    /// Exit window available to users before an emergency upgrade executes, in seconds.
    #[serde(default)]
    pub emergency_upgrade_exit_window_seconds: Option<u64>,
    /// Whether address-specific compelled transfers have occurred. Structured disclosure.
    #[serde(default)]
    pub historical_compelled_transfer: Option<CapabilityDisclosure>,
    /// Whether sender impersonation via upgrade has been demonstrated. Structured disclosure.
    #[serde(default)]
    pub sender_impersonation_via_upgrade: Option<CapabilityDisclosure>,
}

/// v2.0 chain registry advisory block (all optional).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ChainAdvisory {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub consensus: Option<String>,
    #[serde(default)]
    pub native_asset: Option<String>,
    #[serde(default)]
    pub evm_compatible: Option<bool>,
    /// RFC-007 CHN-005 — optional governance-capability disclosure.
    #[serde(default)]
    pub governance_capability: Option<GovernanceCapability>,
}

/// Chain Registry entry (chain_registry.json v2.0).
/// Upgraded from flat v1.3.x schema to v2.0 block structure.
/// `code` is the 3-char origin code and the HashMap key.
#[derive(Debug, Clone, Deserialize)]
pub struct ChainRegistryEntry {
    /// 3-char origin code (e.g. "ETH", "BTC"). Map key; equivalent to origin_code.
    pub code: String,
    pub name: String,
    pub status: String,
    pub protocol_required: ChainProtocolRequired,
    #[serde(default)]
    pub advisory: Option<ChainAdvisory>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SourceEntry {
    pub source_type: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReasonCodeEntry {
    pub code: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ObjectTypeEntry {
    pub code: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubjectTypeEntry {
    pub code: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProofTypeEntry {
    pub code: String,
    pub name: String,
    pub description: String,
}

/// Transport channel binding registry entry (channel_bindings.json).
#[derive(Debug, Clone, Deserialize)]
pub struct ChannelBindingEntry {
    pub code: String,
    pub name: String,
    pub description: String,
    pub ordered: bool,
    pub reliable: bool,
    pub supports_ack: bool,
    pub requires_ack: bool,
    pub supports_signature: bool,
    pub requires_signature: bool,
    pub deployment_scope: String,
    pub local_only: bool,
    pub requires_tls: bool,
    /// ISO 8601 duration, e.g. "PT24H".
    pub nonce_retention_min: String,
    #[serde(default)]
    pub freshness_stale_window: Option<String>,
    #[serde(default)]
    pub freshness_future_tolerance: Option<String>,
    pub spec_reference: String,
    pub status: String,
    pub added_version: String,
}

// ── New types in v1.4 (RFC-002) ───────────────────────────────────────────

/// Classification profile embedded in a BridgeRegistryEntry.
/// Describes whether and how this bridge proves source-domain output type.
#[derive(Debug, Clone, Deserialize)]
pub struct BridgeClassificationProfile {
    /// Whether source-domain classification has been verified for this bridge.
    /// If false, all events from this bridge are capped at Confidence C (SEM-017).
    pub classification_verified: bool,
    /// Maximum Confidence claimable by events from this bridge, based on
    /// classification binding strength. Null = no explicit ceiling beyond regime default.
    #[serde(default)]
    pub classification_confidence_ceiling: Option<String>,
    /// Reference to classification_binding_methods.json entry. Declares the
    /// mechanism by which classification is proven, attested, or inferred.
    #[serde(default)]
    pub classification_binding_method: Option<String>,
    /// Reference to an independent audit, formal verification, or governance
    /// approval record. Required when classification_verified=true and
    /// ceiling above C (BRG-003).
    #[serde(default)]
    pub classification_audit_ref: Option<String>,
    /// If true, events from this bridge MUST declare a confidence subprofile
    /// (BRG-006 hard enforcement). If false, missing subprofile caps at D.
    #[serde(default)]
    pub requires_subprofile: bool,
    #[serde(default)]
    pub classification_notes: Option<String>,
}

/// Economic consistency profile embedded in a BridgeRegistryEntry.
/// Describes whether source-side value and destination-side payout
/// have been verified to be consistent under this bridge.
#[derive(Debug, Clone, Deserialize)]
pub struct BridgeEconomicConsistencyProfile {
    /// Whether economic consistency verification is required for value-releasing
    /// events through this bridge.
    pub economic_consistency_required: bool,
    /// Whether economic consistency has been verified at the registry level.
    #[serde(default)]
    pub economic_consistency_verified: bool,
    /// Reference to the audit, formal verification, or review that establishes
    /// economic consistency. Required when economic_consistency_verified=true.
    #[serde(default)]
    pub economic_consistency_audit_ref: Option<String>,
    #[serde(default)]
    pub economic_consistency_notes: Option<String>,
}

/// Bridge Registry entry (bridge_registry.json).
/// Introduced in v1.4 (RFC-002). Each entry represents a specific bridge
/// instance with its own trust classification and validation posture.
#[derive(Debug, Clone, Deserialize)]
pub struct BridgeRegistryEntry {
    /// Unique bridge identifier. Referenced in ArrowEvent.bridge_id.
    pub bridge_id: String,
    pub bridge_name: String,
    #[serde(default)]
    pub description: Option<String>,
    /// chain_id of the source chain (references chain_registry.json).
    pub source_chain_id: String,
    /// chain_id of the destination chain (references chain_registry.json).
    pub destination_chain_id: String,
    /// Contract address on the destination chain. Null for non-contract bridges.
    #[serde(default)]
    pub bridge_contract_address: Option<String>,
    /// Verification regime for this bridge (references verification_regimes.json).
    pub verification_regime: String,
    /// Binary trust flag. 0 = disabled; BRG-001 forces Action BL on all events.
    /// 1 = active. Must be integer, not boolean.
    pub bridge_trust_flag: u8,
    /// Controlled enum: active | disabled | degraded | under_review | restored.
    pub bridge_status: String,
    /// Required when bridge_trust_flag=0 (BRG-002).
    #[serde(default)]
    pub bridge_status_reason: Option<String>,
    /// ISO 8601 UTC. Required when bridge_trust_flag=0 (BRG-002).
    #[serde(default)]
    pub bridge_status_effective_from: Option<String>,
    /// Classification binding profile for this bridge.
    pub classification_profile: BridgeClassificationProfile,
    /// Economic consistency profile for this bridge.
    pub economic_consistency_profile: BridgeEconomicConsistencyProfile,
    /// Confidence ceiling during a BRG-005 restoration period. Null when not in
    /// restoration. Must be B or lower when set.
    #[serde(default)]
    pub restoration_confidence_ceiling: Option<String>,
    /// ISO 8601 UTC or block reference. Null when not in restoration.
    #[serde(default)]
    pub restoration_ceiling_effective_until: Option<String>,
    #[serde(default)]
    pub restoration_conditions_met: bool,
}

/// Confidence Subprofile entry (confidence_subprofiles.json).
/// Introduced in v1.4 (RFC-002). Defines default confidence ceilings and
/// accountability requirements for categories of bridges, oracles, and
/// third-party attestation sources. Referenced by BRG-006.
#[derive(Debug, Clone, Deserialize)]
pub struct ConfidenceSubprofileEntry {
    pub subprofile_id: String,
    pub category: String,
    pub name: String,
    pub description: String,
    /// Maximum Confidence for events from actors matching this subprofile,
    /// absent additional verification.
    pub default_confidence_ceiling: String,
    pub requires_accountable_operator: bool,
    pub requires_source_methodology: bool,
    pub requires_dispute_or_challenge_path: bool,
    /// Controlled enum: reject | downgrade | defer_to_policy.
    pub default_rejection_behavior: String,
    /// Whether this category of actor, by default, verifies source-domain
    /// output type classification.
    #[serde(default)]
    pub classification_bound_default: bool,
    pub status: String,
    pub added_version: String,
}

/// Finality Basis entry (finality_basis.json).
/// Introduced in v1.5. Controlled vocabulary for the finality_basis Tier-3 field.
/// Required when event.finality == 0 (FB-001).
#[derive(Debug, Clone, Deserialize)]
pub struct FinalityBasisEntry {
    pub code: String,
    pub name: String,
    pub description: String,
    /// Whether this basis conventionally benefits from a finality_eta value.
    #[serde(default)]
    pub requires_finality_eta: bool,
}

/// Supranational jurisdiction entry (registries/leg_supra_jurisdictions.json).
/// Introduced in v1.5 (LEG Amendment). Controlled vocabulary for
/// leg_jurisdiction.supra in LEG-origin events. Referenced by LEG-010.
#[derive(Debug, Clone, Deserialize)]
pub struct LegSupraJurisdictionEntry {
    pub code: String,
    pub name: String,
    pub enforcement_basis: String,
    pub enforcement_type: String,
    pub geographic_scope: String,
    pub status: String,
    pub added_version: String,
}

/// Classification Binding Method entry (classification_binding_methods.json).
/// Introduced in v1.4 (RFC-002). Controlled vocabulary for
/// proof_semantics.classification_binding_method. Referenced by SEM-017 and MSG-005.
#[derive(Debug, Clone, Deserialize)]
pub struct ClassificationBindingMethodEntry {
    pub method_id: String,
    pub name: String,
    pub description: String,
    /// Maximum Confidence claimable by events declaring this binding method.
    pub max_confidence_ceiling: String,
    /// Whether events using this method above Confidence C must provide
    /// classification_audit_ref.
    pub requires_audit_ref: bool,
    pub status: String,
    pub added_version: String,
}
