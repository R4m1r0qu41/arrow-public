# Exhaustive Enumeration — Total-Truth Testing of the Message Surface

Status: live
Test: `crates/arrow-core/tests/enumeration.rs` — run with
`cargo test -p arrow-core --test enumeration`

## What it is

Arrow's canonical message is six fields drawn from finite registries:

```text
Class : Origin : State : Action : Finality : Confidence
  6   ×   12   ×   5   ×   6    ×    2     ×     5      = 21,600 messages
```

Because the space is finite and small, we do not have to *sample* it — we
run **every possible message** through the real validator
(`ValidationPipeline::validate_message`) and assert the spec's message-level
MUSTs as **total truths**. "No executing Action ever passes with Finality 0"
is not a test case here; it is a theorem checked against all 21,600 inputs
on every test run. The suite finishes in well under a second.

This changes the epistemic status of the claim "the validator is
semantically sound at the message level" from a judgment (we wrote good
fixtures) to a mechanical fact (no counterexample exists in the entire
space).

## The theorems

Each theorem restates one spec sentence from
`docs/protocol/validation-rules.md` (Semantic Constraints table). For every
tuple that PASSES validation:

| Theorem | Spec rule | Total truth asserted |
|---|---|---|
| T1 | SEM-013 | Never Finality 0 with an executing Action (PR/TR/CL) |
| T2 | SEM-015 | Confidence never exceeds the regime's effective ceiling — the pre-settlement ceiling (`pre_settlement_max_confidence`) at Finality 0, the full ceiling at Finality 1 |
| T3 | SEM-005+012+013 | A passing Class 6 is exactly `6:*:FN:CL:1:*` (the derived closure shape) |
| T4 | SEM-020 | Action CL never passes outside Class 6 |
| T5 | SEM-006 | Class 3 passes only with BL/HL/RW |
| T6 | SEM-007 | Class 5 passes only with TR/PR |
| T7 | SEM-009 | Confidence E never passes with PR/TR |
| T8 | SEM-001 | Finality 1 passes only in State FN or RV |
| T9 | SEM-002/003/004 | PD/TL/CD never pass at Finality 1 |
| — | structural | Every rejected tuple carries at least one Error-severity violation (rejections are always explained) |

Two design rules keep the harness honest:

1. **The oracle is independent of the implementation.** Each theorem is
   restated from the spec using only registry data and a locally defined
   confidence order. The test never calls validator internals (e.g. its
   `confidence_rank`) to decide what *should* happen — otherwise a shared
   bug would satisfy itself.
2. **The space is derived from the registries, not hardcoded.** The loops
   enumerate whatever `classes.json`, `origins.json`, `states.json`,
   `actions.json`, and `confidence.json` contain. Add an origin and the
   space grows to 23,400 automatically; the count assertion keeps the
   harness from silently under-covering.

## What it caught on day one

Building the theorems exposed that five spec MUSTs had **no enforcement
code at all**: SEM-001, SEM-002, SEM-003, SEM-004, and SEM-007. Messages
like `5:ETH:FN:BL:1:A` (a "transfer" that blocks) and `1:ETH:PD:HL:1:A`
(pending-but-final) validated clean until 2026-07-06. Fixture review and a
cross-layer coherence audit had both missed them — sampled checks verify
the cases someone thought to write; enumeration verifies the cases nobody
thought of. That asymmetry is the entire argument for this harness.

## What it deliberately does NOT cover

The six-field space is the *message* surface. Rules keyed on event or
context fields are outside this harness and are covered by fixtures
(`tests/fixtures/`) and unit tests instead:

- FB-001 (`finality_basis` / `finality_eta`), incl. the SEM-015 event
  extension (settled-claim eta contradiction)
- The LEG family (MSG-006, LEG-004…LEG-010) — `leg_subtype`,
  `leg_jurisdiction`, `reason_code`
- Bridge rules (BRG-*, SEM-017/019, MSG-005) — `bridge_id`,
  `proof_semantics`, `economic_consistency`
- Chain rules (SEM-016, CHN-001) — `chain_id`
- Lifecycle rules (LFC-*, SEM-014) — `related_events`, `LifecycleContext`
- SEM-008 (Class 2 SHOULD use PR) — a SHOULD is not a total truth; it is
  deliberately absent, not forgotten.

Do not present the enumeration suite as full-rule coverage; it is
full-**message-surface** coverage. Extending total-truth treatment to the
conditional event fields is a separate, larger effort (a constrained
event-field grid per rule family), tracked in the formalization roadmap.

## Maintenance rules

- **New six-field semantic rule ⇒ new theorem.** If a rule constrains only
  the six fields (e.g. a future TRG-001 class constraint: no passing TR
  outside Classes {2,5}), add it as a theorem here in the same change —
  the harness must never lag the rule set.
- **Registry change ⇒ no action needed** (space is derived), but check the
  printed tuple count moved as expected.
- A theorem failure is a **protocol defect, not a test to fix**: either the
  validator misses an obligation or the spec sentence is wrong. Surface it;
  never weaken the theorem to green the build.
