# Arrow Protocol — RFC-001: KelpDAO Recovery Reference Case

*Reference case demonstrating Arrow Protocol's expression of composed legal-protocol authorizations involving judicial order, decentralized governance, and on-chain execution.*

| Field | Value |
|---|---|
| Document ID | ARROW-RFC-001 |
| Status | Reference Case |
| Version | 1.0 |
| Date of Record | 2026-05-17 |
| Underlying Event Period | 2026-04-18 to 2026-05-09 |
| Depends On | Arrow Protocol v1.3, Validation Rules v1.2, Registries v1.3 |
| Primary Purpose | Demonstrate Arrow's multi-regime composition pattern in a real-world cross-domain authorization case |

---

## 1. Purpose

RFC-001 documents the KelpDAO recovery sequence — a $292 million exploit response involving a federal court order, a Decentralized Autonomous Organization (DAO) governance vote, and on-chain execution on Arbitrum — as a canonical Arrow Protocol reference case.

The case validates three v1.3 design choices:

1. **Legal systems as first-class asserting domains.** The court order is the *authorizing event* that makes the on-chain action lawful, not metadata about an on-chain action.
2. **Composition over hybrids.** Three different verification regimes (legal_attestation, bonded_quorum, optimistic_settled) chain together via `related_events` linkage without requiring a new hybrid regime.
3. **The `related_events` chain encodes operational conditionality.** An auditor reading the lifecycle reconstructs the full authorization stack from the event log alone.

This document does not provide legal advice on the underlying case. It uses publicly reported facts to construct an Arrow Protocol representation suitable as a reference implementation.

---

## 2. Factual Summary

### 2.1 The Exploit

On April 18, 2026, KelpDAO lost approximately $292 million in restaked Ether (rsETH) through an exploit on its LayerZero bridge. The attacker exploited a single-verifier configuration — a 1-of-1 Decentralized Verifier Network (DVN) — compromising Remote Procedure Call (RPC) nodes to feed false data and fraudulently unlock approximately 116,500 rsETH from Ethereum mainnet escrow. The exploit was attributed to North Korea's Lazarus Group.

The attacker moved a significant portion of the stolen rsETH to Aave as collateral for Wrapped Ether (WETH), creating approximately $190 million in bad debt for the Aave protocol.

### 2.2 The Recovery Response

Arbitrum's Security Council froze 30,766 ETH (approximately $71 million) tied to the exploit. The Arbitrum community then voted via Constitutional Arbitrum Improvement Proposal (AIP) — the DAO's binding on-chain governance mechanism — to transfer those assets to a recovery multisig coordinated with Aave, KelpDAO, EtherFi, and Certora.

### 2.3 The Legal Intervention

On May 1, 2026, plaintiffs holding decades-old judgments against North Korea filed a restraining notice in the Southern District of New York (SDNY), arguing that the frozen Ether constituted Democratic People's Republic of Korea (DPRK) property subject to seizure under their terrorism-related judgments.

On May 9, 2026, U.S. District Judge Margaret M. Garnett of the SDNY modified the restraining notice. The court order permitted the Arbitrum Security Council to transfer the ETH to an Aave Limited Liability Company (LLC)-controlled wallet, explicitly shielded voters and operators participating in the on-chain governance vote from personal legal exposure under the existing freeze, and required Aave LLC to abide by the restraining notice as if it had been issued directly to the company until further order.

### 2.4 The On-Chain Execution

On May 12, 2026, Aave launched an amended Constitutional AIP — the binding on-chain governance vote implementing Judge Garnett's order. The proposal authorized the transfer of 30,765 ETH from the Security Council-controlled address to a wallet controlled by Aave LLC, subject to the court's restrictions on subsequent use.

---

## 3. Arrow Protocol Representation

The recovery sequence is expressed as four canonical Arrow events linked through `related_events`. Each event uses a different verification regime appropriate to its asserting domain.

### 3.1 Event 1 — Court Order Authorization

```json
{
  "event_id": "ARR-20260509-RFC001-E1",
  "lifecycle_id": "LFC-KELP-RECOVERY-001",
  "related_events": [],
  "arrow_code": "2:LEG:FN:PR:1:C",
  "class": 2,
  "origin": "LEG",
  "state": "FN",
  "policy": "PR",
  "finality": 1,
  "confidence": "C",
  "verification_regime": "legal_attestation",
  "object_type": "document",
  "object_id": "court-order:SDNY:2026-05-09:garnett",
  "subject_type": "legal_entity",
  "subject_id": "arbitrum-security-council",
  "asset_code": "N/A",
  "amount": "0",
  "reason_code": "COURT_ORDER_AUTHORIZATION",
  "proof_type": "legal_attestation",
  "proof_ref": "court://sdny/case-id/order-2026-05-09",
  "status": "active",
  "timestamp": "2026-05-09T17:00:00Z",
  "jurisdiction": "US",
  "policy_scope": "asset_specific",
  "version": "1.3",
  "metadata": {
    "court": "United States District Court for the Southern District of New York",
    "judge": "Margaret M. Garnett",
    "order_date": "2026-05-09",
    "underlying_case": "DPRK terrorism judgment creditors v. Arbitrum DAO",
    "order_type": "modification_of_restraining_notice",
    "authorizes": "On-chain Arbitrum DAO vote and subsequent ETH transfer to Aave LLC wallet",
    "conditions": "Aave LLC to abide by restraining notice on receipt; participants in vote shielded from personal liability under prior freeze"
  }
}
```

**Why Confidence C, not B:** The order issues from a single federal judge. Under the `legal_attestation` regime, single-authority orders are bounded at Confidence C. A panel ruling (appellate decision, arbitration award) would qualify for Confidence B.

### 3.2 Event 2 — DAO Governance Vote

```json
{
  "event_id": "ARR-20260512-RFC001-E2",
  "lifecycle_id": "LFC-KELP-RECOVERY-001",
  "related_events": ["ARR-20260509-RFC001-E1"],
  "arrow_code": "2:PCY:FN:PR:1:B",
  "class": 2,
  "origin": "PCY",
  "state": "FN",
  "policy": "PR",
  "finality": 1,
  "confidence": "B",
  "verification_regime": "bonded_quorum",
  "object_type": "document",
  "object_id": "aip:arbitrum:amended-constitutional-kelp-recovery",
  "subject_type": "legal_entity",
  "subject_id": "arbitrum-dao",
  "asset_code": "N/A",
  "amount": "0",
  "reason_code": "DAO_VOTE_PASSED",
  "proof_type": "policy_engine_decision",
  "proof_ref": "arbitrum-dao://aip/amended-constitutional-kelp-recovery",
  "status": "active",
  "timestamp": "2026-05-12T19:00:00Z",
  "jurisdiction": null,
  "origin_system": "arbitrum-dao",
  "policy_scope": "asset_specific",
  "version": "1.3",
  "metadata": {
    "proposal_type": "Constitutional Arbitrum Improvement Proposal (AIP)",
    "implements": "Judge Garnett SDNY order 2026-05-09",
    "vote_mechanism": "on-chain token-weighted vote",
    "authorizes": "Transfer of 30,765 ETH from Security Council address to Aave LLC wallet",
    "recipient_wallet": "Aave LLC-controlled address",
    "voter_protection": "Court order explicitly shields voters from personal liability"
  }
}
```

**Why Origin PCY, not LEG:** This is a governance event, not a legal event. The DAO vote operates under bonded_quorum (token-weighted economic security), not legal_attestation. `origin_system: arbitrum-dao` identifies the specific DAO.

**Why Confidence B:** Bonded quorum maximum is B. Token-weighted vote with quorum and threshold parameters is the canonical bonded_quorum case.

**Why `related_events` references Event 1:** The vote is conditional on the court order. Without the order, the vote would be a contempt risk. The `related_events` linkage encodes that conditional dependency.

### 3.3 Event 3 — On-Chain Transfer Execution

```json
{
  "event_id": "ARR-20260512-RFC001-E3",
  "lifecycle_id": "LFC-KELP-RECOVERY-001",
  "related_events": ["ARR-20260509-RFC001-E1", "ARR-20260512-RFC001-E2"],
  "arrow_code": "5:ARB:FN:TR:1:A",
  "class": 5,
  "origin": "ARB",
  "state": "FN",
  "policy": "TR",
  "finality": 1,
  "confidence": "A",
  "verification_regime": "optimistic_settled",
  "object_type": "asset",
  "object_id": "arb-tx:transfer-30765-eth",
  "subject_type": "wallet",
  "subject_id": "aave-llc-controlled-wallet",
  "asset_code": "ETH",
  "amount": "30765.00000000",
  "reason_code": "RWA_TRANSFER_AUTH",
  "proof_type": "native_chain_proof",
  "proof_ref": "arb://tx/transfer-hash",
  "status": "active",
  "timestamp": "2026-05-12T20:00:00Z",
  "jurisdiction": null,
  "origin_system": "arbitrum-one",
  "destination_system": "arbitrum-one",
  "chain_id": "ARB-1",
  "chain_registry_version_at_event": 1,
  "policy_scope": "asset_specific",
  "version": "1.3",
  "metadata": {
    "source_wallet": "arbitrum-security-council-frozen-address",
    "destination_wallet": "aave-llc-controlled-wallet",
    "post_transfer_constraint": "Aave LLC barred from selling or moving funds until court approval"
  }
}
```

**Why Confidence A on optimistic_settled:** Achievable post-challenge-window. For pre-window state, Confidence would be B. The challenge window is 7 days on Arbitrum One.

**Why `related_events` references both Event 1 and Event 2:** The transfer is authorized by both the court order and the DAO vote. The on-chain execution depends on the combined authorization stack. An auditor reconstructing the event later sees both predicates without needing to know operational context.

### 3.4 Event 4 — Settlement

```json
{
  "event_id": "ARR-20260512-RFC001-E4",
  "lifecycle_id": "LFC-KELP-RECOVERY-001",
  "related_events": ["ARR-20260512-RFC001-E3"],
  "arrow_code": "6:ARB:FN:CL:1:A",
  "class": 6,
  "origin": "ARB",
  "state": "FN",
  "policy": "CL",
  "finality": 1,
  "confidence": "A",
  "verification_regime": "optimistic_settled",
  "object_type": "asset",
  "object_id": "arb-tx:transfer-30765-eth",
  "subject_type": "wallet",
  "subject_id": "aave-llc-controlled-wallet",
  "asset_code": "ETH",
  "amount": "30765.00000000",
  "reason_code": "SETTLEMENT_FINAL",
  "proof_type": "native_chain_proof",
  "proof_ref": "arb://tx/transfer-hash#settled",
  "status": "active",
  "timestamp": "2026-05-19T20:00:00Z",
  "jurisdiction": null,
  "origin_system": "arbitrum-one",
  "chain_id": "ARB-1",
  "chain_registry_version_at_event": 1,
  "policy_scope": "asset_specific",
  "version": "1.3",
  "metadata": {
    "challenge_window_elapsed": true,
    "l1_settlement_confirmed": true,
    "note": "Settlement timestamp reflects ETH transfer fully settled after Arbitrum 7-day challenge window. Aave LLC remains bound by court restrictions on subsequent use of the funds."
  }
}
```

**Why Policy CL:** Class 6 (Settle) MUST use Policy CL (SEM-005). The lifecycle terminates here.

---

## 4. Composition Pattern Analysis

### 4.1 What the Pattern Demonstrates

The four-event sequence demonstrates composed authorization across three verification regimes:

| Event | Origin | Regime | Confidence | Trust Basis |
|---|---|---|---|---|
| 1 — Court order | LEG | legal_attestation | C | Judicial authority (single judge) |
| 2 — DAO vote | PCY | bonded_quorum | B | Token-weighted economic security |
| 3 — Transfer | ARB | optimistic_settled | A | L1 settlement after challenge window |
| 4 — Settlement | ARB | optimistic_settled | A | L1 settlement after challenge window |

Each regime is appropriate to its asserting domain. None of the regimes is sufficient alone — the court order without the vote is unenforceable, the vote without the order risks contempt, the transfer without both is a violation of the restraining notice. Composition via `related_events` makes the dependency machine-readable.

### 4.2 Why No Hybrid Regime is Needed

A naive design might propose a single hybrid regime called something like `protocol_legal_hybrid` to express this case. v1.3 rejects that approach for three reasons:

**First, hybrids hide trust assumptions.** A hybrid regime would obscure that the court order alone is Confidence C, the DAO vote alone is Confidence B, and the on-chain execution alone is Confidence A. Each carries different verification infrastructure and different failure modes. Collapsing them into one regime label loses information an auditor needs.

**Second, hybrids do not scale.** If hybrid regimes were created for every composition pattern, the registry would grow combinatorially. Legal+DAO+chain is one pattern. Legal+custodian+chain is another. Multi-court+arbitration+chain is a third. The composition primitive (`related_events`) handles all of these uniformly.

**Third, hybrids weaken validation.** Under composition, each event passes SEM-015 independently against its own regime ceiling. Under a hybrid regime, validation must define the regime ceiling for the combination — which is not well-defined and changes case by case.

### 4.3 What an Auditor Reconstructs from the Log Alone

Given only the four event objects above, an auditor can reconstruct:

- The chronological sequence of authorizations
- The verification infrastructure underlying each authorization
- The conditional dependencies between events (via `related_events`)
- The jurisdiction of the legal authority
- The governance mechanism that implemented the legal authorization
- The chain on which final settlement occurred
- The post-settlement constraints (via metadata)
- The Confidence claim of each step, with regime-bounded validation

The reconstruction is deterministic. No additional context is required beyond the event log and the protocol registries.

---

## 5. Validation Rule Application

Each event in the sequence passes the v1.3 validation order (Part 4.4 of the main specification):

| Step | Event 1 | Event 2 | Event 3 | Event 4 |
|---|---|---|---|---|
| 1. MSG parsing | Pass | Pass | Pass | Pass |
| 2. Origin resolution | Pass (LEG) | Pass (PCY) | Pass (ARB → ARB-1) | Pass (ARB → ARB-1) |
| 3. chain_trust_flag | N/A | N/A | Pass (flag=1) | Pass (flag=1) |
| 4. verification_regime declared | Pass | Pass | Pass | Pass |
| 5. Semantic constraints | Pass | Pass | Pass | Pass |
| 6. SEM-015 regime ceiling | Pass (C ≤ C ceiling) | Pass (B ≤ B ceiling) | Pass (A ≤ A post-window) | Pass (A ≤ A post-window) |
| 7. Divergence (multi-source) | N/A | N/A | N/A | N/A |
| 8. SEM-009 protocol floor | Pass | Pass | Pass | Pass |
| 9. Deployment policy | Per deployment | Per deployment | Per deployment | Per deployment |

All four events pass protocol-level validation. Deployment-level policy is layer-specific and out of scope for this RFC.

---

## 6. Cross-Regime Audit Properties

The composition exhibits four properties relevant for diligence and audit:

**Provenance traceability.** Each event has a verifiable `proof_ref` pointing to its evidentiary artifact: a court order document, a DAO vote tally, an on-chain transaction hash. The artifacts are independently verifiable on their native infrastructures.

**Jurisdictional clarity.** Event 1 declares `jurisdiction: US`. Events 2–4 have `jurisdiction: null` (operational events, no jurisdictional binding). An auditor immediately sees where legal authority anchors.

**Tamper detection.** Each event carries `event_hash` (omitted for readability above) over its full content. Modification of any event invalidates the hash chain.

**Replay protection.** Each `event_id` is globally unique (LFC-006). Resubmitting any event is rejected as a replay attempt.

---

## 7. What This Case Does Not Demonstrate

For clarity on the scope of this reference case:

- The case does not demonstrate divergence detection (Part 8 of main spec). No multi-source observation was active on Arbitrum during this sequence.
- The case does not demonstrate `chain_trust_flag` revocation (SEM-016). Arbitrum One remained `chain_trust_flag: 1` throughout. The underlying LayerZero bridge that enabled the original exploit is a separate system and would be a candidate for trust flag revocation in a more extensive analysis.
- The case does not demonstrate reversal (SEM-014). All events were forward authorizations; no prior final event was contradicted.
- The case does not demonstrate Confidence B from a legal panel. Judge Garnett's order is single-authority, Confidence C.

These are valid Arrow Protocol patterns documented elsewhere in the v1.3 specification.

---

## 8. Open Questions for Further Refinement

The case surfaces three design questions for future iterations:

**Q1 — Is `chain_trust_flag` the right primitive for bridge-level revocation?**
The KelpDAO bridge was the operational failure point. In a more granular registry, bridges might have their own trust flags independent of the chains they connect. v1.3 does not address bridge-level revocation. v1.4 may need a Bridge Registry.

**Q2 — How should subsequent court restrictions be expressed?**
The Garnett order imposes ongoing restrictions on Aave LLC's use of the funds. Each future operational decision by Aave LLC will reference back to Event 1. Arrow's `related_events` supports this, but the protocol does not currently provide a mechanism for an event to declare "I am bound by ongoing constraints from event X." A `constrained_by` field is a candidate for v1.4.

**Q3 — Should DAO votes that implement court orders carry a distinct reason code?**
v1.3 introduces `COURT_ORDER_AUTHORIZATION` and `DAO_VOTE_PASSED`. The case shows these compose naturally via `related_events`. A more explicit `COURT_ORDER_IMPLEMENTATION` code on the DAO vote event might improve discoverability for compliance-focused queries.

---

## 9. Conclusion

RFC-001 demonstrates that Arrow Protocol v1.3 expresses one of the most operationally complex cross-domain authorization cases of 2026 — involving judicial authority, decentralized governance, and on-chain execution — using only the primitives already present in the protocol: Origin diversity, verification regimes, `related_events` composition, and lifecycle tracking.

No hybrid regime is needed. No new event types are needed. The composition is expressed entirely through the existing canonical structure.

The case strengthens the v1.3 design thesis: **legal systems are first-class asserting domains, verification regimes are explicit, and composition replaces hybridization.**

---

## Document Information

| Field | Value |
|---|---|
| Document ID | ARROW-RFC-001 |
| Status | Reference Case |
| Version | 1.0 |
| Genesis Anchor | RFC-000 — `2026-03-22T18:40:00Z` |
| Underlying Period | 2026-04-18 to 2026-05-09 |
| Primary Specification | Arrow Protocol v1.3 |
| Source Material | Public reporting on the KelpDAO exploit, Arbitrum DAO governance records, and SDNY court filings |
