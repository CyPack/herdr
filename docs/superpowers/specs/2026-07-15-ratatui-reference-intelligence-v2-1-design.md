# Ratatui Reference Intelligence v2.1 Design

## Status

- Design date: 2026-07-15
- User decision: approved
- Module: `.codex/skills/ratatui-design-intelligence/`
- Module version: `2.1.0`
- Stable pipeline identity: `reference-project-intelligence-v2`
- Scope: research, corpus publication, and Herdr integration intelligence only
- Product-code authorization: false

## Purpose

Turn a pinned reference project into evidence-backed, cross-stack Herdr design
intelligence without confusing source-language implementation details with the
behavior Herdr must reproduce. The pipeline must preserve the existing P0-P14
phase graph while adding a first-class, machine-validatable translation contract
from the source stack to Rust, Ratatui, and Herdr's runtime/client architecture.

This design does not authorize Rust product changes, dependency adoption,
third-party source copying, stable Herdr runtime access, or upstream writes.

## Decisions

1. Keep the existing fifteen phases P0-P14 and their stable pipeline ID.
2. Version the repo-local intelligence module as `2.1.0` independently from
   artifact `schema_version` values.
3. Add `stack-adaptation-map.json` as a required master-run artifact.
4. Build the first translation record during P2 and P5, then finalize its
   Herdr match and semantic diff during P9.
5. Keep Codebase Memory evidence mandatory for graph-dependent claims. An
   unavailable MCP produces `blocked`, never invented graph data.
6. Keep the module in the repository as the single canonical copy. Do not
   maintain a second drifting global copy.
7. Use a separate, explicitly authorized post-analysis integration pipeline
   for actual Herdr product changes. P14 output is its input contract, not
   permission to implement.

## Module Boundary

The module remains under:

```text
.codex/skills/ratatui-design-intelligence/
├── README.md
├── AGENTS.md
├── module.json
├── SKILL.md
├── assets/
│   ├── reference-project-pipeline-v2.json
│   ├── reference-project-run-template.json
│   └── templates/
│       └── stack-adaptation-map-template.json
├── references/
│   └── module-governance.md
├── scripts/
├── tests/
├── evals/
├── lessons/
└── .cartography/
```

`README.md` is intentionally present because the user requires a human-friendly
entry point. `AGENTS.md` is the subtree authority for agent operation. Detailed
procedures remain in references so `SKILL.md` stays a concise router.

## Module Identity and Version Contract

`module.json` is the machine-readable source of truth for:

- module ID and version `2.1.0`;
- stable pipeline ID;
- pipeline schema version;
- compatibility family;
- required manifests, templates, validators, and references;
- required output artifacts;
- canonical status and scope;
- source and post-analysis handoff boundaries.

The module version and artifact schemas are separate:

- patch: documentation or validator correction without contract change;
- minor: additive contract or taxonomy capability compatible with v2 consumers;
- major: phase identity/order, artifact removal/rename, or authority/status
  semantic break;
- artifact schema: versioned independently when a concrete JSON record shape
  changes.

The pipeline filename and `reference-project-intelligence-v2` identity remain
stable to avoid path churn. Consumers read `pipeline_version: "2.1.0"` and the
independent schema fields.

## Stack Adaptation Record

Each record in `stack-adaptation-map.json` must contain:

- `mapping_id`;
- source language, framework, runtime, layer, symbols, and exact evidence;
- source data flow and behavior trigger;
- normalized stack-neutral behavior contract;
- state, concurrency, ownership, error, render, input, and lifecycle semantics;
- target Herdr layer and authority owner;
- Rust/Ratatui target pattern and exact current target symbols, or explicit
  absence;
- translation mode;
- functional match and architectural fit;
- semantic, failure, performance, and ownership diffs;
- terminal capability and responsive fallback implications;
- license/reuse boundary;
- acceptance and cross-test obligations;
- confidence and verification evidence.

Allowed translation modes are:

- `direct_api`: compatible Ratatui/Rust API or component contract;
- `structural_adapter`: compatible architecture with a bounded adapter;
- `behavior_reimplementation`: preserve behavior through a Herdr-native design;
- `reject`: incompatible, unsafe, unsupported, or unjustified.

No record may claim `direct_api` solely because a project is a TUI or uses a
similar visual style.

## Phase Data Flow

```text
P0-P1 immutable source and graph evidence
  -> P2 source stack, runtime, ownership, and data-flow semantics
  -> P3 source UI behavior and terminal semantics
  -> P4 source-owned verification
  -> P5 evidence/reuse classification and initial translation mode
  -> P6-P7 corpus publication and source audit
  -> P8 current Herdr graph and layer baseline
  -> P9 finalized behavior coverage, target symbols, and semantic diff
  -> P10-P12 authority, fidelity, component, input, focus, and lifecycle binding
  -> P13 RED-first local/cross/capability/performance test obligations
  -> P14 traceability, isolation, graph freshness, and readiness audit
```

P2 and P5 may create incomplete translation records only when their missing
target fields are explicitly marked pending. P9 must either finalize those
fields or classify the record as absent, out of scope, or rejected. P14 rejects
silent omissions.

## Human and Agent Interfaces

`README.md` must provide:

- purpose and supported input types;
- current module/pipeline version;
- concise P0-P14 overview;
- required output inventory;
- canonical validator and test commands;
- status meanings and MCP blocking behavior;
- explicit product-code and stable-runtime exclusions;
- handoff to the separate integration pipeline.

`AGENTS.md` must dictate:

- graph-first discovery;
- immutable source and evidence rules;
- phase ordering and dependency gates;
- source/target isolation;
- no fake MCP data;
- no product edits without a separate authorization artifact;
- test-first changes to validators or executable behavior;
- targeted staging and atomic commit rules;
- CyPack fork-only fast-forward publication;
- no upstream, force-push, stable process, or inherited socket access.

## Error and Stop Semantics

- Missing MCP for P1, P2 graph traces, P8, or target symbol verification:
  `blocked`.
- Unresolved claims or incomplete required evidence: `partial`.
- Source verification failure with trustworthy evidence: record the failure;
  do not rewrite the gate to pass.
- Missing or incompatible license: behavior-only/reference classification until
  reuse is legally proven.
- Target architecture conflict: `reject` or explicit design debt; never hide it
  as a visual match.
- Remote branch divergence: stop publication; never force.
- Validator, schema, eval, traceability, or isolation failure: no completion or
  push claim.
- Product-code diff outside the frozen user-owned boundary: P14 fails.

## Test Contract

Before implementation, add failing tests for these behaviors:

| Test point | Expected result | Reason |
|---|---|---|
| TP-V21-MODULE-IDENTITY | Missing `module.json`, wrong `2.1.0`, pipeline-ID drift, or schema/version conflation fails validation | Version and contract identity must be machine-enforced |
| TP-V21-STACK-ARTIFACT | Missing required stack artifact or template mapping fails | Cross-stack behavior cannot remain an implicit prose convention |
| TP-V21-STACK-SCHEMA | Missing source semantics, target binding, translation mode, diffs, tests, license, or evidence fails | Every mapping must be implementation-auditable |
| TP-V21-TRANSLATION-MODE | Only the four canonical modes are accepted | Classification must remain deterministic |
| TP-V21-PHASE-BINDING | P2/P5/P9 outputs and jobs explicitly create, classify, and finalize the map | A required artifact without phase ownership is dead configuration |
| TP-V21-RUN-TEMPLATE | Run state tracks the stack artifact and phase evidence without authorizing product changes | Every run must be resumable and fail closed |
| TP-V21-DOC-CONSISTENCY | README, AGENTS, SKILL, module manifest, pipeline, and governance versions agree | Human and agent instructions must not drift |
| TP-V21-LEGACY-COMPAT | Existing P0-P14 IDs, order, status set, source/integration artifacts, and gates remain valid | v2.1 must not silently replace the established pipeline |
| TP-V21-NEGATIVE-MCP | Graph-dependent completion remains blocked when MCP evidence is absent | Anti-hallucination is a completion invariant |
| TP-V21-ISOLATION | Validator proves product-code authorization false and no expanded diff boundary | Intelligence work must not mutate Herdr product code |

Execution order is RED, GREEN, refactor, focused tests, complete module
validator, skill validation, JSON validation, eval coverage, cartography audit,
and Git diff cleanliness. Existing baseline evidence is 18/18 tests and a
15-phase/97-job validator pass; those numbers are baseline only and must be
replaced by fresh post-change evidence.

## Git and Publication Contract

The implementation plan must preserve atomic concerns:

1. Commit this approved design spec.
2. Commit the current untracked baseline module without cross-stack changes.
3. Commit compile-valid failing contract tests locally.
4. Commit the minimal v2.1 implementation that turns those tests green.
5. Commit human/agent governance documentation separately.
6. Run all proportional module gates before publication.
7. Fetch and prove fast-forward ancestry.
8. Push only `CyPack/herdr` `feat/native-fm`.
9. Fast-forward `CyPack/herdr` `master` only when remote ancestry remains safe.
10. Verify exact remote SHA equality, then refresh Codebase Memory and prove a
    current symbol or indexed module artifact.

Do not create a Herdr release tag for this module version. Do not push upstream.
Do not publish a failing RED tip by itself.

## Post-Analysis Handoff

P14 may emit `integration-verification.json` with `ready`, `partial`, or
`blocked` evidence. The separate sibling `herdr-change-pipeline` module maps the
immutable P14 set through its reference adapter into the same
`change-intent-package.json` accepted from native feature, page, layout,
design, component, architecture, runtime-capability, and composite analysis.
Its delivery graph must require explicit product-change authorization and
create its own PRD, expected/current/diff contracts, test phases,
implementation slices, cross tests, performance budgets, rollback plan, and
fresh completion evidence. Reference research is one input mode; it must never
infer implementation permission or become a prerequisite for native Herdr
changes.

## Acceptance Criteria

The v2.1 module design is implemented only when:

1. `module.json` reports `2.1.0` and all version-bearing files agree.
2. `stack-adaptation-map.json` is required, templated, phase-owned, run-tracked,
   validator-enforced, documented, and eval-covered.
3. Existing P0-P14 identities and prior contracts remain valid.
4. Every new executable contract was observed RED before GREEN.
5. All module tests, validators, skill validation, JSON, eval, cartography,
   traceability, and diff gates pass with fresh evidence.
6. No Herdr product code, stable runtime, inherited socket, upstream ref, or
   unrelated user change was touched.
7. CyPack feature and master publication, if performed, is fast-forward-only
   with exact remote SHA verification.
