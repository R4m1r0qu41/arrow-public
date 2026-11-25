# Arrow Protocol — Integration Test Fixtures

This directory contains JSON fixtures used by the data-driven regression suite
in `crates/arrow-core/tests/fixtures.rs`.

## Running the suite

```bash
cargo test -p arrow-core --test fixtures
```

## Structure

| File | Purpose |
|------|---------|
| `index.json` | Master manifest — lists all fixtures and expected outcomes |
| `F-NN-<slug>.json` | Individual `ArrowEvent` JSON files |

## `index.json` schema

```jsonc
{
  "version": "1.0",
  "fixtures": [
    {
      "id": "F-01",                       // short ID used in failure messages
      "file": "F-01-pass-foo.json",       // filename in this directory
      "description": "human-readable",
      "expected_result": "pass",          // "pass" | "reject"
      "expected_violations": ["BRG-001"], // rule IDs that must appear (may be [])
      "expected_reason_code": "BRIDGE_TRUST_REVOKED"  // optional substring in violation message
    }
  ]
}
```

## Fixture authoring contract

1. **Every fixture must be a valid `ArrowEvent` JSON** — deserializable by the
   `ArrowEvent` struct in `crates/arrow-core/src/types/event.rs`.

2. **Required fields** — the following fields have no `#[serde(default)]` and
   must be present in every fixture:
   - `arrow_code` — canonical Arrow message string, e.g. `"1:ARB:FN:PR:1:A"`
   - `event_id` — globally unique identifier
   - `lifecycle_id` — lifecycle group identifier
   - `timestamp` — RFC 3339 timestamp
   - `object_type` — type of the object being reported on
   - `object_id` — identifier of the object

3. **Arrow code must use registered values** — all six fields (class, origin,
   state, action, finality, confidence) must appear in the embedded registries.
   Unregistered origins trigger `ORG-003` and stop further validation.

4. **Bridge events require `proof_semantics`** — any event with `bridge_id`
   pointing to a bridge whose `verification_regime` is not `native_canonical`
   or `optimistic_settled` must include a `proof_semantics` block or `SEM-017`
   fires.

5. **Finality 0 requires `finality_basis`** — events with finality = 0 must
   supply a registered `finality_basis` value or `FB-001` fires.

6. **LEG-origin events require `leg_subtype`** — events with `origin = LEG`
   must supply a `leg_subtype` from the controlled vocabulary
   (`JUD`, `ADM`, `SUPRA`, `LEG_TEXT`, `PREC`) or `LEG-006` fires.

7. **Use registered bridges only** — `bridge_id` must reference an entry in
   `registries/bridge_registry.json`. Unknown bridge IDs trigger `BRG-000`.

8. **`subject_id` and `subject_type` are flat fields** — the struct has
   `subject_id: Option<String>` and `subject_type: Option<String>` at the top
   level, not a nested `subject` object.

## Current fixtures

| ID | Arrow code | Origin | Expected | Key rule(s) exercised |
|----|------------|--------|----------|-----------------------|
| F-01 | `1:ARB:FN:PR:1:A` | ARB | pass | SEM-015 ceiling A (optimistic_settled) |
| F-02 | `1:THR:PD:HL:0:C` | THR | pass | BRG-001 not triggered (trust_flag=1); SEM-017 satisfied at C |
| F-03 | `2:LEG:PD:HL:0:C` | LEG | pass | LEG-006 satisfied; FB-001 satisfied |
| F-04 | `1:ETH:FN:PR:1:A` | ETH | pass | SEM-015 ceiling A (native_canonical) |
| F-05 | `5:THR:FN:PR:1:B` | THR | reject | BRG-001 (VRSC-ETH-001 trust_flag=0) |
| F-06 | `2:ETH:FN:PR:1:A` | ETH | reject | SEM-017 (classification_verified=false, Confidence A > C cap) |
