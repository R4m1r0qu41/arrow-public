# ADR-ARRW-001 — Protocol Authority, Baseline Liveness, and ADR Governance

**Status:** live
**Date:** 2026-06-19
**Scope:** Stack-level — normative for Arrow Protocol, Tarquin, and Orlog.
**Supersedes (governance scope only):** [Tarquin ADR-0002](../../../../Tarquin/decisions/ADR-0002-v1.6-reboot-and-liveness-v1.md) — the *stack-governing* portions. Tarquin ADR-0002 is preserved unchanged as historical decision record (see §4).
**Companions:** Tarquin [ADR-0001](../../../../Tarquin/decisions/ADR-0001-stack-boundaries-v2.md) (stack boundaries), [ADR-0003](../../../../Tarquin/decisions/ADR-0003-interface-pinning-v1.md) (interface pinning).
**Ratified by:** Soft-reboot human-ratification decisions 1 and 2 (`reviews/soft-reboot-2026-06-17/Unresolved Decisions Requiring Human Ratification — Answers.md`).

---

## 1. Context

The v1.6 soft reboot established a stack-wide liveness and archive discipline in **Tarquin ADR-0002**. That ADR declared *stack-level* scope over all three CipherLayer repositories (Arrow, Tarquin, and the foundry then named Contract Labs). It also fixed the homologation rule: all three repos mark live artifacts against the **Arrow** baseline (`arrow-v1.6.0`), not a private version line.

This produced a governance contradiction, surfaced in `reviews/soft-reboot-2026-06-17/ADR-002-REVIEW.md`:

- The soft-reboot instruction holds that **Arrow owns** the governing version-control and protocol-authority decision, and that the stack must not carry three competing copies of one governance ADR.
- Yet the *governing* ADR physically lived in **Tarquin**, a downstream consumer of Arrow's protocol surface.

A downstream leg cannot be the normative owner of the policy that the upstream protocol authority is bound by. The homologation rule (BASE-001) already says version authority flows from Arrow; the ADR location did not. This ADR repairs that, **without rewriting Tarquin history** (decision 1).

## 2. Decision

### 2.1 Authority location (Amendment A1)

Arrow is the normative protocol authority for Arrow protocol semantics, release identifiers, baseline manifests, and live/superseded protocol-document classification. Tarquin and Orlog may maintain local adoption ADRs and independent software or product versions, but they **do not independently version Arrow protocol semantics**.

The prior Tarquin ADR-0002 remains preserved as the historical decision record for the v1.6 soft-reboot discovery. Its stack-governing policy is **superseded by this Arrow-owned ADR as of this ADR's date**. Tarquin and Orlog adoption ADRs **reference** this ADR rather than copy it.

### 2.2 Governing rules (Arrow-owned; carried forward from ADR-0002 §2)

These rules were correct in substance in ADR-0002; this ADR re-homes their **ownership** to Arrow so there is a single authoritative source. Rule IDs are preserved for continuity — a citation of `LIVE-001` resolves here, not to the superseded Tarquin copy.

| Rule | Statement |
|---|---|
| **AUTH-001** | Arrow alone assigns the protocol baseline identifier (`arrow-vMAJOR.MINOR.PATCH`) and classifies protocol documents as live or superseded. No downstream repo mints, forks, or reinterprets an Arrow protocol version. |
| **LIVE-001** | Every normative artifact (spec, ADR, registry, generated export, runtime contract) carries a `Status:` line whose value is exactly **`live`** or **`superseded`**. The marker carries **no version baseline** (`live`, not `live @ arrow-v1.6.0`) — the version, where one legitimately exists, lives in the document title/filename and in Git history, not duplicated into the liveness marker. An artifact with no `Status:` is treated as `live` and flagged by the next audit until marked. |
| **FLAT-001** | A live spec is **replaced, never amended by a second document.** To change a live spec, edit it in place; Git history is the version record. Publishing an "amendment"/"addendum" that a reader must compose with the original is prohibited — the documented root cause of the v1.5→v1.6 drift. |
| **ARC-001** | When a document becomes `superseded` it moves to the repo's **single top-level `archive/`** in one operation: repoint live references to the replacement, then relocate. `archive/` is organized by theme (or by version where cleanly stated) and carries a `README.md` curation index. **AI agents ignore `archive/` entirely** — it is never read, cited, edited, reused, or scanned by coherence/drift audits. Archived files are never edited or migrated; their terminology and versions are correct *as of their era*. |
| **ANCH-001** | Cryptographically anchored artifacts that remain canonical (e.g. `rfc/RFC-000`, minted on Bitcoin/Ethereum) stay **`live`** in the working tree. They are never edited but are **not** archived — an anchor is current law, not history. |
| **BASE-001** | The Arrow protocol baseline identifier (`arrow-v1.6.0`) is a legitimate protocol release tag, set by Arrow. Cross-repo prose cites it where a concrete protocol version is meant; it is **not** embedded into liveness markers (see LIVE-001). |
| **DOCVER-001** | Active documents use **stable filenames** and evolve through Git history; rolling document-version suffixes (`*-v0.1.md`) are retired from active material across all three repos. This governs *documents* only — software package releases, protocol release tags, schema-compatibility identifiers, and deployment versions remain legitimately versioned (decision 4). Archived snapshots retain era version markers as historical fact (ARC-001). |

### 2.3 Status vocabulary

Two states only. Liveness is binary; physical location follows from it.

| State | Meaning | Location | Editable? |
|---|---|---|---|
| `live` | Current and authoritative (includes anchored canonical artifacts per ANCH-001) | working tree | Yes — edit in place; Git history is the version record |
| `superseded` | Replaced by a live document | moved to `archive/` (agent-ignored) | No — never edited or migrated |

### 2.4 ADR identification and ownership (decision 2)

| Rule | Statement |
|---|---|
| **ADR-001** | ADRs use **repository-specific identifiers**, mirroring the RFC naming model: Arrow `ADR-ARRW-NNN`, Tarquin `ADR-TARQ-NNN`, Orlog `ADR-ORLG-NNN`. Each repository maintains its own ADR directory and index. |
| **ADR-002** | **Cross-repository governance ADRs are owned by Arrow.** Downstream repos express their adoption through repository-local adoption ADRs that **reference** the Arrow ADR by identifier and state repository-local application only — they do not restate or fork the governing decision. |
| **ADR-003** | Pre-existing ADRs minted under the prior `ADR-NNNN` scheme (Tarquin `ADR-0001`/`ADR-0002`/`ADR-0003`) are **preserved as historical records under their original identifiers**. They are not renumbered into the new scheme; renumbering would falsify decision history. New ADRs use the repository-specific scheme. |

## 3. Supersession map — what this ADR takes over from Tarquin ADR-0002

| Tarquin ADR-0002 section | Disposition under ADR-ARRW-001 |
|---|---|
| §2.1 Liveness marker (as **stack policy**) | **Superseded** — re-homed here as LIVE-001 + §2.3 vocabulary, Arrow-owned. |
| §2.2 Numbered rules (LIVE/FLAT/ARC/FRZ/BASE-001) | **Superseded** — carried forward verbatim in substance as §2.2, Arrow-owned; rule IDs preserved. |
| §2.3 Enforcement via the Dynamic Workflow suite | **Superseded** — enforcement is now keyed to this ADR; the W3/W6/W7 wiring is unchanged. |
| §1 Context, §3 Archive Manifest, §4 Rollout, §5 Consequences | **Preserved as history** — these record the v1.6 discovery and the one-time archive manifest. They remain readable in Tarquin ADR-0002 and are **not** re-owned here. |

Nothing in Tarquin ADR-0002 is edited, moved, or deleted. Its `STATUS:` is transitioned to `superseded` by Tarquin's own adoption ADR (`ADR-TARQ-001`), with `superseded-by:` pointing at this document — the supersession is recorded by the owning repo, not imposed by rewriting the file from outside.

## 4. Preserving Tarquin ADR-0002 history

Per decision 1 and ADR-003, Tarquin ADR-0002 is **not** silently rewritten or deleted. The mechanics:

1. This ADR (Arrow-owned) becomes the live governing source on its date.
2. Tarquin authors `ADR-TARQ-001` (adoption), which: (a) references this ADR as the governing authority, (b) records that ADR-0002's stack-governing scope is superseded here, and (c) sets ADR-0002's header to `superseded` with a `superseded-by:` pointer. The body of ADR-0002 is left intact.
3. Orlog authors `ADR-ORLG-001` (adoption) on the same pattern.

This keeps the *decision history* (why the discipline exists) legible where it was discovered, while the *normative authority* (who owns it now) sits in Arrow.

## 5. Consequences

- **Positive:** Version and governance authority physically coincides with the protocol authority. "Who owns this version line" is mechanical and matches BASE-001. There is exactly **one** governing copy of the liveness/archive discipline; downstream repos reference it and cannot drift it. The amend-on-amend ban that prompted the reboot is now owned at the layer that mints baselines.
- **Cost:** A one-time authoring of this ADR plus two short downstream adoption ADRs, and the status transition of Tarquin ADR-0002 to `superseded`. No protocol semantics, registries, fixtures, or code change as a result of this ADR.
- **Invariant repaired:** *A downstream consumer must not own the governance policy binding its upstream authority.* Inverting this (leaving the governing ADR in Tarquin) is exactly the contradiction the soft-reboot review flagged; it would let a legal-runtime leg dictate Arrow's version policy.

## 6. Enforcement

- LIVE-001 / DOCVER-001 are asserted by the registry/normative lints (`tools/validate-registries.js` and the per-repo validators), extended to flag any `superseded` artifact lacking a resolvable `superseded-by:`.
- W6 (coherence audit) scopes its reads to `live` artifacts via `Status:`; W1 (drift sweep) and W7 (MCD simplification) skip `archive/` paths entirely.
- This ADR's own `STATUS:` line is the first instance it governs.
