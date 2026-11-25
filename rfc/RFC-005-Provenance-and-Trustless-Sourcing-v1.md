# RFC-005 — Provenance, Trustless Sourcing, and Adapter Interchange

| Field | Value |
|---|---|
| Document ID | ARROW-RFC-005 |
| Version | v1.1-draft |
| Status | Draft Amendment — Normative Candidate |
| Date | 2026-06-01 |
| Track | Protocol (normative) |
| Author | Arrow Protocol — Editor |
| Depends on | Arrow Protocol v1.5.1-draft; RFC-003 v1.0; RFC-004 v1.0; RFC-005 v1.0 |
| Targets | Arrow Protocol post-v1.5.1 adapter layer |
| Genesis Anchor | RFC-000 — `2026-03-22T18:40:00Z` |
| Supersedes | RFC-005 v1.0 upon finalization |
| Informs | RFC-006; future Transport Model v2.0; future Adapter Security RFC |

---

## 0. Status of This Draft

RFC-005 v1.0 specified the Provenance Principle, new provenance rules, registry obligations, and the normative Origin Adapter contract. This v1.1 draft preserves that contract and adds the missing counterpart: the normative Consumer Adapter contract and the Arrow Adapter Interchange Contract.

RFC-005's central claim is architectural: **the trustworthiness of an Arrow event is determined by how it was observed, not by which origin it names.** This principle is made normative here, expressed as concrete rules, and applied to the Origin Adapter, the registry layer, and the `verification_regime` field.

This amendment does not change the immutable six-field Arrow string. It does not turn Arrow into a bridge, settlement network, legal-execution engine, or asset mover. It specifies how heterogeneous Origin Adapters and Consumer Adapters interoperate without pairwise source-to-destination integrations.

---

## 1. Purpose

Arrow Protocol v1.5 has a well-formed ingestion model (Origin Adapter, RFC-003) and a correctly specified field layer (RFC-004). What it lacks is a normative answer to: *where does trust actually enter the system, and how does the protocol enforce that the claimed trust level reflects the actual observation?*

The answer exists in partial form. SEM-015 caps Confidence at the regime ceiling. SEM-017 prevents a bridge adapter from claiming classification it cannot prove. SAN-001 and BRG-001 force Policy on external signals. These are all applications of the same principle: **a claim about external state must be backed by observation adequate to support it.**

RFC-005 makes this principle general, uniform, and enforceable across all fields and all origins — not only bridges.

RFC-005 specifies:

1. The **Provenance Principle** — generalized from SEM-017.
2. The **Four Provenance Classes** — the taxonomy of how any field's value can be established.
3. The **normative Origin Adapter contract** — what an adapter must observe, sign, and attest.
4. The **regime-binding rule** — `verification_regime` MUST reflect actual observation method.
5. The **derive-don't-assert enforcement** — chain-derivable fields on chain-native events MUST be protocol-derived; submitter assertion is rejected.
6. **Library provenance obligations** — the Chain Registry and Origin Registry must carry sourced `provenance` blocks; RFC-006 operationalizes this for the Chain Registry specifically.
7. **The honest limit** — Arrow Core verifies provenance consistency; it does not re-execute chains.
8. The **Arrow Adapter Interchange Contract** — the normalized boundary between Origin and Consumer Adapters.
9. The **normative Consumer Adapter contract** — receiver binding, re-verification, replay/idempotency, reduction, typed intent, receipt, dry-run, and fail-closed behavior.

Arrow requires two adapter boundaries:

1. An **Origin Adapter**, which observes a Source of Truth and mints a normalized Arrow event.
2. A **Consumer Adapter**, which receives a normalized Arrow lifecycle, re-verifies it, reduces it to a disposition, and applies a destination-specific action only where authorized.

The normative architecture is:

```text
N source domains -> N Origin Adapters -> Arrow Adapter Interchange Contract
Arrow Adapter Interchange Contract -> N Consumer Adapters -> N destination domains
```

It is not:

```text
N source domains x N destination domains -> N² pairwise adapters
```

The Arrow Adapter Interchange Contract is the normalization boundary. After an Origin Adapter mints a conforming Arrow event, a Consumer Adapter must not require the source domain's raw internal data model to determine the event's Arrow meaning.

---

## 2. The Provenance Principle

### 2.1 The general statement

**RFC-005 Provenance Principle:** Any field in an Arrow event that purports to convey trust — including `Confidence`, `verification_regime`, `Finality`, `finality_basis`, `proof_type`, `proof_ref`, `chain_trust_flag`, and `bridge_trust_flag` — MUST be set to a value consistent with the actual observation method used to establish the underlying fact. A field set to a value that the actual observation method cannot support is a provenance violation and MUST be rejected.

This principle is the generalization of SEM-017. RFC-002's SEM-017 states: "A proof of existence is not a proof of classification — a bridge adapter cannot claim it verified the source event's classification unless its proof actually covers the classification claim." RFC-005 extends that prohibition to the entire protocol: no field may claim a provenance quality the observation cannot supply.

### 2.2 Precedent in the existing protocol

The Provenance Principle is not new to the protocol's intentions. It is already instantiated in three places:

| Existing rule | Provenance principle applied to |
|---|---|
| SEM-015 | Confidence ceiling: claim must not exceed what the declared regime can support |
| SEM-017 | Bridge classification: the proof must cover the claim |
| BRG-001, SAN-001, SEM-016 | Trust-flag overrides: external data overrides the submitter's claim when those signals are authoritative |

RFC-005 makes these instances of a general rule, rather than isolated per-feature constraints. The general rule is stated once; the existing instances become corollaries.

### 2.3 What the principle does not require

The Provenance Principle does not require Arrow Core to independently verify every claim. Arrow Core verifies *consistency*: that the observation method declared by the Origin Adapter is capable of supporting the Confidence and field values claimed. It does not re-execute the Bitcoin blockchain or re-read an Ethereum node. The Origin Adapter observes reality; Arrow Core verifies that the adapter's declared observation method is capable of what the adapter claims.

This is the same model SEM-015 already uses: Arrow does not run a node; it checks that the declared regime is capable of the claimed Confidence.

---

## 3. The Four Provenance Classes

Every field value in an Arrow event is established by one of four methods. The Provenance Class of a field determines the maximum Confidence level that may be claimed for an event that depends on it, and whether the field may be asserted by the submitter or must be derived.

### 3.1 Class I — Chain-Observed

**Definition:** The value is established by independent re-reading or cryptographic verification of canonical chain state, without reliance on any trusted third party.

**Instruments:** Merkle inclusion proof against a finalized block header; ZK state proof; light-client proof; on-chain log re-verification.

**Trust basis:** The cryptographic proof itself. The identity of the observer is irrelevant — any party can verify the proof independently.

**Confidence ceiling:** A.

**Assert or derive?** For chain-native events, chain-derivable facts (block inclusion, finality, chain ID, challenge window) MUST be derived from chain state and/or the Chain Registry. They may not be asserted by the submitter. See §6.

**Examples:** The fact that a transaction with a given hash was included in Ethereum block 21,000,000; the fact that a Base transaction's challenge window has not yet elapsed.

### 3.2 Class II — Library-Derived

**Definition:** The value is established by lookup in a versioned, sourced, authoritative registry. The registry entry is itself auditable: it carries a `provenance` block identifying the source (dev docs, chain RFC, audited contract, whitepaper) and the verification date.

**Instruments:** Chain Registry lookup for finality model, challenge-window duration, reorg depth; Origin Registry lookup for available regimes; Verification Regime Registry.

**Trust basis:** The registry entry's own provenance. A registry entry sourced to an audited contract and verified recently is more trustworthy than one sourced to "manual assessment" from two years ago. RFC-006 operationalizes this for the Chain Registry.

**Confidence ceiling:** Bounded by the registry entry's source quality. A well-sourced, recently verified Chain Registry entry supports B-level reliance; a stale or manually-assessed entry supports C.

**Assert or derive?** MUST be derived from the registry. The submitter never asserts a field whose value is determinable by registry lookup.

**Examples:** Base's `finality_model = optimistic_rollup`; Ethereum's `reorg_safety_confirmations = 12`; the LEG origin's `max_confidence = C` for manual `legal_attestation` (`B` requires `structured_legal_attestation` or `multiple_legal_attestation`; `legal_authority_panel` is authority/finality metadata).

### 3.3 Class III — Attested

**Definition:** The value is established by a registered, accountable entity that signs an attestation. The entity's identity is verifiable; its attestation is traceable to a specific key or credential.

**Instruments:** Custodian signed balance attestation; structured legal attestation (RFC-004 / LEG amendment); institutional oracle signed report.

**Trust basis:** The attesting entity's integrity and accountability. Multiple independent attestors increase confidence.

**Confidence ceiling:** B (with multi-attesting panel or bonded oracle quorum); C (single attesting entity).

**Assert or derive?** Attested values are submitted (the attestation itself is the input), but MUST carry a verifiable `proof_ref` and `asserted_by` that permit independent verification of the attestation.

**Examples:** A custodian's attestation that a balance is held; a legal oracle's structured attestation that a court order was issued; an AML/KYC provider's signed clearance.

### 3.4 Class IV — Self-Declared

**Definition:** The value is provided by the submitter and cannot be independently verified by any external source. No chain state, registry, or external attestor can confirm it.

**Instruments:** KYC identity data; travel rule party information; declared intent; user-provided metadata.

**Trust basis:** None external. Trust is entirely placed in the submitter's honesty.

**Confidence ceiling:** The protocol places no floor but places this class inside `extensions.*`, where it is always below the core event's Confidence level. Specifically, a Class IV field cannot elevate the event's overall Confidence.

**Assert or derive?** This is the only class where submission is the correct input model — because no derivation is possible. Class IV fields MUST be in `extensions.*`. They MUST NOT appear in the canonical string or in Tier-2 required Event Object fields.

**Examples:** The identity of a travel rule recipient; the purpose of a transaction as declared by the sender; user-provided jurisdiction assertions where no legal oracle is available.

### 3.5 Provenance class assignment — selected fields

| Field | Provenance class | Notes |
|---|---|---|
| `Finality` (chain-native events) | I (derive) / II (from Registry) | Derived from chain state or Chain Registry. Never asserted for chain-native events. |
| `finality_basis` (chain-native events) | II (derive from Chain Registry) | Set by Origin Adapter from Chain Registry entry. |
| `chain_id` | I or II | Read from the chain itself or derived from Chain Registry by Origin code. |
| `verification_regime` | Set by Origin Adapter | Must reflect actual observation method. See §5. |
| `Confidence` | Constrained by regime | SEM-015; cannot exceed regime ceiling. |
| `proof_type`, `proof_ref` | III (Attested) for LEG; I for chain-native | The proof must be adequate for the declared class. |
| `asserted_by` | III (Attested) | Must resolve to a registered identity for regimes that require it. |
| `extensions.travel_rule.*` | IV (Self-Declared) | In extensions; cannot elevate core Confidence. |
| `extensions.legal_instrument.*` | III (Attested) or IV | Depending on whether a legal oracle is present. |
| `chain_trust_flag` | II (Library-Derived) | From Bridge Registry / Chain Registry. Never asserted by submitter. |
| `leg_subtype` | III (Attested) | The Origin Adapter classifies the legal authority; must be backed by `proof_ref`. |

---

## 4. The Normative Origin Adapter Contract

### 4.1 Overview

An Origin Adapter is the component that observes a Source of Truth and mints Arrow events. RFC-003 defined the Origin Adapter architecturally; RFC-005 specifies its normative obligations.

An Origin Adapter is not Arrow Core. The protocol cannot run every possible Origin Adapter — the set is open (any chain, any legal system, any custodian). The protocol can, however, specify what a conforming Origin Adapter must do and must not do, and can reject events whose provenance metadata is inconsistent with a conforming adapter.

### 4.2 Normative obligations

A conforming Origin Adapter MUST:

**OA-001 — Observation adequacy.** Observe the Source of Truth by a method that is capable of supporting the `verification_regime` it declares. An adapter that calls a single RPC endpoint MUST NOT declare `native_canonical`. An adapter that verifies a Merkle proof against a finalized header MAY declare `native_canonical`.

**OA-002 — Regime reflects method.** Set `verification_regime` to accurately reflect the actual observation method used for the specific event being minted — not the best-case regime available to the Origin class in general.

**OA-003 — Confidence within ceiling.** Set `Confidence` within the ceiling permitted by the declared `verification_regime`. Do not claim a Confidence level the regime cannot support.

**OA-004 — Derive chain-derivable fields.** For chain-native events: derive `Finality`, `finality_basis`, `chain_id`, and `finality_eta` from chain state and/or the Chain Registry. Do not accept or relay these values from the submitter. A submitter's assertion of chain state is untrusted; the adapter's observation is the authoritative source.

**OA-005 — Attach adequate proof.** Attach a `proof_ref` adequate for the declared regime and Provenance Class. For `native_canonical`: a Merkle proof and block hash. For `institutional_attestation`: a signed API response or equivalent. For `legal_attestation`: the legal instrument, legal attestation record, or structured attestation record per LEG amendment §5.

**OA-006 — Independently bound identity for attestation regimes.** For regimes requiring accountability (`legal_attestation`, `institutional_attestation`, `bonded_quorum`, and future multi-attested regimes), the adapter MUST populate `asserted_by` with an identity reference for the entity making the attestation. That identity MUST be either:

1. self-evident from the proof artifact itself, where the signature, credential, or source-of-truth system independently identifies the attesting entity; or
2. confirmed by an independently maintained identity record, such as a public registry, authoritative institutional record, or Arrow-maintained registry.

A DID by itself is not sufficient unless it is independently bound to the attesting entity. The identity binding SHOULD identify the controller, key material or credential, scope of authority, effective dates, and revocation status.

**OA-007 — Chain Registry version pinning.** When deriving `finality_basis` and `finality_eta` from the Chain Registry, the adapter MUST record the registry version it used (`chain_registry_version_at_event`). This permits downstream verification that the derivation was based on a specific, retrievable registry state.

**OA-008 — No fabrication.** An Origin Adapter MUST NOT mint events asserting state it has not observed. It MUST NOT relay state asserted by the submitter for fields in Provenance Classes I or II without independent verification.

**OA-009 — Normalized interchange output.** An Origin Adapter MUST emit a normalized event conforming to the Arrow Adapter Interchange Contract. Source-specific observation logic MUST terminate before the interchange boundary.

**OA-010 — Portable proof references and registry pins.** An Origin Adapter MUST emit typed proof references and registry-version pins sufficient for downstream re-verification where the declared regime requires them.

**OA-011 — No destination execution in arbitrary metadata.** An Origin Adapter MUST NOT embed destination execution instructions inside arbitrary metadata. Origin output records normalized source facts and provenance. Any destination action evaluated by a Consumer Adapter MUST be represented as a typed action intent at the consumer/deployment boundary, not inferred from source-specific raw metadata.

### 4.3 What Arrow Core enforces

Arrow Core cannot independently verify observation — it does not run nodes. What it CAN enforce:

- That `verification_regime` is a registered value (MSG-004).
- That `Confidence` does not exceed the regime ceiling (SEM-015).
- That Provenance Class I / II fields are not populated in positions where submitter assertion is forbidden (new rule PRV-001; see §6).
- That `proof_ref` is present when required by the regime.
- That `asserted_by` is present when required by the regime.

Arrow Core's provenance enforcement is **consistency verification**, not observation re-execution. It catches adapters that claim a regime but fail to populate its required artifacts. It cannot catch an adapter that populates all required fields with fabricated data — that is an accountability problem addressed by OA-006 (independent identity binding) and OA-008 (prohibition on fabrication), not by protocol rules.

### 4.4 Origin Adapter registry (optional, deployment-layer)

This RFC does not mandate a canonical Origin Adapter registry at the protocol level. Deployments MAY maintain a registry of conforming Origin Adapters. If a deployment operates such a registry, Arrow recommends — but does not require — that `asserted_by` on chain-native events reference the Origin Adapter identity, paralleling the legal oracle identity model in the LEG amendment.

A future RFC may promote this to a normative requirement if security analysis warrants it.

---

## 5. Regime-Binding Rule

### 5.1 The over-claim correction

As identified in RFC-003 §6 and RFC-004 Part 3, the current Origin Registry hard-codes `ETH → native_canonical` as if all Ethereum events are observed via self-verifying proof. This is incorrect. The regime is a property of the observation method, not of the origin.

### 5.2 New normative rule: PRV-002

| Rule | Severity | Constraint |
|---|---|---|
| **PRV-002** | Fatal | The `verification_regime` field MUST reflect the observation method actually used by the Origin Adapter to mint the event. The regime entry in the Origin Registry represents the *maximum available* regime for that origin class — it does not represent the default or assumed regime. An event that declares a regime whose artifact requirements are not met by the attached `proof_ref` MUST be rejected. |

**Corollary:** The Origin Registry must be amended to change per-origin regime entries from "this origin uses this regime" to "this origin's maximum available regime is this; actual regime is set by the Origin Adapter per OA-002."

### 5.3 Origin Registry amendment

For each chain-type origin entry, replace the `"regime"` field with:

```json
{
  "max_regime": "native_canonical",
  "max_confidence": "A",
  "regime_note": "Maximum available when observation is via self-verifying proof. Actual regime set by the Origin Adapter per OA-002. Most deployments using RPC APIs will use institutional_attestation (max Confidence C)."
}
```

This amendment is a documentation correction in the registry. The validation rule PRV-002 provides the enforcement.

### 5.4 Confidence implication

Most deployed Arrow systems today rely on RPC API observation. Under PRV-002, those systems MUST declare `institutional_attestation` and claim Confidence C maximum. This is a *downward correction* of the effective Confidence for many existing event streams. It is the honest accounting. The multi-source Confidence upgrade (RFC-003 §17) provides a path to upgrade from C toward B without requiring every operator to run a full node.

---

## 6. Derive-Don't-Assert Enforcement

### 6.1 The principle

For fields in Provenance Classes I and II, the value is determinable from chain state or a registry. A conforming Origin Adapter derives these values; it does not accept or relay them from the submitter. The protocol enforces this by rejecting events where the declared observation method is inconsistent with a field value the adapter should have derived.

### 6.2 New normative rule: PRV-001

| Rule | Severity | Constraint |
|---|---|---|
| **PRV-001** | Fatal | For chain-native events (`Origin` resolves to a registered chain-type origin and `verification_regime` is `native_canonical` or `institutional_attestation`): the fields `Finality`, `finality_basis`, `chain_id`, and `chain_registry_version_at_event` MUST be set by the Origin Adapter from chain state and/or the Chain Registry. A submitter-provided value for these fields, where it conflicts with the Chain Registry entry or chain state, MUST be rejected. Where it is indistinguishable, OA-004 (the adapter obligation) is the enforcement mechanism. |

PRV-001 is the protocol enforcement hook for OA-004 and OA-008. It requires the Origin Adapter output to be derived from chain state and/or the Chain Registry for Provenance Classes I and II, and it rejects inconsistent submitter assertions. The trusted runtime boundary by which an adapter proves that it actually performed the derivation — hardware enclave, HSM signing path, reproducible node process, remote attestation, or another mechanism — is not specified here. That implementation-security layer is explicitly deferred to a future Origin Adapter implementation/security RFC.

**Practical meaning:** An event submitted with `1:ETH:FN:PR:0:A` where the chain state at the referenced block shows the transaction is included with sufficient confirmations for `FN+1` MUST be rejected — the `Finality = 0` claim is inconsistent with chain state. Conversely, an event submitted with `1:BASE:FN:PR:1:B` for a transaction on Base (an optimistic rollup with a 7-day challenge window) MUST be rejected — `Finality = 1` cannot be claimed for a Base transaction within the challenge window.

### 6.3 Scope limitation

PRV-001 applies to chain-native events only. For custodial events (`Origin = CST`), legal events (`Origin = LEG`), and oracle events (`Origin = ORC`), finality is determined by the attesting entity (Class III), not by chain state. For those origins, `Finality` is a Class III attested field — the attestor declares it, and the protocol enforces only that the declaration is consistent with the regime and `leg_subtype` constraints.

---

## 7. Arrow Adapter Interchange Contract

### 7.1 Definition

The **Arrow Adapter Interchange Contract (AAIC)** is the normalized protocol boundary between Origin Adapters and Consumer Adapters.

A conforming Origin Adapter emits:

1. the immutable six-field Arrow code;
2. the Tier-2 `ArrowEvent` object;
3. provenance metadata adequate for the declared `verification_regime`;
4. typed source facts or proof references required by the event;
5. registry-version pins required for deterministic downstream verification;
6. a signed `TransportEnvelope` when transported over an authenticated binding.

A conforming Consumer Adapter receives:

1. one signed `TransportEnvelope` or a verified event log;
2. the normalized `ArrowEvent` lifecycle;
3. provenance metadata and proof references;
4. a typed destination action intent when the deployment requests external action;
5. destination policy and execution configuration.

The Consumer Adapter then re-verifies, re-validates, reduces, authorizes, and either executes, records, quarantines, rejects, or routes for human review.

### 7.2 Source independence

A Consumer Adapter MUST NOT require source-specific raw structures after the Origin Adapter has minted the normalized Arrow event, except where a proof-verification module explicitly re-checks a referenced proof artifact.

A Consumer Adapter MAY inspect:

- the `Origin` code;
- normalized source facts;
- provenance metadata;
- proof references;
- lifecycle links;
- validation output;
- reduction disposition;
- typed destination action intent;
- destination execution configuration.

A Consumer Adapter MUST NOT infer semantic meaning or trigger external execution from arbitrary source-specific metadata.

### 7.3 Destination specificity

A Consumer Adapter is destination-specific by design. It translates an Arrow disposition plus a typed action intent into a destination-domain action. It may reject an action that its destination cannot safely execute.

Examples:

- an Ethereum Virtual Machine (EVM) Consumer Adapter may submit a signed EVM transaction;
- a Bitcoin Consumer Adapter may construct or broadcast a UTXO-aware transaction;
- a Solana Consumer Adapter may submit a Solana instruction;
- a LEG Consumer Adapter may create a structured workflow item, review queue record, hold/block instruction, or notarized receipt.

Destination specificity does not permit source specificity to leak across the AAIC boundary.

### 7.4 Interchange contract versioning

AAIC implementations SHOULD version their contract surface independently from the six-field Arrow code. A conforming implementation MAY expose a structured contract object containing adapter identity, envelope, normalized source facts, optional action intent, and receipt references. Versioning the contract object does not mutate the Arrow event itself.

---

## 8. Normative Consumer Adapter Contract

### 8.1 Overview

A Consumer Adapter is the destination-domain component that receives a normalized Arrow lifecycle and applies a destination-specific action only after independent verification and reduction.

A Consumer Adapter is not Arrow Core. It does not determine the truth of an external event. It enforces the destination boundary: no action proceeds unless the received Arrow lifecycle is authentic, coherent, unreplayed, correctly addressed, reduced to an authorizing disposition, and paired with a valid typed action intent.

### 8.2 Obligations

A conforming Consumer Adapter MUST:

**CA-001 — Receiver binding.** Confirm that the Consumer Adapter or its deployment identity is an intended receiver.

**CA-002 — Explicit sender policy.** Require an explicit non-empty allowed-sender policy. An empty allow-list MUST fail closed.

**CA-003 — Signature and provenance re-verification.** Re-verify the signed envelope and the provenance metadata required by the declared regime. A prior Arrow Node ACK does not replace Consumer Adapter verification.

**CA-004 — Replay and idempotency protection.** Reject replayed envelopes and prevent duplicate destination execution. Replay detection and destination-execution idempotency are separate controls and both MUST be addressed.

**CA-005 — Lifecycle re-validation.** Re-run the canonical `ValidationPipeline` over every event in the lifecycle log before reduction.

**CA-006 — Deterministic reduction.** Run the canonical Arrow reduction layer and obtain a typed disposition before evaluating any destination action.

**CA-007 — Disposition-gated action.** Refuse execution unless the reduced disposition authorizes the requested typed action.

**CA-008 — Typed action intent.** Accept only a typed destination action intent. Arbitrary metadata MUST NOT independently trigger settlement, transfer, legal action, or other external execution.

**CA-009 — Destination validation.** Validate the typed action against the destination domain's execution model, chain identifier, asset or object type, account model, amount rules, and configured limits.

**CA-010 — Destination-specific execution module.** Isolate execution logic behind a destination-specific module. EVM, UTXO, Solana, institutional, and LEG execution models MUST NOT be silently coerced into a single chain model.

**CA-011 — Typed receipt.** Return a typed Consumer Receipt stating whether the action was executed, recorded, deferred, quarantined, rejected, or routed for review, with a destination reference and typed failure reason where applicable.

**CA-012 — Dry-run and record-only support.** Provide a dry-run or record-only mode so semantic interoperability can be tested without external execution.

**CA-013 — Fail closed.** Any signature, provenance, replay, lifecycle, reduction, typed-intent, destination-validation, or execution uncertainty MUST fail closed or enter an explicit quarantine/review state.

**CA-014 — No legal-effect overclaim.** A LEG Consumer Adapter MUST distinguish technical workflow output from legal enforceability. It MUST NOT represent that software output alone creates, modifies, reverses, or extinguishes legal rights unless an independently valid legal mechanism authorizes that result.

### 8.3 Consumer Receipt

A Consumer Receipt SHOULD include:

```json
{
  "receipt_version": "arrow-consumer-receipt-v1",
  "consumer_adapter": "arrow-consume-<domain>",
  "mode": "execute | dry_run | record_only",
  "status": "executed | recorded_only | dry_run | no_action | quarantined | rejected",
  "envelope_id": "...",
  "event_id": "...",
  "lifecycle_id": "...",
  "disposition": "ApprovedForSettlement | Pending | Blocked | Reversed | Quarantined | Closed",
  "policy": "PR | BL | HL | RW | TR | CL",
  "action_intent": { "...": "typed destination action, when applicable" },
  "destination_execution_id": "...",
  "destination_ref": "...",
  "provenance": { "...": "verified signer and key reference" },
  "recorded_at": "RFC3339 timestamp"
}
```

The exact schema may evolve by implementation profile. The normative requirement is a typed, auditable receipt.

---

## 9. Typed Action Intent

### 9.1 Principle

Arrow semantic disposition and destination execution are separate layers.

The reduction layer answers:

> Given this validated lifecycle, what is the current Arrow disposition?

The typed action intent answers:

> What destination-specific action is requested if the disposition authorizes it?

The Consumer Adapter answers:

> Is that typed action valid and executable in this destination domain under this deployment policy?

### 9.2 Requirement

A conforming Consumer Adapter MUST NOT execute an external action based solely on free-form payload metadata.

A typed action-intent schema MUST be used. The schema MAY support destination-specific variants, including:

- EVM transfer or contract call;
- Bitcoin UTXO transfer;
- Solana transfer or instruction;
- record-only action;
- LEG review or workflow action;
- institutional hold/block instruction.

The schema MUST prevent source-domain raw structures from becoming the interoperability boundary.

### 9.3 Normalized source facts are not action intent

Origin Adapters may emit typed source facts to avoid free-form metadata coupling. For example, an Ethereum Origin Adapter may emit a typed source value-transfer observation. That source fact is not itself a destination action. A Consumer Adapter or deployment policy translates normalized source facts into destination action intent only at the destination boundary.

---

## 10. LEG Adapter Profile v0.1

### 10.1 LEG Origin Adapter

A LEG Origin Adapter ingests a structured legal attestation and mints a normalized Arrow event. It MUST NOT treat an unstructured document dump as sufficient provenance for high-confidence automated action.

A structured LEG attestation SHOULD include:

- `leg_subtype`;
- `leg_jurisdiction`;
- `legal_system_id`, where the LEGAL_SYSTEM primitive is used;
- `asserted_by`;
- `proof_type`;
- `proof_ref`;
- authority or source reference;
- effective date;
- appeal, review, or challenge status;
- `Finality`;
- `finality_basis`;
- reason code;
- requested policy;
- credential or signature reference;
- registry-version pins.

The adapter MUST use an honest verification regime, generally `legal_attestation` for single-source legal facts or `bonded_quorum` for oracle-network attestations, and remain within the applicable Confidence ceiling.

### 10.2 LEG Consumer Adapter

A LEG Consumer Adapter consumes validated Arrow lifecycles but does not itself create legal effect.

A `v0.1` LEG Consumer Adapter SHOULD default to a human-in-the-loop workflow and MAY output:

- a structured legal workflow instruction;
- a review queue item;
- a compliance hold or block instruction;
- a record-only notarized receipt;
- a human-approval request;
- a typed downstream instruction for an authorized institutional system.

Automatic external legal execution is out of scope for `v0.1`.

---

## 11. Adapter Interoperability Requirement

### 11.1 N + N interoperability

Any conforming Origin Adapter MUST be capable of emitting AAIC-conforming output that any conforming Consumer Adapter can parse, verify, and reduce.

This does not mean every Consumer Adapter must execute every action. A destination may reject an incompatible typed action with an explicit typed error.

### 11.2 Conformance matrix

Implementations SHOULD maintain a cross-adapter test matrix:

```text
for each Origin Adapter:
    mint normalized signed event
    for each Consumer Adapter:
        consume in dry-run or record-only mode
        verify parse/auth/provenance/reduction behavior
        assert expected execution eligibility or typed incompatibility error
```

The test proves semantic interoperability, not universal execution capability.

---

## 12. Library Provenance Obligations

### 12.1 The Chain Registry and Origin Registry as trust anchors

The Chain Registry and Origin Registry are used to derive protocol-level field values. Their entries are inputs to the validation pipeline, not merely documentation. A poisoned or stale registry entry propagates incorrect field values into every event minted from it.

RFC-004 Part 11 introduced the optional `provenance` block in registry entries. RFC-005 makes `provenance` **mandatory** for Chain Registry and Origin Registry entries that are used to derive Provenance Class II fields.

### 12.2 New rule: PRV-003

| Rule | Severity | Constraint |
|---|---|---|
| **PRV-003** | Fatal | Any Chain Registry entry used to derive `finality_basis`, `finality_eta`, or `chain_trust_flag` MUST include a complete `provenance` block per the schema in RFC-004 §11.2. An entry without a `provenance` block MUST NOT be used for derivation in a v1.5-conforming deployment. |

### 12.3 Registry tamper-evidence

For the Chain Registry specifically: the registry hash (SHA-256 of the canonical JSON) SHOULD be anchored on-chain at each version release, creating a tamper-evident record of the registry state that Origin Adapters and consumers can verify. RFC-006 specifies the ratification and anchoring process in detail.

A deployment that pins a specific Chain Registry version SHOULD verify the registry hash against the on-chain anchor before using the registry for derivation.

---

## 13. The Honest Limit

This section is explicit about what RFC-005 does and does not achieve, to prevent over-claiming.

**What RFC-005 achieves:**
- Every event's provenance class is declared and verifiable by inspection.
- `verification_regime` reflects actual observation, not theoretical capability.
- Chain-derivable fields are derived, not asserted, for chain-native events.
- The Chain Registry and Origin Registry are auditable, not trusted-on-faith.
- An attacker who wants to mint a false high-Confidence event must either (a) provide a valid cryptographic proof (impossible without chain access) or (b) compromise a registered, accountable Origin Adapter (a named, traceable attack).

**What RFC-005 does not achieve:**
- Arrow Core does not re-execute any chain. Provenance enforcement is consistency verification, not observation re-execution. The protocol is as trustworthy as its Origin Adapters for Class I and II fields.
- A compromised or colluding Origin Adapter that fabricates all required fields and proofs defeats the protocol's provenance enforcement at the cryptographic level. Independent identity binding (OA-006) converts this from an anonymous attack to an attributable one — the attacker is named or linked to a revocable credential.
- Self-declared (Class IV) fields remain unverifiable by design. The protocol correctly fences them in `extensions.*` and caps their Confidence contribution.
- Library-derived fields are as trustworthy as the registry that supplies them. RFC-006 remains draft infrastructure work for registry governance, lifecycle testing, and anchoring; v1.5-conforming derivation still requires the provenance fields mandated by PRV-003.

---

## 14. New Rules Summary

| Rule | Severity | Category | Constraint |
|---|---|---|---|
| **PRV-001** | Fatal | Provenance | Chain-derivable fields on chain-native events MUST be derived from chain state / Chain Registry; submitter assertion rejected where inconsistent. |
| **PRV-002** | Fatal | Provenance | `verification_regime` MUST reflect actual observation method; regime artifacts MUST be present and adequate. |
| **PRV-003** | Fatal | Registry | Chain Registry entries used for derivation MUST carry a `provenance` block. |
| **OA-009** | Fatal | Adapter boundary | Origin Adapter MUST emit normalized AAIC-conforming output. |
| **OA-010** | Fatal | Proof portability | Origin Adapter MUST emit typed proof references and required registry-version pins. |
| **OA-011** | Fatal | Action boundary | Origin Adapter MUST NOT embed destination execution instructions in arbitrary metadata. |
| **CA-001** | Fatal | Receiver binding | Consumer Adapter MUST verify intended receiver. |
| **CA-002** | Fatal | Sender policy | Consumer Adapter MUST require non-empty sender policy. |
| **CA-003** | Fatal | Re-verification | Consumer Adapter MUST re-verify signature and required provenance. |
| **CA-004** | Fatal | Replay/idempotency | Consumer Adapter MUST prevent replay and duplicate destination execution. |
| **CA-005** | Fatal | Lifecycle validation | Consumer Adapter MUST re-run canonical validation over the lifecycle. |
| **CA-006** | Fatal | Reduction | Consumer Adapter MUST run canonical reduction before action evaluation. |
| **CA-007** | Fatal | Authorization | Destination action MUST be disposition-gated. |
| **CA-008** | Fatal | Typed intent | Arbitrary metadata MUST NOT independently trigger execution. |
| **CA-009** | Fatal | Destination validation | Consumer Adapter MUST validate destination-specific action constraints. |
| **CA-010** | Fatal | Execution isolation | Consumer Adapter MUST isolate destination-specific execution logic. |
| **CA-011** | Fatal | Receipt | Consumer Adapter MUST return a typed auditable receipt. |
| **CA-012** | Required profile behavior | Testing | Consumer Adapter MUST provide dry-run or record-only mode. |
| **CA-013** | Fatal | Failure mode | Consumer Adapter MUST fail closed or explicitly quarantine. |
| **CA-014** | Fatal | LEG boundary | LEG Consumer Adapter MUST NOT overclaim legal effect. |

PRV-001 generalizes OA-004; PRV-002 generalizes SEM-015 and SEM-017 applied to the Origin; PRV-003 mandates the `provenance` block introduced in RFC-004 Part 11. OA-009 through OA-011 and CA-001 through CA-014 extend the same provenance/accountability principle across the adapter-interchange and destination-execution boundary.

---

## 15. Security and Trust Boundaries

The AAIC does not remove trust assumptions.

- Arrow Core verifies semantic coherence and provenance consistency; it does not re-execute every source domain.
- Origin Adapters remain accountable observation boundaries.
- Consumer Adapters remain accountable destination-execution boundaries.
- A signed envelope proves control of a signing key, not the truth of an external fact.
- A Consumer Adapter receipt proves what the adapter reports it did; independent destination verification may still be required.
- A LEG Consumer Adapter can produce a structured workflow or instruction but does not eliminate courts, registries, regulators, arbitrators, custodians, trustees, counterparties, or human review where those institutions remain legally necessary.

---

## 16. Implementation Consequences

This amendment implies the following implementation work:

1. Define a shared adapter contract crate or equivalent reusable module.
2. Replace free-form settlement metadata with typed source facts and typed destination action intent.
3. Refactor concrete consumers onto the shared Consumer Adapter contract.
4. Centralize duplicated signature/provenance verification logic.
5. Add typed Consumer Receipts.
6. Add dry-run and record-only profiles.
7. Build chain-specific Origin and Consumer Adapters for live Chain Registry entries.
8. Build LEG Origin and Consumer Adapters `v0.1`.
9. Add cross-adapter conformance tests.
10. Preserve the six-field Arrow string unchanged.

---

## 17. Explicit Non-Goals

This amendment does not:

- turn Arrow into a bridge;
- require Arrow to move assets;
- claim that every destination can execute every typed action;
- make legal output self-executing;
- specify production-grade key custody;
- specify a universal settlement transaction format;
- freeze a production adapter registry;
- eliminate the need for transport, nodes, registries, courts, custodians, or institutional integrations.

---

## 18. Future Work

**Multi-source Confidence upgrade.** When an Origin Adapter observes a single RPC API, the honest ceiling is Confidence C. Arrow could run optional parallel observation infrastructure — multiple independent providers, a self-hosted node, or a light-client — and upgrade Confidence on agreement. This reuses the existing multi-source divergence machinery (DIV-001 through DIV-004). It must be strictly opt-in value-added infrastructure, never a dependency of Arrow Core or a requirement for RFC-005 compliance.

**Zero-knowledge proof path to Confidence A without full node.** ZK state proofs (Ethereum `eth_getProof` with zk-verification, Succinct's SP1, Risc Zero, or equivalent) allow an Origin Adapter to prove chain inclusion without running a full node. This is the most promising path to wide Confidence A availability. It is currently category *(b) — exists experimentally* — and warrants a dedicated RFC or a future normative update to the Origin Adapter contract when production-grade ZK light clients mature.

**Origin Adapter registry.** RFC-005 §4.4 leaves the Origin Adapter registry optional. As Arrow deployments multiply, a normative registry of conforming Origin Adapters — with identity anchoring, capability declarations, and revocation — becomes necessary for the accountability model in OA-006 to function at scale.

**Consumer Receipt schema finalization.** This draft requires typed receipts and gives a reference shape. A future profile should finalize the canonical receipt schema, including required hashes, failure codes, and destination references.

**Typed action-intent schema finalization.** This draft requires typed action intent and permits destination-specific variants. A future profile should finalize the canonical schema family for EVM, UTXO, Solana, institutional, record-only, and LEG workflow actions.

**Adapter security RFC.** Runtime isolation, key custody, remote attestation, HSM usage, reproducible adapter builds, operator accountability, and adapter revocation require a dedicated security profile.

**Transport Model v2.0.** Future transport work should align envelope addressing, receipts, replay controls, and adapter conformance reporting with the AAIC.

---

## Document Information

| Field | Value |
|---|---|
| Document ID | ARROW-RFC-005 |
| Version | v1.1-draft |
| Status | Draft Amendment — Normative Candidate |
| Genesis Anchor | RFC-000 — `2026-03-22T18:40:00Z` |
| Primary Specification | Arrow Protocol post-v1.5.1 adapter layer |
| Depends on | RFC-003 v1.0; RFC-004 v1.0; RFC-005 v1.0 |
| Preserves | RFC-005 v1.0 Provenance Principle; PRV-001; PRV-002; PRV-003 |
| Adds | Arrow Adapter Interchange Contract; OA-009..011; CA-001..014 |
| Informs | RFC-006; future Transport Model v2.0; future Adapter Security RFC |
| Governance | Editor draft; requires review before finalization |
