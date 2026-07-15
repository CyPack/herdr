# Canonical Reference Project Intelligence Pipeline

## Contents

1. Purpose and trigger
2. Non-negotiable rules
3. Input and run identity
4. Agent roles
5. Fifteen phases
6. Parallel execution graph
7. Artifact contract
8. Evidence and claim contract
9. Status, retry and stop rules
10. Completion gate

## Purpose and trigger

Use this pipeline whenever the user supplies a repository, owner profile,
article, demo or example application to add to the Ratatui design-intelligence
corpus. It turns a URL into an immutable, indexed, evidence-backed and
classified reference, then maps it against Herdr's current architecture to
produce an implementation-ready behavior/data/layout/component/test blueprint.

This is a research/corpus/integration-intelligence pipeline. It does not authorize Herdr product-code
changes, dependency adoption, copying third-party source or operating the
stable Herdr process/socket.

Canonical machine definition:
`../assets/reference-project-pipeline-v2.json`.

New-run state template:
`../assets/reference-project-run-template.json`.

Deep P8–P14 execution contract:
`herdr-integration-analysis-pipeline.md`.

## Non-negotiable rules

1. Pin an immutable source revision before making code claims.
2. Use Codebase Memory MCP before local code exploration.
3. Stop graph-dependent work when MCP is unavailable; never fabricate graph
   data, qualified names, node counts or call paths.
4. Separate verified behavior from README claims and inference.
5. Record negative evidence: absent license, absent fallback, absent tests and
   absent claimed capabilities matter.
6. Classify results into direct component/API, adaptable architecture or
   algorithm, full application/scenario, and caution/negative buckets.
7. Keep every claim bound to source ID, commit, exact evidence and confidence.
8. Run source-owned verification without executing the project against user
   repositories or external production systems.
9. Modify only corpus/skill/research areas unless the user separately requests
   implementation.
10. Finish with registry, map, JSON, graph-freshness and worktree-isolation
    gates.

## Input and run identity

Required input:

- `source_url`
- `requested_pool`, normally `ratatui`
- `intent`, normally full project intelligence

Optional input:

- user priority notes;
- expected patterns or surfaces;
- preferred source ID;
- explicit exclusions;
- permitted source verification commands.

The orchestrator must derive:

- canonical normalized URL;
- deduplicated source ID;
- default branch;
- immutable commit SHA;
- fetched timestamp;
- local clone path;
- graph project key;
- run ID: `<source-id>-<short-sha>-<timestamp>`.

Rerunning the same source ID and commit updates the existing evidence package;
it must not create a duplicate registry row.

## Agent roles

| Role | Ownership | Forbidden |
|---|---|---|
| Orchestrator | run state, dependencies, merge gates and final status | inventing missing worker evidence |
| Intake agent | URL identity, commit, license and clone boundary | code capability claims |
| Graph cartographer | index, ontology, architecture and exact symbols | README-only architecture claims |
| Runtime analyst | entry points, services, data flow, persistence and failures | UI inference without source |
| UI analyst | pages, components, layout, interaction, visual states and motion | treating non-Ratatui code as direct Ratatui code |
| Source verifier | dependency install, tests, typecheck, lint/build and clean status | operating the source against user production data |
| Classifier/extractor | taxonomy, reuse buckets, adaptation and cautions | copying code before license review |
| Corpus publisher | system map, catalogs, registry and skill references | changing Herdr product code |
| Evidence auditor | cross-checks, validators, freshness, V score and isolation | lowering gates to obtain green status |
| Herdr cartographer | current target layers, authority seams, extension points and docs drift | mixing desired architecture into current-state claims |
| Behavior gap analyst | source behaviors, Herdr coverage and acceptance contracts | treating visual similarity as behavior coverage |
| Data authority analyst | provenance, authority, transport, freshness, retention and failure | naming shared facts after UI widgets |
| Layout fidelity analyst | deterministic captures, cell geometry, tokens, fallbacks and diffs | hiding differences behind a fuzzy score |
| Integration architect | macro/micro/overlay ownership and lifecycle blueprint | render mutation or private-client runtime authority |
| Integration test designer | TDD slices, local/cross/capability/performance gates | marking future tests passed without execution |

One agent may perform multiple roles in a small run, but ownership and artifact
boundaries remain the same. Independent roles may run concurrently only where
the dependency graph allows it.

## Fifteen phases

### P0 — Intake and immutable source freeze

Jobs:

1. Normalize the URL and consolidate duplicates.
2. Reserve or recover the canonical source ID.
3. Fetch default branch, full commit SHA, commit date and repository identity.
4. Inventory `LICENSE`, `COPYING`, `NOTICE` and manifest license fields.
5. Clone/fetch into the reference pool and checkout the pinned commit.

Gate:

- canonical URL and 40-character SHA exist;
- clone `HEAD` equals the recorded SHA;
- license result is explicit, including `not-detected`;
- clone begins clean.

Artifacts: `identity.json`, initial `run.json` and source-directory record.

### P1 — Codebase Memory index and ontology

Jobs:

1. Full-index the pinned repository.
2. Wait for `ready`; record node/edge counts.
3. Run a source-specific freshness query and resolve an exact symbol.
4. Capture architecture, languages, entry points, packages and graph clusters.

Gate:

- MCP tool call succeeded;
- index is `ready` with positive node and edge counts;
- freshness query resolves a symbol at the pinned source;
- ontology defines in-scope, out-of-scope, actors and system type.

Artifacts: `index.json` and initial `.cartography/SYSTEM-MAP.json`.

MCP unavailable means `blocked`, not fallback-generated graph data.

### P2 — Architecture, runtime and data-flow mapping

Jobs:

1. Map CLI/entry points and configuration resolution.
2. Map modules, services and runtime ownership.
3. Trace primary call paths with graph tools.
4. Trace raw source → normalization → state → consumer data flows.
5. Map persistence, caches, retries, degraded paths and complexity hotspots.

Gate:

- every material dependency edge references a claim;
- every flow identifies source, transformations, destination and fallback;
- runtime facts and presentation facts are separated;
- high-complexity junctions and failure paths are recorded.

Artifacts: architecture section of `ANALYSIS.md`, `data-flow.json`, verified
system-map claims and evidence records.

### P3 — Surface, layout and behavior intelligence

Jobs:

1. Inventory application surfaces, pages and project form.
2. Inventory reusable component and behavior families.
3. Map layout geometry, breakpoints, scroll and hit regions.
4. Map keyboard, mouse, focus, modal and lifecycle behavior.
5. Map theme tokens, borders, highlights, terminal capabilities and fallbacks.
6. Map animation truth and negative capabilities.

Gate:

- every surface has data source, state owner, layout role and input model;
- responsive and degenerate behavior is explicit;
- claimed motion distinguishes actual animation from periodic repaint;
- absent tiling, ASCII, color-depth, reduced-motion or other capabilities are
  recorded rather than inferred.

Artifacts: `component-catalog.json`, surface/behavior sections of
`ANALYSIS.md` and evidence records.

### P4 — Source-owned verification

Jobs:

1. Inspect declared verification commands and install dependencies safely.
2. Run the canonical test suite.
3. Run available typecheck, lint and formatting checks.
4. Build or compile to a temporary/ignored output and confirm clone cleanliness.

Gate:

- every attempted command records command, exit code and summarized output;
- failures remain failures and are not described as source capability;
- build/test actions do not leave source changes;
- unexecuted gates are labeled `not_run` with a reason.

Artifact: `verification.json`.

### P5 — Classification and Herdr extraction

Jobs:

1. Classify project form and technology compatibility.
2. Create taxonomy-bound evidence claims.
3. Separate direct, adaptable, application and caution buckets.
4. Define Herdr ownership, purity and runtime/client adaptations.
5. Apply license/reuse constraints and prioritize extraction candidates.

Gate:

- claim categories exist in the pool taxonomy;
- confidence matches evidence strength;
- direct compatibility is never inferred from appearance;
- every candidate has reuse mode, adaptation cost and caution boundary.

Artifact: `classification.json` ready to publish under the source ID.

### P6 — Corpus publication

Jobs:

1. Finalize evidence-backed `SYSTEM-MAP.json`.
2. Publish `ANALYSIS.md`.
3. Publish `component-catalog.json`.
4. Publish `data-flow.json`.
5. Publish the source classification.
6. Update the canonical registry and skill source/reference routing.

Gate:

- registry source ID is contiguous and URL-deduplicated;
- registry identity, index SHA and classification metadata agree;
- all local paths and source URLs resolve;
- source catalog declares the default reuse mode.

Artifacts: complete discovery directory, classification file, registry update
and skill reference entry.

### P7 — Source audit, closure and handoff

Jobs:

1. Validate registry, taxonomy and all JSON artifacts.
2. Validate system-map schema, confidence threshold and V score.
3. Recheck graph readiness and freshness.
4. Check source clone and Herdr product-code isolation.
5. Produce a concise result/handoff with verified facts, cautions and next
   extraction priorities.

Gate:

- all validators pass;
- `V=0` for a complete run, otherwise status is `partial` or `blocked` with
  explicit unresolved claims;
- source clone is pinned and clean;
- no unintended product-code diff is attributed to the pipeline;
- final response never claims more than recorded evidence.

Artifacts: final `run.json`, validation records and handoff summary.

### P8 — Herdr current-state cartography

Freeze the target revision and dirty boundary, verify the Herdr graph, and map
runtime/server/API/client/state/action/view/render/input/persistence/test layers,
current extension seams and documentation drift.

Artifact: `herdr-system-map.json`.

### P9 — Behavior gap and coverage analysis

Enumerate every observable source behavior and compare it with current Herdr.
Classify coverage as exact, partial, absent, conflicting or out of scope. Bind
accepted gaps to owner, dependencies and named acceptance tests.

Artifact: `behavior-gap-matrix.json`.

### P10 — Data authority, contract and transport map

Trace every desired datum from source producer through validation,
normalization, authority, transport, client projection and component consumer.
Classify it as server, API transport, client presentation, absent or out of
scope; define freshness, failure, privacy and retention.

Artifact: `data-authority-map.json`.

### P11 — Cell-level layout fidelity and responsive optimization

Capture deterministic source fixtures at canonical and breakpoint-adjacent
viewports. Specify region constraints, cells/hit regions, semantic tokens,
fallbacks, responsive degradation, fixed-time motion, goldens and measured
performance baselines. The gate is zero undeclared differences.

Artifact: `layout-fidelity-spec.json`.

### P12 — Component, input, focus and overlay integration blueprint

Decompose justified macro, micro and overlay components. Bind each to data,
state, actions, geometry, renderer, input precedence, focus lifecycle, failure
states, responsive behavior, reuse/license mode and tests. Prove pure render and
runtime/client boundary compliance.

Artifact: `component-integration-map.json`.

### P13 — Implementation slicing, TDD and cross-test plan

Order work by authority dependencies. Every slice names protected behavior,
characterization and RED tests, minimal GREEN boundary, local tests,
API/event/multi-client/reconnect/capability gates, benchmark/soak checks,
rollout evidence and rollback.

Artifacts: `implementation-plan.json` and `integration-verification.json`.

### P14 — Integration-readiness audit

Validate schemas and references; audit requirement-to-behavior-to-data-to-
layout-to-component-to-test-to-slice traceability; recheck target graph,
fidelity, authority, purity, license, isolation, failure coverage and
boundedness. `passed` requires integration V=0 and no product-code change.

Artifacts: final run state, integration validation records and readiness
handoff.

## Parallel execution graph

```text
P0 source freeze
├── P1 source graph ──┬── P2 architecture/runtime ──┐
│                     └── P3 UI/behavior ───────────┤
└── P4 source verification ─────────────────────────┤
                                                    v
                                            P5 classification
                                                    |
                                            P6 publication
                                             /             \
                                      P7 source audit   P8 Herdr map
                                                              |
                                                       P9 behavior gap
                                                        /            \
                                              P10 data map       P11 fidelity
                                                        \            /
                                                    P12 components
                                                           |
                                                    P13 tests/slices
                                                           |
                                      P7 ---------------- P14 audit
```

P2 and P3 may run in parallel after P1. P4 may run after P0. P5 is their merge
gate. P7 and P8 may run after P6. P10 and P11 may run in parallel after P9.
P12 merges behavior, data and fidelity; P14 is the final gate.

## Artifact contract

Every run directory contains:

```text
run.json
identity.json
index.json
evidence.jsonl
verification.json
ANALYSIS.md
component-catalog.json
data-flow.json
.cartography/SYSTEM-MAP.json
herdr-system-map.json
behavior-gap-matrix.json
data-authority-map.json
layout-fidelity-spec.json
component-integration-map.json
implementation-plan.json
integration-verification.json
```

The published pool additionally contains:

```text
classifications/<source-id>.json
sources.json entry
skill source-catalog/reference entry
```

Do not duplicate the same fact across artifacts without a stable claim or
source ID connecting them.

## Evidence and claim contract

Every claim records:

- stable claim ID;
- taxonomy category;
- concise claim text;
- evidence type;
- pinned source path or URL;
- exact qualified name/symbol when code-backed;
- confidence from 0 to 1;
- reuse mode and Herdr applicability when relevant;
- status: open, verified, contradicted or rejected.

Evidence ranking:

1. executed test or build evidence;
2. exact source and graph trace;
3. repository-owned documentation;
4. inference explicitly labeled as inference.

README-only claims never become verified code claims without corroboration.

## Status, retry and stop rules

Run and phase statuses:

- `queued`
- `running`
- `passed`
- `partial`
- `blocked`
- `failed`
- `skipped`

Retry only transient network, rate-limit or MCP readiness failures. Preserve
every attempt. Do not repeat the same failed strategy without new data.

Stop immediately when:

- the source identity cannot be pinned;
- MCP is unavailable for graph-dependent phases;
- the source requests credentials or unsafe production operation;
- user-owned or Herdr product changes would be overwritten;
- license/reuse status is being used to justify source copying without proof.

Unknown errors follow the repository troubleshooting protocol and remain
visible in `run.json`.

## Completion gate

A run is `passed` only when:

- source identity and commit are immutable;
- graph index is ready and fresh;
- architecture and data flows are evidence-backed;
- surfaces, layout, components, behavior and negative evidence are mapped;
- source verification results are truthful;
- classification and reuse boundaries are complete;
- registry, map and JSON validators pass;
- source and integration `V=0`;
- source and Herdr isolation checks pass.
- the current Herdr graph and layer map are fresh;
- accepted behavior has complete data/layout/component/test traceability;
- every datum has an explicit authority or approved absent-data action;
- terminal fixtures have zero undeclared cell-level differences;
- every implementation slice begins with a named RED test and includes local
  and cross-layer failure gates.

Otherwise return `partial`, `blocked` or `failed`; never use optimistic
completion language.
