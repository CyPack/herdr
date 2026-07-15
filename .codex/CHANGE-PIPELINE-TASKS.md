# Herdr Change Pipeline — Durable Macro and Micro Task Registry

> **Execution skill:** use `superpowers:executing-plans` only after T1 has
> produced an approved code-level TDD plan. This registry is the durable queue,
> not permission to mutate Herdr product code.

## Goal

Turn native features, bug fixes, pages, layouts, design systems, components,
runtime capabilities, reference-project findings, and composite ideas into
evidence-backed Herdr changes through one canonical analysis and delivery
pipeline.

## Architecture

- `ratatui-design-intelligence` v2.1 owns Ratatui/reference-project analysis.
- `herdr-change-pipeline` v1.0 will own generalized A0-A7 change intelligence
  and I0-I14 delivery governance.
- A validated `change-intent-package.json` is the only analysis-to-delivery
  handoff.
- Product mutation remains forbidden until I0 validates the handoff, target
  approval, current repository state, and explicit product authorization.
- A task already in progress may enter through **mid-flight adoption**: capture
  the existing diff and evidence, reconstruct A0-A7 without discarding valid
  work, then apply the remaining I0-I14 gates from the current point forward.

## Tech Stack

- Rust and Ratatui for Herdr product behavior.
- Markdown, JSON, JSON Schema, and Python validators/tests for the pipeline.
- Codebase Memory MCP for current code graph evidence when available.
- Git with atomic conventional commits and targeted staging.
- `just` recipes by default for Herdr verification.

## Global Constraints

- Stable Herdr processes and inherited stable sockets are never touched.
- Runtime/server facts and TUI/client presentation state stay separated.
- Render remains pure; PTY data is read and parsed once through bounded paths.
- No production code without an observed behavior-specific RED test.
- No completion, graph-freshness, or publication claim without fresh evidence.
- Reference similarity is evidence, not implementation authority.
- S5, S6, S7, and N2.2 stay independently trigger-gated product tasks.
- Only one macro task and one micro task are active at a time unless an
  approved plan explicitly proves safe parallel ownership.
- No push to `upstream`, no force push, and no unrelated staging.

## Canonical Sources

- Ratatui/reference design:
  `docs/superpowers/specs/2026-07-15-ratatui-reference-intelligence-v2-1-design.md`
- Generalized analysis and delivery design:
  `docs/superpowers/specs/2026-07-15-herdr-integration-delivery-pipeline-design.md`
- Product queue: `.codex/TASKS.md`
- Current continuity: `.codex/CURRENT.md`

## Status and Active Pointer

Legend: `[x]` complete with evidence; `[ ]` pending; `ACTIVE` is the next
single permitted planning action.

- Active macro: **T0 — Governance and adoption**.
- Active micro: **T0.6 — written review and approval**.
- Next after approval: **T1.1 — freeze the implementation file tree**.
- Product-code authorization: **false**.
- Pipeline implementation, push, and reindex: **not yet performed**.

## T0 — Design, Governance, and Mid-Flight Adoption

- [x] **T0.1** Define Ratatui/reference intelligence v2.1 and its immutable
  research boundary (`86a25e8`).
- [x] **T0.2** Define the post-analysis Herdr delivery pipeline (`0ea0f77`).
- [x] **T0.3** Generalize the pipeline for native features, bug fixes, pages,
  layouts, designs, components, runtime capabilities, and composite work;
  select a sibling module instead of creating a god skill (`600c0d6`).
- [x] **T0.4** Create this durable macro/micro registry without activating the
  product queue.
- [x] **T0.5** Define the mid-flight adoption rule: preserve valid existing
  work, inventory current diffs/tests/commits, backfill A0-A7, classify the
  current delivery phase, and enforce all remaining gates.
- [ ] **T0.6 — ACTIVE** Obtain written review/approval for both design specs and
  this task registry.
- [ ] **T0.7** Freeze scope, module identities, version identities, artifact
  names, authorization defaults, and the non-product implementation boundary.

### Immediate Mid-Flight Adoption Contract

Until the canonical skills are implemented, another Herdr session must:

1. read `AGENTS.md`, `CLAUDE.md`, `.codex/BOOTSTRAP.md`, `.codex/CURRENT.md`,
   `.codex/TASKS.md`, this registry, and both design specs;
2. use `herdr-native-fm` and `ratatui-design-intelligence` where applicable;
3. inspect the existing branch, worktree, diffs, commits, tests, and graph
   before proposing changes;
4. treat the work as `mid_flight_adoption`, not restart it from zero;
5. build a retrospective A0-A7 package and identify the current I0-I14 phase;
6. preserve already valid RED/GREEN evidence but never invent missing RED;
7. continue feature/bugfix work only after explicit product authorization;
8. keep stable Herdr/socket isolation and atomic Git discipline.

## T1 — Code-Level TDD Implementation Plan

- [ ] **T1.1** Freeze the exact file tree for both modules, shared schemas,
  validators, tests, evals, lessons, and cartography outputs.
- [ ] **T1.2** Define exact module manifests, artifact interfaces, JSON Schema
  `$id` values, enums, conditional requirements, and version fields.
- [ ] **T1.3** Map every contract test to its test file, fixture, command,
  expected RED reason, minimal GREEN change, and regression gate.
- [ ] **T1.4** Define the validator CLI contracts, exit codes, diagnostics,
  deterministic output, and MCP-unavailable blocked behavior.
- [ ] **T1.5** Define the implementation slice graph, dependency order,
  rollback boundary, targeted staging set, and atomic commit sequence.
- [ ] **T1.6** Add exact verification commands and expected results for focused,
  module-wide, repository-wide, compatibility, and publication gates.
- [ ] **T1.7** Self-review the plan for placeholders, false parallelism,
  unowned artifacts, missing negative paths, and accidental product authority.
- [ ] **T1.8** Obtain written implementation-plan approval before T2.

## T2 — Ratatui Design Intelligence v2.1

- [ ] **T2.1** Preserve and audit the current untracked
  `.codex/skills/ratatui-design-intelligence/` baseline as an isolated concern.
- [ ] **T2.2** Write and observe RED tests for module identity/version and
  legacy P0-P14 compatibility.
- [ ] **T2.3** Write RED tests for stack profiles, language/framework mapping,
  architecture/behavior diffs, and reference-versus-native modes.
- [ ] **T2.4** Write RED tests for phase bindings at P2, P5, P9, and P14 plus
  run-state/resume semantics.
- [ ] **T2.5** Write RED tests for README, AGENTS, SKILL, governance, schema,
  validator, eval, and cartography consistency.
- [ ] **T2.6** Write negative tests for MCP absence, source/license uncertainty,
  missing evidence, artifact leakage, and product-code isolation.
- [ ] **T2.7** Implement the minimum module manifest, stack artifact/schema,
  validator, phase bindings, and run template needed for GREEN.
- [ ] **T2.8** Update human/agent guidance, lessons, examples, eval fixtures,
  and compatibility documentation.
- [ ] **T2.9** Run focused and complete module gates; record exact evidence.
- [ ] **T2.10** Commit baseline, RED, GREEN, and governance as separate atomic
  concerns.

## T3 — `herdr-change-pipeline` Module Scaffold

- [ ] **T3.1** Write RED `TP-CHG-MODULE` tests for module identity, version,
  directories, required documents, and default authorization=false.
- [ ] **T3.2** Create `.codex/skills/herdr-change-pipeline/` with `SKILL.md`,
  `README.md`, `AGENTS.md`, `module.json`, `assets/`, `references/`, `scripts/`,
  `tests/`, `evals/`, `lessons/`, and `cartography/`.
- [ ] **T3.3** Implement minimal manifest/schema validation and deterministic
  diagnostics until scaffold tests pass.
- [ ] **T3.4** Document skill routing, output ownership, resume behavior,
  source-of-truth order, and the separation from Ratatui reference research.
- [ ] **T3.5** Add errors, golden paths, edge cases, and shared-error routing.
- [ ] **T3.6** Verify the scaffold independently of Herdr product compilation.

## T4 — A0-A7 Change-Intelligence Engine

### A0 — Intake, Mode, and Evidence Boundary

- [ ] **T4.A0.1** RED-test every intake mode and reject unknown/ambiguous modes.
- [ ] **T4.A0.2** Model goals, inputs, evidence sources, current-work state,
  product authorization=false, and mode-conditional artifacts.
- [ ] **T4.A0.3** Implement `mid_flight_adoption` metadata: existing branch,
  commits, diffs, tests, known debt, current failures, and preserved evidence.
- [ ] **T4.A0.4** Block rather than fabricate when mandatory MCP/source evidence
  is unavailable.

### A1 — Goals, Actors, Scenarios, and Success

- [ ] **T4.A1.1** RED-test missing actors, scenarios, success criteria, and
  explicit non-goals.
- [ ] **T4.A1.2** Emit measurable target behavior and acceptance boundaries for
  features, bugs, pages, layouts, runtime work, and composite requests.

### A2 — Fractal Decomposition

- [ ] **T4.A2.1** RED-test orphan nodes, illegal level jumps, missing ownership,
  and missing failure/recovery leaves.
- [ ] **T4.A2.2** Implement the canonical chain: initiative -> experience/
  workflow -> page -> region/layout -> component -> interaction/behavior ->
  state transition -> failure/recovery.
- [ ] **T4.A2.3** Preserve parent/child traceability and stable identifiers.

### A3 — Parallel Dimensional Investigation

- [ ] **T4.A3.1** RED-test omitted required dimensions, duplicate ownership,
  unresolved contradictions, and unjustified conditional omissions.
- [ ] **T4.A3.2** Cover product; behavior; page/input; layout/capability;
  component/tokens; data authority; runtime/API/event/PTY; failure/security/
  resources; persistence/migration; platform/accessibility; performance; and
  integration/license dimensions.
- [ ] **T4.A3.3** Record evidence, confidence, conflicts, and dependency edges.

### A4 — Concepts, Patterns, and Options

- [ ] **T4.A4.1** RED-test single-option conclusions without explicit
  justification and visual-only pattern matching.
- [ ] **T4.A4.2** Produce alternative concepts, reusable patterns, rejected
  options, tradeoffs, capability fallbacks, and reversibility notes.

### A5 — Fresh Herdr Cartography and Fit

- [ ] **T4.A5.1** RED-test stale/absent graph evidence and `ready`-only
  freshness claims.
- [ ] **T4.A5.2** Map current owners, call/data paths, protocol/persistence
  boundaries, existing tests, and reuse candidates.
- [ ] **T4.A5.3** Emit current-versus-target architectural and functional fit.

### A6 — Cross-Dimension Synthesis

- [ ] **T4.A6.1** RED-test unresolved conflicts and unsupported go decisions.
- [ ] **T4.A6.2** Select target architecture, behavior, data flow, fallbacks,
  budgets, and `go`, `conditional_go`, `no_go`, or `blocked` status.

### A7 — Normalized Handoff and Readiness

- [ ] **T4.A7.1** RED-test incomplete traceability, missing decision evidence,
  conditional gaps, and mutable handoff fields.
- [ ] **T4.A7.2** Emit and validate immutable `change-intent-package.json`.
- [ ] **T4.A7.3** Prove native, reference-adapted, composite, no-go, blocked,
  and mid-flight packages through fixtures/evals.
- [ ] **T4.A7.4** Verify that A7 readiness still grants no product mutation.

## T5 — I0-I14 Delivery Engine

- [ ] **T5.I0** Reject absent/invalid handoff, unapproved target, stale current
  state, or missing product authorization; accept mid-flight evidence only
  after provenance and current-phase classification.
- [ ] **T5.I1** Generate PRD, authority checklist, risk register, non-goals,
  rollback, compatibility, and migration obligations.
- [ ] **T5.I2** Refresh graph/repository evidence and detect drift between A7
  handoff and the live target.
- [ ] **T5.I3** Freeze current behavior, target behavior, semantic diff,
  retained behavior, change strategy, and ownership impact.
- [ ] **T5.I4** Build the test-point catalog with `what`, `current`, `expected`,
  `diff`, `result`, and `reason` for every applicable obligation.
- [ ] **T5.I5** Produce dependency-ordered implementation slices, test slices,
  commit boundaries, rollback points, and owned file sets.
- [ ] **T5.I6** Capture characterization evidence before moving behavior or
  architecture.
- [ ] **T5.I7** Require an observed behavior-specific RED; reject compile,
  environment, setup, flaky, or already-green false REDs.
- [ ] **T5.I8** Implement the minimum GREEN change and preserve exact command
  output as evidence.
- [ ] **T5.I9** Refactor only behind green tests; enforce local ownership and
  invariants.
- [ ] **T5.I10** Run cross-layer and cross-feature tests across all applicable
  families.
- [ ] **T5.I11** Verify failure, recovery, security, resources, capability
  negotiation, and degraded behavior.
- [ ] **T5.I12** Verify declared latency, allocation, throughput, memory, queue,
  and terminal-render budgets with calibrated fixtures.
- [ ] **T5.I13** Run complete repository, platform, protocol, migration,
  dependency, docs, and release-cadence gates applicable to the change.
- [ ] **T5.I14** Audit evidence, targeted staging, atomic commits, allowed
  publication, remote SHA, graph reindex, and current-symbol freshness.

## T6 — Cross-Test Families

- [ ] **T6.1** Server/runtime truth versus TUI/client projection.
- [ ] **T6.2** Snapshot/event ordering, revision, replay, duplicate, gap, and
  slow-subscriber behavior.
- [ ] **T6.3** PTY/terminal chunk boundaries, UTF-8/ANSI splits, queue pressure,
  resize, EOF, detach/reattach, and multi-pane throughput.
- [ ] **T6.4** Plugin host timeouts, crashes, output bounds, process cleanup,
  malformed data, version compatibility, and path confinement.
- [ ] **T6.5** Page/layout/component keyboard, mouse, focus, modal, resize,
  Unicode, narrow viewport, empty/loading/error, and terminal fallback states.
- [ ] **T6.6** Persistence interruption, corruption, migration, disk-full,
  concurrent owner, quota, and large-scrollback behavior.
- [ ] **T6.7** Platform isolation and Linux/macOS/Windows policy differences.
- [ ] **T6.8** Performance regression, slow-client isolation, soak, task leak,
  zombie process, and chaos behavior.
- [ ] **T6.9** Backward/forward protocol, old/new client, old/new plugin, and
  old persisted-state compatibility.

## T7 — Adapters and Scenario Fixtures

- [ ] **T7.1** P14 Ratatui/reference-project adapter.
- [ ] **T7.2** Native feature fixture.
- [ ] **T7.3** Mid-flight file-manager feature plus bugfix fixture.
- [ ] **T7.4** Page and interaction-flow fixture.
- [ ] **T7.5** Responsive layout and tiling fixture.
- [ ] **T7.6** Design-system/component/token fixture.
- [ ] **T7.7** Runtime capability and protocol fixture.
- [ ] **T7.8** Composite multi-dimension conflict fixture.
- [ ] **T7.9** Explicit no-go and blocked-MCP fixtures.
- [ ] **T7.10** Unauthorized delivery fixture proving I0 rejection.
- [ ] **T7.11** Verify that native mode invents no repository/source/license and
  reference mode omits no source/provenance/license obligations.

## T8 — Module and Repository Verification

- [ ] **T8.1** Focused schema/validator unit tests.
- [ ] **T8.2** Complete tests for both skills and all negative fixtures.
- [ ] **T8.3** JSON parse, schema, stable-ID, version, and deterministic-output
  checks.
- [ ] **T8.4** Skill validation, README/AGENTS/SKILL consistency, and lesson
  format checks.
- [ ] **T8.5** Eval coverage for A0-A7, I0-I14, adapters, mid-flight adoption,
  blocked, no-go, and unauthorized paths.
- [ ] **T8.6** Legacy P0-P14 backward-compatibility verification.
- [ ] **T8.7** Product isolation and exact diff-boundary verification.
- [ ] **T8.8** Placeholder, whitespace, broken-link, and untracked-artifact
  scans.
- [ ] **T8.9** Proportional `just check` or documented exact equivalent.

## T9 — Git Publication and Codebase Memory

- [ ] **T9.1** Preserve each baseline, RED, GREEN, refactor, governance, fixture,
  and evidence concern in reviewable atomic commits.
- [ ] **T9.2** Target-stage only the declared owned files and verify the staged
  name list before every commit.
- [ ] **T9.3** Fetch and prove fast-forward ancestry before any authorized push.
- [ ] **T9.4** Push only the permitted CyPack feature branch/ref; never
  `upstream`, never force.
- [ ] **T9.5** Verify exact local/remote SHA equality after publication.
- [ ] **T9.6** Reindex Codebase Memory after committed implementation changes.
- [ ] **T9.7** Record node/edge counts and query current pipeline/module symbols;
  never infer freshness from `ready` alone.

## T10 — Pilots, Lessons, and Closure

- [ ] **T10.1** Run one native page/feature request through A0-A7 without
  product mutation.
- [ ] **T10.2** Run one reference project through P0-P14 -> adapter -> A7.
- [ ] **T10.3** Run one mid-flight file-manager feature/bugfix adoption and
  prove existing evidence preservation plus remaining-gate enforcement.
- [ ] **T10.4** Run one composite conflict to a justified conditional-go/no-go.
- [ ] **T10.5** Prove unauthorized I0 rejection and blocked-MCP truthfulness.
- [ ] **T10.6** If separately authorized, run one non-product fixture through
  I14 before using the pipeline on Herdr product code.
- [ ] **T10.7** Record new errors, golden paths, edge cases, and any cross-skill
  lessons in the required tables.
- [ ] **T10.8** Update this registry, `.codex/CURRENT.md`, `.codex/TASKS.md`, and
  handoff state with exact final evidence and next action.
- [ ] **T10.9** Perform final self-review: requirements, tests, failure paths,
  Git state, publication state, graph freshness, and remaining blockers.

## Completion Definition

The canonical pipeline is operational only when T0-T10 are complete, both
skills pass their contract/eval suites, mid-flight adoption is proven, I0
rejects unauthorized product work, compatible P0-P14 behavior remains green,
Git/graph evidence is fresh, and this registry contains no falsely closed item.
