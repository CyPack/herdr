# Herdr Integration Delivery Pipeline Design

## Status

- Design date: 2026-07-15
- Status: draft for written-spec review
- User decision: separate post-analysis delivery pipeline approved; architecture
  constitution and placement update requested
- Owning module: `.codex/skills/ratatui-design-intelligence/`
- Target module version after implementation: `2.2.0`
- Stable pipeline identity: `herdr-integration-delivery-v1`
- Pipeline version: `1.0.0`
- Input: immutable, validated P14 artifact set from
  `reference-project-intelligence-v2`
- Product-code authorization by default: false

## Purpose

Convert evidence-backed reference intelligence into a production-grade Herdr
feature without allowing visual similarity, source-language convenience, or an
agent's token budget to bypass Herdr's runtime/client architecture, TDD gates,
failure-path coverage, performance evidence, or Git discipline.

This is a separate pipeline from P0-P14. Research completion is an input
contract, never implicit permission to change product code. Every delivery run
must declare its target behavior, current Herdr behavior, semantic diff,
authority boundary, implementation slices, tests, performance budgets,
rollback, and publication evidence.

## Why a Separate Pipeline

Appending implementation phases to P0-P14 would mix immutable research with
mutable product work and would let corpus publication accidentally authorize
Rust changes. A generic execution workflow alone would not enforce Ratatui,
PTY, plugin, event, persistence, protocol, and terminal-capability obligations.

The chosen model is therefore:

```text
reference-project-intelligence-v2 (P0-P14, research authority)
  -> frozen artifact hashes and integration-verification.json
  -> explicit product_code_changes_authorized decision
  -> herdr-integration-delivery-v1 (I0-I14, product authority)
```

## Scale and Decomposition

The canonical model has:

- 15 macro phases, I0-I14;
- 8 mandatory architecture-risk domains;
- feature-specific micro slices generated from the requirements and authority
  graph rather than a fixed arbitrary count;
- 5 test layers;
- 9 cross-test families;
- one traceability chain from P14 evidence to requirement, test point, slice,
  commit, verification result, and publication record.

Micro slices must be independently testable and reversible. A slice may cross
files but must not cross unrelated authority domains merely to reduce commit or
agent count.

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

### 2. Small Core and Extension Ladder

An integration must use the least coupled viable tier:

1. manifest-based detection, action, hook, link handler, or pane;
2. external process plugin using a versioned CLI/socket API;
3. built-in adapter only when ubiquity, latency, security, or terminal
   lifecycle evidence proves the first two tiers insufficient.

Popularity or visual fit is not evidence for core inclusion. The decision must
record dependency cost, release coupling, platform matrix, failure isolation,
and removal path.

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
evidence references. Required artifacts are:

| Artifact | Purpose |
|---|---|
| `delivery-run.json` | Pipeline/module/schema versions, status, target ref, artifact hashes, phase state |
| `authorization.json` | Product-code scope, allowed paths, runtime/socket prohibitions, Git/push authority |
| `integration-prd.md` | Actor, scenario, behavior, non-goals, functional and non-functional requirements |
| `architecture-decision-record.json` | Authority, ownership, extension tier, compatibility, alternatives and tradeoffs |
| `feature-architecture-checklist.json` | The mandatory 15-question feature review with evidence and owner |
| `architecture-risk-register.json` | Eight risk domains, triggers, invariants, failure modes and required gates |
| `requirements-traceability.json` | P14 evidence to requirement, test, slice, commit and verification chain |
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

## I0-I14 Phase Contract

### I0 - Authorization and Immutable Handoff

- validate P14 readiness and hash every consumed artifact;
- require explicit `product_code_changes_authorized: true` before product edits;
- freeze allowed paths, target branch/ref, dirty-worktree boundary, Git remote,
  stable-runtime/socket prohibitions, and push authority;
- fail closed on absent MCP evidence required by the handoff.

Exit: validated `authorization.json` and immutable input manifest.

### I1 - PRD, Scenarios, Authority and Non-Goals

- write user-visible scenarios, behavior templates, actors, data inputs and
  outputs, error/loading/empty states, non-goals, NFRs, and acceptance criteria;
- classify every proposed state/data field by authority;
- choose manifest, process plugin, built-in adapter, or client-only tier;
- create the 15-question checklist, ADR, and initial risk register.

Exit: no unresolved requirement, authority, extension-tier, or scope decision.

### I2 - Fresh Target Cartography and Drift

- refresh Codebase Memory and map current runtime, PTY, terminal, API, event,
  plugin, persistence, page, input, render, platform, tests, and protocol seams;
- trace source-to-target behavior and data flow through exact symbols;
- compare current graph to P14 bindings and record drift or absence;
- use file search only for non-code/literal evidence after graph discovery.

Exit: evidence-backed target map with no invented MCP data.

### I3 - Current, Expected and Semantic Diff

For each behavior, record:

- what the behavior is and who triggers it;
- current Herdr behavior and evidence;
- expected behavior and authority owner;
- exact functional, architectural, data-flow, failure, compatibility,
  capability, performance, and lifecycle diff;
- chosen translation mode from the P14 stack-adaptation record;
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

## Agent Execution Contract

Agents running this pipeline must:

- use Codebase Memory graph tools before file search for code discovery;
- stop MCP-dependent claims when MCP tools are unavailable;
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
- P14 drift without refreshed target evidence: `partial` or `blocked`.
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
   concern before mixing v2.1/v2.2 changes.
4. Commit executable contract tests and implementation in the approved atomic
   sequence; do not publish a failing RED tip as completed work.
5. Use lowercase conventional commits, no emoji, no AI co-author lines.
6. Run fresh proportional verification before every completion claim.
7. Push only to the authorized CyPack fork/ref, fast-forward only.
8. Verify remote SHA equality, then refresh Codebase Memory.
9. Do not create a Herdr release tag for module/pipeline versions.

## Acceptance Criteria

The design is implemented only when:

1. module `2.2.0` exposes the stable `herdr-integration-delivery-v1` pipeline;
2. I0-I14, all required artifacts, statuses, dependencies, and exit gates are
   schema- and validator-enforced;
3. the nine architecture-constitution rules and eight risk domains are present
   in human, agent, template, validator, and eval contracts;
4. the 15-question checklist is created at I1, test-bound at I4, and audited at I14;
5. every implementation test point records what/current/expected/diff/result/reason;
6. all five test layers and nine cross-test families are classified and
   required where applicable;
7. candidate performance budgets require calibrated workload/environment evidence;
8. no token-saving, hidden waiver, unexplained failure, stale graph, or missing
   traceability can pass completion validation;
9. the v2.1 P0-P14 research pipeline remains backward compatible and does not
   acquire product-code authority;
10. every new or changed executable behavior contract is observed RED before
    GREEN and passes fresh full verification without touching stable Herdr,
    inherited sockets, upstream, or unrelated user changes.
