# Herdr Shell Foundation v0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:executing-plans` to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking. Do not dispatch subagents unless the
> user explicitly authorizes delegation.

**Goal:** Implement SF1-SF6: characterize current behavior, build bounded shell
geometry and interactions, introduce typed Stage surfaces and input routing,
render a real AppDock, and migrate Files into the Workspace Stage without
regressing runtime, PTY, persistence, or existing FM behavior.

**Architecture:** Keep `src/ui/shell.rs` as the public facade and split its
bounded model, templates, solver, view projection, and interaction reducers
under `src/ui/shell/`. Add one aggregate `ShellState` to `AppState` and one
aggregate `ShellView` to `ViewState`, not more loose fields. Route input through
semantic hit targets created during `compute_view`; render reads only frozen
view/state. Built-in `TerminalWorkspace` and `NativeFiles` are typed client
surfaces, not plugins and not wire identities.

**Tech Stack:** Rust 2024, Ratatui, Crossterm, Serde/JSON, existing Herdr
snapshot and render-prof systems, Cargo Nextest, Clippy, Bun/Python maintenance
tests, Codebase Memory MCP, Git. No new crate and no protocol change.

## Global Constraints

- Execute after the approved program plan and design specification.
- I0-I5 evidence is required before each phase's first RED; I6-I14 closes each
  phase.
- Keep `AppState` pure/testable without PTYs. Runtime changes belong in `App`
  adapters and existing registries.
- Keep render pure; every hit area and surface model is computed beforehand.
- Outer shell regions are exactly `TopBar`, `AppDock`, `LeftPanel`,
  `WorkspaceStage`, `RightPanel`, and `BottomBar`.
- Existing `CenterContent` call sites migrate to `WorkspaceStage` during SF2;
  a local compatibility projection/test preserves identical rectangles through
  SF6. `CenterContent` is not a v4 persisted region identity.
- Foundation v0 has typed built-in templates only. No arbitrary JSON/TOML DSL,
  runtime plugin tree, redocking, floating window, visual editor, or component
  render loop.
- Stage never collapses. Degradation order is optional clamp -> RightPanel ->
  LeftPanel compact -> AppDock -> Stage -> deterministic TooSmall view.
- Input precedence is overlay -> active capture -> topmost hit -> focused
  component -> active page shortcut -> global shortcut -> consumed/inert.
- Keyboard resize parity is mandatory: a focused divider uses axis arrow keys
  for one-cell preview, Enter to commit,
  and Escape to cancel. A focused collapse control uses Space/Enter to toggle.
  These semantic controls use the same reducer and bounds as mouse input.
- Resize preview performs zero persistence writes and zero PTY resizes. Commit
  performs one normalized state commit, one dirty mark, and at most one resize
  for the affected active surface.
- Invalid persisted shell data falls back to a safe default without discarding
  unrelated workspaces, tabs, panes, sidebar section split, or collapsed-space
  facts.
- Stable Herdr and inherited sockets remain untouched. Only isolated manual
  checks described by `.local/ISOLATED-DEV-TEST.md` are allowed.

---

## Frozen File Ownership

### Create

- `src/ui/shell/model.rs`
- `src/ui/shell/template.rs`
- `src/ui/shell/layout.rs`
- `src/ui/shell/view.rs`
- `src/ui/shell/interaction.rs`
- `src/app/input/shell.rs`
- `src/ui/surface_host.rs`
- `src/ui/app_dock.rs`

### Modify

- `src/ui/shell.rs` — facade, compatibility exports, existing tests.
- `src/ui.rs` — shell/surface projection and pure layer dispatch.
- `src/ui/compose.rs` — only if the typed surface host needs an additive pure
  render context; do not turn it into a mutable registry.
- `src/app/state.rs` — aggregate `ShellState`, `ShellInteractionState`,
  `StageState`, and `ShellView` fields plus invariants.
- `src/app/actions.rs` — typed built-in app activation/close adapters.
- `src/app/mod.rs` — module wiring, initialization, restore/config adapters.
- `src/app/input/mod.rs` — call the semantic shell router in frozen precedence.
- `src/app/input/mouse.rs` and `src/app/input/sidebar.rs` — migrate existing
  divider behavior into transactions without changing sidebar-section ownership.
- `src/app/input/file_manager.rs` — SF6 only: consume `NativeFiles` Stage targets.
- `src/app/session.rs` — capture aggregate shell presentation state.
- `src/persist/snapshot.rs` — v4 shell snapshot, migration, capture tests.
- `src/persist/restore.rs` — valid/fallback restore tests and adapters.
- `src/server/headless.rs`, `src/server/render_stream.rs`, and
  `src/server/client_transport.rs` — characterization/performance tests only
  unless fresh profiling proves a regression caused by this work.

No other path is authorized without a new I0 scope review. In particular, no
protocol/API schema, dependency manifest, platform module, release doc, vendor
file, or user-owned `.superpowers/` file is part of this plan.

## Frozen Public-Within-Crate Interfaces

### Geometry model

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum RegionId {
    TopBar,
    AppDock,
    LeftPanel,
    WorkspaceStage,
    RightPanel,
    BottomBar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum TrackPolicy {
    Fixed { cells: u16 },
    ContentBounded { min: u16, max: u16 },
    Resizable { min: u16, preferred: u16, max: u16 },
    Fill { weight: u16 },
    Collapsed { restore: u16 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ShellValidationError {
    EmptySplit,
    DepthExceeded,
    ChildrenExceeded,
    VisibleLeavesExceeded,
    SerializedNodesExceeded,
    StackChildrenExceeded,
    ComponentPlacementsExceeded,
    DuplicateComponentPlacement,
    DuplicateRegion(RegionId),
    InvalidTrackBounds,
    MissingWorkspaceStage,
    CollapsedWorkspaceStage,
}

pub(crate) enum ShellNode {
    SplitContainer {
        axis: ShellAxis,
        children: Vec<ShellChild>,
        divider: DividerPolicy,
    },
    RegionSlot(RegionSlot),
}

pub(crate) struct RegionSlot {
    pub id: RegionId,
    pub content: RegionContent,
}

pub(crate) enum RegionContent {
    Empty,
    NestedSplit(Box<ShellNode>),
    SurfaceHost(SurfaceHostId),
    StackContainer(StackContainer),
}

pub(crate) struct StackContainer {
    pub children: Vec<SurfaceHostId>,
    pub selected: usize,
}
```

`ShellLayout::validate()` returns `Result<ValidatedShellLayout,
ShellValidationError>`. Only `ValidatedShellLayout` reaches the solver.
Validation is iterative or depth-checked before recursion and rejects duplicate
outer regions. `StackContainer` selects from at most 32 typed hosts and has no
render loop. `OverlayHost` is a separate one-owner z-layer above the shell; it
does not consume persistent tree depth and does not introduce the deferred S7
nested popup stack.

### View projection

```rust
pub(crate) struct ShellGeometryKey {
    pub area: Rect,
    pub layout_revision: u64,
    pub constraints_revision: u64,
    pub collapse_revision: u64,
}

pub(crate) struct ShellView {
    pub generation: u64,
    pub area: Rect,
    pub regions: RegionRects,
    pub dividers: Vec<ResizeDividerView>,
    pub collapse_controls: Vec<CollapseControlView>,
    pub scroll_viewports: Vec<ScrollViewportView>,
    pub hits: Vec<ShellHitArea>,
    pub degradation: ResponsiveDegradation,
}
```

`compute_shell_view(layout, state, area, previous)` is pure. It reuses the
previous generation only when the complete geometry key is unchanged; otherwise
it recomputes in O(node_count), increments generation, and flattens semantic
hits once.

### Presentation and app state

```rust
pub(crate) struct ShellState {
    pub template: ShellTemplateId,
    pub root: ValidatedShellLayout,
    pub constraints: ShellConstraints,
    pub collapsed: BTreeMap<RegionId, CollapsedRegionState>,
    pub component_placements: ComponentPlacements,
    pub stage: StageState,
    pub dock: AppDockState,
    pub revision: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum BuiltInAppId {
    Terminal,
    Files,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum AppSurfaceRef {
    TerminalWorkspace,
    NativeFiles,
}

pub(crate) struct AppDefinition {
    pub id: BuiltInAppId,
    pub launch_policy: AppLaunchPolicy,
    pub surface: AppSurfaceRef,
}

pub(crate) struct AppInstanceId {
    pub app: BuiltInAppId,
    pub generation: u32,
}

pub(crate) struct AppInstance {
    pub id: AppInstanceId,
    pub surface: AppSurfaceRef,
}

pub(crate) struct StageState {
    pub instances: Vec<AppInstance>,
    pub active: AppInstanceId,
    pub previous: Option<AppInstanceId>,
}
```

These are TUI presentation identities. They never replace or serialize server
workspace/tab/pane/terminal IDs. Foundation v0 treats Terminal and Files as
singletons, keeps at most 16 typed built-in instances, and allocates a new
generation only after close/reopen. Apps/Desktop may later add richer instance
policy in its own plan.

### Input routing

```rust
pub(crate) enum ShellInputTarget {
    Overlay(OverlayTarget),
    Divider(DividerId),
    Collapse(RegionId),
    ScrollViewport(ViewportId),
    DockItem(BuiltInAppId),
    Stage(AppInstanceId),
    Region(RegionId),
}

pub(crate) enum ShellDispatch {
    Consumed,
    BeginResize(DividerId),
    PreviewResize { divider: DividerId, pointer: Position },
    CommitResize(DividerId),
    CancelResize(DividerId),
    ToggleCollapse(RegionId),
    Scroll { viewport: ViewportId, delta: i16 },
    ActivateApp(BuiltInAppId),
    ForwardToStage(AppInstanceId),
}
```

Routing consumes stale generations and unknown targets without state mutation.
It never re-derives geometry from coordinates and never reads the filesystem.

## SF1 — Characterization and Baseline

### Test points before any Foundation production behavior

| Test | Expected | Reason |
|---|---|---|
| `default_matches_legacy_outer_split_exactly` | Existing desktop sidebar/main rectangles remain byte-for-byte equal | Protect current outer geometry |
| `desktop_shell_regions_match_computed_geometry` | `ViewState` matches shell projection | Protect state/view wiring |
| `mobile_view_leaves_shell_regions_empty` | Existing mobile full-width contract remains unchanged in SF1 | Prevent accidental desktop shell injection |
| `open_file_manager_renders_directory_list_in_center` | Current curtain/swap behavior is explicitly recorded, not approved target | Gives SF6 a truthful before-state |
| `native_fm_composes_sidebar_breakpoints_and_status_across_full_frames` | Desktop/collapsed/mobile Files composition stays stable before migration | Protect existing FM feature set |
| `old_snapshot_defaults_sidebar_fields` and `capture_contract_tracks_sidebar_state` | v3 fields retain old defaults/capture | Protect migration source |
| `render_and_stream_skips_identical_frame_sends` | Second identical frame sends nothing | Preserve transport dedup |
| `retained_pty_update_streams_dirty_row_from_last_frame` | One dirty terminal row uses retained patch | Preserve PTY hot path |
| `client_writer_queue_keeps_render_slot_bounded` | Second pending render returns Full; queue never grows | Preserve bounded backpressure |

### Task SF1.1: Freeze focused characterization

**Files:** `src/ui/shell.rs`, `src/ui.rs`, `src/persist/snapshot.rs`,
`src/server/headless.rs`, `src/server/client_transport.rs` tests only.

- [x] Run Codebase Memory `search_graph` and `get_code_snippet` for every test
  and production symbol named above; record graph counts and current SHA.
- [x] Run the existing focused baseline:

  ```bash
  cargo nextest run --locked -E 'test(/(default_matches_legacy_outer_split_exactly|desktop_shell_regions_match_computed_geometry|mobile_view_leaves_shell_regions_empty|open_file_manager_renders_directory_list_in_center|native_fm_composes_sidebar_breakpoints_and_status_across_full_frames|old_snapshot_defaults_sidebar_fields|capture_contract_tracks_sidebar_state|render_and_stream_skips_identical_frame_sends|retained_pty_update_streams_dirty_row_from_last_frame|client_writer_queue_keeps_render_slot_bounded)/)' --status-level fail --final-status-level fail --failure-output final --success-output never
  ```

  Expected: all selected tests pass; zero tests selected is failure.
- [x] Add missing assertions only where the current test does not prove the
  named behavior. Do not change production behavior.
- [x] Add characterization `files_curtain_currently_replaces_terminal_surface`
  that proves the current state: FM content occupies `terminal_area`, terminal
  pane content is absent, and the terminal runtime registry is unchanged.
- [x] Run the characterization and require GREEN because SF1 freezes existing
  behavior rather than introducing target behavior.
- [x] Run full baseline nextest and direct maintenance gates.
- [x] Targeted-stage test files only and commit:
  `test: characterize shell foundation baseline`.

Closure evidence: `.codex/evidence/shell-foundation-sf1-characterization.md`.

## SF2 — Shell Geometry Foundation

### Task SF2.1: Observe the first new geometry RED

**Files:** `src/ui/shell.rs` tests only.

- [ ] Add exactly one compile-valid test named
  `shell_layout_places_dock_sidebar_stage_without_overlap` using serialized
  `ShellLayout` input containing `AppDock`, `LeftPanel`, and `WorkspaceStage`.
  It must use existing public-within-module APIs and inspect the private rect
  map from the test module, so missing enum support fails at the behavior
  assertion/deserialize expectation rather than compilation.
- [ ] Assert at area `120x40` with dock `5`, sidebar `26`, and fill stage:
  three regions exist; x/width are `(0,5)`, `(5,26)`, `(31,89)`; all heights
  equal 40; pairwise intersections are empty; the union equals the input area.
- [ ] Run:

  ```bash
  cargo nextest run --locked -E 'test(shell_layout_places_dock_sidebar_stage_without_overlap)' --status-level fail --final-status-level fail --failure-output final --success-output never
  ```

  Expected RED: compilation succeeds and the test fails only because the new
  named regions/track contract is not accepted or not allocated.
- [ ] Targeted-stage `src/ui/shell.rs`; commit
  `test: define shell workspace geometry contract`.

### Task SF2.2: Add bounded model and typed templates

**Files:** create `src/ui/shell/model.rs`, `src/ui/shell/template.rs`; modify
`src/ui/shell.rs`.

- [ ] Move existing model types behind facade re-exports without behavior
  change; keep existing serde round trip green.
- [ ] Add the frozen `RegionId`, `TrackPolicy`, validation errors, limits, and
  `ValidatedShellLayout` interfaces.
- [ ] Add typed `ShellTemplateId::{StageOnly,DockStage,DockSidebarStage,
  DesktopWorkspace,InspectorWorkspace}` and constructors. Users cannot submit
  arbitrary trees in v0.
- [ ] Make `DesktopWorkspace` produce top/bottom fixed tracks and a horizontal
  body of AppDock/LeftPanel/WorkspaceStage/RightPanel. Empty optional top,
  right, and bottom hosts collapse to zero without reserving separators.
- [ ] Add RED/GREEN tests:
  `shell_rejects_depth_above_four`, `shell_rejects_more_than_eight_split_children`,
  `shell_rejects_more_than_sixty_four_visible_leaves`,
  `shell_rejects_more_than_one_hundred_twenty_eight_serialized_nodes`,
  `shell_rejects_more_than_thirty_two_stack_children`,
  `shell_rejects_more_than_sixty_four_component_placements`,
  `shell_rejects_duplicate_component_placement`,
  `shell_rejects_duplicate_outer_region`,
  `shell_rejects_collapsed_or_missing_stage`, and
  `typed_templates_validate_without_runtime_registry`.
- [ ] Require each RED to fail at its named validation assertion, then implement
  only the matching validation branch and rerun GREEN.
- [ ] Run shell family:

  ```bash
  cargo nextest run --locked -E 'test(/shell/)' --status-level fail --final-status-level fail --failure-output final --success-output never
  ```

- [ ] Commit RED tests separately as
  `test: define bounded shell model contracts`; commit GREEN as
  `feat: add bounded shell model and templates`.

### Task SF2.3: Implement deterministic solver and degradation

**Files:** create `src/ui/shell/layout.rs`; modify facade and tests.

- [ ] Add RED tests for all track policies and arithmetic edges:
  `fixed_track_uses_exact_cells_or_available_space`,
  `content_bounded_clamps_measurement`,
  `resizable_track_clamps_preferred`,
  `fill_weights_split_only_remaining_cells`,
  `collapsed_track_is_zero_and_keeps_restore_width`,
  `zero_area_never_underflows`,
  `allocation_remainder_is_deterministic`, and
  `all_rects_are_inside_parent_without_overlap`.
- [ ] Add table-driven tiny-terminal test
  `shell_degrades_in_frozen_priority_order` for widths around every threshold.
  Expected: RightPanel disappears first, LeftPanel becomes compact, AppDock
  collapses next, Stage receives every remaining usable cell, then TooSmall is
  explicit; no negative/overflow rect exists.
- [ ] Implement one O(node_count) measure/allocate pass with saturating checked
  arithmetic. Never loop once per terminal cell and never search siblings from
  inside the child loop.
- [ ] Add a test-only visit counter proving each validated node is measured and
  allocated a bounded constant number of times.
- [ ] Run exact tests, shell family, and `cargo clippy --all-targets --locked --
  -D warnings`.
- [ ] Commit tests as `test: define shell allocation and degradation`; commit
  product as `feat: add deterministic shell layout solver`.

### Task SF2.4: Project cached ShellView and flattened hits

**Files:** create `src/ui/shell/view.rs`; modify `src/app/state.rs`, `src/ui.rs`.

- [ ] Add one aggregate `ShellView` to `ViewState`; do not add one field per
  region/interaction to `AppState`.
- [ ] Add RED tests:
  `unchanged_geometry_key_reuses_shell_generation`,
  `area_or_constraint_change_advances_shell_generation_once`,
  `flattened_hits_are_complete_disjoint_and_in_bounds`,
  `stale_shell_hit_generation_is_rejected`, and
  `legacy_sidebar_and_center_rects_match_compatibility_projection`.
- [ ] Extract `compute_shell_view` from `compute_view_internal`. During SF2,
  map WorkspaceStage back to the existing center/terminal flow and keep visible
  output unchanged.
- [ ] Clear absent region hits and zero-area hits on every projection.
- [ ] Run the exact new tests, all `src/ui.rs` tests, Linux Clippy, and the SF1
  characterization set.
- [ ] Commit RED tests as `test: define shell view projection contracts`;
  commit GREEN as `refactor: project shell geometry through bounded view`.

## SF3 — Resize, Collapse, Scroll, and Persistence

### Task SF3.1: Transactional resize reducer

**Files:** create `src/ui/shell/interaction.rs`; modify `src/app/state.rs`,
`src/app/input/shell.rs`, `src/app/input/mod.rs`, `src/app/input/mouse.rs`.

- [ ] Define pure `ResizeTransaction` with divider ID, view generation,
  pointer origin, original normalized tracks, and transient preview tracks.
- [ ] Add RED tests:
  `divider_down_captures_original_constraints`,
  `drag_preview_clamps_without_dirty_or_pty_resize`,
  `drag_commit_marks_dirty_once_and_requests_at_most_one_resize`,
  `divider_double_click_resets_to_preferred_once`,
  `keyboard_resize_uses_same_clamp_preview_and_commit_path`,
  `escape_restores_original_constraints`,
  `terminal_resize_cancels_and_recomputes_from_original`,
  `stale_divider_generation_is_consumed_inert`, and
  `mouse_up_without_capture_is_inert`.
- [ ] Use counters/test actions rather than a real PTY or disk. Expected preview
  counts are exactly zero; commit counts are exactly one dirty mark and zero or
  one resize request.
- [ ] Route focused axis-arrow/Enter/Escape keyboard intents through the same
  `ResizeTransaction`; do not add a keyboard-only sizing path.
- [ ] Adapt the existing sidebar outer divider to the transaction. Preserve
  `sidebar_section_split` as a sidebar-internal interaction and keep its current
  semantics until a separately scoped nested-container migration.
- [ ] Run sidebar drag tests and new transaction tests.
- [ ] Commit RED as `test: define transactional shell resize`; GREEN as
  `feat: add transactional shell resize`.

### Task SF3.2: Collapse/restore and scroll ownership

**Files:** `src/ui/shell/interaction.rs`, `src/app/state.rs`,
`src/app/input/shell.rs`, `src/ui/shell/view.rs`.

- [ ] Add RED tests:
  `collapse_remembers_last_committed_width`,
  `expand_clamps_restore_width_to_current_bounds`,
  `keyboard_collapse_matches_mouse_and_preserves_valid_focus`,
  `stage_collapse_is_rejected`,
  `empty_optional_region_collapses_to_zero`,
  `scroll_changes_only_topmost_owning_viewport`,
  `horizontal_and_vertical_offsets_clamp`,
  `content_shrink_clamps_stale_scroll_offset`, and
  `zero_area_viewport_consumes_without_mutation`.
- [ ] Implement collapse and scroll as state reducers. Scrolling belongs to the
  owning component/container, never the entire shell tree.
- [ ] Ensure collapse/expand changes one revision and one session dirty mark;
  repeated no-op actions do neither.
- [ ] Run focused and shell/input families; commit RED and GREEN separately:
  `test: define shell collapse and scroll ownership`, then
  `feat: add shell collapse and scroll patterns`.

### Task SF3.3: Snapshot v4 and corruption containment

**Files:** `src/persist/snapshot.rs`, `src/persist/restore.rs`,
`src/app/session.rs`, `src/app/mod.rs`, `src/app/state.rs`.

- [ ] Confirm again that latest released tag and HEAD both use snapshot 3. Do
  not bump the wire `PROTOCOL_VERSION` because no server/client message changes.
- [ ] Define `ShellSnapshotV1` with template ID, validated internal bounded root,
  region constraints, component placements, collapse restore widths, and
  pinned built-in dock order. This is Herdr-owned internal persistence, not a
  user-authored layout DSL. Do not persist transient capture, focus, view
  generation, active overlay, Files worker state, or active stage instance.
- [ ] Read raw `shell` as `Option<serde_json::Value>` so malformed shell data
  can be isolated and defaulted without failing unrelated session restore.
- [ ] Add RED tests:
  `v3_snapshot_migrates_sidebar_width_into_left_panel`,
  `v3_sidebar_section_split_remains_sidebar_owned`,
  `v4_shell_round_trip_is_idempotent`,
  `invalid_v4_shell_falls_back_without_losing_workspaces`,
  `over_limit_v4_shell_falls_back_safely`,
  `duplicate_or_unknown_component_placement_falls_back_safely`,
  `unknown_template_falls_back_safely`,
  `future_snapshot_version_is_still_rejected`,
  `resize_preview_is_not_captured`, and
  `resize_commit_is_captured_once`.
- [ ] Bump `SNAPSHOT_VERSION` from 3 to 4 only in the GREEN slice and update all
  explicit fixtures/constructors.
- [ ] Restore old `sidebar_width` to `LeftPanel` preferred width; retain
  `sidebar_section_split` unchanged. If both valid v4 shell and legacy width
  exist, v4 is authoritative and legacy remains readable for downgrade audit.
- [ ] Run snapshot/restore/session tests, full nextest, Linux and Windows
  Clippy before commit.
- [ ] Commit RED as `test: define shell snapshot migration contracts`; GREEN
  as `feat: persist versioned shell presentation state`.

## SF4 — SurfaceHost and Input Router

### Task SF4.1: Typed Stage surface state

**Files:** create `src/ui/surface_host.rs`; modify `src/app/state.rs`,
`src/app/actions.rs`, `src/app/mod.rs`, `src/ui.rs`.

- [ ] Add `AppDefinition`, bounded `AppInstance`, `AppInstanceId`,
  `BuiltInAppId`, `AppSurfaceRef`, `StageState`, and typed surface view model. Do
  not introduce arbitrary trait-object registration or server IDs.
- [ ] Add RED tests:
  `stage_starts_on_terminal_workspace`,
  `activating_files_records_previous_surface`,
  `reactivating_singleton_files_keeps_one_surface`,
  `stage_rejects_more_than_sixteen_builtin_instances`,
  `instance_generation_exhaustion_fails_without_aliasing`,
  `closing_files_restores_previous_terminal_surface`,
  `failed_files_open_restores_previous_surface_and_focus`, and
  `stage_surface_switch_does_not_destroy_terminal_runtime`.
- [ ] Implement pure state transitions first; use a test-owned fake runtime
  count for lifecycle proof.
- [ ] Keep existing `FmState` storage temporarily while Stage becomes its
  authority marker; SF6 completes render/input migration.
- [ ] Commit RED as `test: define typed stage surface lifecycle`; GREEN as
  `feat: add typed workspace stage surfaces`.

### Task SF4.2: Focus scopes, capture, and semantic input precedence

**Files:** create/modify `src/app/input/shell.rs`; modify
`src/app/input/mod.rs`, `src/app/input/overlays.rs`, `src/app/state.rs`.

- [ ] Add table-driven RED test
  `shell_input_router_follows_frozen_precedence` covering overlay, active
  capture, overlapping topmost hit, focused component, page shortcut, global
  shortcut, and no target.
- [ ] Add RED tests for:
  `overlay_blocks_every_background_mouse_action`,
  `overlay_blocks_background_keyboard_shortcut`,
  `capture_owns_move_and_up_outside_original_rect`,
  `focus_restores_after_overlay_close`,
  `collapsed_or_inert_region_cannot_receive_focus`,
  `stale_hit_generation_fails_closed`, and
  `files_stage_blocks_hidden_terminal_input`.
- [ ] Extract coordinate-to-semantic-target resolution into the frozen
  `ShellView` hit list. `handle_mouse_without_agent_frame_action` dispatches
  the returned semantic action rather than repeating coordinate branches.
- [ ] Keep existing overlay render order; make ownership explicit and total.
  An outside click may close an overlay if its contract says so but must not
  also activate the background in the same event.
- [ ] Run overlay, FM input, sidebar, terminal input, and router tests.
- [ ] Commit RED as `test: define shell focus and input ownership`; GREEN as
  `feat: route shell input through semantic ownership`.

### Task SF4.3: Cross-layer surface projection and render purity

**Files:** `src/ui.rs`, `src/ui/surface_host.rs`, `src/ui/compose.rs`, tests.

- [ ] Add RED tests:
  `active_surface_alone_populates_stage_hits`,
  `hidden_surface_has_no_stale_hits_or_cursor`,
  `surface_render_is_deterministic_for_identical_state`,
  `surface_render_does_not_mutate_app_state`, and
  `terminal_dirty_row_keeps_retained_path_with_static_shell`.
- [ ] Split `compute_view_internal` into shell projection plus active-surface
  projection. Terminal pane/split/hit geometry is computed only for
  `TerminalWorkspace`; Files geometry only for `NativeFiles`.
- [ ] Keep `Compositor` as pure ordered layers. `SurfaceHost` chooses a typed
  renderer from already-computed active surface state.
- [ ] Run full UI and retained-render tests; commit RED and GREEN separately.

## SF5 — AppDock

### Task SF5.1: Dock model, geometry, and pure render

**Files:** create `src/ui/app_dock.rs`; modify `src/app/state.rs`,
`src/ui/shell/template.rs`, `src/ui.rs`.

- [ ] Freeze AppDock size policy: preferred 5 cells, min 3, max 9. Default
  icon-only entries are Terminal and Files; names are available to accessible
  text/popover models, not permanently rendered beside icons.
- [ ] Add RED tests:
  `app_dock_defaults_to_five_cells`,
  `app_dock_renders_icon_only_terminal_and_files`,
  `app_dock_marks_active_and_running_states`,
  `app_dock_hits_are_complete_and_stable`,
  `app_dock_narrow_mode_preserves_distinct_targets`,
  `app_dock_collapses_before_stage`, and
  `zero_height_dock_is_panic_free_and_inert`.
- [ ] Use existing palette/tokens and Unicode-safe cell width handling. Provide
  ASCII-safe fallback symbols in tests.
- [ ] Run deterministic buffer tests at widths 3, 5, 9 and heights 0, 1, 8,
  then full UI tests.
- [ ] Commit RED as `test: define native app dock presentation`; GREEN as
  `feat: render bounded native app dock`.

### Task SF5.2: Dock interaction and anchored app-name popover

**Files:** `src/ui/app_dock.rs`, `src/app/input/shell.rs`,
`src/app/actions.rs`, `src/app/state.rs`, `src/ui.rs`.

- [ ] Add RED tests:
  `left_click_files_activates_existing_singleton_or_opens_one`,
  `left_click_terminal_restores_terminal_stage`,
  `right_click_opens_bounded_name_popover`,
  `popover_blocks_background_input`,
  `popover_reanchors_or_closes_after_terminal_resize`,
  `disabled_app_target_is_consumed_without_activation`, and
  `dock_resize_and_collapse_use_shared_transaction`.
- [ ] Use the SF3 resize/collapse reducer; do not add dock-specific drag state.
- [ ] Right-click popover is an overlay with topmost ownership and a bounded
  rect clamped to the current terminal. Outside close consumes the event.
- [ ] Run dock/router/overlay tests and full UI tests; commit RED and GREEN.

## SF6 — Migrate Files into Workspace Stage

### Task SF6.1: Move Files render projection out of terminal curtain

**Files:** `src/ui.rs`, `src/ui/surface_host.rs`,
`src/ui/file_manager.rs`, `src/app/state.rs`.

- [ ] Replace the SF1 characterization with target RED
  `files_renders_as_native_workspace_stage_surface`.
- [ ] Assert: active `NativeFiles` owns exactly `WorkspaceStage`; Files content
  is clipped to Stage; AppDock and LeftPanel remain separately rendered;
  terminal pane text/hits/cursor are absent; terminal runtime count and process
  state are unchanged.
- [ ] Add responsive RED tests for desktop, collapsed sidebar, tiny terminal,
  and current mobile contract. If mobile remains a dedicated full-width Stage,
  make that explicit and deterministic rather than silently applying desktop
  regions.
- [ ] Render Files through `SurfaceHost`; delete the direct
  `if app.file_manager.is_some()` curtain branch only after the new tests pass.
- [ ] Keep all filesystem-derived FM context in `FmState`; render receives only
  state and rectangles.
- [ ] Commit RED as `test: define Files workspace stage rendering`; GREEN as
  `feat: host Files in the native workspace stage`.

### Task SF6.2: Migrate Files lifecycle and input without losing features

**Files:** `src/app/actions.rs`, `src/app/input/file_manager.rs`,
`src/app/input/shell.rs`, `src/app/file_manager_watcher.rs`,
`src/app/file_preview_worker.rs`, `src/app/image_preview_worker.rs`,
`src/app/file_operation_worker.rs`, `src/app/state.rs` tests/adapters only as
needed.

- [ ] Add RED lifecycle tests for open, repeated activation, close, watcher init
  failure, worker panic/disconnect, operation running, context menu open,
  selected-path deletion, and reopen.
- [ ] Expected: one Files singleton; existing state/workers remain bound to it;
  close cancels/clears only existing documented Files-owned work; previous
  Stage/focus restores; failures never expose hidden terminal input or leave a
  phantom active Files instance.
- [ ] Route Files keyboard and mouse only when `AppSurfaceRef::NativeFiles` is
  active. Consume stale Files hits after surface switch.
- [ ] Preserve context menu/delete/rename/attachment/agent-handoff authority
  and all stable path revalidation. No operation executes from render or raw
  coordinate identity.
- [ ] Run:

  ```bash
  cargo nextest run --locked -E 'test(/(file_manager|file_operation|file_preview|image_preview|watcher|stage_surface|app_dock|shell_input)/)' --status-level fail --final-status-level fail --failure-output final --success-output never
  ```

  Expected: every selected test passes, zero retries, no unexpected skip.
- [ ] Commit RED as `test: define Files stage lifecycle and ownership`; GREEN as
  `feat: migrate Files lifecycle to workspace stage`.

### Task SF6.3: Performance, failure, migration, and isolated closure

**Files:** tests/instrumentation only unless profiling identifies a new
Foundation regression.

- [ ] Add or extend counters for shell compute duration, geometry cache hit,
  resize preview/commit, PTY resize requests, full/retained render selection,
  identical-frame skip, and outgoing bytes. Keep metrics bounded and disabled
  from unbounded accumulation.
- [ ] Benchmark named workloads: idle terminal, one dirty PTY row, AppDock
  activation, Files open, divider drag with 100 move events, collapse/expand,
  overlay open/close, and terminal sizes 80x24, 120x40, 240x80.
- [ ] Require p95 budgets from the program plan and prove 100 preview moves
  produce zero persistence writes and zero PTY resize requests.
- [ ] Run v3->v4 migration, invalid-v4 fallback, future-version rejection,
  overlay leakage, queue-full retry, retained dirty-row, watcher fallback,
  worker failure, and Files close/reopen failure families.
- [ ] Run the full direct `just check` equivalent from the program plan.
- [ ] Build the debug binary, then follow `.local/ISOLATED-DEV-TEST.md`: clear
  both socket variables plus `HERDR_ENV`, workspace/tab/pane IDs; use unique
  throwaway XDG roots; start only test-owned server/client processes; open
  Files through AppDock; prove it owns Stage and the terminal underneath is
  inert; resize/collapse; open/close overlay; return to Terminal; exit
  semantically; require zero test socket/process/root residue.
- [ ] Never close or restart stable Herdr or the user's terminal/browser.
- [ ] Commit any test-only closure additions as
  `test: verify shell foundation integration and performance`.

## Phase Completion and Publication

For each SF phase:

- [ ] Update its checkboxes and evidence in `.codex/TASKS.md` only after fresh
  commands finish.
- [ ] Run `git diff --check`, `git status --short --branch`, and targeted staged
  path audit.
- [ ] Verify no `.superpowers/`, protocol/API, dependency, release, vendor, or
  unrelated file is staged.
- [ ] Fetch `origin`, prove fast-forward ancestry, push only CyPack feature/fork
  refs authorized by continuity, verify remote SHA equality, never push
  `upstream`.
- [ ] Refresh Codebase Memory after the published checkpoint. If native
  parallel extraction fails, do not restart/kill the proxy; use the supported
  `CBM_WORKERS=1` CLI fallback and prove freshness with exact new symbols.

SF6 is complete only when the full repository gate is green, snapshot v4
migration is proven, Files no longer renders as a terminal curtain, hidden
terminal input is blocked, previous Stage/focus restoration is deterministic,
render/queue/retained-PTY budgets hold, isolated runtime proof leaves no
residue, atomic commits and remote SHAs are audited, and fresh graph snippets
show the final ShellView, router, AppDock, and NativeFiles surface symbols.
