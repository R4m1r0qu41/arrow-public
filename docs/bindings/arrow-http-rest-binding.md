# Arrow HTTP REST Binding Pilot — v1.0

**Status:** live (pilot)  
**Binding Code:** `http-rest`  
**Registry Entry:** `registries/channel_bindings.json`  
**Transport Model:** Arrow Transport Model v1.0  
**Date:** 2026-05-23

> **Deployment note:** The Arrow HTTP REST Binding is intended to run behind HTTPS/TLS
> termination in production-like deployments. Native TLS termination is out of scope
> for the pilot node.

---

## 1. Overview

This binding defines how Arrow Transport Envelopes are exchanged over HTTP using a
synchronous request/response pattern with a JSON wire format. The validation
acknowledgment is returned in the HTTP response body.

---

## 2. Security Boundary

This pilot uses **unsigned transport envelopes**. Unsigned envelopes are permitted
only for pilot, local, or controlled test environments.

This pilot demonstrates:

- Envelope delivery
- Replay protection via nonce
- Channel binding enforcement
- Receiver matching
- Validation acknowledgments
- Semantic validation handoff

This pilot **does not** prove:

- Cryptographic sender authorship
- Payload integrity against an active network attacker
- Legal-grade attribution or non-repudiation
- Production VASP readiness

Nonce replay protection reduces duplicate delivery risk. It is **not** a substitute
for cryptographic signatures. A signed binding variant is deferred pending key
registry and DID resolution design.

---

## 3. Wire Format

All request and response bodies are `application/json` (UTF-8). Both `POST /arrow/events`
and `POST /arrow/ack` require the `Content-Type: application/json` header. Requests
without it, or with a non-JSON Content-Type, are rejected with `415 Unsupported Media Type`.

---

## 4. Nonce Format

Nonces must match: `^n-[0-9a-f]{32}$`

Example: `n-aabbccddeeff00112233445566778899`

---

## 5. Endpoints

### 5.1 `POST /arrow/events`

Receives an inbound `TransportEnvelope`. Processing pipeline:

1. Content-Type guard — non-JSON → `415`
2. Deserialize body as `TransportEnvelope` — failure → `400 TPT-MALFORMED`
3. Stateless transport validation (TPT-001..005, TPT-010, TPT-040, TPT-041)
4. Channel binding match — `envelope.channel_binding` must equal `"http-rest"` → `400`
5. Receiver match — `envelope.receiver_id` must include this node's ID → `403 TPT-RECEIVER-MISMATCH`
6. Known sender check → `403 TPT-UNKNOWN-SENDER`
7. Replay protection — `(sender_id, nonce)` pair → `409 TPT-REPLAY-DETECTED`
8. Semantic validation via `ValidationPipeline`
9. Return `AckEnvelope` (validation type) with HTTP `200` regardless of semantic result

A semantic validation failure is reported through `AckEnvelope.result` (`"rejected"`),
not through an HTTP error status (Transport Model Part 5.4).

### 5.2 `POST /arrow/ack`

Receives an inbound `AckEnvelope` (asynchronous delivery case). Validates:

- Content-Type is JSON → `415`
- `ack_version == "1.0"` → `400`
- `channel_binding == "http-rest"` → `400`
- `receiver_id` matches this node → `403 TPT-RECEIVER-MISMATCH`
- `sender_id` is allowlisted → `403 TPT-UNKNOWN-SENDER`
- `sent_at` is valid RFC 3339 and not more than 5 minutes in the future → `400`
- `ref_envelope_id` or `ref_envelope_hash` is present (TPT-030) → `400`

Returns `200 {}` on success.

### 5.3 `GET /arrow/health`

Returns node identity. No authentication required.

```json
{ "node_id": "did:arrow:pilot:node-a", "channel_binding": "http-rest", "transport_model": "1.0" }
```

---

## 6. HTTP Status Mapping

| Condition | `error_code` | HTTP Status |
|---|---|---:|
| Valid envelope | — | 200 |
| Semantic rejection | — | 200 (`result: "rejected"`) |
| Malformed JSON or missing field | `TPT-MALFORMED` | 400 |
| Version mismatch | `TPT-VERSION-UNSUPPORTED` | 400 |
| Malformed nonce | `TPT-MALFORMED` | 400 |
| Future `sent_at` beyond tolerance | `TPT-STALE` | 400 |
| Receiver mismatch | `TPT-RECEIVER-MISMATCH` | 403 |
| Unknown sender | `TPT-UNKNOWN-SENDER` | 403 |
| Duplicate nonce | `TPT-REPLAY-DETECTED` | 409 |
| Wrong Content-Type | `TPT-MALFORMED` | 415 |

---

## 7. Binding Registry Values

| Field | Value |
|---|---|
| `code` | `http-rest` |
| `ordered` | `false` |
| `reliable` | `true` |
| `supports_ack` | `true` |
| `requires_ack` | `true` |
| `supports_signature` | `false` |
| `requires_signature` | `false` |
| `nonce_retention_min` | `PT24H` |
| `freshness_stale_window` | `PT24H` (warning only) |
| `freshness_future_tolerance` | `PT5M` (fatal: TPT-041) |
| `status` | `active` |
| `added_version` | `1.3.2` |

---

## 8. Known Pilot Limitations

- The replay store is in-memory and does not survive node restarts. See `docs/transport/replay-store-sqlite-design-v0.1.md` for the persistent store design.
- Sequence enforcement is not implemented (`ordered: false`); `sequence` fields are parsed and stored only.
- Acknowledgment envelopes are not replay-protected.
- No native TLS; intended to run behind a TLS-terminating proxy.

---

*Arrow Protocol · HTTP REST Binding Pilot · v1.0 · 2026-05-23*
