# Arrow Protocol Validation Rules v1.6.0

**Version:** 1.6.0
**Status:** live
**Date:** 2026-06-10
**Depends on:** Arrow Protocol v1.6.0, Arrow Registries v1.6.0, RFC-003 v1.0, RFC-004 v1.0, RFC-005 v1.1-draft, RFC-006 v0.5 draft, Arrow RFC-008 v0.3
**Supersedes:** Validation Rules v1.5.2-draft (2026-06-07)
**Scope:** Message Standard + Event Object + Chain Registry + Bridge Registry + Multi-Source Observation + Finality Basis + LEG Action Derivation

> **v1.6.0 change — Action field, Class 4 CONTROL, SEM-020, naming freeze.** Canonical field 4 is **`Action`** — final. Codes `PR`/`BL`/`HL`/`RW`/`TR`/`CL` and their semantics are unchanged; the registry is `actions.json`; field-4 rule identifiers are `ACT-001`/`ACT-002`. Class 4 is `CONTROL` (control-plane event class, anchored by draft `control_subtypes.json`; `SEM-021` reserved for its promotion). New rule **SEM-020** makes the Class 6 ⇔ Action CL binding bidirectional. `Action` (field 4), `CONTROL` (Class 4), and Origin `PCY` are three distinct namespaces; the NAM-001 registry lint enforces that no name or code appears in more than one. Prose in historical sections below is modernized to the current field name; rule-ID history is preserved accurately.

---

# Purpose

Defines semantic and structural validation requirements for Arrow events.

These rules define how conforming Arrow implementations MUST validate:

- Canonical Arrow message strings
- Arrow Event Objects
- Lifecycle transitions
- Registry references
- Confidence and verification regime consistency
- Chain trust state
- Bridge trust state
- Proof classification binding
- Economic consistency binding
- Divergence observation events
- Multi-source composition patterns

Validation rules establish deterministic rejection behavior and semantic consistency across Arrow-compatible systems.

---

# Validation Tiers

| Tier | Meaning |
|---|---|
| MUST | Hard constraint. Violations render the event invalid. Receiving systems MUST reject. |
| SHOULD | Strong recommendation. Deviation permitted only with documented justification. |
| MAY | Optional behavior. Implementations may include or omit. |

---

# What's New in v1.6.0

v1.6.0 is the **namespace-decoupling completion release**. The canonical six-field message string is unchanged at the wire layer (it never carried a field label). All v1.5.2 semantic obligations are preserved; one additive semantic rule lands.

- **Field 4 = `Action` (final).** Registry `actions.json`; rules `ACT-001`/`ACT-002`. The name is frozen.
- **Class 4 = `CONTROL`**: the control-plane event class — registry updates, chain/bridge status transitions, protocol upgrades, validator/node (DVN/OVN) topology changes, parameter changes, emergency declarations, supersession. Anchored by draft `control_subtypes.json`; **`SEM-021` reserved** for promoting `control_subtype` to a mandatory, anchor-checked field when that vocabulary graduates to active.
- **SEM-020 added** — Action `CL` MUST use Class 6 (converse of SEM-005). The only valid closure shape is `6:*:FN:CL:1:*`.
- **LFC-007 live (RFC-010)** — post-closure lifecycle discipline: see the Lifecycle Rules table. Consumer-side lifecycle-head checks are Tarquin LC-013/LC-015.
- **NAM-001 naming-integrity lint** added to `tools/validate-registries.js`: no code or display name may appear in more than one canonical six-field namespace. CI-fatal.
- **Rule-ID permanence** — from v1.6.0, rule identifiers are retired, never renumbered, never reused. Cross-repo artifacts SHOULD pin `(rule_id, semantic_slug)` pairs.
- LEG range violations use reason code `LEG_ACTION_RANGE_VIOLATION`.

# What's New in v1.5.1-draft

v1.5.1-draft is a **numbering reconciliation and LEG completion release**. The canonical six-field message string is unchanged. All semantic obligations from v1.5-draft are preserved; the rule identifiers carrying some of them have changed.

**Reconciliation policy:** Numbers retired or reclassified in v1.5 are reused where a thematically adjacent v1.5 rule occupied the next slot. Higher-numbered stable rules are NOT renumbered to fill gaps. Renumbering is contained to the LEG family.

- **SEM-013 reactivated** as a new protocol-wide rule: **FN+0 Executing-Action Ceiling**. Origin-agnostic. Formalizes the "Finality = 0 ⇒ non-executing Action" invariant promoted by RFC-004 §9.3. (The retired-superseded note for SEM-013 is removed; SEM-014 still subsumes the original SEM-013 reversal-reference requirement.)
- **LEG family compaction.** Old LEG-006 ("`leg_subtype` must be one of five values") was reclassified as a schema constraint in RFC-004 §10 — it now lives at MSG-006. Old LEG-007/008/009/010/011 each shift down by one to fill the freed slot:
  - **LEG-006** ← Action Derivation Gate (was LEG-007 in v1.5)
  - **LEG-007** ← SUPRA Sanctions Force-BL (was LEG-008 in v1.5)
  - **LEG-008** ← LEG_TEXT/PREC Finality=1 Warning (was LEG-009 in v1.5)
  - **LEG-009** ← `leg_jurisdiction` Required (was LEG-010 in v1.5)
  - **LEG-010** ← `supra` Registry Membership (was LEG-011 in v1.5)
- **MSG-006 added** — `leg_subtype` registry membership when `origin = LEG`. Schema constraint, not semantic.
- **LEG-007 (new) and LEG-006 (new) fully implemented** in `arrow-core`. The minimal first pass in v1.5 (LEG-004, MSG-006-equivalent only) is replaced by the complete derivation table from RFC-004 §9.3.
- **SEM-018** remains a permanent reclassified gap (MSG-005 in v1.5).
- **LFC-006** remains retired.
- **LEG-012/013/014** remain retired/extension slots.

# What's New in v1.5-draft

- FB-001 — Finality Basis Requirement for all Finality=0 events
- LEG Action Derivation Gate — LEG-004 through LEG-013, with LEG-014 retired (rules later compacted in v1.5.1)
- MSG-005 — Classification Binding Method Registry Membership (reclassified from SEM-018)
- Chain Registry v2.0 references use `event.chain_id` as the Chain Registry key; chain identifiers are distinct from canonical Origin codes

# What's New in v1.4

- SEM-017 — Proof Classification Binding
- Classification binding method registry membership (now MSG-005 in v1.5)
- SEM-019 — Economic Consistency Binding
- BRG-000 to BRG-006 — Bridge Registry rule family
- New error codes: SEM-017, MSG-005, SEM-019, BRG-000, BRG-001, BRG-005
- New rejection behaviors: bridge trust revocation, proof type unbound, economic consistency unbound, bridge restoration blocked
- Updated validation pipeline: Steps 3a–3b (Bridge Registry), Steps 6–9 (SEM-017/MSG-005/SEM-019, BRG-003/006)

# What's New in v1.3

- SEM-015 — Confidence / Verification Regime Consistency
- SEM-016 — Chain Trust Flag Override
- CHN-001 to CHN-004, CHN-008 — Chain Registry Validation
- DIV-001 to DIV-004 — Divergence Event Validation
- LEG-001 — PROTOCOL Jurisdiction Resolution
- Verification regime-aware confidence ceilings
- Multi-source divergence handling
- Chain trust revocation semantics

---

# Scope

These rules apply to:

- Canonical Arrow message strings
- Arrow Event Object schemas
- Lifecycle state transitions
- Chain Registry entries
- Bridge Registry entries
- Bridge-origin event validation
- Multi-source observation systems
- Divergence detection workflows
- Verification regime composition patterns

These rules do NOT govern:

- Transport semantics
- Delivery acknowledgments
- Message routing
- Network-layer trust assumptions
- Deployment-specific policy thresholds

Transport semantics are governed by the Arrow Transport Model v1.0 (`archive/transport/arrow-transport-model-v1.0.md`).

---

# Core Validation Categories

## Structural Validation

Ensures events conform to protocol formatting and schema requirements.

Includes: required fields, type validation, schema conformance, canonical message formatting, registry vocabulary enforcement, timestamp validation, event reference integrity.

## Semantic Validation

Ensures Arrow events represent logically coherent protocol states.

Includes: state transitions, action enforcement, finality constraints, confidence ceilings, verification regime consistency, bridge trust consistency, proof classification binding, economic consistency binding, lifecycle integrity, reversal semantics.

## Registry Validation

Ensures all referenced protocol vocabularies, chain metadata, and bridge metadata are authoritative and consistent.

Includes: origin registry checks, reason code validation, confidence verification, chain registry validation, bridge registry validation, trust flag enforcement, subprofile validation, registry version resolution.

---

# Message String Validation

## Canonical Message Structure

```text
Class:Origin:State:Action:Finality:Confidence
```

Example: `2:CST:FN:PR:1:C`

## Message Validation Rules

| Rule ID | Requirement |
|---|---|
| MSG-001 | MUST contain exactly 6 fields separated by 5 colons. |
| MSG-002 | MUST NOT contain whitespace. |
| MSG-003 | Fields MUST appear in canonical order; registered values are case-sensitive. |
| MSG-004 | Canonical six-field values MUST exist in registered vocabularies. |
| MSG-005 | `proof_semantics.classification_binding_method` MUST reference `classification_binding_methods.json` when present. *(Reclassified from SEM-018 in v1.5.)* |
| MSG-006 | When `origin = LEG`, `leg_subtype` MUST be present and MUST be a registered value in `{JUD, ADM, SUPRA, LEG_TEXT, PREC}`. *(Reclassified from LEG-006 in v1.5.1; schema constraint, not semantic.)* |

---

# Field-Specific Validation

## Class Rules

| Rule ID | Requirement |
|---|---|
| CLS-001 | Class MUST map to registered class definitions. |
| CLS-002 | Class values MUST be integers 1–6. |
| CLS-003 | Unknown Class values MUST be rejected. |

## Origin Rules

| Rule ID | Requirement |
|---|---|
| ORG-001 | Origin MUST be a registered 3-character uppercase code. |
| ORG-002 | Origin MUST resolve to a valid registry entry. |
| ORG-003 | Unknown Origin values MUST be rejected. |

## State Rules

| Rule ID | Requirement |
|---|---|
| STA-001 | State MUST exist in the State Registry. |
| STA-002 | State values MUST be uppercase 2-character codes. |

## Action Rules

| Rule ID | Requirement |
|---|---|
| ACT-001 | Action MUST exist in the Action Registry. |
| ACT-002 | Unknown Action values MUST be rejected. |

## Finality Rules

| Rule ID | Requirement |
|---|---|
| FIN-001 | Finality MUST be binary 0 or 1. |
| FIN-002 | Invalid Finality values MUST be rejected. |

## Confidence Rules

| Rule ID | Requirement |
|---|---|
| CON-001 | Confidence MUST be A–E. |
| CON-002 | Invalid Confidence values MUST be rejected. |

---

# Semantic Constraints

| Rule ID | Constraint |
|---|---|
| SEM-001 | If Finality = 1, State MUST be FN or RV. |
| SEM-002 | If State = PD, Finality MUST be 0. |
| SEM-003 | If State = TL, Finality MUST be 0. |
| SEM-004 | If State = CD, Finality MUST be 0. |
| SEM-005 | Class 6 (CLOSE) MUST use Action CL. *(Bidirectional with SEM-020 from v1.6.)* |
| SEM-006 | Class 3 (CONDITION) MUST use BL, HL, or RW. |
| SEM-007 | Class 5 (TRANSFER) MUST use TR or PR. |
| SEM-008 | Class 2 (INSTRUCT) SHOULD use PR. |
| SEM-009 | Confidence E MUST NOT pair with PR or TR. |
| SEM-010 | Confidence E with BL or HL is permitted. |
| SEM-011 | Implementations SHOULD define minimum acceptable confidence thresholds above E for consequential actions. |
| SEM-012 | Class 6 MUST have State FN. |
| SEM-013 | **FN+0 Executing-Action Ceiling.** If `Finality = 0`, Action MUST NOT be an executing Action. Executing actions are `{PR, TR, CL}`. Permitted Action when `Finality = 0` is the non-executing set `{BL, HL, RW}`, subject to further Origin-, Class-, and subtype-level constraints. Origin-agnostic. *(Slot reused; the original SEM-013 reversal-reference rule remains superseded by SEM-014.)* |
| SEM-014 | Final events (Finality=1) MUST NOT be contradicted except through explicit reversal (Class 1, State RV) or supersession (Class 4, CONTROL). Both MUST reference the prior event in `related_events`. |
| SEM-020 | **Closure Class Binding (v1.6).** If Action = CL, Class MUST be 6 (CLOSE). Converse of SEM-005. With SEM-005/SEM-012/SEM-013, the only valid closure shape is `6:*:FN:CL:1:*`. |

---

# SEM-013 — FN+0 Executing-Action Ceiling (v1.5.1)

**Position in validation order:** Step 5 (semantic constraints), evaluated alongside SEM-001–014.

**Constraint:** Any event with `Finality = 0` MUST NOT carry an executing Action. The executing Action set is `{PR, TR, CL}`. The permitted set when `Finality = 0` is `{BL, HL, RW}`.

**Rationale:** With `FN+0` formally defined in v1.5 (Part 7 of the protocol spec) as "settled but challengeable," and with `PD`, `CD`, and `TL` all confined to `Finality = 0` by SEM-002/SEM-003/SEM-004, no event with `Finality = 0` may carry an executing Action. This rule formalizes a protocol-wide invariant that was previously expressed only via subtype-specific tables (notably LEG-007 in v1.5). Lifting it to a SEM-* rule eliminates per-Origin re-encoding and makes the invariant uniform across all Origins.

**Interaction with Origin-specific rules:**

- LEG-006 (Action Derivation Gate) specifies the `Finality = 1` permitted set per `leg_subtype`. The `Finality = 0` row is fully governed by SEM-013 — LEG-006 does not re-enumerate it.
- SEM-006 (Class 3 ⇒ BL/HL/RW) remains valid and is internally consistent with SEM-013: Class 3 is non-executing by definition.
- SEM-007 (Class 5 ⇒ TR or PR) implies Class 5 events MUST carry Finality = 1 — a Class 5 event with Finality = 0 is rejected by SEM-013 (executing Action required by SEM-007; forbidden by SEM-013).

**Rejection:** Violations are rejected with rule code `SEM-013` and a message identifying the (Action, Finality) pair.

**Examples:**

```text
Invalid:  2:CST:FN:PR:0:C   — PR with Finality=0 → SEM-013
Invalid:  6:LEG:FN:CL:0:C   — CL with Finality=0 → SEM-013 (also SEM-005/SEM-012 may fire)
Invalid:  5:ARB:FN:TR:0:A   — TR with Finality=0 → SEM-013
Valid:    3:LEG:FN:HL:0:C   — HL is permitted at Finality=0
Valid:    1:ETH:FN:RW:0:C   — RW is permitted at Finality=0
Valid:    4:PCY:FN:BL:0:C   — BL is permitted at Finality=0
```

---

# SEM-020 — Closure Class Binding (v1.6.0)

**Position in validation order:** Step 5 (semantic constraints), evaluated alongside SEM-001–014.

**Constraint:** Any event with Action = `CL` MUST have Class = 6 (CLOSE). This is the converse of SEM-005 (Class 6 MUST use Action CL). Together with SEM-012 (Class 6 ⇒ State FN) and SEM-013 (executing Action ⇒ Finality 1), the only valid closure shape is `6:*:FN:CL:1:*`.

**Rationale:** Class 6 and Action CL are a **redundant-and-checked** pair: two independent fields must agree before any consumer treats an event as lifecycle-terminal. A single-field corruption, truncation, or type confusion (bridge, hand-rolled parser, hostile relayer) is therefore detected instead of silently changing lifecycle semantics. Before SEM-020 the check was asymmetric: SEM-005 protected consumers keying off Class, but a consumer keying off the Action field could be fed `1:ETH:FN:CL:1:A` — an observation event wearing a terminal Action — and stop monitoring a live lifecycle.

**Reopening discipline:** SEM-020 makes reopening loud, not impossible. After closure, the only valid follow-ups are the two SEM-014 paths — Class 1 State RV reversal, or Class 4 (CONTROL, subtype `SUPERSESSION`) supersession — each referencing the CL event in `related_events`. Rule slot `LFC-007` is reserved for the full post-closure lifecycle rule.

**Rejection:** Violations are rejected with rule code `SEM-020` and a message identifying the (Class, Action) pair.

**Examples:**

```text
Invalid:  1:ETH:FN:CL:1:A   — CL outside Class 6 → SEM-020
Invalid:  2:LEG:FN:CL:1:C   — CL outside Class 6 → SEM-020
Valid:    6:ARB:FN:CL:1:A   — canonical closure shape
```

---

# Verification Regime Consistency

## SEM-015 — Confidence / Verification Regime Consistency

**Position in validation order:** Step 10 (after SEM-017, MSG-005, SEM-019, BRG-003/006).

Confidence claims MUST NOT exceed the maximum confidence ceiling permitted by the declared verification regime.

Lower confidence claims are always valid.

**Maximum Confidence by Regime:**

| Verification Regime | Maximum Confidence |
|---|---|
| `native_canonical` | A |
| `optimistic_settled` | A post-window (Finality=1) / B pre-window (Finality=0) |
| `bonded_quorum` | B |
| `native_notarized_consensus` | B |
| `institutional_attestation` | C |
| `legal_attestation` | C |
| `structured_legal_attestation` | B |
| `multiple_legal_attestation` | B |
| `legal_native_attestation` | A draft/reserved only; no current Origin depends on this regime |

**Conditional (pre-settlement) ceiling.** A regime whose full ceiling is earned
only after a settlement window elapses declares a second, lower ceiling in its
registry entry via `pre_settlement_max_confidence` (`optimistic_settled` sets
`"B"`; all other current regimes omit the field, meaning their flat
`max_confidence` applies at any Finality). Enforcement is deterministic and
event-carried — validation remains `V(event, registry_snapshot)` with no
ambient clock:

1. **Pre-window (message level).** If `Finality = 0` and the regime declares
   `pre_settlement_max_confidence`, that value is the effective SEM-015
   ceiling. The Finality bit itself is the window discriminator: a Finality-0
   event under such a regime is by definition still inside its settlement
   window, and FB-001 independently forces it to declare
   `finality_basis = optimistic_window` with a dated `finality_eta`
   (`requires_finality_eta`). → Confidence above the pre-settlement ceiling
   REJECTS with `SEM-015`.
2. **Post-window (event level).** If `Finality = 1` and the claimed Confidence
   exceeds the pre-settlement ceiling, the event asserts its window has
   closed. When the event itself dates that close (`finality_eta` present with
   `finality_eta_type = "unix_ts"`), the event's own `timestamp` MUST NOT
   precede `finality_eta`. → An event claiming settled Confidence while dating
   its window close in its own future REJECTS with `SEM-015`. A `timestamp`
   that is not valid RFC 3339 likewise REJECTS with `SEM-015` when this check
   is reached — malformed input rejects, it never disarms the check.

Producer note: the Chain Registry's `challenge_window_seconds` is the
producer-side derivation aid for `finality_eta` (inclusion-block timestamp +
window). An optimistic-L2 origin adapter that cannot date the window MUST fail
closed rather than emit an undated pending event (FB-001).

**Invalid examples:**

```text
2:CST:FN:PR:1:A    — CST is institutional_attestation; ceiling is C
1:PCY:FN:PR:1:A    — PCY is bonded_quorum; ceiling is B
1:ARB:PD:HL:0:A    — ARB is optimistic_settled at Finality=0; pre-settlement ceiling is B
1:ARB:FN:HL:1:A with finality_eta after its own timestamp — settled claim contradicted by event-carried window close
```

**Valid examples:**

```text
1:BTC:FN:PR:1:A    — BTC is native_canonical; ceiling is A
2:CST:FN:PR:1:C    — CST is institutional_attestation; C is at ceiling
2:LEG:FN:PR:1:C    — LEG is legal_attestation; C is at ceiling
2:PCY:FN:PR:1:B    — PCY is bonded_quorum; B is at ceiling
1:ARB:PD:HL:0:B    — ARB pending at the pre-settlement ceiling B
1:ARB:FN:HL:1:A    — ARB settled (Finality=1, window close not in the event's future); full ceiling A
```

For LEG-origin events, a court, arbitration, or appellate panel is a
`legal_authority_panel` fact on the LEG authority/finality axis. It does not by
itself raise Confidence. Confidence B requires the event's proof path to satisfy
`structured_legal_attestation` or `multiple_legal_attestation`; B remains a
ceiling, not a default.

---

# Chain Trust Validation

## SEM-016 — Chain Trust Flag Override

**Position in validation order:** Step 3 (immediately after Origin resolution).

If `chain_trust_flag == 0` in the Chain Registry entry for the event's Origin:

- Action MUST be BL.
- All events claiming PR, TR, HL, RW, or CL on an untrusted chain MUST be rejected.
- This rule takes precedence over all other semantic constraints.
- This rule is **non-overridable** at the deployment policy layer.

---

# Finality Basis Rules

## FB-001 — Finality Basis Required for Non-Final Events

**Position in validation order:** Step 5a (after SEM-001–014 and SEM-013; before bridge rules).

**Constraint:** If `Finality = 0`, the field `finality_basis` MUST be present and MUST contain a value registered in `registries/finality_basis.json`.

- If `finality_basis` is absent and `Finality = 0` → REJECT with rule code `FB-001`.
- If `finality_basis` is present but not a recognized code in the registry → REJECT with rule code `FB-001`.
- If `finality_basis` is present but `Finality ≠ 0` → the field is accepted but carries no validation obligation (no error).
- If `finality_basis` entry specifies `requires_finality_eta: true`, the `finality_eta` field MUST also be present. If it is absent → REJECT with rule code `FB-001` (an undated settlement window carries no reproducible evidence of when settlement is due).

**Registered values (see `registries/finality_basis.json`):** `optimistic_window`, `probabilistic`, `legal_appeal`, `custodial_clawback`, `staking_lock`, `vesting_schedule`, `protocol_pending`.

**Notes:**
- Implementations SHOULD derive `finality_basis` automatically from the Chain Registry entry's `protocol_required.finality_basis_default` when not explicitly supplied by the event source.
- `FN+0` events (State=FN, Finality=0) represent settled-but-challengeable states and are fully valid when `finality_basis` is present.

---

# Bridge Registry Rules

Bridge Registry rules apply when an event includes a `bridge_id` field referencing an entry in `bridge_registry.json`.

> **Scope note:** Rules in this section are **conditionally inactive**. They apply ONLY when `bridge_id` is present in the event. For all events without a `bridge_id` field, this entire rule set is inert and MUST NOT be evaluated. This applies regardless of Class, Origin, or Confidence values.

**Position in validation order:** Steps 3a–3b (after SEM-016; before SEM-005–014).

## BRG-000 — Bridge Registry Entry Required

When an event includes `bridge_id`, the value MUST resolve to an entry in `bridge_registry.json`.

Reject events that reference an unknown bridge identifier. This rule does not apply when `bridge_id` is absent; bridge validation is inert for non-bridge events.

## BRG-001 — Bridge Trust Flag Enforcement

Events referencing a bridge with `bridge_trust_flag == 0` MUST have Action BL, regardless of all other field values.

- Force Action BL.
- Reject PR, TR, HL, RW, CL claims.
- **Non-overridable.** `chain_trust_flag` and `bridge_trust_flag` are independent; a healthy chain does not rehabilitate a disabled bridge.

## BRG-002 — Disabled Bridge Metadata Completeness

A Bridge Registry entry with `bridge_trust_flag == 0` MUST have `bridge_status`, `bridge_status_reason`, and `bridge_status_effective_from` populated.

Reject or hold the event if the registry entry is incomplete.

## BRG-003 — Classification Audit Requirement

A bridge declaring `classification_profile.classification_verified == true` and claiming above Confidence C MUST:

1. Document the enforcement mechanism in `classification_binding_method` and `classification_notes`.
2. Provide `classification_audit_ref` pointing to an independent audit, formal verification result, canonical verifier, or governance-approved review record.

Self-certification alone is insufficient for `classification_confidence_ceiling` above C.

Reject classification-verified claim above C without audit reference; cap event Confidence at C or D depending on subprofile.

## BRG-004 — Registry Version Integrity

Changes to `verification_regime`, `classification_profile`, `source_chain_id`, `destination_chain_id`, or `bridge_contract_address` in a Bridge Registry entry MUST trigger a new Bridge Registry version.

Reject the event if a registry version mismatch exists between the event's `bridge_id` reference and the current operative registry version.

## BRG-005 — Restoration Gate

A bridge disabled for semantic classification failure or economic consistency failure MUST NOT be restored to `bridge_trust_flag == 1` unless the Bridge Registry entry documents ALL of the following:

1. Patched or redeployed contract version
2. Classification-binding mechanism
3. Economic consistency mechanism
4. Independent review or audit reference
5. Effective deployment block and date
6. `restoration_confidence_ceiling` of B or lower
7. `restoration_ceiling_effective_until` period (minimum duration is a governance parameter)

Upon restoration, the Bridge Registry entry MUST specify `restoration_confidence_ceiling`. Restoration to Confidence A is unavailable until the ceiling period has elapsed and a subsequent governance-approved review is recorded.

Reject restoration attempts that do not satisfy all conditions; emit `BRIDGE_RESTORATION_BLOCKED`.

## BRG-006 — Confidence Subprofile Requirement

A bridge profile MUST identify its applicable confidence subprofile in `confidence_subprofiles.json` where bridge-origin events claim Confidence above D.

- Cap Confidence at D if no subprofile is declared.
- Reject only where the Bridge Registry entry expressly sets `requires_subprofile: true`.

---

# Proof Classification Binding

## SEM-017 — Proof Classification Binding

**Position in validation order:** Step 6. After SEM-016 and BRG-001 where `bridge_id` is present; before SEM-015.

A `proof_ref` that proves the existence of a source-domain data structure MUST bind, disclose, or otherwise establish all of the following via the `proof_semantics` object:

1. `proven_content` — the data, hash, state element, message, output, event, or object that exists in the source domain
2. `source_claim_type` — the source-domain classification of the proven object under the source protocol, registry, contract, legal authority, or attestation framework
3. `destination_claim_type` — the destination-domain type or operational meaning asserted by this Arrow event
4. `classification_binding_method` — the mechanism by which the classification is proven, attested, audited, or inferred (registry-controlled)
5. `classification_verified` — boolean result of the classification check

**Rejection rules:**

| Condition | Required Behavior |
|---|---|
| `classification_verified == false` | Event MUST NOT exceed Confidence C |
| Bridge or regime requires classification binding; event does not establish it | Reject with `PROOF_TYPE_UNBOUND` |
| Regime is not classification-bound; event claims Confidence A or B | Reject with `CLASSIFICATION_REGIME_CEILING` |
| `proof_semantics` absent when required | Reject with `PROOF_TYPE_UNBOUND` |

**When SEM-017 applies:** any event with `bridge_id` present where the declared `verification_regime` is not `native_canonical` or `optimistic_settled`.

**Why:** a proof of source-domain data existence is not a proof of source-domain classification of that data. The Verus-Ethereum Bridge exploit (RFC-002) demonstrates that accepting existence proofs without type binding enables classification confusion attacks at the destination.

## MSG-005 — Classification Binding Method Registry Membership

*(Reclassified from SEM-018 in v1.5. This is a schema/format constraint, not a semantic rule.)*

**Position in validation order:** Step 7. After SEM-017; before SEM-019.

Where `proof_semantics.classification_binding_method` is present, its value MUST reference a registered entry in `classification_binding_methods.json`.

- Freeform strings are not permitted.
- Unknown binding method values MUST be rejected.

**When MSG-005 applies:** any event where `proof_semantics` is present and `classification_binding_method` is non-null.

## SEM-019 — Economic Consistency Binding

**Position in validation order:** Step 8. After MSG-005; before BRG-003/006 and SEM-015.

For Class 5 (TRANSFER) or Class 6 (CLOSE) events claiming Confidence A or B where `bridge_id` is present:

The event MUST include an `economic_consistency` object establishing:

1. `source_value` — value demonstrated at the source domain
2. `destination_value` — value claimed at the destination domain
3. `consistency_verified` — whether source and destination values are economically consistent

**Rejection rules:**

| Condition | Required Behavior |
|---|---|
| `economic_consistency` absent when required | Reject with `ECONOMIC_CONSISTENCY_UNBOUND` |
| `consistency_verified == false` | Event MUST NOT claim Confidence A or B |
| Confidence A or B claimed, `consistency_verified == true`, `economic_consistency_audit_ref` absent | Reject with `ECONOMIC_CONSISTENCY_UNBOUND` |

**Why:** source-side value, authority, and destination-side payout must be proven consistent. A proof that funds exist at the source is not proof that the quantity and conditions of those funds authorize the amount being released at the destination.

---

# Chain Registry Rules

| Rule ID | Requirement |
|---|---|
| CHN-001 | Chain-native events SHOULD include `chain_id`. Required when multiple Chain Registry entries exist for the same Origin. |
| CHN-002 | `chain_registry_version_at_event` MUST match the version operative at `timestamp`. |
| CHN-003 | `chain_trust_flag == 0` requires `chain_status`, `chain_status_reason`, and `chain_status_effective_from` to all be populated. |
| CHN-004 | Changes to `consensus_mechanism`, `asset_proof_mechanism`, `slashing`, `safe_reorg_depth`, `finality_type`, or `verification_regime` MUST trigger a new Chain Registry version. |
| CHN-008 | **Registry lint (build-time).** In the mainnet Chain Registry, an entry with `status: active` MUST carry a non-null `lifecycle_test` (RFC-006 R-009). The mainnet on-chain lifecycle test is the activation gate: a chain MAY be staged as `status: proposed` with optional testnet/devnet pre-flight evidence, but MUST NOT be promoted to `active` without a mainnet test. Enforced by `tools/validate-registries.js`; the testnet overlay (`chain_registry.testnet.json`) is exempt. |

---

# Divergence Event Rules

| Rule ID | Requirement |
|---|---|
| DIV-001 | Divergence events MUST be Class 1 (OBSERVE), reason_code `DIVERGENCE_DETECTED`, object_type `chain_observation_state`, State CD. |
| DIV-002 | Divergence event metadata MUST include `source_a`, `source_b`, `score_a`, `score_b`, `delta`, `threshold`, and the divergent `metric`. |
| DIV-003 | State transition events (ACTIVE → CONDITIONAL, etc.) MUST be Class 4 (CONTROL) referencing the triggering divergence event in `related_events`. |
| DIV-004 | A chain in CONDITIONAL state MUST NOT produce events with Confidence above C until restored to ACTIVE. |

---

# LEG-001 — PROTOCOL Jurisdiction Resolution

If `jurisdiction == PROTOCOL` and `metadata.protocol_ref` is absent:

- Action MUST be HL until a valid protocol ruleset reference is provided.

---

# LEG Action Derivation Rules (v1.5.1-draft)

These rules apply to events where `origin = LEG`. They run at **Step 5b** of the validation pipeline, after SEM-001–014, SEM-013, and FB-001.

> **Renumbering note (v1.5.1):** LEG-006 in v1.5 was a schema constraint and has moved to MSG-006. LEG-007/008/009/010/011 from v1.5 each shift down by one. In v1.6.0 the LEG range violation reason code is `LEG_ACTION_RANGE_VIOLATION`.

## LEG-004 — Confidence Floor for Unverifiable Legal Signal

**Severity:** Fatal

If `origin = LEG` and `Confidence = E`, Action MUST be `RW`. All other Action values (`PR`, `TR`, `BL`, `HL`, `CL`) are prohibited. An unverifiable legal signal cannot authorize, block, hold, or close. Implementations MAY escalate to full rejection at the deployment policy layer (Step 13).

*Supersedes SEM-009 for LEG-origin events in the E-confidence case (more restrictive).*

## LEG-005 — Confidence D Subtype Gate

**Severity:** Fatal

If `origin = LEG` and `Confidence = D`, `leg_subtype` MUST be `LEG_TEXT` or `PREC`. Confidence D is not permitted for `JUD`, `ADM`, or `SUPRA` subtypes. A judicial or administrative order that cannot be verified to Confidence C cannot produce an executing Action.

## LEG-006 — Action Derivation Gate *(v1.5.1: was LEG-007)*

**Severity:** Fatal

If `origin = LEG`, the submitted Action MUST fall within the permitted range for the `leg_subtype` + `Finality` combination.

**Finality = 0 handling.** The `Finality = 0` row is governed by **SEM-013** (FN+0 Executing-Action Ceiling), which is protocol-wide and origin-agnostic: executing actions `{PR, TR, CL}` are forbidden. LEG-006 does not re-enumerate the `Finality = 0` row.

**Permitted Action when `Finality = 1`:**

| `leg_subtype` | Permitted Action |
|---|---|
| `JUD` | `PR`, `BL`, `TR`, `CL` |
| `ADM` | `PR`, `BL`, `HL`, `CL` |
| `SUPRA` (non-SA) | `PR`, `HL` |
| `SUPRA` + `reason_code = SA` | `BL` (forced — LEG-007) |
| `LEG_TEXT` | `HL`, `RW` |
| `PREC` | `RW` |

A claim outside the permitted range MUST be rejected with reason code `LEG_ACTION_RANGE_VIOLATION`. **Silent downgrade is prohibited.**

**Evaluation order within Step 5b:** LEG-007 (force-BL) runs *before* LEG-006 (range check) so that the SUPRA+SA branch is normalized to `BL` before its range comparison.

## LEG-007 — SUPRA Sanctions Force-BL *(v1.5.1: was LEG-008)*

**Severity:** Fatal (force)

If `origin = LEG`, `leg_subtype = SUPRA`, `Finality = 1`, and `reason_code = SA`, Action MUST be `BL` regardless of all other fields. Non-overridable. Mirrors BRG-001 and SAN-001. Any other Action claim is rejected and `BL` is applied. Violation recorded with reason code `LEG_SUPRA_SANCTIONS_FORCE`.

## LEG-008 — LEG_TEXT/PREC with Finality=1 Warning *(v1.5.1: was LEG-009)*

**Severity:** Warning

If `origin = LEG`, `leg_subtype ∈ {LEG_TEXT, PREC}`, and `Finality = 1`, the validator SHOULD emit a warning. A legislative norm or precedent flagged as final is unusual. A `LEG_TEXT` event with `Finality = 1` SHOULD reference `related_events` identifying the prior norm it supersedes.

---

# LEG Jurisdiction Rules (v1.5.1-draft)

These rules apply to `origin = LEG` events and run at **Step 5b**, after LEG-004 through LEG-008.

## LEG-009 — `leg_jurisdiction` Required *(v1.5.1: was LEG-010)*

**Severity:** Fatal

If `origin = LEG`, `leg_jurisdiction` MUST be present. Within `leg_jurisdiction`, at least one of `country` or `supra` MUST be present. A missing object or an object with neither `country` nor `supra` MUST be rejected.

## LEG-010 — `supra` Registry Membership *(v1.5.1: was LEG-011)*

**Severity:** Fatal

If `leg_jurisdiction.supra` is present, its value MUST be a registered entry in `registries/leg_supra_jurisdictions.json` with `status = active`. An unregistered `supra` value MUST be rejected.

## Retired and Reclassified LEG Slots

| Slot | Disposition |
|---|---|
| LEG-006 (v1.5) | **Reclassified to MSG-006** in v1.5.1 (schema constraint). |
| LEG-011 (v1.5) | **Renumbered to LEG-010** in v1.5.1. |
| LEG-012 (legacy) | Moved to **LEG-012-EXT** (Compliance Module) in v1.5; v1.5.1 slot remains free for future LEG rules. |
| LEG-013 (legacy) | Moved to **LEG-013-EXT** (Compliance Module) in v1.5; v1.5.1 slot remains free. |
| LEG-014 | **RETIRED** in v1.5. Flat `jurisdiction` is a computed projection of `leg_jurisdiction`; no separate runtime rule required. |

---

# Compliance Module Extensions — LEG-012-EXT and LEG-013-EXT

These rules run at **Step 14** (Compliance Module), after the core LEG rules in Step 5b.

## LEG-012-EXT (formerly LEG-012)

**Severity:** Fatal

When `extensions.legal_instrument.local_forum` is present, its value MUST match `[A-Z]{2,4}` and `leg_jurisdiction.country` MUST be present in the same event. A `local_forum` value without a country anchor MUST be rejected.

## LEG-013-EXT (formerly LEG-013)

**Severity:** Deployment-configurable (silent / warning / fatal)

When `extensions.legal_instrument.local_forum` is present and the deployment has a LexCodex subnational registry configured, the value SHOULD resolve to a registered entry in that registry. The protocol defines the check; severity is set in the deployment policy configuration (Step 13). Default recommended: `warning`. The event is not protocol-level rejected at any configuration level.

---

# Lifecycle Rules

| Rule ID | Requirement |
|---|---|
| LFC-001 | `event_id` MUST be globally unique within the lifecycle context. Duplicate values MUST be rejected as replay attempts. |
| LFC-002 | `lifecycle_id` MUST be present on every event. `lifecycle_id` MUST remain consistent across all events in a shared lifecycle. |
| LFC-003 | `related_events` MUST reference valid, known `event_id` values within the lifecycle context. |
| LFC-004 | Superseded or reversed events MUST reference the prior event in `related_events`. |
| LFC-005 | Arrow events are append-only. No mutation of existing events is permitted. |
| LFC-006 | **RETIRED v1.5** — was a duplicate of LFC-001. Duplicate event_id rejection is fully captured by LFC-001. |
| LFC-007 | **Post-Closure Lifecycle Discipline (RFC-010).** Given lifecycle context, once a lifecycle contains a validated Class 6 closure, a subsequent event is valid ONLY if it is a Class 1 State RV reversal or a Class 4 (CONTROL) supersession, it cites the closure's `event_id` specifically in `related_events`, AND it satisfies the interim origin constraint: the reopening shares the closure's Origin, or is a supersession from `PCY` (the governance origin, whose source domain is the control plane). Anything else — including a second closure or a cross-origin reopening — REJECTS with `LFC-007`, reason code `POST_CLOSURE_VIOLATION`. Context-gated: without lifecycle context the rule does not fire (like LFC-001/003). The `control_subtype = SUPERSESSION` clause is dormant until SEM-021 / `control_subtypes.json` promotion; the origin constraint is interim hardening until then (shared predicate: `arrow_core::validation::reopening`, consumed by gate and consumers alike). |
| LFC-008 | **Closure Expectation Declaration (RFC-010 Part 2A — Universal Eventual Closure).** Every lifecycle MUST eventually close. A Class 1, State FN, Finality=1 event whose Origin's regime declares a *bounded* `closure_discipline` (`depth_exhaustion` or `window_exhaustion`) MUST carry `closure_eta` + `closure_eta_type` declaring when its lifecycle is due to close (0 `block_count` = due immediately at settlement). Absent/incomplete declaration → REJECT with `LFC-008`. `attestation_bound` and `legal_exhaustion` regimes carry no fixed deadline and are exempt; reversals (State RV) and closures (Class 6) are out of scope by shape. The obligation is monitored *passively*: overdue-unclosed is computed consumer-side at reduction/execution touchpoints (Tarquin LC-016), never by an ambient clock in `V`. Deterministic horizons are on the order of hours; legal horizons are weeks or longer with no wall-clock presumption of overdue. An event that fails to close on schedule is possibly under attack — this declaration is what makes "overdue" computable. |

---

# Action Target Rules

## TRG-001 — TR Action-Target Envelope (RFC-010, was draft; live)

Action `TR` (Trigger) differs from `PR` only through this rule: a trigger must name its target, object, and authorizing instrument, machine-checkably.

An event whose Action is `TR`:

1. MUST carry an `action_target` envelope: `target_ref` (non-empty), `object_ref` (non-empty), `authorizing_events` (non-empty).
2. MUST have Class ∈ {2 INSTRUCT, 5 TRANSFER}. (Six-field clause; enumerable.)
3. Every entry of `action_target.authorizing_events` MUST also appear in `related_events` (resolution against lifecycle context remains LFC-003's job).

Violation of any clause → REJECT with rule code `TRG-001`.

**Enforcement split (architect, 2026-07-07):** the gate checks *shape* only; whether `target_ref` resolves to a real workflow is consumer-side (Tarquin LC-014 routes or holds — never executes a TR directly, per LC-002). The envelope binds to the **Action** namespace — it is deliberately not named `control_target`.

Interactions: TR at Finality 0 → SEM-013; at Confidence E → SEM-009; in Class 3 → SEM-006; in Class 6 → SEM-005/020. TRG-001 adds the Class {1,4} rejection and the envelope obligations, which nothing else covered.

---

# Event Object Validation

## Required Fields

| Field | Requirement |
|---|---|
| event_id | MUST be present and globally unique |
| lifecycle_id | MUST be present |
| arrow_code | MUST be present and valid |
| object_type | MUST be present and registry-controlled |
| object_id | MUST be present |
| subject_type | MUST be present and registry-controlled |
| subject_id | MUST be present |
| reason_code | MUST be present and registry-controlled |
| proof_type | MUST be present and registry-controlled |
| proof_ref | MUST be present |
| timestamp | MUST be present and valid ISO 8601 UTC |
| status | MUST be present: active / superseded / reversed |
| version | MUST be present |

## Conditional Fields (v1.3+)

| Field | Condition |
|---|---|
| chain_id | SHOULD be present on chain-native events |
| chain_registry_version_at_event | MUST be present when `chain_id` is present |
| metadata.composite_confidence_numeric | MUST be present for multi-source events |

## Conditional Fields (v1.4)

| Field | Condition |
|---|---|
| bridge_id | MUST be present when event is routed through a registered bridge |
| proof_semantics | Required when `bridge_id` is present and regime is not `native_canonical` or `optimistic_settled` |
| economic_consistency | Required for Class 5/6 events claiming Confidence A or B with `bridge_id` present |
| economic_consistency_audit_ref | Required when `economic_consistency.consistency_verified == true` and Confidence A or B is claimed |

## Conditional Fields (v1.5)

| Field | Condition |
|---|---|
| finality_basis | Required when `Finality = 0` (FB-001) |
| finality_eta | Required when `finality_basis` entry sets `requires_finality_eta: true` |
| action_target | Required when Action = TR (TRG-001): `target_ref`, `object_ref`, `authorizing_events` ⊆ `related_events` |
| closure_eta / closure_eta_type | Required on Class 1 FN Finality=1 events under a bounded `closure_discipline` (LFC-008) |
| leg_subtype | Required when `origin = LEG` (MSG-006) |
| leg_jurisdiction | Required when `origin = LEG` (LEG-009) |

---

# Order of Validation

```
Step 1   Parse canonical message string
         → Reject if MSG-001 to MSG-004 fail
         → If origin = LEG: reject if MSG-006 fails (leg_subtype absent or unregistered)

Step 2   Resolve Origin in the Origin Registry; resolve chain_id in the Chain Registry when present
         → Reject if origin is unregistered or chain_id references an unknown chain

Step 3   Check chain_trust_flag (SEM-016)
         → Force Action BL if 0; reject PR/TR/HL/RW/CL

Step 3a  If bridge_id present: resolve Bridge Registry entry
         → Reject with BRG-000 if bridge_id unregistered

Step 3b  If bridge_trust_flag == 0: apply BRG-001
         → Force Action BL; reject all non-BL actions

Step 4   Verify verification_regime is declared
         → Reject if null

Step 5   Apply semantic constraints SEM-001 to SEM-014 and SEM-020
         → Includes SEM-013 (FN+0 Executing-Action Ceiling): reject if Finality=0 and Action ∈ {PR, TR, CL}
         → Includes SEM-020 (Closure Class Binding): reject if Action=CL and Class ≠ 6
         → Reject if any constraint fails

Step 5a  Apply FB-001 — Finality Basis Requirement
         → If Finality = 0, finality_basis MUST be present and registered
         → Reject with rule code FB-001 if missing
         → Reject with rule code FB-001 if unregistered value

Step 5b  If origin = LEG: apply LEG Action Derivation Gate
         → LEG-009: reject if leg_jurisdiction absent or missing country+supra
         → LEG-004: reject if Confidence = E and Action ≠ RW
         → LEG-005: reject if Confidence = D and subtype ∉ {LEG_TEXT, PREC}
         → LEG-007: force Action BL if subtype=SUPRA + reason_code=SA + Finality=1
         → LEG-006: derive permitted Action range (Finality=1 table); reject if outside range
         → LEG-008: warn if subtype ∈ {LEG_TEXT, PREC} and Finality=1
         → LEG-010: reject if supra not in leg_supra_jurisdictions registry

Step 6   Apply SEM-017 — Proof Classification Binding
         → Reject PROOF_TYPE_UNBOUND
         → Reject CLASSIFICATION_REGIME_CEILING
         → Cap Confidence at C if classification_verified == false

Step 7   Apply MSG-005 — Classification Binding Method Registry Membership
         → Reject if classification_binding_method not registered

Step 8   Apply SEM-019 — Economic Consistency Binding
         → Reject ECONOMIC_CONSISTENCY_UNBOUND
         → Cap Confidence at C if consistency_verified == false on Class 5/6

Step 9   Apply BRG-003 and BRG-006
         → Reject classification-verified claim above C without audit ref
         → Cap Confidence at D if no subprofile declared

Step 10  Apply SEM-015 — Regime/Confidence Ceiling
         → Reject if Confidence exceeds regime max ceiling

Step 11  For multi-source observations: compute numeric confidence
         and evaluate divergence (DIV-001 to DIV-004)
         → Suspend if divergence detected

Step 12  Apply protocol confidence floor (SEM-009)
         → Reject if Confidence E with Action PR or TR

Step 13  Pass to deployment policy layer
         → Deployment-specific thresholds apply; not protocol-level

Step 14  If extensions.* present: apply Compliance Module
         (TRV-001–008, SAN-001–003, LEG-002–003, CMX-001–002,
          LEG-012-EXT, LEG-013-EXT)
         → Fatal: SAN-001, LEG-003, CMX-001, LEG-012-EXT
         → Warning/configurable: SAN-003, TRV-008, LEG-013-EXT
```

Steps 1–12 and Step 14 are protocol-level. Step 13 is deployment policy.

---

# Rejection Behavior

| Failure Type | Required Behavior |
|---|---|
| Missing required field | MUST reject |
| Unknown registry value | MUST reject |
| Semantic violation (SEM-001–014, SEM-020) | MUST reject |
| FN+0 executing action (SEM-013) | MUST reject |
| CL outside Class 6 (SEM-020) | MUST reject |
| Confidence ceiling exceeded (SEM-015) | MUST reject |
| Pre-settlement ceiling exceeded at Finality=0 (SEM-015) | MUST reject |
| Settled Confidence claim with event-carried `finality_eta` in the event's own future (SEM-015) | MUST reject |
| `chain_trust_flag == 0` | MUST reject with Action BL |
| `finality_basis` absent when Finality=0 (FB-001) | MUST reject |
| `finality_basis` unregistered value (FB-001) | MUST reject |
| `finality_eta` absent when basis sets `requires_finality_eta` (FB-001) | MUST reject |
| TR without a complete `action_target` envelope, or outside Classes {2,5} (TRG-001) | MUST reject |
| Event after closure outside the two reopening shapes (LFC-007) | MUST reject with `POST_CLOSURE_VIOLATION` |
| Settled Class 1 event under bounded closure discipline without `closure_eta` (LFC-008) | MUST reject |
| `leg_subtype` absent or unknown when origin=LEG (MSG-006) | MUST reject with `LEG_SUBTYPE_ABSENT` |
| Action outside LEG permitted range (LEG-006) | MUST reject with `LEG_ACTION_RANGE_VIOLATION` |
| SUPRA+SA+Finality=1 non-BL action (LEG-007) | MUST force BL; record `LEG_SUPRA_SANCTIONS_FORCE` |
| `leg_jurisdiction` absent when origin=LEG (LEG-009) | MUST reject |
| `supra` value unregistered (LEG-010) | MUST reject |
| `bridge_trust_flag == 0` (BRG-001) | MUST reject with Action BL — non-overridable |
| Bridge unregistered (BRG-000) | MUST reject |
| Proof classification unbound (SEM-017) | MUST reject with `PROOF_TYPE_UNBOUND` |
| Classification regime ceiling exceeded (SEM-017) | MUST reject with `CLASSIFICATION_REGIME_CEILING` |
| Unknown classification binding method (MSG-005) | MUST reject |
| Economic consistency unbound (SEM-019) | MUST reject with `ECONOMIC_CONSISTENCY_UNBOUND` |
| Bridge restoration conditions unmet (BRG-005) | MUST reject with `BRIDGE_RESTORATION_BLOCKED` |
| Registry version mismatch | MUST reject |
| Malformed divergence event | MUST reject |
| Duplicate `event_id` | MUST reject as replay attempt |
| Invalid timestamp | MUST reject |
| Unresolved `related_events` | SHOULD hold pending resolution |

---

# Error Classes

| Code | Rule | Meaning |
|---|---|---|
| MSG-001 | MSG-001 | Invalid message field count |
| MSG-004 | MSG-004 | Unknown registry vocabulary |
| MSG-005 | MSG-005 | Unknown classification binding method |
| MSG-006 | MSG-006 | `leg_subtype` absent or unregistered — reason code `LEG_SUBTYPE_ABSENT` |
| SEM-001 | SEM-001 | Invalid finality/state combination |
| SEM-009 | SEM-009 | Confidence floor violation — E with PR or TR |
| SEM-013 | SEM-013 | FN+0 executing-action ceiling violation — Finality=0 with Action ∈ {PR, TR, CL} |
| SEM-020 | SEM-020 | Closure class binding violation — Action CL with Class ≠ 6 |
| SEM-015 | SEM-015 | Confidence exceeds regime ceiling |
| SEM-016 | SEM-016 | Chain trust override violation |
| SEM-017 | SEM-017 | Proof type unbound — reason code `PROOF_TYPE_UNBOUND` or `CLASSIFICATION_REGIME_CEILING` |
| SEM-019 | SEM-019 | Economic consistency unbound — reason code `ECONOMIC_CONSISTENCY_UNBOUND` |
| BRG-000 | BRG-000 | `bridge_id` does not resolve to a Bridge Registry entry |
| BRG-001 | BRG-001 | Bridge trust flag override violation |
| BRG-005 | BRG-005 | Bridge restoration blocked — reason code `BRIDGE_RESTORATION_BLOCKED` |
| CHN-002 | CHN-002 | Chain registry version mismatch |
| DIV-002 | DIV-002 | Malformed divergence event |
| FB-001 | FB-001 | `finality_basis` absent or unregistered when Finality=0 |
| LEG-001 | LEG-001 | Missing `protocol_ref` on PROTOCOL jurisdiction |
| LEG-006 | LEG-006 | Action outside permitted LEG range — reason code `LEG_ACTION_RANGE_VIOLATION` |
| LEG-007 | LEG-007 | SUPRA+SA force-BL — reason code `LEG_SUPRA_SANCTIONS_FORCE` |
| LEG-009 | LEG-009 | `leg_jurisdiction` absent or missing country+supra |
| LEG-010 | LEG-010 | `supra` value not in `leg_supra_jurisdictions` registry |
| LFC-001 | LFC-001 | Duplicate event_id replay attempt |
| LFC-007 | LFC-007 | Post-closure lifecycle discipline |
| LFC-008 | LFC-008 | Closure expectation declaration |
| TRG-001 | TRG-001 | TR action-target envelope + class constraint |

---

# Example Valid Sequences

## RFC-001 Composition Pattern (Multi-Regime Authorization)

```text
2:LEG:FN:PR:1:C    — Legal attestation (court order)
                     verification_regime: legal_attestation
                     jurisdiction: US

2:PCY:FN:PR:1:B    — DAO governance vote
                     verification_regime: bonded_quorum

5:ARB:FN:TR:1:A    — On-chain transfer execution
                     verification_regime: optimistic_settled
                     related_events: [court_order_event, dao_vote_event]

6:ARB:FN:CL:1:A    — Settlement
                     related_events: [transfer_event]
```

Demonstrates: legal attestation → bonded quorum governance → optimistic settlement → composed multi-regime authorization.

## SEM-013 — FN+0 Executing-Action Ceiling

```text
Event submitted:
  arrow_code: 5:ARB:FN:TR:0:A
  finality_basis: optimistic_window

Step 5 — SEM-013:
  Finality=0 with Action=TR (executing) → REJECT

Result: SEM-013 violation
```

## LEG-006 — Action Derivation Gate (Finality=1)

```text
Event submitted:
  arrow_code: 2:LEG:FN:PR:1:B
  leg_subtype: PREC
  reason_code: CO

Step 5b — LEG-006:
  PREC + Finality=1 → permitted = {RW}
  Submitted Action is PR → REJECT with LEG_ACTION_RANGE_VIOLATION
```

## LEG-007 — SUPRA Sanctions Force-BL

```text
Event submitted:
  arrow_code: 2:LEG:FN:PR:1:B
  leg_subtype: SUPRA
  leg_jurisdiction: { supra: UNSC }
  reason_code: SA

Step 5b — LEG-007:
  SUPRA + SA + Finality=1 → Action forced to BL
  Submitted Action is PR → REJECT with LEG_SUPRA_SANCTIONS_FORCE
  forced_action = BL
```

## RFC-002 Demo Scenario 1 — Bridge Trust Revoked (BRG-001)

```text
Event submitted:
  arrow_code: 5:THR:FN:PR:1:B
  bridge_id: VRSC-ETH-001

Bridge Registry check (Step 3a):
  bridge_id VRSC-ETH-001 → bridge_trust_flag: 0

BRG-001 (Step 3b):
  bridge_trust_flag == 0 → Action MUST be BL
  Submitted action is PR → REJECT

Result: BRIDGE_TRUST_REVOKED
```

## RFC-002 Demo Scenario 2 — Classification Unbound (SEM-017)

```text
Event submitted:
  arrow_code: 5:THR:FN:PR:1:A
  bridge_id: [active bridge with bonded_quorum regime]
  proof_semantics: {
    proven_content: "0xabc123...",
    source_claim_type: "supplemental_export_output",
    destination_claim_type: "primary_export_instruction",
    classification_binding_method: "context_inference",
    classification_verified: false
  }

SEM-017 (Step 6):
  classification_verified == false → Confidence MUST NOT exceed C
  Submitted Confidence is A → REJECT

Result: Confidence capped; event rejected at A claim
```

## Compromised Chain Revocation (SEM-016)

```text
Registry state:
  chain_id: COMPROMISED-1
  chain_trust_flag: 0
  chain_status: compromised

Event submitted:
  arrow_code: 1:ETH:FN:PR:1:A
  chain_id: COMPROMISED-1

Step 3 — SEM-016:
  chain_trust_flag == 0 → Action MUST be BL
  Submitted action is PR → REJECT
```

---

# Document Information

| Field | Value |
|---|---|
| Document | Arrow Validation Rules |
| Version | 1.6.0 |
| Status | Current v1.6.0 package |
| Date | 2026-06-10 |
| Depends On | Arrow Protocol v1.6.0, Arrow Registries v1.6.0, RFC-003 v1.0, RFC-004 v1.0, RFC-005 v1.1-draft, RFC-006 v0.5 draft, Arrow RFC-008 v0.3 |
| Supersedes | Validation Rules v1.5.2-draft |
| Governance | Editor-controlled for v1.x; rule-ID permanence from v1.6.0 |
