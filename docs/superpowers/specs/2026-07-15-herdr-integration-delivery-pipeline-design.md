# Herdr Change Intelligence and Delivery Pipeline Design

## Status

- Design date: 2026-07-15
- Status: draft for written-spec review
- User decision: generalize the post-analysis module beyond reference-project
  integration to native features, pages, layouts, designs, components,
  architecture changes, and composite analysis
- Owning module: `.codex/skills/herdr-change-pipeline/`
- Initial module version after implementation: `1.0.0`
- Reference/design adapter: `.codex/skills/ratatui-design-intelligence/` v2.1
- Stable analysis pipeline identity: `herdr-change-intelligence-v1`
- Stable delivery pipeline identity: `herdr-change-delivery-v1`
- Analysis and delivery pipeline version: `1.0.0`
- Delivery input: immutable, validated `change-intent-package.json`
- Reference-project P14 input: optional adapter, not the only entry path
- Product-code authorization by default: false

## Purpose

Turn an idea, native feature request, page, layout, design system, component,
interaction flow, architecture change, runtime capability, reference-project
finding, or composite concept/pattern analysis into a production-grade Herdr
change without allowing visual similarity, source-language convenience, or an
agent's token budget to bypass Herdr's runtime/client architecture, TDD gates,
failure-path coverage, performance evidence, or Git discipline.

The new sibling module contains an analysis graph and a delivery graph. The
analysis graph can start without a reference repository. It explores goals,
behaviors, layouts, data flow, ownership, failure semantics, alternatives, and
current Herdr fit, then emits one normalized change-intent package. The
delivery graph accepts only that package plus explicit product authorization.

Ratatui reference intelligence P0-P14 remains the deep reference-project path.
Its completion is one possible analysis input, never implicit permission to
change product code.
Every delivery run must declare its target behavior, current Herdr behavior,
semantic diff, authority boundary, implementation slices, tests, performance
budgets, rollback, and publication evidence.

## Why a Separate Pipeline

Appending implementation phases to P0-P14 would mix immutable research with
mutable product work and would make reference intake a false prerequisite for
native Herdr product design. Merely making P14 optional would be too weak: it
would not define how native features, pages, layouts, or multi-concept
brainstorms obtain equivalent evidence and readiness. Separate pipelines per
change type would duplicate authority, testing, and delivery contracts.

The selected design uses one generalized analysis graph with typed intake
modes, then one delivery graph. A generic execution workflow alone would not
enforce Ratatui, PTY, plugin, event, persistence, protocol, and terminal-
capability obligations.

## Module Boundary

The generalized graph belongs in the new repo-local sibling module
`.codex/skills/herdr-change-pipeline/`, not inside
`ratatui-design-intelligence`. The latter remains responsible for Ratatui
design queries, reference-project corpus intake, source mapping, and P0-P14
adaptation intelligence. Expanding it to every server, persistence, protocol,
or platform feature would create a god module and would fail to route native
non-TUI changes reliably.

`herdr-change-pipeline` owns typed change intake, fractal/dimensional analysis,
target synthesis, normalized handoff, and production delivery governance. It
depends on Herdr project rules and may consume Ratatui/reference artifacts, but
the two modules remain independently versioned, testable, replaceable, and
documented.

The chosen model is therefore:

```text
reference project P0-P14 ----> reference adapter ----\
native feature/page/layout --------------------------+-> A0-A7 change intelligence
design/component/interaction ------------------------+      -> change-intent-package.json
architecture/runtime capability ---------------------+      -> explicit approval/authorization
composite concept and pattern inputs ----------------/      -> I0-I14 change delivery
```

## Scale and Decomposition

The canonical model has two linked graphs:

- 8 analysis phases, A0-A7;
- 15 delivery phases, I0-I14;
- 8 mandatory architecture-risk domains;
- feature-specific micro slices generated from the requirements and authority
  graph rather than a fixed arbitrary count;
- 5 test layers;
- 9 cross-test families;
- one traceability chain from input evidence and analysis nodes to requirement,
  test point, slice, commit, verification result, and publication record.

Micro slices must be independently testable and reversible. A slice may cross
files but must not cross unrelated authority domains merely to reduce commit or
agent count.

## Typed Analysis Intake

Every analysis run declares one primary mode and may declare supporting modes:

- `reference_project_adaptation`;
- `native_feature`;
- `page`;
- `layout`;
- `design_system`;
- `component`;
- `interaction_flow`;
- `behavior_correction`;
- `architecture_refactor`;
- `runtime_capability`;
- `composite`.

A mode changes the required evidence, not the architecture constitution. A
native page does not need a source repository or license record. A reference
adaptation does. A layout analysis requires breakpoints, region ownership, and
terminal capability fallbacks. A runtime capability requires server/client,
protocol, lifecycle, and performance analysis. A composite run names every
input and proves how conflicts were resolved.

Small changes may use a proportionally shallow analysis tree, but they may not
skip authority, current behavior, expected behavior, failure semantics, tests,
or explicit scope merely to save tokens.

## Fractal Analysis Model

Analysis is fractal: the same evidence schema is reusable at different levels
without forcing every change into one giant document. Allowed node levels are:

```text
initiative
  -> experience or workflow
    -> page
      -> region or layout
        -> component
          -> interaction or behavior
            -> state transition
              -> failure and recovery path
```

Every node records:

- stable node ID, parent, children, and analysis mode;
- goal, actor, trigger, inputs, outputs, and observable behavior;
- current evidence, desired outcome, and semantic diff;
- state/data owner and side-effect owner;
- terminal geometry/capability constraints when applicable;
- normal, empty, loading, unavailable, error, cancellation, and recovery states;
- dependencies, risks, non-goals, acceptance criteria, and confidence;
- concept/pattern matches with provenance;
- unresolved conflicts and the decision required to close them.

The root may be ready only when every required descendant is `complete`,
explicitly `not_applicable`, `rejected`, or `blocked`. Silent omission is
invalid.

## Parallel Dimensional Analysis

Each applicable node is evaluated across these dimensions:

1. product goal, actor, scenario, and user value;
2. behavior, state transitions, interaction, and lifecycle;
3. page hierarchy, navigation, overlay, focus, and input ownership;
4. responsive layout, geometry, terminal cells, and capability fallback;
5. component contract, variants, theme tokens, and design consistency;
6. data provenance, transformation, caching, and authority;
7. runtime, server/client, API, event, PTY, and concurrency boundaries;
8. failure, recovery, security, trust, and resource bounds;
9. persistence, migration, compatibility, and rollback;
10. platform, accessibility, Unicode, color, and degraded behavior;
11. performance, allocation, latency, throughput, and soak risk;
12. integration tier, dependency, license, reuse mode, and maintenance cost.

Independent read-only dimensions may run in parallel. Each job has exclusive
artifact ownership, immutable input hashes, evidence provenance, and a bounded
output schema. Parallel jobs do not edit product code. One synthesis owner
merges them, records agreement and conflict, rejects unsupported conclusions,
and preserves minority/caution findings rather than averaging them away.

`not_applicable` requires a reason and evidence. `blocked` preserves the
missing input or decision. No dimension is considered complete merely because
another dimension mentioned the same topic.

## Brainstorming and Synthesis Contract

Before choosing a target, the analysis graph produces two or three viable
approaches when more than one architecture or experience is plausible. Each
option states user value, behavior, authority, data flow, layout/component
impact, failure modes, platform/capability limits, implementation cost,
reversibility, and test burden.

The synthesis decision may be:

- `go` with one approved target;
- `conditional_go` with explicit preconditions;
- `no_go` because current Herdr already satisfies the behavior or the proposed
  abstraction lacks real pressure;
- `blocked` pending a user, evidence, capability, or authority decision.

Brainstorming is not implementation authority. The selected option is frozen
in the normalized handoff and must pass the delivery authorization gate.

## Normalized Change-Intent Handoff

Every analysis path converges on `change-intent-package.json`. It contains:

- package identity, schema version, analysis mode, source hashes, and status;
- approved goals, actors, scenarios, non-goals, and target experience;
- fractal analysis tree and dimensional coverage summary;
- current Herdr evidence and graph freshness;
- behavior, architecture, data-flow, failure, capability, and performance diff;
- authority and side-effect ownership decisions;
- page/layout/component contracts when applicable;
- selected option, rejected alternatives, and decision provenance;
- concept/pattern/reference matches and license boundaries when applicable;
- requirements, acceptance criteria, initial test obligations, risks, and
  open decisions;
- delivery readiness and explicit user approval evidence.

The P14 reference-project adapter maps source and stack-adaptation artifacts
into this schema. It does not receive a privileged delivery path. Native and
design modes create the same schema directly from their own evidence.

## Architecture Constitution

These rules apply to every phase, artifact, task, agent, and review gate.

### 1. Authority First

Every state field and behavior is classified before implementation as one of:

- `server_runtime_truth`;
- `tui_client_projection`;
- `plugin_owned_state`;
- `host_plugin_metadata`;
- `platform_capability`;
- `derived_ephemeral_view`;
- `out_of_scope`.

Shared pane, terminal, agent, process, task, session, lifecycle, integration,
and remote-client facts default to server/runtime authority. Selection, modal,
hover, theme, screen geometry, animation, unsent form input, and other display
preferences default to TUI/client authority. Exceptions require an explicit
ADR and cross-client evidence.

The server exposes neutral domain data. The TUI creates labels, rows, icons,
colors, selected state, and responsive projections. Protocol and event names
must not contain presentation terms such as `sidebar`, `row`, `card`, or
`widget` unless the data is genuinely client-local and never enters the shared
runtime protocol.

### 2. Small Core and Change Placement

Every native feature first chooses its correct runtime/client, page, component,
or platform owner. An external integration must additionally use the least
coupled viable tier:

1. manifest-based detection, action, hook, link handler, or pane;
2. external process plugin using a versioned CLI/socket API;
3. built-in adapter only when ubiquity, latency, security, or terminal
   lifecycle evidence proves the first two tiers insufficient.

Popularity, novelty, or visual fit is not evidence for core inclusion or for a
generic UI abstraction. The decision must record dependency cost, release
coupling, platform matrix, failure isolation, abstraction trigger, and removal
path. A new page or layout may justify a private concrete seam without
automatically authorizing a registry, plugin surface, protocol field, or core
dependency.

### 3. Single PTY and Terminal Authority

PTY output is read once and parsed once into canonical terminal state. TUI,
detection, remote clients, logging, and plugins consume snapshots, deltas, or
domain events; they do not attach independent readers or parsers.

The PTY/terminal hot path must have explicit ownership, bounded queues, EOF and
write-failure behavior, shutdown and child-process cleanup, resize semantics,
backpressure, resynchronization, and slow-consumer isolation. Terminal bytes
must not be dropped casually because ANSI, UTF-8, and control sequences may
span chunks.

Batching values are measured policies, not universal constants. A run may
evaluate a 1-8 ms parser/input coalescing range or 16-33 ms UI frame range, but
must calibrate the final budget against workload and environment evidence.

### 4. Process-Isolated Plugins

Plugins remain ordinary external processes. They do not receive `Frame<'_>`,
`&mut AppState`, an internal widget tree, server memory, PTY callbacks, or
unversioned private socket types.

Plugin-host work must declare manifest validation, path confinement,
environment/secret exposure, concurrency, timeout, cancellation, process-tree
cleanup, bounded stdout/stderr/logs, malformed-data behavior, event-loop
prevention, version compatibility, install/update/uninstall lifecycle,
resource accounting, and restart policy. Unsupported controls are recorded as
gaps rather than assumed.

Custom plugin pages should first use a versioned declarative view/data schema
rendered by Herdr-owned Ratatui components. Native in-process UI injection
requires a new approved architecture design, not an implementation shortcut.

### 5. Ordered and Bounded Events

Shared event names describe domain facts, not UI consequences. Each event
addition must declare:

- producer and source of truth;
- session/entity identity;
- sequence and, when required, entity revision semantics;
- snapshot relationship and reconnect gap handling;
- ordering, duplicate, idempotency, replay, and stale-event behavior;
- subscriber backpressure and bounded retention;
- recursion and event-storm prevention;
- payload/schema version and old/new client compatibility.

Not every event must gain every field. Existing sequence mechanisms are reused
where sufficient; new revision or identity fields require domain evidence and
protocol review.

### 6. Pure Projection and Render

Runtime state is read immutably by the page. Page-local selection, forms,
modals, focus, filters, viewport, and animation remain client-owned. Side
effects leave page update logic as commands. Geometry/projection is computed
before render; render consumes a prepared view model and only writes Ratatui
buffers.

Render must not perform process execution, filesystem/network/socket I/O,
config reads, regex compilation, JSON parsing, terminal snapshot construction,
full-scrollback rewrapping, or hidden state mutation. Allocation, cloning, and
frame-time budgets are required for critical or high-frequency pages.

### 7. Separated Persistence Domains

The run distinguishes:

- Herdr session truth;
- host plugin metadata, trust, permissions, and version;
- plugin-owned application data;
- bounded terminal scrollback/history;
- TUI preferences and configuration.

Persistence changes must define schema/version ownership, atomicity,
concurrency, crash consistency, migration, downgrade/rollback, disk-full and
read-only behavior, corruption recovery, save latency, and data-size limits.
Plugin-owned data is namespaced and confined; the host does not silently merge
it into session truth.

### 8. Isolated Platform Behavior

OS-specific process, PTY, shell, signal, clipboard, notification, path,
permission, and socket behavior stays in `src/platform/<os>.rs` or the
appropriate existing platform seam. Core code consumes shared contracts.
Every platform-affecting feature declares supported, degraded, and unavailable
capability behavior and has compile/runtime evidence proportional to risk.

### 9. Evidence Cannot Be Compressed Away

Agents must not reduce required analysis, test coverage, failure evidence,
documentation, or cross-layer verification to conserve tokens or time. Large
work is decomposed, resumed, or handed off with durable artifacts; it is not
declared complete early.

No completion claim is valid with an unexplained in-scope bug, failing test,
flaky or retry-only green, missing required artifact, undeclared semantic diff,
stale graph, unresolved product-code boundary, or unverified publication SHA.

## Current Herdr Evidence Baseline

The delivery pipeline begins from observed code, not a greenfield model. The
2026-07-15 Codebase Memory graph contained 19,534 nodes and 91,017 edges. The
following are baseline observations to revalidate at I2, not permanent truths:

- `PaneRuntime`/`TerminalRuntime` already own terminal lifecycle seams;
- Unix PTY user input uses a bounded actor command channel and waits for
  capacity, while separate control state preserves critical handoff/resize
  behavior;
- PTY read callbacks update canonical terminal state and rendering is
  coalesced through dirty/notify mechanisms;
- `EventHub` retains a bounded 512-event sequence-indexed history, while the
  public `EventEnvelope` currently carries event and data rather than exposing
  that internal sequence directly;
- plugin commands already have a 32-command concurrency cap, 64 KiB captured
  stdout/stderr cap, and 200-log retention cap;
- the inspected plugin execution path waits for child exit but did not prove a
  command timeout, cancellation, restart policy, or full descendant cleanup;
- session saves are scheduled/debounced and normally moved to a background
  writer, with an inline fallback when the save thread cannot start;
- platform behavior is divided across Linux, macOS, Windows, and fallback
  modules;
- Herdr's project rules require pure render, testable state, isolated platform
  code, and a stronger server-owned runtime/TUI-client boundary.

I2 must refresh these claims. A changed or missing symbol becomes a recorded
drift, not an excuse to preserve a stale design.

## Required Artifacts

Every run uses a run directory with machine-readable status and immutable
evidence references. Analysis artifacts are:

| Artifact | Purpose |
|---|---|
| `analysis-run.json` | Mode, pipeline/module/schema versions, phase state, source hashes and status |
| `analysis-inputs.json` | User intent, reference inputs, constraints, approvals and provenance |
| `fractal-analysis-tree.json` | Initiative-to-recovery nodes with stable identity and parent/child traceability |
| `analysis-dimension-matrix.json` | Twelve-dimension applicability, owner, evidence, conflict and status |
| `concept-pattern-match.json` | Source-backed or Herdr-native concepts, patterns, cautions and confidence |
| `option-set.json` | Viable approaches and tradeoffs, or evidence that only one option is legitimate |
| `synthesis-decision.json` | Selected target, decision status, conflicts, conditions and approval evidence |
| `change-intent-package.json` | Validated normalized handoff consumed by delivery I0 |

Reference-project artifacts, stack-adaptation maps, license records, layout
maps, design-token inventories, or capability probes are conditionally required
by analysis mode and dimension applicability. They are never fabricated for a
native mode and never omitted for a reference mode.

Delivery artifacts are:

| Artifact | Purpose |
|---|---|
| `delivery-run.json` | Pipeline/module/schema versions, status, target ref, artifact hashes, phase state |
| `authorization.json` | Product-code scope, allowed paths, runtime/socket prohibitions, Git/push authority |
| `change-prd.md` | Actor, scenario, behavior, non-goals, functional and non-functional requirements |
| `architecture-decision-record.json` | Authority, ownership, extension tier, compatibility, alternatives and tradeoffs |
| `feature-architecture-checklist.json` | The mandatory 15-question feature review with evidence and owner |
| `architecture-risk-register.json` | Eight risk domains, triggers, invariants, failure modes and required gates |
| `requirements-traceability.json` | Analysis evidence/node to requirement, test, slice, commit and verification chain |
| `current-target-diff.json` | Current behavior, expected behavior, semantic/ownership/failure/performance diff |
| `test-point-catalog.json` | Pre-code named tests with explicit current/expected/diff/result/reason contracts |
| `test-strategy.json` | Five layers, fakes/fixtures, platforms, cadence, commands and stop rules |
| `task-graph.json` | Dependency-ordered tasks, owners, inputs, outputs, gates and rollback edges |
| `implementation-slices.json` | RED/GREEN/refactor slices, exact boundaries, commits and acceptance evidence |
| `slice-evidence.jsonl` | Append-only observed RED/GREEN/refactor commands and results |
| `cross-test-matrix.json` | Nine cross-test families and scenario coverage |
| `performance-budget.json` | Workload, environment, baseline, target, regression ceiling and measurements |
| `soak-chaos-plan.json` | Scenario, cadence, duration, telemetry, leak/zombie criteria and result |
| `migration-plan.json` | Protocol/data/config migration, compatibility and downgrade behavior |
| `rollback-plan.json` | Per-slice and release rollback trigger, command and data handling |
| `verification-ledger.json` | Fresh focused, full, cross, performance and repository gate evidence |
| `publication-record.json` | Atomic commits, ancestry proof, remote SHAs, graph refresh and final state |

Artifacts use independent `schema_version` fields. Module, pipeline, and
artifact versions must not be conflated.

## Mandatory Feature Architecture Checklist

I1 creates the checklist; I4 converts each applicable answer into tests; I14
rejects stale or unresolved answers.

1. Is each state field owned by server/runtime or TUI/client?
2. Can a headless or remote client use the shared behavior?
3. Are new API/event names neutral and independent of UI projection?
4. Does the feature add work to the PTY/terminal hot path?
5. Does any work run on every output chunk, event, or frame?
6. Are all queues, logs, histories, and subscriber buffers bounded?
7. Can blocking work occupy the main loop, PTY actor, or async worker?
8. Is core code being chosen where a manifest or process plugin is viable?
9. Does render perform allocation, cloning, parsing, mutation, or I/O beyond its budget?
10. What happens across detach, output while detached, and reattach?
11. What is the source of truth with multiple clients?
12. Can a slow client, plugin, subscriber, or persistence writer block another actor?
13. Is schema/protocol/config/state migration or downgrade handling required?
14. Are Linux, macOS, and Windows differences isolated behind platform seams?
15. Which performance regression test and budget proves the change is acceptable?

Each answer includes `classification`, `current_evidence`, `target_decision`,
`required_tests`, `owner`, `status`, and `waiver`. `waiver` is allowed only with
an approver, reason, expiry, and residual-risk entry.

## Architecture Risk Register

The eight required domains are:

1. runtime/client authority;
2. PTY/terminal hot path;
3. plugin host and declarative extension surface;
4. event/protocol/subscriptions;
5. persistence/storage/migration;
6. page/projection/render/input/focus;
7. platform capabilities;
8. integration/core dependency boundary.

Each record contains domain trigger, current symbols and evidence, target owner,
invariant, hot-path classification, failure modes, required test points,
performance/capability budget, rollout, rollback, and status. Non-applicable
domains require evidence, not omission.

## A0-A7 Change-Intelligence Phase Contract

### A0 - Intake, Mode and Evidence Boundary

- capture the user's goal, constraints, supplied references, target surface,
  current decision authority, and explicit non-goals;
- select one primary analysis mode and supporting modes;
- hash inputs, freeze source/target Git boundaries, and keep product-code
  authorization false;
- determine required dimensions and conditional artifacts by mode.

Exit: validated analysis run and no ambiguous change identity.

### A1 - Goals, Actors, Scenarios and Success

- define user value, actors, triggers, workflows, observable behavior, and
  normal/empty/loading/unavailable/error/recovery outcomes;
- distinguish requested appearance from requested capability;
- define measurable success, rejection, and no-go criteria;
- preserve unresolved user decisions explicitly.

Exit: goal and scenario contract sufficient to judge alternatives.

### A2 - Fractal Decomposition

- decompose the change into initiative, experience, page, region/layout,
  component, interaction, transition, and failure/recovery nodes as applicable;
- assign stable identities, dependencies, evidence needs, and analysis owners;
- stop decomposition when a node has one coherent behavior and authority owner;
- reject giant nodes and speculative empty abstraction layers.

Exit: complete fractal tree with no orphan requirement or hidden descendant.

### A3 - Parallel Dimensional Investigation

- run independent applicable dimensions in parallel with immutable inputs and
  exclusive artifact ownership;
- map behavior, UX, layout, component, data, runtime, failure, persistence,
  platform, performance, and integration concerns;
- for reference mode, trace source symbols/data flow and license/reuse limits;
- for native modes, use current Herdr evidence and product goals without
  inventing an external source requirement.

Exit: every dimension is complete, evidenced not-applicable, or blocked.

### A4 - Concept, Pattern and Option Brainstorm

- match corpus patterns, existing Herdr seams, native design alternatives, and
  caution/negative references by behavior rather than name or appearance;
- produce two or three viable options when the decision is non-trivial;
- record tradeoffs, authority, data flow, lifecycle, failure, performance,
  terminal fallback, cost, reversibility, and test burden;
- keep source inspiration separate from target architecture.

Exit: decision-ready option set with explicit rejection reasons.

### A5 - Fresh Herdr Cartography and Fit

- refresh Codebase Memory and map exact runtime, API, event, state, page,
  layout, component, render, input, persistence, platform, and test seams;
- trace current behavior and identify reuse, extension, replacement, or absence;
- compare proposed abstractions against real consumer pressure;
- permit an evidence-backed no-go when Herdr already satisfies the behavior or
  a general interface is not yet earned.

Exit: source-backed current-state and fit map with fresh graph evidence.

### A6 - Cross-Dimension Synthesis and Target Contract

- merge parallel findings under one synthesis owner;
- resolve or preserve conflicts; do not average incompatible claims;
- select `go`, `conditional_go`, `no_go`, or `blocked`;
- freeze target behavior, page/layout/component contracts, authority, data
  flow, architecture diff, failure semantics, budgets, risks, and acceptance;
- obtain explicit user approval for the selected target.

Exit: coherent approved target or honest no-go/blocked result.

### A7 - Normalized Handoff and Readiness Audit

- generate and validate `change-intent-package.json`;
- prove every goal, fractal node, dimension, option, decision, requirement,
  risk, and initial test obligation is traceable;
- hash the complete analysis input set and record graph freshness;
- keep product-code authorization false and classify readiness as `ready`,
  `partial`, `blocked`, `no_go`, or `rejected`.

Exit: immutable normalized handoff accepted by delivery I0, or no delivery run.

## I0-I14 Phase Contract

### I0 - Authorization and Immutable Handoff

- validate `change-intent-package.json`, its A7 readiness, user-approved target,
  source hashes, and conditional mode artifacts;
- when the package came from a reference project, validate the P14 adapter and
  stack-adaptation trace; do not require those artifacts for native modes;
- require explicit `product_code_changes_authorized: true` before product edits;
- freeze allowed paths, target branch/ref, dirty-worktree boundary, Git remote,
  stable-runtime/socket prohibitions, and push authority;
- fail closed on absent MCP evidence required by the handoff.

Exit: validated `authorization.json` and immutable input manifest.

### I1 - PRD, Scenarios, Authority and Non-Goals

- materialize `change-prd.md` from the approved handoff and confirm user-visible
  scenarios, behavior templates, actors, data inputs and outputs,
  error/loading/empty states, non-goals, NFRs, and acceptance criteria;
- preserve analysis node IDs and do not silently reinterpret the approved target;
- classify every proposed state/data field by authority;
- choose server/runtime, TUI/client, platform adapter, manifest, process plugin,
  or built-in integration placement as applicable;
- create the 15-question checklist, ADR, and initial risk register.

Exit: no unresolved requirement, authority, extension-tier, or scope decision.

### I2 - Fresh Target Cartography and Drift

- refresh Codebase Memory and map current runtime, PTY, terminal, API, event,
  plugin, persistence, page, input, render, platform, tests, and protocol seams;
- trace intent-to-target behavior and data flow through exact symbols;
- compare the current graph to A5 bindings and record drift or absence;
- for a reference-derived package, retain source-to-target traceability; for a
  native package, retain goal/scenario/fractal-node-to-target traceability;
- use file search only for non-code/literal evidence after graph discovery.

Exit: evidence-backed target map with no invented MCP data.

### I3 - Current, Expected and Semantic Diff

For each behavior, record:

- what the behavior is and who triggers it;
- current Herdr behavior and evidence;
- expected behavior and authority owner;
- exact functional, architectural, data-flow, failure, compatibility,
  capability, performance, and lifecycle diff;
- chosen delivery strategy: `direct_reuse`, `structural_adapter`,
  `behavior_reimplementation`, `herdr_native`, or `reject`;
- chosen P14 source translation mode when and only when the package is
  reference-derived;
- why the change belongs in Herdr and what remains out of scope.

Exit: every requirement has a declared diff or an explicit no-change/reject decision.

### I4 - Test Architecture Before Production Code

Immediately after PRD/diff acceptance and before production code, create the
complete named test-point catalog. Every test point must state:

- ID and layer;
- what is tested;
- current observed behavior;
- expected behavior;
- behavior/architecture diff;
- expected test result and failure signature;
- why the test is required;
- fixture/fake/platform/workload;
- command and evidence location;
- cross-test and performance links;
- implementation slice that the test gates.

The catalog covers pure unit, deterministic component, real integration,
performance-regression, and soak/chaos layers. A test that passes before the
target behavior is implemented must be reclassified as characterization or
corrected; it is not valid RED evidence.

Exit: requirements, risks, checklist answers, tests, and expected failures are traceable.

### I5 - Dependency-Ordered Task and Slice Graph

- order work by source-of-truth and dependency, not visual convenience;
- define characterization, RED, GREEN, refactor, cross-test, performance,
  migration, docs, rollback, and publication tasks;
- assign exact path/symbol ownership and conflict boundaries;
- plan lowercase conventional atomic commits and safe rollback points;
- declare PR, nightly, release, and manual-only gate cadence.

Exit: executable task graph with one in-progress slice and no hidden dependencies.

### I6 - Characterization and Baseline

- prove current behavior, architecture invariants, failure behavior, and
  performance baseline before mutation;
- use fake clock/PTY/platform/plugin/socket where determinism is required;
- capture known bugs as exact failing contracts rather than weakening gates;
- preserve existing behavior not named in the approved diff.

Exit: current-state evidence is fresh and the intended RED failure is distinguishable.

### I7 - Observed RED Per Slice

- add the smallest compile-valid test that expresses the approved missing or
  incorrect behavior;
- run it and record the expected failure output;
- reject setup, compilation, unrelated fixture, or flaky timing failures as RED;
- commit RED locally only according to the approved Git plan; never publish a
  failing branch tip as completed work.

Exit: each production slice has genuine, attributable RED evidence.

### I8 - Minimal Production-Grade GREEN

- implement the least code that satisfies the approved behavior and
  architecture constitution;
- keep shared facts server-owned, projections client-owned, hot paths bounded,
  plugins isolated, events neutral, render pure, storage separated, and
  platform behavior isolated;
- do not pull unrelated cleanup into the slice.

Exit: targeted test is green and no adjacent characterization regresses.

### I9 - Refactor and Local Invariant Gates

- remove duplication and incidental complexity without changing behavior;
- run local unit/component tests, invariant assertions, format/lint/static
  checks, render allocation checks, and relevant schema validators;
- prove no god-state, hidden I/O, lock, queue, ownership, or platform leakage
  was introduced.

Exit: slice is maintainable and locally complete.

### I10 - Cross-Layer Behavioral Tests

The nine mandatory cross-test families are:

1. server state -> snapshot/API -> TUI projection;
2. command/input -> runtime mutation -> event -> client convergence;
3. snapshot/reconnect -> event ordering/gap -> stale/duplicate handling;
4. multi-client -> detach/output/reattach -> common source of truth;
5. PTY output -> parser -> detection/remote/TUI with a slow consumer;
6. plugin process -> versioned data/event -> declarative Herdr rendering;
7. event hook -> recursion/storm/bounded queue -> recovery;
8. persistence save/migration/restart -> restored runtime/client projection;
9. platform capability -> supported/degraded/unavailable UI and API behavior.

Exit: every applicable family has passing tests or an approved bounded waiver.

### I11 - Failure, Recovery, Security and Capability

Test applicable non-happy paths including malformed manifests/data, missing
executables, timeout/cancellation, plugin crash, descendant leaks, output/log
caps, secret exposure, path traversal/symlinks, key conflicts, event storms,
queue saturation, ANSI/UTF-8 chunk boundaries, reader EOF, writer failure,
rapid resize, shutdown under output, disk full, read-only/corrupt storage,
migration failure, old/new protocol combinations, and platform degradation.

Exit: no unexplained in-scope failure, leak, deadlock, data loss, or privilege expansion.

### I12 - Performance Budgets and Optimization

Measure before optimizing. `performance-budget.json` records metric, workload,
environment, baseline, target, maximum regression, sample count, method,
result, and waiver. Candidate starting budgets include:

- idle CPU approximately zero and no render without an event;
- local input-processing overhead p99 below 10 ms;
- frame composition p95 below 8 ms;
- attach snapshot below 100 ms for a declared representative session;
- lossless PTY consumption for declared throughput workloads;
- bounded, approximately linear memory at 50-100 panes when in scope;
- negligible impact from a slow client on other clients;
- plugin crash isolated from server availability;
- bounded event, output, log, history, and persistence queues.

These are hypotheses until I12 records environment-specific baselines and
approved thresholds. Optimization must retain functional, failure, and
capability behavior.

Exit: applicable budgets pass with reproducible evidence and no semantic regression.

### I13 - Full Repository, Migration and Cadence Gates

- run focused tests, `just check`, maintenance/vendor/integration-asset tests,
  protocol/schema compatibility, docs/migration validation, and platform gates
  proportional to risk;
- schedule heavy throughput, 24-hour stream, thousands of pane lifecycle,
  repeated detach/reattach, resize storm, plugin crash/restart, disk-full,
  network interruption, client kill, memory/task/zombie leak tests as nightly,
  release, or manual gates when unsuitable for each PR;
- record cadence; deferred does not mean omitted.

Exit: zero in-scope failing tests or known untracked bugs and a complete release-risk record.

### I14 - Evidence Audit, Git Publication and Graph Refresh

- audit every requirement, risk, checklist answer, test, slice, commit,
  migration, rollback, and performance result;
- inspect the final diff and prove no unauthorized path or stable runtime/socket
  was touched;
- commit atomically, fetch, prove fast-forward ancestry, push only the allowed
  CyPack refs, and verify exact remote SHA equality;
- refresh Codebase Memory and prove current symbols/artifacts are indexed;
- publish completion only when the verification ledger has no unexplained
  in-scope failure, stale evidence, missing artifact, or retry-only green.

Exit: reproducible completion evidence or an honest `partial`/`blocked` status.

## Five Test Layers

### Layer 1 - Pure Unit

State transitions, reducers, layout geometry, manifests, protocol validation,
view-model construction, event semantics, capability policy, and migrations
without PTY, filesystem, network, or Tokio when practical.

### Layer 2 - Deterministic Component

Fake PTY, platform, plugin process, socket client, filesystem, and clock for
timeout, ordering, backpressure, retry, and lifecycle tests.

### Layer 3 - Real Integration

Real PTY/shell, Unix socket or named pipe, process plugin, detach/reattach,
multi-client, persistence restart, and relevant OS behavior in isolated test
environments.

### Layer 4 - Performance Regression

Throughput, latency, allocation, CPU, memory, queue depth, snapshot size, slow
consumer, and many-pane measurements with declared workload and environment.

### Layer 5 - Soak and Chaos

Long-running output, repeated lifecycle, disconnect/reconnect, disk/network
failure, plugin crash, resize storms, client termination, and leak/zombie
observability at declared PR/nightly/release/manual cadence.

## Pipeline Contract Test Points

Before implementing module manifests, validators, templates, or scripts, add
failing tests for these contracts:

| Test point | Expected result | Reason |
|---|---|---|
| TP-CHG-MODULE | Missing/wrong module identity, version, pipeline ID, or independent schema version fails | The sibling module and its two graphs need one machine-enforced identity |
| TP-CHG-MODES | Every canonical intake mode validates; unknown mode fails; mode-conditional artifacts are required without being fabricated | Native and reference work must share governance without sharing false prerequisites |
| TP-CHG-FRACTAL | Orphan nodes, invalid parent/level transitions, silent descendants, or missing node contracts fail | Fractal analysis needs complete traceability rather than nested prose |
| TP-CHG-DIMENSIONS | Missing dimension status/evidence and unsupported `not_applicable` fail | Parallel dimensional analysis cannot silently omit risk surfaces |
| TP-CHG-PARALLEL | Duplicate artifact ownership, mutable input drift, unmerged conflicts, or absent synthesis owner fails | Parallel analysis must remain deterministic and accountable |
| TP-CHG-OPTIONS | Non-trivial decisions without alternatives/tradeoffs fail; single-option runs require explicit necessity evidence | Brainstorming must expose real choices without manufacturing fake alternatives |
| TP-CHG-HANDOFF | Incomplete goals, diffs, ownership, risks, tests, approval, or readiness fails the normalized package | Delivery must receive equivalent evidence from every analysis mode |
| TP-CHG-REFERENCE-ADAPTER | P14 maps losslessly into the common package while native modes remain valid without P14/license/source artifacts | Reference intelligence is one adapter, not privileged architecture |
| TP-CHG-NO-GO | `no_go`, `blocked`, and `rejected` terminate without product authorization or delivery tasks | Analysis must be able to prevent speculative implementation |
| TP-CHG-DELIVERY-AUTH | I0 rejects absent/invalid handoff, missing user target approval, or false product authorization | Analysis and brainstorming never imply mutation permission |
| TP-CHG-TRACE | Analysis input/node/dimension/decision to requirement/test/slice/commit/evidence links are complete and acyclic | Cross-session and agentic execution needs durable provenance |
| TP-CHG-COMPAT | Ratatui v2.1 P0-P14 behavior and artifacts remain backward compatible | The generalized module must not destabilize the corpus pipeline |

Execution order is RED, minimal GREEN, refactor, focused module tests, complete
validators, skill validation, JSON/schema checks, eval fixtures for every
analysis mode, cartography audit, and Git diff/isolation gates.

## Agent Execution Contract

Agents running this pipeline must:

- use Codebase Memory graph tools before file search for code discovery;
- stop MCP-dependent claims when MCP tools are unavailable;
- select a typed analysis mode instead of assuming a reference project exists;
- apply the same fractal node schema from initiative through failure/recovery;
- parallelize only independent read-only analysis jobs with exclusive artifact
  ownership, then merge through one accountable synthesis owner;
- preserve conflicting/caution findings until explicitly resolved;
- work phase-by-phase and slice-by-slice with durable artifact updates;
- create the test-point catalog before product code;
- use RED, GREEN, refactor with fresh command evidence;
- record current, expected, diff, expected result, and reason for every test;
- treat failures as data, preserve failed attempts, and never retry until green
  without explaining the cause;
- use one authority owner per state/behavior and one source of truth;
- preserve unrelated user changes and use isolated worktrees when required;
- never touch the installed stable Herdr process or inherited stable socket;
- never reduce scope or gates to fit an agent context window;
- hand off with exact next step, artifact state, failures, and commands if work
  must continue in another session.

## Error and Stop Semantics

- Missing explicit product authorization: `blocked` before product edits.
- Missing required MCP evidence: dependent claims and phases are `blocked`.
- Missing or invalid change-intent package: delivery is `blocked`.
- Analysis dimension omitted without evidence: A7 cannot become `ready`.
- P14 drift for a reference-derived package, or A5 drift for any package,
  without refreshed target evidence: `partial` or `blocked`.
- Unresolved cross-dimension conflict: `blocked`, not averaged into consensus.
- False RED, unexplained failure, or flaky retry-only green: stop the slice.
- Architecture checklist ambiguity that changes authority or extension tier:
  stop for decision; do not infer.
- Queue, hot-path, protocol, migration, security, capability, or rollback risk
  without tests: no GREEN/completion status.
- Remote divergence: stop publication; never force-push.
- Any unauthorized product path, stable runtime/socket access, or upstream
  write: pipeline failure.

## Git and Publication Contract

1. Commit this design specification as its own documentation concern.
2. After written approval, create a detailed TDD implementation plan.
3. Preserve the current untracked module baseline as a separately reviewable
   concern; land Ratatui v2.1 and the new `herdr-change-pipeline` v1.0 as
   separate module/version concerns.
4. Commit executable contract tests and implementation in the approved atomic
   sequence; do not publish a failing RED tip as completed work.
5. Use lowercase conventional commits, no emoji, no AI co-author lines.
6. Run fresh proportional verification before every completion claim.
7. Push only to the authorized CyPack fork/ref, fast-forward only.
8. Verify remote SHA equality, then refresh Codebase Memory.
9. Do not create a Herdr release tag for module/pipeline versions.

## Acceptance Criteria

The design is implemented only when:

1. sibling module `herdr-change-pipeline` version `1.0.0` exposes stable
   `herdr-change-intelligence-v1` and `herdr-change-delivery-v1` pipelines;
2. A0-A7 and I0-I14, all required and mode-conditional artifacts, statuses,
   dependencies, and exit gates are schema- and validator-enforced;
3. the nine architecture-constitution rules and eight risk domains are present
   in human, agent, template, validator, and eval contracts;
4. every supported intake mode can produce the same validated
   `change-intent-package.json` without fabricated reference artifacts;
5. fractal nodes, twelve analysis dimensions, parallel ownership, conflict
   synthesis, option decisions, and no-go outcomes are validator-enforced;
6. the 15-question checklist is created at I1, test-bound at I4, and audited at I14;
7. every implementation test point records what/current/expected/diff/result/reason;
8. all five test layers and nine cross-test families are classified and
   required where applicable;
9. candidate performance budgets require calibrated workload/environment evidence;
10. no token-saving, hidden waiver, unexplained failure, stale graph, or missing
   traceability can pass completion validation;
11. `ratatui-design-intelligence` v2.1 P0-P14 remains backward compatible,
    acts only as one optional handoff adapter, and does not acquire product-code
    authority or generic Herdr-change ownership;
12. native feature, page, layout, design, component, behavior, architecture,
    runtime-capability, reference, and composite fixture runs are eval-covered;
13. every new or changed executable behavior contract is observed RED before
    GREEN and passes fresh full verification without touching stable Herdr,
    inherited sockets, upstream, or unrelated user changes.
