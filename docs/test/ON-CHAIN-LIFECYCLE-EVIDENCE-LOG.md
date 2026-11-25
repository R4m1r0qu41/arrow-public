# On-Chain Lifecycle Evidence Log

This log records real on-chain lifecycle-test evidence generated for the RFC-006
Chain Registry backfill process. Testnet evidence proves the harness,
transaction observation, artifact hashing, and registry backfill mechanics; it
does not prove mainnet `protocol_required` values.

## 2026-06-15 — ETH Mainnet

**Status:** Complete for ETH mainnet. The full mainnet pass remains partial
until Base, Arbitrum, Solana, and Bitcoin mainnet evidence are recorded.

**Purpose:** RFC-006 R-009 / R-011 public mainnet pass for the Ethereum
probabilistic-finality path.

**Transaction**

- Network: Ethereum Mainnet
- EIP-155 chain id: `1`
- Source: `0x7cD33930465A2258B89A25B59A1ACd5aa033932E`
- Target: `0x7cB85E88a52726D70CCf26308f46fA2E7522c1F4`
- Value: `1` wei
- Transaction hash:
  `0x505bb0130e46b17e1adec44b288b7fc544fb31776f0f902d46bc475f14e77175`
- Inclusion block: `25320710`
- Receipt status: `1`
- Gas used: `21000`
- Effective gas price: `54878869` wei

**Extraction Artifact**

- Artifact file:
  `docs/test/evidence/arrow-eth-mainnet-lifecycle.json`
- Arrow code: `1:ETH:FN:PR:1:A`
- Observed chain id: `1`
- Confirmations at observation: `12`
- Observed at: `2026-06-15T05:10:45.187108+00:00`
- Extraction artifact hash:
  `sha256:ba36852ae91e758b70f916c6f7c190d14fcab27d62a3242cc36d37db497929cd`

**Commands**

```sh
cast send \
  --rpc-url https://ethereum-rpc.publicnode.com \
  --private-key "$ARROW_MAINNET_KEY" \
  0x7cB85E88a52726D70CCf26308f46fA2E7522c1F4 \
  --value 1wei
```

```sh
cargo run -p arrow-registry-test -- lifecycle run \
  --origin ETH \
  --network mainnet \
  --tx 0x505bb0130e46b17e1adec44b288b7fc544fb31776f0f902d46bc475f14e77175 \
  --rpc https://ethereum-rpc.publicnode.com \
  --out docs/test/evidence/arrow-eth-mainnet-lifecycle.json
```

```sh
cargo run -p arrow-registry-test -- artifact hash \
  docs/test/evidence/arrow-eth-mainnet-lifecycle.json
```

**Registry Note**

This evidence is retained as the ETH component of the mainnet pass. Per the
runbook, `registries/chain_registry.json` should be backfilled after all five
mainnet artifacts are generated, then hashed and timestamped as one registry
snapshot.

## 2026-06-15 — Base Mainnet

**Status:** Complete for Base mainnet. The full mainnet pass remains partial
until Arbitrum, Solana, and Bitcoin mainnet evidence are recorded.

**Purpose:** RFC-006 R-009 / R-011 public mainnet pass for the Base
optimistic-rollup path.

**Transaction**

- Network: Base Mainnet
- EIP-155 chain id: `8453`
- Source: `0x7cD33930465A2258B89A25B59A1ACd5aa033932E`
- Target: `0x7cB85E88a52726D70CCf26308f46fA2E7522c1F4`
- Value: `1` wei
- Transaction hash:
  `0x720648dfed963eba6b52e591b361f19368c8de56c201b6552a5ff0b4da9fba8a`
- Inclusion block: `47355664`
- Block hash:
  `0x0c349a5e03347f63373fda0217a95f932ce29a2395708c5c4778b6a5ab91c3c4`
- Receipt status: `1`
- Gas used: `21000`
- Effective gas price: `5207160` wei

**Extraction Artifact**

- Artifact file:
  `docs/test/evidence/arrow-base-mainnet-lifecycle.json`
- Arrow code: `1:BSE:PD:HL:0:B`
- Observed chain id: `8453`
- Confirmations at observation: `2`
- Observed at: `2026-06-15T05:17:59.878205+00:00`
- Extraction artifact hash:
  `sha256:93c4d659cfd58d9d7e1d3723297ad2ba1ed91fcc2c7cbf7a1f365b1f834d854d`

**Commands**

```sh
cast send \
  --rpc-url https://mainnet.base.org \
  --private-key "$ARROW_MAINNET_KEY" \
  0x7cB85E88a52726D70CCf26308f46fA2E7522c1F4 \
  --value 1wei
```

```sh
cargo run -p arrow-registry-test -- lifecycle run \
  --origin BASE \
  --network mainnet \
  --tx 0x720648dfed963eba6b52e591b361f19368c8de56c201b6552a5ff0b4da9fba8a \
  --rpc https://mainnet.base.org \
  --out docs/test/evidence/arrow-base-mainnet-lifecycle.json
```

```sh
cargo run -p arrow-registry-test -- artifact hash \
  docs/test/evidence/arrow-base-mainnet-lifecycle.json
```

**Registry Note**

This evidence is retained as the Base component of the mainnet pass. Per the
runbook, `registries/chain_registry.json` should be backfilled after all five
mainnet artifacts are generated, then hashed and timestamped as one registry
snapshot.

## 2026-06-15 — Arbitrum One Mainnet

**Status:** Complete for Arbitrum One mainnet. The full mainnet pass remains
partial until Solana and Bitcoin mainnet evidence are recorded.

**Purpose:** RFC-006 R-009 / R-011 public mainnet pass for the Arbitrum One
optimistic-rollup path.

**Transaction**

- Network: Arbitrum One Mainnet
- EIP-155 chain id: `42161`
- Source: `0x7cD33930465A2258B89A25B59A1ACd5aa033932E`
- Target: `0x7cB85E88a52726D70CCf26308f46fA2E7522c1F4`
- Value: `1` wei
- Transaction hash:
  `0x401abd713b25a8a037132091f26369b19c42aa34b46e0eee0686336ed7709652`
- Inclusion block: `473645390`
- Block hash:
  `0xb343856899e3d23efc9b005a5d514e42d97174d8739088f77a339e631e5c9977`
- Receipt status: `1`
- Gas used: `21127`
- Effective gas price: `20072000` wei

**Extraction Artifact**

- Artifact file:
  `docs/test/evidence/arrow-arb-mainnet-lifecycle.json`
- Arrow code: `1:ARB:PD:HL:0:B`
- Observed chain id: `42161`
- Confirmations at observation: `14`
- Observed at: `2026-06-15T05:20:23.927800+00:00`
- Extraction artifact hash:
  `sha256:5d3567bee90d29cb4dfc2fc0c13412a14ecd000f9f34d7ee9909ed6dd7935770`

**Commands**

```sh
cast send \
  --rpc-url https://arb1.arbitrum.io/rpc \
  --private-key "$ARROW_MAINNET_KEY" \
  0x7cB85E88a52726D70CCf26308f46fA2E7522c1F4 \
  --value 1wei
```

```sh
cargo run -p arrow-registry-test -- lifecycle run \
  --origin ARB \
  --network mainnet \
  --tx 0x401abd713b25a8a037132091f26369b19c42aa34b46e0eee0686336ed7709652 \
  --rpc https://arb1.arbitrum.io/rpc \
  --out docs/test/evidence/arrow-arb-mainnet-lifecycle.json
```

```sh
cargo run -p arrow-registry-test -- artifact hash \
  docs/test/evidence/arrow-arb-mainnet-lifecycle.json
```

**Registry Note**

This evidence is retained as the Arbitrum component of the mainnet pass. Per
the runbook, `registries/chain_registry.json` should be backfilled after all
five mainnet artifacts are generated, then hashed and timestamped as one
registry snapshot.

## 2026-06-15 — Solana Mainnet-Beta

**Status:** Complete for Solana mainnet-beta. The full mainnet pass remains
partial until Bitcoin mainnet evidence is recorded.

**Purpose:** RFC-006 R-009 / R-011 public mainnet pass for the Solana
BFT-consensus path.

**Transaction**

- Network: Solana Mainnet-Beta
- Genesis hash:
  `5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d`
- Source: `AbZmoJqS2KrxVsEo3wi5CMHjpJCBrdovgcu1xH9KGpHE`
- Target: `BAyrxGvrV8DLwFVZygMMp3GgYNMpE1tJNGD53Lw2Ffi2`
- Value: `4105036` lamports (`0.004105036 SOL`)
- Transaction signature:
  `pbY7vFe5ZL1nUHpMPmKWYdUoMTVEBTutdDLYqd4PqdukX3XquJ9GMHKY7NajH6uLZBnFp77A1h47LtRsz9EKhGg`
- Slot: `426574628`
- Status: `Ok`
- Finalization: `Finalized`
- Fee: `5005` lamports

**Extraction Artifact**

- Artifact file:
  `docs/test/evidence/arrow-sol-mainnet-lifecycle.json`
- Arrow code: `1:SOL:FN:PR:1:A`
- Observed genesis hash:
  `5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d`
- Observed slot: `426574628`
- Observed at: `2026-06-15T05:57:00.280495+00:00`
- Extraction artifact hash:
  `sha256:163c3e645e8b6aa2627c802ad3a84bdeb7d2682bc5ba34a248c7c6e268563479`

**Commands**

```sh
solana confirm -v \
  pbY7vFe5ZL1nUHpMPmKWYdUoMTVEBTutdDLYqd4PqdukX3XquJ9GMHKY7NajH6uLZBnFp77A1h47LtRsz9EKhGg \
  --url https://api.mainnet-beta.solana.com
```

```sh
cargo run -p arrow-registry-test -- lifecycle run \
  --origin SOL \
  --network mainnet \
  --tx pbY7vFe5ZL1nUHpMPmKWYdUoMTVEBTutdDLYqd4PqdukX3XquJ9GMHKY7NajH6uLZBnFp77A1h47LtRsz9EKhGg \
  --rpc https://api.mainnet-beta.solana.com \
  --out docs/test/evidence/arrow-sol-mainnet-lifecycle.json
```

```sh
cargo run -p arrow-registry-test -- artifact hash \
  docs/test/evidence/arrow-sol-mainnet-lifecycle.json
```

**Registry Note**

This evidence is retained as the Solana component of the mainnet pass. Per the
runbook, `registries/chain_registry.json` should be backfilled after all five
mainnet artifacts are generated, then hashed and timestamped as one registry
snapshot.

## 2026-06-16 — Mainnet Registry OTS Anchor

**Status:** Bitcoin-confirmed (stamped and upgraded 2026-06-16).

**Purpose:** RFC-006 R-004 / R-010 timestamp proof for the mainnet Chain Registry
after the full five-chain lifecycle-test backfill. All five entries (ETH, BTC,
SOL, BASE, ARB) now carry a populated `lifecycle_test` block bound to real
mainnet on-chain evidence.

**Registry Snapshot**

- Registry file: `registries/chain_registry.json`
- Canonical registry version hash (stored in each entry's `anchoring`;
  reproducible by setting every `anchoring.entry_hash` and
  `anchoring.registry_version_hash` to `null`, then JCS + SHA-256):
  `sha256:e77c959a411b3a4dd5f52bd2a56dd296d408ac8957174e6539969664138fa0fc`
- As-is JCS hash (`anchor hash` over the populated file, informational only —
  not a fixed point):
  `sha256:4657ac104867116573a9e57d711aa83be2063c6f6b4e56e0e5994391bd1c0fc1`
- OTS proof file: `registries/chain_registry.json.ots`
- File SHA-256 reported by `ots info` (the bytes the proof commits to):
  `1585029b1c2f374e05a5cbab9c1701e26cbde048456b77cf1b8e0f88619b8b5b`

**Per-entry hashes** (fixed-point; computed with the entry's `anchoring`
hash-fields nulled):

| Code | entry_hash |
|------|------------|
| ETH  | `sha256:8abc63f8af92e604d7b588b8d8a4326480b694a968490e04c092b42674cc37f9` |
| BTC  | `sha256:071d5ef9d8770f8fcfbf18536e99254c99b59e0ebb6777227716030b7deaf17e` |
| SOL  | `sha256:bc24cadda836623d8a6a0c8c247dc37fcb46fa0e962653a16852c6e2885ff30e` |
| BASE | `sha256:5d7af6eeea562e1093d7bb7a768f8a06427aa426998ed715f443c615cd89ae18` |
| ARB  | `sha256:8bad0a4af34d86b624ef7dde504687aed78df5ff9323c5fa024a2b40685e8f2e` |

Only BTC's `entry_hash` changed in this snapshot (its `lifecycle_test` went from
`null` to populated, `fb0c857a…` → `071d5ef9…`); the other four are unchanged.

**OpenTimestamps Status**

Stamped 2026-06-16 and upgraded the same day. The committed file digest
(`1585029b…`) is anchored in Bitcoin block `953889` (mined
`2026-06-16T04:57:45Z`) via the `alice.btc.calendar.opentimestamps.org`
attestation — its expected block merkle root
(`5a26532e717a49f97c581d69c40017b5b78f51dbaaea4044dade9075fbfb70aa`) was
confirmed against the local synced node. The other three calendars
(`bob…`, `finney…`, `catallaxy…`) still hold pending attestations and will
attach on a later `ots upgrade`; one confirmed Bitcoin attestation already
establishes the timestamp.

**Anchor record placement.** As with the testnet anchor, `anchoring.onchain_anchor`
is left `null` and the anchor is recorded here in the evidence log. The OTS proof
commits the exact file bytes; writing the anchor record back into the registry
would change those bytes and invalidate both the proof and the canonical
`registry_version_hash`. This intentionally departs from the runbook's literal
"Record the resulting commitment in `anchoring.onchain_anchor`" instruction, which
is incompatible with hashing the full file — flagged for a runbook correction.

**Commands**

```sh
cargo run -p arrow-registry-test -- anchor hash registries/chain_registry.json --entries
ots stamp registries/chain_registry.json
ots upgrade registries/chain_registry.json.ots
ots info registries/chain_registry.json.ots
```

## 2026-06-25 — Mainnet Registry OTS Anchor Desync (decision-9 review)

During the decision-9 external-lifecycle verification
(`docs/implementation/soft-reboot-2026-06-17/DECISION-9-EXTERNAL-LIFECYCLE-DETERMINATION.md`)
a desync was found between the stamped registry bytes and the live file. Commit
`aebcdf2` ("Strip all RFC references from machine-readable JSON") changed one descriptive
string in `registries/chain_registry.json` *after* the 2026-06-16 stamp
(`"…deferred to RFC-006 v0.2, all null."` → `"…deferred, all null."`). It touched no
`lifecycle_test`, `tx_ref`, or hash value, so the substantive mainnet evidence is intact,
but the file digest moved:

- OTS-committed digest (proof `registries/chain_registry.json.ots`, anchored in Bitcoin
  block `953889`): `1585029b1c2f374e05a5cbab9c1701e26cbde048456b77cf1b8e0f88619b8b5b`
- Current file SHA-256: `cb0e2e5af07e8ad14d818df5765c1a60c964566ac0bd9548fb7f6fac6a831050`

The Bitcoin attestation therefore proves the *2026-06-16 stamped snapshot* existed, not
the current file. Remediation: re-run `ots stamp registries/chain_registry.json` so the
proof commits to the current bytes (or pin the originally stamped snapshot alongside its
`.ots`). Not a pilot blocker; the lifecycle evidence itself remains valid.

## 2026-06-14 — Testnet Registry OTS Anchor

**Status:** Bitcoin-confirmed (upgraded 2026-06-15).

**Purpose:** RFC-006 R-004 / R-010 timestamp proof for the testnet Chain
Registry overlay after lifecycle-test backfill.

**Registry Snapshot**

- Registry file: `registries/chain_registry.testnet.json`
- Registry version hash:
  `sha256:dd9005ffaf05071c69389aad8260b28700406c2fc07d4035dcde468920668ae6`
- OTS proof file: `registries/chain_registry.testnet.json.ots`
- File SHA-256 reported by `ots info`:
  `17232fffb7122b73f7ea6f48500b431ae105a6829f22771a32674b8158556601`

**OpenTimestamps Status**

`ots upgrade` on 2026-06-15 attached Bitcoin block-header attestations from all
four calendars. The committed file digest (`17232fff…`) is now anchored in:

- Bitcoin block `953742` (earliest; mined `2026-06-15T04:47:56Z`)
- Bitcoin block `953743`
- Bitcoin block `953770`

Each attestation's expected block merkle root was independently confirmed
against the local synced mainnet node (`getblockheader`):

| Block | Merkle root |
|-------|-------------|
| `953742` | `36097569a301089dda7b4db345c14a77616708b690b64f930b945e0b10df5b47` |
| `953743` | `a9729f775eac1c57d289ebcc49adddf4549bf4b1e061603b41359ea65adc22a8` |
| `953770` | `98f8918b22da0530ce63000324bb202ee3482316a1e1b4b1000553021bfbe7f3` |

**Anchor record placement.** The anchor metadata is recorded here, in the
evidence log — *not* written back into `chain_registry.testnet.json`'s
`anchoring.onchain_anchor`. The OTS proof commits to the exact file bytes
(`17232fff…`); mutating the registry to embed its own anchor would change those
bytes and invalidate the proof. Any in-registry anchor reference must therefore
target a *new* registry version that is itself re-stamped.

**Commands**

```sh
ots stamp registries/chain_registry.testnet.json
ots upgrade registries/chain_registry.testnet.json.ots
ots info registries/chain_registry.testnet.json.ots
```

## 2026-06-14 — ETH Sepolia

**Status:** Complete.

**Purpose:** RFC-006 R-009 / R-011 public testnet pass for the Ethereum
probabilistic-finality path.

**Transaction**

- Network: Ethereum Sepolia
- EIP-155 chain id: `11155111`
- Source: `0x7cD33930465A2258B89A25B59A1ACd5aa033932E`
- Target: `0x7cB85E88a52726D70CCf26308f46fA2E7522c1F4`
- Value: `1` wei
- Transaction hash:
  `0xce219718431bb06fe14776b28d7a29ad68d75aacf5cc55be65af2c8fea709243`
- Inclusion block: `11058123`
- Receipt status: `1`

**Extraction Artifact**

- Artifact file:
  `docs/test/evidence/2026-06-14-eth-sepolia-lifecycle.json`
- Arrow code: `1:ETH:FN:PR:1:A`
- Observed chain id: `11155111`
- Confirmations at observation: `17`
- Observed at: `2026-06-14T11:51:58.298589+00:00`
- Extraction artifact hash:
  `sha256:ab100442c1cc440436ec80994f2c91b56adb0d7b6eb100ff046ec7c11831a896`

**Commands**

```sh
cast send \
  --rpc-url "$ARROW_SEPOLIA_RPC" \
  --private-key "$ARROW_TESTNET_KEY" \
  "$ARROW_TESTNET_RECIPIENT" \
  --value 1wei
```

```sh
cargo run -p arrow-registry-test -- lifecycle run \
  --origin ETH \
  --network sepolia \
  --tx 0xce219718431bb06fe14776b28d7a29ad68d75aacf5cc55be65af2c8fea709243 \
  --rpc https://ethereum-sepolia-rpc.publicnode.com \
  --out /tmp/arrow-eth-sepolia-lifecycle.json
```

```sh
cargo run -p arrow-registry-test -- artifact hash \
  /tmp/arrow-eth-sepolia-lifecycle.json
```

**Harness Note**

This run exposed a harness defect: `arrow-origin-evm` used a plain Hyper
`HttpConnector`, so the documented HTTPS public RPC endpoints failed with
`invalid URL, scheme is not http`. The observer was updated to use `reqwest`
with rustls for read-only JSON-RPC calls. That fix is retained because the
runbook and default testnet RPCs use HTTPS endpoints.

**Project Meaning**

This is the first real public-chain lifecycle extraction for the Chain Registry
graduation gate. It proves the ETH/Sepolia path can:

- execute an independent minimal-value transfer;
- bind the artifact to the observed EIP-155 chain id;
- map observed transaction state into the expected Arrow code;
- produce a reproducible R-011 extraction artifact hash.

It does not by itself complete RFC-006 graduation. The full testnet lifecycle
set is complete once ETH Sepolia, Base Sepolia, Arbitrum Sepolia, Solana devnet,
and Bitcoin Signet evidence are all recorded.

## 2026-06-15 — Base Sepolia

**Status:** Complete lifecycle observation. The resulting Arrow state is
intentionally pending because Base is modeled as an optimistic rollup with an
optimistic-window finality basis.

**Purpose:** RFC-006 R-009 / R-011 public testnet pass for the Base optimistic
rollup finality path.

**Transaction**

- Network: Base Sepolia
- EIP-155 chain id: `84532`
- Source: `0x04e22876d00d1D9f88468713b10CF364898014bb`
- Target: `0x7cB85E88a52726D70CCf26308f46fA2E7522c1F4`
- Value: `1` wei
- Transaction hash:
  `0x1d0a412248ceed527f5373d09faedbfbd6fe58448b3d667cc5bfd98b14fdaef4`
- Inclusion block: `42862780`
- Receipt status: `1`

**Extraction Artifact**

- Artifact file:
  `docs/test/evidence/2026-06-15-base-sepolia-lifecycle.json`
- Arrow code: `1:BSE:PD:HL:0:B`
- Observed chain id: `84532`
- Confirmations at observation: `66`
- Observed at: `2026-06-15T03:26:19.403518+00:00`
- Finality timing: `unix_ts:1782098648`
- Extraction artifact hash:
  `sha256:64a03d1a2509583ae8e309a4daeba42f33a67210107679fd2bae4dd8fd6aabdf`

**Commands**

```sh
cast send \
  --rpc-url "$ARROW_BASE_SEPOLIA_RPC" \
  --private-key "$ARROW_TESTNET_KEY" \
  "$ARROW_TESTNET_RECIPIENT" \
  --value 1wei
```

```sh
cargo run -p arrow-registry-test -- lifecycle run \
  --origin BASE \
  --network base-sepolia \
  --tx 0x1d0a412248ceed527f5373d09faedbfbd6fe58448b3d667cc5bfd98b14fdaef4 \
  --rpc https://sepolia.base.org \
  --out /tmp/arrow-base-sepolia-lifecycle.json
```

```sh
cargo run -p arrow-registry-test -- artifact hash \
  /tmp/arrow-base-sepolia-lifecycle.json
```

**Project Meaning**

This proves the Base Sepolia path can observe a real minimal-value L2 transfer,
bind the artifact to EIP-155 chain id `84532`, preserve the optimistic-window
finality basis, compute the ETA field, and emit the expected pending Arrow code
for an optimistic rollup.

## 2026-06-15 — Arbitrum Sepolia

**Status:** Complete lifecycle observation. The resulting Arrow state is
intentionally pending because Arbitrum is modeled as an optimistic rollup with
an optimistic-window finality basis.

**Purpose:** RFC-006 R-009 / R-011 public testnet pass for the Arbitrum
optimistic rollup finality path.

**Transaction**

- Network: Arbitrum Sepolia
- EIP-155 chain id: `421614`
- Source: `0x04e22876d00d1D9f88468713b10CF364898014bb`
- Target: `0x7cB85E88a52726D70CCf26308f46fA2E7522c1F4`
- Value: `1` wei
- Transaction hash:
  `0x1f901e3e51364f68f10c31a697987f8e27719a02239ec2c374c43f96337ab57d`
- Inclusion block: `277354032`
- Receipt status: `1`

**Extraction Artifact**

- Artifact file:
  `docs/test/evidence/2026-06-15-arb-sepolia-lifecycle.json`
- Arrow code: `1:ARB:PD:HL:0:B`
- Observed chain id: `421614`
- Confirmations at observation: `147`
- Observed at: `2026-06-15T03:26:19.264702+00:00`
- Finality timing: `unix_ts:1782098741`
- Extraction artifact hash:
  `sha256:37438a6f027e693a1df2f2d5682a284d4c7aa2a91855ea2b19c44fec0c3de324`

**Commands**

```sh
cast send \
  --rpc-url "$ARROW_ARB_SEPOLIA_RPC" \
  --private-key "$ARROW_TESTNET_KEY" \
  "$ARROW_TESTNET_RECIPIENT" \
  --value 1wei
```

```sh
cargo run -p arrow-registry-test -- lifecycle run \
  --origin ARB \
  --network arb-sepolia \
  --tx 0x1f901e3e51364f68f10c31a697987f8e27719a02239ec2c374c43f96337ab57d \
  --rpc https://sepolia-rollup.arbitrum.io/rpc \
  --out /tmp/arrow-arb-sepolia-lifecycle.json
```

```sh
cargo run -p arrow-registry-test -- artifact hash \
  /tmp/arrow-arb-sepolia-lifecycle.json
```

**Project Meaning**

This proves the Arbitrum Sepolia path can observe a real minimal-value L2
transfer, bind the artifact to EIP-155 chain id `421614`, preserve the
optimistic-window finality basis, compute the ETA field, and emit the expected
pending Arrow code for an optimistic rollup.

## 2026-06-14 — SOL Devnet

**Status:** Complete.

**Purpose:** RFC-006 R-009 / R-011 public devnet pass for the Solana
BFT-consensus finality path.

**Transaction**

- Network: Solana devnet
- Genesis hash: `EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG`
- Source: `5K4KLyoqBwS58sWjuJvhnuu794us4XGjTcXSTDUjwmJ2`
- Target: `AbZmoJqS2KrxVsEo3wi5CMHjpJCBrdovgcu1xH9KGpHE`
- Value: `1000000000` lamports
- Signature:
  `u4vKrDnkhjqTbAWJJ5pFKkjsPJZ8YLYVxDhrC3piSCKui71Tnr6vcT3LstFGy1huFD5fJFQUqgqF3xb4JLkx99v`
- Inclusion slot: `469448447`
- Status: `Ok`
- Commitment: `finalized`

**Extraction Artifact**

- Artifact file:
  `docs/test/evidence/2026-06-14-sol-devnet-lifecycle.json`
- Arrow code: `1:SOL:FN:PR:1:A`
- Observed genesis hash: `EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG`
- Observer head: `469448447`
- Observed at: `2026-06-14T20:41:57.982501+00:00`
- Extraction artifact hash:
  `sha256:1285a69ba88562200e14341b6341ccf351340c09686333735759744e70e8de15`

**Commands**

```sh
solana confirm -v \
  u4vKrDnkhjqTbAWJJ5pFKkjsPJZ8YLYVxDhrC3piSCKui71Tnr6vcT3LstFGy1huFD5fJFQUqgqF3xb4JLkx99v \
  --url https://api.devnet.solana.com
```

```sh
cargo run -p arrow-registry-test -- lifecycle run \
  --origin SOL \
  --network devnet \
  --tx u4vKrDnkhjqTbAWJJ5pFKkjsPJZ8YLYVxDhrC3piSCKui71Tnr6vcT3LstFGy1huFD5fJFQUqgqF3xb4JLkx99v \
  --rpc https://api.devnet.solana.com \
  --out /tmp/arrow-sol-devnet-lifecycle.json
```

```sh
cargo run -p arrow-registry-test -- artifact hash \
  /tmp/arrow-sol-devnet-lifecycle.json
```

**Harness Note**

This run exposed the same HTTPS compatibility defect previously found in the
EVM observer: `arrow-origin-sol` used a plain Hyper `HttpConnector`, so the
documented HTTPS Solana devnet RPC endpoint failed with
`invalid URL, scheme is not http`. The observer was updated to use `reqwest`
with rustls for read-only JSON-RPC calls.

**Project Meaning**

This proves the Solana devnet path can observe a finalized native SOL System
Program transfer, bind the artifact to the observed devnet genesis hash, map
Solana commitment finality into the expected Arrow code, and produce a
reproducible R-011 extraction artifact hash.

## 2026-06-16 — Bitcoin Mainnet

**Status:** Complete.

**Purpose:** RFC-006 R-009 / R-011 public mainnet pass for the Bitcoin
probabilistic-finality path. Completes the five-chain mainnet lifecycle set
(ETH, Base, Arbitrum, Solana, Bitcoin).

**Transaction**

- Network: Bitcoin Mainnet
- Chain identity: `main`
- Source: external funded wallet
- Target: `bc1qaqssn3xe52h8h5fsg7dn8qn6pdtsrktarefp0x`
- Value: `4348` sats (non-dust)
- Transaction id:
  `6c484f6a4ff467a72158f86cb29b08c9cf30d5e4f9f39ced55d940fa9aa72870`
- Output index: `0`
- Outpoint:
  `6c484f6a4ff467a72158f86cb29b08c9cf30d5e4f9f39ced55d940fa9aa72870:0`
- Inclusion block: `953827`

**Extraction Artifact**

- Artifact file:
  `docs/test/evidence/arrow-btc-mainnet-lifecycle.json`
- Arrow code: `1:BTC:FN:PR:1:A`
- Observed network: `main`
- Confirmations at observation: `58`
- Observed at: `2026-06-16T04:10:00.195756+00:00`
- Extraction artifact hash:
  `sha256:5e4e506ffca506b44ddf342c883d10ef845b89f61e41bd184e02048acef28b4e`

**Observer Configuration**

The recipient address is not owned by the key-bearing `arrow-mainnet` wallet,
and the mainnet node is pruned (no `txindex`, which is incompatible with
pruning). A dedicated keyless watch-only wallet `arrow-btc-watch` was created and
the recipient imported watch-only, so the observer's `getrawtransaction` →
`gettransaction` fallback resolves the confirmed transaction. The RPC URL
therefore carries the `/wallet/arrow-btc-watch` path.

**Commands**

```sh
ARROW_BTC_RPC_USER=__cookie__ \
ARROW_BTC_RPC_PASS="$(cut -d: -f2 "$HOME/Library/Application Support/Bitcoin-mainnet-pruned/.cookie")" \
cargo run -p arrow-registry-test -- lifecycle run \
  --origin BTC \
  --network mainnet \
  --tx 6c484f6a4ff467a72158f86cb29b08c9cf30d5e4f9f39ced55d940fa9aa72870 \
  --rpc http://127.0.0.1:28335/wallet/arrow-btc-watch \
  --btc-vout 0 \
  --out docs/test/evidence/arrow-btc-mainnet-lifecycle.json
```

```sh
cargo run -p arrow-registry-test -- artifact hash \
  docs/test/evidence/arrow-btc-mainnet-lifecycle.json
```

**Superseded Attempt**

An earlier mainnet transfer
(`a58d4f8c12cf11b492b78b088940b749af795a774a28eb94aa421dc8aad56e46`, block
`953754`, `1768` sats) was recorded before the node finished syncing and was
below the runbook's preferred non-dust size. It was never reduced to an
extraction artifact and is retained here only as provenance; the artifact above
supersedes it.

**Project Meaning**

This proves the Bitcoin mainnet path can observe a real non-dust UTXO transfer
through a keyless watch-only observer wallet, bind the artifact to the observed
`main` network identity, wait through the `>= 6` reorg-safety threshold, and emit
the expected final Arrow code. With this entry the mainnet lifecycle set is
complete (5/5); the registry `lifecycle_test` blocks are backfilled and the
mainnet registry snapshot can now be hashed and timestamped.

## 2026-06-14 — BTC Signet

**Status:** Complete.

**Purpose:** RFC-006 R-009 / R-011 public testnet pass for the Bitcoin UTXO
probabilistic-finality path.

**Transaction**

- Network: Bitcoin Signet
- Wallet: `arrow-signet`
- Recipient: `tb1qned6kfc6fjh4y0a3wlerwey8y0ne8t2avezdet`
- Value: `1000` sats
- Transaction id:
  `3b373919b9654e0a24f5a14c2eb7daf1cf795889371e0fe6ba95c61c758a3c10`
- Output index: `1`
- Outpoint:
  `3b373919b9654e0a24f5a14c2eb7daf1cf795889371e0fe6ba95c61c758a3c10:1`
- Inclusion block: `308817`

**Extraction Artifact**

- Artifact file:
  `docs/test/evidence/2026-06-14-btc-signet-lifecycle.json`
- Arrow code: `1:BTC:FN:PR:1:A`
- Observed network: `signet`
- Confirmations at observation: `36`
- Observed at: `2026-06-14T19:41:03.988469+00:00`
- Extraction artifact hash:
  `sha256:bd8212af659d0140ce207a76ca441c9b120357ceafccf1b57b5dcc9fd078ac32`

**Prior Observation**

The same outpoint was first observed at `1` confirmation and retained as
`docs/test/evidence/2026-06-14-btc-signet-lifecycle-pending.json`. That pending
artifact produced Arrow code `1:BTC:PD:HL:0:B` and extraction artifact hash
`sha256:e6ada40e39cec55af743a7c070de03915e1f712a541d565e31c0441bb6d7b82f`.

**Commands**

```sh
bitcoin-cli -signet -rpcwallet=arrow-signet sendtoaddress \
  "tb1qned6kfc6fjh4y0a3wlerwey8y0ne8t2avezdet" \
  0.00001000
```

```sh
ARROW_BTC_RPC_USER=__cookie__ \
ARROW_BTC_RPC_PASS="$(cut -d: -f2 "$HOME/Library/Application Support/Bitcoin/signet/.cookie")" \
cargo run -p arrow-registry-test -- lifecycle run \
  --origin BTC \
  --network signet \
  --tx 3b373919b9654e0a24f5a14c2eb7daf1cf795889371e0fe6ba95c61c758a3c10 \
  --rpc http://127.0.0.1:38332 \
  --btc-vout 1 \
  --out /tmp/arrow-btc-signet-lifecycle.json
```

```sh
cargo run -p arrow-registry-test -- artifact hash \
  /tmp/arrow-btc-signet-lifecycle.json
```

**Project Meaning**

This proves the Bitcoin Signet path can observe a real wallet-created UTXO,
bind the artifact to the observed Signet network identity, preserve the
specific lifecycle outpoint, wait through the Bitcoin reorg-safety threshold,
and emit the expected final Arrow code after `>= 6` confirmations.
