# Native File Manager Files Workspace Design

## Status

- Design date: 2026-07-15
- Adoption mode: `mid_flight_adoption`
- User-selected option: **A — Files Workspace**
- Selection evidence: terminal approval plus final browser choice
  `files-workspace` after comparing the Files Workspace and Surface Tab options
- Primary modes: `behavior_correction`, `layout`, `interaction_flow`
- Supporting mode: `page`
- Product-code authorization: true for the existing native file-manager feature
  and its reported UX corrections
- Product implementation status: not started by this design
- Delivery status: A6 target selected; A7 written handoff under review; I0 has
  not yet authorized an implementation slice

## Executive Decision

The native file manager will become a concrete client-owned **Files Workspace**.
It will no longer be rendered as a replacement for only the terminal canvas
while terminal chrome and background hit targets continue to exist around it.

When open on desktop, Files Workspace owns the complete named
`RegionId::CenterContent` region, including the row currently used by the
terminal tab bar. The regular terminal tab bar is not rendered and exposes no
hit geometry. Files Workspace renders its own page header, breadcrumb,
responsive Miller column viewport, horizontal scrollbar, action surfaces, and
status surface. The existing Files sidebar remains visible and interactive.

Terminal processes, pane runtime state, agent state, and session organization
continue to be server/runtime truth and remain alive. They are not rendered as
the active center surface and receive no keyboard or mouse input while Files
Workspace owns the client page. Opening or closing the page creates or destroys
no terminal, pane, process, or server resource.

This is a concrete page implementation. It does not authorize a general page
registry, component registry, persisted shell tree, popup stack rewrite, or
retained per-directory cursor/back-forward history.

## User-Reported Product Delta

The existing native file manager already provides verified parent/current/
preview rendering, live watching, bounded text and image previews, selection,
context actions, safe file operations, agent handoff, and Files sidebar
navigation. The following observed UX gaps are new product evidence:

1. Parent and directory-preview columns render useful context but do not accept
   mouse row interaction.
2. Repeated directory navigation replaces a fixed parent/current/preview
   projection instead of creating the spatial, horizontally navigable column
   trail expected from Finder-style Miller navigation.
3. Column separators cannot be dragged to resize the adjacent columns.
4. The file manager feels like a curtain over an existing terminal surface.
   The terminal tab bar, pane geometry, notifications, and non-center input
   paths are still computed while only the terminal canvas is swapped.
5. The product lacks one explicit page-level input owner that defines which
   background surfaces remain permitted while Files Workspace is active.

The right preview changing with the current selection is accepted existing
behavior and must be preserved.

## Preserved Mid-Flight Work

The design adopts rather than restarts the current branch.

- Branch: `feat/native-fm`
- Adoption checkpoint: `eacd32b`
- Published product checkpoint: `d1c287a`
- Worktree product diff: clean at adoption
- Local planning state: four documentation commits ahead of
  `origin/feat/native-fm`
- Existing untracked concern:
  `.codex/skills/ratatui-design-intelligence/`
- Visual companion artifacts: `.superpowers/`; local-only and excluded from
  every product/documentation commit
- Latest preserved full product gate: 3202/3202 nextest with only the named B0
  real-host probe skipped, Linux and canonical Windows clippy, Bun 17/17,
  Python 64/64, format and diff checks clean
- Latest preserved product graph: 19,534 nodes / 91,017 edges; current snippets
  for `miller_layout`, `handle_file_manager_mouse`, `FmState`,
  `BaseLayer::render`, and `compute_view_internal` were re-read during adoption

Historical RED/GREEN evidence remains valid only for the behavior it originally
proved. This design does not invent missing historical RED evidence and does not
reinterpret compile, setup, environment, or flaky failures as behavioral RED.

## Retrospective A0-A7 Adoption Package

### A0 — Intake, Mode, and Evidence Boundary

The change identity is the user-approved Files Workspace correction to the
existing native file manager. The primary authority is direct user observation
against the current product. Local project rules, current source, current graph
snippets, preserved test evidence, and Ratatui reference guidance are supporting
evidence.

Authorized product scope:

- concrete Files Workspace composition;
- exclusive page input ownership;
- mouse interaction for prepared Miller directory columns;
- bounded horizontally scrollable Miller path projection;
- FM-local separator resizing;
- required state, geometry, input, render, lifecycle, failure, performance,
  platform, and verification tests.

Explicit non-goals:

- S5 general `ComponentRegistry` or general page registry;
- S6 persisted `ShellLayout` tree or cross-session shell migration;
- S7 general popup ownership stack;
- N2.2 per-directory cursor history or generic back/forward history;
- server protocol, private TUI socket, shared runtime identity, or session
  snapshot changes;
- new dependency, external framework adoption, upstream issue/PR, release
  documentation, or stable Herdr installation;
- unrelated refactor or speculative Files/Projects/Spaces redesign.

### A1 — Goals, Actors, Scenarios, and Success

Primary actor: a Herdr user navigating project files with mouse and keyboard
while terminal agents continue running in the same session.

Primary scenario:

1. The user activates Files Workspace from the existing Files surface or
   configured native-FM action.
2. The center becomes one coherent Files page; terminal tab chrome and pane
   targets are absent.
3. The user navigates the Files sidebar or any prepared directory column.
4. Each directory choice advances the Miller path and keeps the active path
   visible in a horizontal viewport.
5. The user drags complete separators to resize adjacent columns without
   mutating ShellLayout or runtime/session state.
6. The user closes Files Workspace or switches to Spaces/Projects; terminal
   rendering resumes against unchanged runtime resources.

Success is measurable when:

- no hidden terminal pane, terminal tab, split border, pane scrollbar, agent
  frame action, or terminal mouse-reporting path receives input while the page
  is active;
- Files sidebar navigation and topmost Files modal/context input remain usable;
- parent/current/directory-preview rows use current exact path identity and
  reject stale geometry;
- repeated descent creates a bounded navigable path projection and auto-keeps
  the active path visible;
- separator drag is clamped, cancelable, resettable, and valid at every tested
  width;
- render performs no filesystem, socket, process, config, or worker I/O;
- closing/reopening cannot accept a stale watcher, preview, row, chain, scroll,
  or drag generation;
- the complete repository gate is fresh and green.

### A2 — Fractal Decomposition

```text
FW initiative: coherent Finder-like native file workspace
  -> FW-PAGE: Files Workspace client page
     -> FW-PAGE-REGION: complete CenterContent ownership
        -> FW-PAGE-HEADER: Files identity, breadcrumb, close/help affordance
        -> FW-PAGE-INPUT: exclusive page input boundary
        -> FW-PAGE-ROUTE: Files sidebar and Spaces/Projects transition
     -> FW-MILLER: bounded Miller path projection
        -> FW-MILLER-COLUMN: prepared directory snapshot and selection identity
        -> FW-MILLER-VIEWPORT: visible column window and horizontal scroll
        -> FW-MILLER-MOUSE: parent/current/preview/path-column row activation
        -> FW-MILLER-RESIZE: adjacent column divider gesture
     -> FW-PREVIEW: existing file/directory/text/image projection
     -> FW-ACTIONS: existing header/row/context/status owners
     -> FW-RECOVERY: watcher, path, permission, generation, and tiny-area paths
```

Each leaf has one client authority owner. Filesystem reads remain in existing
refresh/worker paths; terminal/process truth remains outside the page.

### A3 — Dimensional Investigation

| Dimension | Current evidence | Target decision |
|---|---|---|
| Product | Direct user testing proves useful preview but poor page and mouse fidelity | Correct the concrete native FM without restarting completed modules |
| Behavior | `FmState::enter/leave` replace cwd and refresh a fixed projection | Add a bounded path projection; preserve enter/leave exact-path semantics |
| Page/input | `BaseLayer::render` swaps only `terminal_area`; `handle_file_manager_mouse` returns `NotHandled` outside it | Files Workspace owns complete CenterContent input except explicit sidebar/top-overlay routes |
| Layout | `miller_layout` emits fixed 1/2/3 equal columns at 12-cell minimum | Compute a scrollable visible window from bounded widths and complete separators |
| Component/theme | Existing FM visual roles and action models are complete | Reuse them; add only page-header, scrollbar, divider-focus/drag roles from current palette |
| Data | `FmState` owns cwd/current entries plus cached parent and preview | Retain exact path-segment identity and a bounded prepared column cache outside render |
| Runtime/API/PTY | Terminal and agent runtime already exist independently of the FM | No runtime/API/protocol/PTY change; background processes stay alive and input-inert |
| Failure/security/resources | Watchers, preview generations, file operations, and path-stable hit areas already fail closed | Bind chain/scroll/drag to FM generation and exact paths; add hard cache/gesture bounds |
| Persistence/migration | Current FM layout state is not a persisted shell tree | Widths and horizontal viewport are open-session client state; no migration |
| Platform/accessibility | Ratatui cell geometry and filesystem paths are cross-platform; canonical Windows lint exists | Saturating geometry, Unicode-width tests, no-color semantics, compile-gated platform behavior unchanged |
| Performance | Current render consumes prepared state | No I/O during drag/render; cost proportional to visible rows/columns; cache limits verified before closure |
| Integration/license | Ratatui APIs are already dependencies; references are behavior/pattern evidence | Reimplement through existing code; copy no third-party source and add no dependency |

### A4 — Options and Pattern Decision

#### Selected: Files Workspace

The existing Files sidebar remains application navigation. Files Workspace owns
the full center region and provides its own page chrome. This gives one visual
and input owner without conflating terminal tabs with client-only page identity.

#### Rejected: Client Surface Tab

This looks natural beside terminal tabs, but the current tab bar projects shared
workspace/session organization. Adding a client-only Files tab there would
create two identity and lifecycle domains in one control before a general page
contract is earned.

#### Rejected: Full-Screen Files Page

This provides maximum width but removes the existing Files sidebar and weakens
the Spaces/Projects/Files navigation context. The user selected the workspace
composition instead.

#### Rejected: Overlay or floating curtain

An overlay preserves background visuals and expands focus/input stack pressure.
It is appropriate for the existing short-lived attachment picker, not for the
primary file-management workflow.

#### Source-backed pattern buckets

- Direct Ratatui APIs: R014 at pinned commit
  `de5168de6ba2f4b310565c287764f213f249a61f`, MIT. Exact target APIs are
  `ratatui::layout::{Layout, Constraint, Rect}` plus existing `Block`,
  `Paragraph`, and buffer rendering. Reuse mode: direct existing dependency/API.
  Adaptation cost: low.
- Adaptable architecture: R033 Hypertile at pinned commit
  `fa6011e9fbfb1246b8fe04efdf614f8890cce6f0`, MIT. Exact source evidence is
  `src/core/state/mutation.rs::HypertileState.resize_focused` and
  `extras/src/runtime/render.rs::render_resize_hover`. Reuse mode: behavior
  reimplementation only for original-width drag preview/commit/cancel, not its
  split tree or runtime. Adaptation cost: medium.
- Scenario reference: R061 TUI Studio at pinned commit
  `af75d2cc41805e9c6ac9e9de803fefc6c3cc03d0`, MIT. Exact source evidence is
  `src/components/editor/Canvas.tsx::getResizeHandles` and
  `src/utils/layout/engine.ts::LayoutEngine.calculateLayout`. Reuse mode:
  behavioral/authoring reference; its DOM/CSS runtime is not compatible with
  Herdr. Adaptation cost: medium.
- Caution reference: generic `ratatui-interact` split panes and unindexed
  component frameworks are not selected. No dependency or source reuse occurs
  without a separate license/API/dependency review.

### A5 — Fresh Herdr Cartography and Fit

Current exact owners:

- `src/ui.rs::compute_view_internal` computes `LeftPanel`, `CenterContent`,
  terminal tab bar, terminal area, FM rows, pane infos, split borders, and
  action areas even while the FM is open.
- `src/ui.rs::BaseLayer::render` always renders the sidebar and terminal tab bar,
  then switches only `terminal_area` between `render_file_manager` and
  `render_panes`.
- `src/app/input/mod.rs::handle_mouse_without_agent_frame_action` routes the
  overlay, then FM center input, then sidebar/pane/general mouse paths.
- `src/app/input/file_manager.rs::handle_file_manager_mouse` consumes events
  inside `view.terminal_area`, but returns `NotHandled` outside it.
- `src/ui/file_manager.rs::miller_layout` computes a fixed responsive 1/2/3
  column projection with 12-cell minimum columns and one-cell dividers.
- `src/ui/file_manager.rs::compute_file_manager_row_geometry` creates stable
  path row/action targets only for the CURRENT column.
- `src/fm/mod.rs::FmState` owns cwd, current entries, cursor, viewport, cached
  `FmParent`, selected-entry `FmPreview`, preview generation, and explicit
  multi-selection.
- `FmState::enter` changes cwd to the selected directory and reloads;
  `FmState::leave` returns to the parent and focuses the exact departed path.

Fit decision: the feature belongs in existing client FM/App/UI owners. No new
server state, protocol field, runtime owner, general component registry, or
persisted shell tree is required.

### A6 — Synthesized Target Contract

Decision: `go` for the Files Workspace architecture, conditional on the ordered
test-first slices below and on final calibration of cache/performance ceilings
before the chain slice reaches GREEN.

The target has four independent behavioral increments:

1. page composition and exclusive input ownership;
2. mouse parity for already prepared parent/current/directory-preview columns;
3. bounded path-chain projection with horizontal viewport;
4. FM-local separator resize with responsive degradation.

They must not be combined into one implementation commit. A later slice may
depend on an earlier one, but each preserves its own RED, GREEN, rollback, file
ownership, and verification record.

### A7 — Normalized Handoff Readiness

This specification is the human-readable interim `change-intent-package` while
the canonical pipeline module is not implemented. It freezes:

- approved target and rejected alternatives;
- current and expected behavior;
- authority and side-effect ownership;
- non-goals and activation boundaries;
- test-point catalog;
- ordered implementation and commit slices;
- failure, resource, platform, verification, publication, and rollback gates.

A7 becomes `ready` only after the user reviews this written specification.
Product authorization alone does not bypass that review. I0 may then freeze
exact paths and generate the code-level TDD plan.

## Target Architecture

### Page Composition

Desktop composition:

```text
full frame
  LeftPanel                         CenterContent
  +----------------------+          +------------------------------------+
  | Spaces Projects Files|          | Files header / breadcrumb / close  |
  | FAVORITES            |          +------------------------------------+
  | PINNED               |          | horizontally windowed Miller path  |
  | LOCATIONS            |          | | col | col | col | preview |       |
  |                      |          | |     resizable dividers     |       |
  |                      |          +------------------------------------+
  |                      |          | horizontal scrollbar / status      |
  +----------------------+          +------------------------------------+
```

When Files Workspace is open:

- `CenterContent` is the page area.
- `terminal_area` and terminal tab-bar hit areas are not page input authority.
- terminal pane infos may remain available as immutable runtime projection only
  where another background feature genuinely requires them, but their hit areas
  and frame actions are cleared.
- ambient notifications must not cover actionable Files controls. Existing
  notifications may render only in a page-declared non-interactive placement
  or be deferred while a blocking Files overlay is active.
- topmost existing modal/context ownership remains above the Files page.

Mobile composition remains one active Files page surface with a compact header.
The global mobile menu remains the route out of the page. Secondary Miller
columns become navigation history/preview affordances rather than squeezed
sub-12-cell columns.

### Page Route and Lifecycle

- Existing native-FM activation opens Files Workspace at the active workspace
  cwd, preserving current behavior.
- Clicking an accessible Files sidebar item keeps the page open and requests
  exact directory navigation through the existing scheduled App boundary.
- Selecting the Spaces or Projects sidebar tab closes Files Workspace page
  state before exposing that surface. It does not stop terminal runtimes.
- Closing Files Workspace returns to the currently focused terminal/session
  state without creating, deleting, or refocusing a different pane.
- Reopening begins from the active workspace cwd unless a separately approved
  preference contract is added later. Open-session chain, scroll, and divider
  state do not persist across close/reopen in this scope.

### Input Ownership Matrix

| Surface/event while Files Workspace is open | Owner | Result |
|---|---|---|
| Files page header, breadcrumb, column, scrollbar, divider, action/status controls | Files Workspace | Consume and dispatch typed client action |
| Accessible Files sidebar item | Files sidebar -> scheduled App refresh | Navigate exact path once |
| Spaces/Projects sidebar tab | Global sidebar route | Close Files page, then switch surface |
| Sidebar collapse/resize control | Existing sidebar owner | Allowed if it cannot target a hidden terminal surface |
| File context menu or delete/rename modal | Existing top overlay owner | Consume before page input; restore Files page focus on close |
| Terminal tab bar coordinate | Files Workspace blocker | Consume; no tab switch |
| Hidden pane, split border, scrollbar, agent frame action | Files Workspace blocker | Consume; no focus, resize, scroll, send, or launcher action |
| Terminal mouse reporting or URL click | Files Workspace blocker | Never reached |
| Ambient toast | Existing non-blocking notification policy | No page action obstruction; no hidden terminal action |
| Unknown mouse event inside CenterContent | Files Workspace | Consume fail-closed |
| Keyboard not mapped by Files Workspace | Files Workspace | Consume; never forward to terminal |

### Bounded Miller Path Projection

The target is a path projection, not arbitrary history.

Conceptual client model:

```text
MillerPathProjection
  generation
  segments[]: exact directory path + selected child identity
  active_segment
  horizontal_view_start
  prepared_columns: bounded cache keyed by exact path + refresh generation
  column_widths: open-session presentation widths
  resize_gesture: optional transient divider operation
```

Rules:

- Segments describe the current directory ancestry and current selection
  projection. They are not a back/forward stack and store no arbitrary cursor
  history.
- Selecting a directory in any prepared column truncates descendants after
  that column, appends the selected directory, and prepares only the required
  next context outside render.
- Selecting a directory in the right contextual preview advances it into the
  current path. Selecting a regular file updates the existing file preview and
  does not create another directory column.
- Every hit target carries exact directory path, entry path, column generation,
  and row generation. Coordinate/index equality without path equality is
  insufficient.
- Watcher refresh updates only matching current prepared identities. Missing,
  hidden, replaced, or unreadable segments truncate or enter an explicit
  unavailable state; they never retarget a sibling by index.
- Keyboard enter/leave continues to use the same path transition owner and
  auto-scrolls the horizontal viewport so the active column and preview remain
  visible when geometry allows.

Initial hard safety ceilings for the implementation plan:

- at most 32 retained path identity segments;
- at most 6 simultaneously visible directory columns;
- at most 6 materialized non-preview column snapshots;
- at most 8192 cached non-current entry identities in aggregate;
- exactly one active horizontal drag and one active divider drag;
- no filesystem read in render, hit-test, drag-move, or scrollbar-move paths;
- cache miss schedules one replaceable preparation request for the current
  generation; no hot retry or unbounded queue.

If a path exceeds 32 segments, the projection keeps the nearest 32 segments
and renders a leading collapsed-ancestor indicator. Exact cwd remains the
authority. If non-current cache budget is exceeded, least-recent non-active
snapshots are evicted; exact path segments remain and are re-prepared on demand.
The active current directory list is not truncated to satisfy the contextual
cache budget.

These ceilings are implementation hypotheses and must be measured at I12.
Changing them requires updating the test catalog and performance evidence, not
silently widening the bounds.

### Horizontal Viewport

- The visible column window is derived from current column widths, complete
  divider widths, available page width, and the active path position.
- The active current column is always visible. The preview is visible when at
  least one complete 12-cell column plus divider can fit after it.
- Manual horizontal scrolling is exposed through a bottom scrollbar and
  semantic horizontal-scroll actions. Raw key bindings are selected only after
  the existing keymap collision audit in I5.
- Vertical wheel input over a column remains vertical row navigation/scroll.
  Horizontal scrollbar drag and declared horizontal-scroll gestures move only
  the column viewport.
- Track clicks page by the number of complete currently visible columns.
- Resize during a scrollbar drag cancels or recomputes from the original
  generation; it never commits an out-of-range offset.

### Separator Resize

- A separator is interactive only when completely visible and both adjacent
  columns are present.
- Press snapshots the divider identity, adjacent path identities, initial
  widths, pointer coordinate, page generation, and available area.
- Drag computes a preview from the original widths. It does not repeatedly
  mutate persistent layout and performs no I/O.
- Release commits only when both adjacent columns remain exact and at least 12
  cells wide. Otherwise it cancels.
- Leaving the terminal, losing the button, changing route, changing FM
  generation, or resizing the terminal cancels safely.
- Double-click resets the adjacent widths to the current responsive equal-share
  policy.
- Widths are open-session TUI/client presentation state. This scope adds no
  serialization, session snapshot field, migration, or S6 `ShellLayout` tree.

### Responsive Degradation

| Available page content width | Behavior |
|---|---|
| Below 12 cells | Explicit too-narrow Files state; no row/divider/scroll hit targets |
| 12-24 cells | One complete active column; preview via explicit open action/status |
| 25-38 cells | Two complete columns with one divider |
| 39+ cells | Three or more columns according to saved open-session widths, capped at six visible |

The exact page header/status rows reduce the content rectangle before these
rules apply. All arithmetic is saturating and boundary tests cover exact width,
one below, and one above.

### Render and Side-Effect Boundary

`compute_view` or a focused pure geometry helper owns:

- page/header/content/status rectangles;
- visible column window;
- column, row, divider, scrollbar, and action hit rectangles;
- truncation and responsive degradation;
- clearing stale hit geometry on every frame.

State refresh/update owners perform:

- directory reads and metadata preparation;
- path-chain transition and cache generation changes;
- watcher reconciliation;
- preview worker scheduling and result acceptance;
- operation requests and lifecycle updates.

`render` receives `&AppState`, prepared models, and computed rectangles. It
does not read the filesystem, mutate cursor/scroll/widths, advance gestures,
parse config, inspect sockets, send terminal input, or create runtime state.

## Test-Point Catalog

Every implementation slice begins with the smallest compile-valid behavioral
test. The observed failure must match the declared current behavior.

### TP-FW-1-PAGE-COMPOSITION

- Layer: pure geometry and deterministic full-frame component.
- What: desktop normal/narrow/mobile, sidebar expanded/collapsed, terminal tab
  bar present/hidden, zero/tiny page, and close/reopen frames.
- Current: `BaseLayer::render` renders terminal tab bar and swaps only
  `terminal_area` to the FM.
- Expected: Files Workspace owns complete CenterContent, renders Files page
  chrome, omits terminal chrome, and clears all stale terminal-page hit areas.
- Diff: center-canvas content swap -> concrete center-region page.
- Expected RED: terminal tab bar remains in the buffer or its hit area remains
  non-empty while Files Workspace is active.
- Reason: this is the visual and geometry root of the curtain behavior.
- Initial owned files: `src/ui.rs`, `src/ui/file_manager.rs`, `src/app/state.rs`.

### TP-FW-2-INPUT-OWNERSHIP

- Layer: deterministic App input component.
- What: clicks/wheel/drag over hidden terminal tab, pane, split border, pane
  scrollbar, agent frame actions, terminal mouse reporting, Files sidebar,
  sidebar tabs, and topmost Files overlays.
- Current: `handle_file_manager_mouse` returns `NotHandled` outside
  `view.terminal_area`, allowing later background routes.
- Expected: hidden terminal surfaces are consumed without mutation; accessible
  Files navigation and valid global route changes remain functional; topmost
  overlay consumes first.
- Diff: partial center capture -> explicit page ownership matrix.
- Expected RED: at least one hidden terminal/tab/pane target changes state or
  receives a terminal event while FM is open.
- Reason: hidden background interactivity is a focus and trust defect.
- Initial owned files: `src/app/input/mod.rs`,
  `src/app/input/file_manager.rs`, relevant existing mouse tests.

### TP-FW-3-MILLER-MOUSE-PARITY

- Layer: pure geometry plus deterministic input component.
- What: parent/current/directory-preview rows at every breakpoint; regular-file
  preview, empty/header/divider/status/action gaps; plain/Ctrl/Shift/right
  clicks; watcher reorder/delete/replacement and stale generations.
- Current: row geometry and activation exist only for CURRENT entries.
- Expected: every prepared directory-column row maps to an exact path-stable
  column action; preview content that is not a directory list remains inert;
  stale identity is consumed fail-closed.
- Diff: current-only row map -> typed per-column directory row map.
- Expected RED: parent/preview click produces no transition while current-row
  characterizations remain green.
- Reason: rendered navigation context must have mouse parity without creating
  coordinate-derived filesystem authority.
- Initial owned files: `src/ui/file_manager.rs`, `src/app/state.rs`,
  `src/app/input/file_manager.rs`, `src/fm/mod.rs` only if a new pure transition
  seam is required.

### TP-FW-4-COLUMN-CHAIN

- Layer: pure state, deterministic filesystem fixture, and watcher component.
- What: repeated descent, branch change from an ancestor, enter/leave, root,
  file selection, hidden toggle, reorder/delete/permission failure, 32-segment
  boundary, collapsed ancestor, cache eviction, close/reopen, and stale worker.
- Current: each transition replaces a fixed parent/current/preview projection.
- Expected: exact bounded path segments project deterministically; descendant
  branches truncate; active path remains visible; cache eviction never changes
  cwd/selection identity; no back/forward history is introduced.
- Diff: fixed three-context cache -> bounded path identity projection plus
  bounded prepared column cache.
- Expected RED: second or later descent cannot retain the preceding prepared
  column/path segment, while existing `leave` exact-child tests stay green.
- Reason: this is the user-requested Finder-like spatial continuity and the
  main resource-risk surface.
- Initial owned files: `src/fm/mod.rs`; App watcher/worker files only when the
  approved plan proves existing refresh seams cannot own preparation.

### TP-FW-5-SEPARATOR-RESIZE

- Layer: pure geometry and deterministic gesture component.
- What: press/drag/release, left/right direction, minimum widths, terminal edge,
  pointer leave, lost release, generation/path change, terminal resize,
  double-click reset, zero/tiny area, and close/reopen.
- Current: equal `Constraint::Min` columns and non-interactive one-cell dividers.
- Expected: one exact adjacent pair previews and commits valid widths; every
  stale/invalid gesture cancels; reset restores responsive equal share; no
  ShellLayout/session state changes.
- Diff: static divider -> FM-local bounded gesture.
- Expected RED: divider has no hit target/gesture and widths remain equal after
  a valid drag sequence.
- Reason: requested user control must not become a general persisted tiler.
- Initial owned files: `src/app/state.rs`, `src/ui/file_manager.rs`,
  `src/app/input/file_manager.rs`.

### TP-FW-6-RESPONSIVE-DEGRADE

- Layer: pure geometry and buffer component.
- What: every boundary width, one below/above, header/status deductions,
  sidebar collapse/resize, mobile, Unicode names, no-color/ASCII, zero area,
  and width distributions after drag.
- Current: fixed 1/2/3 equal projection.
- Expected: only complete >=12-cell columns and dividers are visible/addressable;
  active column survives; preview/scrollbar degrades explicitly; no overlap.
- Diff: fixed breakpoint projection -> width-aware horizontal window.
- Expected RED: dragged/bounded widths cannot be represented or a clipped
  column/divider retains a hit target.
- Reason: resize and horizontal scroll are unsafe without exact degradation.

### TP-FW-7-LIFECYCLE-RECOVERY

- Layer: state, deterministic component, and isolated real filesystem.
- What: watcher refresh/burst/fallback, cwd disappearance/unreadable, selected
  path removal, hidden toggle, preview generation, operation completion,
  Files/Spaces/Projects transition, close/reopen, same-cwd reopen, and stale
  row/column/drag/scroll generations.
- Current: existing current/parent/preview lifecycle is safe but knows no page
  or path-chain generation.
- Expected: only matching page/Fm/path generation mutates projection; invalid
  descendants truncate or expose explicit failure; every stale geometry and
  gesture clears.
- Diff: single cwd context generation -> page plus bounded column identities.
- Expected RED: stale column/gesture survives a route/generation transition or
  current lifecycle has no way to reject it.
- Reason: filesystem and user input race by design.

### TP-FW-8-PERFORMANCE-RESOURCE

- Layer: performance regression plus bounded stress fixture.
- What: 32 path segments, six visible/materialized columns, aggregate 8192
  non-current cached entries, rapid horizontal scroll, divider drag storm,
  watcher burst, repeated cache miss/eviction, and idle page.
- Current: no chain/drag overhead exists.
- Expected: hard ceilings hold; one replaceable preparation request; no I/O or
  allocation growth per drag move beyond bounded geometry; idle produces no
  render loop; frame composition remains within the calibrated I12 ceiling.
- Diff: fixed projection -> bounded cache and gesture work.
- Expected RED: new contract types/metrics are absent, not an artificial timing
  failure.
- Reason: Finder fidelity cannot create unbounded directory memory or hot UI
  work.

### TP-FW-9-GATES

- Layer: complete repository/platform/manual closure.
- What: focused state/geometry/input/render/lifecycle/performance families,
  full nextest, Linux all-target and canonical Windows MSVC clippy, Bun,
  Python maintenance, format, diff, production `unwrap()`, ignored-test and
  artifact scans, graph freshness, and isolated real PTY mouse review.
- Expected: zero in-scope failure/retry-only green; only the named B0 real-host
  probe may remain skipped; no stable Herdr/socket/process access; zero
  throwaway residue.
- Reason: page/input/geometry changes cross several mature FM and shell seams.

## Ordered Delivery Slices and Commit Boundaries

No slice begins production code before its matching observed RED.

### FW0 — Written Contract

- This design specification.
- No product file.
- Commit: `docs(native-fm): define Files Workspace experience`.

### FW1 — Concrete Page Composition and Input Ownership

- Characterize retained sidebar, terminal runtime, modal/context, and route
  behavior.
- RED TP-FW-1 and TP-FW-2.
- GREEN the smallest page area and input blocker.
- No path chain, divider resize, new cache, or visual redesign.
- Proposed commits:
  - `test: define Files Workspace ownership`
  - `feat: make native file manager a Files Workspace`

### FW2 — Existing Three-Context Mouse Parity

- RED TP-FW-3 against parent/current/directory-preview.
- GREEN typed exact per-column targets using the existing cached contexts.
- This slice deliberately proves useful mouse behavior before introducing the
  broader chain.
- Proposed commits:
  - `test: define Miller column mouse navigation`
  - `feat: add mouse navigation across Miller columns`

### FW3 — Bounded Path Chain and Horizontal Viewport

- Characterize current enter/leave/watcher/preview/selection behavior.
- RED TP-FW-4 and the non-timing portions of TP-FW-8.
- GREEN bounded path identity, prepared cache, auto-follow, and horizontal
  scrollbar in separately reviewable state/geometry increments if needed.
- N2.2 history remains absent.
- Proposed commits:
  - `test: define bounded Miller path projection`
  - `feat: add a scrollable Miller column path`

### FW4 — FM-Local Divider Resize

- RED TP-FW-5 and TP-FW-6.
- GREEN gesture preview/commit/cancel/reset and responsive rendering.
- No persistence or ShellLayout mutation.
- Proposed commits:
  - `test: define Miller column resize behavior`
  - `feat: add resizable Miller columns`

### FW5 — Cross-Layer and Production Closure

- TP-FW-7 lifecycle/recovery.
- TP-FW-8 calibrated resource/performance proof.
- TP-FW-9 complete gates and isolated PTY review.
- Any unrelated fixture stabilization is a separate test-only commit after
  root-cause proof.
- Continuity/evidence update remains separate from product changes.

## Git and Publication Contract

- Never use `git add -A`.
- Target-stage only the declared slice files and inspect staged names/diff.
- Keep `.codex/` continuity/tooling, `.superpowers/` visuals, product Rust, and
  test-only stability concerns separate.
- Preserve lowercase conventional commits, no emoji, no AI co-author.
- Do not publish a failing RED tip alone as completed work.
- Before publication, run fresh proportional gates, fetch, prove fast-forward
  ancestry, and push only the CyPack fork feature/master refs authorized by the
  standing workflow.
- Never push `upstream`, force-push, open upstream issues/PRs, install Herdr, or
  touch the stable socket/process.
- After committed implementation, refresh Codebase Memory and prove current
  Files Workspace symbols beyond `ready`.

## Rollback Contract

- FW1 rollback restores the existing terminal-area FM swap without touching
  runtime state.
- FW2 rollback removes only additional column hit geometry/transitions.
- FW3 rollback returns to the verified bounded parent/current/preview state;
  no persisted chain requires migration.
- FW4 rollback removes open-session widths/drag state and returns to equal
  responsive columns; no snapshot migration exists.
- Any slice that cannot preserve existing full gates, input isolation, or
  resource bounds is reverted independently before the next slice.

## Acceptance Criteria

The Files Workspace initiative is complete only when:

1. the user-approved A composition owns complete CenterContent and does not
   display or activate terminal tab/pane chrome;
2. Files sidebar navigation and valid route transitions remain exact;
3. topmost modal/context ownership precedes page input;
4. parent/current/directory-preview/path-column mouse targets are exact and
   stale-safe;
5. repeated directory navigation produces a bounded horizontally scrollable
   path projection with explicit cache/segment ceilings;
6. complete separators resize adjacent columns safely with reset/cancel and
   responsive minimums;
7. terminal runtime remains alive but receives no page-time input;
8. no server/protocol/persistence/dependency/general-registry change appears;
9. render remains pure and all I/O/preparation remains in state refresh/worker
   paths;
10. every production slice has observed behavior-specific RED and fresh GREEN;
11. failure, recovery, platform, capability, resource, and performance gates
    pass with declared evidence;
12. full repository verification, isolated PTY cleanup, Git publication,
    remote SHA, and graph freshness close without an unexplained failure.
