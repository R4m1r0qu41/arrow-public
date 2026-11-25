# Arrow Protocol

A stateful, append-only, machine-readable event standard for coordinating across blockchains, custodians, and legal systems — without requiring shared execution environments, shared consensus, or native interoperability between chains.

## Core Idea

Arrow does not move assets.
Arrow standardizes and routes verified events.

Arrow enables systems to agree on **what happened**, even when they cannot agree on **how it happened**.

## What Makes Arrow Different

Most blockchain interoperability standards treat legal systems as out of scope. Arrow treats legal systems as **first-class asserting domains** on equal footing with on-chain origins.

Every Origin in Arrow declares an explicit **verification regime** — the cryptographic, economic, or legal infrastructure that produces its proofs. Confidence claims are bounded by regime.

Arrow is a **stateful event coordination standard with embedded legal semantics and explicit verification regime binding.**

## Canonical Format

```
Class:Origin:State:Action:Finality:Confidence
```

Example:
```
2:CST:FN:PR:1:C
```

Six fields. Five delimiters. No whitespace. Machine-parseable. Human-readable. Unchanged since genesis.

## Quick Reference

| Field | Values | Meaning |
|---|---|---|
| Class | 1–6 | OBSERVE, INSTRUCT, CONDITION, CONTROL, TRANSFER, CLOSE |
| Origin | BTC, ETH, SOL, ARB, BSE, THR, CST, LEG, PCY, ORC, EMG | Asserting domain |
| State | FN, PD, CD, RV, TL | Final, Pending, Conditional, Reversed, Timelocked |
| Action | PR, BL, HL, RW, TR, CL | Proceed, Block, Hold, Review, Trigger, Closed |
| Finality | 0, 1 | Not finality-locked / Finality-locked |
| Confidence | A–E | Cryptographic proof → Unverified |

## Verification Regimes (v1.6.0)

| Regime | Trust Basis | Max Confidence |
|---|---|---|
| `native_canonical` | Origin's own canonical state (renamed from `chain_native`) | A |
| `optimistic_settled` | L1 settlement + challenge window | A (post) / B (pre) |
| `bonded_quorum` | Economic stake / bond | B |
| `native_notarized_consensus` | Source chain's own notarization mechanism | B |
| `institutional_attestation` | Regulated entity signature | C |
| `legal_attestation` | Manual legal attestation, court order, arbitration, or notarial proof | C |
| `structured_legal_attestation` | Single Sibyl/LBRY-style structured legal proof | B ceiling |
| `multiple_legal_attestation` | Independent multi-Sibyl legal evidence | B ceiling |
| `legal_native_attestation` | Direct legal-system integration with Sibyl or equivalent infrastructure | draft / reserved |
| `bonded_quorum` + `ORC` | Oracle network or bonded attestor set | B |

## Repository Structure

```
/
├── README.md                         ← Project overview and current orientation
├── LICENSE                           ← Apache-2.0 license
├── CHANGELOG.md                      ← Version and change history
├── VERSION                           ← Current repository/protocol version marker
├── Cargo.toml                        ← Rust workspace manifest
├── Cargo.lock                        ← Rust dependency lockfile
│
├── crates/                           ← Rust implementation workspace
│   └── ...                           ← Protocol crates and implementation modules
│
├── docs/                             ← Canonical and supporting documentation
│   ├── normative/                    ← Active-baseline manifest for protocol docs
│   ├── protocol/                     ← Protocol specification and validation rules
│   ├── ui/                           ← Human-facing reference material / cheatsheets
│   └── ...                           ← Supporting documents and working notes
│
├── registries/                       ← Machine-readable protocol registries (v1.6.0)
│   ├── index.json                    ← Registry manifest
│   ├── origins.json                  ← Origin codes and verification-regime binding
│   ├── classes.json                  ← Class codes
│   ├── states.json                   ← State codes
│   ├── actions.json                  ← Action codes (field 4; final name as of v1.6)
│   ├── control_subtypes.json         ← Class 4 CONTROL anchor vocabulary (v1.6, draft)
│   ├── confidence.json               ← Confidence codes and protocol floor
│   ├── verification_regimes.json     ← Verification regimes (v1.5: native_canonical)
│   ├── chain_registry.json           ← Per-chain trust metadata (v2.0 schema)
│   ├── finality_basis.json           ← Finality basis vocabulary (v1.5, FB-001)
│   ├── sources.json                  ← Observation source typing
│   ├── reason_codes.json             ← Reason codes
│   ├── object_types.json             ← Object types
│   ├── subject_types.json            ← Subject types
│   ├── proof_types.json              ← Proof types
│   ├── compliance_extensions.json    ← Compliance extension vocabularies
│   └── reserved_tokens.json          ← Rule-family/cross-repo token reservation (v1.6, NAM-002)
│
├── rfc/                              ← Request for Comments and reference cases
│   ├── RFC-000.md                    ← Genesis record and authorship trail
│   ├── RFC-000-canonical.txt         ← Canonical text block (SHA-256 anchored)
│   ├── RFC-000-canonical.txt.ots     ← OpenTimestamps Bitcoin receipt
│   ├── RFC-001-KelpDAO.md            ← Reference case: KelpDAO recovery
│   ├── RFC-002-Verus-SemanticTypeConfusion-v1.md
│   ├── RFC-003-Arrow-as-Interoperable-State-Machine-v1.md
│   ├── RFC-004-Coherence-and-Minimization-v1.md
│   ├── RFC-005-Provenance-and-Trustless-Sourcing-v1.md
│   ├── RFC-006-Robust-Chain-Registry-v1.md       ← Graduated 2026-06-16
│   └── RFC-007-Declared-Policy-and-Integration-Strength-v1.md  ← Published 2026-06-17
│
└── tools/                            ← Repository tooling and helper scripts
```

## Status

**Baseline: `arrow-v1.6.0`, live**

Following the June 2026 soft reboot (ADR-ARRW-001), Arrow no longer tracks
numbered document versions. Active documents use stable filenames and evolve
through Git history; each is classified `live` or `superseded`, not assigned
a rolling version number. `arrow-v1.6.0` remains a legitimate protocol
release tag (BASE-001) — it names the baseline the soft reboot was performed
on, not a counter this README increments going forward.

The active protocol-document baseline is indexed in
`docs/normative/manifest.json`, which lists current (`live`) documents only.
Superseded protocol packages are retired from this tree. Tooling uses the
manifest, not versioned filenames. Full change history, including prior numbered
releases, is in [`CHANGELOG.md`](./CHANGELOG.md).

| Document / Layer | Status |
|---|---|
| Arrow Protocol v1.6.0 | Live protocol package — `docs/protocol/arrow-protocol.md`; field 4 is `Action`, Class 4 is `CONTROL` with draft anchor vocabulary, SEM-020 closure binding, NAM-001 naming lint, rule-ID permanence |
| Validation Rules v1.6.0 | Live validation-rule package — `docs/protocol/validation-rules.md`; `ACT-001/002` (retiring `CTL-001/002`), SEM-020 added; `TRG-001`, `LFC-007`, `LFC-008` live (RFC-010, 2026-07-07); SEM-001/002/003/004/007 now enforced in `validate_semantic`, locked by an exhaustive six-field enumeration harness (`cargo test -p arrow-core --test enumeration`) |
| Registries package | Live manifest at `1.6.0`; field 4 is `Action`, Class 4 is `CONTROL`, `control_subtypes.json` is a draft anchor vocabulary, Chain Registry schema remains `2.0`, and `reserved_tokens.json` (NAM-002) is now indexed |
| RFC-000 Genesis Record | Finalized — anchored on Bitcoin and Ethereum |
| RFC-001 KelpDAO Reference Case | Reference Case — finalized |
| RFC-002 Verus-Ethereum Bridge Exploit | Reference Case — finalized |
| RFC-003 Arrow as an Interoperable State Machine | Final — `rfc/RFC-003-Arrow-as-Interoperable-State-Machine-v1.md` |
| RFC-004 Coherence and Minimization | Final normative — `rfc/RFC-004-Coherence-and-Minimization-v1.md` |
| RFC-005 Provenance, Trustless Sourcing, and Adapter Interchange | v1.1 draft amendment — `rfc/RFC-005-Provenance-and-Trustless-Sourcing-v1.md` |
| RFC-006 Robust Chain Registry | **Graduated** (2026-06-16; published v1.0) — `rfc/RFC-006-Robust-Chain-Registry-v1.md` |
| RFC-007 Declared Policy, Recognized Authority, Integration Strength | **Published** (v1.0, 2026-06-17) — specification graduated; `DPL-*`/`INT-*` enforcement deferred by design — `rfc/RFC-007-Declared-Policy-and-Integration-Strength-v1.md` |
| RFC-008 Semantic Disentanglement and Clarification | Draft — Part 1 namespace disentanglement (executed v1.6.0), Part 2 Action-value clarification (CL via SEM-020; TR via TRG-001), Part 3 open work — TRG-001 and LFC-007 items now implemented via RFC-010 (remainder still open), Part 4 Class 5 TRANSFER constraints. Supersedes RFC-008 v0.2 and folds in RFC-009 v0.2, whose supersession stub is archived at `archive/rfc/RFC-009-tr-cl-action-semantics-v0.2.md`. |
| RFC-010 Lifecycle Gates and Naming Reservation | **Ratified v0.3, implemented** (2026-07-07) — Parts 1, 2, 2A, 3 live on Arrow main: TRG-001, LFC-007, LFC-008, NAM-002. Part 4 lands in Tarquin specs; LC-016 consumer-side staleness computation remains Tarquin implementation work. |
| Rust Reference Implementation | In progress — `crates/arrow-core` |
| Governance Framework | Planned — v2.0 |

## Recent Changes

Per-release change narratives (what changed layer-by-layer, including the
prior `v1.5-draft` → `v1.6.0` progression) live in
[`CHANGELOG.md`](./CHANGELOG.md), not in this README. This keeps the README
describing current state only, consistent with the soft-reboot liveness
model (`live`/`superseded`, no rolling version headers).

## On-Chain Anchors

The genesis record (RFC-000) is anchored on two independent blockchains:

| Anchor | Record |
|---|---|
| Bitcoin Ordinal | `19b4b9a1813b3faf32747ff5974a8d50ce63dbbe85c2507e08ac410d1f17e5b9i0` |
| Ethereum Calldata | `0xfc5f442feb8e6352043edc3afd533592a0f3612320b3084220e13d8e6f636ba8` |
| OpenTimestamps | `rfc/RFC-000-canonical.txt.ots` |

## License

Apache 2.0 — see [LICENSE](./LICENSE)
