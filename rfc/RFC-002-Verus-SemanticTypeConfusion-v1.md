# Arrow Protocol — RFC-002: Cross-Domain Semantic Consistency

*Reference case and protocol extension: Verus-Ethereum Bridge Exploit, May 2026*

| Field | Value |
|---|---|
| Document ID | ARROW-RFC-002 |
| Status | Final |
| Version | 1.0 |
| Date | 2026-05-22 |
| Updated | 2026-05-23 |
| Depends on | Arrow Protocol v1.3, Validation Rules v1.3, RFC-001 |
| Addresses | RFC-001 Open Question Q1 — Bridge Registry |
| Proposes | SEM-017, SEM-018, SEM-019, BRG-001 to BRG-006, new reason codes, Confidence Registry definition update, Verification Regimes Registry extensions, Confidence Subprofiles Registry, Bridge Registry v1.4 candidate |
| Genesis Anchor | RFC-000 — `2026-03-22T18:40:00Z` |
| Target Build | Arrow Protocol v1.4 |

---

## Abstract

Arrow's foundational promise is that systems must agree on *what happened*. The Verus-Ethereum Bridge exploit of May 2026 demonstrates a failure class not fully addressed by the existing Arrow validation rules: a destination system accepted a proof of source-chain data existence without proving the source-chain classification of that data.

The reference-case analysis based on the Verus post-mortem indicates that the source-side Verus PBaaS daemon treated certain crafted outputs as inactive supplemental export outputs, while the Ethereum bridge contract accepted the proven data as if it were an active primary export instruction. External reporting frames the same incident more generally as a bridge validation failure: a forged or counterfeit cross-chain transfer message, backed by valid proof material, was accepted because the Ethereum-side validation path did not enforce the economic and semantic relationship between source-side inputs and destination-side payouts.

RFC-002 therefore establishes the **Cross-Domain Semantic Consistency Invariant** as a named Arrow design principle, introduces **SEM-017**, **SEM-018**, and **SEM-019**, proposes explicit classification and economic-consistency metadata, adds new reason codes, extends the Verification Regimes Registry, and formally proposes a Bridge Registry for Arrow v1.4.

---

## 1. Background

### 1.1 The Verus-Ethereum Bridge Exploit — 2026-05-18

On May 18, 2026, an unknown attacker drained approximately USD 11.5M to USD 11.7M from the Verus-Ethereum Bridge across three principal assets: approximately 1,625 ETH, 103.6 tBTC, and 147K to 148K USDC. The assets were later swapped into approximately 5,402 ETH and consolidated into an Ethereum address associated with the exploiter.

The attack did not appear to rely on a private-key compromise, reentrancy, frontend compromise, oracle price manipulation, or a conventional balance-spoofing bug. The incident is better understood as a cross-chain verification failure: the Ethereum-side bridge accepted a proof-backed import that did not prove the full claim being executed.

### 1.2 Source Reconciliation

This RFC distinguishes between three layers of incident description:

| Layer | Description | Confidence for RFC-002 purposes |
|---|---|---|
| Public reporting baseline | Bridge drained for approximately USD 11.5M to USD 11.7M; assets included ETH, tBTC, and USDC; stolen assets converted into ETH; Tornado Cash funding reported. | High |
| External security analysis | The Ethereum-side bridge validation failed to enforce the relationship between source-side value/input and destination-side payout/output. The bridge accepted a valid proof or proof-like message for an invalid economic claim. | High |
| Verus-specific post-mortem analysis | The underlying mechanism involved supplemental export outputs on Verus being accepted as non-active by the Verus daemon but interpreted as active primary export material by the Ethereum contract. | High if relying on the Verus post-mortem; narrower than public reporting |

RFC-002 uses the incident as a reference case for a general Arrow rule. The rule does not require every implementation to use Verus terminology such as “supplemental export output” or “primary export.” The reusable class is broader:

> A destination system verified that some source-chain data existed, but did not verify that the data had the source-chain type, role, authorization status, and economic meaning required by the destination-side action.

### 1.3 Relationship to Arrow's Core Premise

Arrow exists to solve the problem that systems cannot agree on *what happened*. The Verus case sharpens that premise: the systems did not merely disagree about whether data existed. They disagreed about what the data *meant*.

Arrow v1.3 already distinguishes state, policy, finality, and confidence. However, the existing validation stack does not explicitly reject the case where:

1. a proof is valid as to content;
2. the destination event asserts a stronger type or operational meaning than the proof establishes; and
3. the verification regime lacks a disclosed mechanism for binding type classification to the proven claim.

This is the gap RFC-002 addresses.

### 1.4 RFC-001 Open Question Q1

RFC-001 identified the following open question:

> **Q1 — Is `chain_trust_flag` the right primitive for bridge-level revocation?**

The Verus incident confirms that chain-level revocation is too blunt for bridge-specific failures. The Verus chain itself was not necessarily compromised as a chain. The operational failure was bridge-specific. A bridge-specific trust primitive is therefore required.

RFC-002 formally proposes the Bridge Registry for Arrow v1.4.

---

## 2. Cross-Domain Semantic Consistency Invariant

This RFC establishes the following named Arrow design principle:

> **Cross-Domain Semantic Consistency Invariant:**
> A proof submitted to, referenced by, or relied on by a destination-domain event must prove not only the *content* of the source-domain data, but also the source-domain classification, operational role, and authorization-relevant type of that data. Where a verification regime cannot enforce this binding, or where type is inferred from submission context rather than proven or attested, the resulting Arrow event MUST NOT exceed Confidence C.

Operationally:

```text
Content proof ≠ type proof.
Existence proof ≠ authorization proof.
Hash match ≠ semantic validity.
```

The invariant applies to bridges, cross-chain messaging systems, rollup withdrawals, oracle attestations, legal attestations, institutional attestations, application programming interface (API) feeds, and any other Arrow event that relies on a source-domain proof.

However, RFC-002 does not treat all of these systems as the same kind of verifier. Arrow v1.4 should distinguish between:

1. the **verification regime** — how the claim is verified;
2. the **verification actor** — who or what performs the verification;
3. the **verified property** — whether the regime verifies content, classification, economic consistency, authority, finality, or only an external fact; and
4. the **rejection behavior** — whether failure invalidates the event, caps Confidence, forces Policy BL, or defers to deployment policy.

This distinction matters because bridges, oracles, custodians, legal authorities, and APIs can all appear to be “third parties,” but they do not create the same trust profile. A bridge may verify a Merkle proof but fail to verify economic consistency. An oracle may verify an external price but not source-chain authorization. A legal attestation may verify a court order but not cryptographic finality. Confidence C and Confidence D therefore should not be assigned by actor label alone.

### 2.1 Confidence C vs Confidence D — Interpretive Clarification

RFC-002 proposes the following interpretive distinction for Arrow v1.4:

| Confidence | Working Meaning | Typical Verification Profile | Policy Implication |
|---|---|---|---|
| C | Trusted third-party or structured attestation. The verifier is identifiable, the claim format is structured, and the claim can be audited or challenged. | Institutional attestation, legal attestation, audited oracle adapter, bridge watcher committee, custodian report, signed validator/notary statement. | May support PR/TR only where SEM-009 is satisfied and deployment policy accepts C. |
| D | Unstructured or weak external dependency. The source may be observable, but the verification path is not sufficiently accountable, reproducible, registry-bound, cryptographically authenticated, or procedurally normalized. | Raw API feed, unaudited oracle endpoint, scraper, informal human report, weak bridge telemetry, screenshot, PDF copy, cryptographically or registry-unauthenticated legal document, unsupported explorer observation. | Should generally be observational only; PR/TR should be disfavored or deployment-policy blocked. |
| E | Unverified, self-asserted, contradictory, or unresolved. | User claim, unauthenticated data, unresolved conflict, stale or broken feed. | SEM-009 already prohibits pairing Confidence E with PR or TR. |

The practical distinction is accountability plus structure. Confidence C means Arrow can identify a verification actor, regime, registry entry, structured claim format, and challenge path. Confidence D means Arrow may have a data source or document, but does not yet have enough structure to treat the source as a trusted verifier.

For legal documents, “unauthenticated” should be read narrowly. A legal document may be valid as evidence under off-chain law even if it is not yet machine-verifiable by Arrow. The Arrow question is not whether the document could have legal effect in court. The Arrow question is whether the document is cryptographically authenticated, registry-verifiable, procedurally traceable, or attested by an accountable legal oracle strongly enough to support an on-chain or cross-domain event.

### 2.2 Proposed Canonical Confidence Definition Update — v1.4

RFC-002 proposes the following update to the canonical Confidence Registry for Arrow v1.4. This update does not change the canonical six-field Arrow message string. It clarifies the meaning of Confidence C, D, and E so that downstream validation, registries, and deployment policy can apply them consistently.

| Confidence | Proposed v1.4 Canonical Meaning | Verification Structure |
|---|---|---|
| A | Native or canonical verification. The claim is verified directly by the authoritative source system, canonical state, or equivalent native verifier. | Direct canonical source verification. |
| B | Strong indirect verification. The claim is verified by robust settlement, quorum, proof, multi-source corroboration, or multi-attested structured verification with clear accountability and reproducibility. | Strong non-native verification; usually structured, auditable, and independently reviewable. |
| C | Structured accountable attestation. The claim is verified by an identifiable third party, institution, oracle, bridge operator, legal authority, custodian, notary, registry-bound source, or equivalent accountable actor using structured data, documented methodology, and a challenge, audit, or review path. | Structured trust. The verifier and claim format are identifiable, accountable, and reviewable. |
| D | Unstructured or weak external observation. The claim is externally observable or document-supported, but lacks sufficient structure, accountability, reproducibility, registry binding, cryptographic authentication, procedural normalization, or challenge path. | Unstructured observation. Suitable for awareness, monitoring, or provisional state, but not normally for high-consequence execution without additional verification. |
| E | Unverified, self-asserted, contradictory, stale, broken, unresolved, or otherwise unreliable. | No adequate verification structure. SEM-009 continues to prohibit Confidence E with Policy PR or TR. |

**Canonical clarification:** Confidence is not determined by the topic or actor category alone. A bridge message, oracle report, court order, application programming interface (API) feed, institutional attestation, notarial instrument, or legal document may fall into C, D, or E depending on the verification structure. The question is whether the claim is structured, accountable, reproducible, registry-bound, cryptographically or procedurally authenticated, and challengeable.

**Practical rule:** C means structured trust. D means unstructured observation. E means unverified or unreliable.

For v1.4, bridges and oracles should not be automatically assigned to C or D. Instead, they should be classified through subprofiles:

- a bridge with source proof verification, audited adapter logic, and explicit classification/economic consistency checks may support Confidence C or B depending on the verification regime;
- a bridge that infers source type from destination context should be capped at C under SEM-017 and may be downgraded to D by deployment policy if monitoring is weak;
- an oracle with signed reports, accountable operators, dispute windows, and source methodology may support Confidence C;
- a legal document oracle that verifies court registry data, electronic signatures, document hashes, issuing authority, procedural status, and scope may support Confidence C;
- a legal document oracle with multiple independent structured checks — for example official registry confirmation, valid electronic signature, document hash match, issuing-authority validation, and notarial or institutional attestation — may support Confidence B if the applicable verification regime permits it;
- a raw API or unaudited oracle feed should default to Confidence D;
- an unsupported or contradictory feed should default to Confidence E.

---

## 3. Protocol Changes Proposed for Arrow v1.4

### 3.1 Confidence Subprofiles — Proposed Companion Registry

RFC-002 recommends that Arrow v1.4 add a companion registry or companion document similar to the Validation Rules document:

```text
/registries/confidence_subprofiles.json
```

Purpose: distinguish categories of third parties, bridges, oracles, APIs, and attestations without forcing all of them into a single Confidence C or Confidence D bucket.

Candidate schema:

```json
{
  "subprofile_id": "bridge_audited_classification_bound",
  "category": "bridge",
  "default_confidence_ceiling": "B",
  "requires_registry_entry": true,
  "requires_accountable_operator": true,
  "requires_audit_or_review_ref": true,
  "requires_classification_binding": true,
  "requires_economic_consistency": true,
  "default_rejection_behavior": "reject_if_claim_exceeds_ceiling",
  "notes": "Bridge profile for audited or independently reviewed bridges that bind content, classification, and economic consistency."
}
```

```json
{
  "subprofile_id": "oracle_signed_accountable",
  "category": "oracle",
  "default_confidence_ceiling": "C",
  "requires_registry_entry": true,
  "requires_accountable_operator": true,
  "requires_source_methodology": true,
  "requires_dispute_or_challenge_path": true,
  "default_rejection_behavior": "cap_confidence",
  "notes": "Oracle profile for signed reports from accountable operators with documented methodology."
}
```

```json
{
  "subprofile_id": "legal_document_oracle_structured",
  "category": "legal_oracle",
  "default_confidence_ceiling": "C",
  "requires_registry_entry": true,
  "requires_accountable_operator": true,
  "requires_source_methodology": true,
  "requires_document_hash_or_signature_check": true,
  "requires_issuing_authority_check": true,
  "requires_procedural_status_check": true,
  "requires_dispute_or_challenge_path": true,
  "default_rejection_behavior": "cap_confidence",
  "notes": "Legal oracle profile for structured translation of off-chain legal documents, court orders, judgments, notarial instruments, or administrative acts into Arrow events."
}
```

```json
{
  "subprofile_id": "legal_document_oracle_multi_attested",
  "category": "legal_oracle",
  "default_confidence_ceiling": "B",
  "requires_registry_entry": true,
  "requires_accountable_operator": true,
  "requires_source_methodology": true,
  "requires_document_hash_or_signature_check": true,
  "requires_official_or_semi_official_data_source": true,
  "requires_issuing_authority_check": true,
  "requires_procedural_status_check": true,
  "requires_independent_attestation": true,
  "requires_dispute_or_challenge_path": true,
  "default_rejection_behavior": "reject_if_claim_exceeds_ceiling",
  "notes": "Higher-confidence legal oracle profile where multiple structured primitives corroborate the same legal document or event. This may include official publication, verifiable electronic signature, document hash match, notarial attestation, institutional attestation, or equivalent legally accountable verification."
}
```

```json
{
  "subprofile_id": "api_raw_unaccountable",
  "category": "api",
  "default_confidence_ceiling": "D",
  "requires_registry_entry": false,
  "requires_accountable_operator": false,
  "requires_source_methodology": false,
  "requires_dispute_or_challenge_path": false,
  "default_rejection_behavior": "defer_to_policy",
  "notes": "Raw API, scraper, or unaudited feed. Suitable for observation, not for high-consequence execution without additional verification."
}
```

This registry should not replace SEM-015. It should feed SEM-015 by specifying more precise ceilings for classes of third-party and oracle-like verification.

### 3.2 SEM-017 — Proof Classification Binding

**Position in validation order:** after SEM-016 — Chain Trust Flag; after BRG-001 where a `bridge_id` is present; before SEM-015 — Regime / Confidence Ceiling.

**Rule:**

A `proof_ref` that proves the existence of a source-domain data structure MUST bind, disclose, or otherwise establish all of the following:

1. `proven_content` — the data, hash, state element, message, output, event, or object that exists in the source domain;
2. `source_claim_type` — the source-domain classification of the proven object under the source protocol, registry, contract, legal authority, or attestation framework;
3. `destination_claim_type` — the destination-domain type or operational meaning asserted by the Arrow event;
4. `classification_binding_method` — the mechanism by which the classification is proven, attested, audited, or inferred;
5. `classification_verified` — boolean result of the classification check.

If `classification_verified == false`, the event MUST NOT exceed Confidence C.

If the applicable bridge or verification regime declares that classification binding is required and the event does not establish it, the event MUST be rejected with reason code `PROOF_TYPE_UNBOUND`.

If the applicable bridge or verification regime is not classification-bound and the submitted event claims Confidence A or B, the event MUST be rejected with reason code `CLASSIFICATION_REGIME_CEILING`.

### 3.3 SEM-018 — Classification Claim Transparency

**Rule:**

Where a proof-backed Arrow event asserts that a source-domain object has a specific operational type, the event SHOULD include the following metadata fields:

```json
{
  "source_claim_type": "supplemental_export_output",
  "destination_claim_type": "primary_export",
  "classification_declared": true,
  "classification_verified": false,
  "classification_binding_method": "context_inference",
  "classification_audit_ref": null
}
```

If `source_claim_type != destination_claim_type`, or if `classification_verified == false`, the event MUST NOT exceed Confidence C unless the applicable Bridge Registry entry documents a valid classification-binding mechanism and the event references it.

**Purpose:** SEM-018 makes the semantic mismatch machine-readable. SEM-017 enforces the classification-binding consequence; SEM-018 specifies the disclosure format.

### 3.4 SEM-019 — Economic Consistency Binding

**Position in validation order:** after SEM-017 and SEM-018; before SEM-015 — Regime / Confidence Ceiling.

**Rule:**

A value-releasing, value-minting, value-unlocking, or settlement-triggering Arrow event, as declared by the applicable verification regime or deployment policy, MUST prove, attest, or disclose that the source-side value, authority, and destination-side payout are economically consistent under the applicable bridge, oracle, legal, institutional, or verification regime.

At minimum, the event SHOULD disclose:

```json
{
  "economic_consistency": {
    "source_value_declared": true,
    "source_value_verified": false,
    "destination_value_claimed": true,
    "source_destination_value_match": false,
    "economic_consistency_verified": false,
    "economic_consistency_method": "none",
    "economic_consistency_audit_ref": null
  }
}
```

If `source_value_verified == false`, `source_destination_value_match == false`, or `economic_consistency_verified == false`, the event MUST NOT exceed Confidence C. `economic_consistency_verified` is a computed or declared boolean indicating that the applicable economic-consistency verification process has actually run and passed.

If the event attempts to claim Confidence A or B without establishing economic consistency, or where `economic_consistency_audit_ref == null` and `economic_consistency_verified == false`, the event MUST be rejected with reason code `ECONOMIC_CONSISTENCY_UNBOUND`.

**Interpretation:** SEM-017 covers classification binding. SEM-018 covers classification disclosure. SEM-019 covers economic value. Together they close the full failure path demonstrated by the Verus reference case: a proof-backed import may establish that some source-domain data exists while failing to establish that the source-side type, value, or authority matches the destination-side payout.

### 3.5 Classification Binding Methods

The following controlled vocabulary is proposed for v1.4:

| Value | Meaning | Default Confidence Implication |
|---|---|---|
| `none` | No classification-binding mechanism disclosed. | C ceiling |
| `context_inference` | Destination system infers type from the submission path, function call, caller behavior, or envelope context. | C ceiling |
| `source_fullnode_validation` | Source-domain full-node or equivalent canonical verifier enforces type classification under source protocol rules. | Eligible for A/B depending on regime |
| `merkle_inclusion_plus_type_tag` | Merkle or state proof includes both content and explicit type tag. | Eligible for A/B depending on regime |
| `zk_proof` | Zero-Knowledge Proof (ZKP) proves both content inclusion and the relevant classification predicate inside the circuit's constraint system. | Eligible for A/B only if the type predicate is a proven statement |
| `notarized_type_attestation` | Authorized notary, validator set, institution, or legal actor attests to both content and type. | Eligible for B/C depending on attestation quality |
| `audited_bridge_adapter` | Audited adapter maps source-domain types to destination-domain types under documented logic. | Eligible for B/C; A only if backed by canonical source verification |

**ZKP qualification:** A ZKP of Merkle inclusion or state inclusion does not by itself prove type classification. `zk_proof` is eligible for A/B only where the circuit's constraint system includes the source-domain type classification predicate as a constrained statement, not merely as contextual input. A ZKP proving content inclusion without an explicit type constraint reverts to `merkle_inclusion_plus_type_tag` or `context_inference`, depending on what is actually proven.

### 3.6 Verification Regimes Registry Extension

Each entry in `/registries/verification_regimes.json` gains the following mandatory fields in v1.4:

```json
{
  "regime_id": "native_canonical",
  "description": "Origin's own canonical state is the proof.",
  "max_confidence": "A",
  "classification_default": {
    "classification_declared": true,
    "classification_verified_default": true,
    "classification_binding_method_default": "source_fullnode_validation",
    "classification_confidence_ceiling": "A",
    "classification_notes": "Full-node validation enforces source-domain object type natively."
  }
}
```

```json
{
  "regime_id": "bonded_quorum",
  "description": "Economic stake / quorum threshold.",
  "max_confidence": "B",
  "classification_default": {
    "classification_declared": false,
    "classification_verified_default": false,
    "classification_binding_method_default": "context_inference",
    "classification_confidence_ceiling": "C",
    "classification_notes": "Quorum attestations often prove message acceptance or signer agreement, but do not necessarily prove source-domain object classification unless expressly included in the attested payload."
  }
}
```

```json
{
  "regime_id": "institutional_attestation",
  "description": "Institutional or custodian attestation.",
  "max_confidence": "B",
  "classification_default": {
    "classification_declared": false,
    "classification_verified_default": false,
    "classification_binding_method_default": "notarized_type_attestation",
    "classification_confidence_ceiling": "C",
    "classification_notes": "Institutional attestations may attest to external facts, but foreign-chain object type must be expressly included before classification is treated as verified."
  }
}
```

```json
{
  "regime_id": "legal_oracle_structured",
  "description": "Structured legal oracle translating off-chain legal documents or legal events into Arrow events through accountable verification procedures.",
  "max_confidence": "C",
  "classification_default": {
    "classification_declared": true,
    "classification_verified_default": false,
    "classification_verified_condition": "conditional_on_oracle_checks",
    "classification_binding_method_default": "notarized_type_attestation",
    "classification_confidence_ceiling": "C",
    "classification_notes": "Legal type classification is verified only if the required legal-oracle checks have run and passed, including document hash or signature checks, issuing-authority confirmation, and procedural status review. This does not imply native cryptographic finality of the underlying legal event."
  }
}
```
```json
{
  "regime_id": "optimistic_settled",
  "description": "Settlement-backed verification where an optimistic challenge window, settlement window, or equivalent dispute period affects final confidence.",
  "max_confidence": "A",
  "classification_default": {
    "classification_declared": true,
    "classification_verified_default": false,
    "classification_verified_condition": "conditional_on_type_tag_or_settlement_proof",
    "classification_binding_method_default": "merkle_inclusion_plus_type_tag",
    "classification_confidence_ceiling": "A",
    "classification_notes": "Eligible for Confidence A only after the relevant settlement or challenge window has passed and the proof includes the required classification predicate or explicit type tag. Otherwise classification falls back to context inference and is capped at C."
  }
}
```

```json
{
  "regime_id": "legal_attestation",
  "description": "Legal authority, court, arbitrator, notary, regulator, or legally accountable institution attests to an off-chain legal fact, document, order, or status.",
  "max_confidence": "C",
  "classification_default": {
    "classification_declared": true,
    "classification_verified_default": false,
    "classification_verified_condition": "conditional_on_structured_legal_attestation",
    "classification_binding_method_default": "notarized_type_attestation",
    "classification_confidence_ceiling": "C",
    "classification_notes": "Legal attestation supports Confidence C only where the attestation identifies the legal document or legal event, issuing authority, procedural status, scope, and verification method. Unstructured legal documents or unverified copies remain D or E depending on source quality."
  }
}
```

#### Default Regime Table

| Regime | Default Binding Method | Default Classification Verified | Classification Ceiling |
|---|---:|---:|---:|
| `native_canonical` | `source_fullnode_validation` | `true` | A |
| `optimistic_settled` | `merkle_inclusion_plus_type_tag` if included; otherwise `context_inference` | conditional | A if included; C otherwise |
| `bonded_quorum` | `context_inference` | `false` | C |
| `institutional_attestation` | `notarized_type_attestation` only if expressly included | conditional | C by default; B only under an applicable multi-attested subprofile |
| `legal_attestation` | `notarized_type_attestation` only if expressly included | conditional | C by default; B only under an applicable multi-attested legal subprofile |
| `legal_oracle_structured` | `notarized_type_attestation` plus document hash/signature and issuing-authority checks | conditional on required oracle checks | C by default; B only under `legal_document_oracle_multi_attested` or equivalent subprofile |

**Bonded quorum implication:** The bonded-quorum C ceiling applies by default to all quorum-attested cross-chain events, including native notarization systems, unless a deployment-specific registry entry documents classification binding and economic consistency. This is conservative by design. Deployments relying on high-integrity notary sets may register a bridge-level or system-level override with documented classification binding, economic consistency checks, and independent review, elevating the ceiling to B where justified.

**Potential v1.4 registry expansion:** Verus-style native notarization, especially Protocol-Based Blockchain-as-a-Service (PBaaS) to PBaaS notarization that does not depend on an external destination bridge contract, may deserve its own verification regime sitting between `bonded_quorum` and `native_canonical`. Candidate regime name: `native_notarized_consensus`. This RFC does not define that regime but flags it for the v1.4 registry expansion track.

**Interpretive rule:** regime-level values are defaults only. Bridge-level, oracle-level, legal-oracle-level, or deployment-level entries override them only if they document the specific classification-binding mechanism and satisfy the applicable subprofile requirements. A subprofile may set a ceiling above the regime default only where the subprofile independently satisfies the classification, economic consistency, audit, and accountability requirements that justify the higher ceiling.

---

## 4. Bridge Registry — v1.4 Formal Proposal

### 4.1 Purpose

The Bridge Registry provides a bridge-specific trust, classification, and status layer. It prevents Arrow from using chain-level trust flags to solve bridge-level failures.

**Proposed file:** `/registries/bridge_registry.json`

### 4.2 Bridge Registry Schema

```json
{
  "bridge_id": "VERUS-ETH-1",
  "bridge_name": "Verus-Ethereum Bridge",
  "source_chain_id": "VRSC-1",
  "destination_chain_id": "ETH-1",
  "source_origin_code": "VRS",
  "destination_origin_code": "ETH",
  "bridge_contract_address": "0x715185...",
  "bridge_contract_version": null,
  "verification_regime": "bonded_quorum",
  "requires_subprofile": false,
  "classification_profile": {
    "classification_declared": true,
    "classification_verified": false,
    "classification_binding_method": "context_inference",
    "classification_confidence_ceiling": "C",
    "classification_notes": "Reference-case analysis indicates supplemental-vs-primary export classification was not enforced at the Ethereum contract layer. External analysis also describes the incident as a validation failure between source-side value/input and destination-side payout/output."
  },
  "bridge_trust_flag": 0,
  "bridge_status": "compromised",
  "bridge_status_reason": "Semantic validation failure confirmed 2026-05-18. Ethereum-side bridge accepted a proof-backed import that did not establish the full source-domain claim executed on Ethereum.",
  "bridge_status_effective_from": "2026-05-18T00:00:00Z",
  "registry_version": 1,
  "restoration_confidence_ceiling": null,
  "restoration_ceiling_effective_until": null,
  "restoration_conditions": {
    "requires_redeployment": true,
    "requires_independent_review": true,
    "requires_classification_bound_proof": true,
    "requires_effective_block_or_timestamp": true,
    "requires_residual_confidence_ceiling": true
  },
  "classification_audit_ref": null,
  "notes": "Post-exploit registry entry. bridge_trust_flag may be restored only after BRG-005 conditions are satisfied."
}
```

### 4.3 Bridge Registry Validation Rules

| Rule | Constraint | Rejection Behavior |
|---|---|---|
| BRG-001 | Events referencing a bridge with `bridge_trust_flag == 0` MUST have Policy BL, regardless of all other field values. Non-overridable. | Force Policy BL; reject PR/TR/HL/RW/CL claims. |
| BRG-002 | `bridge_trust_flag == 0` requires `bridge_status`, `bridge_status_reason`, and `bridge_status_effective_from` to be populated. | Reject registry entry or hold pending registry completion. |
| BRG-003 | A bridge declaring `classification_profile.classification_verified == true` and claiming above Confidence C MUST document the enforcement mechanism and provide `classification_audit_ref` pointing to an independent audit, formal verification result, canonical verifier, or governance-approved review record. Self-certification alone is insufficient for `classification_confidence_ceiling` above C. | Reject classification-verified claim above C; cap event Confidence at C or D depending on subprofile. |
| BRG-004 | Changes to `verification_regime`, `classification_profile`, `source_chain_id`, `destination_chain_id`, or `bridge_contract_address` MUST trigger a new Bridge Registry version. | Reject event if registry version mismatch exists. |
| BRG-005 | A bridge disabled for semantic classification failure or economic consistency failure MUST NOT be restored to `bridge_trust_flag == 1` unless the entry includes a patched or redeployed version, a classification-binding mechanism, an economic consistency mechanism, an independent review or audit reference, an effective deployment block/time, and a residual Confidence ceiling. Upon restoration, the Bridge Registry entry MUST specify `restoration_confidence_ceiling` of B or lower and a `restoration_ceiling_effective_until` period measured in blocks, days, or governance epochs. Restoration to Confidence A is unavailable until the ceiling period has elapsed and a subsequent review is recorded. The minimum duration for the restoration ceiling is a governance parameter. See Open Question Q1. | Reject restoration; emit `BRIDGE_RESTORATION_BLOCKED`. |
| BRG-006 | A bridge profile MUST identify its applicable confidence subprofile where bridge-origin events claim Confidence above D. | Cap Confidence at D if no subprofile is declared. Reject only where the Bridge Registry entry expressly sets `requires_subprofile: true`. |

### 4.4 Relationship to `chain_trust_flag`

`chain_trust_flag` and `bridge_trust_flag` are independent.

A bridge may have `bridge_trust_flag: 0` while both source and destination chains remain `chain_trust_flag: 1`. An event from a compromised bridge is rejected at BRG-001 regardless of chain trust status. This preserves the ability to observe valid events on both chains while blocking claims routed through the compromised bridge.

### 4.5 Scope and Liability Boundary

The Bridge Registry is a trust classification and event-consequence layer. It is not a bridge governance mechanism, not an audit certification, and not a general-purpose safety rating.

Entries in the Bridge Registry express Arrow's routing and validation posture for Arrow-compatible events. They should not be represented as authoritative assessments of bridge safety for investment, custody, insurance, or operational-risk purposes unless a separate audit, legal opinion, or governance process expressly says so.

This distinction matters because protocols may rely on Arrow registry entries when deciding whether to route, block, downgrade, or release value. Arrow's bridge registry therefore carries operational consequences, but it should remain scoped as a protocol-level trust-classification primitive, not as a substitute for bridge due diligence.

---

## 5. Reason Codes — v1.4 Additions

The following entries are proposed for `/registries/reason_codes.json`:

```json
[
  {
    "reason_code": "PROOF_TYPE_UNBOUND",
    "description": "A proof-backed event asserts an operational type or destination-side claim that is not bound to the source-domain classification of the proven object.",
    "expected_class": 3,
    "expected_policy": "BL",
    "expected_state": ["CD", "FN"],
    "introduced_in": "RFC-002",
    "target_version": "v1.4"
  },
  {
    "reason_code": "CLASSIFICATION_REGIME_CEILING",
    "description": "The applicable verification regime or bridge profile does not verify source-domain classification and therefore cannot support Confidence A or B.",
    "expected_class": 3,
    "expected_policy": "BL",
    "expected_state": ["FN"],
    "introduced_in": "RFC-002",
    "target_version": "v1.4"
  },
  {
    "reason_code": "ECONOMIC_CONSISTENCY_UNBOUND",
    "description": "A value-releasing or settlement-triggering event does not prove or attest that source-side value, authority, and destination-side payout are economically consistent under the applicable bridge or verification regime.",
    "expected_class": 3,
    "expected_policy": "BL",
    "expected_state": ["CD", "FN"],
    "introduced_in": "RFC-002",
    "target_version": "v1.4"
  },
  {
    "reason_code": "BRIDGE_TRUST_REVOKED",
    "description": "The referenced bridge has been disabled or revoked in the Bridge Registry.",
    "expected_class": 4,
    "expected_policy": "BL",
    "expected_state": ["FN"],
    "introduced_in": "RFC-002",
    "target_version": "v1.4"
  },
  {
    "reason_code": "BRIDGE_RESTORATION_BLOCKED",
    "description": "A bridge restoration attempt failed because BRG-005 restoration conditions were not satisfied.",
    "expected_class": 3,
    "expected_policy": "BL",
    "expected_state": ["CD", "FN"],
    "introduced_in": "RFC-002",
    "target_version": "v1.4"
  },
  {
    "reason_code": "ANOMALOUS_PENDING_EXPORT_ACCUMULATION",
    "description": "A bridge export queue accumulated blank, non-standard, or economically inconsistent exports within a short window under suspicious funding or submission conditions.",
    "expected_class": 1,
    "expected_policy": "HL",
    "expected_state": ["PD", "CD"],
    "introduced_in": "RFC-002",
    "target_version": "v1.4"
  }
]
```

---

## 6. Event Metadata Schema — v1.4 Candidate

Where `proof_ref` is present, Arrow v1.4 SHOULD support the following optional metadata object:

```json
{
  "proof_semantics": {
    "proof_ref": "tx://verus/f899e6984dc7c3d7737bbca5d87db3682de355743349d40396a5fc34b9f5a733#supplemental_output_1",
    "proof_type": "cross_chain_merkle_proof",
    "source_claim_type": "supplemental_export_output",
    "destination_claim_type": "primary_export",
    "classification_declared": true,
    "classification_verified": false,
    "classification_binding_method": "context_inference",
    "classification_audit_ref": null,
    "source_amount": "0",
    "destination_amount": "11600000_usd_equivalent"
  }
}
```

`proof_semantics` describes what the proof claims and how the source and destination meanings relate. Economic-consistency verification results belong in the `economic_consistency` object below. Where legacy or implementation-specific events include `proof_semantics.economic_consistency_verified`, SEM-019 applies the stricter value if it conflicts with `economic_consistency.economic_consistency_verified`.

### 6.1 Economic Consistency Extension

External analysis of the Verus exploit emphasizes that the bridge failed to enforce economic consistency between the source-side input and destination-side payout. RFC-002 therefore includes an economic consistency extension to avoid over-narrowing the rule to type labels alone.

```json
{
  "economic_consistency": {
    "source_value_declared": true,
    "source_value_verified": false,
    "destination_value_claimed": true,
    "source_destination_value_match": false,
    "economic_consistency_verified": false,
    "economic_consistency_method": "none",
    "economic_consistency_audit_ref": null
  }
}
```

**Interpretation:** If a destination-side event causes value release, minting, unlocking, settlement, or deployment-policy-defined performance, the proof semantics MUST include both classification consistency and economic consistency. Failure of either dimension caps Confidence at C. Attempts to claim Confidence A or B without economic consistency MUST be rejected under SEM-019.

---

## 7. Updated Validation Order — v1.4 Candidate

```text
1.  Parse canonical message string                     → reject if MSG-001 to MSG-005 fail
2.  Resolve Origin → Chain Registry entry              → reject if origin unregistered
3.  Check chain_trust_flag (SEM-016)                   → reject with Policy BL if 0
4.  If bridge_id is present, resolve Bridge Registry    → reject if bridge is unregistered
5.  Check bridge_trust_flag (BRG-001)                  → reject with Policy BL if 0
6.  Resolve verification_regime                         → reject if required regime field is null
7.  Resolve confidence subprofile if present/required   → cap at D if missing for third-party bridge/oracle/API claims above D; continue validation using capped Confidence
8.  Apply semantic constraints (SEM-001 to SEM-014)     → reject if any fail
9.  Apply proof classification binding (SEM-017)        → reject or cap according to SEM-017
10. Apply classification transparency (SEM-018)         → require/cap where claim type fields conflict or are absent
11. Apply economic consistency binding (SEM-019):        → if missing or failed, cap at Confidence C
    11a. If Confidence A/B is claimed without economic consistency, reject with ECONOMIC_CONSISTENCY_UNBOUND
12. Apply regime/confidence consistency (SEM-015)       → reject if Confidence exceeds regime ceiling
13. Apply subprofile confidence ceiling                 → reject/cap if Confidence exceeds subprofile ceiling
14. Apply bridge-specific confidence ceiling            → reject if Confidence exceeds Bridge Registry ceiling
15. For multi-source observations: compute numeric       → suspend if divergence detected
    confidence and check divergence (DIV rules)
16. Apply protocol floor (SEM-009)                      → reject if Confidence E with Policy PR/TR
17. Pass to deployment policy layer
```

Step 7 applies a Confidence cap and does not halt validation. Subsequent validation steps apply to the capped Confidence value unless a later rule requires rejection.

---

## 8. Observation State Machine Extension

**Architectural premise:** Arrow does not autonomously monitor chains in v1.3 or v1.4. The conditions in this section are raised by external monitoring agents, indexers, bridge watchers, validators, auditors, or other observation sources that submit Arrow observation events. Arrow's role is to validate the submitted observation event and enforce the state-transition consequence; it is not to independently detect the condition unless a future Arrow deployment expressly includes active chain-reading capability.

### 8.1 Condition: `ANOMALOUS_PENDING_EXPORT_ACCUMULATION`

The Verus incident included a preparation pattern that may be detectable before exploit execution: blank or non-standard export transactions, a recently funded address, and no legitimate transfer payload.

**Trigger criteria — suggested default:**

1. A bridge export queue receives multiple blank, non-standard, or economically inconsistent export transactions within a short window.
2. The submitting address was recently funded.
3. The export transactions contain no legitimate transfer payload or contain payloads inconsistent with their declared export type.
4. The destination bridge has high-value reserves or high-value unlock authority.

**State transition:** ACTIVE → CONDITIONAL on the affected bridge.

**Effect:** Events from the bridge during the CONDITIONAL window MUST NOT exceed Confidence C unless manually cleared or independently verified.

**Resolution:** CONDITIONAL → ACTIVE upon manual review, legitimate export reconciliation, or proof that the anomalous queue state is harmless. CONDITIONAL → HALTED if exploit execution, invalid import, or bridge-side revocation is confirmed.

---

## 9. Verus Reference Event Sequence in Arrow

```text
PRE-EXPLOIT OBSERVATION

Step 1 — Blank / anomalous export injection
1:VRS:PD:HL:0:C
  reason_code: ANOMALOUS_PENDING_EXPORT_ACCUMULATION
  object_type: bridge_export_queue
  object_id: VERUS-ETH-1
  metadata.pending_exports: 4
  metadata.legitimate_transfers: 0
  metadata.bridge_id: VERUS-ETH-1
  → Observation State Machine: ACTIVE → CONDITIONAL

Step 2 — Handcrafted cross-chain import submitted to Ethereum
3:ETH:FN:BL:1:C
  reason_code: PROOF_TYPE_UNBOUND
  object_type: cross_chain_import
  object_id: 0x6990f017...
  proof_type: cross_chain_merkle_proof
  proof_ref: tx://verus/f899e698...supplemental_output_1
  metadata.proof_semantics.source_claim_type: supplemental_export_output
  metadata.proof_semantics.destination_claim_type: primary_export
  metadata.proof_semantics.classification_verified: false
  metadata.economic_consistency.source_value_verified: false
  metadata.economic_consistency.source_destination_value_match: false
  metadata.economic_consistency.economic_consistency_verified: false
  metadata.bridge_id: VERUS-ETH-1
  → SEM-017: type not bound
  → SEM-018: classification mismatch disclosed
  → SEM-019: economic consistency not established
  → Confidence A/B claims rejected; event capped at C or rejected depending on policy

POST-EXPLOIT REGISTRY RESPONSE

Step 3 — Bridge revocation
4:ETH:FN:BL:1:B
  reason_code: BRIDGE_TRUST_REVOKED
  object_type: bridge_registry_entry
  object_id: VERUS-ETH-1
  metadata.bridge_trust_flag_prior: 1
  metadata.bridge_trust_flag_new: 0
  metadata.bridge_status: compromised
  metadata.bridge_status_reason: semantic validation failure; proof-backed import accepted without classification/economic consistency
```

---

## 10. What RFC-002 Does Not Propose

RFC-002 does not propose that Arrow verify cross-chain proofs directly. Arrow remains an event coordination, classification, and trust-calibration standard.

RFC-002 does not define a universal bridge implementation standard. It defines the claims a bridge-origin event may make inside Arrow and the Confidence consequences when those claims are not supported.

RFC-002 does not require changes to Arrow's canonical six-field message string.

RFC-002 does not use lettered validation identifiers for v1.4. Rule identifiers are sequential: SEM-017, SEM-018, SEM-019. Earlier draft labels such as SEM-017A were draft-only insertion artifacts.

RFC-002 does not resolve exploit recovery, restitution, insurance, litigation, or governance questions for Verus or any other affected community.

RFC-002 does not treat AI-assisted reconnaissance as an element of the exploit classification. Possible AI assistance is a threat-model consideration only.

---

## 11. Open Questions

### Q1 — Should restored bridges use staged confidence by default?

Yes, as a structural rule under BRG-005. The open governance question is the duration and review mechanism for the restoration ceiling. The registry should define whether the minimum period is measured in days, blocks, governance epochs, transaction count, or a combination of those measures.

### Q2 — Should ANOMALOUS_PENDING_EXPORT_ACCUMULATION thresholds be global or deployment-specific?

Deployment-specific thresholds are safer. A low-volume bridge and a high-volume bridge have materially different normal behavior.

### Q3 — Can structured legal oracles support Confidence B?

Possibly, but only where the legal oracle is not merely a trusted human intermediary. A legal oracle may support Confidence C where it provides structured, accountable translation of an off-chain legal document into an Arrow event. Confidence B should require multiple corroborating primitives, such as official or semi-official registry data, valid electronic signature verification, document hash matching, issuing-authority validation, procedural status checks, and independent notarial, institutional, or auditor attestation.

This is especially relevant in jurisdictions where court documents, administrative acts, judgments, or official records already contain verifiable electronic signatures, validation codes, hashes, QR codes, or public/semi-public database references. In those cases, the legal oracle is not inventing trust from scratch; it is aggregating and normalizing existing legal verification primitives into structured machine-readable form.

### Q4 — Should Verus-style native notarization become its own verification regime?

Possibly. PBaaS-native notarization that does not depend on an external destination bridge contract may not fit cleanly into either `bonded_quorum` or `native_canonical`. It may justify a new regime such as `native_notarized_consensus`, with its own default ceiling and classification/economic consistency requirements. This should be considered during the v1.4 Verification Regimes Registry expansion.

---

## 12. Build Checklist for Arrow v1.4

### Registry Files

- [ ] Add `/registries/bridge_registry.json`
- [ ] Extend `/registries/verification_regimes.json` with `classification_default`
- [ ] Extend `/registries/reason_codes.json` with RFC-002 reason codes
- [ ] Update `/registries/confidence.json` or equivalent canonical Confidence Registry with v1.4 C/D/E definitions from Section 2.2
- [ ] Consider adding `/registries/classification_binding_methods.json`
- [ ] Add `/registries/confidence_subprofiles.json` or equivalent companion document for bridges, oracles, APIs, and third-party attestation categories
- [ ] Add legal oracle subprofiles for structured legal documents, court orders, judgments, notarial instruments, and administrative acts
- [ ] Add `VRSC-1` to `/registries/chain_registry.json` if Verus remains the reference case
- [ ] Define sentinel handling for unknown bridge contract versions; use `null` or a controlled enum rather than freeform strings
- [ ] Consider adding `native_notarized_consensus` to `/registries/verification_regimes.json`

### Validation Rules

- [ ] Add SEM-017 — Proof Classification Binding
- [ ] Add SEM-018 — Classification Claim Transparency
- [ ] Add SEM-019 — Economic Consistency Binding
- [ ] Add BRG-001 to BRG-006

### Schema

- [ ] Add optional `proof_semantics` metadata object
- [ ] Add optional `economic_consistency` metadata object
- [ ] Add `bridge_id` as a first-class reference field where bridge-origin events are used
- [ ] Add `economic_consistency_audit_ref` where Confidence A/B is claimed for value-releasing events
- [ ] Add `restoration_confidence_ceiling` and `restoration_ceiling_effective_until` to Bridge Registry schema

### Tests

- [ ] Reject Confidence A/B when `classification_verified == false`
- [ ] Reject events from `bridge_trust_flag == 0`
- [ ] Reject bridge restoration without BRG-005 fields
- [ ] Cap events from `context_inference` classification method at Confidence C
- [ ] Cap unprofiled bridge/oracle/API events at Confidence D when they claim above D
- [ ] Validate that structured accountable attestations map to Confidence C, while unstructured external observations map to Confidence D unless upgraded by a declared subprofile
- [ ] Reject bridge-origin Confidence C/B/A claims where no confidence subprofile is declared
- [ ] Verify that chain-level trust remains active while bridge-level trust is revoked
- [ ] Classify cryptographically or registry-unauthenticated legal documents as D unless independently attested
- [ ] Permit legal oracle Confidence C only where structured legal verification fields are populated
- [ ] Permit legal oracle Confidence B only where multiple independent verification primitives are present
- [ ] Reject Confidence A/B for value-releasing events where `economic_consistency_verified == false`
- [ ] Reject Confidence A/B for value-releasing events where `economic_consistency_audit_ref == null` and `economic_consistency_verified == false`
- [ ] Reject bridge restoration where `restoration_confidence_ceiling` or `restoration_ceiling_effective_until` is missing
- [ ] Verify ZKP classification binding only where the circuit proves the type predicate

---

## 13. Conclusion

The Verus-Ethereum Bridge exploit is a strong reference case for Arrow because it shows a bridge accepting a proof-backed claim without proving the full semantic and economic meaning of that claim. The important design lesson is not that Arrow would have stopped the exploit directly. Arrow is not a bridge runtime or proof verifier.

The correct claim is narrower and stronger:

> Arrow v1.4 should make missing classification binding and missing economic consistency explicit, machine-readable, and confidence-limiting.

RFC-002 therefore converts the Verus incident into a reusable protocol extension: proof semantics, bridge-specific trust, classification transparency, economic consistency, confidence ceilings, and restoration gates.

---

## Document Information

| Field | Value |
|---|---|
| Document ID | ARROW-RFC-002 |
| Version | 1.0 |
| Status | Final v1.0 |
| Genesis Anchor | RFC-000 — `2026-03-22T18:40:00Z` |
| Underlying Incident | Verus-Ethereum Bridge Exploit — 2026-05-18 |
| Source Material | Verus post-mortem attributed to Mike Toutonghi; external reports from Halborn, CoinDesk, Bitcoin.com, and related security/news summaries |
| Primary Specification | Arrow Protocol v1.3 |
| Validation Rules Dependency | Arrow Validation Rules v1.3 |
| Proposed Target Version | Arrow Protocol v1.4 |
| Numbering Note | v1.0 uses sequential SEM-017, SEM-018, SEM-019; no lettered SEM identifiers are proposed for v1.4. |
| Addresses | RFC-001 Open Question Q1 |
