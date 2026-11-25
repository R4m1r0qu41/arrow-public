# RFC-004 — Coherence and Minimization

| Field | Value |
|---|---|
| Document ID | ARROW-RFC-004 |
| Version | v1.0 |
| Status | Final — **Normative** |
| Date | 2026-05-25 |
| Finalized | 2026-05-26 |
| Track | Protocol (normative) |
| Author | Arrow Protocol — Editor |
| Depends on | Arrow Protocol v1.5-draft; Arrow Validation Rules v1.5-draft; Arrow Registries v1.5.0-draft; RFC-000; RFC-003 v1.0 |
| Targets | Arrow Protocol v1.5; Validation Rules v1.5 |
| Genesis Anchor | RFC-000 — `2026-03-22T18:40:00Z` |
| Supersedes | Nothing — corrects and extends v1.4 |
| Informs | RFC-005; RFC-006 |

---

## 0. Status of This Document

RFC-004 is **final and normative**. It introduces specific changes to the Arrow Protocol specification, the Validation Rules, and several registry files. All changes target Arrow Protocol v1.5 and Validation Rules v1.5. Each Part of this RFC identifies whether its changes are **additive**, **corrective**, or **clarifying**, and whether they affect the canonical six-field string or implementation conformance.

**The canonical six-field string `Class:Origin:State:Policy:Finality:Confidence` is unchanged by this RFC.** Every amendment in RFC-004 operates on the Event Object, the Validation Rules, the registries, or on the specification's prose definitions. Position 5 (Finality) is retained as a binary integer; its semantics are extended, not restructured.

---

## 1. Purpose and Motivation

Arrow Protocol v1.4 was produced iteratively. The result is a sound conceptual architecture with localized coherence failures: registry files that diverge from the specification they are supposed to instantiate; validation rules that overlap, contradict, or are mislabeled; and one field (Finality) whose independent value was never explicitly defined, causing it to appear redundant until its purpose is examined carefully.

RFC-004 applies three disciplines:

1. **Reconcile** — every field, rule, and registry value has exactly one authoritative definition, and definitions agree across documents.
2. **Minimize** — the least number of rules and fields that achieves the protocol's goals. Derived values are computed, not stored. Rules that are structural schema checks are not presented as semantic rules.
3. **Define what was left implicit** — particularly `FN+0`, the one combination that makes Finality a genuinely independent axis rather than a State echo.

The guiding principle, stated once here and not repeated per section: **absorb complexity into the protocol; emit the minimum necessary for safe relay between systems that speak different languages.**

---

## 2. Summary of Changes

| Part | Subject | Change type | Conformance impact |
|---|---|---|---|
| 1 | Policy registry — `RW` missing; `TR`/`CL` name conflict | Corrective | No — additive and rename, no deletions |
| 2 | State registry — `TL` absent; `HL`/`ER` orphaned | Corrective | `TL` restored; `HL`/`ER` removed |
| 3 | Origin length — parser vs spec mismatch | Corrective | Potentially — tightens an over-permissive parser |
| 4 | Verification regime naming — `native_canonical` / `chain_native` | Corrective | No — rename with alias |
| 5 | `LFC-001` overload; LFC rule mislabeling in `pipeline.rs` | Corrective | No — label correction |
| 6 | SEM-013 redundancy | Corrective | No — rule deletion |
| 7 | Finality — define `FN+0`; State × Finality matrix | Clarifying | No — extension of existing semantics |
| 8 | `finality_basis` — new Tier-3 conditional field | Additive | No |
| 9 | LEG minimization — flat `jurisdiction` derivation; `local` demotion | Corrective/Additive | Minor — changes LEG field placement |
| 10 | Rule taxonomy — semantic vs schema-constraint separation | Clarifying | No |
| 11 | Registry `provenance` block (forward hook) | Additive | No |

---

## Part 1 — Policy Registry Reconciliation

### 1.1 Defects identified

The committed `registries/policies.json` (versioned 1.3.1) contains five entries: `PR`, `BL`, `HL`, `TR`, `CL`. The value `RW` (Review Warranted) is absent.

This creates a fatal contradiction:
- **SEM-006** mandates `RW` as a permitted Policy value for Class 3 (CONDITION).
- **MSG-004** mandates that every Policy value must exist in `registries/policies.json`.
- An event asserting Class 3 with Policy `RW` simultaneously satisfies SEM-006 and is rejected by MSG-004.

Additionally, the registry name fields for `TR` and `CL` diverge from the specification:

| Code | Registry name (1.3.1) | Specification name (v1.4) | Correct name |
|---|---|---|---|
| `TR` | "Trusted" | "Trigger" | **Trigger** |
| `CL` | "Clear / Cleared after review" | "Closed" | **Closed** |

"Trusted" and "Trigger" are semantically different claims. The specification is authoritative; the registry is wrong.

### 1.2 Corrections

**Add `RW` to `registries/policies.json`:**

```json
{
  "code": "RW",
  "name": "Review Warranted",
  "description": "The event cannot be acted on autonomously. It must be routed to human review or an elevated decision layer before any policy action is taken. Appropriate when an event's classification is uncertain, when Confidence is low, or when regulatory or legal context requires human judgment.",
  "permitted_classes": [1, 2, 3, 4, 5, 6],
  "notes": "RW is the only permitted Policy for Confidence E LEG-origin events (LEG-004). RW is also the only permitted Policy for PREC-subtype LEG events regardless of Confidence (LEG-007).",
  "added_version": "1.5",
  "status": "active"
}
```

**Correct `TR` name:** `"Trusted"` → `"Trigger"`.

**Correct `CL` name:** `"Clear / Cleared after review"` → `"Closed"`.

**Version bump:** `registries/policies.json` → v1.5.

### 1.3 Conformance impact

Additive (`RW`) and name-only corrections (`TR`, `CL`). No existing valid event is invalidated. Systems that have stored Policy values by code (the canonical identifier) are unaffected. Systems that stored values by name — a violation of the registry contract, since codes are canonical — may need a name-map update.

---

## Part 2 — State Registry Reconciliation

### 2.1 Defects identified

**Defect A — `TL` (Timelocked) absent from `registries/states.json`.** SEM-003 states: "State `TL` requires Finality 0." Every validation run applying SEM-003 to an event with State `TL` will reject the event at MSG-004 (unregistered value) before SEM-003 is reached. SEM-003 is dead code because the field it protects cannot pass the vocabulary check. The specification defines `TL`; the registry does not contain it.

**Defect B — `HL` present in `registries/states.json` but not in the specification.** `HL` exists as a Policy value (`HL` = Hold). Its presence as a State value creates a namespace collision: the same two-character code means different things depending on which field it occupies. This is exactly the class of ambiguity a protocol should not tolerate.

**Defect C — `ER` (Error) present in `registries/states.json` but referenced by nothing.** An error is a validation outcome, not a persisted event state. An event with State `ER` would have passed validation despite representing a validation failure — incoherent. `ER` should not exist as a registered state.

### 2.2 Corrections

**Restore `TL` to `registries/states.json`:**

```json
{
  "code": "TL",
  "name": "Timelocked",
  "description": "The event's conditions are met but execution is deferred to a future time. The lock is defined by a time condition (block height, timestamp, or protocol clock), not by a contingent external event. Finality MUST be 0 (SEM-003). Transitions to FN+1 when the time condition is satisfied and no challenge has been raised.",
  "finality_permitted": [0],
  "added_version": "1.5",
  "status": "active"
}
```

**Remove `HL` from `registries/states.json`.** `HL` is a Policy value. Remove its State entry entirely. Add a `registry_notes` field to the Policy entry for `HL` noting the historical collision and confirming `HL` is Policy-only.

**Remove `ER` from `registries/states.json`.** `ER` is not a valid persistent event state. An event that errors is rejected; it does not receive a stored state value.

**Align `finality_permitted` with SEM-002 through SEM-004.** `PD`, `CD`, and `TL` all permit only `Finality = 0`. `FN` and `RV` permit `0` or `1`.

**Version bump:** `registries/states.json` → v1.5.

### 2.3 Conformance impact

`TL` restoration: additive, no existing events invalidated.
`HL` removal from states: any implementation storing Arrow events with `State = HL` holds invalid events (the registry never supported it against the spec). Removal is corrective; those events were already non-conforming.
`ER` removal: same analysis. No conforming event uses `ER` as a State.

---

## Part 3 — Origin Length Correction

### 3.1 Defect identified

Rule ORG-001 and the specification state that Origin is exactly **three uppercase characters** (`[A-Z]{3}`). The cheatsheet flags `CUST` (4 characters) as an ORG-001 violation.

The Rust parser in `crates/arrow-core/src/parser/message.rs` accepts **2–6 uppercase alphanumeric** characters. The reference implementation is more permissive than the specification. One of them is wrong.

### 3.2 Decision and correction

**The specification is authoritative.** Three characters is the correct constraint. The parser must be corrected to `[A-Z]{3}` — no digits, no length variation.

**Rationale for keeping 3:** three characters is a clean, fixed-width vocabulary. The current registry has entries of exactly three characters (`BTC`, `ETH`, `SOL`, `LEG`, `ORC`, `CST`, `PCY`). Allowing 2–6 means every consumer must handle variable-length position 2; fixed-width eliminates that complexity. A three-character namespace with 17,576 possible values (`A–Z` cubed) is adequate for any foreseeable Origin vocabulary.

**Parser correction:** `parser/message.rs` — change the Origin token regex from `[A-Z0-9]{2,6}` to `[A-Z]{3}`.

### 3.3 Conformance impact

This tightens the parser. Any deployed system submitting 2-character, 4-character, or alphanumeric Origin values is currently passing validation incorrectly. Tightening is corrective. If the protocol has been deployed with non-conforming Origin values, those systems need a one-time migration.

---

## Part 4 — Verification Regime Naming

### 4.1 Defect identified

The specification and README use `native_canonical` as the code for the highest-trust chain-native verification regime. The v1.4 SEM-015 regime table uses `chain_native` for the same concept. Both are committed. A system reading the SEM table and a system reading the registry will behave differently for the same regime.

### 4.2 Correction

**`native_canonical` is the canonical code.** It is already registered in `registries/verification_regimes.json`. The v1.4 SEM-015 table entry must be corrected from `chain_native` to `native_canonical`. The registry entry gains an `aliases` field: `["chain_native"]` to document the historical spelling while preserving one canonical value.

**Registry amendment:**

```json
{
  "code": "native_canonical",
  "aliases": ["chain_native"],
  ...
}
```

**SEM-015 table correction:** all references to `chain_native` → `native_canonical`.

---

## Part 5 — LFC Rule Mislabeling

### 5.1 Defect identified

Rule `LFC-001` is assigned to three different constraints across the codebase and specification:

- Validation rules specification: `LFC-001` = global `event_id` uniqueness.
- `crates/arrow-core/src/pipeline.rs`: `LFC-001` label = `lifecycle_id` present check.
- `crates/arrow-core/src/event.rs`: `LFC-001` label = unresolved `related_events` check (which the spec calls `LFC-003`).

Additionally, `event_id` uniqueness is also stated in `LFC-006`, creating a duplicate rule.

### 5.2 Correction

**Canonical LFC rule assignments (v1.5):**

| Rule | Constraint |
|---|---|
| LFC-001 | `event_id` MUST be globally unique within a lifecycle |
| LFC-002 | `lifecycle_id` MUST be present on every event |
| LFC-003 | All `related_events` references MUST resolve to existing events in the log |
| LFC-004 | Superseded or reversed events MUST reference the prior event |
| LFC-005 | Events are append-only; no event may be mutated after admission |
| LFC-006 | *(retired — duplicate of LFC-001)* |

**Code correction:** `pipeline.rs` and `event.rs` must be updated to use the correct LFC labels per the table above.

---

## Part 6 — SEM-013 Retirement

### 6.1 Defect identified

SEM-013 states: "Reversal events must reference prior events via `related_events`."

SEM-014 states: "An event with `State = FN` or `Finality = 1` cannot be contradicted by a later event unless the later event references the prior event in `related_events` and carries a Class that permits reversal or supersession."

SEM-013 is fully subsumed by SEM-014. SEM-014 is more general (covers supersession, not only reversal), more precise (specifies the `related_events` linkage and Class constraint), and applies to a broader trigger condition. SEM-013 adds nothing.

### 6.2 Correction

**Retire SEM-013.** Remove from Validation Rules v1.5. Add a note to SEM-014: "This rule supersedes the former SEM-013 (reversal linkage), which it fully subsumes."

**Rule count impact:** net −1 semantic rule.

---

## Part 7 — Finality: `FN+0` and the State × Finality Matrix

### 7.1 Background and decision

Earlier analysis concluded that Finality was potentially redundant — derivable from State. This RFC corrects that conclusion.

Finality and State answer **two different questions**:
- State: what is the *disposition* of this event? (pending, final, conditional, reversed, timelocked)
- Finality: is that disposition *finality-locked against protocol-level reversal*? (0 = not yet finality-locked; 1 = finality-locked)

A first-instance court judgment, rendered but subject to appeal, is not "pending" and not "conditional." It is *final-shaped* (a decision has been made) but *not finality-locked* (it can be overturned on appeal). No single State value can express this without either (a) splitting every State into "open" and "finality-locked" variants — doubling the vocabulary — or (b) using a separate binary axis. The separate binary is the minimization choice.

For this RFC, **locked** is not a free-standing primitive. When used in the State × Finality matrix, it means **finality-locked**: `Finality = 1`, no ordinary protocol-level reversal path remains, and the event is irreversible for Arrow validation purposes. This is distinct from economic, contractual, staking, escrow, custody, TVL, or time-lock conditions. Those conditions are represented by `State = TL`, by `finality_basis`, by registry metadata, or by extension fields as appropriate; they do not imply `Finality = 1`.

Design Principle 2 already states this explicitly: "event, policy, finality, and confidence are distinct layers, never collapsed." Keeping Finality is consistent with the protocol's own principles.

**The real gap was not the field — it was the undefined `FN+0` combination.** `FN` with `Finality = 0` is the canonical encoding of "settled but not yet finality-locked," but it was never defined. This RFC defines it.

### 7.2 State × Finality Matrix (normative)

This matrix is the single authoritative definition of which State/Finality combinations are valid and what each means.

| State | Finality 0 | Finality 1 |
|---|---|---|
| `PD` Pending | **Valid.** Event has been submitted and observed but is not yet included, settled, or decided. Transition: `PD+0 → FN+0` on inclusion. | **Forbidden.** A pending event cannot be finality-locked. SEM-002. |
| `CD` Conditional | **Valid.** Event is awaiting a future condition. The condition has not been met. Transition: `CD+0 → FN+0` or `FN+1` depending on the resolution mechanism. | **Forbidden.** An event awaiting a condition cannot be finality-locked until the condition resolves. SEM-004. |
| `FN` Final | **Valid — `FN+0` is "settled but challengeable."** A decision or inclusion has occurred; the disposition is real and may be acted on; but a reversal or challenge mechanism remains open. See §7.3. | **Valid — `FN+1` is "settled and finality-locked."** The disposition is irreversible for Arrow validation purposes. |
| `RV` Reversed | **Valid.** Reversal has been asserted but the reversal itself is challengeable or under appeal. | **Valid.** Reversal is final and irreversible. |
| `TL` Timelocked | **Valid (only valid combination).** A time condition controls when the event may progress; the event is not yet unlocked. SEM-003. | **Forbidden.** A timelocked event cannot be finality-locked before its time condition resolves. SEM-003. |

### 7.3 `FN+0` — the canonical encoding for delayed finality

`FN+0` represents the state of an event that has reached a *settled disposition* but for which a challenge, appeal, reorg, or reversal window remains open. It is the required encoding for:

- **Optimistic rollup outputs** — included and provisionally settled; the fraud-proof window is open.
- **Probabilistic chain outputs** — included but not yet past the reorg-safety confirmation threshold.
- **Legal orders under appeal** — rendered by the authority; operative and potentially enforceable; but the statutory appeal period has not elapsed.
- **Custodial outputs in clawback windows** — settled by the custodian; ACH return or contractual clawback window is open.

The protocol uses `FN+0` rather than `PD` because `PD` ("pending") means the event has not yet been *observed at all*. A Base transaction included in a batch is not pending — it is real, it exists, it may be acted on — but it has not reached irreversibility. `FN+0` is the only accurate encoding.

### 7.4 The `FN+0 → FN+1` transition

Because Arrow is append-only, `FN+0 → FN+1` is not a mutation. It is a **new event** appended to the lifecycle, asserting that the challenge or reversal window has elapsed without a successful challenge.

The transition event SHOULD be:
- Class 1 (OBSERVE) or Class 4 (POLICY), depending on whether it is an attestation of finality attainment or a protocol state update.
- `State = FN`, `Finality = 1`.
- `related_events` referencing the prior `FN+0` event.
- `proof_type` and `proof_ref` appropriate to the basis: a block height and timestamp for an optimistic window elapsed, a confirmation count for probabilistic, a jurisdiction-specific appeal-deadline record for legal.

The `finality_basis` field (Part 8) on the `FN+0` event provides the context that makes the transition interpretable.

### 7.5 Specification amendments

**Add to §2 (Definitions):** definition of `FN+0` as "settled but challengeable."

**Add §X — State × Finality Matrix:** the table in §7.2 above.

**Amend SEM-001 (currently "Finality 1 requires State FN or RV"):** extend to explicitly address `FN+0` as a valid and defined combination with a specified semantics.

**Amend the Finality field description** to state: "Finality is an independent axis from State. State describes the *disposition* of an event; Finality describes whether that disposition is *finality-locked against protocol-level reversal*. A `FN+0` event has reached a final disposition that is not yet irreversible. A `FN+1` event is irreversible for Arrow validation purposes."

---

## Part 8 — `finality_basis` (New Tier-3 Conditional Field)

### 8.1 Purpose

`FN+0` exists and is now defined. But `Finality = 0` alone does not tell a consumer *why* the event is not yet finality-locked — and the reason matters, because different bases have different expected resolution paths, durations, and transition triggers.

A `finality_basis` of `optimistic_window` tells a bridge: wait 7 days. A basis of `probabilistic` tells it: wait for N more confirmations. A basis of `legal_appeal` tells a compliance system: this order is operative but appealable; apply `HL` until the period lapses. Without this field, `FN+0` is underspecified for any automated consumer.

### 8.2 Field definition

| Attribute | Value |
|---|---|
| Field name | `finality_basis` |
| Required when | `Finality = 0` |
| Type | String — controlled vocabulary (§8.3) |
| Position | Tier-3 Event Object field (not in the canonical string) — same tier as `chain_id`, `bridge_id` |
| Validation rule | **FB-001** (new): If `Finality = 0`, `finality_basis` MUST be present and MUST be a registered value. |

`finality_basis` is a Tier-3 field: required when its trigger fires (`Finality = 0`), absent otherwise. It is never in the canonical string. It operates identically to `chain_id` — a flag in the code triggers a mandatory type in the Event Object.

### 8.3 Controlled vocabulary

| Code | Name | Typical resolution | `finality_eta` applicable? |
|---|---|---|---|
| `optimistic_window` | Optimistic Rollup Challenge Window | Challenge window elapses without a successful fraud proof | Yes — block height or timestamp of window close |
| `probabilistic` | Probabilistic Finality (reorg risk) | Sufficient confirmations accumulate to reach the deployment's reorg-safety threshold | Yes — estimated confirmation block |
| `legal_appeal` | Legal Order Under Appeal | Statutory appeal period elapses, or the order is upheld / overturned on appeal | Yes — jurisdiction-specific deadline timestamp, where deterministic |
| `custodial_clawback` | Custodial Clawback Window | Contractual or regulatory clawback period elapses | Yes — contractual clawback deadline timestamp |
| `staking_lock` | Staking / Yield-Bearing Lock | A staked, delegated, restaked, or yield-bearing position is locked against movement until an unstaking, withdrawal, or protocol release condition completes | Yes — unlock timestamp, epoch, or block height where deterministic |
| `vesting_schedule` | Vesting Schedule | A token, token fraction, or real-world asset claim is restricted by a vesting or release schedule before movement or settlement can finalize | Yes — next release timestamp, schedule milestone, or block height where deterministic |
| `protocol_pending` | Protocol-Defined Pending Condition | A protocol-level condition (e.g., threshold signature, governance vote) must resolve | Conditional — present if deadline is deterministic |

TVL is an aggregate measurement, not a per-event finality basis. Events involving TVL should encode the specific mechanism affecting the asset: `staking_lock` for staked or yield-bearing instruments, `vesting_schedule` for scheduled release rights, `custodial_clawback` for custody reversal windows, or another registered basis where the lock mechanism is different. A token that is trying to move while locked, or that will become locked once moved, should declare the basis that controls the movement constraint.

### 8.4 Optional companion field: `finality_eta`

`finality_eta` is an optional field that MAY accompany `finality_basis` when the resolution time is deterministic.

| Attribute | Value |
|---|---|
| Field name | `finality_eta` |
| Required when | Optional; present only when `finality_basis` has a deterministic end time |
| Type | Integer (Unix timestamp in seconds) or block height integer; MUST be qualified by `finality_eta_type` |
| Companion | `finality_eta_type`: `"unix_ts"`, `"block_count"`, or `"iso_date"` |

`finality_eta` is absent for `probabilistic` basis where no deterministic end can be stated. It is present for `optimistic_window` (the challenge window close is a deterministic block height) and `legal_appeal` where the appeal deadline is a fixed statutory date.

**`finality_eta` is derived from the Chain Registry or from the legal oracle's structured attestation — never from the submitter's assertion for chain-native events.** The derive-don't-assert rule (RFC-005) applies.

### 8.5 New rule: FB-001

| Rule | Severity | Constraint |
|---|---|---|
| **FB-001** | Fatal | If `Finality = 0`, `finality_basis` MUST be present and MUST be a registered value in the `finality_basis` vocabulary. Absence MUST be rejected. |

### 8.6 Chain Registry integration

The Chain Registry entry for each chain SHOULD include:

- `finality_model`: one of `{instant, probabilistic, optimistic_rollup, zk_rollup, bft_consensus}`
- `challenge_window_seconds`: for `optimistic_rollup` chains
- `reorg_safety_confirmations`: for `probabilistic` chains
- `finality_basis_default`: the `finality_basis` value an Origin Adapter should set for events from this chain with `Finality = 0`

This is the mechanism by which the challenge-window problem is solved without user input. The user transacts on Base. The Origin Adapter reads the Chain Registry: `Base → optimistic_rollup, 604800 seconds`. The adapter sets `Finality = 0`, `finality_basis = optimistic_window`, `finality_eta = (current_block_timestamp + 604800)`. The user is never asked; the data is never asserted by a human.

---

## Part 9 — LEG Minimization Pass

### 9.1 Derive-don't-store: flat `jurisdiction` field

The LEG amendment addendum v0.1 introduced `leg_jurisdiction` — a structured object with `country`, `supra`, and `local` fields — while retaining the flat `jurisdiction` string alongside it and adding **LEG-014** to police their consistency. This is the stored-and-policed anti-pattern applied to a derived value.

**§3.4 of the addendum already defines the derivation rule precisely:**

```
if leg_jurisdiction.country is present:
    jurisdiction = leg_jurisdiction.country
else if leg_jurisdiction.supra is present:
    jurisdiction = leg_jurisdiction.supra
```

**Correction:** the flat `jurisdiction` field, for LEG-origin events that carry `leg_jurisdiction`, MUST be treated as a **computed projection** by the validation pipeline, not as a separately stored value. The field continues to exist in the Event Object because LEG-002 and related rules read it, but its value is always derived — it is never authored independently when `leg_jurisdiction` is present.

**LEG-014 is retired.** A rule that enforces consistency between a stored value and the derivation that should have populated it is redundant once derivation is enforced. The validation pipeline derives the flat field; it does not check it.

**Net rule count impact:** −1 (LEG-014 retired).

### 9.2 Demote `leg_jurisdiction.local` to extensions

The addendum states explicitly: "Arrow validates only structure (`[A-Z]{2,4}` + `country` present). It does not maintain a canonical list of valid local codes. The LexCodex subnational registry is the authoritative source for semantic validation."

A field that Arrow only structure-checks and delegates all semantic validation to LexCodex is payload — it belongs in `extensions.legal_instrument`, not in the core Event Object. Its presence in the core object requires LEG-012 (structure check) and LEG-013 (LexCodex consistency, deployment-configurable). Moving it to extensions means those rules become extension-module rules, not core protocol rules.

**Correction:** `leg_jurisdiction.local` is moved from the required/conditional core `leg_jurisdiction` object to `extensions.legal_instrument.local_forum` (new optional field). The `leg_jurisdiction` object in the core Event Object retains only `country` and `supra`.

**Rules LEG-012 and LEG-013 are moved to the Compliance Module** (Step 14), where extension-block rules live. They are renamed `LEG-012-EXT` and `LEG-013-EXT` to signal their extension-block scope.

**Net protocol-core rule count impact:** −2 (from core semantic rules; moved, not deleted).

### 9.3 LEG-007 simplification

With `FN+0` properly defined in Part 7, the general rule "Finality=0 ⇒ non-executing policy" becomes protocol-wide (it is now a consequence of the State × Finality matrix and SEM-004 analog for `FN+0`). The LEG-007 derivation table currently re-encodes this general rule for every subtype row.

**Simplification:** LEG-007's derivation table is refactored. The `Finality=0` column for all subtypes collapses to a single protocol-wide rule: "`FN+0` events MUST NOT carry an executing Policy (`PR`, `TR`, `CL`) regardless of Origin." The LEG-007 table then specifies only the genuinely LEG-specific constraint: the subtype → permitted executing Policy set when `Finality = 1`.

The simplified LEG-007 table:

| `leg_subtype` | Permitted Policy when `Finality = 1` | Notes |
|---|---|---|
| `JUD` | `PR`, `BL`, `TR`, `CL` | Direct execution permitted for final judicial orders |
| `ADM` | `PR`, `BL`, `HL`, `CL` | `TR` excluded — administrative orders do not trigger autonomous execution |
| `SUPRA` (non-SA) | `PR`, `HL` | `TR`, `BL`, `CL` excluded by default; `BL` available only via LEG-008 |
| `SUPRA` + `reason_code = SA` | `BL` (forced by LEG-008) | Non-overridable |
| `LEG_TEXT` | `HL`, `RW` | The norm itself cannot authorize; must pair with `JUD`/`ADM` via `related_events` |
| `PREC` | `RW` | Informational only |

*All subtypes, `Finality = 0` → governed by the protocol-wide `FN+0` rule: `{HL, RW}` maximum.*

### 9.4 Legal oracle / authority split

Earlier RFC-003 and LEG amendment drafts defined `legal_oracle_structured` as a seventh verification regime and used `legal_attestation_panel` for two different ideas: the legal authority that issued an act, and independent corroboration of the proof. That overloaded court/arbitration/appellate panels with confidence semantics even though a panel primarily belongs to the legal authority and finality axis.

**Decision for v1.5.2:** split the terms. `legal_authority_panel` means a court, arbitration, appellate, regulatory, or equivalent legal-authority panel and is relevant to LEG authority/finality analysis only. The confidence axis uses legal-oracle attestation paths: `structured_legal_attestation` for single Sibyl/LBRY-style structured proof (ceiling B, not default) and `multiple_legal_attestation` for independent multi-Sibyl evidence (ceiling B for now). `legal_oracle_structured` is superseded by those terms. `legal_native_attestation` is reserved as draft experimental terminology for future direct legal-system-to-Sibyl binding and is not a live production path.

---

## Part 10 — Rule Taxonomy: Semantic vs Schema-Constraint

### 10.1 The conflation

Arrow Validation Rules v1.4 presents all rules under a unified numbering scheme, mixing two fundamentally different types:

**Semantic rules** enforce protocol-level meaning: "a `FN+1` event cannot be contradicted except by a reversal that references it" (SEM-014); "Confidence E may not authorize" (SEM-009); "a final judicial order may only produce executing policy when `Finality = 1`" (LEG-007). These require protocol-level reasoning.

**Schema-constraint rules** enforce vocabulary membership and structural formatting: "the Policy value must exist in the Policy registry" (MSG-004); "the `binding_method` string must exist in a registry" (SEM-018); "leg_subtype must be one of five values" (LEG-006); "`local` must match `[A-Z]{2,4}`" (LEG-012). These are JSON Schema validations. They require no protocol-level reasoning; they run identically for every field with a controlled vocabulary.

Presenting them as peers inflates the apparent rule count, obscures the genuinely semantic constraints, and creates maintenance duplication (every new controlled-vocabulary field generates a new "must be in registry" rule).

### 10.2 Correction

Validation Rules v1.5 separates the rule set into two explicitly labeled sections:

**Section A — Schema Constraints:** All vocabulary-membership and structural-format checks. These are the normative equivalent of a JSON Schema definition. They are referenced by rule ID for traceability but are grouped as a schema section, not interleaved with semantic rules. The MSG-* prefix is extended to cover all schema constraints.

**Section B — Semantic Rules:** All protocol-level constraints that require reasoning about the relationship between fields, events, or external state. These are the SEM-*, LFC-*, BRG-*, LEG-*, TRV-*, SAN-*, CMX-* rules — minus any entry that on inspection is purely a vocabulary check.

**Immediate impact on the committed rule set:**
- **SEM-018** ("binding_method must be in a registry") is a schema constraint, not a semantic rule. Reclassified as MSG-005 (or equivalent schema-constraint ID). Removed from the SEM series.
- **LEG-006** ("leg_subtype must be one of five values") is a schema constraint. Reclassified. (LEG-012 and LEG-013 were already moved to extensions in Part 9.)
- **LEG-011** ("`supra` must be a registered entry") is a schema constraint. Reclassified.

**Net semantic-rule count impact:** −3 from SEM/LEG series (reclassified, not deleted). Schema-constraint section gains the same entries.

---

## Part 11 — Registry `provenance` Block (Forward Hook)

### 11.1 The provenance gap

RFC-005 will establish the normative principle that derived values must be sourced to a verifiable origin, and that library-derived values (Chain Registry, Origin Registry, Policy Registry) must themselves be auditable. The groundwork for that principle is laid here by amending the registry schema to include a `provenance` block.

### 11.2 Registry schema amendment

All registry JSON files gain an optional (for now) `provenance` block per entry. Marking it optional in v1.5 lets the conceptual package land coherently while RFC-005 mandates `provenance` for all entries in the Chain Registry and Origin Registry that are used to *derive* protocol-level fields.

**Standard `provenance` block structure:**

```json
"provenance": {
  "source_type": "dev_docs | chain_rfc | audited_contract | whitepaper | regulatory_filing | manual_assessment",
  "source_ref": "https://docs.base.org/... OR content-hash:sha256:<hash>",
  "source_version": "as of 2026-05-25",
  "verified_by": "Arrow Registry Maintainer",
  "verified_at": "2026-05-25T00:00:00Z",
  "notes": "Optional. Flag if the entry's source is contested, stale, or requires periodic review."
}
```

**Chain Registry entries MUST include `provenance` in v1.5 for all fields that will be used by Origin Adapters to derive `finality_basis` and `finality_eta`.** These are the highest-risk entries because their values flow directly into Arrow events without submitter assertion.

---

## 12. Consistency Impact and Adoption Notes

Arrow Protocol is still in a conceptual and draft-validation phase. No production network of Arrow messages is being preserved by this RFC, and no grandfathering rule is introduced. These notes describe the conformance impact for implementers and repository maintainers adopting v1.5 as the coherence baseline.

| Change | Impact | Required update |
|---|---|---|
| Policy `RW` added to registry | No | None |
| `TR`/`CL` name corrections | No (code is canonical) | Name-map update if names were stored |
| `TL` restored to State registry | No | None |
| `HL`/`ER` removed from State registry | Corrective | Events with `State = HL` or `ER` were already non-conforming |
| State `finality_permitted` aligned with SEM-002 through SEM-004 | Corrective | `PD`, `CD`, and `TL` permit only `Finality = 0` |
| Origin parser tightened to `[A-Z]{3}` | Corrective | Implementations must reject non-conforming Origin values |
| `native_canonical` alias for `chain_native` | No | Alias documents the historical name while `native_canonical` is canonical |
| LFC label corrections | Corrective | Code corrections to `pipeline.rs`, `event.rs` |
| SEM-013 retired | No | Consumers of SEM-013 rule ID use SEM-014 |
| `FN+0` defined | Clarifying | No migration; existing `FN+0` events gain a definition |
| `finality_basis` added as required field when `Finality = 0` | Additive | All v1.5-conforming events with `Finality = 0` must include it |
| `leg_jurisdiction.local` moved to extensions | Corrective for LEG | LEG-origin events must relocate `local` to `extensions.legal_instrument.local_forum` |
| Flat `jurisdiction` derivation enforced | Corrective for LEG | LEG events that stored both independently must derive, not assert |
| Rule taxonomy separation | Organizational | Rule IDs preserved; organizational change only |
| Registry `provenance` block | Additive | Optional in v1.5; mandated by RFC-005 |

---

## 13. Promotion Sequence

Promotion to Arrow Protocol v1.5 and Validation Rules v1.5 follows this order to minimize dependency conflicts:

1. Registry corrections (Parts 1–4) — no code changes required, highest-priority coherence fix.
2. LFC label corrections (Part 5) — code corrections only, no spec change.
3. SEM-013 retirement (Part 6) — spec and rules only.
4. `FN+0` definition and matrix (Part 7) — spec amendment.
5. `finality_basis` field (Part 8) — spec, rules (FB-001), Chain Registry schema.
6. LEG minimization (Part 9) — spec, rules, extension-module relocation.
7. Rule taxonomy (Part 10) — rules document restructure.
8. Registry `provenance` block (Part 11) — registry schema, partial population.

---

## Document Information

| Field | Value |
|---|---|
| Document ID | ARROW-RFC-004 |
| Version | v1.0 |
| Status | Final — Normative |
| Genesis Anchor | RFC-000 — `2026-03-22T18:40:00Z` |
| Primary Specification | Arrow Protocol v1.5 |
| Supersedes | Nothing |
| Governance | Finalized by Editor for the v1.5 package |
