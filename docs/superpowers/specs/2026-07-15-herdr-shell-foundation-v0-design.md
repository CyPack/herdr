# Herdr Shell Foundation v0 Design

## Status

- Design date: 2026-07-15
- Design status: approved by the user; implementation plans frozen
- Change mode: `mid_flight_adoption`
- Product direction: **Thin Shell Foundation -> Files first consumer -> FM UX continuation -> Apps/Desktop expansion**
- Product authorization: approved for the bounded Shell Foundation and the
  subsequent Files migration described here
- Product implementation status: not started; SF1 characterization is next
- Delivery status: A0-A7 complete; I0-I5 manually evidenced by the approved
  three-plan package; current delivery phase is I6 characterization
- Repository: `/home/ayaz/projects/herdr`
- Branch at design freeze: `feat/native-fm`
- Design checkpoint parent: `11a5503`
- Codebase Memory evidence: 19,534 nodes / 91,017 edges, with freshness proven
  by the current `miller_layout` symbol rather than `ready` alone

## Executive Decision

Herdr will gain a bounded, client-owned **Shell Foundation v0** before further
Finder-style file-manager interaction work. The foundation is not a visual
layout editor, free-form window manager, or arbitrary runtime component tree.
It is the smallest reusable shell contract that can safely host native and
terminal application surfaces while preserving Herdr's pure render, runtime
ownership, and low-latency terminal streaming behavior.

The shell itself remains core infrastructure. It will be designed to become
plugin-ready, but it will not be implemented as a plugin. Plugins and built-in
apps may contribute application definitions and surfaces only through
capability-checked shell contracts. Geometry, input routing, focus, overlay
ownership, persistence, render scheduling, and runtime boundaries remain
controlled by Herdr core.

The first real consumer is the existing native Files experience. Files moves
from a terminal-canvas replacement into a native application surface hosted by
the workspace stage. Once that migration is proven, work returns immediately
to FM UX: horizontally navigable Miller history, resizable Miller columns,
complete column mouse ownership, and Finder-like path interaction.

## Relationship to the Existing Files Workspace Design

This specification extends and partially supersedes
`2026-07-15-native-file-manager-files-workspace-design.md` at the shell and
page-architecture boundary.

The previous design remains authoritative for these product observations and
behavior goals:

- Files must not behave like a curtain above an interactive terminal surface;
- hidden terminal tabs, panes, borders, scrollbars, and mouse-reporting paths
  must not receive input while Files owns the active surface;
- the Files sidebar, prepared previews, file operations, watcher behavior, and
  exact-path authority remain preserved;
- Miller navigation needs bounded horizontal history, full column mouse
  interaction, and resizable separators;
- render performs no filesystem, process, socket, worker, or persistence I/O.

The earlier non-goals for a general page contract and persisted shell tree were
correct at that design checkpoint because there was no independent consumer.
Direct user demand now provides the missing activation evidence:

- AppDock plus Agent Sidebar plus Workspace Stage are separate durable regions;
- RightPanel, TopBar, and BottomBar require real named-region contracts;
- multiple regions must resize, collapse, scroll, restore, and persist;
- Apps/Desktop and Files are independently owned page/application lifecycles;
- Files must become a native stage surface rather than a special base-layer
  conditional.

Historical implementation and test evidence is preserved. This specification
does not reinterpret old compile/setup failures as RED, invent missing RED
evidence, or authorize reimplementation of completed FM modules.

## Mission and Product Outcome

Herdr's product mission is to provide a desktop-like environment from a
terminal while retaining terminal-native speed, composability, remote/SSH
fitness, and process transparency.

Shell Foundation v0 succeeds when Herdr can express this composition through
reusable contracts rather than page-specific coordinate arithmetic:

```text
+---------------------------------------------------------------------+
| TopBar                                                              |
+--------+------------------+--------------------------+---------------+
|        |                  |                          |               |
| AppDock| Agent Sidebar    | Workspace Stage          | RightPanel    |
|        |                  |                          |               |
| Files  | Agents           | Terminal / Files         | Inspector     |
| btop   | Projects         | Music / plugin apps      | Context       |
| Music  | Context          |                          |               |
|        |                  |                          |               |
+--------+------------------+--------------------------+---------------+
| BottomBar                                                           |
+---------------------------------------------------------------------+

                    OverlayHost above the shell
```

The target is a platform shell with bounded composition, not a pixel desktop
or GUI toolkit clone.

## Goals

Shell Foundation v0 standardizes:

1. stable named outer regions;
2. bounded nested split containers;
3. fixed, content-bounded, resizable, fill, and collapsed track policies;
4. local horizontal and vertical scroll viewports;
5. divider resize preview, commit, cancel, and reset;
6. collapse, restore, and previous-size retention;
7. native and terminal application surface hosting;
8. deterministic focus, hit testing, mouse capture, and keyboard routing;
9. topmost overlay input blocking and focus restoration;
10. versioned client-layout persistence with legacy migration;
11. reusable page templates and interaction patterns;
12. measurable geometry, render, queue, and SSH-performance invariants.

## Explicit Non-Goals

Foundation v0 does not include:

- dragging a component to an arbitrary docking target;
- an unbounded runtime component or plugin tree;
- a visual layout editor or authoring canvas;
- floating desktop windows;
- user-authored arbitrary JSON/TOML layout DSL;
- independent render loops per component;
- a new network, socket, or layout protocol;
- native Rust dynamic-library plugins;
- WASM UI components or an out-of-process native UI protocol;
- general undo/redo for layout mutations;
- free split/swap/close operations in every region;
- implementation of every future TopBar, BottomBar, or RightPanel consumer;
- unrelated workspace/tab/pane identity refactors;
- changes to installed stable Herdr, its process, or its socket.

## Current Architecture Evidence

### Reusable foundations

- `src/ui/shell.rs::ShellLayout` already owns a recursive `ShellNode` and named
  `RegionId` values, but production defaults to LeftPanel plus CenterContent.
- `src/ui.rs::compute_view_internal` is the current geometry and derived-view
  owner.
- `src/ui/compose.rs::Component` is a small pure-render contract.
- `src/app/input/sidebar.rs::set_manual_sidebar_width` and
  `set_sidebar_section_split` prove existing mouse-resize and dirty-session
  behavior for one concrete region.
- `src/server/render_stream.rs::ClientRenderState::prepare_frame` skips
  semantically identical frames and prepares terminal ANSI differences.
- `src/server/headless.rs::render_retained_pty_update_and_stream` updates dirty
  terminal rows from the retained frame when safe.
- `src/server/client_transport.rs::ClientWriterQueueState` retains one pending
  render slot while control messages remain prioritized.
- plugin pane manifests can launch commands into Overlay, Split, Tab, or
  Zoomed placements, providing a future terminal-app launch substrate.

### Missing contracts

- no persistent shell-tree or versioned layout schema;
- no general constraint model beyond current dynamic/fill shell sizing;
- no reusable resize transaction with transient preview and one-shot commit;
- no shell-level focus/capture/hit router;
- no native `AppDefinition -> AppInstance -> AppSurface` lifecycle;
- no `SurfaceHost` separating application identity from runtime identity;
- no page-template catalog;
- no common scroll viewport contract;
- no bounded nested-container validation;
- no geometry generation/cache contract;
- no plugin-facing application metadata for icons, singleton policy, or native
  surface capability.

## Architectural Options

### Selected: bounded core shell with real consumers

Keep fixed semantic outer regions, allow bounded split containers inside
approved regions, and implement primitives only in dependency order with Files
as the first native surface. This provides reuse without building a speculative
general-purpose UI framework.

### Rejected: finish FM before shell work

Continuing Miller interaction on the current page/canvas boundary would bind
new scroll, resize, and mouse authority to geometry that is about to move into
the workspace stage. It creates avoidable migration and duplicate input logic.

### Rejected: full layout framework before FM

A complete docking manager, arbitrary component registry, visual editor, and
plugin UI runtime would be too large to validate through current product
consumers. It increases state, persistence, failure, and performance risk while
delaying the user-visible FM correction.

## Core Ownership Boundary

```text
Herdr core
  ShellState / ShellView / ShellLayoutEngine
  focus, input, capture, overlays, persistence, render scheduling
                 |
                 +-> built-in native surfaces
                 |     Files
                 |
                 +-> terminal app definitions
                 |     terminal, btop, music, plugin panes
                 |
                 +-> future declarative/out-of-process extensions
```

The shell is the host and cannot be a plugin. Apps can become plugins only
after the host supplies stable, capability-checked contracts.

### Runtime and server ownership

The following remain shared/runtime truth:

- workspace, tab, pane, terminal, process, and agent identity;
- PTY dimensions and process lifecycle;
- terminal parser/buffer state;
- server events and API-visible runtime facts.

### TUI/client ownership

The following remain client presentation truth:

- shell tree, region constraints, collapse state, and restored sizes;
- active client surface and local focus scope;
- page route, dock ordering, hover, hit areas, scroll offsets, and drag state;
- overlay stack and background-input blocking;
- derived region rectangles and layout cache generations.

No new shared behavior is added only through the private TUI socket. Shell
geometry is not serialized per render frame.

## Target State Model

The conceptual model is separated into persistent, transient, and derived
state:

```text
ShellState                         persistent client presentation state
  schema_version
  template_id
  root: ShellNode
  region_preferences
  component_placements
  dock_preferences

ShellInteractionState              transient, never persisted
  focused_component
  captured_pointer
  resize_gesture
  active_scroll_owner
  overlay_stack

ShellView                          derived for current terminal geometry
  generation
  responsive_mode
  region_rects
  divider_hit_areas
  component_hit_areas
  visible_components
  clipped_components

AppCatalogState                    app definitions and pinned order
AppInstanceState                   active client app instances
AppSurfaceRef                      native or terminal-backed surface identity
```

`ShellView` never owns PTYs, processes, filesystem handles, watcher handles, or
worker channels.

## Stable Identity Model

### Named outer regions

Foundation v0 defines these stable semantic region IDs:

- `TopBar`
- `AppDock`
- `LeftPanel`
- `WorkspaceStage`
- `RightPanel`
- `BottomBar`

Existing `CenterContent` behavior migrates to the semantic WorkspaceStage
without requiring shared runtime identity to adopt UI terminology. A local
compatibility adapter may temporarily map the existing name while tests are
migrated.

### Component identity

Every leaf references a stable `ComponentId`. Built-in IDs are typed and
closed over the v0 inventory. Future external IDs require a qualified plugin
namespace. Rectangles, indexes, or tree positions are never component
identity.

### Application identity

```text
AppDefinition
  stable app ID, title, icon/fallback, surface kind, launch policy

AppInstance
  stable open instance ID, definition ID, lifecycle and current surface

AppSurface
  NativeFiles | TerminalTab reference | future capability-checked surface
```

Dock items represent application definitions. Tabs and stage selections
represent running instances. Singleton activation focuses the existing
instance; multi-instance activation may create a new instance only when the
definition permits it.

## Shell Tree and Layout Primitives

### Node categories

```text
ShellNode
  SplitContainer
    axis: horizontal | vertical
    children: bounded ordered nodes
    gap/divider policy
  RegionSlot
    stable region ID
    track policy
    collapse policy
    child container or surface host
  SurfaceHost
    current AppSurfaceRef
  StackContainer
    bounded selected child set without independent render loop
```

`ScrollViewport`, `ResizeDivider`, and `CollapseControl` are behavior/view
contracts associated with a container or region; they are not arbitrary
recursive shell nodes.

`OverlayHost` remains a separate z-ordered owner above the shell tree. Overlay
geometry does not consume persistent split-tree depth.

### Track policies

```text
Fixed { cells }
ContentBounded { min, max }
Resizable { min, preferred, max }
Fill { weight }
Collapsed { restore }
```

Rules:

- all values use checked/saturating terminal-cell arithmetic;
- `min <= preferred <= max` is validated before a tree becomes active;
- fill weights are positive, bounded integers and normalized during compute;
- a collapsed resizable track retains the last valid committed size;
- expanding clamps the retained size to current min/max and available area;
- zero-area nodes produce no child rect, divider, focus, or hit authority;
- an invalid persisted tree never becomes partially active.

## Bounded Composition Invariants

Foundation v0 freezes these implementation limits:

- outer shell topology is fixed by the selected page template;
- panel-local nested split depth is at most 4;
- one split node has at most 8 children;
- one resolved shell has at most 64 visible component leaves;
- one template has at most 128 total serialized nodes, including collapsed
  nodes;
- one stack container has at most 32 children;
- layout computation is `O(node_count)` and performs no tree search inside a
  per-cell or per-row loop;
- hit areas are flattened once per shell-view generation;
- ratios/weights and restored sizes are normalized before commit;
- invalid, duplicate, cyclic, over-depth, over-count, or unknown-region trees
  fail closed to the built-in safe template;
- no geometry operation can produce negative, wrapped, or out-of-frame rects.

These are internal v0 compatibility limits, not network protocol promises.

## Default Desktop Template and Region Policy

```text
DesktopWorkspace
  Vertical
    TopBar: Fixed, zero when no consumer
    Horizontal
      AppDock: Resizable, icon-only
      LeftPanel: Resizable
      WorkspaceStage: Fill
      RightPanel: Resizable, zero when no consumer
    BottomBar: Fixed, zero when no consumer
```

Default policies:

- AppDock preferred width: 5 cells; minimum 3; maximum 9;
- AppDock icon/fallback occupies at most two display cells and uses a complete
  three-cell-or-wider row hit target;
- TopBar and BottomBar have fixed configured heights while populated and zero
  height while empty;
- RightPanel has zero width while empty or explicitly collapsed;
- WorkspaceStage is the primary content owner and cannot be collapsed;
- LeftPanel reuses existing sidebar min/max/config behavior during migration;
- dividers exist only between two non-zero adjacent regions;
- empty regions create no separator or pointer target.

## Responsive Degradation

Responsive behavior is driven by content minimums, not device names.

Modes:

- `Workspace`: all populated requested regions satisfy preferred dimensions;
- `Wide`: all primary regions remain, optional regions may clamp;
- `Standard`: RightPanel collapses; LeftPanel may use its existing collapsed
  presentation;
- `Compact`: only the active stage plus explicit compact navigation remains;
- `TooSmall`: a bounded visible diagnostic replaces unsafe composition.

Width degradation order:

1. clamp resizable optional regions toward minimum;
2. collapse RightPanel;
3. collapse LeftPanel to its existing compact representation;
4. collapse AppDock and expose its launcher through compact navigation;
5. preserve WorkspaceStage minimum;
6. if the stage minimum cannot fit, render a stable too-small surface with no
   stale hit areas.

Height degradation order:

1. remove non-essential BottomBar consumer rows;
2. reduce content-bounded TopBar/BottomBar to their declared minimum;
3. collapse optional bars to zero;
4. preserve one safe stage row plus any required border/status contract;
5. otherwise render the too-small surface.

Every threshold is tested at one cell below, exactly at, and one cell above.

## Reusable Component and Pattern Pool

### Layout primitives

- `SplitContainer`
- `RegionSlot`
- `SurfaceHost`
- `ScrollViewport`
- `ResizeDivider`
- `CollapseControl`
- `StackContainer`
- `OverlayHost`

### Component lifecycle contract

```text
ComponentState      pure client presentation data
ComponentAction     semantic input/update intent
ComponentView       derived rects, visible data and hit regions
update(action)      state transition outside render
compute_view(area)  geometry/projection without I/O
render(view, state) pure buffer drawing
```

Leaf components do not mutate sibling state, allocate runtime resources,
perform I/O, or recursively choose arbitrary component implementations during
render. Containers own child ordering and layout; the application update path
owns state mutation and effects.

### Interaction patterns

- divider drag preview/commit/cancel/reset;
- collapse/restore with retained valid size;
- anchored right-click menu;
- focus scope and restoration;
- topmost overlay input ownership;
- scroll capture and release;
- keyboard resize parity;
- tiny-terminal degradation;
- disabled/inert region behavior;
- stale hit-area rejection by stable identity and view generation.

Each pattern requires shared state-transition, geometry, input, render, and
failure-path tests before a product consumer can claim it.

## Reusable Page Templates

Foundation v0 defines a typed, built-in template catalog:

```text
StageOnly
  WorkspaceStage

DockStage
  AppDock
  WorkspaceStage

DockSidebarStage
  AppDock
  LeftPanel
  WorkspaceStage

DesktopWorkspace
  TopBar
  AppDock
  LeftPanel
  WorkspaceStage
  RightPanel
  BottomBar

InspectorWorkspace
  LeftPanel or Navigation
  WorkspaceStage
  RightPanel or Inspector
```

Templates declare topology, allowed regions, defaults, minimums, collapse
priority, and compatible component slots. They do not hard-code colors,
content, or every exact user size.

Foundation v0 uses typed Rust-owned templates plus versioned persisted choices.
It does not expose an arbitrary layout DSL.

## Resize Transaction

Every mouse resize is one explicit transaction:

```text
MouseDown on current divider
  validate view generation and divider identity
  capture pointer
  snapshot original committed constraints

MouseMove while captured
  clamp a transient preview against current area and minimums
  update ShellInteractionState only
  do not mark session persistence dirty
  do not resize a PTY

MouseUp
  revalidate tree, divider, area and adjacent identities
  normalize and commit once
  clear capture
  mark session dirty once
  resize affected PTY surfaces at most once

Escape
  restore the original constraints
  clear capture without persistence or PTY resize

Terminal resize during gesture
  recompute from the original committed tree
  cancel the stale gesture unless the same divider remains valid
```

Preview may clip the current rendered surface into the preview rect. It does
not repeatedly reflow the terminal application.

Double-click reset is a discrete commit to the template preferred size and is
subject to current min/max/available-area clamping.

## Collapse and Expand Contract

Collapse is not equivalent to forgetting the preferred size:

```text
committed width = 32
collapse -> current = 0, restore = 32
expand   -> current = clamp(32, current min, current max, available area)
```

Rules:

- hidden regions have no focus or hit authority;
- collapsing a focused region moves focus deterministically to the stage or
  nearest valid declared fallback;
- expanding never steals focus unless activation requested it;
- a region removed from the active template discards transient interaction
  state but retains only schema-approved preferences;
- repeated collapse/expand is idempotent;
- tiny-terminal auto-collapse does not overwrite the user's restore size.

## Scroll Viewport Contract

Scrolling belongs to the relevant container/component, not the complete shell
tree.

Each viewport declares:

- axis: horizontal or vertical;
- bounded content extent;
- current offset;
- visible extent;
- scroll step/page behavior;
- focus/capture policy;
- scrollbar visibility policy;
- identity/generation used to reject stale hit targets.

Offsets clamp after content shrink, terminal resize, template change, or
surface replacement. A zero-area or non-overflowing viewport exposes no
scrollbar and consumes only the events explicitly declared by its owner.

## Focus, Mouse, and Keyboard Routing

Input uses this precedence:

```text
1. topmost blocking overlay
2. active pointer/resize/scroll capture
3. current z-ordered hit target
4. focused component
5. page/template shortcut owner
6. global application shortcuts
7. fail-closed consumption when a hidden background surface would otherwise act
```

Requirements:

- hit targets carry stable component/region identity and ShellView generation;
- stale same-coordinate targets are rejected;
- only one owner captures a pointer gesture;
- capture is cleared on owner removal, overlay replacement, terminal resize,
  client detach, or page/template change;
- topmost blocking overlays prevent every background shortcut and mouse route;
- closing an overlay restores the previous valid focus owner or the template
  fallback;
- keyboard and mouse actions emit semantic intents rather than mutating
  filesystem/runtime state in the input handler;
- unknown input within an exclusively owned active surface fails closed rather
  than leaking to a hidden terminal pane.

## Overlay Contract

Foundation v0 preserves existing modal/context behavior while introducing one
explicit topmost ownership rule. Overlay rendering remains separate from the
persistent shell tree.

Every blocking overlay declares:

- stable overlay identity and kind;
- bounded or anchored placement;
- focus scope;
- dismiss policy;
- background blocking policy;
- parent focus owner;
- overflow behavior.

Overlay regions are cleared before drawing. Nested overlay support is required
only for real parent-to-confirmation flows used by the Files migration; this
does not authorize arbitrary floating windows.

## Persistence and Migration

### Persisted data

Foundation v0 persists only bounded client presentation preferences:

- schema version;
- selected built-in template ID;
- approved region sizes and collapse preferences;
- retained restore sizes;
- AppDock pinned order and supported built-in app preferences;
- schema-approved component placements.

Transient focus, hover, hit areas, active capture, overlay stack, current
terminal geometry, computed rects, and render generations are never persisted.

### Legacy migration

The existing session snapshot persists concrete sidebar width and section split
fields. Migration rules:

1. a valid legacy sidebar width becomes the LeftPanel preferred/committed
   width in the default compatible template;
2. the existing sidebar section split remains owned by the sidebar component,
   not promoted into the outer shell tree;
3. missing new shell data selects the built-in compatibility template;
4. unknown future schema versions do not partially deserialize into current
   authority;
5. invalid sizes, duplicate IDs, unknown regions, over-limit trees, or corrupt
   preferences fall back to safe defaults while preserving valid unrelated
   session/runtime data;
6. migration is deterministic and idempotent;
7. the old snapshot remains readable throughout Foundation v0 rollout.

The exact snapshot version bump and compatibility fixture set are decided in
the implementation plan after comparison with the latest released tag.

## Surface Host and Application Lifecycle

`SurfaceHost` presents an `AppSurfaceRef`; it does not own runtime resources.

### Native Files surface

```text
AppDefinition: Files
  -> AppInstance: files:<stable-local-instance-id>
  -> AppSurface: NativeFiles
  -> SurfaceHost: WorkspaceStage
```

Files migration requirements:

- preserve current `FmState`, watcher, preview workers, operation worker,
  selection, context actions, and exact-path authority;
- remove the terminal-canvas curtain behavior;
- suppress hidden terminal tab/pane/split/scrollbar hit authority;
- leave terminal processes and server/runtime state alive;
- close or switch without creating/deleting unrelated pane resources;
- restore the previous valid stage surface/focus on close;
- block background input while a Files overlay is active;
- preserve current mobile safety and define compact-stage navigation.

### Terminal applications

Terminal, btop, music, and current plugin panes continue to use existing
terminal tab/pane runtime paths. AppDefinition is an application catalog layer
above those existing resources, not a replacement runtime.

### Plugin boundary

Foundation v0 is plugin-ready but does not expose native UI plugins.

Current plugin placement remains Overlay/Split/Tab/Zoomed. A future manifest
extension may add icon, surface kind, launch policy, and singleton metadata
only after separate schema, capability, migration, and API review.

Native Rust `.so`/`.dll` UI plugins are rejected because Rust ABI stability,
crash containment, platform parity, and security cannot be guaranteed. Future
native extension research may consider declarative or out-of-process protocols
or sandboxed WASM, but none is part of this delivery.

## Apps/Desktop Future Consumer

After Files proves the shell contracts, Apps/Desktop may consume
`DesktopWorkspace`:

```text
Apps/Desktop
  Installed apps
  Pinned apps
  Running instances
  Recent workspaces
  Launch actions
```

This future page is not required to close Foundation v0 or resume FM work.
Only the template and stable extension seam are prepared now.

## Render and SSH Performance Contract

### Preserved pipeline

```text
UI action -> state/update -> compute_view -> Ratatui buffer
                                             |
PTY dirty rows -> retained-frame patch -------+
                                             |
                              semantic equality check
                                             |
                                      ANSI cell diff
                                             |
                                one pending render slot
                                             |
                                            SSH
```

Foundation v0 does not transmit the shell tree over SSH per frame. The terminal
client receives the resulting semantic/ANSI frame path already used by Herdr.

### Required invariants

- render remains pure and performs no I/O or state mutation;
- geometry recomputes only when terminal area, template, tree, constraints, or
  relevant component-view generation changes;
- ShellView cache key includes terminal area and all layout-authoritative
  generations;
- hit areas flatten once per computed ShellView;
- identical state produces no outgoing render frame;
- the client writer retains at most one pending render frame;
- control messages keep priority over render output;
- a static shell does not force full redraw for a dirty PTY row;
- mouse resize preview performs zero persistence writes and zero PTY resizes;
- resize commit performs one persistence dirty transition and at most one PTY
  resize per affected surface;
- idle shell state creates zero periodic render traffic;
- layout compute remains linear in bounded node count;
- no component owns an independent high-frequency timer/render loop.

### Measurement budgets

Hardware timing is recorded as environment-specific evidence, not a flaky
cross-machine unit assertion.

Reference-machine targets:

- maximum supported shell-tree geometry compute p95: at most 0.5 ms;
- 120x40 normal full frame preparation p95: at most 8 ms;
- 240x80 wide full frame preparation p95: at most 16 ms;
- pending render frames: at most 1;
- unchanged-state outgoing frames: 0;
- mouse-move PTY resize count before commit: 0;
- normal retained PTY character/row update: no shell-induced full redraw.

Deterministic CI tests assert counts, equality, cache invalidation, queue bounds,
and byte/path classification. Timing budgets run in an explicitly recorded
local performance profile and compare against the pre-change baseline.

Instrumentation should extend existing render profiling with shell compute
duration, node count, cache hit/miss, visible component count, resize preview
count, and resize commit count. Instrumentation must remain disabled or
negligible by default.

## Failure and Recovery Contract

Every failure path fails closed without corrupting runtime/session truth:

- invalid template/tree -> built-in safe template;
- invalid persisted constraint -> region default, unrelated state preserved;
- over-depth/over-count tree -> reject complete tree, no partial activation;
- terminal shrinks during drag -> cancel or revalidate from original commit;
- pointer release is lost -> bounded capture cleanup on the next lifecycle
  boundary;
- captured owner disappears -> clear capture and choose valid focus fallback;
- component/surface closes during input -> reject stale identity/generation;
- active surface fails to resolve -> render explicit unavailable stage and
  preserve unrelated runtime;
- overlay parent disappears -> close orphan overlay and choose template focus;
- scroll content shrinks -> clamp offset before geometry/hit projection;
- persistence writer fails -> keep current in-memory shell, report existing
  session-save failure path, do not hot retry in render;
- PTY resize fails after shell commit -> retain valid shell geometry, surface
  explicit runtime failure through existing owner, do not roll back unrelated
  region state;
- plugin app launch fails -> no phantom running instance and no focus theft.

## Security and Resource Contract

- shell/application IDs are bounded, validated, and never shell-interpreted;
- plugin commands continue through direct argv handling and existing capability
  checks;
- persisted layout data cannot allocate unbounded nodes, strings, stacks, or
  children;
- render and input handlers do not read filesystem/config/plugin manifests;
- application activation revalidates current definition and capability at the
  scheduled effect boundary;
- stale geometry cannot authorize file operation, terminal input, or plugin
  launch;
- no new dependency is required for Foundation v0 geometry, input, persistence,
  or rendering;
- no stable Herdr process/socket is addressed by automated or manual tests.

## Test Architecture Before Production Code

Characterization tests must be green before new behavior RED tests begin.
Compile, setup, tool, filter, environment, or flaky failures are not accepted
as behavioral RED evidence.

### SF1 characterization catalog

| ID | What is tested | Expected result | Reason |
|---|---|---|---|
| TP-SF1-DEFAULT-SHELL | Existing desktop sidebar plus center geometry | Current valid rects and configured sidebar behavior remain unchanged | Protect current composition before changing the layout engine |
| TP-SF1-MOBILE | Existing mobile header/menu/full-width surface | Current compact behavior remains bounded and usable | Prevent desktop foundation from regressing narrow terminals |
| TP-SF1-FILES-SWAP | Current Files versus terminal BaseLayer behavior | Exact current behavior is recorded, including the known curtain boundary | Separates preserved behavior from intentional migration delta |
| TP-SF1-SIDEBAR-RESIZE | Existing width clamp, collapse, section split, and reset | Existing valid semantics remain green | Reuse proven behavior rather than silently replacing it |
| TP-SF1-FRAME-SKIP | Repeated identical semantic frame | No second outgoing frame | Protect idle/SSH efficiency |
| TP-SF1-RETAINED-PTY | Single dirty PTY row in a safe state | Retained path matches the full-render frame without full redraw | Protect terminal typing latency |
| TP-SF1-WRITER-BOUND | Render writer queue under pressure | One pending render slot and prioritized control path | Protect backpressure and memory bounds |
| TP-SF1-LEGACY-SNAPSHOT | Current snapshot without shell state | Restore succeeds with exact current sidebar/session facts | Freeze migration input |

### SF2 geometry RED catalog

The first observed behavior RED is:

`shell_layout_places_dock_sidebar_stage_without_overlap`

It must fail because the current shell lacks AppDock plus WorkspaceStage
geometry, not because a type, import, fixture, command, or setup is broken.

| ID | What is tested | Expected result | Reason |
|---|---|---|---|
| TP-SF2-NAMED | AppDock, LeftPanel, WorkspaceStage layout | Complete disjoint deterministic rects | Establish the first real shell contract |
| TP-SF2-TRACKS | Fixed, bounded, resizable, fill, collapsed policies | Checked normalized allocations satisfy declared constraints | Prove the common geometry vocabulary |
| TP-SF2-EXHAUST | Aggregate minimum exceeds terminal width/height | Declared collapse priority then TooSmall fallback | Prevent invalid tiny-terminal geometry |
| TP-SF2-BOUNDS | Maximum depth, children, leaves, serialized nodes | Limit is accepted; one above is rejected completely | Enforce complexity/resource ceilings |
| TP-SF2-ZERO | Empty/collapsed/zero-area region | No rect-derived divider/focus/hit authority | Prevent invisible interactive surfaces |
| TP-SF2-CACHE | Same and changed layout generations | Same key reuses geometry; each authoritative change invalidates once | Prevent redundant compute and stale rects |
| TP-SF2-HIT-FLAT | Nested layout hit projection | Complete z-ordered stable-ID hit list generated once | Align render and input geometry |
| TP-SF2-RESPONSIVE | Every threshold below/exact/above | Deterministic mode and collapse transition | Prevent boundary flicker and squeezed content |

### SF3 interaction RED catalog

| ID | What is tested | Expected result | Reason |
|---|---|---|---|
| TP-SF3-DRAG-PREVIEW | Mouse down/move before release | Bounded preview only; zero persistence/PTY resize | Prevent disk churn, PTY reflow, and SSH amplification |
| TP-SF3-DRAG-COMMIT | Valid release | One normalized commit, one dirty transition, at most one PTY resize | Transaction correctness |
| TP-SF3-DRAG-CANCEL | Escape, invalid divider, owner removal, terminal resize | Original committed state restored or safely recomputed | Avoid stuck capture/corrupt ratios |
| TP-SF3-RESET | Divider double-click | Preferred size restored within current bounds | Predictable recovery from manual sizing |
| TP-SF3-COLLAPSE | Collapse, repeated collapse, expand after terminal shrink | Idempotent zero-size state and clamped retained restore size | Preserve user intent safely |
| TP-SF3-SCROLL | Horizontal/vertical overflow, shrink, zero area | Only owner offset changes and clamps; no stale scrollbar | Contain scroll authority |
| TP-SF3-KEYBOARD | Keyboard resize/collapse parity | Same constraint and focus rules as mouse | Accessibility and deterministic testing |
| TP-SF3-STALE | Old generation same-coordinate divider/hit | Consumed/rejected with no state mutation | Prevent coordinate reuse authority bugs |

### SF4 surface and input RED catalog

| ID | What is tested | Expected result | Reason |
|---|---|---|---|
| TP-SF4-FOCUS | Focus movement across visible/collapsed/removed components | Only valid declared owner receives focus; deterministic fallback | Prevent hidden focus owners |
| TP-SF4-CAPTURE | Pointer/scroll capture lifecycle | One owner; cleanup on all lifecycle boundaries | Prevent stuck or competing gestures |
| TP-SF4-OVERLAY | Blocking overlay above active surface | Overlay consumes input; background receives none; close restores valid focus | Correct the current curtain/input leak class |
| TP-SF4-SURFACE | Native/terminal surface selection | Exactly one active stage surface is rendered and actionable | Establish SurfaceHost authority |
| TP-SF4-UNKNOWN | Unknown input inside exclusive active surface | Fail-closed consumption, no terminal forwarding | Protect hidden terminal processes |
| TP-SF4-CLOSE | Active surface closes or becomes unavailable | Previous valid surface/focus or explicit empty stage | Lifecycle recovery without phantom instance |

### SF5 AppDock RED catalog

| ID | What is tested | Expected result | Reason |
|---|---|---|---|
| TP-SF5-GEOMETRY | 3/5/9-cell widths and tiny/zero areas | Complete display-cell-safe icons and row hit targets | Terminal-cell correctness |
| TP-SF5-POPOVER | Right-click current dock item | Anchored name/action popover; background input blocked as declared | Satisfy icon-only discoverability |
| TP-SF5-SINGLETON | Activate running singleton | Existing instance focuses; no duplicate process/tab | App lifecycle correctness |
| TP-SF5-LAUNCH-FAIL | Missing/unsupported/failing command | No phantom running state or focus theft | Failure truthfulness |
| TP-SF5-RESIZE | Dock preview/commit/collapse | Common SF3 contract reused without private arithmetic | Prove pattern reuse |
| TP-SF5-UNICODE | wide/missing icon glyph and fallback | Whole icon family falls back safely; no half-cell hit drift | Host/font portability |

### SF6 Files migration RED catalog

| ID | What is tested | Expected result | Reason |
|---|---|---|---|
| TP-SF6-STAGE | Files activates as NativeFiles in WorkspaceStage | Files owns complete stage; terminal chrome/hits are absent | Remove curtain behavior |
| TP-SF6-RUNTIME | Files open/close while terminal processes run | No unrelated pane/process create/delete/restart | Preserve runtime separation |
| TP-SF6-INPUT | Files active with hidden pane/tab coordinates | Hidden terminal routes receive zero input | Close the observed interaction bug |
| TP-SF6-OVERLAY | Files context/confirmation overlay | Topmost overlay owns input and restores Files focus | Preserve file operation safety |
| TP-SF6-SIDEBAR | Existing Files sidebar navigation and resize | Exact-path navigation and current styling remain green | Preserve mature FM behavior |
| TP-SF6-WORKERS | Watcher/preview/operation activity through surface switches | Current generation publishes; stale generation cannot paint or mutate | Preserve asynchronous lifecycle safety |
| TP-SF6-CLOSE | Close Files and return to previous app | Exact prior valid instance/focus restored without new runtime | Desktop-like app lifecycle |
| TP-SF6-MOBILE | Files stage at compact boundaries | Safe compact surface and navigation; no squeezed invalid columns | Platform/terminal resilience |

### Performance and full-gate catalog

| ID | What is tested | Expected result | Reason |
|---|---|---|---|
| TP-SFP-STATIC | Static shell plus PTY dirty row | Retained update remains eligible; no shell-induced full redraw | SSH typing latency |
| TP-SFP-IDLE | No state/PTY change | Zero outgoing frames and no high-frequency tick | Idle CPU/network budget |
| TP-SFP-QUEUE | Rapid shell invalidation under blocked writer | Pending render remains bounded at one; control remains prioritized | Backpressure safety |
| TP-SFP-DRAG | Long divider gesture | Zero pre-commit PTY resize/persistence; bounded render traffic | Prevent resize storms |
| TP-SFP-GEOMETRY | Maximum valid tree and one-over invalid tree | Linear valid compute and immediate bounded rejection | Algorithm/resource proof |
| TP-SFP-BASELINE | Pre/post timing and ANSI byte profile | No unexplained regression beyond recorded budget | Evidence-based optimization |
| TP-SFP-PLATFORMS | Linux and canonical Windows compile/lint paths | Every applicable warning-as-error gate passes | Cross-platform closure |
| TP-SFP-REPOSITORY | Full nextest, Bun, Python, fmt, diff, ignored inventory | Zero unreported fail/retry; skips and N/A named exactly | No background failures |

## Ordered Delivery Boundaries

This section establishes dependency order, not the code-level implementation
plan. The detailed plan is written only after user review of this specification.

### SF0 - Design and baseline freeze

- approve this specification;
- freeze current Git/diff/graph/test evidence;
- identify exact owned files and rollback boundary;
- perform broad-refactor characterization review before product edits.

### SF1 - Characterization

- add or identify every SF1 characterization test;
- run fresh green baseline;
- do not change production behavior.

### SF2 - Geometry foundation

- observe TP-SF2-NAMED RED first;
- add bounded tree validation, track constraints, responsive allocation,
  ShellView projection, cache key, and flattened hit geometry;
- do not render AppDock or migrate Files.

### SF3 - Shared interaction patterns

- implement resize preview/commit/cancel/reset test-first;
- implement collapse/restore and scroll ownership test-first;
- keep persistence commit-only and PTY resize one-shot.

### SF4 - SurfaceHost and input router

- establish focus/capture/overlay precedence;
- add native versus terminal stage-surface selection;
- prove background blocking and lifecycle cleanup.

### SF5 - AppDock real consumer

- add icon-only AppDock, right-click naming popover, running indicators,
  singleton focus, resize, and collapse;
- reuse SF2-SF4 contracts rather than introducing AppDock-private variants.

### SF6 - Files first native consumer

- migrate existing Files state/render/input into NativeFiles SurfaceHost;
- preserve all current FM workers/actions/failure behavior;
- remove hidden terminal interaction authority;
- close the shell foundation with performance and full-repository gates.

### FM-next - immediate product focus

After SF6 closure, resume FM UX in independent RED/GREEN slices:

1. bounded horizontal Miller path viewport;
2. Miller column separator resize using the shared gesture contract;
3. complete parent/current/directory-preview column mouse ownership;
4. path-stable Finder-like descent/return behavior;
5. evidence-based preview/inspector placement decision.

Apps/Desktop, additional dock apps, and real RightPanel/TopBar/BottomBar
consumers are subsequent independently approved increments and do not delay
FM-next.

## Git and Commit Discipline

- this specification is one documentation-only commit;
- characterization, RED, GREEN, refactor, migration, and performance closure
  remain separate atomic concerns;
- a failing RED checkpoint is never pushed as the branch publication tip;
- stage only declared files; never use `git add -A`;
- inspect staged names and staged diff before every commit;
- use lowercase conventional commit subjects without AI co-author lines;
- run fresh proportional verification before each commit and complete gates
  before publication;
- fetch and prove fast-forward ancestry before push;
- push only the CyPack fork refs authorized by project continuity;
- never force, never push `upstream`, and never open an upstream issue/PR;
- refresh Codebase Memory after committed product changes and prove freshness
  with new symbols rather than `ready` alone.

## Rollback Boundaries

- SF2 may fall back to the legacy compatibility template without changing
  runtime/session facts;
- SF3 interaction state is transient and can be disabled without rewriting the
  persisted tree;
- SF4 SurfaceHost can retain the current terminal surface as the only enabled
  host until NativeFiles is ready;
- SF5 AppDock can be disabled while Stage/LeftPanel compatibility remains;
- SF6 Files migration must remain independently reversible to the last fully
  green Files Workspace behavior during development;
- no phase deletes or rewrites mature FM watcher, preview, operation, selection,
  or handoff state as a rollback mechanism;
- persisted schema changes require fixtures proving downgrade-safe fallback or
  an explicit documented inability to consume future fields without corrupting
  older valid session data.

## Completion Criteria

Shell Foundation v0 is complete only when:

1. SF1-SF6 have behavior-specific observed RED and minimal GREEN evidence in
   dependency order;
2. default, mobile, tiny, zero, maximum-valid, and one-over-limit layouts are
   deterministic and panic-free;
3. resize/collapse/scroll patterns are shared by real consumers and have no
   private duplicate coordinate logic;
4. Files is a native WorkspaceStage surface and hidden terminals receive no
   input;
5. existing FM watcher, preview, operations, selection, plugins, and agent
   handoff remain green;
6. old snapshots migrate deterministically and invalid shell state fails to a
   safe template without losing unrelated session facts;
7. identical frames, retained PTY updates, bounded queue, idle traffic, and
   resize-transaction performance invariants pass;
8. environment-specific timing/byte baselines meet the declared budgets or a
   measured regression is resolved before closure;
9. Linux, canonical Windows, full nextest, Bun, Python, formatting, diff,
   ignored-test inventory, production-unwrap, and artifact gates are fresh and
   truthfully reported;
10. every commit is atomic, targeted, reviewable, and the permitted CyPack
    publication has exact remote-SHA evidence;
11. Codebase Memory is freshly indexed and returns the current Shell, Surface,
    input, persistence, AppDock, and NativeFiles symbols;
12. stable Herdr, inherited stable sockets, and unrelated user processes were
    never touched.

## A0-A7 Readiness Summary

- A0 intake and evidence boundary: complete;
- A1 goals, actors, scenarios, and measurable success: complete;
- A2 fractal decomposition: complete through SF0-SF6 and FM-next;
- A3 dimensional investigation: complete for architecture, state, layout,
  input, persistence, plugin boundary, performance, failure, and platform;
- A4 options and pattern decision: bounded core shell selected;
- A5 fresh Herdr cartography and fit: complete at the recorded graph;
- A6 target contract: frozen in this specification;
- A7 normalized handoff/readiness: complete through explicit user approval of
  this written specification and its twelve-phase direction.

I0-I5 are now represented by the approved program plan plus the two code-level
test-first implementation plans. The next executable delivery phase is I6:
run and, where required, strengthen SF1 characterization before observing the
first new SF2 behavior RED. Product code must not begin directly from this
design document or skip SF1.
