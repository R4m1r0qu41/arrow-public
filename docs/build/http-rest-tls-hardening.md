# HTTP REST TLS Hardening v0.2

Date: 2026-06-01

This is a narrow hardening pass for the existing HTTP REST pilot. It is not a production-readiness claim.

## Implemented Architecture

- `http-rest-local` is the active local-development binding. It is restricted by startup validation to loopback listeners and local `http://` public URLs.
- `https-rest` is the active production-facing deployment profile. The MVP uses TLS termination at a trusted reverse proxy or managed ingress, then forwards to `arrow-node` on loopback or a private interface.
- Legacy `http-rest` remains in `registries/channel_bindings.json` as an inactive compatibility marker. It is rejected by validated node startup config and by TPT-004 envelope validation.
- `arrow-node` validates `channel.binding` against the channel-binding registry at startup and uses the resolved binding code in health responses, event ACKs, ACK receives, and channel mismatch checks.
- Application-level Ed25519 signed envelopes remain mandatory for validated node startup configuration. TLS does not replace envelope signatures.
- The receive pipeline remains fail-closed: content type, deserialization, stateless transport validation, channel binding, receiver binding, known sender, signature verification, replay protection, semantic validation, then ACK/error response.

## Threat Model

The v0.2 profile addresses accidental public plain-HTTP exposure and misconfigured TLS termination boundaries. It assumes:

- The reverse proxy terminates TLS and is operated inside the deployment trust boundary.
- `arrow-node` is reachable only from loopback or a private network path behind that proxy.
- Clients still sign Arrow transport envelopes with Ed25519 at the application layer.
- Registry validation is part of release and deployment checks.

It does not defend against a compromised reverse proxy, stolen signing keys, host compromise, replay database tampering by a local attacker, or application payload disclosure after TLS termination.

## TLS Protects

HTTPS/TLS protects data in transit between the external client and the TLS terminator. With the v0.2 model, the external public hop is encrypted and authenticated by the proxy certificate.

## TLS Does Not Protect

TLS does not encrypt data at rest. It does not protect SQLite replay databases, identity key files, local logs, backups, process memory, or payloads after they have been decrypted at the proxy. TLS also does not prove Arrow sender authority, receiver intent, replay freshness, or semantic validity. Those remain application-level responsibilities.

## Deployment Diagram

```text
Internet client
  |
  | HTTPS, valid public certificate
  v
Caddy / Nginx / managed ingress
  |  X-Forwarded-Proto: https
  |  trusted proxy boundary
  v
arrow-node on 127.0.0.1 or private address
  |
  | Ed25519 envelope verification, replay check, semantic validation
  v
Arrow validation result / ACK
```

## Request Enforcement

For `https-rest`, `arrow-node` requires:

- `tls.mode = "terminated-upstream"`;
- `tls.require_forwarded_proto_https = true`;
- `tls.trusted_proxy` set to the reverse proxy IP address;
- request peer address matching `tls.trusted_proxy`;
- `X-Forwarded-Proto: https` or `Forwarded: proto=https`.

Peer-address enforcement is available on the main server path through Axum connect info. Tests that construct routers directly should use the local profile unless they explicitly serve with connect info.

## Sample Caddy Configuration

```caddyfile
node.example.test {
  reverse_proxy 127.0.0.1:8080 {
    header_up X-Forwarded-Proto https
    header_up X-Forwarded-Host {host}
  }
}
```

## Sample Nginx Configuration

```nginx
server {
    listen 443 ssl http2;
    server_name node.example.test;

    ssl_certificate     /etc/letsencrypt/live/node.example.test/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/node.example.test/privkey.pem;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-Proto https;
        proxy_set_header X-Forwarded-Host $host;
    }
}
```

## Local Development TOML

```toml
listen = "127.0.0.1:8080"
node_id = "did:arrow:pilot:node-a"
registries = "embedded"

[channel]
binding = "http-rest-local"
public_base_url = "http://127.0.0.1:8080"

[tls]
mode = "local-disabled"
require_forwarded_proto_https = false

[known_senders]
allow = ["did:key:z6Mk..."]

[identity]
require_signature = true
allowed_did_methods = ["did:key"]
allowed_algorithms = ["Ed25519"]
```

## Production-Facing TOML

```toml
listen = "127.0.0.1:8080"
node_id = "did:arrow:prod:node-a"
registries = "/etc/arrow/registries"

[channel]
binding = "https-rest"
public_base_url = "https://node.example.test"

[tls]
mode = "terminated-upstream"
require_forwarded_proto_https = true
trusted_proxy = "127.0.0.1"

[replay]
backend = "sqlite"
path = "/var/lib/arrow/replay.db"

[known_senders]
allow = ["did:key:z6Mk..."]

[identity]
require_signature = true
allowed_did_methods = ["did:key"]
allowed_algorithms = ["Ed25519"]
```

## Data At Rest Assessment

SQLite replay database: the SQLite replay database path is configured with `[replay].path` and defaults relative to the node config when SQLite is enabled. Encryption at rest is not implemented. Deployments must use restrictive file permissions, protected host storage, and backup controls. Replay DB contents and backups should be treated as operational security data.

Identity key files: development key files are raw dev/test key material and must not be used for production. Before production, signing keys need KMS, HSM, or operating-system secure storage with access control, rotation, and auditability. This pass does not implement key custody.

Logs: logs must not include private keys, signing secrets, unnecessary personal/legal payload data, or committed secrets. Existing logging remains operationally narrow; deployment logging policy is still required.

Configuration secrets: no secrets should be committed. The example configs contain placeholders only.

## Deferred Production Requirements

- Native Rust TLS listener support is deferred; the MVP profile uses upstream TLS termination.
- mTLS is deferred as a later channel-binding profile.
- Encryption at rest for replay databases, logs, and key material is deferred to platform/KMS/HSM controls.
- Full proxy hardening, certificate lifecycle management, WAF/rate limits, and operational monitoring are deployment responsibilities.
- This profile does not claim production readiness.

## Tests Run

Final verification commands run on 2026-06-01:

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --all-targets -- -D warnings
node tools/validate-registries.js
git status --short
git diff --stat
```

Result for this patch: all format, workspace tests, Clippy, and registry validation pass; workspace tests report 206 passed, 0 failed, 2 ignored.
