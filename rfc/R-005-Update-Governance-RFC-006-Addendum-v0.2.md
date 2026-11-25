# RFC-006 Addendum — R-005 Registry Update Governance

**Document ID:** RFC-006-ADD-R005
**Title:** R-005 Update Governance
**Status:** Ratified 2026-06-16 — normative; incorporated by reference into RFC-006 §5.5. Maintained independently and may be revised on its own cadence (RFC-006 pins the ratified v0.2).
**Version:** v0.2
**Date:** 2026-06-08
**Supersedes:** v0.1 (2026-06-08)
**Applies to:** Arrow Protocol registries and registry-derived validation logic
**Normative language:** MUST, MUST NOT, SHOULD, SHOULD NOT, and MAY are to be interpreted as binding requirements unless expressly stated otherwise.

**Changes in v0.2 (taxonomy binding).** The closed-option `registry` vocabulary, entry identifiers, field paths, protected-path globs, repository layout, and worked examples are bound to the **live Arrow registry set** and the **Chain Registry v2.0 schema** (`code` + `protocol_required.*`). The registry changelog is reconciled to `registries/index.json` (the existing source of truth); the release manifest is additive and references the `index.json` version. Change-class codes are disambiguated from the Arrow message Class field and RFC-007 event identifiers. No process semantics change.

---

## 1. Purpose

This addendum defines the operational process required by **R-005 — Defined update process**.

The purpose is to ensure that registry entries are not modified informally, manually, or through unstructured edits. Every update MUST follow a traceable Registry Change Proposal (RCP) process with:

1. a machine-readable intake record;
2. closed-option classification fields;
3. mandatory supporting evidence;
4. an automated Continuous Integration (CI) validation path;
5. a defined review and ratification process; and
6. a separate expedited track for emergency updates.

The process is designed to preserve auditability while keeping ordinary contributors away from direct JavaScript Object Notation (JSON) editing.

---

## 2. Design Principles

### 2.1 Structured intake before registry mutation

An RCP is the only permitted entry point for registry updates.

A proposer MUST submit a structured RCP intake form. The intake form MUST use closed options wherever reasonably possible. Free text MAY be used only for explanation, rationale, or evidence description. Free text MUST NOT directly alter a canonical registry field.

### 2.2 No direct JSON editing by ordinary proposers

Canonical registry JSON files are rail-guarded protocol data.

Ordinary RCP editors MUST NOT edit canonical JSON files directly. Registry JSON mutations MUST be produced by an approved generator, transformation script, or maintainer-controlled interface that converts validated RCP selections into a deterministic patch.

### 2.3 Evidence-first governance

A registry update MUST be supported by evidence.

A proposer MUST upload or reference at least one evidentiary artifact capable of independent review. A typed assertion without an attached or linked artifact is insufficient.

### 2.4 CI as a process gate

Every proposed registry mutation MUST have a clean machine-readable line into CI.

CI MUST validate both:

1. the structured RCP intake record; and
2. the generated registry patch.

CI MUST reject a pull request where the registry changes do not match the structured RCP selections.

### 2.5 Emergency does not mean undocumented

Emergency updates MAY use a shorter path, but MUST remain evidence-backed, attributable, and reviewable.

An emergency change MUST receive provisional confirmation, quasi-definitive confirmation, amendment, or reversion no later than **48 hours** after provisional activation.

---

## 3. R-005 Normative Rule

### R-005 — Defined Registry Update Process

Registry entries MUST be created, amended, activated, suspended, deprecated, removed, or otherwise modified only through a documented Registry Change Proposal (RCP).

Each RCP MUST:

1. use the prescribed machine-readable intake format;
2. select from the permitted closed-option fields;
3. identify the affected registry and entry;
4. identify the permitted update operation;
5. include at least one supporting evidentiary artifact;
6. generate a deterministic registry patch through an approved transformation process;
7. pass CI validation;
8. complete the applicable review period;
9. receive the required ratification; and
10. be recorded in the registry changelog and release manifest.

Ordinary proposers MUST NOT modify canonical registry JSON files directly.

Where a proposed change cannot be represented using the existing closed options, the proposer MUST NOT force the change into an adjacent category. The proposer MAY submit a **C4 — Exceptional / Taxonomy Expansion** proposal for review.

Emergency updates MUST follow the expedited **E1** track and MUST receive confirmatory action within 48 hours after provisional activation.

---

## 4. Registry Change Proposal Lifecycle

### 4.1 Ordinary lifecycle

```text
Draft Intake
    ↓
Structured Submission
    ↓
Evidence Check
    ↓
CI Intake Validation
    ↓
Deterministic Patch Generation
    ↓
CI Patch Validation
    ↓
Review Period
    ↓
Ratification
    ↓
Merge
    ↓
Effective Registry Release
    ↓
Archive
```

### 4.2 Emergency lifecycle

```text
Emergency Intake
    ↓
Evidence Check
    ↓
CI Intake Validation
    ↓
Deterministic Patch Generation
    ↓
CI Patch Validation
    ↓
Provisional Ratification
    ↓
Immediate Activation
    ↓
Mandatory Confirmatory Review ≤ 48 Hours
    ↓
Confirmed / Amended / Reverted / Escalated
```

---

## 5. RCP Classification

Each RCP MUST use exactly one change class.

> **Naming note.** These RCP change-class codes (`C1`–`C4`, `E1`) name *categories of registry change* and form a governance-layer namespace. They are distinct from — and MUST NOT be conflated with — the Arrow message **Class** field (Classes 1–5) or the reference-case **event identifiers** used in RFC-007 (`E0`–`E5`).

| Code | Classification | Description | Default review period | Ratification requirement |
|---|---|---|---:|---|
| `C1` | Editorial | Non-substantive correction that does not alter validation, classification, trust assumptions, or registry behavior. | 24 hours | One registry maintainer |
| `C2` | Ordinary substantive | Supported update represented by existing closed options that does not materially alter validation outcomes or trust assumptions. | 5 calendar days | Two reviewers, including one registry maintainer |
| `C3` | Material | Change that may affect validation outcomes, confidence ceilings, trust flags, activation status, legal classification, security assumptions, or dependent registry behavior. | 10 calendar days | One registry maintainer and one relevant domain reviewer |
| `C4` | Exceptional / Taxonomy Expansion | Change that cannot be safely represented using the current closed-option taxonomy or requires a new permitted option, field, enum value, operation, evidence class, or validation rule. | Minimum 10 calendar days | One registry maintainer, one relevant domain reviewer, and explicit schema or taxonomy approval |
| `E1` | Emergency | Expedited change required to prevent materially incorrect, unsafe, or misleading registry behavior where ordinary review delay would create unacceptable risk. | Immediate provisional activation; confirmatory action required within 48 hours | Two authorized emergency approvers |

### 5.1 Automatic escalation rules

An RCP MUST be escalated to `C3` where it modifies a field that affects:

- validation outcomes;
- trust flags;
- confidence levels or ceilings;
- chain or bridge activation status;
- legal classification;
- security assumptions;
- verification regimes;
- dependent fixtures; or
- downstream registry resolution.

An RCP MUST be escalated to `C4` where:

- the selected registry field does not exist;
- the requested value is not included in the approved closed options;
- the requested operation is not supported;
- the evidence type cannot be represented;
- the proposal requires a new schema field;
- the proposal changes the meaning of an existing enum value; or
- the proposer requests an interpretation that would expand the existing taxonomy.

A `C4` proposal is not a workaround. It is a controlled request to expand or revise the registry model.

---

## 6. Machine-Readable RCP Intake

### 6.1 Intake record

Each proposal MUST include a machine-readable intake file:

```text
registry-changes/RCP-XXXX/intake.yaml
```

YAML Ain't Markup Language (YAML) is used for proposer-facing intake because it is more legible than JSON. The canonical registry remains JSON.

The intake file MUST be validated against a closed schema before a patch can be generated.

### 6.2 CI control line

Each RCP MUST include a single machine-readable control line at the top of `intake.yaml`:

```yaml
rcp_control: "RCP|v1|RCP-0007|C3|chain_registry|ETH|UPDATE|SUBMITTED"
```

The pipe-delimited control line provides CI with a simple deterministic entry point.

The required order is:

```text
RCP | schema_version | rcp_id | change_class | registry | entry_id | operation | status
```

The permitted values are closed:

```text
schema_version:
  - v1

change_class:
  - C1
  - C2
  - C3
  - C4
  - E1

registry:
  - origins
  - classes
  - states
  - policies
  - confidence
  - finality_basis
  - verification_regimes
  - chain_registry
  - bridge_registry
  - channel_bindings
  - confidence_subprofiles
  - classification_binding_methods
  - sources
  - object_types
  - compliance_extensions
  - reason_codes
  - leg_supra_jurisdictions
  - proof_types
  - subject_types
  - legal_systems          # currently under registries/drafts/
  - other_registry_added_by_C4_ratification

operation:
  - CREATE
  - UPDATE
  - ACTIVATE
  - SUSPEND
  - DEPRECATE
  - REMOVE
  - RESTORE

status:
  - DRAFT
  - SUBMITTED
  - VALIDATED
  - UNDER_REVIEW
  - RATIFIED
  - PROVISIONAL
  - CONFIRMED
  - AMENDED
  - REVERTED
  - REJECTED
  - ARCHIVED
```

CI MUST reject:

- missing control lines;
- malformed control lines;
- unknown values;
- mismatches between the control line and the remaining intake file;
- unsupported registry names;
- unsupported operations;
- unsupported change classes; and
- any registry mutation without a corresponding validated RCP.

### 6.3 Closed-option intake structure

Example:

```yaml
rcp_control: "RCP|v1|RCP-0007|C3|chain_registry|ETH|UPDATE|SUBMITTED"

rcp:
  id: RCP-0007
  title: Deepen Ethereum reorg-safety confirmation threshold
  schema_version: v1
  change_class: C3
  registry: chain_registry
  entry_id: ETH
  operation: UPDATE
  status: SUBMITTED

proposer:
  id: github:username
  submitted_at: 2026-06-08T06:00:00Z

change_request:
  field: protocol_required.reorg_safety_confirmations
  current_value: 12
  requested_value: 64
  reason_code: PROTOCOL_PARAMETER_CHANGE
  rationale: >
    Updated finalized-epoch safety analysis in the official Ethereum
    consensus documentation supports a deeper reorg-safety threshold
    for high-Confidence derivation.

evidence:
  - evidence_id: EVD-001
    artifact_type: WEB_ARCHIVE
    authority_level: PRIMARY
    source_type: OFFICIAL_DOCUMENTATION
    file_path: evidence/EVD-001-eth-consensus-docs.warc
    sha256: "<required>"
    retrieved_at: 2026-06-08T05:45:00Z
    supports_fields:
      - protocol_required.reorg_safety_confirmations

review:
  review_period: C3_DEFAULT
  requested_domain_review: CHAIN_PROTOCOL
```

The `registry`, `entry_id`, and `field` values MUST resolve against the live registry and its schema; `reason_code`, `artifact_type`, `authority_level`, `source_type`, and `requested_domain_review` MUST be defined by schema enums. Enumerated registry fields MUST use a permitted enum value; typed fields (integers, booleans) MUST satisfy the registry schema's type and range constraints.

### 6.4 Free-text limitations

Free text MAY be included only in fields such as:

- `title`;
- `rationale`;
- `reviewer_notes`;
- `evidence_description`; and
- `emergency_rationale`.

Free text MUST NOT directly populate canonical registry fields.

---

## 7. JSON Isolation and Deterministic Patch Generation

### 7.1 Canonical registry isolation

Canonical registry JSON files MUST be protected.

Ordinary RCP editors MUST interact only with:

```text
registry-changes/RCP-XXXX/intake.yaml
registry-changes/RCP-XXXX/evidence/
registry-changes/RCP-XXXX/proposal.md
```

They MUST NOT directly modify:

```text
registries/**/*.json
registries/**/*.schema.json
```

### 7.2 Approved transformation layer

A maintainer-controlled generator MUST convert validated RCP selections into a deterministic patch.

Recommended command:

```bash
cargo run -p arrow-registry-governance -- generate-patch registry-changes/RCP-0007/intake.yaml
```

Alternative implementation for an early prototype:

```bash
python3 scripts/generate_registry_patch.py registry-changes/RCP-0007/intake.yaml
```

The generator MUST:

1. validate the intake schema;
2. parse the `rcp_control` line;
3. confirm that closed options are valid;
4. load the current canonical registry entry;
5. verify the declared `current_value`;
6. apply only the permitted operation;
7. generate a patch file;
8. generate a machine-readable patch manifest;
9. reject unsupported fields or values; and
10. require `C4` where the requested change falls outside the current taxonomy.

### 7.3 Generated artifacts

The generator SHOULD produce:

```text
registry-changes/RCP-0007/generated/patch.diff
registry-changes/RCP-0007/generated/patch-manifest.json
registry-changes/RCP-0007/generated/registry-preview.json
```

These generated artifacts MAY be reviewed in the pull request.

The generator, not the ordinary proposer, is responsible for producing the JSON mutation.

### 7.4 CI patch verification

CI MUST independently regenerate the patch from the submitted intake and compare it against the pull request diff.

CI MUST fail where:

- the registry diff was edited manually;
- the generated patch differs from the submitted registry mutation;
- a protected field was changed without the correct class;
- the intake and patch manifest differ;
- the evidence requirements are not satisfied; or
- a `C4` proposal attempts to mutate canonical registry JSON before taxonomy approval.

---

## 8. Mandatory Supporting Evidence

### 8.1 Evidence requirement

Every RCP MUST include at least one evidentiary artifact.

A proposal MUST NOT be accepted solely on the basis of typed text, free-form description, or unsupported assertion.

### 8.2 Permitted evidence artifact types

The permitted artifact types are:

```text
PDF_ORIGINAL
PDF_SCAN
JPEG_SCAN
PNG_SCAN
WEB_ARCHIVE
SIGNED_DIGITAL_DOCUMENT
ONCHAIN_TRANSACTION_PROOF
REGISTRY_EXPORT
API_RESPONSE_ARCHIVE
OFFICIAL_RELEASE_ARCHIVE
OTHER_EVIDENCE_TYPE_APPROVED_BY_C4
```

Where the evidence is a scanned image or non-searchable PDF, the RCP SHOULD also include Optical Character Recognition (OCR) output.

Recommended file structure:

```text
registry-changes/RCP-0007/evidence/
├── EVD-001-original.pdf
├── EVD-001-ocr.txt
├── EVD-001-metadata.yaml
└── EVD-001.sha256
```

### 8.3 Manual intake evidence

Manual intake is permitted, but it remains evidence-bound.

A manually submitted RCP MUST include at least one of:

- a scanned PDF of the original document;
- a JPEG or PNG scan with OCR output;
- a digitally signed source document;
- a web archive or official release archive;
- an on-chain transaction proof;
- a registry export;
- an archived application programming interface (API) response; or
- another evidence type previously approved through `C4`.

Manual typing MAY summarize the source. It MUST NOT substitute for the source artifact.

### 8.4 Evidence metadata

Each artifact MUST have a metadata record:

```yaml
evidence_id: EVD-001
artifact_type: PDF_SCAN
authority_level: PRIMARY
source_type: OFFICIAL_DOCUMENTATION
file_path: evidence/EVD-001-original.pdf
ocr_path: evidence/EVD-001-ocr.txt
sha256: "<required>"
retrieved_at: 2026-06-08T05:45:00Z
supports_fields:
  - protocol_required.reorg_safety_confirmations
```

CI MUST reject an evidence record where:

- the artifact file is missing;
- the SHA-256 hash is missing;
- the recorded hash does not match the file;
- the artifact type is not permitted;
- the evidence does not identify the field or fields supported; or
- OCR is required by policy but missing.

---

## 9. Emergency Track — E1

### 9.1 Permitted use

`E1` MAY be used only where delay would risk materially incorrect, unsafe, or misleading registry behavior.

Examples include:

- chain fork;
- chain halt;
- bridge exploit;
- compromised validator set;
- critical parameter change;
- invalidated attestation;
- emergency smart-contract upgrade;
- material endpoint compromise;
- discovered registry error with immediate validation impact; or
- other emergency type previously approved through `C4`.

### 9.2 E1 intake requirements

An `E1` proposal MUST include:

```yaml
emergency:
  emergency_type: CHAIN_FORK
  risk_if_delayed: >
    Existing registry values may cause incorrect validation outcomes.
  provisional_action: SUSPEND
  provisional_effective_at: 2026-06-08T06:00:00Z
  confirmatory_review_due_at: 2026-06-10T06:00:00Z
  fallback_if_unconfirmed: REVERT_OR_SUSPEND
```

Permitted `emergency_type` values:

```text
CHAIN_FORK
CHAIN_HALT
BRIDGE_EXPLOIT
VALIDATOR_COMPROMISE
CRITICAL_PARAMETER_CHANGE
ATTESTATION_INVALIDATION
EMERGENCY_CONTRACT_UPGRADE
ENDPOINT_COMPROMISE
MATERIAL_REGISTRY_ERROR
OTHER_EMERGENCY_TYPE_APPROVED_BY_C4
```

### 9.3 Evidence requirement for E1

An `E1` proposal MUST still include supporting evidence.

Where a final authoritative document is not yet available, a provisional evidentiary artifact MAY be used, such as:

- archived official announcement;
- signed maintainer notice;
- on-chain transaction proof;
- incident response notice;
- captured official status page;
- archived governance execution record; or
- authenticated API response archive.

A typed description alone is insufficient.

### 9.4 48-hour confirmatory deadline

An `E1` update MAY be provisionally activated immediately after:

1. structured intake validation;
2. evidence attachment validation;
3. deterministic patch generation;
4. CI patch verification; and
5. approval by two authorized emergency approvers.

A confirmatory review MUST be completed no later than **48 hours** after provisional activation.

The confirmatory decision MUST be one of:

```text
CONFIRMED
AMENDED
REVERTED
ESCALATED_TO_C3
ESCALATED_TO_C4
```

An `E1` update MUST NOT remain provisionally effective beyond 48 hours without a recorded decision.

Where complete resolution is not possible within 48 hours, the registry MUST default to the safer available state, such as suspension, deactivation, or explicit uncertainty classification, pending a `C3` or `C4` review.

---

## 10. CI Validation Requirements

The CI workflow MUST enforce the following invariant:

```text
Any mutation under registries/** MUST be attributable to one validated RCP,
and the registry diff MUST be reproducible from that RCP intake.
```

### 10.1 Minimum CI checks

CI MUST check:

1. RCP folder naming;
2. presence of `intake.yaml`;
3. valid `rcp_control` line;
4. closed-option schema compliance;
5. evidence artifact presence;
6. evidence metadata presence;
7. evidence SHA-256 integrity;
8. OCR presence where required;
9. permitted registry;
10. permitted operation;
11. permitted field mutation;
12. permitted requested value;
13. correct change class;
14. required escalation to `C3` or `C4`;
15. deterministic patch generation;
16. generated patch consistency;
17. canonical registry schema validation;
18. fixture impact;
19. review-period compliance;
20. required reviewer approvals;
21. changelog update; and
22. release manifest update.

### 10.2 CI rejection conditions

CI MUST reject:

- direct JSON edits without a valid RCP;
- registry diffs that cannot be regenerated from intake;
- unsupported enum values;
- unsupported fields;
- missing evidence;
- evidence hash mismatches;
- unsupported evidence types;
- missing OCR where required;
- incorrect review class;
- emergency changes without a 48-hour deadline;
- `C4` changes that mutate canonical registry state before taxonomy approval; and
- changes that bypass required fixture updates.

---

## 11. Repository Structure

Recommended structure:

```text
docs/
└── governance/
    └── R-005-registry-update-governance.md

registry-changes/
├── README.md
├── schema/
│   ├── rcp-intake.schema.json
│   ├── evidence.schema.json
│   └── patch-manifest.schema.json
├── templates/
│   ├── C1-intake.yaml
│   ├── C2-intake.yaml
│   ├── C3-intake.yaml
│   ├── C4-intake.yaml
│   └── E1-intake.yaml
└── RCP-XXXX/
    ├── intake.yaml
    ├── proposal.md
    ├── evidence/
    │   ├── EVD-001-original.pdf
    │   ├── EVD-001-ocr.txt
    │   └── EVD-001-metadata.yaml
    └── generated/
        ├── patch.diff
        ├── patch-manifest.json
        └── registry-preview.json

registries/
├── index.json                      # canonical version + per-version changelog (source of truth)
├── origins.json
├── chain_registry.json
├── bridge_registry.json
├── channel_bindings.json
├── verification_regimes.json
├── reason_codes.json
├── …                               # remaining canonical registries (classes, states, policies, …)
├── drafts/
│   └── legal_systems.json
└── releases/
    └── YYYY.MM.DD-NN.json          # R-005 release manifests (additive; reference index.json version)
```

The canonical changelog is `registries/index.json` (per-version change arrays), not a separate `CHANGELOG.md`. The `releases/` directory is a new additive artifact introduced by this addendum; it does not replace `index.json`.

---

## 12. Roles

| Role | Function |
|---|---|
| Proposer | Completes structured intake and submits evidence artifacts. |
| Registry maintainer | Reviews schema compliance, evidence sufficiency, and generated patch. |
| Domain reviewer | Reviews substantive chain, bridge, legal, identity, or verification implications. |
| Emergency approver | Authorizes provisional `E1` action. |
| Release maintainer | Merges ratified patch and publishes registry release. |
| Taxonomy reviewer | Approves or rejects `C4` schema and enum expansion. |

A small project MAY assign multiple roles to the same person, except that an `E1` provisional activation SHOULD receive two distinct human approvals.

---

## 13. Registry Release Manifest

Each merged update MUST generate a versioned release manifest.

Example:

```json
{
  "registry_release": "2026.06.08-01",
  "registry_version": "1.5.2-draft",
  "released_at": "2026-06-08T08:30:00Z",
  "included_rcps": ["RCP-0007"],
  "commit_hash": "abc123...",
  "content_hash": "sha256:...",
  "generator_version": "arrow-registry-governance-v0.1"
}
```

The `registry_version` field mirrors the canonical version string in `registries/index.json` (e.g. `1.5.2-draft`); the date-based `registry_release` id is an additional release handle and does not replace the `index.json` version.

Arrow nodes SHOULD be capable of reporting the registry release used for validation:

```text
Validated against registry release 2026.06.08-01
```

---

## 14. Minimum Viable Product Implementation

### Phase 1 — Manual but structured

Implement:

```text
docs/governance/R-005-registry-update-governance.md
registry-changes/templates/
registry-changes/schema/
registry-changes/README.md
registries/CHANGELOG.md
```

Use YAML intake forms, evidence folders, and a pull-request checklist.

### Phase 2 — Deterministic generator

Implement:

```text
scripts/generate_registry_patch.py
scripts/validate_rcp.py
```

The scripts MUST:

- validate closed options;
- validate evidence;
- generate patch artifacts;
- reject unsupported changes; and
- require `C4` where the taxonomy is insufficient.

### Phase 3 — CI enforcement

Implement:

```text
.github/workflows/registry-governance.yml
```

CI MUST reject direct JSON mutation unless the diff is reproducible from a validated RCP.

### Phase 4 — Rust integration

Migrate the generator and validator into a dedicated Rust crate:

```text
crates/arrow-registry-governance/
```

This crate SHOULD become the authoritative transformation layer for registry mutation and release-manifest generation.

---

## 15. Main Risks

### 15.1 Taxonomy overconstraint

Closed options reduce ambiguity but may prevent legitimate updates.

Mitigation: use `C4` as a controlled expansion mechanism.

### 15.2 Evidence quality drift

A process can become formalistic while accepting poor evidence.

Mitigation: classify evidence authority levels and require field-level evidence mapping.

### 15.3 Emergency-track abuse

Maintainers may overuse `E1` to bypass ordinary review.

Mitigation: require two approvers, attached evidence, a mandatory 48-hour decision, and safe-state fallback.

### 15.4 Hidden manual JSON edits

A contributor may modify JSON after generation.

Mitigation: CI independently regenerates the patch and compares it to the submitted diff.

### 15.5 Generator bugs

A deterministic generator may encode an incorrect mutation.

Mitigation: validate against registry schemas, preserve patch manifests, require review of generated previews, and maintain fixtures.

---

## 16. Implementation Decision

The recommended Arrow Protocol position is:

1. Use RCP intake as the only permitted registry-update interface.
2. Use closed-option YAML fields for proposer interaction.
3. Keep canonical registry JSON isolated from ordinary proposers.
4. Generate JSON patches deterministically.
5. Require evidence artifacts for every update, including manual intake.
6. Use `C4` where the existing taxonomy is insufficient.
7. Permit `E1` provisional action only with evidence, CI validation, and two approvers.
8. Require confirmatory action within 48 hours for every `E1` update.
9. Default unresolved emergencies to the safer registry state.
10. Treat CI reproducibility as the core enforcement invariant.

---

## 17. Open Questions

1. Which registry fields should be designated as automatically material and therefore force `C3`?
2. Which evidence types require mandatory OCR output?
3. Should web archives be stored directly in the repository or referenced by content hash in external storage?
4. Should the first implementation use Python scripts or proceed directly to a Rust crate?
5. Should a registry update form be added later as a graphical user interface (GUI) over the YAML intake format?
6. Which maintainers are authorized to approve `E1` changes?
7. Should unresolved `E1` cases automatically suspend the relevant registry entry by default?

---

## 18. Next Concrete Step

Create the following initial files:

```text
docs/governance/R-005-registry-update-governance.md
registry-changes/README.md
registry-changes/templates/C1-intake.yaml
registry-changes/templates/C2-intake.yaml
registry-changes/templates/C3-intake.yaml
registry-changes/templates/C4-intake.yaml
registry-changes/templates/E1-intake.yaml
registry-changes/schema/rcp-intake.schema.json
scripts/validate_rcp.py
scripts/generate_registry_patch.py
```

The first implementation SHOULD remain narrow: validate one registry, one permitted field update, one evidence artifact, and one deterministic patch path before generalizing the framework.
