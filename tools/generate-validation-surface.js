#!/usr/bin/env node
/**
 * generate-validation-surface.js
 *
 * Generates docs/ui/arrow-validation-surface.html — the Arrow validation
 * surface page ("what passes, what doesn't, and why") — by running the REAL
 * Rust validator (arrow-cli `validate --json`) over:
 *
 *   1. every fixture in tests/fixtures/index.json, and
 *   2. the Tarquin late-payment pilot LEG events (the sibyl-demo Stage A
 *      event plus its two e2e_slc_handoff-derived stages: the invalid
 *      Finality-0 PR twin and the final Stage B event).
 *
 * No validation logic is reimplemented here: every verdict on the page is
 * the arrow-core pipeline's own output, captured at generation time. This
 * script only adds presentation data (registry display names, per-rule
 * lawyer-readable narratives) and cross-repo pipeline status read from the
 * pinned artifacts that actually exist.
 *
 * Usage:
 *   cargo build -p arrow-cli          # once, to produce target/debug/arrow
 *   node tools/generate-validation-surface.js
 *
 * Regenerate whenever fixtures, registries, validator behavior, or the
 * Tarquin pins change. The output file is generated — do not hand-edit it.
 */

"use strict";

const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const REPO = path.resolve(__dirname, "..");
const TARQUIN = path.resolve(REPO, "..", "Tarquin");
const CLI = path.join(REPO, "target", "debug", "arrow");
const TEMPLATE = path.join(__dirname, "validation-surface.template.html");
const OUT = path.join(REPO, "docs", "ui", "arrow-validation-surface.html");

function die(msg) {
  console.error(`error: ${msg}`);
  process.exit(1);
}

if (!fs.existsSync(CLI)) {
  die(`arrow-cli binary not found at ${CLI} — run: cargo build -p arrow-cli`);
}
if (!fs.existsSync(TEMPLATE)) {
  die(`template not found at ${TEMPLATE}`);
}

const readJson = (p) => JSON.parse(fs.readFileSync(p, "utf8"));

// ── Real validator invocation ─────────────────────────────────────────────

function validateFile(p, contextFiles = []) {
  // arrow-cli exits non-zero on FAILED verdicts; both stdout payloads are the
  // ValidationResult JSON, so capture stdout in either case. Context files
  // (LFC-gated fixtures) ride the CLI's --context flag so the verdict here is
  // still the real pipeline's, context included.
  const args = ["validate", p, "--json"];
  for (const cf of contextFiles) args.push("--context", cf);
  try {
    const out = execFileSync(CLI, args, { encoding: "utf8" });
    return JSON.parse(out);
  } catch (e) {
    if (e.stdout) return JSON.parse(e.stdout.toString());
    throw e;
  }
}

function validateEventObject(obj, label) {
  const tmp = path.join(os.tmpdir(), `arrow-surface-${label}.json`);
  fs.writeFileSync(tmp, JSON.stringify(obj, null, 2));
  const result = validateFile(tmp);
  fs.unlinkSync(tmp);
  return result;
}

// ── Registry vocabulary (display names come from the registries, not prose) ─

function entriesOf(p) {
  const d = readJson(path.join(REPO, "registries", p));
  return d.entries || d;
}

function nameMap(file) {
  const m = {};
  for (const e of entriesOf(file)) m[String(e.code)] = e.name;
  return m;
}

const vocab = {
  classes: nameMap("classes.json"),
  origins: nameMap("origins.json"),
  states: nameMap("states.json"),
  actions: nameMap("actions.json"),
  confidence: nameMap("confidence.json"),
  regimes: nameMap("verification_regimes.json"),
};

const originRegime = {};
for (const e of entriesOf("origins.json"))
  originRegime[e.code] = e.verification_regime;

// ── Lawyer-readable narratives (presentation copy, keyed by rule ID). ──────
// Sourced from validation-rules.md and the arrow-cli ack narratives; Lex.Codex
// framing: state the legal obligation the rule protects, in plain language.

const narratives = {
  "SEM-001": "Finality requires a settled state: a final event must sit in FN or RV.",
  "SEM-002": "What is still pending cannot claim to be final.",
  "SEM-003": "What is still timelocked cannot claim to be final.",
  "SEM-004": "What is still conditional cannot claim to be final.",
  "SEM-007": "A transfer must transfer: Class 5 events are limited to trigger or proceed.",
  "SEM-005": "A closure must act like a closure: Class 6 (CLOSE) events must carry the closing Action CL.",
  "SEM-006": "A condition may restrain but not execute: Class 3 events are limited to block, hold, or review.",
  "SEM-009": "Unverified claims cannot authorize movement: Confidence E may not proceed or trigger.",
  "SEM-012": "Nothing closes before it is final in state: Class 6 requires State FN.",
  "SEM-013": "No enforcement before finality: a pending event (Finality 0) may not carry an executing Action.",
  "SEM-014": "A final event stands unless formally reversed or superseded, with the prior event cited.",
  "SEM-015": "The event claims more certainty than its verification method can support: Confidence is capped by the regime's ceiling — and, while a settlement window is still open, by the lower pre-settlement ceiling.",
  "SEM-016": "The source chain's trust has been revoked; everything from it is blocked, without exception.",
  "SEM-017": "Proof that data exists is not proof of what it is: bridge events must bind their classification.",
  "SEM-019": "Value claimed at high confidence must add up: economic consistency must be attested and auditable.",
  "SEM-020": "Only a closure may wear the closing Action: CL outside Class 6 is rejected.",
  "FB-001": "A pending event must say how — and when — it will become final: a registered finality basis, dated where the basis requires it.",
  "CHN-001": "The event references a chain this registry does not recognize.",
  "ORG-003": "The event's origin is not registered; unknown sources are not trusted.",
  "MSG-005": "The bridge's classification method is not registered.",
  "MSG-006": "A legal event must declare what kind of legal act it is (judicial, administrative, supranational, text, or precedent).",
  "LEG-004": "A legal assertion at the lowest confidence may only ask for review, nothing stronger.",
  "LEG-005": "Confidence D from a legal origin is reserved for legal texts and precedent.",
  "LEG-006": "This kind of legal act does not authorize that Action: the subtype's permitted-action table governs.",
  "LEG-007": "Final supranational sanctions block, non-negotiably.",
  "LEG-008": "A legal text or precedent claiming finality is unusual and is flagged for attention.",
  "LEG-009": "A legal event must say where its law comes from: jurisdiction is required.",
  "LEG-010": "The claimed supranational body is not registered or not active.",
  "BRG-000": "The event references a bridge this registry does not recognize.",
  "BRG-001": "This bridge's trust has been revoked; everything crossing it is blocked, without exception.",
  "BRG-003": "Self-certification is not audit: classification above Confidence C needs an audit reference.",
  "BRG-006": "A bridge without a declared confidence method is capped at Confidence D.",
  "ACT-001": "The Action code is not in the Action registry.",
  "ACT-002": "Unknown Action codes are rejected, not guessed at.",
  "LFC-001": "Every event identifier is single-use: a repeated event_id is treated as a replay.",
  "LFC-007": "What is closed stays closed: after closure, only formal reversal or supersession — each citing the closure — may follow.",
  "LFC-008": "Every lifecycle owes the ledger a dated closure: a settled event must say when its lifecycle is due to close.",
  "TRG-001": "A trigger must name what it triggers, on what, and on whose authority — or it is not a trigger.",
  "LFC-003": "An event may only build on events that exist in its lifecycle.",
};

const passNarrative =
  "Conforms: every applicable rule of the v1.6.0 surface passed.";

function narrativeFor(rule) {
  return (
    narratives[rule] ||
    "No narrative on file; see docs/protocol/validation-rules.md."
  );
}

// ── Feed rows: Arrow fixtures ─────────────────────────────────────────────

function decompose(code) {
  const f = code.split(":");
  if (f.length !== 6) return null;
  const [cls, origin, state, action, finality, confidence] = f;
  return {
    class: { code: cls, name: vocab.classes[cls] || "?" },
    origin: {
      code: origin,
      name: vocab.origins[origin] || "?",
      regime: originRegime[origin] || null,
      regime_name: vocab.regimes[originRegime[origin]] || null,
    },
    state: { code: state, name: vocab.states[state] || "?" },
    action: { code: action, name: vocab.actions[action] || "?" },
    finality: {
      code: finality,
      name: finality === "1" ? "Final" : "Pending finality",
    },
    confidence: { code: confidence, name: vocab.confidence[confidence] || "?" },
  };
}

function row(id, sourceLabel, event, result, expected) {
  const errors = result.violations.filter((v) => v.severity === "Error");
  const warnings = result.violations.filter((v) => v.severity !== "Error");
  const deciding = errors.length ? errors[0].rule_id : null;
  const agreement = expected
    ? expected.expected_result === (result.passed ? "pass" : "reject") &&
      expected.expected_violations.every((r) =>
        result.violations.some((v) => v.rule_id === r)
      )
    : null;
  return {
    id,
    source: sourceLabel,
    arrow_code: event.arrow_code,
    fields: decompose(event.arrow_code),
    verdict: result.passed ? "PASS" : "FAIL",
    deciding_rule: deciding,
    reason: deciding ? narrativeFor(deciding) : passNarrative,
    violations: result.violations.map((v) => ({
      rule_id: v.rule_id,
      field: v.field,
      severity: v.severity,
      message: v.message,
      narrative: narrativeFor(v.rule_id),
    })),
    warnings_count: warnings.length,
    forced_action: result.forced_action,
    expected: expected
      ? {
          result: expected.expected_result,
          violations: expected.expected_violations,
          agreement,
        }
      : null,
    description: expected ? expected.description : undefined,
    event,
  };
}

const rows = [];
const fixtureIndex = readJson(
  path.join(REPO, "tests", "fixtures", "index.json")
);
for (const spec of fixtureIndex.fixtures) {
  const p = path.join(REPO, "tests", "fixtures", spec.file);
  const event = readJson(p);
  const ctx = (spec.context_files || []).map((f) =>
    path.join(REPO, "tests", "fixtures", f)
  );
  const result = validateFile(p, ctx);
  const r = row(spec.id, "Arrow fixture", event, result, spec);
  if (ctx.length) r.context_note = `validated with lifecycle context (${(spec.context_files || []).join(", ")})`;
  rows.push(r);
}

// ── Feed rows: Tarquin late-payment pilot LEG events ──────────────────────
// Stage A is the actual exported sibyl-demo event. The negative twin and the
// Stage B final event mirror crates/tarquin-lex-exe/tests/e2e_slc_handoff.rs
// (steps 3-5); their verdicts here are the live validator's, not the test's.

const pilot = [];
const demoDir = path.join(TARQUIN, "exports", "sibyl-demo");
if (fs.existsSync(demoDir)) {
  const stageA = readJson(path.join(demoDir, "leg-event.json"));
  const review = readJson(path.join(demoDir, "review-item.json"));
  const rA = row(
    "PILOT-A",
    "Tarquin pilot",
    stageA,
    validateEventObject(stageA, "pilot-a"),
    null
  );
  rA.consumer_disposition = `${review.disposition} — ${review.reason}`;
  rA.description =
    "Late-payment pilot Stage A: non-final legal hold emitted from the LBRY attestation (e2e_slc_handoff step 3).";
  pilot.push(rA);

  const twin = { ...stageA, arrow_code: "2:LEG:PD:PR:0:C", event_id: stageA.event_id + "-twin" };
  const rT = row(
    "PILOT-TWIN",
    "Tarquin pilot",
    twin,
    validateEventObject(twin, "pilot-twin"),
    null
  );
  rT.description =
    "Late-payment pilot negative twin: the same pending event attempting Action PR at Finality 0 (e2e_slc_handoff step 4 — must reject).";
  pilot.push(rT);

  const stageB = {
    ...stageA,
    arrow_code: "2:LEG:FN:PR:1:C",
    event_id: stageA.event_id + "-final",
    finality_basis: null,
  };
  const rB = row(
    "PILOT-B",
    "Tarquin pilot",
    stageB,
    validateEventObject(stageB, "pilot-b"),
    null
  );
  rB.consumer_disposition =
    "Quarantined — held for explicit human ratification; execution is single-shot after ratification (e2e_pr_path).";
  rB.description =
    "Late-payment pilot Stage B: final proceed after the disputed-001 recomputation (e2e_slc_handoff step 5). Valid at the gate, still human-gated at the consumer.";
  pilot.push(rB);
}
rows.push(...pilot);

// ── Supply-chain pipeline (statuses read from artifacts that exist) ───────

const orlogPin = readJson(path.join(TARQUIN, "imports", "orlog", "PIN.json"))
  .pins[0];
const arrowPin = readJson(path.join(TARQUIN, "imports", "arrow", "PIN.json"))
  .pins[0];
const ruleSurface = readJson(
  path.join(REPO, "exports", "rule-surface.v1.6.0.json")
);
const pinStatus = (ruleId) => {
  const p = ruleSurface.rule_pins.find((r) => r.rule_id === ruleId);
  return p ? p.status : "absent";
};

const pipeline = [
  {
    stage: "Orlog",
    role: "SLC foundry (upstream)",
    items: [
      {
        label: `SLC bundle ${orlogPin.artifact_id} v${orlogPin.version}`,
        status: "verified",
        detail: `Pinned at Orlog ${orlogPin.source_commit}; MC-006 content hash recomputed and matched by Tarquin's pinned_bundle_hash_verifies_and_drift_rejects test (${orlogPin.artifact_hash.slice(0, 23)}…).`,
      },
      {
        label: "Legal-grounding handoff (lex → Lex.Codex pin)",
        status: "pending",
        detail:
          "Orlog PRs #2/#3 (pilot legal verification, MX/NY e-signature grounding) are open; no content-hashed legal-grounding surface is pinned into Tarquin yet. Until then lex_refs are auditable pointers, not Tarquin-validated grounding.",
      },
    ],
  },
  {
    stage: "Tarquin",
    role: "Lex.Codex · Lex.exe · Sibyl (runtime, AI-free)",
    items: [
      {
        label: "Lex.Codex recomputation (late-payment pilot stub)",
        status: "live",
        detail:
          "Deterministic Rust recomputation of the pinned bundle's disputed-001 fixture (e2e_slc_handoff step 2). Scope: this one artifact; general compilation is future work.",
      },
      {
        label: "Sibyl registry + LBRY attestation record",
        status: "live",
        detail:
          "Ed25519-signed structured legal-attestation record (lbry-3719b18145dc0c41) with canonical content hash; fail-closed registry validation.",
      },
      {
        label: "Signed envelope transport + replay protection",
        status: "live",
        detail:
          "DID-keyed Ed25519 envelope (arrow-canonical-json-v0); consumer rejects replays and unsigned traffic.",
      },
      {
        label: "Human ratification gate",
        status: "enforced",
        detail:
          "Execution stays blocked until explicit human ratification and is single-shot afterwards (e2e_pr_path). Signature validity never removes human review.",
      },
      {
        label: "Docverify finding → LBRY → LEG lowering",
        status: "pending",
        detail:
          "The document-verification engine reduces and reports but does not yet lower findings into signed LBRY records or emit LEG events. Gated behind the legal-grounding pin and the frozen-stack acceptance test.",
      },
    ],
  },
  {
    stage: "Arrow",
    role: "Protocol authority (LEG gate + consumers)",
    items: [
      {
        label: "LEG gate validation (v1.6.0 surface)",
        status: "live",
        detail:
          "Every verdict in the feed above is the arrow-core pipeline's own output at generation time — registry-driven, deterministic, no clock input.",
      },
      {
        label: "Consumer disposition (pilot)",
        status: "hold",
        detail:
          "Stage A sits in the review queue as Pending (held pending review, HL); Stage B is quarantined for human ratification. Nothing auto-executes.",
      },
      {
        label: "TR (Trigger) primitive",
        status: "gated",
        detail: `Gated on TRG-001 — status "${pinStatus("TRG-001")}" in the pinned rule surface. No TR primitives are implemented downstream until it is live.`,
      },
      {
        label: "CL post-closure lifecycle enforcement",
        status: "gated",
        detail: `Closure binding itself is live (SEM-005 + SEM-020), but post-closure lifecycle discipline is gated on LFC-007 — status "${pinStatus("LFC-007")}" in the pinned rule surface.`,
      },
      {
        label: "Cross-repo pin recompute in CI (IFP-004)",
        status: "pending",
        detail:
          "Pins verify by test today; the automated three-repository CI recompute (ADR-0003 IFP-004) is not yet in place.",
      },
    ],
  },
];

// ── Assemble page data ────────────────────────────────────────────────────

function gitHead(repo) {
  try {
    return execFileSync("git", ["-C", repo, "rev-parse", "--short", "HEAD"], {
      encoding: "utf8",
    }).trim();
  } catch {
    return "unknown";
  }
}

const data = {
  generated_at: new Date().toISOString(),
  baseline: ruleSurface.baseline,
  arrow_commit: gitHead(REPO),
  tarquin_commit: gitHead(TARQUIN),
  arrow_pin: {
    source_commit: arrowPin.source_commit.slice(0, 7),
    hash: arrowPin.hash.slice(0, 22) + "…",
    pinned_at: arrowPin.pinned_at,
  },
  orlog_pin: {
    source_commit: orlogPin.source_commit,
    hash: orlogPin.artifact_hash.slice(0, 22) + "…",
    pinned_at: orlogPin.pinned_at,
  },
  rows,
  pipeline,
};

const template = fs.readFileSync(TEMPLATE, "utf8");
const BEGIN = "/*DATA:BEGIN*/";
const END = "/*DATA:END*/";
const b = template.indexOf(BEGIN);
const e = template.indexOf(END);
if (b === -1 || e === -1) die("template is missing DATA markers");
const html =
  template.slice(0, b + BEGIN.length) +
  "\nconst DATA = " +
  JSON.stringify(data, null, 1) +
  ";\n" +
  template.slice(e);

fs.mkdirSync(path.dirname(OUT), { recursive: true });
fs.writeFileSync(OUT, html);

const passed = rows.filter((r) => r.verdict === "PASS").length;
const disagreements = rows.filter(
  (r) => r.expected && !r.expected.agreement
).length;
console.log(
  `wrote ${path.relative(REPO, OUT)} — ${rows.length} rows (${passed} pass / ${
    rows.length - passed
  } fail), ${disagreements} expectation disagreement(s)`
);
if (disagreements > 0) {
  console.error(
    "warning: validator output disagrees with fixture expectations — investigate before publishing"
  );
  process.exit(2);
}
