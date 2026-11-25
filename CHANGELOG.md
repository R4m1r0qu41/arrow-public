# Changelog

## Unreleased

**RFC-010 implementation: TRG-001, LFC-007, LFC-008, NAM-002 (Universal Eventual Closure)**

- **TRG-001 live** (was draft): Action TR requires an `action_target`
  envelope (`target_ref`, `object_ref`, `authorizing_events` ⊆
  `related_events`) and Class ∈ {2,5}. Shape checked at the gate;
  resolvability is consumer-side (Tarquin LC-014). RFC-001's historical TR
  event is compat-patched in its lifecycle test. Fixtures F-22..F-25;
  enumeration theorem T10.
- **LFC-007 live** (was reserved): post-closure lifecycle discipline. After
  a validated Class 6 closure, only a Class 1 State RV reversal or Class 4
  CONTROL supersession citing the closure's event_id specifically is valid;
  everything else (including a second closure) rejects with new reason code
  POST_CLOSURE_VIOLATION. Context-gated. Fixture harness gains
  `context_files` support; fixtures F-26..F-30.
- **LFC-008 live** (new slot, RFC-010 Part 2A — Universal Eventual Closure):
  every lifecycle MUST eventually close. All nine verification regimes now
  declare `closure_discipline`; settled Class 1 events under bounded
  disciplines (depth_exhaustion, window_exhaustion) MUST carry
  `closure_eta`/`closure_eta_type` (0 block_count = due immediately).
  Origin adapters (BTC, SOL, EVM L1) emit the declaration; overdue-unclosed
  is a passive consumer-side computation (Tarquin LC-016) and an attack
  indicator, never an ambient-clock check in V. Fixture F-31.
- **NAM-002** — new `registries/reserved_tokens.json` + CI-fatal lint:
  rule-family prefixes (SEM, LEG, TRG, LFC, ...) and cross-repo tokens (MC,
  W#, LC, ENG, SBL) are machine-reserved; every pinned rule ID must use a
  registered prefix; `CL`/`TR`/`PCY` permanently barred as prefixes.
- Rule surface: TRG-001 → live, LFC-007 → live, LFC-008 added live; sidecar
  regenerated; Tarquin re-pin required and performed.
- Closure watcher groundwork: `arrow_adapter::closure::build_closure_event`
  constructs the canonical terminal `6:{ORIGIN}:FN:CL:1:*` referencing the
  observation it terminates; a lapsed-unsettled lifecycle closes as
  non-consummated with new reason code `NON_CONSUMMATION` (abandoned
  lifecycles must not linger as dormant attack surface). Tests prove both
  paths validate with lifecycle context and that a second closure is
  rejected by LFC-007.

**Exhaustive six-field enumeration harness + five unenforced SEM MUSTs**

- New `cargo test -p arrow-core --test enumeration`: enumerates the entire
  registry-defined message tuple space (currently 21,600 tuples: 6 classes ×
  12 origins × 5 states × 6 actions × 2 finalities × 5 confidences) and
  asserts the spec's message-level MUSTs as total-truth theorems against the
  real validator — SEM-013, SEM-015 (incl. the pre-settlement conditional
  ceiling), the closure shape (SEM-005/012/020), SEM-006, SEM-007, SEM-009,
  SEM-001, SEM-002/003/004, plus "every rejection carries an Error
  violation". The oracle restates the spec independently (registry data + a
  locally defined confidence order); it never calls validator internals.
- Building the harness exposed five spec MUSTs with no enforcement code:
  SEM-001 (Finality 1 requires FN/RV), SEM-002/003/004 (PD/TL/CD require
  Finality 0), and SEM-007 (Class 5 TRANSFER must use TR or PR). All five
  are now enforced in `validate_semantic`; fixtures F-18 through F-21 lock
  them. SEM-008 (Class 2 SHOULD use PR) is a SHOULD, not a total truth, and
  remains deliberately unimplemented.
- CLI ack narratives added for the five new rules; validation-surface
  narratives extended to match.

**SEM-015 conditional (pre-settlement) ceiling — validator-enforced**

- `optimistic_settled`'s long-standing spec row "A post-window / B pre-window"
  is now enforced by the validator instead of being producer-advisory.
  `registries/verification_regimes.json` gains the optional field
  `pre_settlement_max_confidence` (set to `B` on `optimistic_settled`); SEM-015
  applies it as the effective ceiling whenever `Finality = 0`. The Finality bit
  is the window discriminator — deterministic, event-carried, no clock input.
- SEM-015 event extension: a `Finality = 1` event claiming Confidence above the
  pre-settlement ceiling while carrying a `finality_eta` (`unix_ts`) later than
  its own `timestamp` is rejected — a settled claim may not date its window
  close in its own future.
- FB-001 now enforces `requires_finality_eta`: a registered `finality_basis`
  whose entry sets the flag (e.g. `optimistic_window`) rejects events missing
  `finality_eta`. The EVM origin mapping fails closed
  (`EvmConfigError::MissingBlockTimestamp`) when an optimistic-L2 observation
  cannot date its challenge window.
- Fixtures F-13 through F-17 lock the new surface (settled pass, pre-window
  reject, pre-window compliant pass, undated-window reject, eta-contradiction
  reject).
- SEM-005 duplicate obligation removed from the Rust validator: the
  "Class 6 requires Finality = 1" check was mislabeled under SEM-005 (pinned
  slug `class6_requires_cl` covers exactly one obligation). Verdicts are
  unchanged — the finality shape is already derived by SEM-005 (forces CL) +
  SEM-013 (rejects executing Actions at Finality 0). CLI narrative and
  Class 6 comments corrected from legacy "Settle" to CLOSE.
- `exports/rule-surface.v1.6.0.json`: added live pins for SEM-015
  (`regime_confidence_ceiling`) and FB-001 (`finality_basis_required`);
  sidecar hash regenerated.

**On-chain lifecycle evidence**

- Recorded the first real mainnet lifecycle extraction: Ethereum mainnet
  1 wei transfer
  `0x505bb0130e46b17e1adec44b288b7fc544fb31776f0f902d46bc475f14e77175`
  produced Arrow code `1:ETH:FN:PR:1:A` and extraction artifact hash
  `sha256:ba36852ae91e758b70f916c6f7c190d14fcab27d62a3242cc36d37db497929cd`.
- Recorded the Base mainnet lifecycle extraction for transaction
  `0x720648dfed963eba6b52e591b361f19368c8de56c201b6552a5ff0b4da9fba8a`;
  it produced the expected optimistic-rollup pending code `1:BSE:PD:HL:0:B`
  and extraction artifact hash
  `sha256:93c4d659cfd58d9d7e1d3723297ad2ba1ed91fcc2c7cbf7a1f365b1f834d854d`.
- Recorded the Arbitrum One mainnet lifecycle extraction for transaction
  `0x401abd713b25a8a037132091f26369b19c42aa34b46e0eee0686336ed7709652`;
  it produced the expected optimistic-rollup pending code `1:ARB:PD:HL:0:B`
  and extraction artifact hash
  `sha256:5d3567bee90d29cb4dfc2fc0c13412a14ecd000f9f34d7ee9909ed6dd7935770`.
- Recorded the Solana mainnet-beta lifecycle extraction for signature
  `pbY7vFe5ZL1nUHpMPmKWYdUoMTVEBTutdDLYqd4PqdukX3XquJ9GMHKY7NajH6uLZBnFp77A1h47LtRsz9EKhGg`;
  at finalized commitment it produced Arrow code `1:SOL:FN:PR:1:A`
  and extraction artifact hash
  `sha256:163c3e645e8b6aa2627c802ad3a84bdeb7d2682bc5ba34a248c7c6e268563479`.
- Recorded the Bitcoin mainnet transfer
  `a58d4f8c12cf11b492b78b088940b749af795a774a28eb94aa421dc8aad56e46`;
  the network reports 7 confirmations and the output to
  `bc1qaqssn3xe52h8h5fsg7dn8qn6pdtsrktarefp0x` is visible on-chain, but the
  local pruned wallet observer is still syncing and has not yet generated the
  BTC artifact.
- Backfilled `registries/chain_registry.json` with the completed ETH, Base,
  Arbitrum, and Solana mainnet lifecycle-test blocks, including extraction
  artifact hashes and the current mainnet registry snapshot hash. BTC remains
  unfilled pending a local BTC mainnet extraction artifact.
- Recorded the first real public testnet lifecycle extraction:
  ETH Sepolia 1 wei transfer
  `0xce219718431bb06fe14776b28d7a29ad68d75aacf5cc55be65af2c8fea709243`
  produced Arrow code `1:ETH:FN:PR:1:A` and extraction artifact hash
  `sha256:ab100442c1cc440436ec80994f2c91b56adb0d7b6eb100ff046ec7c11831a896`.
- Recorded the final Bitcoin Signet lifecycle extraction for outpoint
  `3b373919b9654e0a24f5a14c2eb7daf1cf795889371e0fe6ba95c61c758a3c10:1`;
  at 36 confirmations it produced Arrow code `1:BTC:FN:PR:1:A`
  and extraction artifact hash
  `sha256:bd8212af659d0140ce207a76ca441c9b120357ceafccf1b57b5dcc9fd078ac32`.
- Recorded the Solana devnet lifecycle extraction for signature
  `u4vKrDnkhjqTbAWJJ5pFKkjsPJZ8YLYVxDhrC3piSCKui71Tnr6vcT3LstFGy1huFD5fJFQUqgqF3xb4JLkx99v`;
  at finalized commitment it produced Arrow code `1:SOL:FN:PR:1:A`
  and extraction artifact hash
  `sha256:1285a69ba88562200e14341b6341ccf351340c09686333735759744e70e8de15`.
- Recorded the Base Sepolia lifecycle extraction for transaction
  `0x1d0a412248ceed527f5373d09faedbfbd6fe58448b3d667cc5bfd98b14fdaef4`;
  it produced the expected optimistic-rollup pending code `1:BSE:PD:HL:0:B`
  and extraction artifact hash
  `sha256:64a03d1a2509583ae8e309a4daeba42f33a67210107679fd2bae4dd8fd6aabdf`.
- Recorded the Arbitrum Sepolia lifecycle extraction for transaction
  `0x1f901e3e51364f68f10c31a697987f8e27719a02239ec2c374c43f96337ab57d`;
  it produced the expected optimistic-rollup pending code `1:ARB:PD:HL:0:B`
  and extraction artifact hash
  `sha256:37438a6f027e693a1df2f2d5682a284d4c7aa2a91855ea2b19c44fec0c3de324`.
- Backfilled `registries/chain_registry.testnet.json` lifecycle-test blocks
  from the recorded ETH, BTC, SOL, Base, and Arbitrum artifacts, including
  `tx_ref`, `observed_at`, `maps_to_fields`, extraction artifact hashes, next
  state-test due dates, per-entry hashes, and the testnet registry version hash.
- Created an OpenTimestamps proof for the backfilled testnet registry snapshot
  at `registries/chain_registry.testnet.json.ots`; the proof is pending Bitcoin
  confirmation before `anchoring.onchain_anchor` can be marked complete.
- Added `docs/test/ON-CHAIN-LIFECYCLE-EVIDENCE-LOG.md` and retained the
  extraction artifact under `docs/test/evidence/`.

**Harness compatibility**

- `arrow-origin-evm` now uses a rustls-backed `reqwest` JSON-RPC client for
  read-only observation, so lifecycle runs work with the HTTPS public RPC
  endpoints already documented in the testnet runbook.
- `arrow-origin-sol` now uses the same HTTPS-capable `reqwest` transport for
  Solana JSON-RPC observation.

## v1.6.0 (2026-06-10)

**Theme:** Policy-decoupling completion and naming freeze (Tarquin RFC-008 v0.2 / RFC-009 v0.2 alignment). Non-breaking at the wire layer — the canonical six-field string never carried a field label. One additive semantic rule (SEM-020).

**Field 4 final rename (`Control` → `Action`)**

- `registries/controls.json` → `registries/actions.json` (registry `actions`); codes `PR`/`BL`/`HL`/`RW`/`TR`/`CL` and semantics unchanged. **Final rename; name frozen.**
- Field-4 rule identifiers `CTL-001`/`CTL-002` retired → `ACT-001`/`ACT-002`. Last rule-ID rename: rule identifiers are **permanent** from v1.6.0 (retired, never renumbered, never reused).
- `arrow-core`: `types::message::Control` renamed to `Action`; `ArrowMessage.control` → `.action`; deprecated `pub type Control = Action` and `pub type Policy = Action` aliases restore the downstream compile contract (Tarquin crates) that the v1.5.2 rename broke. Repo-wide identifier rename across all crates, CLI, fixtures, and tools.

**Class 4 rename (`POLICY` → `CONTROL`)**

- `classes.json`: Class 4 is the control-plane event class — registry updates, chain/bridge status transitions (DIV-003), protocol/consensus upgrades, validator/node (DVN/OVN) topology changes, parameter changes, emergency declarations (EMG), supersession (SEM-014).
- NEW `registries/control_subtypes.json` (draft): anchor vocabulary with per-subtype `anchor_requirements`; inert while draft; **SEM-021 reserved** for promotion to a mandatory gate.
- The word "Policy" now appears in exactly one canonical namespace: the Origin `PCY` display name. "Deployment policy" remains the only unqualified use of the word in prose (Step 13).

**SEM-020 — Closure Class Binding**

- New rule: Action `CL` MUST use Class 6 (converse of SEM-005). Implemented in `arrow-core` `semantic.rs`; the previously doc-only SEM-005 action direction (Class 6 MUST use CL) is now also enforced in code. The only valid closure shape is `6:*:FN:CL:1:*` (with SEM-012/SEM-013).
- New fixture `F-12-reject-sem-020.json`; fixtures index bumped to protocol 1.6.0.
- **LFC-007 reserved** for post-closure lifecycle discipline (reversal/supersession referencing the CL event only).

**NAM-001 — Naming-integrity lint**

- `tools/validate-registries.js`: no code or display name may appear in more than one canonical six-field namespace (classes, origins, states, actions, confidence). Case-insensitive, CI-fatal. Would have caught the State-HL/Control-HL collision (v1.5) and the three-way "Policy" overload.

**Docs**

- NEW `docs/protocol/arrow-protocol-v1.6.0.md` (delta spec: Δ-1 Action rename, Δ-2 Class 4 CONTROL + anchors, Δ-3 SEM-020 + follow-up event model, Δ-4 NAM-001 + rule-ID permanence).
- NEW `docs/protocol/validation-rules-v1.6.0.md` (full rules doc; historical sections modernized to current field name, rule-ID history preserved).
- Cleanup pass: `VERSION`, README orientation, AI context, and live registry package labels normalized to `1.6.0`; active field-4 metadata/prose uses Action terminology while historical version notes remain preserved.

## v1.5.3-draft (2026-06-05)

**Theme:** On-chain lifecycle build (RFC-006 v0.3 graduation prerequisites R-009, R-010, R-011). Adds chain-identity binding, R-011 extraction artifacts, EIP-155 raw signing, testnet registry overlay, and a full CI split. No breaking change to the Arrow message format or existing adapter APIs.

**Observer hardening (R-009 / §4.1–§4.2)**

- `arrow-origin-evm`: `ObservedTx` gains `observed_chain_id: u64` (from `eth_chainId` RPC call) and `block_timestamp: Option<u64>` (from `eth_getBlockByNumber` for mined txs). `EvmChainConfig` gains `expected_eip155_chain_id: Option<u64>`; a mismatch is a hard error before any event is minted.
- `arrow-origin-btc`: `ObservedUtxo` gains `observed_network: Option<String>` bound from `getblockchaininfo.chain` (`"main"` / `"signet"` / `"regtest"`).
- `arrow-origin-sol`: `ObservedTransaction` gains `genesis_hash: Option<String>` bound from `getGenesisHash` to identify the cluster.
- New testnet chain configs: `EvmChainConfig::base_sepolia()` (chain id 84532) and `EvmChainConfig::arb_sepolia()` (chain id 421614).

**Optimistic L2 finality ETA (§4.5)**

- Base/Arb pending events now carry `finality_eta` as a unix timestamp derived from `block_timestamp + challenge_window_seconds` and `finality_eta_type = "unix_ts"`. The adapter remains intentionally conservative: no `FN` is minted from L2 block depth alone (FB-001 preserved).

**R-011 Extraction Artifact (§4.8 / §5)**

- New crate `arrow-registry-test`: `artifact.rs` provides `ArtifactCore`, `ArtifactObservation`, `ExtractionArtifact`, `build_artifact`, and `verify_hash`. Core is hashed via JCS (RFC 8785) + SHA-256 as `"sha256:" + hex`. Observation layer is informational and excluded from the hash.
- `lifecycle.rs` drives end-to-end EVM / UTXO / SVM lifecycle runs and returns an `ExtractionArtifact`.
- `anchor.rs` supports calldata anchoring + `anchor verify` for the registry-version hash.
- `networks.rs` defines testnet chain ids, genesis hashes, default RPC endpoints, and `resolve_rpc(env_var, default)`.

**EIP-155 raw signing (§4.4)**

- `arrow-consume-evm`: `OperatorConfig` gains `raw_key_hex: Option<String>` and `unlocked: bool`. `SettleParams` carries both. `settle::send_settlement` dispatches to `eth_sendRawTransaction` (raw path) or `eth_sendTransaction` (unlocked path). Raw signing uses `k256 = "=0.13.3"` (pinned) + `sha3` keccak256 + manual RLP — no external RLP crate. Known-vector test covers the EIP-155 test vector.

**Testnet registry overlay (§4.10 / R-010)**

- `registries/chain_registry.testnet.json`: 5 entries (ETH/sepolia, BASE/base-sepolia, ARB/arb-sepolia, SOL/devnet, BTC/signet). All have `chain_trust_flag: 0`, `derivation_eligible: false`, `state_test_cadence: "30d"`.
- `registries/chain_registry.json`: all 5 entries gain `state_test_cadence: "30d"` and `next_state_test_due: "2026-07-05"` in their `provenance` blocks.
- `tools/validate-registries.js`: validates `state_test_cadence ≤ 30d`, `lifecycle_test` block shape, testnet `chain_trust_flag ≠ 1`, and testnet `derivation_eligible ≠ true`.

**CI split (§4.11)**

- `.github/workflows/rust.yml` restructured into three jobs: Job A (always — fmt, test, clippy, validate-registries), Job B (local devnet — workflow_dispatch or label), Job C (public testnet — workflow_dispatch + secrets-gated).

**Live integration tests (§7.3–§7.4)**

- `crates/arrow-origin-btc/tests/live_regtest.rs`: Bitcoin Core `-regtest` end-to-end (ignored; requires `bitcoin-cli`).
- `crates/arrow-origin-sol/tests/live_local_validator.rs`: Solana test-validator end-to-end (ignored; requires `solana-test-validator`).
- `crates/arrow-registry-test/tests/public_testnet.rs`: public testnet smoke tests for ETH Sepolia, Base Sepolia, Arb Sepolia, SOL devnet, BTC Signet, and `a_to_b_sepolia` raw-signed settlement (all ignored; env-gated).

**Tests**

- Gate count: all existing tests pass; 3 new ignored integration tests added (live_regtest, live_local_validator, public_testnet suite). `cargo clippy -D warnings` clean.

---

## v1.5.2-draft (2026-06-03)

**Theme:** Adapter expansion and transport hardening package. The canonical
six-field Arrow message structure remains six fields and five delimiters.

**Field naming**

- Canonical field 4 renamed from `Policy` to `Control`: `Class:Origin:State:Control:Finality:Confidence`. Codes `PR`/`BL`/`HL`/`RW`/`TR`/`CL` and validation semantics are unchanged.
- `registries/policies.json` renamed to `registries/controls.json`; field-rule identifiers now use `CTL-001`/`CTL-002`.
- Origin `PCY` remains the Policy/governance origin. There is no `CTL` Origin.

**Adapters**

- Added shared EVM-family origin/consumer libraries and thin ETH/Base/Arbitrum wrappers.
- Added Bitcoin UTXO origin/consumer adapters with sats-native typed transfers and RecordOnly/DryRunOnly consumption.
- Added Solana origin/consumer adapters with commitment-based finality, lamports-native typed transfers, and RecordOnly/DryRunOnly consumption.
- Added a cross-product conformance crate covering the 6 x 6 origin/consumer matrix across ETH, BSE, ARB, BTC, SOL, and LEG.
- Hardened consumer settlement intent derivation so canonical Arrow origin, chain_id, source_system, source_chain_id, and unit constrain typed extensions before any action intent is constructed.

**Registries and Transport**

- `registries/index.json`: bumped to `1.5.2-draft`.
- `registries/origins.json`: added `BSE` for Base and corrected SOL from `optimistic_settled` to `native_canonical`.
- `registries/origins.json`: registered `ORC` under `bonded_quorum` (ceiling B), providing the oracle-network path without a separate C-ceiling oracle regime.
- `registries/verification_regimes.json`: superseded draft `legal_oracle_structured` with `structured_legal_attestation` (single Sibyl/LBRY-style structured proof, ceiling B) and `multiple_legal_attestation` (independent multi-Sibyl evidence, ceiling B). Added `legal_native_attestation` as draft/reserved experimental terminology; no current Origin depends on it.
- `registries/confidence_subprofiles.json`: replaced `legal_attestation_panel` with `structured_legal_attestation` and `multiple_legal_attestation`. Legal authority panels are now tracked on the LEG authority/finality axis rather than the confidence axis.
- `registries/channel_bindings.json`: added `http-rest-local` and `https-rest`; legacy `http-rest` is inactive.

**RFC-007 additive registry metadata (advisory only; non-breaking)**

The canonical six-field message string is unchanged. RFC-007 itself remains a
*draft* — only the additive registry/struct metadata lands here; no `DPL-*`,
`INT-*`, or `CHN-005–007` rules are wired into the validation pipeline.

- `registries/origins.json`: added `EMG` (Emergency Authority), `origin_type`
  `emergency_authority`, `verification_regime` `institutional_attestation`
  (ceiling C). The `governance` origin_type pre-existed on `POL`; not added.
  `CHA` (Chain Authority) deferred until a concrete case requires it.
- `registries/chain_registry.json`: ARB entry gains
  `advisory.governance_capability` (RFC-007 CHN-005) documenting the 21/04/2026
  Arbitrum Security Council action — structured `historical_compelled_transfer`
  and `sender_impersonation_via_upgrade` (`CapabilityDisclosure`:
  status / source_ref / observed_at), plus `emergency_upgrade_authority`,
  `multisig_threshold`, `upgrade_delay_seconds`,
  `emergency_upgrade_exit_window_seconds`. Advisory only — CHN-006 forbids any
  Policy effect; PRV-003 is not triggered.
- `crates/arrow-core/src/types/registry.rs`: new `CapabilityDisclosure` struct
  (status / source_ref / observed_at) and `GovernanceCapability` struct;
  `ChainAdvisory` gains optional `governance_capability:
  Option<GovernanceCapability>`. All new fields optional and backward-compatible;
  every existing chain entry deserializes unchanged.

**Tests**

- Final gate count: 302 passed / 2 ignored. The ignored tests remain the pre-existing Foundry/Anvil-gated ETH end-to-end tests.
- Added `docs/test/ADAPTER-EXPANSION-TEST-REPORT-v0.1.md` with the final gate output and matrix result.

## v1.5.1-draft (2026-05-31)

**Theme:** Validation-rule numbering reconciliation and completion of the LEG Policy Derivation Gate that v1.5-draft had landed as a minimal first pass. Non-breaking at the message-string and event-object layer; all v1.5 events validate identically under v1.5.1 with the rule-identifier mapping below applied.

**Validation Rules — Renumbering**

The reconciliation policy: numbers retired or reclassified in v1.5 are reused only where a thematically adjacent v1.5 rule occupied the next slot and the compaction is short-range. Higher-numbered stable rules (SEM-014 through SEM-019, MSG-005, BRG-*, etc.) are NOT renumbered.

- SEM-013 (retired-superseded in v1.5) → **SEM-013 (FN+0 Executing-Policy Ceiling)**. Origin-agnostic, protocol-wide. Slot reused; the original SEM-013 reversal-reference rule remains absorbed by SEM-014.
- LEG-006 (`leg_subtype` controlled vocabulary; was semantic in v1.5) → **MSG-006** (reclassified as schema constraint per RFC-004 §10).
- LEG-007 (Policy Derivation Gate) → **LEG-006**.
- LEG-008 (SUPRA Sanctions Force-BL) → **LEG-007**.
- LEG-009 (LEG_TEXT/PREC Finality=1 Warning) → **LEG-008**.
- LEG-010 (`leg_jurisdiction` Required) → **LEG-009**.
- LEG-011 (`supra` Registry Membership) → **LEG-010**.

Permanent gaps preserved (not reused in v1.5.1): SEM-018 (reclassified to MSG-005 in v1.5), LFC-006 (retired in v1.5), LEG-011/012/013/014.

**Validation Rules — New / Completed**

- **SEM-013 — FN+0 Executing-Policy Ceiling.** New, origin-agnostic. If `Finality = 0`, Policy MUST NOT be in the executing set `{PR, TR, CL}`. Lifted out of v1.5's LEG-007 per-subtype table to formalize the protocol-wide invariant identified by RFC-004 §9.3. Runs at Step 5 alongside SEM-001–014.
- **MSG-006 — `leg_subtype` Registry Membership.** New schema constraint. Replaces v1.5 LEG-006 (semantic). Fires when `origin = LEG` and `leg_subtype` is absent or not in `{JUD, ADM, SUPRA, LEG_TEXT, PREC}`. Reason code `LEG_SUBTYPE_ABSENT` unchanged.
- **LEG-006 — Policy Derivation Gate (full implementation).** Implements the complete subtype → permitted-policy matrix from RFC-004 §9.3 for `Finality = 1`: JUD={PR,BL,TR,CL}; ADM={PR,BL,HL,CL}; SUPRA(non-SA)={PR,HL}; LEG_TEXT={HL,RW}; PREC={RW}. The `Finality = 0` row is governed by SEM-013 protocol-wide; LEG-006 does not re-enumerate it. Reason code `LEG_POLICY_RANGE_VIOLATION` unchanged.
- **LEG-007 — SUPRA Sanctions Force-BL.** Implemented. Forces `Policy = BL` and sets `forced_policy: Some("BL")` on the validation result when `leg_subtype = SUPRA`, `Finality = 1`, `reason_code = SA`. Records a violation with reason code `LEG_SUPRA_SANCTIONS_FORCE` if the submitted Policy was not already BL. Mirrors BRG-001 and SAN-001. Evaluated **before** LEG-006 inside Step 5b so the range check is short-circuited.
- **LEG-008 — LEG_TEXT/PREC + Finality=1 Warning.** Implemented as a non-fatal warning. Emits when `origin = LEG`, `leg_subtype ∈ {LEG_TEXT, PREC}`, `Finality = 1`.

**Spec**

- New `docs/protocol/arrow-protocol-v1.5.1-draft.md` — delta specification. v1.5-draft is preserved; v1.5.1 documents only the renumbering reconciliation, the new SEM-013 (Part V1.5.1-Δ-1), and the LEG amendments (new Part 17).
- New `docs/protocol/validation-rules-v1.5.1-draft.md` — full validation rules with v1.5.1 numbering. Supersedes v1.5-draft.

**Rust (arrow-core)**

- `validation/pipeline.rs`:
  - Added SEM-013 check in `validate_semantic`: origin-agnostic Finality=0 / executing-Policy rejection.
  - Replaced the minimal Step 5b block with the full LEG Policy Derivation Gate: MSG-006 subtype check, LEG-004 (E→RW), LEG-005 (D→subtype restriction), LEG-007 (SUPRA+SA force-BL), LEG-006 (Finality=1 derivation table), LEG-008 (LEG_TEXT/PREC warning).
  - LEG-007 sets `forced_policy = Some("BL")` and short-circuits LEG-006 for the SUPRA+SA branch.
- `tests/valid_messages.rs`: updated one example (`1:BTC:PD:PR:0:A` → `1:BTC:PD:HL:0:A`) which now correctly violates SEM-013 — Class 1 pending observations should not assert executing Policy.
- Added 13 new unit tests in `validation/pipeline.rs::tests` covering SEM-013, MSG-006, LEG-005/006/007/008, including the LEG-007 force-BL no-double-fire interaction with LEG-006.

**Registries**

- `registries/index.json`: bumped to `1.5.1-draft`; added `v1_5_1_changes` block. Registry payloads are unchanged from v1.5.
- `registries/reason_codes.json`: bumped to `1.5.1-draft`. Descriptions for `LEG_POLICY_RANGE_VIOLATION`, `LEG_SUPRA_SANCTIONS_FORCE`, `LEG_SUBTYPE_ABSENT` updated to reference v1.5.1 rule numbers (LEG-006/LEG-007/MSG-006). Reason-code identifiers unchanged.
- `registries/drafts/leg_supra_jurisdictions.json`: UNSC and FATF entry notes updated to reference LEG-007 (was LEG-008 in v1.5).

## v1.5-draft (2026-05-26)

**Registries**
- `policies.json` v1.5: Added `RW` (Review Warranted); renamed `TR` → "Trigger", `CL` → "Closed"; added registry note on `HL` namespace-collision history.
- `states.json` v1.5: Added `TL` (Timelocked); corrected `PD`/`CD` finality permissions to `[0]`; removed `HL` (namespace collision with Policy) and `ER` (validation outcome, not a persisted state).
- `verification_regimes.json` v1.5: Renamed `chain_native` → `native_canonical` (alias retained for backward compatibility); temporarily added draft `legal_oracle_structured` (later superseded in v1.5.2 by structured and multiple legal attestation regimes).
- `finality_basis.json` (new): Seven entries — `optimistic_window`, `probabilistic`, `legal_appeal`, `custodial_clawback`, `staking_lock`, `vesting_schedule`, `protocol_pending`. Required when Finality=0 (FB-001).
- `chain_registry.json` v2.0: Full schema restructure with `protocol_required` / `advisory` / `provenance` / `anchoring` blocks. Entries: ETH, BTC, SOL, BASE, ARB.
- `origins.json`: Updated BTC and ETH `verification_regime` to `native_canonical`.
- `reason_codes.json`: Added `LEG_POLICY_RANGE_VIOLATION`, `LEG_SUPRA_SANCTIONS_FORCE`, `LEG_SUBTYPE_ABSENT`.
- `compliance_extensions.json`: Added `local_forum` to `legal_instrument` vocabulary.
- `registries/drafts/leg_supra_jurisdictions.json` (new): Eight supranational jurisdictions — EU, EBA, ESMA, ECB, UNSC, FATF, ICC, ICSID.
- `registries/index.json`: Bumped to `1.5.0-draft`; added `finality_basis` registry entry; updated chain_registry and verification_regimes descriptions.

**Validation Rules (v1.5-draft)**
- FB Rules section added: FB-001 — `finality_basis` required when Finality=0.
- Bridge stack scope note added: SEM-017/019/BRG-001–006 are conditionally inactive without `bridge_id`.
- SEM-013 retired (superseded by SEM-014).
- SEM-018 reclassified as MSG-005 (schema constraint, not semantic).
- LFC label corrections: LFC-001=event_id uniqueness, LFC-002=lifecycle_id present, LFC-003=related_events resolve, LFC-006 retired.
- LEG Policy Derivation Gate (Step 5b): LEG-004–009 (subtype, confidence floors, policy range, SUPRA sanctions force, finality warnings); LEG-010–011 (jurisdiction structure); LEG-014 retired; LEG-012-EXT/LEG-013-EXT moved to Compliance Module.
- Validation pipeline updated: Step 5a (FB-001), Step 5b (LEG gate).

**Spec (v1.5-draft)**
- New `arrow-protocol-v1.5-draft.md` (v1.4 preserved).
- §2.5: Tier-3 finality fields (`finality_basis`, `finality_eta`, `finality_eta_type`); FN+0 definition.
- §6.5–6.6: Finality Invariants and normative State × Finality Matrix.
- Part 17: LEG subtype vocabulary (JUD/ADM/SUPRA/LEG_TEXT/PREC), `leg_jurisdiction` object, policy derivation table, supra jurisdictions reference.
- `chain_native` → `native_canonical` throughout (3 spec occurrences).

**RFCs**
- RFC-003 promoted to final non-normative architecture RFC in `rfc/RFC-003-Arrow-as-Interoperable-State-Machine-v1.md`.
- RFC-004 and RFC-005 promoted to final normative RFCs in `rfc/`; RFC-004 clarifies finality-locked semantics and adds staking/yield and vesting finality bases; RFC-005 strengthens Origin Adapter identity binding and PRV enforcement.
- RFC-006 remains draft as `rfc/drafts/RFC-006-Robust-Chain-Registry-v0_3.md`, with independent lifecycle test and periodic anchored verification requirements added.

**LEG Amendments**
- `docs/architecture/drafts/leg-amendment-v0.1.md` — LEG policy derivation gate, leg_subtype vocabulary.
- `docs/architecture/drafts/leg-amendment-jurisdiction-addendum-v0.1.md` — leg_jurisdiction structure, supra jurisdictions.
- `docs/architecture/drafts/legal-system-primitive-v0.1.md` — LEGAL_SYSTEM primitive draft (v1.5+ deferred).
- `registries/drafts/legal_systems.json` + schema.

**Rust (arrow-core)**
- Origin regex tightened: `[A-Z]{3}` exact (was `[A-Z0-9]{2,6}`). ORG-001 message updated.
- LFC labels corrected in `pipeline.rs` and dead-code validation files.
- FB-001 pipeline step added to `pipeline.rs`.
- `finality_basis` field + `FinalityBasisEntry` added to event struct and registry loader.
- `leg_subtype`, `leg_jurisdiction` fields + `LegJurisdiction` struct added to event struct.
- `LegSupraJurisdictionEntry` added to registry types; loader reads `registries/drafts/leg_supra_jurisdictions.json`.
- LEG-004 and LEG-006 checks added to pipeline (minimal first pass; full LEG-007 derivation table deferred).
- Chain registry v2.0 struct (`ChainProtocolRequired`, `ChainAdvisory`, `ChainRegistryEntry`) in registry types.

## v1.4 (2026-05-24)

**Spec**
- New `docs/protocol/arrow-protocol-v1.4.md` — full protocol specification.
- New `docs/protocol/validation-rules-v1.4.md` — validation rules including SEM-017 (Proof Classification Binding), MSG-005 (Classification Binding Method Registry Membership), SEM-019 (Economic Consistency Binding), BRG-001–006 (Bridge Registry rules).

**Registries**
- `bridge_registry.json` (new): Per-bridge trust entries with `bridge_trust_flag` kill-switch, `classification_profile`, and `economic_consistency_profile`.
- `confidence_subprofiles.json` (new): Confidence subprofile vocabulary for BRG-006.
- `classification_binding_methods.json` (new): Controlled vocabulary for `proof_semantics.classification_binding_method` (MSG-005).
- `confidence.json`: Extended with numeric scoring fields for multi-source composition.
- `reason_codes.json`: Added `PROOF_TYPE_UNBOUND`, `CLASSIFICATION_REGIME_CEILING`, `ECONOMIC_CONSISTENCY_UNBOUND`, `BRIDGE_TRUST_REVOKED`, `BRIDGE_RESTORATION_BLOCKED`.
- `verification_regimes.json`: Added `classification_bound` field to all entries; added `native_notarized_consensus` regime.
- `registries/index.json`: Updated for v1.4 — registered bridge, subprofile, and classification-method registries.

**Rust (arrow-core)**
- `validation/bridge.rs` (new): BRG-001–006, SEM-017, SEM-019 bridge validation rules.
- `types/event.rs`: Added `bridge_id`, `proof_semantics`, `economic_consistency` event fields.
- `types/registry.rs`: Added `BridgeRegistryEntry`, `BridgeClassificationProfile`, `BridgeEconomicConsistencyProfile`, `ConfidenceSubprofileEntry`, `ClassificationBindingMethodEntry`.
- `registry/loader.rs`: Loads bridge_registry, confidence_subprofiles, classification_binding_methods.
- `validation/pipeline.rs`: Integrated bridge validation at Steps 3a–3b/6–9.

**Drafts (staging)**
- `staging/incoming/legal-system-primitive-v0.1.md` — LEGAL_SYSTEM primitive draft.
- `staging/incoming/legal_systems.json` + schema.
- `rfc/RFC-003-Arrow-as-Interoperable-State-Machine-v1.md` — current RFC-003 final architecture RFC.

## v1.3.2

- Added Arrow HTTP REST transport binding.
- Added `registries/channel_bindings.json`.
- Added Rust pilot crates for Arrow core, CLI, transport, and node.
- Added replay-store SQLite design note.
