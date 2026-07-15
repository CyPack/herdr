# Herdr Layered Architecture Baseline for Reference-Page Integration

## Contents

1. Status and evidence stamp
2. Architectural invariants
3. Layer map
4. Existing end-to-end data flows
5. Page, mode and overlay reality
6. Layout and responsive reality
7. Input, focus and hit-authority reality
8. Persistence, reconnect and multi-client reality
9. Test architecture
10. Proven missing or insufficient concepts
11. Reference-page decision table
12. Refresh protocol

## Status and evidence stamp

This is the canonical analysis baseline for P8 of the reference-project
pipeline. It is not public release documentation and does not authorize product
changes.

Evidence stamp:

- target repository: `/home/ayaz/projects/herdr`;
- target branch at analysis time: `feat/native-fm`;
- target commit: `a61cfb640c315cff89df772acf0b5dec4111319b`;
- Codebase Memory project: `home-ayaz-projects-herdr`;
- graph at final freshness check: 19,534 nodes / 91,017 edges;
- freshness symbol: `src/app/worktrees.rs::install_focused_agent_worktree_launcher` from the current product head;
- worktree: product paths clean; untracked research skill only;
- stable Herdr process/socket: not touched;
- product code: not changed by this analysis.

Every future P8 run must refresh this baseline. Exact symbol evidence is more
authoritative than this document if they drift.

## Architectural invariants

1. `AppState` is pure data and testable without PTYs or async runtime.
2. Pane state is separate from pane runtime.
3. Shared runtime/session facts belong to server-owned state and neutral public
   API/event paths when practical.
4. Page selection, local filters, focus, scroll, overlay and visual tokens are
   TUI/client presentation state.
5. `compute_view` owns geometry, normalization and hit projection.
6. Rendering consumes `&AppState` and draws without mutation or I/O.
7. Input acts on the same computed geometry; stale rectangles have no authority.
8. Platform-specific implementation remains isolated.
9. Agent detection reads evidence snapshots and publishes state; it does not
   make the renderer authoritative.
10. New reference pages must not deepen private client-socket coupling.

## Layer map

### L0 — External producers and control clients

Responsibilities:

- coding-agent processes and their terminal output;
- integration hooks that report neutral agent metadata;
- CLI, API and attached TUI clients;
- terminal host capability and input bytes.

Integration rule:

An external reference application is never imported as authority. Its behavior
and data semantics are mapped to current or proposed Herdr contracts.

### L1 — Pane and terminal runtime

Primary modules:

- `src/pane.rs`;
- `src/terminal/`;
- `src/terminal_runtime.rs` and runtime registry seams;
- `src/detect/`.

Verified symbol:

- `src/pane.rs::publish_state_changed_event`.

It publishes `AppEvent::StateChanged` with pane identity, agent, detected
state, visible blocker/working signals, process-exit fact and observation time.
The async detector task waits for bounded channel space instead of dropping a
correctness-critical state transition, while PTY I/O remains unblocked.

Integration consequence:

- raw terminal output is evidence, not page state;
- a desired page datum needs an explicit producer or derived-state rule;
- slow-consumer and delivery behavior must be defined;
- UI components cannot inspect mutable parser/runtime internals during render.

### L2 — Server-owned session/runtime state

Responsibilities:

- workspace/tab/pane organization;
- terminal/process lifecycle;
- shared agent facts;
- client connection and session survival;
- server event handling and frame/client coordination.

Primary modules include `src/server/`, `src/workspace.rs`, `src/pane.rs` and
headless runtime integration.

Integration consequence:

A fact belongs here when multiple clients, CLI commands or reconnects must
observe the same truth. Shared types use neutral domain names such as run,
phase, activity or report only if those concepts are approved domain facts.
They must not be called sidebar rows, status cards or widgets.

### L3 — Public schema, requests, events and subscriptions

Primary modules:

- `src/api/schema/`;
- `src/api/event_hub.rs::EventHub`;
- `src/api/subscriptions.rs`;
- `src/api/client.rs`;
- request handlers under `src/app/api/`.

Verified symbols:

- `src/api/schema/events.rs::EventKind`;
- `src/api/event_hub.rs::EventHub`;
- `src/api/subscriptions.rs::ActiveSubscription::new`;
- `src/app/api/panes.rs::App::handle_pane_report_metadata`;
- `src/app/api.rs::App::emit_pane_state_update`.

Current subscription families include workspace/tab/pane lifecycle, worktrees,
layout update, pane output matching, agent status/presentation changes and pane
scroll changes.

`handle_pane_report_metadata` demonstrates the current ingestion contract:

- resolve public pane identity;
- normalize optional agent label;
- validate metadata source;
- normalize title, display-agent and custom-status text;
- validate state labels;
- validate TTL;
- reject simultaneous set and clear;
- reject empty operations;
- emit an internal typed event.

`emit_pane_state_update` converts internal agent/presentation transitions to
neutral public event envelopes.

Integration consequence:

- the API transports shared facts but is not their original authority;
- every new shared datum needs snapshot semantics and, when live, event
  semantics;
- initial-snapshot/live-event races, ordering and reconnect must be tested;
- version/schema consequences must be explicit before implementation.

### L4 — App orchestration and side-effect boundary

Primary modules:

- `src/app/mod.rs`;
- `src/app/actions.rs` and action-specific modules;
- `src/app/api.rs` and `src/app/api/`;
- runtime registries owned by `App` rather than `AppState`.

Verified symbols:

- `src/app/api.rs::App::handle_internal_event`;
- `src/app/mod.rs::App::route_client_input`.

`handle_internal_event` is a high-connectivity transition junction. It receives
typed internal events, updates state/runtime coordination, emits public events
where appropriate and marks rendering dirty. New page work should not turn it
into a page-specific god switch; new behavior must be decomposed by authority.

Scheduled/revalidated side effects belong at this layer. Input may create a
typed intent, but filesystem/process/API work occurs after current identity and
authority are revalidated.

Integration consequence:

- client adapters and async loaders live outside pure state/render;
- stale async completion uses page/query generation identity;
- page close invalidates or ignores in-flight work deterministically;
- page-local actions do not mutate shared runtime facts directly.

### L5 — Pure application and presentation state

Primary module:

- `src/app/state.rs`.

Verified symbols:

- `src/app/state.rs::AppState`;
- `src/app/state.rs::ViewState`;
- `src/app/state.rs::Mode`.

`AppState` is explicitly documented as testable without PTYs or Tokio.
`ViewState` is the computed geometry/hit snapshot used by rendering and input.

`Mode` is currently an interaction/overlay mode enum. It must not be assumed to
be a complete page router. A new page strategy must compare:

- center-surface state similar to native FM;
- a dedicated page enum/router;
- a tab-owned surface;
- an overlay mode;
- a composable component registry.

The choice is made from real lifecycle and second-consumer evidence, not from
the reference application's navigation terminology.

Integration consequence:

Client-local page state can own:

- current subpage;
- selected item;
- expanded/collapsed groups;
- scroll and follow-tail state;
- filters and search query;
- active focus target;
- overlay state;
- logical animation time;
- current load generation;
- prepared bounded presentation records.

It must not own PTYs, file handles, sockets, tasks or authoritative shared
runtime facts.

### L6 — Geometry and view projection

Primary modules:

- `src/ui.rs`;
- `src/ui/shell.rs`;
- component-specific pure geometry helpers.

Verified symbols:

- `src/ui.rs::compute_view_internal`;
- `src/ui/shell.rs::ShellLayout`;
- `src/ui.rs::sync_file_manager_view`.

`compute_view_internal` currently:

1. chooses mobile or desktop projection;
2. computes sidebar width/collapse behavior;
3. computes named shell regions;
4. derives center tab/terminal areas;
5. synchronizes file-manager row/action/header geometry;
6. normalizes sidebar/project/agent scroll;
7. computes tab hits and split borders;
8. computes pane information;
9. computes attachment/worktree/picker geometry;
10. computes toast hit area;
11. replaces the complete `ViewState` snapshot.

The function mutates projected/normalized view state before render; render
itself stays pure. Page integration should avoid making this junction grow
without decomposition. A page-specific pure layout function can return a
typed view model that `compute_view` stores.

Integration consequence:

- every visible interactive region has a stable semantic ID and rectangle;
- rendering and mouse dispatch consume the same snapshot;
- responsive degradation is computed, not improvised during drawing;
- hidden/partial controls have no hit authority;
- canonical geometry can be unit-tested without a terminal runtime.

### L7 — Render and composition

Primary modules:

- `src/ui.rs`;
- `src/ui/compose.rs`;
- `src/ui/panes.rs`, sidebar and modal helpers.

Verified symbols:

- `src/ui.compose.Component`;
- `src/ui.compose.Compositor`;
- `src/ui.render_with_runtime_registry`.

`render_with_runtime_registry` receives `&AppState`, runtime registry and frame,
builds a render context, and composes `BaseLayer` followed by `OverlayLayer`.
The compositor's later component overwrites earlier overlapping cells.

Current reality is a small explicit composition seam, not yet proof of a fully
dynamic page/component registry. A new abstraction requires multiple real
consumers and lifecycle pressure.

Integration consequence:

- render reads prepared state and computed areas;
- overlays clear/replace their target region;
- theme tokens are semantic;
- no filesystem/API/task/clock work occurs here;
- exact `TestBackend` buffers are the primary layout oracle.

### L8 — Input, mouse, focus and overlay ownership

Primary modules:

- `src/app/input/`;
- `src/app/mod.rs::App::route_client_input`;
- page/feature-specific input modules;
- computed `ViewState` hit areas;
- modal/focus helpers in app/UI modules.

Input authority is temporal and layered. A canonical reference page should
define:

```text
blocking overlay
  -> active non-blocking overlay
  -> page-local interaction mode
  -> page component
  -> global Herdr shortcut
  -> terminal passthrough
```

Integration consequence:

- typed dispatch results distinguish not-handled, consumed and scheduled
  action;
- disabled controls cannot fall through into the terminal;
- stale/hidden hit areas cannot trigger actions;
- closing overlays restores semantic focus only when still valid;
- resize reprojects focus rather than retaining raw coordinates.

### L9 — Persistence, snapshots and restoration

Verified symbols:

- `src/persist/snapshot.rs::SessionSnapshot`;
- `src/api/schema/session.rs::SessionSnapshot`.

Persistent session state and public session snapshot are distinct types and
must not be conflated. Before persisting any page state ask:

- is it required to restore shared runtime/session organization?
- is it harmless local presentation preference?
- can it be recomputed from authority?
- what migration/version behavior is required?
- what stale identities can it retain?

Default for transient reference-page selection, filter, focus, overlay and
animation state is non-persistent client state unless a user requirement proves
otherwise.

### L10 — Verification architecture

Existing test surfaces include:

- unit tests beside state, actions, geometry and render code;
- Ratatui buffer/layout tests;
- API schema/server/subscription/wait tests;
- headless runtime tests;
- `tests/cross_area.rs` cross-boundary tests;
- platform compile/lint gates;
- isolated throwaway runtime/manual PTY validation for release-risk behavior.

Current cross-area evidence includes detach/reattach, agent survival, two
clients sharing view, server restart/reconnect and client/API consistency.

Integration consequence:

A new shared datum extends API/event and reconnect tests. A client-local page
still needs geometry, buffer, input, focus, capability and cleanup tests but
does not need a server contract solely for visual composition.

## Existing end-to-end data flows

### Agent detection to public presentation

```text
terminal/detector evidence
  -> publish_state_changed_event(AppEvent::StateChanged)
  -> App::handle_internal_event
  -> pane/workspace state transition
  -> App::emit_pane_state_update
  -> EventHub/EventKind::PaneAgentStatusChanged
  -> subscription/client snapshot
  -> AppState/ViewState
  -> sidebar/pane render
```

This flow is a valid source for neutral agent status/presentation. It does not
prove run phase, todo, cost or usage semantics.

### Hook metadata to public presentation

```text
integration hook/API request
  -> handle_pane_report_metadata validation and normalization
  -> AppEvent::HookMetadataReported
  -> internal state/presentation update
  -> public pane status/presentation event
  -> client render
```

This flow demonstrates source, TTL, sequence and set/clear patterns that a new
contract may reuse conceptually. It must not be overloaded with unrelated page
data merely because it already reaches the UI.

### Client state to interactive cells

```text
AppState/domain state
  -> compute_view/page layout helper
  -> ViewState rectangles and hit regions
  -> render_with_runtime_registry/Component
  -> terminal cells

mouse/key bytes
  -> route_client_input/input precedence
  -> ViewState hit authority
  -> typed action/intent
  -> pure state transition or scheduled revalidated side effect
```

This is the canonical pattern for reference-page components.

## Page, mode and overlay reality

Current evidence supports:

- one explicit base/overlay compositor;
- `Mode` for interaction/overlay states;
- center-surface replacement precedent through native FM;
- reusable modal-shell/focus patterns;
- tabs and panes as shared session organization.

Current evidence does not by itself prove:

- a general arbitrary page registry;
- unlimited nested overlay stack ownership;
- a general free docking/tiling page manager;
- persisted page navigation history;
- a plugin-delivered Ratatui component runtime.

P12 must choose the smallest architecture that satisfies the reference page and
at least one justified reuse seam.

## Layout and responsive reality

Herdr has named shell regions and responsive mobile/desktop computation. Native
FM has bounded responsive Miller projection. Existing pane splits are runtime
session layout, not automatically a page-local dashboard tiler.

Do not conflate:

- Miller parent/current/preview columns;
- workspace pane split layout;
- responsive dashboard regions;
- arbitrary docking/tiling manager.

An Archer-like page can use a pure page-region tree without first generalizing
all Herdr layout. If the desired behavior includes user-resizable/swap/dock
regions, P9 must identify those behaviors and P12 must justify a dedicated
client-local split-tree model.

## Input, focus and hit-authority reality

Existing design repeatedly prepares exact hit rectangles in view computation
and revalidates current state before scheduled actions. This is the preferred
pattern for buttons, pills, rows, tabs, resize handles and overlays.

Rounded buttons remain terminal-cell illusions. Their clickable rectangle
includes the complete padded control; an incomplete narrow fallback is hidden
or replaced, never partially active.

## Persistence, reconnect and multi-client reality

The server survives client disconnect and exposes shared session snapshots and
events. A new shared page datum must define:

- initial snapshot;
- event ordering;
- late subscriber behavior;
- reconnect recovery;
- server restart semantics;
- two-client convergence;
- local presentation isolation.

Client-local page state may reset on reconnect if explicitly designed, but it
must never silently overwrite shared runtime truth.

## Test architecture

Minimum categories for a new reference page:

1. pure state/reducer invariants;
2. geometry at canonical and adjacent breakpoints;
3. exact buffer goldens across capability profiles;
4. keyboard/mouse boundary and precedence;
5. focus/overlay lifecycle;
6. async generation and stale-result rejection;
7. API/event/reconnect when shared data changes;
8. two-client shared/local isolation;
9. bounded history/cache/channel behavior;
10. performance baseline and idle tick;
11. platform/capability fallback;
12. isolated manual PTY only when automated evidence cannot prove host behavior.

## Proven missing or insufficient concepts

At the evidence stamp, targeted graph-augmented source searches found:

- no orchestration `Pipeline` product type;
- no orchestration `RunState`/`RunMetadata` product type;
- no product `run_id` authority; the sole `run_id` hit asserts an integration
  asset does not contain it;
- no `todo` product model;
- no `token usage` product model;
- no cost model corresponding to agent usage;
- no evidence that `Mode` is a complete page router;
- no evidence that the current compositor is a dynamic component registry.

These are snapshot-scoped negative claims. P8/P10 must re-run exact searches.
They may become approved new contracts, client-derived facts, rejected scope or
remain blockers. They must never be mocked as real Herdr data in an
implementation-ready plan.

## Reference-page decision table

| Need | Default Herdr location | Escalation trigger |
|---|---|---|
| selected row/filter/expanded group | client page state | persistence explicitly required |
| scroll/focus/overlay/logical animation time | client presentation state | none; never server merely for synchronization |
| agent status/title/custom status | existing shared pane/API facts | reference semantics exceed existing meaning |
| run/phase/todo/report/cost/usage | absent until P10 proves authority | approved producer and multi-client/runtime requirement |
| page geometry/hit regions | pure page view projected by `compute_view` | second consumer justifies broader shell abstraction |
| visual tokens/rounded borders | client theme/component renderer | terminal capability needs fallback profile |
| pane/process action | App scheduled authority or neutral server/API | shared command/reconnect consistency required |
| reference-only behavior | reimplemented from contract/tests | license permits direct reuse and compatibility is proven |

## Refresh protocol

At the start of every P8:

1. record target commit and dirty paths;
2. get graph architecture and status;
3. resolve `AppState`, `ViewState`, `compute_view_internal`,
   `render_with_runtime_registry`, `route_client_input`, `EventHub`,
   `ActiveSubscription::new` and target-specific symbols;
4. trace data flows relevant to the requested page;
5. run negative searches for every desired semantic datum;
6. compare continuity/architecture docs with current code;
7. update `herdr-system-map.json` with evidence and drift;
8. do not edit this baseline merely to make a desired design appear native;
9. pass product-isolation and graph-freshness gates before P9.
