---
title: "RFC-007 — Declared Policy, Recognized Authority, and Integration Strength"
document_id: "ARROW-RFC-007"
version: "1.0"
status: "Published 2026-06-17 — specification graduated. The additive metadata surface, the recognized-authority model (Part B), and the capability-disclosure layer (Part D) are normative; the DPL-*/INT-* enforcement rules (Parts A/C) remain deferred from the live validation pipeline by design, gated on the adapter interchange contract and reduction layer."
date: "2026-06-03"
corrections_applied: "RFC-007 Suggested Corrections — Editing Checklist v0.1"
project: "Arrow Protocol"
depends_on: "RFC-004 v1.0; RFC-005 v1.0; RFC-006 v1.0 (graduated); LEG rules (LEG-001, LEG-004–010); Validation Rules v1.6.0"
reference_case: "Arbitrum Security Council emergency action (21/04/2026) following the KelpDAO / rsETH exploit, and the subsequent United States District Court proceedings affecting the frozen Ethereum"
canonical_message_string: "Unchanged. RFC-007 adds no seventh field."
validation_prefixes: "DPL- (Declared Policy), INT- (Integration Strength), CHN-005+ (Capability Disclosure, advisory)"
implementation_phase: "Phase 1 (Class 3 / SEM-006 mismatch) RESOLVED in v1.5.2-draft, commit 200633e — pipeline aligned to spec, Class 3 → Finality=0 constraint removed. Additive registry metadata (EMG, governance_capability) landed in v1.5.2-draft; reference fixtures E-02/E-03 committed. RFC-006 graduated 2026-06-16, lifting the §1 dependency hold. DPL/INT enforcement rules remain deferred from the live pipeline by design. RFC-007 published as v1.0 on 2026-06-17 — the specification surface is graduated; the enforcement layer stays deferred."
---

# RFC-007 — Declared Policy, Recognized Authority, and Integration Strength

## Abstract

The KelpDAO / Arbitrum incident demonstrates that finalized on-chain outcomes may later be affected by exceptional authority-installed Policy, external legal inputs, and infrastructure-level enforcement decisions. RFC-007 defines the narrow protocol surface this exposes.

Arrow should not dilute finality, invent runtime override detection, or treat legal documents as self-executing transactions. It should distinguish declared Policy establishment from deterministic Policy application; bind exceptional declarations to recognized authority and evidence; disclose where enforcement is installed; disclose whether the connected infrastructure can bypass that enforcement; and keep latent governance capabilities in advisory registry metadata rather than the semantic core.

This RFC is **metadata-first**. The first implementation pass lands safe additive registry entries and advisory blocks. Enforcement rules (`DPL-*`, `INT-*`) are defined here but are deferred from the live validation pipeline pending a mature adapter interchange contract and reduction layer.

RFC-007 is **additive and non-breaking**. The canonical six-field message string is unchanged.

---

## 1. Status and Scope

This RFC is **published (v1.0, 2026-06-17)** as a specification. Its landed surface is normative: the additive registry metadata (Patches v0.2 — `EMG` and `CHA` origins, ARB `governance_capability`, the `CapabilityDisclosure`/`GovernanceCapability` Rust structs), the recognized-authority model (Part B, backed by `registries/authorities.json`), and the advisory capability-disclosure layer (Part D, CHN-005–007). The `DPL-*` and `INT-*` **enforcement rules (Parts A and C) are intentionally not wired into the live validation pipeline** — they remain deferred by design, gated on a mature adapter interchange contract and reduction layer. Publication graduates the specification; it does not promote the enforcement layer. **RFC-006 graduated 2026-06-16** (published v1.0 2026-06-17), lifting this document's prior dependency hold: the capability-disclosure layer rides on the now-graduated RFC-006 v2.0 Chain Registry advisory block, which is no longer provisional.

**This RFC classifies and constrains. It does not adjudicate.** Arrow does not determine what law applies, does not decide whether a governance action was legitimate, and does not detect override functions in deployed code. It records asserted or verified policy-authority and integration metadata and enforces the coherence of what is asserted.

### 1.1 What this RFC explicitly does NOT do (resolved elsewhere or out of scope)

| Candidate from working notes | Disposition | Authority |
|---|---|---|
| Multiple machine-enforced finality categories (consensus / execution / governance / legal) | **Rejected.** Binary `Finality` axis plus `finality_basis` already expresses "settled but challengeable." `legal_appeal` is the basis for an operative-but-appealable order. | RFC-004 Part 7; `finality_basis.json` |
| `RV` as an undefined "returned value / classification / relationship" | **Closed.** `RV` is a **State** value (Reversed). A finalized event is contradicted only by Class 1 + State `RV` (reversal) or Class 4 (supersession), both referencing the prior event. | SEM-001, SEM-014 |
| `LEGAL` auto-executes a transaction | **Rejected as unsafe.** A `LEG` event asserts a permitted Action within the LEG derivation gate; it never executes. A `Finality=0` legal event is capped at `HL`/`RW`. | LEG-006; RFC-004 Part 7; LEG-004 |
| `BL` / `AL` as validator return values | **Re-framed.** `BL` is an **Action** value (Block); `PR` is the proceed Action; `AL` does not exist. Arrow accepts or rejects an asserted message. The enforcement point applies the Action. In expressly enumerated hard-override cases (SEM-016, BRG-001, LEG-007) Arrow Core forces Action `BL` as a non-overridable safety result. | `actions.json`; SEM-006; SEM-016; BRG-001; LEG-007 |
| `OR` override primitive; runtime detection of arbitrary override functions | **Rejected.** Arrow is not a code-auditing oracle. The lesson is absorbed as capability disclosure (Part D), not as a primitive. | Arrow Core §15 (Non-Goals) |

### 1.2 What this RFC owns

The policy-authorship boundary (derived vs declared Policy), the recognized-authority model, the honesty of integration claims, and capability disclosure. These are the genuine unaddressed surface — the protocol↔infrastructure interface. **Arrow Validators** (infrastructure-native enforcement points) are the first concrete instantiation of `integration_strength = infrastructure_native`.

---

## 2. Reference Case — the incident in canonical messages

The incident is reconstructed in actual six-field messages. `ARB`, `PCY`, and `EMG` are registered origins. Confidence values reflect the applicable regime ceiling per SEM-015.

> **Note on E2 and E3.** These fixtures assert Class 3 with `Finality=1`. This is valid under the specification — SEM-006 requires Class 3 to use Action `BL`, `HL`, or `RW`; it does not mandate `Finality=0`. The Class 3 → Finality=0 mismatch in the Rust pipeline (§13 Q1, Phase 1) was **resolved in v1.5.2-draft (commit `200633e`)**: SEM-006 now enforces only the action constraint, with regression tests that accept `3:ARB:FN:BL:1:A` and reject executing actions. Both fixtures are now valid under the corrected pipeline. E2 additionally requires the `EMG` origin, which was added to `origins.json` in v1.5.2-draft. The remaining note on E3 (§13 Q2) is a design question about whether the event is *mandatory*, not a validity blocker.

| # | Event (canonical) | Extensions / refs | Reading |
|---|---|---|---|
| E0 | `1:ARB:FN:PR:1:B` | — | Attest: the exploit-linked transfer settled. **Final. Stays final.** |
| E1 | `5:ARB:FN:PR:1:B` | `related_events:[E0]` | Execute: the Security-Council-compelled transfer to `0x…0DA0`. A **new forward event**, not `RV`. Action `PR` (Proceed) is correct — `TR` (Trigger) would assert a downstream conditional, not an executed compelled transfer; see §11 Q3. |
| E2 | `3:EMG:FN:HL:1:C` | `extensions.declared_policy`; `authority_ref` → Arbitrum Security Council | Restrict: the immobilization rule, **declared** by a recognized emergency authority, installed at the enforcement point. C is at the `institutional_attestation` ceiling. Valid under corrected SEM-006; `EMG` registered in v1.5.2-draft. |
| E3 | `3:ARB:FN:BL:1:B` *(see §13 Q2)* | `related_events:[E2]` | Restrict: a subsequent outbound attempt, **derived** from the installed rule (DPL-003). Per-transaction Block; no re-declaration. B reflects chain-observable evidence post-window. Valid under corrected SEM-006; §13 Q2 is a design question on whether this event is mandatory. |
| E4 | `2:LEG:FN:HL:0:C` | `leg_subtype:JUD`, `finality_basis:legal_appeal`, `leg_jurisdiction.country:US` | Authorize (legal): the court order while appealable. `FN+0` caps Action at `HL` (LEG-006; RFC-004). Operative, not finality-locked. |
| E5 | `2:PCY:FN:PR:1:B` | `extensions.declared_policy`; `related_events:[E4]` | Authorize: the DAO's amended operational policy permitting the Aave custody transfer — **declared** exceptional Policy referencing the legal input. B at the `bonded_quorum` ceiling. |
| E6 | `5:ARB:FN:PR:1:B` | `related_events:[E1,E5]` | Execute: the custody transfer to the Aave-controlled wallet. **`PR`, not `RV`.** Custody changed; nothing was reversed. |
| E7 | `3:LEG:FN:BL:1:C` | `leg_subtype:JUD`; scope → Aave address | Restrict: the continuing judicial restraint on the new custodian. JUD + Finality=1 permits `BL` under LEG-006. *Alternative if order is still appealable:* `3:LEG:FN:HL:0:C` with `finality_basis:legal_appeal`. |

> **Canonical class names.** The Reading column above uses action-verbs (*Attest, Authorize, Restrict, Execute*) as glosses describing each event's Action semantics — they are **not** class names. The canonical Class vocabulary is fixed by `registries/classes.json` (v1.6.0 naming freeze): Class 1 = OBSERVE, Class 2 = INSTRUCT, Class 3 = CONDITION, Class 4 = CONTROL, Class 5 = TRANSFER, Class 6 = CLOSE. All formal class references in this RFC use those names.

Two structural facts drive Parts A–C:

- **E2 is declared; E3 is derived.** Declared *establishes*; derived *applies*. These are distinct phases and must not carry the same extension block (DPL-003).
- **Neither E1 nor E6 is a reversal.** The remediation is a sequence of forward events linked by `related_events`. `RV` is reserved for Class 1 + State `RV` explicitly undoing a prior FN+1 disposition (SEM-014).

---

## 3. Part A — Derived vs Declared Policy

### 3.0 The declaration boundary (normative)

**Discretion is quarantined in the `declared_policy` extension. The six canonical fields — Class, Origin, State, Action, Finality, Confidence — are always coherence-derived. No canonical field is valid by authority fiat; validity flows from derivation against the protocol rules and registry.**

Two clarifications that follow from this:

**Origins are asserted and validated, not declared.** A message asserts an Origin code; the protocol validates it against `origins.json`. An authority-origin (`origin_type ∈ {governance, emergency_authority, chain_authority}`) does not grant itself standing — the registry confers it. The origin is the *declarer*; it is not itself the *declared thing*.

**Action is always derived; it can never be declared.** An authority may declare a Policy (the governance rule, its scope, and its installation status) via `extensions.declared_policy`. It may not declare an Action. Action is determined by the total field context — Class, the installed rules, the hard-overrides (SEM-016, BRG-001, LEG-007), and the regime ceiling (SEM-015) — and no asserted authority can make an incoherent Action valid. The renaming of field 4 from *Policy* to *Action* (v1.6.0) completes this separation: the word *Policy* now refers exclusively to the governance-rule concept that recognized authorities declare; the derived enforcement field has a distinct name.

| | Declared (by authority) | Derived (by coherence) |
|---|---|---|
| **What** | A Policy — the governance rule, its scope, its installation | Every canonical field, including **Action** |
| **Where** | `extensions.declared_policy` | The Arrow code string + validated extensions |
| **Who** | An authority-origin: {PCY, EMG, CHA, …} | The validation pipeline, for every event without exception |
| **Constraint** | DPL-101/201 + registry-recognized authority | SEM-*, BRG-*, LEG-*, CHN-*, hard-overrides, registries |

The purpose of `extensions.declared_policy` is to carry the *Policy* across the establishment/application boundary without contaminating the six canonical fields. Everything inside that extension block is the declared artifact; everything outside it is derived.

### 3.1 Definitions

**Derived Policy.** Follows deterministically from existing rules, registry vocabulary, and verified state. No discretionary actor is required. Default case; the large majority of transitions.

**Declared Policy.** Cannot be inferred from existing rules and verified state. An authorized actor introduces, modifies, or activates it. A machine must not invent it; the authority must declare it and have it installed at an enforcement point.

### 3.2 The two-step (establishment → application)

A declared Policy operates in two phases, kept strictly separate:

1. **Establishment** — a recognized authority (Part B) asserts a Class 2 (INSTRUCT) or Class 3 (CONDITION) event carrying an `extensions.declared_policy` block, and the rule is installed at an enforcement point.
2. **Application** — once installed, subsequent transitions are evaluated by derivation against the installed rule. Per-transition events reference the establishing event; they do not re-declare.

### 3.3 Rules (`DPL-`)

| Rule | Severity | Constraint |
|---|---|---|
| **DPL-001** | Fatal / Warning | A declared Policy MUST identify: the recognized authority; the authority action reference; the governed scope; the installation target; and the claimed installation status. A declared Policy SHOULD be used only where no registered deterministic derivation rule already determines the applicable Policy. Where an explicit derivation rule already determines the Policy, redundant declaration SHOULD emit a warning; implementations that can prove redundancy MAY reject. |
| **DPL-002** | Fatal / Advisory | A declared Policy MUST carry `installation_status ∈ {declared, installed, active, lapsed}`. Where `installation_status ∈ {installed, active}`, the event SHOULD include `installation_proof_ref`, `installation_verification_regime`, and `installation_confidence`. The level of reliance placed on `installation_status` MUST NOT exceed what the actual observation method can support. |
| **DPL-003** | Fatal | A derived per-transition event applying an installed declared Policy MUST reference the establishing event in `related_events`. The derived event MUST NOT carry an `extensions.declared_policy` block (it is application, not establishment). |
| **DPL-004** | Fatal | If `temporary = true`, `expiry` (RFC 3339) MUST be present. `expiry` defines when a lapse event becomes eligible; it does not itself terminate the Policy. The declared Policy ceases to be operative only when a Class 4 supersession, lapse, or replacement event is appended referencing the establishing event in `related_events`. Arrow Core MUST NOT silently mutate lifecycle disposition based only on wall-clock time. Consumers MAY warn when `expiry` has elapsed without a recorded lapse event. |
| **DPL-005** | Warning | A declared Policy SHOULD be incident-scoped. `scope` SHOULD identify the address, asset, lifecycle, or object class it governs. An unscoped declared Policy SHOULD emit a warning for governance review. |

> Rule numbering: `DPL-1xx` (authority sub-rules) and `DPL-2xx` (schema sub-rules) are sub-ranges of the `DPL` prefix; flatten at promotion if preferred.

---

## 4. Part B — Recognized Authority

### 4.1 Authority is an Origin property, not a new taxonomy

Recognized authority is a classification of an Origin via `origin_type` and `verification_regime`. No parallel taxonomy is introduced.

The `governance` origin_type already exists (assigned to `PCY`). RFC-007 adds one new `origin_type` value:

| `origin_type` | Origin | Regime (ceiling) | Description |
|---|---|---|---|
| `governance` | `PCY` (existing) | `bonded_quorum` (B) | Governance process representing the relevant protocol or chain community. |
| `emergency_authority` | `EMG` (new) | `institutional_attestation` (C) | Actor empowered to take urgent action under a pre-existing emergency mandate. C ceiling correctly encodes that a no-delay unilateral action is less verifiable at decision time than a full governance quorum. |
| `chain_authority` | `CHA` | `institutional_attestation` (C) | Actor with direct technical authority to implement protocol-level changes outside the standard governance vote path (e.g., core developer team, protocol foundation). Distinct from EMG (emergency mandate holder) and PCY (governance quorum). |

The `origin_type` field is a single string (`String`, not `Vec<String>`). Multi-role composition is deferred.

### 4.2 `extensions.declared_policy` schema (DPL-2xx)

The message Origin carries the authority class. The specific authority instance is identified via `authority_ref` and `authority_identity` per RFC-005 OA-006. `authority_origin` (present in v0.1) is removed as redundant.

```json
{
  "extensions": {
    "declared_policy": {
      "authority_ref": "arbitrum-security-council",
      "authority_identity": {
        "controller": "arbitrum-security-council",
        "credential_ref": "registry://authority/arbitrum-security-council",
        "effective_from": "2026-04-21T00:00:00Z",
        "revoked_at": null
      },
      "authority_action_ref": "arb-sc-action-2026-04-21",
      "scope": { "type": "address", "value": "0x0000000000000000000000000000000000000DA0" },
      "temporary": false,
      "expiry": null,
      "installation_target": "governance_executor",
      "installation_status": "active",
      "installation_proof_ref": "arb://governance/action/arb-sc-action-2026-04-21",
      "installation_verification_regime": "institutional_attestation",
      "legal_input_ref": null
    }
  }
}
```

| Rule | Severity | Constraint |
|---|---|---|
| **DPL-101** | Fatal | The message Origin MUST resolve to a registered Origin whose `origin_type ∈ {governance, emergency_authority, chain_authority}`. An Origin without a recognized authority role MUST NOT establish a declared Policy. |
| **DPL-102** | Fatal | The declared event's Confidence MUST NOT exceed the ceiling permitted by the asserting Origin's `verification_regime` (reuses SEM-015). |
| **DPL-201** | Fatal | `extensions.declared_policy` MUST contain: `authority_ref`, `authority_action_ref`, `scope`, `temporary`, `installation_target`, and `installation_status`. `authority_identity` is required for `infrastructure_native` integrations and SHOULD be present for all. |
| **DPL-202** | Fatal | `installation_target` MUST be one of `{bridge_validators, chain_validators, contract_state, governance_executor}`. `installation_status` MUST be one of `{declared, installed, active, lapsed}`. |
| **DPL-203** | Conditional / Fatal | If the declared Policy is grounded in external legal input, `legal_input_ref` MUST reference a `LEG`-origin event in `related_events`. The `LEG` event remains subject to LEG-004–010; this RFC adds no path by which `LEG` executes. |

### 4.3 Installation determinism

> Once a declared Policy is validly installed, every enforcing participant evaluating the same proposed transition under the same active Policy returns the same result.

Determinism holds within the integrated execution path. Reliance on `installation_status` is bounded by the observation method (`DPL-002`). It does not hold against a system that can upgrade around or bypass the integration — which is the subject of Parts C and D.

### 4.4 The authority-registry and credential binding

Authority instances are registered in a centralized authority-registry (`registries/authorities.json`). The `authority_identity.credential_ref` field uses the URI scheme `registry://authority/{authority_id}`, which resolves to an entry in the authority-registry bearing a matching `authority_id` field.

**Authority-registry entry schema:**

```json
{
  "authority_id": "arbitrum-security-council",
  "name": "Arbitrum Security Council",
  "origin": "EMG",
  "origin_type": "emergency_authority",
  "verification_regime": "institutional_attestation",
  "controller": "arbitrum-security-council",
  "description": "...",
  "source_ref": "https://docs.arbitrum.foundation/dao-constitution",
  "effective_from": "2023-09-01",
  "revoked_at": null,
  "added_version": "1.6.0"
}
```

| Field | Requirement | Purpose |
|---|---|---|
| `authority_id` | REQUIRED | Unique identifier; matches the path component in `registry://authority/{id}` |
| `name` | REQUIRED | Human-readable name of the authority |
| `origin` | REQUIRED | Origin code (`EMG`, `PCY`, `CHA`, etc.) this authority maps to |
| `origin_type` | REQUIRED | Mirrors the `origin_type` from `origins.json` for the mapped origin; enables schema consistency checks |
| `verification_regime` | REQUIRED | Mirrors the `verification_regime` from `origins.json`; constrains Confidence via SEM-015 |
| `controller` | REQUIRED | Canonical identifier for the authority actor (e.g., multisig address, DNS name, public key) |
| `description` | REQUIRED | Human-readable description of the authority's mandate and scope |
| `source_ref` | REQUIRED | Primary source establishing the authority's legal, constitutional, or operational mandate |
| `effective_from` | REQUIRED | RFC 3339 timestamp; the date the authority's mandate became effective |
| `revoked_at` | REQUIRED (may be null) | RFC 3339 timestamp or null; when the authority was revoked (if applicable) |
| `added_version` | REQUIRED | Protocol version in which this entry was registered |

The authority-registry is maintained under the same governance model as other protocol registries (RFC-006 §5). A declared Policy event MUST resolve its `authority_ref` to a non-revoked authority entry before that event can be accepted.

---

## 5. Part C — Integration Strength

### 5.1 The three tiers

| Tier | Arrow's role | Can it block execution? |
|---|---|---|
| `observational` | Receives event data; classifies or records. | **No.** Records `BL`/`HL` as observed classification only. |
| `enforcing` | Validates a proposed transition; the connected infrastructure executes or rejects on Arrow's result. | **Yes, within the integrated path.** |
| `infrastructure_native` | Declared/derived Policy is installed directly into bridge validators, chain validators, contract logic, or a governance executor. **Arrow Validators** are this tier. | **Yes, and more deeply** — but still subject to the underlying upgrade model. |

### 5.2 Placement

`integration_strength` is declared on the **Origin Adapter record, Consumer Adapter record, or deployment profile** — not on generic `origins.json` entries. The same Origin may be used at different integration strengths across deployments:

```text
ARB via third-party RPC adapter    → observational
ARB via proof-verifying adapter    → enforcing
ARB via chain-native module        → infrastructure_native
```

### 5.3 Rules (`INT-`)

| Rule | Severity | Constraint |
|---|---|---|
| **INT-001** | Fatal | Each Arrow integration MUST declare `integration_strength ∈ {observational, enforcing, infrastructure_native}` on its Origin Adapter record, Consumer Adapter record, or deployment profile. Generic `origins.json` entries do not carry this field. |
| **INT-002** | Fatal | An `observational` integration MUST mark events it emits with `extensions.integration.enforced = false`. It MAY record Action `BL`/`HL` as observed classification but MUST NOT represent that a Block was actually applied at an enforcement point. Only `enforcing` and `infrastructure_native` integrations may assert `enforced = true`. |
| **INT-003** | Fatal | An `infrastructure_native` integration MUST disclose whether the underlying system retains an upgrade path capable of removing, replacing, or bypassing Arrow enforcement. If this disclosure is absent, consumers MUST treat the integration as undisclosed with respect to bypass risk. Absence of disclosure MUST NOT be interpreted as absence of capability. |

### 5.4 The hard limit (normative disclosure)

> Arrow enforces deterministic semantic rules only within the execution path in which it is integrated. It cannot guarantee compliance where a connected system retains the capacity to upgrade around, remove, or bypass the integration.

This is a trust assumption inherent in upgradeable systems. RFC-007 requires it to be **disclosed**, not solved. Reliance on `integration_strength` claims is bounded by the observation method of the adapter asserting them.

---

## 6. Part D — Capability Disclosure (the Arbitrum lesson, absorbed as metadata)

### 6.1 Why no `OR` primitive

Detecting arbitrary future override functions would require Arrow to monitor proxy upgrades, deployed bytecode, function additions, sender semantics, and restoration steps. That contradicts the Non-Goals and is not viable. The capability is recorded as disclosure so consumers can price the latent risk.

### 6.2 `governance_capability` advisory sub-block

Added to the Chain Registry v2.0 `advisory` block. Disclosure-only; CHN-006 forbids any effect on derived Actions or canonical fields.

Two capability fields use a structured `CapabilityDisclosure` sub-object rather than a plain boolean, carrying evidence provenance aligned with Arrow provenance discipline. Disclosure status values: `documented`, `demonstrated`, `inferred`, `unknown`.

```json
{
  "advisory": {
    "governance_capability": {
      "emergency_upgrade_authority": true,
      "multisig_threshold": "9-of-12",
      "upgrade_delay_seconds": 0,
      "emergency_upgrade_exit_window_seconds": 0,
      "historical_compelled_transfer": {
        "status": "demonstrated",
        "source_ref": "https://forum.arbitrum.foundation/t/security-council-emergency-action-21-04-2026/30803",
        "observed_at": "2026-04-21"
      },
      "sender_impersonation_via_upgrade": {
        "status": "demonstrated",
        "source_ref": "https://forum.arbitrum.foundation/t/security-council-emergency-action-21-04-2026/30803",
        "observed_at": "2026-04-21"
      }
    }
  }
}
```

Field notes: `sender_impersonation_via_upgrade` (structured) records the `sendUnsignedTransactionOverride`-class capability demonstrated on 21/04/2026. `emergency_upgrade_exit_window_seconds: 0` records no user exit window before emergency action. `historical_compelled_transfer` with `status: demonstrated` is the strongest disclosure signal — it records that a compelled transfer has actually occurred, not merely that the capability exists.

### 6.3 Rules (`CHN-005` onward)

| Rule | Severity | Constraint |
|---|---|---|
| **CHN-005** | Optional | A Chain Registry `advisory` block MAY carry a `governance_capability` sub-block with the fields defined above. The Bridge Registry MAY carry an analogous block. |
| **CHN-006** | Fatal (negative) | `governance_capability` fields are **advisory**. They MUST NOT force, alter, or override any derived Action or canonical field. Arrow MUST NOT attempt runtime detection of override functions. |
| **CHN-007** | Advisory | A chain whose `governance_capability` is absent is **undisclosed**, not **safe**. Consumers SHOULD treat undisclosed governance capability as a trust risk. |

> **PRV-003 coherence.** `governance_capability` is advisory and is never used to derive `finality_basis`, `finality_eta`, or `chain_trust_flag`; PRV-003 does not apply unless a future version promotes any field to `protocol_required`.

---

## 7. Part E — RV and LEGAL: closed by cross-reference

RFC-007 introduces **no new rules** for reversal or for legal-input semantics.

### 7.1 RV

`RV` is settled by SEM-001 and SEM-014. In the reference case, E1 and E6 are forward Class 5 events, not reversals. `RV` is reserved for a Class 1 event, State `RV`, that explicitly asserts a prior FN+1 disposition is undone.

### 7.2 LEGAL non-executable

| Working-notes field | Canonical home |
|---|---|
| `source_verified` | Confidence (legal_attestation regime; ceiling C single authority / B panel) |
| `interpretation_status` | `leg_subtype` (JUD/ADM/SUPRA/LEG_TEXT/PREC) + LEGAL_SYSTEM "classify, never determine" |
| `applicability_status` | `leg_jurisdiction` + LEGAL_SYSTEM recognition_status |
| *operational adoption* | `extensions.declared_policy.installation_status` (Part B) — the only genuinely missing field; RFC-007 supplies it. |

The safe structure: `LEG` (evidence) → declared Policy by a recognized authority (Part B) → proposed transition → validation → execution if permitted. `LEGAL` does not activate `RV` and does not execute a transaction.

LEG rule reference: **LEG-006** is the Policy Derivation Gate; **LEG-007** is SUPRA Sanctions Force-BL.

---

## 8. Conformance Impact

| Change | Type | Conformance impact |
|---|---|---|
| Canonical six-field message string | — | **Unchanged.** |
| `origins.json`: add `EMG` (`emergency_authority`, `institutional_attestation`) | Additive | None. `governance` origin_type pre-existed on `PCY`; not a new type. |
| Chain Registry `advisory.governance_capability` | Additive | None — advisory only; CHN-006 forbids policy effect. |
| `extensions.declared_policy` block | Additive | None — new optional extension. |
| `extensions.integration` (`enforced`) | Additive | None — extends RFC-005 Origin Adapter record. |
| `integration_strength` | Adapter/deployment record, not `origins.json` | None to Origins parser. |
| `DPL-`, `INT-`, `CHN-005–007` rules | Additive — deferred from pipeline | None until promoted. |
| Reversal / legal semantics | — | No change. Inherited from SEM-001, SEM-014, LEG-004–010, RFC-004. |

**Net:** additive, non-breaking. No deletions, no message-format change, no change to existing validation outcomes.

---

## 9. Deferred from Phase 1 implementation

Do not wire into `arrow-core` in the first pass:

```text
DPL fatal rules
INT fatal rules
automatic installation-status reliance
wall-clock expiry enforcement (DPL-004 lapse is append-only by design)
cross-authority precedence
adapter-tier capping
runtime detection of override functions
```

These require a mature adapter interchange contract and reduction layer.

---

## 10. Phase 1 — Class 3 / SEM-006 mismatch (RESOLVED)

**Specification (SEM-006):** Class 3 (CONDITION) MUST use Action `BL`, `HL`, or `RW`. No `Finality` constraint.

**Resolution (Option A, applied in v1.5.2-draft, commit `200633e`):** The Rust pipeline previously enforced an additional `Class 3 → Finality = 0` constraint the specification does not require. That constraint was removed; `validate_semantic` now checks only that a Class 3 event uses Action `BL`/`HL`/`RW`. Regression tests `sem006_accepts_class3_non_executing_policies` (incl. `3:ARB:FN:BL:1:A`) and `sem006_rejects_class3_executing_policies` lock the corrected behavior.

A final restriction is conceptually valid — a binding judicial restraint, final sanctions block, or irreversible protocol restriction is no longer forced into `Finality = 0` solely because it is restrictive. E2 (`3:EMG:FN:HL:1:C`) and E3 (`3:ARB:FN:BL:1:B`) are valid under the corrected pipeline (E2 requires the `EMG` origin, added in v1.5.2-draft).

Phase 1 was the prerequisite for finalizing RFC-007 fixtures and promoting DPL rules to fatal; that prerequisite is now discharged. DPL/INT promotion remains gated on the adapter interchange contract and reduction layer (§9), not on Phase 1.

---

## 11. Open Questions (§13)

1. Should Class 3 permit `Finality=1`? **RESOLVED — yes (Option A applied in v1.5.2-draft, commit `200633e`).**
2. Must every blocked outbound attempt mint an Arrow event, or may the enforcement point reject execution while optionally emitting an audit event? **RESOLVED — emission is optional.** The enforcement point may reject execution while optionally emitting an audit event; Arrow Core does not require a per-block-attempt event. E3 (`3:ARB:FN:BL:1:B`) illustrates a derived application event referencing an installed declared Policy (DPL-003); it is an example of what such an event looks like, not a mandate that every enforcement outcome must produce one. The establishing declared Policy event (E2) is required; downstream per-attempt events are audit artifacts, not protocol-mandated.
3. Is `PR` or `TR` the correct Action for the Security Council compelled transfer (E1)? **RESOLVED — `PR`.** `TR` (Trigger) asserts a downstream conditional workflow; it is never a generic Proceed and never itself proof of transfer (`actions.json` registry note). E1 is an executed compelled transfer — Class 5 (TRANSFER) + Action `PR` correctly encodes that this forward execution occurred. `TR` would misrepresent E1 as triggering something further rather than asserting the execution itself. Same reasoning holds for E6.
4. What proof is sufficient for `installation_status = active`? Is a governance action reference adequate, or is a verifiable on-chain hook required? *(Open — enforcement-layer; gated on adapter interchange contract.)*
5. Does `infrastructure_native` require cryptographically verifiable installation evidence? *(Open — enforcement-layer; same gate.)*
6. Should `governance_capability` use a generic provenance object shared across registries (CHN, BRG, bridge)? *(Open — cross-registry refactor; non-blocking.)*
7. Does `EMG` need a confidence subprofile in `confidence_subprofiles.json`? *(Open — EMG ceiling is already bounded by `institutional_attestation` (C) via SEM-015; a subprofile is additive polish, not required until EMG sub-categorization is needed.)*
8. When should `CHA` (`chain_authority`) be introduced, if ever? **RESOLVED — introduced in v1.6.0.** `CHA` is now registered in `origins.json` with `origin_type: chain_authority` and `verification_regime: institutional_attestation`. The authority-registry (§4.4) supports CHA entries. First concrete CHA authority registration awaits a deployment scenario requiring chain-level protocol-change authority, but the origin and mechanism are now in place.
9. Should a temporary declared Policy lapse require an explicit Class 4 supersession event, or is a consumer warning on elapsed `expiry` sufficient? **RESOLVED — Class 4 append required; consumer warning is advisory.** DPL-004 already answers this: the declared Policy ceases to be operative only when a Class 4 supersession, lapse, or replacement event is appended referencing the establishing event. Arrow Core MUST NOT silently terminate based on wall-clock time alone. Consumer MAY warn on elapsed `expiry` — that warning is an advisory prompt to append the lapse event, not a substitute for it.
10. Which parts of RFC-007 belong in a later Arrow Validators RFC, and which are protocol-core? **RESOLVED.** Protocol-core (stays in RFC-007): the declaration boundary and DPL-* rules (§3); the integration strength taxonomy and INT-* rules (§5); capability disclosure and CHN-005–007 (§6); the authority-origin model (§4). A future Arrow Validators RFC owns the first concrete instantiation of `integration_strength = infrastructure_native`: how Validators are deployed, their interface contract, `infrastructure_native` installation verification mechanics, and the adapter interchange contract that gates DPL-*/INT-* promotion to the live pipeline (§9). §1.2's statement that "Arrow Validators are the first concrete instantiation of `integration_strength = infrastructure_native`" marks the handoff point.

---

## 12. Reference List

Protocol:

1. Arrow Protocol v1.5-draft — `docs/protocol/arrow-protocol-v1.5-draft.md`
2. Validation Rules v1.6.0 — `docs/protocol/validation-rules-v1.6.0.md` (SEM-001, SEM-006, SEM-013, SEM-014, SEM-015, SEM-016, SEM-020; LEG-001, LEG-004–010; BRG-001; FB-001; CHN-001–004, CHN-008; ACT-001, ACT-002; NAM-001)
3. RFC-004 — Coherence and Minimization (FN+0; State × Finality Matrix; `finality_basis`)
4. RFC-005 — Provenance and Trustless Sourcing (Origin Adapter identity binding; OA-006; PRV-003)
5. RFC-001 — KelpDAO Reference Case (prior reference case)
6. `registries/chain_registry.json` v2.0; `registries/finality_basis.json`; `registries/origins.json`
7. `docs/architecture/drafts/legal-system-primitive-v0.1.md` (LEGAL_SYSTEM, deferred)

Reference-case primary sources:

8. Arbitrum Forum — Security Council Emergency Action, 21/04/2026 — forum.arbitrum.foundation/t/security-council-emergency-action-21-04-2026/30803
9. Arbitrum DAO Constitution — docs.arbitrum.foundation/dao-constitution
10. Arbitrum Forum — [Constitutional] AIP: Approve Release of Frozen ETH (and amended discussion) — forum.arbitrum.foundation/t/constitutional-aip-approve-release-of-frozen-eth/30825

---

## 13. Status Note

This RFC (published v1.0, 2026-06-17) incorporates all corrections from *RFC-007 Suggested Corrections — Editing Checklist v0.1* and the 2026-06-17 graduation-readiness pass. It is not a legal opinion and not an assertion that Arrow could have prevented or detected the Arbitrum Security Council action. The published specification is normative for its landed surface; the `DPL-*`/`INT-*` enforcement rules are not yet an implementation specification (see below).

The Phase 1 prerequisite (Class 3 / SEM-006 resolution) is **complete** (v1.5.2-draft, commit `200633e`), and the additive registry metadata (`EMG` and `CHA` origins, ARB `governance_capability`, `authorities.json`) has landed, with reference fixtures `E-02`/`E-03` committed. **RFC-006 graduated 2026-06-16** (published v1.0 2026-06-17), lifting the §1 dependency hold. The specification is now **published**: `DPL-*`/`INT-*` promotion to the live pipeline remains deferred by design behind the adapter interchange contract and reduction layer (Parts A/C), while Parts B (recognized authority) and D (capability disclosure) are normative. The field-4 `Policy`→`Action` terminology sweep is complete (2026-06-17 pass); residual uses of "Policy" in this document refer to the governance-rule concept (Declared Policy, derived Policy) and are correct under the declaration-boundary invariant established in §3.0. One advisory item remains for whenever the DPL rules promote: the composite rule severities ("Fatal / Warning", "Fatal / Advisory", "Conditional / Fatal") must collapse to single grades (readiness item B-8).
