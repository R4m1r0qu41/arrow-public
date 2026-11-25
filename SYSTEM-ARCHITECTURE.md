# CipherLayer Three-Repository Architecture

Status: Informative orientation, not a normative protocol document.

Arrow, Orlog, and Tarquin are one ordered system with separate authority
boundaries.

```text
Orlog deterministic SLC artifacts
  -> Tarquin legal runtime, Sibyl/LBRY checks, and LEG adapter/consumer
  -> Arrow LEG events and Arrow-compatible consumers
```

Authority does not follow that dataflow exactly:

- Arrow owns protocol semantics, canonical event format, registries, validation
  rules, transport semantics, and the Rust reference implementation.
- Orlog owns deterministic agreement artifacts: taxonomy, law links, SLC
  models, template text, TypeScript logic, fixtures, and structural validation.
- Tarquin owns the legal runtime boundary: pinned Orlog imports, Lex.Codex /
  Lex.exe interpretation, Sibyl/LBRY attestation handling, fail-closed rule
  execution, and Arrow LEG event emission/consumption.

Current compatibility baseline:

- Arrow protocol baseline: `arrow-v1.6.0`.
- Active Arrow protocol-document index: `docs/normative/manifest.json`.
- Tarquin declares Arrow compatibility in `arrow-compatibility.json` and pins
  Arrow rule-surface input in `imports/arrow/PIN.json`.
- Orlog declares stack/Arrow compatibility in `arrow-compatibility.json`; it is
  not an Arrow event emitter.

Primary machine-readable controls:

- Arrow validates registries, `exports/rule-surface.v1.6.0.json`, and
  `docs/normative/manifest.json` with `node tools/validate-registries.js`.
- Orlog validates SLC artifacts and its Arrow compatibility declaration with
  `npm run check`.
- Tarquin validates the Arrow rule-surface pin with
  `cargo test -p tarquin-lex-exe --test arrow_rule_surface_pin` and validates
  cross-target rule parity with `bash tests/parity/run.sh`.
- Arrow `tools/check-three-repo-baseline.sh --fast` runs the current local
  three-repo baseline check.

Governance:

- Arrow owns the governing liveness/version and protocol-authority policy. The
  governing ADR is Arrow `docs/architecture/decisions/ADR-ARRW-001`, which
  supersedes the stack-governing scope of the prior Tarquin ADR-0002 (preserved
  as history). Tarquin and Orlog record adoption in `ADR-TARQ-001` /
  `ADR-ORLG-001`.
- Orlog's active rename from Contract Labs is complete: package name, paths,
  pins, and metadata use `Orlog`; the former name is retained only in historical
  provenance and former-name disclosures.

Unresolved governance boundaries:

- Tarquin cross-repo CI still needs a sibling Arrow checkout strategy pinned to
  the intended Arrow ref.
