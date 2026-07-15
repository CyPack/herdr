# Herdr Master Integration Intelligence Pipeline

## Contents

1. Purpose
2. What “implementation-ready” means
3. Current Herdr evidence baseline
4. Pipeline boundary and phase graph
5. Roles and ownership
6. P8 — Herdr current-state cartography
7. P9 — Behavior gap and coverage analysis
8. P10 — Data authority, contract and transport map
9. P11 — Cell-level layout fidelity and responsive optimization
10. P12 — Component, input, focus and overlay integration blueprint
11. P13 — Implementation slicing, TDD and cross-test plan
12. P14 — Integration-readiness audit
13. Artifact linkage and traceability
14. Agentic orchestration protocol
15. Failure, retry and stop rules
16. Archer-as-a-page application
17. Completion checklist

## Purpose

P0–P7 answer: “What is this reference project, what does it actually do, and
what can the corpus safely learn from it?”

P8–P14 answer a different and stricter question: “Given Herdr's current code,
runtime boundary and TUI rules, exactly what would be required to reproduce an
approved reference surface as a Herdr page, where would every datum come from,
which behavior already exists, which behavior is missing, how would every cell
be laid out, and which tests must fail and then pass before implementation can
be called correct?”

The output is implementation intelligence, not implementation. This pipeline
does not authorize edits to `src/`, dependency adoption, protocol changes,
stable runtime operations, copying source code, commits or pushes. A separate
user request is required before product work begins.

Canonical machine definition:
`../assets/reference-project-pipeline-v2.json`.

Canonical run state:
`../assets/reference-project-run-template.json`.

Canonical artifact shapes:
`../assets/templates/`.

Refreshable target architecture baseline:
`herdr-layered-architecture-baseline.md`.

## What “implementation-ready” means

Implementation-ready is a traceability property, not a confidence adjective.
A run is ready only when every approved visible behavior can be followed across
this complete chain:

```text
reference evidence
  -> behavior contract
  -> source datum and semantics
  -> Herdr authority decision
  -> API/event/snapshot transport, when shared
  -> client state and action
  -> pure geometry/view projection
  -> component renderer
  -> keyboard/mouse/focus ownership
  -> canonical viewport golden
  -> local test
  -> cross-layer/failure test
  -> implementation slice
```

No link may be replaced by “similar enough,” “probably available,” a screenshot
alone, or a README claim. A missing link is an explicit gap and contributes to
the integration V score.

“Exact 1:1” in a terminal means deterministic terminal-cell equivalence, not
browser pixels:

- the same approved region geometry at the same terminal size;
- the same visible symbols and text after declared glyph mapping;
- the same semantic colors and modifiers after declared capability mapping;
- the same focus, selection, disabled, loading, empty and failure states;
- the same hit regions and input results;
- the same animation frame at the same logical time;
- zero undeclared differences.

Declared adaptations are allowed only when the source technology cannot be
represented literally in terminal cells, Herdr's architecture requires a
different ownership boundary, a terminal capability is absent, or license
constraints require behavior-only reimplementation. Every exception needs a
reason, affected fixtures and acceptance tests.

## Current Herdr evidence baseline

The baseline must be refreshed for every run. The facts below are the current
map at target revision `a61cfb640c315cff89df772acf0b5dec4111319b`; product
paths are clean and the only current worktree addition is this untracked
research skill. A future run must still record revision and relevant local diff
boundary rather than assuming the revision alone describes the target.

| Layer | Current evidence | Integration consequence |
|---|---|---|
| Runtime/session authority | workspace, tab, pane, terminal, process and agent facts are existing shared concepts | reuse neutral runtime concepts; do not create page-named server facts |
| Agent state production | `src/pane.rs::publish_state_changed_event` publishes correctness-critical `AppEvent::StateChanged` through a bounded async channel and waits for queue space | shared live facts require an explicit producer and loss policy |
| Metadata ingestion | `src/app/api/panes.rs::App::handle_pane_report_metadata` validates pane, agent label, source, TTL, presentation text, labels and set/clear conflicts before emitting `HookMetadataReported` | new shared data needs validation, ordering, TTL/staleness and error semantics before UI use |
| Public events | `src/app/api.rs::App::emit_pane_state_update` emits agent-detected and agent-status/presentation changes | use public neutral events for shared facts; do not deepen private TUI coupling |
| Subscriptions | `src/api/subscriptions.rs::ActiveSubscription::new` supports workspace/tab/pane lifecycle, layout, output match, agent status and scroll subscriptions | existing events may feed a page; missing run/phase data must not be inferred from pane status |
| Client state | `src/app/state.rs::AppState` is pure application/client data and `ViewState` stores projected geometry and hit regions | page-local selection, focus, filters, scroll and theme belong here or in a decomposed client model |
| Geometry | `src/ui.rs::compute_view_internal` computes mobile/desktop layout, named shell regions, FM geometry, tab geometry, pane geometry, overlays and hit regions, then snapshots `ViewState` | new page geometry must be computed before render and shared by render/input |
| Rendering | `src/ui/compose.rs::Component` renders from context; `Compositor` draws base and overlay layers | rendering reads state and draws; it does not fetch, mutate, sleep or advance time |
| Input | `src/app/mod.rs::App::route_client_input` routes raw client input through the existing input pipeline | page activation and topmost overlay ownership must be explicit before background dispatch |
| Existing page-like precedent | native FM swaps the center surface through client-local state and uses responsive Miller geometry, prepared models, row/action hit regions and pure rendering | reuse the precedent where it fits, but do not assume a general page router or tiling manager exists |
| Existing cross tests | `tests/cross_area.rs` covers shared view, detach/reattach, server restart/reconnect and API/client consistency | extend cross tests for any new shared data, reconnect or multi-client semantics |

Targeted graph-augmented source searches currently found no product model named
for orchestration pipeline, run ID, todo, token usage or cost. The only `run_id`
source hit explicitly asserts that an integration asset does not report one.
This is negative evidence, not a permanent architectural decision. P10 must
re-run the search and classify each desired datum as existing or absent.

The continuity documentation is rich and evidence-backed, especially
`.codex/CURRENT.md`, `.codex/HANDOFF.md`, `.codex/TASKS.md` and the focused
evidence files. It is not, by itself, the canonical implementation map for a
new reference page: information is organized by delivery checkpoint rather
than by the complete behavior→data→component→test chain. P8 closes that gap for
each pipeline run without rewriting product documentation.

## Pipeline boundary and phase graph

```text
P0 source identity ── P1 source graph ──┬─ P2 runtime/data ─┐
                                       └─ P3 UI/behavior ─┤
P0 ────────────────────────────────────── P4 verification ─┤
                                                           v
                                                 P5 classification
                                                           |
                                                 P6 corpus publication
                                                  /                \
                                      P7 source audit        P8 Herdr map
                                                                  |
                                                        P9 behavior gap
                                                          /             \
                                              P10 data authority   P11 fidelity
                                                          \             /
                                                   P12 integration map
                                                           |
                                                   P13 test/slice plan
                                                           |
                                  P7 source audit -------- P14 readiness audit
```

P10 and P11 intentionally run in parallel after P9. Data authority must not be
chosen from appearance, and layout fidelity must not wait for implementation.
P12 is the merge point: it cannot assign a component until its behavior, data
and geometry contracts exist.

## Roles and ownership

### Herdr cartographer

Owns the current target map. It proves code symbols, layer boundaries, existing
tests and documentation drift. It does not design the desired feature while
mapping current reality.

### Behavior gap analyst

Owns observable behavior equivalence. It treats keyboard, mouse, focus,
loading and failure results as first-class behaviors. It never substitutes
visual resemblance for state-transition evidence.

### Data authority analyst

Owns provenance, semantic meaning, authority, transport, freshness, retention
and failure policy for every datum. It prevents presentation names from leaking
into server contracts.

### Layout fidelity analyst

Owns deterministic source captures, terminal-cell fixtures, region constraints,
tokens, glyph mappings, capability fallbacks, responsive degradation and diff
reports. It does not hide differences behind a percentage score.

### Integration architect

Owns component decomposition and the state/action/view/render/input/focus map.
It must prove pure render and runtime/client boundary compliance.

### Integration test designer

Owns implementation slices, RED tests, cross tests, benchmarks, soak checks,
rollout evidence and rollback. It cannot mark an unexecuted future test passed.

### Evidence auditor

Owns P7 and P14 gates. It validates references and computes V independently of
the producer roles. It cannot weaken a gate to obtain green status.

## P8 — Herdr current-state cartography

### Objective

Produce a fresh, source-backed map of the Herdr layers that the target page
would use or affect. Do not start with desired components. Start with authority,
state transitions, geometry, render and input reality.

### Inputs

- source classification and extraction candidates from P5/P6;
- exact target repository and requested page/feature;
- target revision and worktree status;
- Codebase Memory project key;
- current project rules and continuity/evidence documents.

### Required jobs

#### P8.1 Freeze the target boundary

Record:

- repository absolute path;
- branch and commit;
- dirty paths relevant to the analysis;
- paths that must not be touched;
- stable runtime/socket prohibition;
- product-code authorization, which defaults to false.

Never treat a dirty checkout as the clean commit. Existing modifications belong
to the user and remain outside pipeline ownership.

#### P8.2 Verify the target graph

Use Codebase Memory MCP first:

1. `index_status` or `get_architecture` for project identity and counts;
2. `search_graph` for exact current symbols;
3. `trace_path` for ownership and calls;
4. `get_code_snippet` only after resolving qualified names;
5. graph-augmented `search_code` for negative-evidence terms;
6. local `rg` only for non-code documents, literals or insufficient graph
   results.

`ready` alone is insufficient. Resolve at least one symbol that changed or is
central to the target surface.

#### P8.3 Map the layers

At minimum map:

1. external agent/tool producer;
2. terminal/pane runtime;
3. server-owned shared session state;
4. public schema and API request handling;
5. event hub, subscription, snapshot and reconnect paths;
6. client `App` orchestration and runtime registries;
7. pure `AppState` and page-local state;
8. actions/reducers/scheduled side-effect boundaries;
9. geometry/view computation and hit projection;
10. base/overlay composition and component renderers;
11. keyboard, mouse and focus routing;
12. persistence, restore and migration seams;
13. unit, buffer, integration and cross tests.

For each layer capture its current symbols, inputs, transformations, outputs,
failures, invariants, extension seams and evidence.

#### P8.4 Map state/action/view/render/input ownership

Every state field under consideration must answer:

- Is it a shared runtime fact or TUI presentation fact?
- Who is allowed to mutate it?
- Which action or event causes the mutation?
- Is the effect synchronous, scheduled or async?
- How does geometry project it?
- Which renderer consumes it?
- Which input path refers to the same computed hit region?
- How is stale authority rejected?
- Which invariant test protects it?

#### P8.5 Map lifecycle and recovery

Map startup, activation, refresh, disconnect, reconnect, server restart, page
close, overlay close, terminal resize and application shutdown. Identify
bounded channels, dropped/coalesced events, generation counters, TTL, caches
and cleanup ownership.

#### P8.6 Inventory extension seams

Classify each seam:

- reuse as-is;
- extend without public contract;
- extend public neutral contract;
- create new client-local component;
- incompatible or overloaded;
- absent.

Do not generalize a seam solely for aesthetic symmetry. A second real consumer
or explicit target behavior must justify new abstraction.

#### P8.7 Audit documentation against code

For each architecture claim in relevant docs, label:

- current and code-backed;
- current but missing exact symbol evidence;
- historical checkpoint only;
- contradicted by code;
- insufficient for the requested integration.

The output is `herdr-system-map.json`, not an unstructured prose dump.

### P8 gate

- target revision/worktree boundary recorded;
- target graph fresh with exact symbols;
- all relevant layers mapped;
- current vs desired reality separated;
- no product code or stable runtime touched;
- documentation drift explicit.

## P9 — Behavior gap and coverage analysis

### Objective

Turn the reference application into a complete observable behavior inventory,
then compare each behavior with current Herdr capability.

### Behavior grammar

Every row uses this grammar:

```text
Given <preconditions and state>
when <keyboard, mouse, timer, event or data trigger>
the reference transitions <old state -> new state>
and shows <visible result>
while <focus/input owner> owns interaction;
if <failure or unavailable condition>, it shows/does <failure result>.
```

### Required behavior families

- page entry, exit and return;
- primary navigation and selection;
- keyboard shortcuts and chord precedence;
- mouse click, double click, wheel, drag and hover-like affordances where
  terminal semantics permit them;
- focus movement and restoration;
- scroll anchoring and follow-tail behavior;
- expansion, collapse, split and responsive transformation;
- popup, drawer, tooltip, command palette, confirmation and toast lifecycle;
- loading, skeleton, empty, stale, disconnected, blocked, error and recovery;
- filter, search, sort and grouping;
- status highlights, badges, progress and state changes;
- time-based animation and reduced/idle behavior;
- destructive or externally visible actions;
- resize during interaction;
- data arrival out of order;
- capability fallback.

### Coverage classification

`exact` means current Herdr already has the same observable contract and
ownership.

`partial` means some primitives exist but at least one trigger, transition,
failure state, input path or lifecycle differs.

`absent` means no code-backed capability exists.

`conflicting` means an existing behavior would be violated or ambiguous.

`out_of_scope` means the behavior is intentionally rejected for Herdr, with a
reason.

### Gap analysis rules

1. Match behavior to code, not component name.
2. Record source and Herdr evidence separately.
3. A renderer without input/state lifecycle is not behavior coverage.
4. A pane agent status is not a pipeline phase unless a contract proves that
   semantic equivalence.
5. A periodic redraw is not animation unless logical time changes a frame.
6. A desktop viewport match does not prove responsive behavior.
7. Missing failure behavior is a gap even when the happy path exists.
8. Every accepted gap gets state owner, data/layout dependencies and named
   acceptance tests.

### Prioritization

Rank by:

- user-visible value;
- architectural prerequisite;
- reuse across pages;
- failure and data-integrity risk;
- source-license boundary;
- implementation and verification cost;
- whether the gap blocks faithful layout or only polish.

The output is `behavior-gap-matrix.json`.

### P9 gate

No source behavior is unclassified. No accepted behavior lacks a target action,
owner, dependencies and acceptance tests. Negative and rejected behaviors stay
visible.

## P10 — Data authority, contract and transport map

### Objective

Trace every visible and behavioral datum from its real producer to its terminal
cell consumer, and decide whether Herdr already owns it or needs a new neutral
contract.

### Datum inventory

Inventory text, numbers, enums, timestamps, IDs, progress, status, lists,
hierarchies, selections, filters, derived labels, badges, diffs, costs, usage,
todos, reports, logs, errors, loading flags, freshness and capability flags.

Decorative constants still need a theme/token owner but are not server data.

### Provenance chain

For each datum record:

```text
reference producer
  -> raw shape
  -> validation
  -> normalization/derivation
  -> authoritative state
  -> snapshot/event/poll transport
  -> client cache/state
  -> view projection
  -> component consumer
  -> stale/failure presentation
```

### Authority decision

Use exactly one primary value:

- `server`: shared runtime/session truth needed consistently by multiple
  clients or commands;
- `api_transport`: neutral schema/event representation, never the authority;
- `client_presentation`: selection, focus, scroll, expanded state, animation
  time, filters, local composition and visual tokens;
- `absent`: desired semantic datum has no current producer/contract;
- `out_of_scope`: intentionally not represented.

The API cannot be the original authority. `api_transport` is used when the map
row describes the wire representation itself; the underlying semantic datum
still points to its server or external producer.

### Existing-contract proof

For a claimed existing datum, record:

- exact type/field/function;
- producer path;
- mutation or event path;
- snapshot/query path;
- ordering or sequence behavior;
- client consumer;
- reconnect semantics;
- tests.

If any of these are missing, coverage is partial.

### Missing-contract design

For `absent` data, design before implementation:

- neutral domain name and semantic definition;
- producer and authority;
- stable identity and ordering;
- snapshot shape;
- event delta shape, if needed;
- initial snapshot plus live-event race handling;
- sequence/generation token;
- retry/backoff/coalescing policy;
- TTL and stale behavior;
- disconnect/reconnect recovery;
- retention and maximum size;
- privacy/redaction;
- failure representation;
- compatibility/versioning consequence;
- contract and cross tests.

Never name shared fields after visual surfaces such as card, row, widget,
sidebar or badge.

### Refresh and staleness

Choose one refresh model per datum:

- snapshot on page activation;
- event-driven delta after an initial snapshot;
- bounded poll while visible;
- explicit user refresh;
- static configuration;
- client-derived from already current inputs.

Record how old responses are rejected. Async page loading normally needs a
generation token containing page instance and query/filter revision. A result
for an old generation cannot overwrite the current page.

### Boundedness

Every collection and stream needs a bound or eviction contract:

- maximum records retained;
- maximum text/bytes per record;
- visible-window projection;
- pagination/windowing;
- event channel capacity and overflow behavior;
- cache key and eviction;
- transcript/log tail retention;
- cleanup owner.

The output is `data-authority-map.json`.

### P10 gate

Every datum has provenance, semantic meaning, primary authority, transport,
normalization, refresh, stale, failure, retention, consumer and verification.
Unknown data remains `absent`; it is never synthesized for a mock screenshot.

## P11 — Cell-level layout fidelity and responsive optimization

### Objective

Specify an exact Ratatui-representable layout before implementation, including
responsive rules, terminal capability fallbacks, logical-time motion and
machine-readable golden comparisons.

### Deterministic source capture

For every canonical fixture record:

- pinned source commit;
- launch command and deterministic data fixture;
- terminal width and height;
- terminal capability profile;
- theme/background;
- page state, selection, focus, scroll and overlay stack;
- logical time for motion;
- raw ANSI or cell capture when available;
- screenshot as secondary visual evidence;
- capture timestamp and tool version;
- any nondeterministic values normalized by an explicit rule.

Canonical viewport matrix:

| Profile | Purpose |
|---|---|
| `160x48` | wide desktop, full information hierarchy |
| `120x36` | normal desktop baseline |
| `100x30` | compact desktop and collision exposure |
| `80x24` | traditional terminal baseline |
| `60x20` | narrow degradation |
| `40x12` | tiny-terminal fail-closed behavior |

Add source-specific critical sizes at every breakpoint minus one, exactly at
the breakpoint and plus one.

### Region tree

Derive a named tree, never a list of magic rectangles:

```text
page
├── header
│   ├── identity
│   ├── status
│   └── actions
├── body
│   ├── navigation-or-summary
│   ├── primary-content
│   └── detail-or-inspector
├── footer-or-command-hints
└── overlays
    ├── modal
    ├── palette
    ├── tooltip-or-help
    └── toast
```

Each region records:

- layout direction;
- fixed, percentage, ratio, min and max constraints;
- intrinsic content minimum;
- border and padding consumption;
- alignment;
- clipping/scroll behavior;
- visibility predicate;
- degradation priority;
- hit region;
- z-order and focus scope.

### Exact cell comparison

Use Ratatui `TestBackend` or the project-equivalent deterministic buffer. At
matching viewport/state/time compare:

1. region rectangles;
2. clickable/scrollable hit rectangles;
3. every cell symbol;
4. foreground/background semantic token;
5. modifiers;
6. focus and selection markers;
7. cursor visibility/position where owned;
8. clearing of overlay regions;
9. responsive visibility and ordering.

The diff report names coordinates and expected/actual cell facts. The gate is
zero undeclared differences, not a fuzzy similarity percentage.

### Glyph and color profiles

Every fixture is defined across:

- `truecolor_unicode`;
- `ansi256_unicode`;
- `ansi16_unicode`;
- `no_color_ascii`.

Rounded TUI corners are cell glyphs such as `╭─╮` and `╰─╯`, not pixel radius.
Pill controls use padding, semantic state tokens and complete corner families.
If glyph width/alignment is unreliable, switch the whole component to its
plain/ASCII family. Do not mix fallback corners.

Color cannot be the only state carrier. Focus, selection, warning, error,
disabled and progress retain a glyph, modifier, label or border distinction in
reduced profiles.

### Responsive rules

For every width and height transition define:

- what remains mandatory;
- what shortens;
- what wraps;
- what scrolls;
- what collapses;
- what moves to an overlay/drawer;
- what disappears and in which order;
- minimum usable size;
- explicit too-small presentation.

Never clip interactive controls into partial hit regions. A control is either
fully visible with a complete hit rectangle or absent/disabled.

Responsive optimization is constraint optimization, not indiscriminate density.
Preserve in order:

1. active task and failure truth;
2. focus and current selection;
3. primary action and exit path;
4. navigation context;
5. secondary metadata;
6. decoration.

### Animation and loading

Motion is updated outside render from monotonic elapsed time. Each fixture
samples a fixed `logical_time_ms`. Define:

- start, active, completion and cancellation;
- frame or interpolation function;
- update/tick demand;
- page visibility condition;
- reduced/disabled motion;
- idle tick behavior;
- resize during motion;
- stale async completion behavior.

Do not keep a high-frequency tick active while no time-varying surface is
visible. Do not `sleep`, read files or mutate state inside render.

### Performance contract

Measure before choosing numeric budgets. Record the command, machine profile,
sample count and distribution. Then gate:

- layout work proportional to visible regions/rows, not unbounded history;
- render work proportional to visible cells;
- no I/O or blocking in render;
- bounded allocation/caches;
- no idle high-frequency redraw;
- no material regression from the named baseline without an approved reason.

The output is `layout-fidelity-spec.json`.

### P11 gate

Every canonical source state has deterministic captures, constraints, goldens,
fallbacks and responsive behavior. Every difference is either eliminated or
declared with tests and rationale.

## P12 — Component, input, focus and overlay integration blueprint

### Objective

Decompose the approved page into macro, micro and overlay components and bind
each one to the Herdr ownership model.

### Counting rule

Do not choose an arbitrary component count before analysis.

A macro component is a named region with independent layout/lifecycle or data
loading responsibility.

A micro component is a reusable render/input unit with its own visual-state
contract, intrinsic size or hit behavior.

An overlay is a z-ordered focus scope that can block or coexist with the base
surface.

Split a component when at least one is true:

- different state owner;
- different refresh/lifecycle;
- independently testable geometry;
- independently reusable behavior;
- separate focus/input scope;
- separate failure/degradation behavior.

Do not split pure text fragments that share all ownership and behavior merely
to inflate the component catalog.

### Required component map

Every component records:

- source counterpart and evidence;
- macro/micro/overlay kind;
- direct/adapted/behavior-only/rejected reuse mode;
- license boundary;
- data inputs and datum IDs;
- state owner;
- actions and mutations;
- view geometry function;
- renderer;
- input owner and precedence;
- focus scope and restore target;
- overlay clear/block rules;
- loading, empty, error, stale, blocked and disabled states;
- responsive degradation;
- capability fallback;
- existing Herdr seam or new seam;
- local and cross tests.

### Page lifecycle

Define:

1. activation request;
2. page instance/generation creation;
3. initial shared snapshot request, if any;
4. local loading state;
5. snapshot acceptance or stale rejection;
6. live subscription attachment;
7. input/focus ownership;
8. refresh/filter generation replacement;
9. overlay push/pop and focus restoration;
10. resize projection;
11. page exit and async cancellation/ignore policy;
12. reconnect and rehydration.

### Input precedence

Canonical ordering:

```text
blocking overlay
  -> non-blocking active overlay
  -> active page-local interaction mode
  -> page component
  -> global Herdr shortcut
  -> focused terminal passthrough
```

Every consumed input returns a typed result. Disabled and stale hit targets
consume or reject according to their contract; they never fall through into a
dangerous background action.

### Focus graph

Record focus nodes and legal transitions. Opening an overlay stores a semantic
restore target, not a raw rectangle. Closing restores the prior target only if
it remains valid; otherwise use a deterministic page fallback. Resize must not
leave focus on an invisible component.

### Pure-render proof

For every renderer prove:

- input is `&AppState`/prepared model and computed area;
- no mutation;
- no filesystem/network/process/API access;
- no sleep or clock advancement;
- no async polling;
- no hidden geometry calculation that input cannot share;
- target region is cleared when overlay replacement requires it.

The output is `component-integration-map.json`.

### P12 gate

No component is an orphan: all have behaviors, data, geometry, input/focus,
failure states and tests. No shared runtime fact is owned by a visual component.

## P13 — Implementation slicing, TDD and cross-test plan

### Objective

Produce an executable delivery sequence without writing product code. Every
slice begins with a named failing test, preserves current invariants and ends
with fresh verification evidence.

### Slice order

Prefer this dependency order:

1. characterization tests for protected existing behavior;
2. neutral shared domain types and invariants, only if P10 requires them;
3. server producer/storage, if required;
4. API snapshot/event schema and compatibility, if required;
5. client adapter and generation/stale policy;
6. pure page state/actions/reducers;
7. pure layout/view geometry;
8. static buffer renderer and capability profiles;
9. keyboard/mouse/focus routing;
10. overlays and complex lifecycle;
11. animation/loading update lifecycle;
12. integration and cross tests;
13. benchmarks/soak and manual terminal validation;
14. docs and rollout evidence when the behavior is release-facing.

Client-only pages skip server/API slices, but the skip reason must cite the P10
authority decision.

### TDD contract per slice

```text
RED
  name protected behavior and new behavior
  write the smallest failing test
  run it and record the expected failure

GREEN
  make the smallest production change
  run focused tests and record pass evidence

REFACTOR
  remove duplication without widening behavior
  rerun focused and affected regression tests

CROSS-GATE
  run the named cross/failure/capability tests
  record fresh command, exit code and result
```

### Local test layers

#### State and reducer tests

- valid transitions;
- illegal transitions fail closed;
- generation replacement;
- selection/focus invariants;
- bounded collections;
- serialization/migration only where persisted.

#### Geometry and view tests

- canonical and breakpoint-adjacent sizes;
- zero/tiny rectangles;
- complete disjoint hit areas;
- clipping and scroll normalization;
- same geometry consumed by render and input;
- resize during interaction.

#### Buffer and golden tests

- exact cell content/style;
- focus/selected/disabled/loading/empty/error states;
- Unicode and ASCII profiles;
- truecolor/256/16/no-color profiles;
- overlay clearing;
- fixed logical animation times.

#### Input and focus tests

- keyboard chords and precedence;
- mouse cells at inside/outside/boundary;
- wheel and drag cancellation;
- top overlay blocks background;
- close restores valid semantic focus;
- stale hit geometry has no authority.

### Cross-test layers

#### Adapter and contract

- producer→normalization→snapshot;
- snapshot plus concurrent event race;
- event ordering/sequence;
- malformed and oversized payload rejection;
- TTL/stale/clear behavior;
- generation-safe async completion.

#### API and event consistency

- API snapshot agrees with event-reduced client state;
- client view agrees with shared authority;
- reconnect obtains a coherent fresh snapshot;
- protocol compatibility expectations are explicit.

#### Multi-client isolation

- shared facts converge for two clients;
- page-local selection/focus/filter does not leak;
- one client disconnect does not stop runtime authority;
- late subscriber gets correct initial state.

#### Failure and recovery

- producer missing;
- server restart;
- disconnect/reconnect;
- slow consumer and bounded queue;
- stale response after page close;
- partial data;
- permission/config/capability failure;
- resize while overlay or drag is active.

#### Capability and platform

- canonical terminal profiles;
- Unix/Windows compile gates for touched platform behavior;
- narrow and short terminals;
- Unicode width fallback;
- no-color semantics;
- mouse unavailable/disabled behavior.

#### Benchmark and soak

- layout/render baseline distribution;
- idle redraw/tick behavior;
- long feed/history boundedness;
- repeated open/close cleanup;
- reconnect loop resource stability;
- cache eviction.

### Test count rule

Do not announce an impressive fixed test count. Generate tests from the matrix:

```text
minimum test obligations =
  accepted behavior rows
  x applicable state/failure variants
  + canonical/breakpoint geometry fixtures
  + capability fallbacks
  + every changed cross-layer contract
```

Deduplicate only when one test genuinely proves multiple traceable obligations.
The verification artifact records those links.

### Rollout and rollback

Each slice records:

- feature activation path;
- observability signals;
- safe fallback/degraded state;
- rollback boundary;
- persisted/protocol compatibility consequence;
- manual verification requirement;
- exact completion evidence.

The outputs are `implementation-plan.json` and
`integration-verification.json`.

### P13 gate

Every slice has dependencies, ownership, protected behavior, named RED tests,
minimal change boundary, local tests, cross tests, risks, rollback and fresh
completion evidence requirements.

## P14 — Integration-readiness audit

### Objective

Independently prove that the analysis is complete enough to begin a separate
implementation task without rediscovering architecture or silently inventing
data/layout behavior.

### Schema and reference audit

Validate:

- all JSON parses;
- artifact type and record contracts;
- source IDs and commits agree;
- target revision and graph project agree;
- every referenced behavior, datum, component, fixture, test and slice exists;
- no dangling evidence IDs;
- no duplicate stable IDs.

### Requirement traceability audit

For every user requirement, prove at least one path through:

```text
requirement
  -> source evidence
  -> behavior row
  -> datum rows
  -> layout fixtures
  -> component rows
  -> acceptance/local/cross tests
  -> implementation slices
```

Decorative requirements may not need shared data but still require tokens,
fixtures, fallbacks and buffer tests.

### Fidelity audit

- every canonical viewport/state/profile has a fixture;
- source captures are deterministic or explicitly bounded;
- zero undeclared geometry/hit/content/style differences;
- declared adaptations have rationale and tests;
- animations use fixed logical time;
- tiny-terminal and fallback profiles are covered.

### Architecture audit

- shared facts are server-owned or explicitly external;
- API is transport, not UI authority;
- page-local interaction stays client-local;
- render is pure;
- geometry and input share hit regions;
- overlay precedence and focus restore are explicit;
- async data is generation-safe;
- collections, caches and channels are bounded;
- no new dependency or protocol change is assumed without a slice and gate.

### Integration V score

Count unresolved units:

```text
V =
  unverified target components
  + open/below-threshold claims
  + unresolved behavior decisions
  + unowned or absent data without approved action
  + undeclared fidelity differences
  + components missing ownership/tests
  + implementation slices missing RED or cross gates
  + dangling artifact references
```

`passed` requires V=0. If useful implementation intelligence exists but V>0,
status is `partial` with exact blockers. MCP absence makes graph-dependent
closure `blocked`. Test or schema failure is `failed` until corrected.

### Isolation audit

Before closure compare product-code status with the frozen P8 boundary. The
pipeline may add or edit only corpus/skill/research artifacts. It must not
attribute pre-existing user changes to itself and must not operate stable
Herdr/socket state.

The output is the final `run.json`, validation records and an integration-ready
handoff or explicit blockers.

## Artifact linkage and traceability

Stable ID prefixes:

| Artifact | Prefix | Example |
|---|---|---|
| source claim | `CLM` | `CLM-ARCHER-UI-001` |
| behavior | `BEH` | `BEH-ARCHER-PAGE-001` |
| datum | `DAT` | `DAT-ARCHER-RUN-STATUS` |
| Herdr layer | `LYR` | `LYR-HERDR-API-EVENTS` |
| component | `CMP` | `CMP-ARCHER-PHASE-LIST` |
| layout fixture | `FIX` | `FIX-ARCHER-120X36-ACTIVE` |
| test | `TST` | `TST-ARCHER-RECONNECT-001` |
| implementation slice | `SLI` | `SLI-ARCHER-CLIENT-STATE` |

Every artifact stores IDs rather than copying prose as its only connection.
The auditor should be able to build a directed traceability graph and report
orphans.

## Agentic orchestration protocol

The orchestrator instantiates `reference-project-run-template.json`, assigns
roles and advances only when dependencies are terminal.

### Work packets

Every worker packet includes:

- run ID and exact phase/job IDs;
- source and target revisions;
- allowed paths;
- forbidden product/runtime operations;
- required inputs;
- exact output artifact/record ownership;
- MCP requirements;
- evidence and confidence contract;
- gate and stop rules.

### Safe parallelism

Allowed:

- P1 source graph and P4 source verification after P0;
- P2 runtime/data and P3 UI/behavior after P1;
- P7 source audit and P8 target mapping after P6;
- P10 data authority and P11 layout fidelity after P9;
- independent records within one artifact when stable IDs and ownership do not
  overlap.

Not allowed:

- P12 before both P10 and P11 finish;
- P13 before component ownership is complete;
- two workers editing the same artifact records without an assigned partition;
- a worker changing product code to “confirm” an analysis;
- graph claims from a worker that did not successfully call MCP.

### Merge protocol

1. Validate each worker artifact structurally.
2. Resolve duplicate IDs and contradictory claims explicitly.
3. Cross-link behavior, datum, fixture and component IDs.
4. Run artifact validators.
5. Recompute V.
6. If V is nonzero, route exact unresolved units back to their owner.
7. Preserve attempts and failures in run state.

One agent may execute all roles in a small run; the same phase and evidence
boundaries still apply. “Agentic” describes explicit ownership, dependency and
machine-verifiable handoffs, not mandatory parallelism.

## Failure, retry and stop rules

Retry only when new data or a transient condition justifies it:

- network fetch failure;
- rate limit;
- MCP readiness transition;
- source-owned dependency mirror failure;
- deterministic tool timeout with a narrower safe retry.

Do not retry unchanged:

- missing license;
- absent product capability;
- failing test;
- schema contradiction;
- product-code isolation violation;
- unsupported terminal capability.

Stop and report when:

- source or target identity cannot be pinned;
- Codebase Memory MCP is unavailable for graph-required phases;
- source execution requests credentials or production data;
- existing user work would be overwritten;
- a data authority decision requires product strategy not provided by the user;
- exact fidelity is impossible without an unapproved adaptation;
- three systematic attempts leave the same unknown blocker.

## Archer-as-a-page application

For Archer, P0–P7 already provide the source-system, component and data-flow
evidence package. Applying P8–P14 means:

1. map the current Herdr target revision and page seams;
2. enumerate every Archer behavior/state from the source evidence, not memory;
3. compare each behavior with Herdr primitives;
4. list every displayed Archer datum and trace its real source;
5. mark orchestration-specific run/phase/todo/usage/cost/report facts `absent`
   unless the fresh Herdr graph proves a neutral current authority;
6. design neutral shared contracts only for approved shared facts;
7. keep page selection, expansion, scroll, focus, overlay and animation state
   client-local;
8. capture Archer at canonical viewports and translate its visual language into
   explicit Ratatui cells/tokens/fallbacks;
9. decompose the page only after behavior, data and layout contracts converge;
10. generate implementation slices and tests from the matrices;
11. finish only when P14 traceability and V=0 pass.

The result is not “copy Archer into Ratatui.” It is a complete, evidence-backed
Herdr implementation blueprint that preserves Archer's approved behavior and
layout while respecting Herdr's runtime/client and pure-render architecture.

## Completion checklist

- [ ] P0–P7 source intelligence passed or has explicit accepted limitations.
- [ ] Target revision, dirty boundary and stable-runtime prohibition recorded.
- [ ] Fresh Herdr graph resolves exact target symbols.
- [ ] Herdr layers, dependencies, failures, tests and extension seams mapped.
- [ ] Every source behavior classified against current Herdr.
- [ ] Every desired datum has provenance and authority.
- [ ] Missing data has an approved neutral contract or remains a blocker.
- [ ] Canonical and breakpoint-adjacent viewport captures exist.
- [ ] Region constraints, hit regions, tokens and fallbacks are explicit.
- [ ] Zero undeclared cell-level fidelity differences remain.
- [ ] Macro, micro and overlay components have complete ownership maps.
- [ ] Input precedence and focus restoration are deterministic.
- [ ] Loading, empty, stale, disconnected, blocked, error and recovery exist.
- [ ] Every implementation slice starts with a named RED test.
- [ ] Local, cross, capability, benchmark and soak obligations are linked.
- [ ] Artifact schemas and stable references validate.
- [ ] Source and integration maps both have V=0.
- [ ] Product-code diff remains exactly at the frozen user-owned boundary.
- [ ] Stable Herdr/socket/processes were untouched.
