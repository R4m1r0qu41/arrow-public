# RFC-003 — Arrow as an Interoperable State Machine

| Field | Value |
|---|---|
| Document ID | ARROW-RFC-003 |
| Version | v1.0 |
| Status | Final — **Non-Normative Architecture RFC** |
| Date | 2026-05-25 |
| Finalized | 2026-05-26 |
| Track | Architecture (non-normative) |
| Author | Arrow Protocol — Editor |
| Depends on | Arrow Protocol v1.5-draft; Arrow Validation Rules v1.5-draft; Arrow Transport Model v1.0; Arrow Registries v1.5.0-draft |
| Genesis Anchor | RFC-000 — `2026-03-22T18:40:00Z` |
| Supersedes | RFC-003 v0.2 |
| Informs | RFC-004 (normative); RFC-005 (normative); Arrow Protocol v1.5+ |
| Finalization notes | Promoted from RFC-003 v0.3 without changing its architectural scope. Added Origin Adapter as first pipeline stage (§4); corrected pipeline diagram; added §5 (Observation Method Bounds Confidence); promoted Compatibility Level 0 from passive label to named ingestion stage (§8); amended MVP scope (§16); updated Open Questions. |

---

## 0. Status of This Document

RFC-003 is **final and non-normative**. It introduces no new validation rules, no registry values, no schema fields, and no change to the canonical six-field message string. It defines an architectural position and vocabulary for how Arrow becomes operationally real.

Concrete bindings — a normative Origin Adapter contract, a reduction-rule set, an adapter interface specification, and signature/proof primitives — are deferred to normative RFCs. Where RFC-003 names a mechanism that does not yet exist in the protocol, it says so.

This final text closes the gap identified in review: the v0.2 pipeline began at "a validated event exists" and never specified how truthful events enter the log. The Origin Adapter is the missing first stage. All other architecture is unchanged.

---

## 1. Purpose

This RFC addresses a single question: **what is the minimum that has to be true for Arrow Protocol to be operationally real, rather than a semantic messaging structure described only in the abstract?**

Arrow Core is well-specified as three things: a canonical message grammar, an Event Object schema, and a deterministic validation regime. It is **not yet** specified as a system that (a) consumes a history of events observed from real sources and (b) tells an external system what it is permitted to do. Both pieces are part of "operationally real," and both are addressed here.

RFC-003 makes three claims:

1. Arrow's path to being real runs through **interoperable execution** — external systems consuming Arrow output through adapters — not through native Arrow infrastructure.
2. Truthful events must enter the log through **Origin Adapters**: components that observe a source of truth and mint events whose `verification_regime` and `Confidence` reflect the actual observation method.
3. To be consumed, Arrow needs a defined **reduction layer**: a deterministic function that takes a validated, append-only Arrow event log and produces a current **disposition** that external systems can act on.

RFC-003 names all three. It does not yet specify the normative contracts for the Origin Adapter or the reduction rules.

---

## 2. Scope

**In scope.** The corrected full pipeline; the Origin Adapter (architectural definition); the principle that Confidence reflects observation method; the reduction-layer model; the distinction between native and interoperable execution; Arrow Compatibility Levels; the Consumption Adapter model; the relationship of RFC-003 to existing Arrow layers; the reserved status of native infrastructure; the MVP; constraints and risks.

**Out of scope.** A new blockchain; a token; a validator set; native settlement; a bridge implementation; a liquidity model; a production custody or asset-movement system; the normative Origin Adapter contract (RFC-005); the normative reduction-rule set (RFC-004 and future normative RFC); the normative adapter interface; signature and proof-of-state primitives; multi-source Confidence upgrade infrastructure.

---

## 3. Terminology

**Arrow State (field).** One of the six fields of the canonical string `Class:Origin:State:Policy:Finality:Confidence`. Its registry has five values — `FN, PD, CD, RV, TL` — each a two-character uppercase code. The State field describes the condition of one event. RFC-003 does not modify it.

**Arrow Disposition (derived).** The output of reducing an append-only log of Arrow events for a single lifecycle. A disposition answers: *given everything attested so far, what is this lifecycle's current standing, and what may act on it?* Dispositions are not Arrow State values. They are derived labels — for example `APPROVED_FOR_SETTLEMENT`, `QUARANTINED`, `BLOCKED`. In v0.3 these labels are illustrative only; a minimal canonical disposition vocabulary, if warranted, is a question for a future normative RFC.

**Arrow State Machine.** The reduction layer plus its integration surface. Formally: a deterministic function `R(event_log) → disposition`, together with the model by which external systems consume that disposition. It is not an execution environment, does not hold funds, and does not run untrusted code.

**Origin Adapter.** A host-specific integration component that **observes a source of truth** and **mints Arrow events**. This is the input-side first stage of the pipeline. An Origin Adapter for a blockchain chain reads chain state (via node, light-client proof, or API); an Origin Adapter for a legal system reads structured legal attestations. The Origin Adapter is responsible for correctly setting `verification_regime` and `Confidence` to reflect the actual observation method — not the theoretical best case for the named origin. The normative Origin Adapter contract is deferred to RFC-005.

**Consumption Adapter.** A host-specific integration component that consumes Arrow disposition output and translates it into host-system actions. RFC-003 v0.2 called these "adapters"; the term is now qualified to distinguish them from Origin Adapters.

**Source of Truth.** The canonical, external authority whose state an event describes: a blockchain's consensus output, a custodian's authoritative ledger, a court's issued order, a regulatory decision, an oracle's signed observation. The Source of Truth is outside Arrow. Arrow events represent it; they do not replace it.

---

## 4. The Corrected Pipeline

### 4.1 The full pipeline

```
Source of Truth           → chain consensus, custodian ledger, legal authority, oracle
        ↓
Origin Adapter            → observes the Source; mints the Arrow event;
                            sets verification_regime and Confidence to reflect
                            actual observation method
        ↓
Transport Envelope        → carries the event between parties
        ↓
Arrow Core + Validation   → admits the event: well-formedness gate only, not a truth gate
        ↓
Reduction Function R      → reduces the validated event log → disposition + canonical Policy
        ↓
Consumption Adapter       → translates disposition + Policy into host-system action
        ↓
Settlement Backend        → moves the asset
```

### 4.2 The critical distinction: validation is not a truth gate

This point is load-bearing and must not be blurred. Arrow Validation Rules v1.5-draft determine whether an event is **well-formed and internally consistent**. They check structural validity, registry membership, Confidence ceiling against the declared regime, State/Finality consistency, finality-basis completeness, LEG-specific constraints, and bridge trust flags. They do **not** independently verify that the asserted event actually occurred in the named source system.

An event asserting `1:ETH:FN:PR:1:A` — Ethereum, final, native canonical proof, Confidence A — passes every rule in v1.5-draft whether or not any such Ethereum transaction exists, as long as the message is structurally valid. Validation confirms the claim is admissible. The Origin Adapter, operating before validation, is responsible for ensuring the claim is true.

This is not a defect in the validation rules. Validation rules are a *correctness* gate; they run fast, stateless, and without chain access. The truth gate is the Origin Adapter — it is slow, stateful, and chain-dependent by design. Separating the two is the right architecture.

### 4.3 What the pipeline does not do

The pipeline does not: move assets; create legal effect by itself; guarantee that a Consumption Adapter behaves correctly; or substitute for external legal recognition. Each stage does exactly its job and no more.

---

## 5. The Origin Adapter

### 5.1 Role and boundaries

An Origin Adapter is the component that bridges a Source of Truth and the Arrow event log. It is not Arrow Core. Arrow Core defines the grammar, schema, and validation rules. The Origin Adapter observes reality and translates it into Arrow's grammar.

Every real Arrow deployment necessarily includes at least one Origin Adapter. Arrow Core itself never runs a node, queries a chain, or contacts an external system. Arrow as a *protocol* is infrastructure-free; Arrow as a *deployment* requires adapters at both ends of the pipeline.

This is a deliberate and principled separation. Arrow's chain-agnostic guarantee — that the same canonical message format works for Ethereum, Solana, Bitcoin, Cosmos chains, custodians, and legal systems — depends on Arrow Core never coupling itself to any one of them. Origin Adapters provide the coupling; Arrow Core provides the grammar that makes all couplings interoperable.

### 5.2 What an Origin Adapter must do (architectural, pre-normative)

At minimum, an Origin Adapter:

1. **Observes** the Source of Truth by the best method available to the deployment.
2. **Mints** an Arrow event whose fields correctly represent the observed state.
3. **Sets `verification_regime`** to reflect the actual observation method — not the best-case regime for the Origin's class. If the observation was via a single RPC API call, the regime is `institutional_attestation`, regardless of whether the chain named would support `native_canonical` under ideal conditions.
4. **Sets `Confidence`** within the ceiling that regime permits. Confidence A is not available unless the observation method can actually support it.
5. **Attaches a `proof_ref`** adequate for the declared regime: a block hash and Merkle proof for `native_canonical`; a signed API response for `institutional_attestation`; a structured legal attestation record for `legal_attestation`; a quorum attestation for `bonded_quorum`.

The normative Origin Adapter contract — the full interface definition, proof artifact requirements, identity/accountability rules, and the derive-don't-assert enforcement for chain-derivable fields — is specified in RFC-005.

### 5.3 Origin Adapters and the Chain Registry

Many fields an Origin Adapter would otherwise need to discover — challenge-window duration, typical reorg depth, finality model, chain trust flag — are derivable from the Chain Registry. An Origin Adapter for Base (an Ethereum Layer 2 (L2) optimistic rollup) does not ask the submitter whether Base has a challenge window. It reads the Chain Registry entry for Base, finds `finality_model = optimistic_rollup`, `challenge_window_duration = 604800` (7 days in seconds), and derives `Finality = 0` and `finality_basis = optimistic_window` automatically. The submitter is never burdened with — and never able to misrepresent — protocol-level finality facts about the chain they are transacting on.

The Chain Registry's own trustworthiness is a downstream dependency. RFC-006 addresses the Chain Registry as a first-class, governed, sourced artifact. Until RFC-006 is normative, the Chain Registry is treated as authoritative but manually maintained — a known limitation acknowledged here.

### 5.4 The ingestion gap in v0.2

RFC-003 v0.2 filed Compatibility Level 0 "Observed" — an Arrow indexer reads Ethereum calldata — as a passive compatibility classification. It did not recognize this as the active first stage of the production pipeline. This was an error of framing, not substance: the Level-0 indexer is precisely the component that must produce the minted events that `R` later reduces. The architecture was always correct; the framing was not. v0.3 promotes the ingestion stage to a named architectural component — the Origin Adapter — and places it first in the pipeline.

---

## 6. Observation Method Bounds Confidence

### 6.1 The over-claim problem

The current Arrow Origin Registry hard-codes `ETH → native_canonical`. This is an over-claim. It names the best-case observation method for Ethereum events — a full validating node with Merkle-proof verification — as if it were the method used for every Ethereum event. In practice, the vast majority of Ethereum observations in deployed systems are made through third-party RPC providers. An event observed by calling `eth_getTransactionReceipt` on a single Infura endpoint is not `native_canonical`. It is `institutional_attestation`: one provider's word.

The over-claim matters because Confidence A is staked on it. A system consuming `1:ETH:FN:PR:1:A` expects chain-native, cryptographically verifiable proof. If the claim was minted from a single API call, the consumer is trusting one company, not the chain. The Confidence field is the main instrument by which Arrow communicates trust quality; allowing it to misrepresent the observation method destroys the instrument.

### 6.2 The three honest observation tiers

| Observation method | What is trusted | Regime | Confidence ceiling |
|---|---|---|---|
| **Self-verifying proof** — Merkle inclusion against a finalized block header, ZK state proof, light-client proof, on-chain log re-read by the consumer | The cryptographic proof itself; the observer is untrusted and irrelevant | `native_canonical` | A |
| **Full validating node, accountable** — the Origin Adapter runs a validating node; the adapter's identity is registered and its attestation is signed | The adapter's integrity and its node's honest operation | `institutional_attestation` (with signing) | B |
| **Third-party RPC / API** — observation via one or more API providers | The providers' honesty and availability; single point of failure | `institutional_attestation` | C |

Confidence A is therefore not a property of the chain (Ethereum is capable of A-grade proofs), but a property of the **observation method** used for a specific event. An ETH event minted by a single-API Origin Adapter is Confidence C, regardless of Ethereum's theoretical capabilities.

### 6.3 The correction

The Origin Registry must be amended — in a normative RFC — to remove the hard-coding of `native_canonical` as the default regime for chain-type origins. The regime must be set by the Origin Adapter, not inherited from the Origin class. The registry entry for `ETH` should state the *maximum available* regime and ceiling, not the assumed regime. RFC-005 carries this correction as a normative rule.

---

## 7. The Reduction Model

### 7.1 Arrow is append-only; disposition is recomputed, never mutated

Arrow events are append-only (design principle 1; rule LFC-005). A lifecycle is an immutable, ordered log. There is no mutable "current state." The current disposition is always the result of reducing the log to the present:

```
disposition_now = R(event[0], event[1], ..., event[n])
```

When a new event is appended, the disposition is re-derived from the extended log. This is consistent with the protocol's deterministic-reconstruction principle (design principle 3).

### 7.2 What the reduction function consumes

`R` operates over a validated event log — events that have passed Arrow Validation Rules v1.5-draft **and were minted by a conforming Origin Adapter**. The reduction function reads, per event:

- the canonical message fields: `Class`, `Origin`, `State`, `Policy`, `Finality`, `Confidence`;
- the verification regime and resulting Confidence ceiling;
- lifecycle linkage: `lifecycle_id`, `related_events`;
- proof and reason references: `proof_type`, `proof_ref`, `reason_code`;
- where present: Bridge Registry and Chain Registry trust state.

### 7.3 What the reduction function produces

`R` produces a **disposition** and a **canonical Policy**. Policy is one of the six existing registry values (`PR, BL, HL, RW, TR, CL`) — the minimal, machine-actionable output. The disposition is a derived human-legible label summarizing lifecycle standing. Consumption Adapters consume both.

### 7.4 The reduction rules are not yet specified

RFC-003 names the reduction layer but does not specify its rules. The precedence logic — how an active `BL` dominates a prior `PR`, how `HL` interacts with a later `PR`, how `RV` re-scores a lifecycle — is the normative content a future RFC must define. Until then, `R` is a named layer with a defined shape and an unspecified body.

---

## 8. Compatibility Levels

A host system may integrate Arrow at one of five depths.

**Level 0 — Observed (Ingestion Stage).** An Origin Adapter observes the host system; the host does not know Arrow exists. **This is now recognized as the first production pipeline stage, not a passive observation mode.** Example: an Arrow indexer reads Ethereum calldata, logs, and traces and mints Arrow events. *Buildable today.*

**Level 1 — Referenced.** The host system carries Arrow identifiers in metadata — an event hash, a `lifecycle_id`. Arrow is referenced but not verified. Example: a bridge transaction includes an Arrow event identifier. *Buildable today.*

**Level 2 — Verified.** The host system cryptographically verifies Arrow output before relying on it. **Not buildable today.** Arrow Core defines no canonical event-signature or proof-of-state scheme. The Transport Model v1.0 defers signatures to channel bindings; the only shipped binding (HTTP REST pilot) is unsigned by design. Level 2 is blocked on the verification primitive (Q5). Category *(c) — theoretically sound, not yet built.*

**Level 3 — Enforced.** The host system changes behavior based on Arrow disposition — for example, a bridge adapter holds settlement when disposition is `QUARANTINED`. This is where Arrow becomes operationally meaningful. Depends on Level 2.

**Level 4 — Native.** Arrow rules are part of the host's own state machine. Expressly outside current implementation scope.

The honest near-term target is **Levels 0, 1, and 3**, with Level 3 dependent on first delivering the Level 2 primitive.

---

## 9. Native vs Interoperable Execution

**Native execution** means Arrow rules are part of a host system's own state-transition logic. Native execution is not required and is deferred (§12).

**Interoperable execution** means an external system interacts with Arrow through contracts, programs, adapters, Application Programming Interfaces (APIs), proofs, signatures, registries, or relayers.

Arrow **cannot be inserted into the Ethereum Virtual Machine (EVM) or Solana Virtual Machine (SVM) as a native state machine by declaration.** Those environments are already state machines with their own execution rules, storage models, fee systems, and finality assumptions. Arrow interacts with them through adapters; it does not become them.

---

## 10. Integration Surfaces

Arrow may be integrated at the following surfaces. None require native Arrow infrastructure.

**Smart contract (EVM).** Contract components — a registry, a policy verifier, a settlement adapter, a guarded wrapper — check Arrow disposition before allowing transfer, redemption, settlement, or execution.

**Program (SVM).** A verifier program and associated accounts check an Arrow instruction before releasing escrowed assets.

**Bridge adapter.** The highest-priority surface. A bridge consults Arrow before minting, releasing, burning, locking, or unlocking — evaluating source-event validity, finality, taint, and disposition. This surface connects directly to the failure class documented in RFC-002. Both an Origin Adapter (observing the source chain) and a Consumption Adapter (enforcing disposition on the destination) are required.

**Solver / intent.** Solvers execute a cross-chain route only if Arrow disposition permits. Arrow supplies programmable validity, not liquidity.

**Off-chain and institutional.** Arrow integrates into custodian workflows, bank settlement processes, legal escrow, Virtual Asset Service Provider (VASP) compliance pipelines, and exchange risk engines as a machine-readable legal/technical disposition layer.

---

## 11. Relationship to Existing Arrow Layers

RFC-003 introduces a layer above and below Arrow Core. It does not overlap with the layers Arrow already has.

**Arrow Core (Protocol v1.5-draft).** Defines the canonical string, the Event Object, and the verification-regime model. The reduction function `R` consumes Core output. Origin Adapters mint Core-conforming events. Neither alters Core.

**Validation Rules v1.5-draft.** Determine whether an individual event is well-formed and admissible. `R` operates only on events that have already passed validation. Validation is the well-formedness gate; the Origin Adapter is the truth gate; reduction is what happens after both.

**Transport Model v1.0.** Governs how an event is carried between parties. RFC-003 governs what a system does with an event after it arrives and validates. Transport is delivery; reduction and adapters are interpretation and enforcement.

**Bridge Registry (RFC-002).** Supplies bridge-level trust state. Where a lifecycle's events reference a bridge, `R` reads that trust state as an input.

```
Source of Truth           → external ground truth
Origin Adapter            → mints Arrow events        ← RFC-003
Transport Envelope        → carries the event
Arrow Core + Validation   → admits the event
Reduction Function R      → derives disposition        ← RFC-003
Consumption Adapter       → translates to host action  ← RFC-003
Settlement Backend        → moves the asset
```

---

## 12. Consumption Adapter Model and Policy

A Consumption Adapter consumes two things from `R`: a canonical **Policy** value and a **disposition** label.

**Policy is canonical.** `PR, BL, HL, RW, TR, CL` are the minimal, machine-actionable output. RFC-003 does not extend the Policy registry.

**Adapter actions are not canonical.** Host systems need richer local action sets — `ALLOW`, `WARN`, `DELAY`, `FREEZE`, `ESCALATE`, `REQUIRE_HUMAN_REVIEW`. These are adapter-local translations of `(Policy, disposition, host-local rules)`. The mapping is the adapter's responsibility.

| Canonical Policy | Typical adapter translation |
|---|---|
| `PR` | proceed / allow |
| `TR` | trigger a defined downstream action |
| `HL` | hold / delay / freeze pending resolution |
| `RW` | route to human or elevated review |
| `BL` | hard block |
| `CL` | lifecycle closed — no further action |

A normative Consumption Adapter interface is deferred to a future RFC.

---

## 13. Reserved Native Infrastructure

RFC-003 reserves but does not implement future Arrow-native infrastructure — an appchain, a rollup, a canonical registry chain, a settlement chain, a validator network.

Such infrastructure should be considered only after Arrow has: a working non-native MVP; at least one real adapter integration; demonstrated useful dispositions across at least two systems; external systems willing to consume Arrow output; and credible security, governance, and operational-funding models.

Native infrastructure does not remove the adapter problem. Even with a native chain, Arrow must integrate with Ethereum, Solana, Bitcoin, Cosmos chains, bridges, custodians, exchanges, wallets, indexers, and legal systems through adapters. A native chain changes only where canonical Arrow state lives.

---

## 14. Constraints and Risks

**No premature chain or token assumption.** Arrow specifications must not assume an Arrow chain or require an Arrow token.

**No universal enforcement claim.** Arrow cannot force a non-integrating system to obey its disposition. Arrow output is consequential only where a system has chosen to consume it.

**No false native compatibility.** Arrow must not claim native EVM, SVM, or Cosmos compatibility unless Arrow logic is implemented in that environment.

**Origin Adapter risk.** The Origin Adapter is the trust entry point and the most consequential attack surface. An Origin Adapter that missets `verification_regime`, inflates `Confidence`, or mints fabricated events undermines the entire chain. This is why RFC-005's normative contract — including identity/accountability requirements for Origin Adapters — is critical.

**Consumption Adapter risk.** A correct Arrow event and correct disposition can still produce unsafe behavior if the Consumption Adapter misreads the disposition, uses stale data, fails to verify, or carries an upgrade vulnerability.

**False positives and false negatives.** A disposition used to block, freeze, or quarantine assets carries legal, economic, and reputational cost if wrong in either direction.

**Missing Level 2 primitive.** Verifiable consumption depends on a signature/proof scheme Arrow does not yet have. Tracked as Q5. Note: this primitive is required at both ends — the Origin Adapter must sign what it mints; the Consumption Adapter must verify what it consumes.

---

## 15. Legal and Institutional Considerations

Where an external system relies on an Arrow disposition, that disposition can have legal consequences — settlement approval, asset quarantine, redemption block, escrow release, default recognition.

Arrow must keep **technical disposition** and **legal effect** strictly distinct. A technical transition does not, by itself, create legal effect. Legal effect additionally requires defined parties, governing law, competent authority, an evidence standard, an enforcement mechanism, a dispute process, a remedy, and off-chain recognition. This is consistent with Arrow's treatment of legal systems as first-class asserting domains — Origin `LEG` under the `legal_attestation` regime — rather than as an automatic consequence of on-chain events.

---

## 16. Recommended MVP

The first MVP must demonstrate both halves of the pipeline, not only one.

**Back half (what v0.2 described):** prove that a Consumption Adapter can observe Arrow disposition and change behavior accordingly. An EVM smart-contract adapter is the recommended first target, connecting directly to the RFC-002 failure class.

**Front half (new in v0.3):** prove that a real Origin Adapter can observe a real chain, correctly determine `verification_regime` and `Confidence` from the observation method, derive `finality_basis` from the Chain Registry, and mint a correctly graded event. A reduction engine fed hand-fixtured events proves computation; a real Origin Adapter proves trust. The front half comes first, because trust originates there.

Minimum components: Arrow event schema (exists); `arrow-core` Rust crate (exists); HTTP REST transport pilot (exists); **Origin Adapter** observing Ethereum via a defined observation tier (new); reduction engine implementing `R` (new); one EVM Consumption Adapter (new).

The MVP succeeds when: (a) a real Ethereum event observed by the Origin Adapter produces a correctly graded Arrow event; and (b) an external EVM contract demonstrably changes behavior — proceeds, holds, or blocks — based on the resulting disposition.

---

## 17. Future Work

**Multi-source Confidence upgrade (provisional, post-v1.5).** When an Origin Adapter relies on a single source — almost always one API — the honest Confidence ceiling is C. Arrow could optionally run parallel observation infrastructure — independent RPC providers, a self-hosted node, a light-client proof — and upgrade Confidence on agreement, flag divergence on disagreement. This reuses the existing multi-source divergence machinery (DIV-001 through DIV-004), which was built for cross-chain divergence and applies cleanly to cross-source divergence of the same chain. It must remain strictly opt-in value-added infrastructure — never a dependency of Arrow Core.

**Robust Chain Registry (RFC-006).** The Chain Registry is currently a static JSON file with no per-entry provenance, no ratification process, and no tamper-evidence mechanism. Fields that will be *derived* from it — `finality_basis`, challenge-window duration, reorg depth, `chain_trust_flag` — make it load-bearing infrastructure. RFC-006 specifies per-entry `provenance` sourcing, a versioning and ratification process, on-chain anchoring of the registry hash, and an update cadence. It is downstream of RFC-005 (provenance principle).

---

## 18. Open Questions

1. **Reduction rules.** What is the normative precedence logic for `R`? How do `BL`, `HL`, `RW`, `RV`, and supersession interact when reducing a multi-event log?
2. **Disposition vocabulary.** Should Arrow standardize a minimal canonical disposition set, or leave dispositions deployment-defined?
3. **Consumption Adapter interface.** What is the minimum normative adapter interface — call shape, disposition contract, error semantics?
4. **Finality across chains.** How should `R` weigh heterogeneous finality? *Partially answered by the `finality_basis` field specified in RFC-004; technical-vs-legal finality distinction remains open.*
5. **Verification primitive.** What signature or proof-of-state scheme unblocks Compatibility Level 2? This is a **bi-directional** requirement: Origin Adapters must sign minted events; Consumption Adapters must verify consumed dispositions. Should it live in Arrow Core, the Transport Model, or a dedicated RFC?
6. **Advisory dispositions.** Should `R` emit soft advisory dispositions distinct from enforceable ones?
7. **First adapter.** Should the first MVP Consumption Adapter be a smart-contract adapter or a bridge adapter?
8. **Native-infrastructure trigger.** What concrete, measurable conditions would justify reopening the native-infrastructure question?
9. **Origin Adapter registry.** Should Arrow maintain a registry of conforming Origin Adapters, or leave adapter conformance entirely to the deployment layer? If a registry, what are the registration and revocation criteria?

---

## 19. Conclusion

Arrow becomes real by being **consumed** — and by being **truthfully ingested** in the first place. The path is: Origin Adapters that observe real sources and mint correctly graded events; a reduction layer that turns a validated append-only event log into a disposition; Consumption Adapters that carry that disposition into the systems that actually move assets; and a compatibility ladder that makes the depth of any given integration explicit and honest.

```
Source of Truth
        ↓
Origin Adapter            → truth enters here; Confidence must reflect this
        ↓
Transport + Validation    → well-formedness gate
        ↓
Reduction R               → disposition + canonical Policy
        ↓
Consumption Adapter       → host-system action
        ↓
Settlement Backend
```

The working principle:

```
Origin Adapter first — truth enters here.
Reduction layer second — meaning is derived here.
Consumption Adapters third — action is taken here.
Native infrastructure later — only if a measurable trigger justifies it.
```

---

## Document Information

| Field | Value |
|---|---|
| Document ID | ARROW-RFC-003 |
| Version | v1.0 |
| Status | Final — Non-Normative Architecture RFC |
| Genesis Anchor | RFC-000 — `2026-03-22T18:40:00Z` |
| Primary Specification | Arrow Protocol v1.5-draft |
| Supersedes | RFC-003 v0.2 |
| Informs | RFC-004; RFC-005; RFC-006 |
| Governance | Finalized by Editor for the v1.5 package |
