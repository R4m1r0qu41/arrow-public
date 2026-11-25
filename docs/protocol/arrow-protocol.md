# Arrow Protocol v1.6.0

**Status:** live
**Date:** 2026-06-10
**Supersedes:** Arrow Protocol v1.5.2-draft (2026-06-07)
**Depends on:** Arrow Validation Rules v1.6.0, Arrow Registries v1.6.0, Arrow Transport Model v1.0, RFC-003 v1.0, RFC-004 v1.0, RFC-005 v1.1-draft, RFC-006 v0.5 draft, Arrow RFC-008 v0.3
**Genesis record:** [RFC-000](../../rfc/RFC-000.md)
**Reference cases:** [RFC-001 — KelpDAO Recovery](../../rfc/RFC-001-KelpDAO.md) · [RFC-002 — Verus-Ethereum Bridge Exploit](../../rfc/RFC-002-Verus-SemanticTypeConfusion-v1.md)
**Architecture / normative RFCs:** [RFC-003 — Arrow as an Interoperable State Machine](../../rfc/RFC-003-Arrow-as-Interoperable-State-Machine-v1.md) · [RFC-004 — Coherence and Minimization](../../rfc/RFC-004-Coherence-and-Minimization-v1.md) · [RFC-005 — Provenance, Trustless Sourcing, and Adapter Interchange](../../rfc/RFC-005-Provenance-and-Trustless-Sourcing-v1.md)
**Graduated infrastructure RFCs:** [RFC-006 — Robust Chain Registry](../../rfc/RFC-006-Robust-Chain-Registry-v1.md) · [RFC-007 — Declared Policy, Recognized Authority, and Integration Strength](../../rfc/RFC-007-Declared-Policy-and-Integration-Strength-v1.md)

---

## Abstract

Arrow Protocol is a stateful, append-only, machine-readable event standard for coordinating across blockchains, custodians, legal systems, and applications — without requiring shared execution environments, shared consensus, or native interoperability between chains.

v1.6.0 is the **namespace-decoupling completion release.** Canonical field 4 is **`Action`** (final; codes `PR`/`BL`/`HL`/`RW`/`TR`/`CL` unchanged). Class 4 is **`CONTROL`**, scoped as the control-plane event class and anchored by a new draft `control_subtypes.json` vocabulary. Origin `PCY` is unchanged. After this release each contested term lives in exactly one canonical namespace:

```text
Action  = canonical field 4            (what the consumer may/must do)
CONTROL = Class 4                      (control-plane event class)
PCY     = Origin code for governance source domain (display name: Policy)
"deployment policy" = Step 13 configuration, never a registry term
```

One new semantic rule lands: **SEM-020** (Action `CL` requires Class 6 — the converse of SEM-005), making the Class 6 ⇔ CL binding bidirectional with mismatch rejection. A naming-integrity lint (**NAM-001**) and a **rule-ID permanence policy** make this the last rename release.

This document is a **delta specification.** It SHOULD be read alongside [Arrow Protocol v1.5.2-draft](./arrow-protocol-v1.5.2-draft.md) and the parts it carries forward. All parts not explicitly amended here remain in force, with the field-4 naming mapping in §V1.6.0-Δ-1 and the Class 4 rename in §V1.6.0-Δ-2 applied.

---

## What's New in v1.6.0

v1.6.0 is **non-breaking at the wire layer.** The canonical six-field message string never carried a field label; every v1.5.2 event re-states byte-identically. The changes are name-layer (registry, rule IDs, documentation, reference-implementation types) plus one additive semantic rule (SEM-020) that rejects a combination no registered fixture has ever used legitimately.

| Layer | Change |
|---|---|
| Registry | `actions.json` is the canonical field-4 registry. Codes and downstream semantics unchanged. **Final name frozen.** |
| Registry | `classes.json` — Class 4 is `CONTROL` (control-plane event class). |
| Registry | NEW `control_subtypes.json` (draft) — anchor vocabulary for Class 4 events; inert while draft; `SEM-021` reserved for its promotion. |
| Validation | Field-4 rule identifiers are `ACT-001` / `ACT-002`. Obligations unchanged. |
| Validation | NEW `SEM-020` — Action `CL` MUST use Class 6 (converse of SEM-005). |
| Tooling | NEW `NAM-001` naming-integrity lint in `tools/validate-registries.js` — cross-namespace code/name uniqueness, CI-fatal. |
| Governance | Rule-ID permanence policy: from v1.6.0, rule identifiers are retired, never renumbered, never reused. |
| Reference impl | `arrow-core` exposes `Action` as the field-4 type and `ArrowMessage.action` as the parsed field. |
| Cross-repo | Aligns with Arrow RFC-008 v0.3 and the Tarquin shared LEG surface v0.2. |

---

## Part V1.6.0-Δ-1 — Field 4: Action

The canonical six-field message remains six fields and five delimiters; the fourth field label is now `Action`:

```text
Class:Origin:State:Action:Finality:Confidence
```

The field's registered codes and semantics are unchanged:

| Code | Action |
|---|---|
| `PR` | Proceed |
| `BL` | Block |
| `HL` | Hold |
| `RW` | Review Warranted |
| `TR` | Trigger |
| `CL` | Closed |

The machine-readable registry is `registries/actions.json`. Rule identifiers for the fourth field use the `ACT-*` family:

| Rule ID | Notes |
|---|---|
| `ACT-001` | Action MUST exist in the Action Registry. |
| `ACT-002` | Unknown Action values MUST be rejected. |

**What this enforces.** Field 4 answers exactly one question: *what may or must the downstream consumer do with this event?* — proceed, block, hold, review, trigger, close. "Action" names that question directly. With v1.6.0 the three contested meanings are fully separated: the consumer-instruction field is `Action`, the control-plane event class is `CONTROL`, and the governance source domain is Origin `PCY`.

**What breaks without it.** If the Action field and Class 4 reuse a single term, the sentence "a control event" becomes undecidable between *an event whose Action field is significant* and *a Class 4 control-plane event* — across two repos, where no single linter can see both usages. If Class 4 and Origin `PCY` reuse a single term, `4:PCY:…` cannot be read without consulting the registry to determine whether the event is governance-sourced, control-plane, or both. Both failure modes are the type-confusion class that RFC-002 documents as an exploit primitive.

**Known residual risk (accepted).** "Action" is a legal term of art (a lawsuit or proceeding). In LEG-context prose this creates a collision between *the action before the court* and *the Action field*. This was weighed and accepted by the protocol designer: the field 4 value is the protocol's own disposition vocabulary, registry prose says "Action field" with the capital and never "legal action." Documentation discipline: when both senses can occur in one sentence, write "the Action field" and "the proceeding/matter," never bare "action."

**Layers touched.** Registry (`actions.json` — source of truth); Validation Rules (`ACT-001`/`ACT-002`, all field-4 prose); Spec (this delta); Rust reference implementation (`Action` type, `action` field); fixtures (expectation keys); Tarquin shared surface (consumes the v1.6.0 Action rename).

---

## Part V1.6.0-Δ-2 — Class 4: CONTROL

Class 4 is `CONTROL` and scoped as the **control-plane event class**:

> A Class 4 (CONTROL) event records a protocol-, chain-, registry-, or infrastructure-level state change: registry updates, chain/bridge trust-status transitions (DIV-003), protocol or consensus upgrades, validator/node topology changes (DVN/OVN set or threshold changes), parameter changes, emergency declarations (Origin EMG), and supersession of prior final events (SEM-014).

**Anchor discipline.** A control-plane event with no referent is unauditable. Class 4 events SHOULD carry a `control_subtype` registered in the new draft vocabulary `registries/control_subtypes.json`, whose entries each declare `anchor_requirements` — what the event MUST reference for the change to be auditable (e.g. `REGISTRY_UPDATE` → registry name, affected codes, version before/after; `CHAIN_STATUS` → chain_id, prior/new status, triggering event in `related_events`). While `control_subtypes.json` carries `status: draft` it imposes **no live validation obligation** (gated-registry discipline). Rule slot **`SEM-021` is reserved** for its promotion: *Class 4 events MUST carry a registered `control_subtype` and satisfy its anchor requirements.* Promotion requires the vocabulary to graduate to `active` and fixtures to land with it.

| Subtype (draft) | Control surface |
|---|---|
| `REGISTRY_UPDATE` | Arrow controlled-vocabulary changes |
| `CHAIN_STATUS` | Chain observation/trust transitions (DIV-003, SEM-016, CHN-003) |
| `BRIDGE_STATUS` | Bridge disable/restoration (BRG-001/002/005) |
| `PROTOCOL_UPGRADE` | Consensus changes, version upgrades, forks (cf. CHN-004) |
| `NODE_TOPOLOGY` | Validator/DVN/OVN set, threshold, quorum reconfiguration |
| `PARAMETER_CHANGE` | Bounded protocol/deployment parameter changes |
| `EMERGENCY_DECLARATION` | EMG declared-policy actions |
| `SUPERSESSION` | SEM-014 supersession of a prior final event |

**What this enforces.** Class 4's existing duties in the rule set were always control-plane duties — DIV-003 routes chain-state transitions through it, SEM-014 routes supersession through it, RFC-007 routes emergency declarations through it. The rename makes the class mean what the rules already use it for, and the subtype vocabulary turns "control-plane event" from a vibe into an enumerated, anchored claim. The separation from Origin `PCY` is preserved verbatim from RFC-008: `PCY` answers *who asserts* (a governance body); Class `CONTROL` answers *what kind of change* (control-plane); the Action field answers *what consumers do about it*.

**What breaks without it.** Without the anchor vocabulary, Class 4 becomes a junk drawer: any event that is not obviously observation, instruction, condition, transfer, or close lands in CONTROL with no machine-checkable obligation to say what it changed — which is exactly how an attacker would smuggle a consequential registry change past review, framed as a routine Class 4 event.

**Layers touched.** Registry (`classes.json`, new `control_subtypes.json`); Spec (this delta); Validation Rules (DIV-003 prose, SEM-021 reservation); fixtures unchanged (no live obligation while draft).

---

## Part V1.6.0-Δ-3 — SEM-020: Bidirectional Class 6 ⇔ Action CL Binding

New semantic rule:

> **SEM-020.** If Action = `CL`, Class MUST be 6 (CLOSE). *(Converse of SEM-005: Class 6 MUST use Action CL.)* Violations are rejected; severity Fatal.

Together with SEM-005, SEM-012 (Class 6 ⇒ State FN) and SEM-013 (executing Action ⇒ Finality 1), the **only valid closure shape** in the protocol is:

```text
6:*:FN:CL:1:*
```

**What this enforces.** Inside Class 6, `CL` is informationally redundant — and that is deliberate. The pair is a *redundant-and-checked* encoding: two independent fields must agree before any consumer treats an event as lifecycle-terminal, so a single-field corruption, truncation, or type-confusion (in a bridge, a hand-rolled parser, or a hostile relayer) is detected instead of silently changing lifecycle semantics. Before SEM-020, the check ran in one direction only: a Class 1 OBSERVE event could carry Action `CL` and present terminal semantics to any consumer that keyed off the Action field alone.

**What breaks without it.** A consumer that treats `CL` as "lifecycle closed" — the registry's own definition — could be fed `1:ETH:FN:CL:1:A` (a mere observation wearing a terminal Action) and stop watching a lifecycle that is still live. The inverse consumer, keying off Class 6 only, was already protected by SEM-005. Asymmetric protection is an exploit instruction manual: it tells the attacker which field to lie in.

**Follow-up and reopening discipline (scoping note).** SEM-020 does not make closure eternal; it makes *reopening loud*. The follow-up event model for legal lifecycles (FN+0 decisions later affirmed, reversed, or discharged) is:

| Legal development | Arrow shape |
|---|---|
| Decision affirmed / window closed | New event, same `lifecycle_id`, State `FN`, Finality `1`, referencing the FN+0 event in `related_events` (finality promotion — never a wall-clock mutation) |
| Decision reversed / stayed | Class 1, State `RV`, referencing the reversed event (SEM-014 path) |
| Matter fully discharged | Class 6 `CL` closure: `6:*:FN:CL:1:*` referencing the lifecycle events it terminates |
| Post-closure challenge | Only via SEM-014's two paths — Class 1 State RV reversal or Class 4 (CONTROL) `SUPERSESSION` — each citing the CL event's `event_id` in `related_events`. **LFC-007 (live, RFC-010):** given lifecycle context, no other event is valid after CL; rejection carries reason code `POST_CLOSURE_VIOLATION`. Interim origin constraint until SEM-021: a reopening shares the closure's Origin, or is a supersession from `PCY` (governance). |

Closure is therefore *the* terminal punctuation of a lifecycle, and it is mandatory in this sense: **a lifecycle can only end through Class 6.** Lifecycles are not required to end — a standing hold may stay open indefinitely — but consumers may rely on the invariant that any lifecycle without a Class 6 CL event is still capable of producing follow-up events and must be monitored accordingly.

**Layers touched.** Validation Rules (SEM-020, LFC-007 reservation); Rust (`semantic.rs`, new check + tests); fixtures (new reject fixture F-12); registries (`actions.json` / `classes.json` cross-reference notes).

**RFC-010 addendum (2026-07-07, live).** Three lifecycle gates landed after the paragraph above was written:

- **TRG-001 (live).** Action `TR` carries machine-checkable obligations: an `action_target` envelope (`target_ref`, `object_ref`, `authorizing_events` ⊆ `related_events`) and Class ∈ {2 INSTRUCT, 5 TRANSFER}. The gate checks shape; resolution of `target_ref` is consumer-side (Tarquin LC-014).
- **LFC-007 (live).** Post-closure lifecycle discipline as described in the table above.
- **LFC-008 (live) — Universal Eventual Closure.** Every lifecycle MUST eventually close. Verification regimes declare a `closure_discipline` (`depth_exhaustion`, `window_exhaustion`, `attestation_bound`, `legal_exhaustion`); a Class 1, State FN, Finality 1 event under a bounded discipline MUST carry `closure_eta` / `closure_eta_type` declaring when its lifecycle is due to close (`0` `block_count` = due immediately at settlement). The obligation is monitored passively at consumer touchpoints (Tarquin LC-016); an overdue-unclosed lifecycle is stale and treated as a possible attack indicator. Legal lifecycles carry no fixed deadline. The event object accordingly adds three conditional fields: `action_target`, `closure_eta`, `closure_eta_type` (normative detail in Validation Rules).

---

## Part V1.6.0-Δ-4 — Naming Integrity (NAM-001) and Rule-ID Permanence

Two governance hardenings close the rename era:

**NAM-001 — Cross-namespace naming integrity (registry lint, CI-fatal).** No code and no display name may appear in more than one canonical six-field namespace (classes, origins, states, actions, confidence). The lint runs in `tools/validate-registries.js`. The failure mode recurs because nothing checked it; now CI does.

**Rule-ID permanence.** From v1.6.0, rule identifiers are **permanent**: a rule may be retired, but its identifier is never renumbered and never reused for a different obligation. Rationale: rule IDs are part of the cross-repo governance wire format — Tarquin's producer contract (OA-008, OA-010) cites Arrow rule IDs, and the v1.5.1 LEG renumbering left dangling references in a hash-pinned contract system. Cross-repo artifacts SHOULD pin `(rule_id, semantic_slug)` pairs (e.g. `("LEG-006", "action_derivation_gate")`) so any future drift fails CI instead of failing a human.

---

## Parts Carried Forward Unchanged from v1.5.2-draft

All parts of [Arrow Protocol v1.5.2-draft](./arrow-protocol-v1.5.2-draft.md) — and the v1.5.1/v1.5 parts it carries forward — are normative in v1.6.0 with the field-4 naming mapping in §V1.6.0-Δ-1 and the Class 4 rename in §V1.6.0-Δ-2 applied. No SEM-*, LEG-*, MSG-*, BRG-*, STA-*, FIN-*, or CFD-* rule changes obligation; SEM-020 is additive. The canonical message still has six fields and five delimiters, and the State × Finality matrix is unchanged. The LEG range violation reason code is `LEG_ACTION_RANGE_VIOLATION` in v1.6.0.

---

## Document Information

| Field | Value |
|---|---|
| Version | 1.6.0 |
| Status | Current v1.6.0 package |
| Date | 2026-06-10 |
| Genesis Record | RFC-000 — `2026-03-22T18:40:00Z` |
| Reference Case 1 | RFC-001 — KelpDAO Recovery |
| Reference Case 2 | RFC-002 — Verus-Ethereum Bridge Exploit (2026-05-18) |
| Validation Rules | Arrow Validation Rules v1.6.0 |
| Transport Model | Arrow Transport Model v1.0 |
| Registries | Arrow Registries v1.6.0 |
| Governance | Editor-controlled for v1.x; rule-ID permanence from v1.6.0 |
| Supersedes | Arrow Protocol v1.5.2-draft (2026-06-07) |
| Depends on | RFC-003 v1.0; RFC-004 v1.0; RFC-005 v1.1-draft; RFC-006 v0.5 draft; Arrow RFC-008 v0.3 |
