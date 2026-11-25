# RFC-006 — The Robust Chain Registry

| Field | Value |
|---|---|
| Document ID | ARROW-RFC-006 |
| Version | v1.0 |
| Status | **Graduated 2026-06-16; published as v1.0 on 2026-06-17** — see §0 |
| Date | 2026-05-25 |
| Updated | 2026-06-17 |
| Track | Infrastructure / Registry Governance |
| Author | Arrow Protocol — Editor |
| Depends on | RFC-004 v1.0 (registry provenance schema); RFC-005 v1.0 (provenance obligations PRV-003); R-005 Update Governance Addendum v0.2 (ratified); Validation Rules v1.6.0 (CHN-008 mainnet activation-gate lint); Arrow Protocol v1.5+ |
| Targets | Chain Registry v2.0 |
| Genesis Anchor | RFC-000 — `2026-03-22T18:40:00Z` |
| Supersedes | Arrow Registries v1.x — `registries/chain_registry.json` (current ad hoc file) |
| Informs | Future RFC — Origin Adapter Registry; future RFC — multi-source Confidence upgrade |

---

## 0. Status of This Document

**RFC-006 is graduated, effective 2026-06-16.** It defines the problem, the requirements, and the target architecture for the Chain Registry as a first-class, governed, sourced artifact. v0.3 added independent on-chain lifecycle test requirements and periodic state-verification anchoring. v0.4 recorded the graduation blockers below and the resolved §7 `POL` namespace correction. v0.5 discharged the governance blockers — the §5.5 emergency update track by reference to the **R-005 Update Governance Addendum** (`rfc/R-005-Update-Governance-RFC-006-Addendum-v0.2.md`, **ratified 2026-06-16**), and §6 anchoring committed normatively to **OpenTimestamps**. The remaining evidentiary blocker was discharged the same day: the on-chain lifecycle-test backfill is complete for all five active entries, and both the testnet and mainnet registry snapshots are Bitcoin-anchored via OpenTimestamps. The three editorial gates raised by the 2026-06-16 graduation-readiness review (§4.1 `onchain_anchor` example, `challenge_window_source` schema drift, dependency re-point to Validation Rules v1.6.0) are closed in this revision — see §0.1. Graduation lifts RFC-007's §1 dependency hold on its advisory layer.

RFC-006 is downstream of RFC-005, which mandates per-entry `provenance` blocks for registry entries used to derive Provenance Class II fields. RFC-006 operationalizes that mandate specifically for the Chain Registry.

### 0.1 Graduation blockers (discharged)

The Chain Registry v2.0 schema is already live and committed (`registries/chain_registry.json`, `schema_version: 2.0`, with ETH/BTC/SOL/BASE/ARB populated). RFC-006 was held at draft until the following were discharged. These were deliberate holds, not oversights:

1. **~~Lifecycle-test backfill (R-006, R-009, R-010, R-011).~~ DISCHARGED 2026-06-16.** All five active v2.0 entries (ETH, BTC, SOL, BASE, ARB) now carry a populated `lifecycle_test` block bound to a real mainnet chain-native `tx_ref`, a `state_test_cadence` (`30d`), and a reproducible `extraction_artifact_hash`. The five-chain mainnet lifecycle set was executed and recorded in `docs/test/ON-CHAIN-LIFECYCLE-EVIDENCE-LOG.md`, and the mainnet registry snapshot is Bitcoin-anchored via OpenTimestamps (block `953889`). The data satisfies R-006/R-009/R-010/R-011 by direct on-chain evidence, and this is now **machine-enforced**: **CHN-008** (build-time registry lint, `tools/validate-registries.js`) requires every `active` mainnet entry to carry a non-null `lifecycle_test`. A chain therefore cannot be promoted to `active` without a mainnet on-chain test — it stays `proposed` (staged) until then; testnet/devnet evidence is optional pre-flight.
2. **~~Emergency update track (§5.5) and anchoring implementation (§6) are placeholders.~~ DISCHARGED in v0.5.** The defined update process and its emergency sub-track (R-005) are now specified in full by the **R-005 Update Governance Addendum** (RCP intake, change classes, CI reproducibility gate, evidence requirements, and the `E1` emergency track with a 48-hour confirmatory deadline and safe-state fallback). §5.5 incorporates that addendum by reference; the addendum was **ratified 2026-06-16** and is normative, so §5.5's MUST-level requirements no longer rest on an unratified draft. R-004 anchoring is committed normatively in §6 to OpenTimestamps (Bitcoin commitment, following the RFC-000 genesis precedent), explicitly provisional and supersedable by a later §6 revision; both the testnet and mainnet registry snapshots are Bitcoin-confirmed under that mechanism.
3. **`POL → Polygon` namespace collision — discharged in v1.5.2-draft (see §7).** Freeing the `POL` code for Polygon required renaming the committed `governance` origin and migrating ratified references. v1.5.2-draft completed that rename by moving the governance/Policy origin to `PCY`, preserving its `bonded_quorum` semantics. Polygon's Chain Registry entry remains TBD until a later registry patch adds the full sourced entry.
4. **Editorial gates from the 2026-06-16 graduation-readiness review — DISCHARGED in this revision.** The §4.1 example showed `anchoring.onchain_anchor` populated, contradicting the normative R-004 preimage rule (which requires it to remain `null` in-file); the example now shows `null` with a pointer to R-004. The §4.1 example also carried `protocol_required.challenge_window_source`, a field absent from every live entry; it has been removed from the example. The dependency declaration now names **Validation Rules v1.6.0**, the version `CHN-008` actually lives in.

---

## 1. Purpose

The Arrow Chain Registry (`registries/chain_registry.json`) currently exists as a manually-maintained JSON file. It is the source of:

- `finality_model` — whether a chain is probabilistic, optimistic, ZK, or instant-final.
- `challenge_window_seconds` — the fraud-proof window for optimistic rollups.
- `reorg_safety_confirmations` — the confirmation depth for probabilistic finality.
- `chain_trust_flag` — the protocol's assessment of whether a chain is trusted for high-Confidence events.
- `finality_basis_default` — the `finality_basis` value Origin Adapters derive for new events.

In RFC-004 and RFC-005, these fields became **load-bearing**: Origin Adapters derive `Finality`, `finality_basis`, and `finality_eta` from Chain Registry entries without submitter input. A stale entry produces incorrect field values in every event minted from it. A poisoned entry produces incorrect field values in every event minted from it, at whatever scale that Origin Adapter is deployed.

The current ad hoc file is not adequate infrastructure for this responsibility.

RFC-006 specifies the Chain Registry as **governed, sourced, versioned, and tamper-evident** infrastructure.

---

## 2. The Problem in Concrete Terms

### 2.1 Scale of dependency

Arrow Protocol v1.5+ requires Origin Adapters to derive challenge-window duration, finality model, and reorg depth from the Chain Registry for every chain-native event they mint. The protocol then **rejects** submitter assertions of these values for chain-native events (PRV-001). The Chain Registry is not optional infrastructure — it is the derivation source the protocol depends on.

### 2.2 The attack surface of a manually-maintained file

A static JSON file maintained by an editor has no:
- Per-entry provenance (where did this value come from?).
- Version audit trail (what changed, when, and why?).
- Tamper-evidence (how do I know the file I am reading is the one the protocol published?).
- Update process (who can modify an entry, by what process, with what review?).
- Staleness detection (challenge-window durations can change with protocol upgrades — how do consumers know when an entry is outdated?).

An attacker with access to the repository can change `challenge_window_seconds` for Base from `604800` (7 days) to `0` (instant) and, if that change goes undetected, every Origin Adapter reading the registry will mint events with `Finality = 1` for Base transactions that are still within the challenge window. The protocol's security guarantee has been silently degraded.

### 2.3 The staleness problem

Chain protocol parameters change. Optimism's fraud proof architecture has evolved; challenge-window durations have been modified by governance votes; new chains are deployed; old chains are deprecated or forked. A registry that does not track when entries were last verified against source is a registry that accumulates silent technical debt. The `verified_at` timestamp in the `provenance` block (RFC-004 §11.2) is the minimum staleness signal; RFC-006 adds an active review cadence.

---

## 3. Requirements

The Robust Chain Registry must satisfy the following requirements:

**R-001 — Per-entry provenance.** Every entry MUST carry a `provenance` block (RFC-004 schema) identifying the source of each technical parameter value: the chain's official documentation, its protocol RFC, its audited contracts, or a formal governance decision.

**R-002 — Sourced to verifiable originals.** Source references MUST point to verifiable original documents. Acceptable sources: the chain's own published protocol documentation, the chain's GitHub repository at a specific commit, the chain's governance forum vote, a deployed and verified smart contract. Unacceptable sources: secondary aggregators, unverified community wikis, "manual assessment."

**R-003 — Version control with audit trail.** The registry MUST be versioned with a full change history: what changed, when, who proposed, who approved, what source justified the change.

**R-004 — Tamper-evident anchoring.** Each published version of the registry MUST have its canonical hash anchored on at least one public blockchain. Origin Adapters that pin a registry version SHOULD verify the pinned hash against the on-chain anchor before using the registry for derivation.

**Canonical-hash preimage (normative).** The *registry version hash* is `SHA-256(JCS(registry))` and an *entry hash* is `SHA-256(JCS(entry))`, where `JCS` is RFC 8785 JSON Canonicalization (the same canonicalization used for the R-011 extraction-artifact hash). To make these a stable fixed point, the preimage MUST be computed with every `anchoring.entry_hash` and `anchoring.registry_version_hash` field set to JSON `null`; `anchoring.onchain_anchor` is included as-is. A verifier reproduces any stored hash by nulling those two fields and recomputing. Consequently `anchoring.onchain_anchor` MUST remain `null` in the anchored file: writing an in-file anchor reference would change the very bytes the anchor commits to and invalidate both the proof and the stored hashes. The anchor commitment (the OpenTimestamps proof and any supplemental transaction reference) MUST instead be retained out-of-file — in the sidecar `.ots` proof and the on-chain lifecycle evidence log. Recording an in-file anchor reference is permitted only by minting a new registry version, which is then re-hashed and re-anchored.

**R-005 — Defined update process.** Registry entries MUST be updated via a defined process: proposal, source citation, review period, ratification. Emergency updates (chain fork, critical parameter change) have an expedited track with post-ratification review. The full process — Registry Change Proposal (RCP) intake, the `C1`–`C4` change classes, the CI reproducibility gate, evidence requirements, and the `E1` emergency track — is specified in the **R-005 Update Governance Addendum** (`rfc/R-005-Update-Governance-RFC-006-Addendum-v0.2.md`), incorporated here by reference.

**R-006 — Staleness signals.** Each entry MUST carry a `review_cadence` field specifying the maximum interval between active re-verifications. Entries that have not been re-verified within their cadence MUST be marked `status: stale` and MUST trigger a registry staleness warning when used for derivation.

For protocol-required fields used by Origin Adapters, `review_cadence` covers source-document review and `state_test_cadence` covers live on-chain lifecycle verification. The longest permitted state-test cadence in this draft is monthly.

**R-007 — Separation of protocol-required and advisory fields.** Registry entries MUST distinguish between fields required for Arrow derivation (protocol-required) and fields that are informational or advisory. Consumers must be able to read only the protocol-required fields without parsing the full entry.

**R-008 — Chain Registry index on-chain.** The registry's version history (version number → on-chain anchor hash → publication timestamp) SHOULD itself be published on-chain, so that any party can verify which version was current at any given block.

**R-009 — Independent on-chain lifecycle test.** A new or materially changed registry entry MUST be accompanied by a fully on-chain lifecycle test transaction performed independently by an Arrow Registry Maintainer, Protocol Editor, or delegated reviewer. The test MAY be a minimal-value transfer, such as a single base unit or a fractional token amount (for example `0.000000001 ETH`) where economically safe. The observed transaction data MUST map to the submitted `protocol_required` fields in §3 / §4.1, including chain identity, finality model, finality basis, and any deterministic finality timing.

**R-010 — Periodic anchored re-verification.** Each active entry MUST define a state-verification cadence appropriate to its risk and volatility. The cadence MAY be hourly, daily, weekly, or monthly, but MUST NOT be longer than monthly unless a future RFC creates an explicit exception process. Each periodic verification SHOULD produce an on-chain or otherwise tamper-evident anchor for the test result.

**R-011 — Arrow-native extraction artifact.** Lifecycle tests MUST produce a standardized, tamper-evident extraction artifact that records the relevant transaction fields in an Arrow-native verification format. The artifact MUST be reproducible from the public transaction data, bind to the chain-native transaction reference (`tx_ref`) and registry entry version, and be usable as a second source of truth or corroborating attestation where multiple attestations are required.

---

## 4. Chain Registry Entry Schema (v2.0)

### 4.1 Minimal required structure

```json
{
  "code": "BASE",
  "name": "Base",
  "registry_version": "2.0",
  "entry_version": "1.0",
  "status": "active",

  "protocol_required": {
    "finality_model": "optimistic_rollup",
    "finality_basis_default": "optimistic_window",
    "challenge_window_seconds": 604800,
    "reorg_safety_confirmations": null,
    "chain_trust_flag": 1,
    "layer": 2,
    "settlement_chain": "ETH"
  },

  "advisory": {
    "description": "Base is an Ethereum Layer 2 optimistic rollup built and operated by Coinbase, using the OP Stack.",
    "consensus": "Optimistic rollup — transactions are presumed valid; fraud proofs may challenge within the window.",
    "native_asset": "ETH",
    "mainnet_genesis": "2023-07-13",
    "evm_compatible": true,
    "rpc_providers": ["https://mainnet.base.org", "https://base.llamarpc.com"]
  },

  "provenance": {
    "source_type": "dev_docs",
    "source_ref": "https://docs.base.org/docs/base-contracts",
    "source_secondary": "https://github.com/ethereum-optimism/optimism/blob/develop/specs/fault-proof/stage-one/fault-dispute-game.md",
    "source_version": "as of 2026-05-25",
    "verified_by": "Arrow Registry Maintainer",
    "verified_at": "2026-05-25T00:00:00Z",
    "review_cadence": "90d",
    "state_test_cadence": "30d",
    "next_review_due": "2026-08-23",
    "notes": "Challenge window duration is a governance parameter on the OP Stack. Monitor Optimism governance for changes."
  },

  "lifecycle_test": {
    "test_type": "minimal_value_transfer",
    "performed_by": "Arrow Registry Maintainer",
    "tx_ref": {
      "type": "evm_tx_hash",
      "value": "0x..."
    },
    "observed_at": "2026-05-25T00:00:00Z",
    "maps_to_fields": [
      "code",
      "protocol_required.finality_model",
      "protocol_required.finality_basis_default",
      "protocol_required.challenge_window_seconds",
      "protocol_required.settlement_chain"
    ],
    "extraction_artifact_hash": "sha256:<hash-of-arrow-native-extraction-artifact>",
    "next_state_test_due": "2026-06-24"
  },

  "anchoring": {
    "entry_hash": "sha256:<hash-of-this-entry-normalized>",
    "registry_version_hash": "sha256:<hash-of-full-registry-at-this-version>",
    "onchain_anchor": null
  }
}
```

`anchoring.onchain_anchor` MUST remain `null` in the anchored file — see the canonical-hash preimage rule in §3 (R-004). The actual anchor commitment (OpenTimestamps proof, transaction reference) is recorded out-of-file, in the sidecar `.ots` proof and the on-chain lifecycle evidence log, never inline.

### 4.2 `finality_model` vocabulary

| Code | Name | Description |
|---|---|---|
| `instant` | Instant finality | Transactions are immediately final on inclusion (e.g., centralized systems, some appchains) |
| `probabilistic` | Probabilistic (PoW / longest chain) | Finality accumulates with confirmations; reorg risk decays exponentially |
| `bft_consensus` | BFT Consensus | Finality is deterministic after 2/3+ validator threshold (Cosmos chains, Tendermint, Casper FFG) |
| `optimistic_rollup` | Optimistic Rollup | Transactions are settled on L1 subject to a fraud-proof challenge window |
| `zk_rollup` | ZK Rollup | Transactions are settled on L1 with a validity proof; finality is near-instant after proof inclusion |

**Origin Adapter behavior by `finality_model`:**

| Model | `Finality` for new events | `finality_basis` | `finality_eta` |
|---|---|---|---|
| `instant` | 1 | Not required (`Finality = 1`) | Not applicable |
| `bft_consensus` | 1 after BFT threshold | Not required if post-threshold | Not applicable |
| `probabilistic` | 0 before threshold | `probabilistic` | Block height of estimated safety threshold |
| `optimistic_rollup` | 0 within challenge window | `optimistic_window` | Block timestamp of window close |
| `zk_rollup` | 0 before proof inclusion; 1 after | `protocol_pending` before proof; not required after | Proof inclusion block estimate |

### 4.3 `chain_trust_flag` definition

| Value | Meaning |
|---|---|
| `1` | Trusted. The chain's consensus mechanism and security model are assessed as sound for Arrow-level trust. High-Confidence events from this chain are not automatically discounted. |
| `0` | Untrusted or deprecated. BRG-001 / SEM-016 equivalent: Action is forced to `BL` for events originating from this chain unless the deployment policy layer overrides. Applies to chains with known security failures, deprecated or abandoned chains, and chains under active assessment. |
| `null` | Unassessed. The chain is registered (so its code is recognized) but its trust status has not been formally assessed. Downstream behavior is deployment-defined; the conservative default is to treat as `0`. |

`chain_trust_flag` assesses **consensus/security soundness** only. It is orthogonal to the advisory `governance_capability` disclosure introduced by RFC-007 (CHN-005–007): a chain may hold `chain_trust_flag: 1` (its consensus is sound) while simultaneously disclosing, in `advisory.governance_capability`, a zero-delay emergency-upgrade authority and even a *demonstrated* compelled transfer. The Arbitrum entry is exactly this case — `chain_trust_flag: 1` alongside a `historical_compelled_transfer` with `status: demonstrated`. The two answer different questions ("is the consensus sound?" vs. "what latent governance power exists, and has it been exercised?"), and `governance_capability` is advisory-only — CHN-006 forbids it from altering Policy. Consumers price the latent governance risk separately from the trust flag.

---

## 5. Registry Governance Model (Draft)

This section defines the minimum governance shape for v0.3. The full ratification process, role model, and emergency notification workflow remain draft.

### 5.1 Maintenance entity

The Arrow Registry Maintainer — in the current phase, the Arrow Protocol Editor — is the entity responsible for:
- Reviewing and ratifying new entry proposals.
- Executing the periodic review cadence for existing entries.
- Publishing new registry versions and anchoring their hashes.
- Triggering emergency updates when a chain protocol parameter changes materially.

The intent is to move toward a distributed registry governance model as Arrow achieves adoption — multiple independent Maintainers, a formal proposal and voting mechanism. The architecture is designed to support this transition without breaking changes to the registry schema.

### 5.2 Entry lifecycle

| Status | Meaning |
|---|---|
| `proposed` | Entry has been submitted with source citations; under review (the *staged* state). Not yet usable for derivation; MAY carry only testnet/devnet pre-flight evidence, or none. |
| `active` | Entry has been ratified and is within its review cadence. Used for derivation. **Promotion to `active` requires a mainnet on-chain lifecycle test (enforced by CHN-008); testnet/devnet evidence is optional pre-flight and is never the activation gate.** |
| `stale` | Entry has exceeded its review cadence without re-verification. Triggers a registry staleness warning. Not automatically removed. |
| `deprecated` | The chain is no longer actively used or has been superseded. New events from this chain receive a Warning. |
| `revoked` | The entry has been revoked due to security failure or confirmed incorrect data. Triggers PRV-003 Fatal for derivation. |

### 5.3 Proposal requirements (standard track)

A new registry entry proposal MUST include:
- The chain code (registered uppercase Chain Registry identifier), name, and parent chain if Layer 2. Chain codes are distinct from the three-character canonical Origin field governed by ORG-001.
- A source citation for each `protocol_required` field.
- A completed `provenance` block.
- A proposed `review_cadence` based on the chain's governance activity and parameter volatility.
- A proposed `state_test_cadence`; the cadence MUST NOT exceed monthly in this draft.
- Evidence of an independently performed on-chain lifecycle test transaction by an Arrow Registry Maintainer, Protocol Editor, or delegated reviewer, unless the entry is still explicitly marked `proposed`.
- A mapping from the lifecycle test transaction to the submitted `protocol_required` fields.
- An Arrow-native extraction artifact, or a placeholder identifying the extraction format to be completed before ratification.

A proposal MAY include the full advisory section and anchoring block; these are populated by the Maintainer on ratification.

### 5.4 Lifecycle state verification

Lifecycle state verification is the registry's live check against chain reality. For each active chain entry, a Maintainer SHOULD periodically execute or observe a minimal on-chain lifecycle message and anchor the result. For account-based chains, the test MAY be a minimal token or native-asset transfer between Maintainer-controlled addresses. For UTXO, account-abstraction, appchain, or non-transfer chains, the test MAY use the smallest safe native state transition that proves the relevant registry fields.

The test result MUST bind:
- the registry entry code and entry version;
- the transaction hash or equivalent chain-native state reference;
- the observed chain identifier and parent/settlement chain where applicable;
- the finality model and finality basis inferred from the observation;
- the finality timing data used by an Origin Adapter, if deterministic;
- the extraction artifact hash.

The intent is to make the registry as trust-minimized and transparent as the current phase allows: source documents establish what the chain claims, lifecycle tests confirm that the chain can be observed in the way Arrow expects, and anchors make the test history tamper-evident.

### 5.5 Emergency update track

The emergency update track is specified by the **R-005 Update Governance Addendum** (`rfc/R-005-Update-Governance-RFC-006-Addendum-v0.2.md`, §9, change class `E1`), **ratified 2026-06-16** and incorporated here by reference as normative. The addendum is maintained as an independent document so it can be revised on its own cadence; this RFC pins the ratified v0.2. Its normative requirements are:

- **Trigger conditions.** `E1` MAY be used only where delay would risk materially incorrect, unsafe, or misleading registry behavior — chain fork or halt, bridge exploit, validator-set compromise, critical parameter change, attestation invalidation, emergency contract upgrade, endpoint compromise, or a discovered registry error with immediate validation impact.
- **Expedited timeline.** Provisional activation is permitted immediately after structured intake validation, evidence attachment, deterministic patch generation, CI patch verification, and approval by **two** authorized emergency approvers.
- **Mandatory post-ratification review.** A confirmatory decision (`CONFIRMED`, `AMENDED`, `REVERTED`, `ESCALATED_TO_C3`, or `ESCALATED_TO_C4`) MUST be recorded no later than **48 hours** after provisional activation. An `E1` update MUST NOT remain provisionally effective beyond 48 hours without a recorded decision.
- **Safe-state fallback.** Where resolution is not possible within 48 hours, the registry MUST default to the safer available state (suspension, deactivation, or explicit uncertainty classification) pending a `C3`/`C4` review.

**Notification of Origin Adapters pinning affected versions** remains an open question (§9 Q4): whether the notification mechanism belongs to the registry specification or the Origin Adapter contract (RFC-005). The emergency track is fully specified independently of that routing decision.

---

## 6. On-Chain Anchoring Implementation

**Normative commitment (v0.5, provisional).** Each published registry version's canonical hash (computed per the R-004 canonical-hash preimage rule) MUST be anchored via **OpenTimestamps** (Option B below), producing a Bitcoin-backed timestamp commitment. This follows the RFC-000 genesis-record precedent, which already uses OpenTimestamps. The choice is deliberately **provisional and supersedable**: OpenTimestamps requires no smart contract, no key management, and no deployed infrastructure, so it is the cheapest mechanism to adopt now and the easiest to supersede later (e.g., by adding Option A or C) without invalidating already-anchored versions. A future §6 revision MAY add or migrate to an additional mechanism; anchors already produced under OpenTimestamps remain valid.

Origin Adapters that pin a registry version SHOULD verify the pinned hash against its OpenTimestamps commitment before using the registry for derivation (R-004).

### 6.1 Anchoring approaches considered

Three approaches were evaluated, non-exclusive. v0.5 commits to **Option B**; A and C are retained here as future options:

**Option A — Ethereum calldata.** The registry hash is submitted as calldata in a zero-value Ethereum transaction from a registered Arrow Protocol address. Established precedent: the Arrow Protocol genesis record (RFC-000) was anchored via Ethereum calldata. No smart contract required; tamper-evident by Ethereum consensus; simple to verify.

**Option B — OpenTimestamps.** The registry hash is committed to OpenTimestamps, which creates a Bitcoin blockchain commitment. Established precedent: Arrow Protocol genesis record uses OTS. Bitcoin's security properties provide a strong timestamp anchor.

**Option C — Smart contract registry.** A minimal Ethereum smart contract stores the version-to-hash mapping. Supports programmatic verification by Origin Adapters and consumers. Requires contract deployment and key management.

The genesis-record precedent favors Option A + Option B in parallel (belt-and-suspenders, as used for RFC-000). **v0.5 commits to Option B alone as the provisional baseline** — the minimal mechanism sufficient to satisfy R-004 — and leaves Option A as a low-cost future addition and Option C for when programmatic verification at scale is required. Adding Option A later is a non-breaking superset.

### 6.2 Verification by Origin Adapters

When an Origin Adapter pins a Chain Registry version (per OA-007), it SHOULD:

1. Record the version number and the hash it computed of the registry file it received.
2. Verify the computed hash against the on-chain anchor for that version.
3. Log the verification result with the anchor transaction hash.

If the computed hash does not match the on-chain anchor, the Origin Adapter MUST NOT use that registry version for derivation and MUST alert the deployment operator.

---

## 7. Initial Chain Registry Population Targets

The following chains are priority entries for Chain Registry v2.0, based on Arrow Protocol's primary integration surfaces:

| Priority | Chain | Code | Key fields | Source complexity |
|---|---|---|---|---|
| 1 | Ethereum | `ETH` | `probabilistic`; `reorg_safety_confirmations = 12` | Well-documented |
| 1 | Bitcoin | `BTC` | `probabilistic`; `reorg_safety_confirmations = 6` | Well-documented |
| 1 | Solana | `SOL` | `bft_consensus`; near-instant after BFT threshold | Well-documented |
| 2 | Base | `BASE` | `optimistic_rollup`; `challenge_window_seconds = 604800` | OP Stack — documented |
| 2 | Optimism | `OP` | `optimistic_rollup`; `challenge_window_seconds = 604800` | OP Stack — documented |
| 2 | Arbitrum One | `ARB` | `optimistic_rollup`; Nitro; 7-day window | Documented |
| 2 | Polygon PoS | _TBD — see note_ | `bft_consensus` (Heimdall + Bor); checkpoint to ETH | Moderately complex |
| 3 | Cosmos Hub | `ATOM` | `bft_consensus`; Tendermint | Well-documented |
| 3 | Avalanche C-Chain | `AVAX` | `bft_consensus`; Snowball | Documented |
| 3 | BNB Chain | `BNB` | `bft_consensus`; PoSA | Documented |

Entries for `LEG`, `ORC`, `CST`, and `PCY` (non-chain origins) are not in the Chain Registry. The Chain Registry covers blockchain networks only. Non-chain origins are in the Origin Registry.

> **Note — the `POL` namespace collision (resolved in v1.5.2-draft).** Earlier drafts of this table assigned Polygon PoS the chain code `POL`. That collided with the governance/Policy origin formerly registered as `POL` (`origin_type: governance`, `verification_regime: bonded_quorum`). Polygon's real-world token is also now `POL`, so the contention was genuine, not cosmetic.
>
> **Resolution:** v1.5.2-draft renamed the governance/Policy origin to `PCY`, preserving its `bonded_quorum` semantics and freeing `POL` for Polygon PoS. Polygon's chain entry remains unregistered in the live Chain Registry until RFC-006 graduation or a later registry patch adds the full sourced entry.

---

## 8. Relationship to Other RFCs

**RFC-003 §17** named the Robust Chain Registry as a future RFC with the observation that Origin Adapters depend on it for derivation and cannot safely treat it as a hand-maintained file.

**RFC-004 Part 11** introduced the optional `provenance` block schema. RFC-006 makes `provenance` mandatory for Chain Registry entries and extends the schema.

**RFC-005 PRV-003** mandates that Chain Registry entries used for derivation carry a `provenance` block. RFC-006 operationalizes that mandate by defining the full entry schema, governance process, and anchoring mechanism.

**Future — Origin Adapter Registry.** RFC-005 §4.4 notes that a normative registry of conforming Origin Adapters may eventually be required. That registry will reference Chain Registry versions (to establish which registry version an Origin Adapter was certified against) and is a natural successor to RFC-006.

**Future — Multi-source Confidence upgrade.** The parallel observation infrastructure noted in RFC-003 §17 and RFC-005 §10 will require a trust-calibration model that maps source combinations to Confidence levels. That model will reference Chain Registry entries to determine whether a given chain supports self-verifying proof observation (ZK / light-client).

---

## 9. Open Questions for v0.5

1. **Anchoring implementation.** ~~Ethereum calldata + OpenTimestamps, or a smart contract?~~ **RESOLVED (v0.5):** committed to OpenTimestamps as the provisional baseline (§6), supersedable. Remaining sub-question: when does Option A (Ethereum calldata) or Option C (smart-contract registry) become warranted as an addition?
2. **Review cadence enforcement.** Who triggers the review when an entry hits `next_review_due`? What is the escalation path if the Maintainer is unavailable?
3. **Governance transition.** At what adoption milestone does the registry governance move from single-Maintainer to a distributed model? What is the minimum viable distributed governance structure?
4. **Emergency update notification.** How are Origin Adapters notified of an emergency update to a pinned registry version? Is a notification mechanism part of the registry specification or the Origin Adapter contract (RFC-005)?
5. **`chain_trust_flag` assessment process.** Who performs the security assessment? What criteria determine `1` vs `0` vs `null`? Is this a separate document?
6. **Coverage of non-EVM chains.** The initial population targets include Bitcoin, Solana, and Cosmos. Their finality models are well-documented but differ structurally from EVM chains. Does the schema need chain-type-specific extensions?
7. **Versioned sub-registries.** Should the Chain Registry be split into sub-registries per chain family (EVM, Cosmos SDK, UTXO) with a top-level index? Or does a flat JSON file scale?
8. **Extraction artifact format.** What is the canonical Arrow-native extraction artifact for lifecycle tests, and how does it remain reproducible across EVM, UTXO, SVM, Cosmos, and non-transfer state machines?
9. **Maintainer test identity.** Should lifecycle test transactions be sent from a public Arrow Registry Maintainer address, an editor address, or rotating delegated reviewer addresses?
10. **Multi-attestation correlation.** When registry tests serve as a second source of truth, what quorum or correlation rule is required before an Origin Adapter may treat them as corroborating evidence?

---

## Document Information

| Field | Value |
|---|---|
| Document ID | ARROW-RFC-006 |
| Version | v1.0 |
| Status | **Graduated 2026-06-16; published as v1.0 on 2026-06-17.** Evidentiary blockers discharged (5/5 lifecycle backfill + Bitcoin-anchored snapshots); governance blockers discharged (R-005 ratified, §6 anchoring committed); editorial gates from the 2026-06-16 and 2026-06-17 readiness reviews closed. Moved out of `rfc/drafts/` to the published RFC shelf. |
| Genesis Anchor | RFC-000 — `2026-03-22T18:40:00Z` |
| Depends on | RFC-004 v1.0; RFC-005 v1.0; R-005 Update Governance Addendum v0.2 (ratified); Validation Rules v1.6.0 (CHN-008) |
| Supersedes | RFC-006 v0.4; `registries/chain_registry.json` (ad hoc) |
| Governance | Editor-controlled, graduated. §5.5 emergency track rests on the **R-005 addendum, ratified 2026-06-16** (normative); §6 anchoring (OpenTimestamps, provisional) is specified and **satisfied** — both registry snapshots Bitcoin-confirmed. On-chain lifecycle-test backfill (R-006/R-009/R-010/R-011) is **complete (5/5)**. The `POL` namespace collision is discharged in v1.5.2-draft (§7). The R-004 canonical-hash preimage rule is codified in §3, and **CHN-008** enforces non-null `lifecycle_test` on `active` mainnet entries as a build-time lint. |
