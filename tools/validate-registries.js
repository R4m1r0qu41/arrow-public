const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const REPO_ROOT = path.join(__dirname, '..');
const ROOT = path.join(REPO_ROOT, 'registries');
let failed = false;

function error(message) {
  failed = true;
  console.error(message);
}

function readJson(file) {
  return JSON.parse(fs.readFileSync(path.join(ROOT, file), 'utf8'));
}

function requireFields(file, entry, fields, id) {
  for (const field of fields) {
    if (entry[field] === undefined || entry[field] === null || entry[field] === '') {
      error(`${file}: ${id} missing required field ${field}`);
    }
  }
}

function checkAddedVersion(file, entry, id) {
  if (entry.added_version === undefined || entry.added_version === null || entry.added_version === '') {
    return;
  }
  if (!/^[0-9]+\.[0-9]+(?:\.[0-9]+)?$/.test(String(entry.added_version || ''))) {
    error(`${file}: ${id} invalid added_version ${entry.added_version}`);
  }
}

function validateUnique(file, key) {
  const json = readJson(file);
  const seen = new Set();
  for (const entry of json.entries || []) {
    const id = entry[key];
    if (seen.has(id)) error(`${file}: duplicate ${key} ${id}`);
    seen.add(id);
    checkAddedVersion(file, entry, id);
  }
}

function validateOrigins() {
  const json = readJson('origins.json');
  const seen = new Set();
  for (const entry of json.entries || []) {
    requireFields('origins.json', entry, ['code', 'verification_regime'], entry.code);
    if (seen.has(entry.code)) error(`origins.json: duplicate code ${entry.code}`);
    seen.add(entry.code);
    checkAddedVersion('origins.json', entry, entry.code);
  }
}

// Parse an ISO-8601-style duration string (e.g. "30d", "90d") and return days.
// Accepts "Nd" (N days) only — strict subset for registry cadence values.
function parseDurationDays(s) {
  if (typeof s !== 'string') return null;
  const m = s.match(/^(\d+)d$/);
  if (!m) return null;
  return parseInt(m[1], 10);
}

function validateChainRegistryEntries(file, entries, isTestnet) {
  const seen = new Set();
  for (const entry of entries || []) {
    const id = entry.code;
    requireFields(file, entry, ['code', 'name', 'registry_version', 'entry_version', 'status', 'protocol_required', 'provenance', 'anchoring'], id);
    if (entry.code && !/^[A-Z]{3,5}$/.test(entry.code)) {
      error(`${file}: ${id} code must be 3-5 uppercase ASCII letters`);
    }

    if (entry.protocol_required) {
      requireFields(file, entry.protocol_required, ['finality_model', 'chain_trust_flag', 'layer'], `${id}.protocol_required`);
      const flag = entry.protocol_required.chain_trust_flag;
      if (flag !== 0 && flag !== 1) {
        error(`${file}: ${id} protocol_required.chain_trust_flag must be 0 or 1, not ${flag}`);
      }
      if (typeof flag === 'boolean') {
        error(`${file}: ${id} protocol_required.chain_trust_flag must not be boolean`);
      }
      if (entry.protocol_required.layer !== 1 && entry.protocol_required.layer !== 2) {
        error(`${file}: ${id} protocol_required.layer must be 1 or 2`);
      }
      if (entry.protocol_required.layer === 2 && !entry.protocol_required.settlement_chain) {
        error(`${file}: ${id} L2 entry missing protocol_required.settlement_chain`);
      }

      // §4.10 — testnet entries must have chain_trust_flag 0, never 1.
      if (isTestnet && flag === 1) {
        error(`${file}: ${id} testnet entry must not have chain_trust_flag=1`);
      }
    }

    if (entry.provenance) {
      requireFields(file, entry.provenance, ['source_type', 'source_ref', 'source_version', 'verified_by', 'verified_at', 'review_cadence', 'next_review_due'], `${id}.provenance`);

      // §4.10 — R-010 state_test_cadence is required; must be ≤ 30 days.
      const cadence = entry.provenance.state_test_cadence;
      if (cadence === undefined || cadence === null || cadence === '') {
        error(`${file}: ${id} provenance.state_test_cadence is required (R-010)`);
      } else {
        const days = parseDurationDays(cadence);
        if (days === null) {
          error(`${file}: ${id} provenance.state_test_cadence must be in Nd format (e.g. "30d"), got: ${cadence}`);
        } else if (days > 30) {
          error(`${file}: ${id} provenance.state_test_cadence exceeds 30-day R-010 ceiling: ${cadence}`);
        }
      }
    }

    // §4.10 — lifecycle_test block shape validation (when populated).
    if (entry.lifecycle_test !== null && entry.lifecycle_test !== undefined) {
      const lt = entry.lifecycle_test;
      requireFields(file, lt, ['tx_ref', 'performed_by', 'observed_at', 'maps_to_fields', 'extraction_artifact_hash'], `${id}.lifecycle_test`);
      if (lt.tx_hash !== undefined) {
        error(`${file}: ${id} lifecycle_test.tx_hash is obsolete; use lifecycle_test.tx_ref`);
      }
      if (lt.tx_ref !== undefined && lt.tx_ref !== null) {
        if (typeof lt.tx_ref !== 'object' || Array.isArray(lt.tx_ref)) {
          error(`${file}: ${id} lifecycle_test.tx_ref must be an object with type and value`);
        } else {
          requireFields(file, lt.tx_ref, ['type', 'value'], `${id}.lifecycle_test.tx_ref`);
          const allowedTypes = new Set(['evm_tx_hash', 'btc_outpoint', 'sol_signature']);
          if (!allowedTypes.has(lt.tx_ref.type)) {
            error(`${file}: ${id} lifecycle_test.tx_ref.type must be one of evm_tx_hash, btc_outpoint, sol_signature`);
          }
          if (typeof lt.tx_ref.value !== 'string' || lt.tx_ref.value === '') {
            error(`${file}: ${id} lifecycle_test.tx_ref.value must be a non-empty string`);
          }
        }
      }
      if (!Array.isArray(lt.maps_to_fields) || lt.maps_to_fields.length === 0) {
        error(`${file}: ${id} lifecycle_test.maps_to_fields must be a non-empty array`);
      }
      const hash = lt.extraction_artifact_hash;
      if (typeof hash === 'string' && !hash.startsWith('sha256:')) {
        error(`${file}: ${id} lifecycle_test.extraction_artifact_hash must be sha256:-prefixed, got: ${hash}`);
      }
    }

    // CHN-008 (RFC-006 R-009) — Mainnet activation gate.
    // In the production (mainnet) registry, an `active` entry MUST carry a
    // non-null lifecycle_test. The mainnet on-chain lifecycle test is the
    // activation gate: a chain may be `proposed` (staged) with testnet/devnet
    // pre-flight evidence only, but cannot be promoted to `active` ("committed")
    // without a mainnet test. The testnet registry (isTestnet) is exempt — its
    // evidence is optional pre-flight, never the gate.
    if (!isTestnet && entry.status === 'active' &&
        (entry.lifecycle_test === null || entry.lifecycle_test === undefined)) {
      error(`${file}: ${id} CHN-008 — active mainnet entry must carry a non-null lifecycle_test (the mainnet on-chain test is the activation gate; keep status:"proposed" until it is run)`);
    }

    // §4.10 — testnet overlay must not have derivation_eligible:true.
    if (isTestnet && entry.derivation_eligible === true) {
      error(`${file}: ${id} testnet entry must not have derivation_eligible:true`);
    }

    if (seen.has(id)) error(`${file}: duplicate code ${id}`);
    seen.add(id);
  }
}

function validateChainRegistry() {
  const json = readJson('chain_registry.json');
  if (json.network === 'testnet') {
    error('chain_registry.json: mainnet registry must not have network:testnet');
    return;
  }
  validateChainRegistryEntries('chain_registry.json', json.entries, false);
}

function validateTestnetChainRegistry() {
  const filePath = path.join(ROOT, 'chain_registry.testnet.json');
  if (!require('fs').existsSync(filePath)) return; // optional file
  const json = readJson('chain_registry.testnet.json');
  if (json.network !== 'testnet') {
    error('chain_registry.testnet.json: must have top-level network:"testnet"');
  }
  validateChainRegistryEntries('chain_registry.testnet.json', json.entries, true);
}

function validateBridgeRegistry() {
  const json = readJson('bridge_registry.json');
  const seen = new Set();
  for (const entry of json.entries || []) {
    requireFields('bridge_registry.json', entry, ['bridge_id', 'source_chain_id', 'destination_chain_id', 'verification_regime', 'bridge_trust_flag', 'bridge_status', 'added_version'], entry.bridge_id);
    if (entry.bridge_trust_flag !== 0 && entry.bridge_trust_flag !== 1) {
      error(`bridge_registry.json: ${entry.bridge_id} bridge_trust_flag must be 0 or 1, not ${entry.bridge_trust_flag}`);
    }
    if (typeof entry.bridge_trust_flag === 'boolean') {
      error(`bridge_registry.json: ${entry.bridge_id} bridge_trust_flag must not be boolean`);
    }
    if (entry.bridge_trust_flag === 0) {
      if (!entry.bridge_status_reason) error(`bridge_registry.json: ${entry.bridge_id} missing bridge_status_reason`);
      if (!entry.bridge_status_effective_from) error(`bridge_registry.json: ${entry.bridge_id} missing bridge_status_effective_from`);
      if (!entry.restoration_notes) error(`bridge_registry.json: ${entry.bridge_id} missing restoration_notes`);
    }
    if (seen.has(entry.bridge_id)) error(`bridge_registry.json: duplicate bridge_id ${entry.bridge_id}`);
    seen.add(entry.bridge_id);
    checkAddedVersion('bridge_registry.json', entry, entry.bridge_id);
  }
}

function validateConfidenceSubprofiles() {
  const json = readJson('confidence_subprofiles.json');
  const seen = new Set();
  for (const entry of json.entries || []) {
    requireFields('confidence_subprofiles.json', entry, ['subprofile_id', 'category', 'default_confidence_ceiling', 'classification_bound_default', 'default_rejection_behavior', 'added_version'], entry.subprofile_id);
    if (seen.has(entry.subprofile_id)) error(`confidence_subprofiles.json: duplicate subprofile_id ${entry.subprofile_id}`);
    seen.add(entry.subprofile_id);
    checkAddedVersion('confidence_subprofiles.json', entry, entry.subprofile_id);
  }
}

function validateClassificationBindingMethods() {
  const json = readJson('classification_binding_methods.json');
  const seen = new Set();
  for (const entry of json.entries || []) {
    requireFields('classification_binding_methods.json', entry, ['method_id', 'max_confidence_ceiling', 'requires_audit_ref', 'status', 'added_version'], entry.method_id);
    if (seen.has(entry.method_id)) error(`classification_binding_methods.json: duplicate method_id ${entry.method_id}`);
    seen.add(entry.method_id);
    if (!['A', 'B', 'C', 'D', 'E'].includes(entry.max_confidence_ceiling)) {
      error(`classification_binding_methods.json: ${entry.method_id} invalid max_confidence_ceiling ${entry.max_confidence_ceiling}`);
    }
    if (typeof entry.requires_audit_ref !== 'boolean') {
      error(`classification_binding_methods.json: ${entry.method_id} requires_audit_ref must be boolean`);
    }
    checkAddedVersion('classification_binding_methods.json', entry, entry.method_id);
  }
}

function validateChannelBindings() {
  const json = readJson('channel_bindings.json');
  const seen = new Set();
  const requiredCodes = new Set(['http-rest-local', 'https-rest']);
  for (const entry of json.entries || []) {
    const id = entry.code;
    requireFields('channel_bindings.json', entry, [
      'code', 'name', 'description', 'ordered', 'reliable', 'supports_ack', 'requires_ack',
      'supports_signature', 'requires_signature', 'deployment_scope', 'local_only',
      'requires_tls', 'nonce_retention_min', 'spec_reference', 'status', 'added_version'
    ], id);
    if (seen.has(id)) error(`channel_bindings.json: duplicate code ${id}`);
    seen.add(id);
    requiredCodes.delete(id);
    checkAddedVersion('channel_bindings.json', entry, id);

    if (!['local-development', 'production-facing'].includes(entry.deployment_scope)) {
      error(`channel_bindings.json: ${id} invalid deployment_scope ${entry.deployment_scope}`);
    }
    for (const field of ['ordered', 'reliable', 'supports_ack', 'requires_ack', 'supports_signature', 'requires_signature', 'local_only', 'requires_tls']) {
      if (typeof entry[field] !== 'boolean') {
        error(`channel_bindings.json: ${id} ${field} must be boolean`);
      }
    }
    if (!['active', 'inactive'].includes(entry.status)) {
      error(`channel_bindings.json: ${id} invalid status ${entry.status}`);
    }
    if (id === 'http-rest-local') {
      if (!entry.local_only || entry.requires_tls || entry.deployment_scope !== 'local-development') {
        error('channel_bindings.json: http-rest-local must be local-only and must not require TLS');
      }
    }
    if (id === 'https-rest') {
      if (entry.local_only || !entry.requires_tls || entry.deployment_scope !== 'production-facing') {
        error('channel_bindings.json: https-rest must be production-facing and require TLS');
      }
    }
  }
  for (const code of requiredCodes) {
    error(`channel_bindings.json: missing required binding ${code}`);
  }
}

// NAM-001 (v1.6) — Cross-namespace naming integrity.
// No code and no display name may appear in more than one canonical six-field
// namespace (classes, origins, states, actions, confidence). History: State HL
// vs Action HL (collision, removed v1.5); POL Origin vs Polygon (renamed
// v1.5.2); field/class/origin name overloads. CI-fatal.
function validateNamingIntegrity() {
  const namespaces = [
    ['classes.json', 'classes'],
    ['origins.json', 'origins'],
    ['states.json', 'states'],
    ['actions.json', 'actions'],
    ['confidence.json', 'confidence'],
  ];
  const seen = new Map(); // normalized value -> [{ns, kind, raw}]
  for (const [file, ns] of namespaces) {
    const json = readJson(file);
    for (const entry of json.entries || []) {
      for (const kind of ['code', 'name']) {
        const raw = entry[kind];
        if (raw === undefined || raw === null) continue;
        const norm = String(raw).toLowerCase();
        if (!seen.has(norm)) seen.set(norm, []);
        seen.get(norm).push({ ns, kind, raw });
      }
    }
  }
  for (const [norm, uses] of seen) {
    const distinctNs = new Set(uses.map((u) => u.ns));
    if (distinctNs.size > 1) {
      error(
        `NAM-001: "${norm}" appears in multiple canonical namespaces: ` +
          uses.map((u) => `${u.ns}.${u.kind}="${u.raw}"`).join(', ')
      );
    }
  }
}

// NAM-002 (RFC-010 Part 3) — Reserved-token registry, cross-artifact naming
// integrity. Extends NAM-001 beyond the six-field namespaces: every rule-ID
// prefix in the pinned rule surface must be a registered rule_family; no
// reserved token may collide with a six-field code except via a declared
// alias/prohibited entry; six-field codes barred as prefixes stay barred.
// CI-fatal, like NAM-001.
function validateReservedTokens() {
  const reserved = readJson('reserved_tokens.json');
  const entries = reserved.entries || [];

  // (3) internal uniqueness (case-insensitive)
  const seen = new Map();
  for (const e of entries) {
    const norm = String(e.token).toLowerCase();
    if (seen.has(norm)) error(`NAM-002: reserved token "${e.token}" registered more than once`);
    seen.set(norm, e);
  }

  // (2)+(4) collisions with six-field namespaces: only six_field_alias /
  // prohibited_prefix entries (or entries carrying an alias note) may match
  // a canonical code; conversely every canonical code that IS reserved must
  // be one of those kinds.
  const sixFieldCodes = new Set();
  for (const file of ['classes.json', 'origins.json', 'states.json', 'actions.json', 'confidence.json']) {
    for (const entry of readJson(file).entries || []) {
      sixFieldCodes.add(String(entry.code).toLowerCase());
    }
  }
  for (const e of entries) {
    const norm = String(e.token).toLowerCase();
    const collides = sixFieldCodes.has(norm);
    const declared = e.kind === 'six_field_alias' || e.kind === 'prohibited_prefix' || (e.note || '').includes('alias');
    if (collides && !declared) {
      error(`NAM-002: reserved token "${e.token}" collides with a six-field code without a declared alias/prohibited kind`);
    }
  }

  // (1) every rule ID in the rule surface uses a registered rule_family prefix
  const families = new Set(
    entries.filter((e) => e.kind === 'rule_family').map((e) => String(e.token).toLowerCase())
  );
  const surfacePath = path.join(__dirname, '..', 'exports', 'rule-surface.v1.6.0.json');
  const surface = JSON.parse(fs.readFileSync(surfacePath, 'utf8'));
  for (const pin of surface.rule_pins || []) {
    const prefix = String(pin.rule_id).split('-')[0].toLowerCase();
    if (!families.has(prefix)) {
      error(`NAM-002: rule surface pins "${pin.rule_id}" but prefix "${prefix.toUpperCase()}" is not a registered rule_family in reserved_tokens.json`);
    }
  }
}


function sha256File(filePath) {
  return crypto.createHash('sha256').update(fs.readFileSync(filePath)).digest('hex');
}

function readRepoJson(file) {
  return JSON.parse(fs.readFileSync(path.join(REPO_ROOT, file), 'utf8'));
}

// Validate that `p` is a repository-relative path that exists on disk.
// Emits an error and returns null on failure; returns the absolute path on success.
function resolveRepoRelativePath(rel, label, p) {
  if (path.isAbsolute(p) || p.includes('..')) {
    error(`${rel}: ${label} must be repository-relative and must not contain ..`);
    return null;
  }
  const abs = path.join(REPO_ROOT, p);
  if (!fs.existsSync(abs)) {
    error(`${rel}: ${label} does not exist: ${p}`);
    return null;
  }
  return abs;
}

function validateNormativeManifest() {
  const rel = 'docs/normative/manifest.json';
  const filePath = path.join(REPO_ROOT, rel);
  if (!fs.existsSync(filePath)) { error(`${rel}: required file not found`); return; }

  const manifest = readRepoJson(rel);
  if (manifest.manifest_id !== 'arrow-active-baseline') {
    error(`${rel}: manifest_id must be arrow-active-baseline`);
  }
  if (manifest.current_baseline !== 'arrow-v1.6.0') {
    error(`${rel}: current_baseline must be arrow-v1.6.0`);
  }
  if (manifest.owning_repository !== 'Arrow') {
    error(`${rel}: owning_repository must be Arrow`);
  }

  const documents = manifest.documents || [];
  if (!Array.isArray(documents) || documents.length === 0) {
    error(`${rel}: documents must be a non-empty array`);
    return;
  }

  const seenIds = new Set();
  const currentDocs = [];
  const requiredFields = [
    'id', 'title', 'status', 'classification', 'owning_repository', 'path',
    'authority', 'arrow_compatibility', 'supersedes', 'schema_dependencies',
    'implementation_dependencies', 'test_references'
  ];

  for (const doc of documents) {
    const id = doc.id || '<missing-id>';
    requireFields(rel, doc, requiredFields, id);
    if (seenIds.has(id)) error(`${rel}: duplicate document id ${id}`);
    seenIds.add(id);

    // Value-level checks run regardless of path validity.
    if (!['current', 'historical', 'draft', 'superseded'].includes(doc.status)) {
      error(`${rel}: ${id} invalid status ${doc.status}`);
    }
    if (!['normative', 'informative', 'historical'].includes(doc.classification)) {
      error(`${rel}: ${id} invalid classification ${doc.classification}`);
    }
    if (doc.owning_repository !== 'Arrow') {
      error(`${rel}: ${id} owning_repository must be Arrow`);
    }
    if (doc.classification === 'normative' && doc.status !== 'current') {
      error(`${rel}: ${id} only current documents may be classified normative in this manifest`);
    }
    if (doc.status === 'current') currentDocs.push(id);

    // Path-dependent checks.
    if (typeof doc.path !== 'string') continue;

    const docPath = resolveRepoRelativePath(rel, `${id} path`, doc.path);
    if (!docPath) continue;

    const sha256Required = doc.status === 'current' || doc.classification === 'normative';
    if (sha256Required && !doc.sha256) {
      error(`${rel}: ${id} sha256 is required for current/normative documents`);
    } else if (doc.sha256 && doc.sha256 !== sha256File(docPath)) {
      error(`${rel}: ${id} sha256 mismatch for ${doc.path}`);
    }

    if (doc.path.includes('-draft') && doc.status === 'current') {
      error(`${rel}: ${id} draft document must not be marked current`);
    }
  }

  for (const id of ['arrow-protocol', 'arrow-validation-rules']) {
    if (!currentDocs.includes(id)) error(`${rel}: missing current document ${id}`);
  }
}

function validateRuleSurface() {
  const rel = 'exports/rule-surface.v1.6.0.json';
  const filePath = path.join(REPO_ROOT, rel);
  if (!fs.existsSync(filePath)) { error(`${rel}: required file not found`); return; }

  const surface = readRepoJson(rel);
  if (surface.artifact !== 'arrow-rule-surface') {
    error(`${rel}: artifact must be arrow-rule-surface`);
  }
  if (surface.baseline !== 'arrow-v1.6.0') {
    error(`${rel}: baseline must be arrow-v1.6.0`);
  }

  const sidecarRel = surface.hash_sidecar || 'exports/rule-surface.v1.6.0.sha256';
  const sidecarPath = resolveRepoRelativePath(rel, 'hash_sidecar', sidecarRel);
  if (sidecarPath) {
    const expected = fs.readFileSync(sidecarPath, 'utf8').trim().split(/\s+/)[0];
    const actual = sha256File(filePath);
    if (expected !== actual) {
      error(`${rel}: hash sidecar mismatch; expected ${expected}, actual ${actual}`);
    }
  }

  for (const srcRel of surface.source_documents || []) {
    resolveRepoRelativePath(rel, 'source_document', srcRel);
  }

  const actions = new Set((readJson('actions.json').entries || []).map((entry) => entry.code));
  for (const entry of surface.registries?.actions?.entries || []) {
    if (!actions.has(entry.code)) error(`${rel}: exported Action code ${entry.code} not in actions.json`);
    if (!entry.semantic_slug) error(`${rel}: exported Action code ${entry.code} missing semantic_slug`);
  }

  const classes = new Set((readJson('classes.json').entries || []).map((entry) => entry.code));
  for (const entry of surface.registries?.classes?.entries || []) {
    if (!classes.has(entry.code)) error(`${rel}: exported Class code ${entry.code} not in classes.json`);
    if (!entry.semantic_slug) error(`${rel}: exported Class code ${entry.code} missing semantic_slug`);
  }

  const reasonCodes = new Set((readJson('reason_codes.json').entries || []).map((entry) => entry.code));
  for (const entry of surface.registries?.reason_codes?.entries || []) {
    if (!reasonCodes.has(entry.code)) error(`${rel}: exported reason code ${entry.code} not in reason_codes.json`);
    if (!entry.semantic_slug) error(`${rel}: exported reason code ${entry.code} missing semantic_slug`);
  }

  const seenRuleIds = new Set();
  for (const pin of surface.rule_pins || []) {
    if (!pin.rule_id || !pin.semantic_slug || !pin.status) {
      error(`${rel}: rule pin must carry rule_id, semantic_slug, and status`);
      if (pin.rule_id) seenRuleIds.add(pin.rule_id);
      continue;
    }
    if (seenRuleIds.has(pin.rule_id)) error(`${rel}: duplicate rule pin ${pin.rule_id}`);
    seenRuleIds.add(pin.rule_id);
  }
}

validateNamingIntegrity();
validateReservedTokens();
validateNormativeManifest();
validateRuleSurface();
validateOrigins();
validateChainRegistry();
validateTestnetChainRegistry();
validateBridgeRegistry();
validateConfidenceSubprofiles();
validateClassificationBindingMethods();
validateChannelBindings();
for (const [file, key] of [
  ['classes.json', 'code'], ['states.json', 'code'], ['actions.json', 'code'], ['confidence.json', 'code'],
  ['verification_regimes.json', 'code'], ['reason_codes.json', 'code'], ['object_types.json', 'code'],
  ['subject_types.json', 'code'], ['proof_types.json', 'code'], ['sources.json', 'source_type'],
  ['finality_basis.json', 'code'], ['control_subtypes.json', 'code']
]) validateUnique(file, key);

if (failed) process.exit(1);
console.log('All registries validated successfully');
process.exit(0);
