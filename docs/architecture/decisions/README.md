# Arrow Architecture Decision Records

Arrow-owned ADRs. Identifier scheme: `ADR-ARRW-NNN` (per ADR-ARRW-001 §2.4, mirroring
the RFC naming model). Cross-repository governance ADRs are owned here; Tarquin and Orlog
express adoption through their own `ADR-TARQ-NNN` / `ADR-ORLG-NNN` records that reference
the Arrow ADR rather than copy it.

| ADR | Title | Status | Date |
|---|---|---|---|
| [ADR-ARRW-001](ADR-ARRW-001-protocol-authority-and-baseline-liveness.md) | Protocol Authority, Baseline Liveness, and ADR Governance | live | 2026-06-19 |

## Relationship to prior ADRs

The CipherLayer stack previously kept its governing liveness/archive ADR in Tarquin
(`Tarquin/decisions/ADR-0002`). ADR-ARRW-001 supersedes the *stack-governing scope* of
that ADR and re-homes governance authority to Arrow, the protocol authority. Tarquin
ADR-0001/0002/0003 are preserved unchanged under their original identifiers as historical
decision records; downstream adoption is recorded in Tarquin `ADR-TARQ-001` and Orlog
`ADR-ORLG-001`.
