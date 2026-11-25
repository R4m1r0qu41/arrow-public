use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::{collections::HashMap, fs, path::Path};

use crate::{
    errors::RegistryError,
    types::registry::{
        ActionEntry, BridgeRegistryEntry, ChainRegistryEntry, ChannelBindingEntry, ClassEntry,
        ClassificationBindingMethodEntry, ConfidenceEntry, ConfidenceSubprofileEntry,
        FinalityBasisEntry, LegSupraJurisdictionEntry, ObjectTypeEntry, OriginEntry,
        ProofTypeEntry, ReasonCodeEntry, SourceEntry, StateEntry, SubjectTypeEntry,
        VerificationRegimeEntry,
    },
};

#[derive(Debug, Clone)]
pub struct Registries {
    // ── Core registries (v1.0–v1.3.1) ─────────────────────────────────
    pub origins: HashMap<String, OriginEntry>,
    pub classes: HashMap<u8, ClassEntry>,
    pub states: HashMap<String, StateEntry>,
    pub actions: HashMap<String, ActionEntry>,
    pub confidence: HashMap<String, ConfidenceEntry>,
    pub verification_regimes: HashMap<String, VerificationRegimeEntry>,
    pub chain_registry: HashMap<String, ChainRegistryEntry>,
    pub sources: HashMap<String, SourceEntry>,
    pub reason_codes: HashMap<String, ReasonCodeEntry>,
    pub object_types: HashMap<String, ObjectTypeEntry>,
    pub subject_types: HashMap<String, SubjectTypeEntry>,
    pub proof_types: HashMap<String, ProofTypeEntry>,
    // ── Transport registry (v1.3.2) ─────────────────────────────────────
    pub channel_bindings: HashMap<String, ChannelBindingEntry>,
    // ── Bridge / classification registries (v1.4) ───────────────────────
    /// Bridge Registry — keyed by bridge_id. Source of BRG rule evaluation.
    pub bridge_registry: HashMap<String, BridgeRegistryEntry>,
    /// Confidence Subprofiles — keyed by subprofile_id. Referenced by BRG-006.
    pub confidence_subprofiles: HashMap<String, ConfidenceSubprofileEntry>,
    /// Classification Binding Methods — keyed by method_id. Referenced by MSG-005.
    pub classification_binding_methods: HashMap<String, ClassificationBindingMethodEntry>,
    // ── Finality registries (v1.5) ──────────────────────────────────────────
    /// Finality Basis — keyed by code. Required when event.finality == 0 (FB-001).
    pub finality_basis: HashMap<String, FinalityBasisEntry>,
    // ── LEG registries ──────────────────────────────────────────────────────
    /// Supranational jurisdiction entries — keyed by code. Referenced by LEG-010.
    /// Loaded from registries/leg_supra_jurisdictions.json.
    pub leg_supra_jurisdictions: HashMap<String, LegSupraJurisdictionEntry>,
}

#[derive(Debug, Deserialize)]
struct Entries<T> {
    entries: Vec<T>,
}

fn parse_entries<T: DeserializeOwned>(raw: &str, path: &str) -> Result<Vec<T>, RegistryError> {
    let parsed: Entries<T> = serde_json::from_str(raw).map_err(|source| RegistryError::Json {
        path: path.to_owned(),
        source,
    })?;
    Ok(parsed.entries)
}

fn load_entries<T: DeserializeOwned>(
    dir: &Path,
    file: &'static str,
) -> Result<Vec<T>, RegistryError> {
    let path = dir.join(file);
    let raw = fs::read_to_string(&path).map_err(|source| RegistryError::Io {
        path: path.display().to_string(),
        source,
    })?;
    parse_entries(&raw, &path.display().to_string())
}

impl Registries {
    pub fn load_from_dir(path: &Path) -> Result<Self, RegistryError> {
        Self::from_entries(
            load_entries::<OriginEntry>(path, "origins.json")?,
            load_entries::<ClassEntry>(path, "classes.json")?,
            load_entries::<StateEntry>(path, "states.json")?,
            load_entries::<ActionEntry>(path, "actions.json")?,
            load_entries::<ConfidenceEntry>(path, "confidence.json")?,
            load_entries::<VerificationRegimeEntry>(path, "verification_regimes.json")?,
            load_entries::<ChainRegistryEntry>(path, "chain_registry.json")?,
            load_entries::<SourceEntry>(path, "sources.json")?,
            load_entries::<ReasonCodeEntry>(path, "reason_codes.json")?,
            load_entries::<ObjectTypeEntry>(path, "object_types.json")?,
            load_entries::<SubjectTypeEntry>(path, "subject_types.json")?,
            load_entries::<ProofTypeEntry>(path, "proof_types.json")?,
            load_entries::<ChannelBindingEntry>(path, "channel_bindings.json")?,
            load_entries::<BridgeRegistryEntry>(path, "bridge_registry.json")?,
            load_entries::<ConfidenceSubprofileEntry>(path, "confidence_subprofiles.json")?,
            load_entries::<ClassificationBindingMethodEntry>(
                path,
                "classification_binding_methods.json",
            )?,
            load_entries::<FinalityBasisEntry>(path, "finality_basis.json")?,
            load_entries::<LegSupraJurisdictionEntry>(path, "leg_supra_jurisdictions.json")?,
        )
    }

    pub fn load_embedded() -> Result<Self, RegistryError> {
        Self::from_entries(
            parse_entries::<OriginEntry>(
                include_str!("../../../../registries/origins.json"),
                "embedded/origins.json",
            )?,
            parse_entries::<ClassEntry>(
                include_str!("../../../../registries/classes.json"),
                "embedded/classes.json",
            )?,
            parse_entries::<StateEntry>(
                include_str!("../../../../registries/states.json"),
                "embedded/states.json",
            )?,
            parse_entries::<ActionEntry>(
                include_str!("../../../../registries/actions.json"),
                "embedded/actions.json",
            )?,
            parse_entries::<ConfidenceEntry>(
                include_str!("../../../../registries/confidence.json"),
                "embedded/confidence.json",
            )?,
            parse_entries::<VerificationRegimeEntry>(
                include_str!("../../../../registries/verification_regimes.json"),
                "embedded/verification_regimes.json",
            )?,
            parse_entries::<ChainRegistryEntry>(
                include_str!("../../../../registries/chain_registry.json"),
                "embedded/chain_registry.json",
            )?,
            parse_entries::<SourceEntry>(
                include_str!("../../../../registries/sources.json"),
                "embedded/sources.json",
            )?,
            parse_entries::<ReasonCodeEntry>(
                include_str!("../../../../registries/reason_codes.json"),
                "embedded/reason_codes.json",
            )?,
            parse_entries::<ObjectTypeEntry>(
                include_str!("../../../../registries/object_types.json"),
                "embedded/object_types.json",
            )?,
            parse_entries::<SubjectTypeEntry>(
                include_str!("../../../../registries/subject_types.json"),
                "embedded/subject_types.json",
            )?,
            parse_entries::<ProofTypeEntry>(
                include_str!("../../../../registries/proof_types.json"),
                "embedded/proof_types.json",
            )?,
            parse_entries::<ChannelBindingEntry>(
                include_str!("../../../../registries/channel_bindings.json"),
                "embedded/channel_bindings.json",
            )?,
            parse_entries::<BridgeRegistryEntry>(
                include_str!("../../../../registries/bridge_registry.json"),
                "embedded/bridge_registry.json",
            )?,
            parse_entries::<ConfidenceSubprofileEntry>(
                include_str!("../../../../registries/confidence_subprofiles.json"),
                "embedded/confidence_subprofiles.json",
            )?,
            parse_entries::<ClassificationBindingMethodEntry>(
                include_str!("../../../../registries/classification_binding_methods.json"),
                "embedded/classification_binding_methods.json",
            )?,
            parse_entries::<FinalityBasisEntry>(
                include_str!("../../../../registries/finality_basis.json"),
                "embedded/finality_basis.json",
            )?,
            parse_entries::<LegSupraJurisdictionEntry>(
                include_str!("../../../../registries/leg_supra_jurisdictions.json"),
                "embedded/leg_supra_jurisdictions.json",
            )?,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn from_entries(
        origins: Vec<OriginEntry>,
        classes: Vec<ClassEntry>,
        states: Vec<StateEntry>,
        actions: Vec<ActionEntry>,
        confidence: Vec<ConfidenceEntry>,
        verification_regimes: Vec<VerificationRegimeEntry>,
        chain_registry: Vec<ChainRegistryEntry>,
        sources: Vec<SourceEntry>,
        reason_codes: Vec<ReasonCodeEntry>,
        object_types: Vec<ObjectTypeEntry>,
        subject_types: Vec<SubjectTypeEntry>,
        proof_types: Vec<ProofTypeEntry>,
        channel_bindings: Vec<ChannelBindingEntry>,
        bridge_registry: Vec<BridgeRegistryEntry>,
        confidence_subprofiles: Vec<ConfidenceSubprofileEntry>,
        classification_binding_methods: Vec<ClassificationBindingMethodEntry>,
        finality_basis: Vec<FinalityBasisEntry>,
        leg_supra_jurisdictions: Vec<LegSupraJurisdictionEntry>,
    ) -> Result<Self, RegistryError> {
        Ok(Self {
            origins: origins.into_iter().map(|e| (e.code.clone(), e)).collect(),
            classes: classes.into_iter().map(|e| (e.code, e)).collect(),
            states: states.into_iter().map(|e| (e.code.clone(), e)).collect(),
            actions: actions.into_iter().map(|e| (e.code.clone(), e)).collect(),
            confidence: confidence
                .into_iter()
                .map(|e| (e.code.clone(), e))
                .collect(),
            verification_regimes: verification_regimes
                .into_iter()
                .map(|e| (e.code.clone(), e))
                .collect(),
            chain_registry: chain_registry
                .into_iter()
                .map(|e| (e.code.clone(), e))
                .collect(),
            sources: sources
                .into_iter()
                .map(|e| (e.source_type.clone(), e))
                .collect(),
            reason_codes: reason_codes
                .into_iter()
                .map(|e| (e.code.clone(), e))
                .collect(),
            object_types: object_types
                .into_iter()
                .map(|e| (e.code.clone(), e))
                .collect(),
            subject_types: subject_types
                .into_iter()
                .map(|e| (e.code.clone(), e))
                .collect(),
            proof_types: proof_types
                .into_iter()
                .map(|e| (e.code.clone(), e))
                .collect(),
            channel_bindings: channel_bindings
                .into_iter()
                .map(|e| (e.code.clone(), e))
                .collect(),
            bridge_registry: bridge_registry
                .into_iter()
                .map(|e| (e.bridge_id.clone(), e))
                .collect(),
            confidence_subprofiles: confidence_subprofiles
                .into_iter()
                .map(|e| (e.subprofile_id.clone(), e))
                .collect(),
            classification_binding_methods: classification_binding_methods
                .into_iter()
                .map(|e| (e.method_id.clone(), e))
                .collect(),
            finality_basis: finality_basis
                .into_iter()
                .map(|e| (e.code.clone(), e))
                .collect(),
            leg_supra_jurisdictions: leg_supra_jurisdictions
                .into_iter()
                .map(|e| (e.code.clone(), e))
                .collect(),
        })
    }

    pub fn chain_entries_for_origin<'a>(
        &'a self,
        origin: &'a str,
    ) -> impl Iterator<Item = &'a ChainRegistryEntry> + 'a {
        self.chain_registry
            .values()
            .filter(move |entry| entry.code == origin)
    }

    /// Returns the Bridge Registry entry for a given bridge_id, if present.
    pub fn bridge_entry(&self, bridge_id: &str) -> Option<&BridgeRegistryEntry> {
        self.bridge_registry.get(bridge_id)
    }

    /// Returns the classification binding method entry for a given method_id.
    pub fn classification_method(
        &self,
        method_id: &str,
    ) -> Option<&ClassificationBindingMethodEntry> {
        self.classification_binding_methods.get(method_id)
    }

    /// Test helper — produces a Registries with all HashMaps empty.
    /// Not available in release builds.
    #[cfg(test)]
    pub fn empty() -> Self {
        Self {
            origins: HashMap::new(),
            classes: HashMap::new(),
            states: HashMap::new(),
            actions: HashMap::new(),
            confidence: HashMap::new(),
            verification_regimes: HashMap::new(),
            chain_registry: HashMap::new(),
            sources: HashMap::new(),
            reason_codes: HashMap::new(),
            object_types: HashMap::new(),
            subject_types: HashMap::new(),
            proof_types: HashMap::new(),
            channel_bindings: HashMap::new(),
            bridge_registry: HashMap::new(),
            confidence_subprofiles: HashMap::new(),
            classification_binding_methods: HashMap::new(),
            finality_basis: HashMap::new(),
            leg_supra_jurisdictions: HashMap::new(),
        }
    }
}
