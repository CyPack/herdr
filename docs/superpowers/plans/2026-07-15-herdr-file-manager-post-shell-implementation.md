# Herdr File Manager Post-Shell Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:executing-plans` to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking. Do not dispatch subagents unless the
> user explicitly authorizes delegation.

**Goal:** After SF6 is verified and published, deliver the five approved
FM-next increments: horizontal Miller viewport, transactional column resize,
mouse ownership in every rendered column, Finder-like path-stable growing
navigation, and an evidence-backed preview/inspector placement decision.

**Architecture:** Extend `FmState` with a bounded logical path chain and a
bounded resident directory-projection cache. Project columns and semantic hit
targets during `compute_view`; render remains filesystem-free. Reuse Shell
Foundation resize, capture, scroll, focus, overlay, and generation contracts.
Every actionable row carries directory path, entry path, column identity, and
view generation and is revalidated before mutation.

**Tech Stack:** Rust 2024, Ratatui, Crossterm, existing FM watcher/text/image
workers and Shell Foundation reducers, Cargo Nextest, render-prof, Clippy,
Bun/Python maintenance tests, Codebase Memory MCP, Git. No new dependency,
wire protocol, watcher backend, or persistence schema is planned.

## Entry Gate

FM1 cannot start until all SF6 completion evidence is fresh:

- Files is the active `NativeFiles` Workspace Stage surface, not a terminal
  curtain.
- AppDock/LeftPanel/Stage focus and mouse ownership are deterministic.
- Overlay-first blocking, resize transactions, horizontal scroll primitives,
  snapshot v4 migration, retained PTY, bounded render queue, and full gates are
  green.
- CyPack remote SHAs match and Codebase Memory contains final SF symbols.

If any item is missing, return to the owning SF task. Do not duplicate a Shell
primitive inside FM.

## Global Constraints

- FM-next is exactly five phases. Apps/Desktop, arbitrary app plugins, general
  right-panel consumers, new file operations, tabs-as-apps redesign, and
  visual layout editing are out of scope.
- Preserve all published FM behavior and failure paths: watcher fallback,
  selection authority, text/image preview generations, operation worker,
  rename/delete/context menu, agent handoff, hidden toggle, path-stable leave,
  and close/reopen cleanup.
- Render and hit routing perform no filesystem I/O. Directory refresh happens
  in `FmState`/App refresh paths before projection.
- Keep one watcher bound to current `cwd`; do not multiply native watchers per
  visible ancestor. Non-current column actions perform live path revalidation
  and fail closed when stale.
- Logical history depth is at most 32 directory segments. At most five complete
  directory projections are resident: the existing current `FmState.entries`
  plus at most four non-current cached projections. Eviction never removes the
  active current projection and never leaves a hit target pointing at an
  evicted generation.
- A resident projection may contain the existing complete sorted directory
  entry list, but no new unbounded per-entry index, duplicate list, history, or
  queue is allowed. Each resident directory owns exactly one entry vector.
- At most five Miller columns render in one frame. Additional logical columns
  are reached through the horizontal viewport.
- Column widths are min 16, preferred 28, max 64 cells. Stage geometry may
  reduce visible column count but never creates width below 1 or hides the
  focused column without a deterministic viewport adjustment.
- Column widths and history are client-local to one Files singleton and are not
  persisted in snapshot v4. Close/reopen rebuilds from current opening cwd.
- All Shell resize rules apply: preview writes no persistence, schedules no
  directory read, resubmits no image target, and resizes no PTY; commit changes
  one preference revision and may schedule one image target update.
- Stale column, row, divider, scroll, preview, worker, watcher, and overlay
  generations are consumed inertly.
- No stable process/socket contact; manual checks use only the isolated recipe.

---

## Frozen File Ownership

### Create

- `src/fm/miller.rs` — logical chain, column identity, resident cache, pure
  transitions and bounds.
- `src/ui/file_manager/miller.rs` — pure column geometry, viewport projection,
  semantic hit creation, and render helpers.
- `src/app/file_manager_miller.rs` — App refresh/controller adapter that loads,
  revalidates, and evicts projections outside render.

### Modify

- `src/fm/mod.rs` — expose `MillerState` as one aggregate inside `FmState` and
  reuse existing directory snapshot/loading helpers.
- `src/ui/file_manager.rs` — facade and `NativeFiles` surface composition.
- `src/ui.rs` — active Files Stage projection only; no new filesystem calls.
- `src/app/state.rs` — typed Miller view/hit snapshots inside the existing
  Files view aggregate; avoid loose AppState fields.
- `src/app/mod.rs` — controller lifecycle and scheduled refresh wiring.
- `src/app/actions.rs` — Files close/reopen reset and exact app lifecycle.
- `src/app/input/file_manager.rs` — semantic all-column dispatch.
- `src/app/input/shell.rs` — reuse capture/scroll routing; no FM-specific global
  precedence.
- `src/app/file_manager_watcher.rs` — current-cwd generation reconciliation
  only where the chain must prune stale descendants.
- `src/app/file_preview_worker.rs` and `src/app/image_preview_worker.rs` — tests
  or one committed target refresh adapter only; no worker redesign.

No protocol/API, dependency, snapshot, platform, release, vendor, stable docs,
or `.superpowers/` path is authorized.

## Frozen Interfaces and Bounds

```rust
pub(crate) const MAX_MILLER_HISTORY_DEPTH: usize = 32;
pub(crate) const MAX_RESIDENT_MILLER_COLUMNS: usize = 5;
pub(crate) const MILLER_COLUMN_MIN_WIDTH: u16 = 16;
pub(crate) const MILLER_COLUMN_PREFERRED_WIDTH: u16 = 28;
pub(crate) const MILLER_COLUMN_MAX_WIDTH: u16 = 64;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct MillerColumnId {
    pub directory: PathBuf,
    pub generation: u64,
}

pub(crate) struct MillerPathSegment {
    pub directory: PathBuf,
    pub focused_child: Option<PathBuf>,
    pub cursor: usize,
    pub viewport_start: usize,
    pub preferred_width: u16,
}

pub(crate) struct MillerDirectoryProjection {
    pub id: MillerColumnId,
    pub entries: Vec<FileEntry>,
    pub status: FmDirectoryStatus,
    pub writable: bool,
}

pub(crate) struct MillerState {
    pub chain: VecDeque<MillerPathSegment>,
    pub resident_non_current: VecDeque<MillerDirectoryProjection>,
    pub horizontal: MillerHorizontalViewport,
    pub focused_directory: PathBuf,
    pub revision: u64,
}
```

`MillerState::validate()` asserts all bounds, unique path identity within the
active chain, focused directory membership, at most four unique non-current
cached projections, and exact current `FmState` membership outside that cache.
Production transitions maintain these invariants with fallible/total results;
tests use `FmState::assert_miller_invariants_for_test()`.

```rust
pub(crate) struct MillerRowTarget {
    pub shell_generation: u64,
    pub files_generation: u64,
    pub column: MillerColumnId,
    pub directory_path: PathBuf,
    pub entry_index: usize,
    pub entry_path: PathBuf,
    pub rect: Rect,
}

pub(crate) struct MillerDividerTarget {
    pub shell_generation: u64,
    pub files_generation: u64,
    pub left: MillerColumnId,
    pub right: MillerColumnId,
    pub rect: Rect,
}
```

Before an action, the controller verifies every generation, the current or
cached column ID and directory path, the entry index, and exact entry path. For
non-current columns it additionally reloads/revalidates the directory in the
state refresh path. Any mismatch returns `ConsumedStale` with no cursor,
selection, chain, operation, context menu, or filesystem mutation.

## FM1 — Horizontal Miller Viewport

### Test-point catalog

| Test | Expected | Reason |
|---|---|---|
| History seed | Opening a deep cwd retains nearest at most 32 ancestor segments and current cwd | Deep paths must not grow memory without bound |
| Resident cache | At most five unique complete directory projections; active current never evicted | Bound the new memory multiplier |
| Horizontal geometry | 1-5 complete disjoint columns plus dividers, focused column visible | Deterministic responsive behavior |
| Horizontal scroll | Native left/right wheel, Shift+wheel, and header arrows clamp to valid window | Reach hidden path segments without leaking vertical scroll |
| Shrink/reorder | Area/path/cache shrink clamps first-visible and clears stale hits | Resize/watcher changes cannot leave ghost targets |
| Close/reopen | History/cache/widths reset; new cwd produces a fresh generation | Client-local state must not leak across Files lifecycle |

### Task FM1.1: Add bounded chain/cache RED tests

**Files:** create `src/fm/miller.rs`; modify `src/fm/mod.rs` test wiring only.

- [ ] Add compile-valid tests against a test-only builder seam:
  `miller_history_keeps_nearest_thirty_two_segments`,
  `miller_history_never_drops_focused_current_segment`,
  `resident_cache_plus_current_keeps_at_most_five_directories`,
  `resident_eviction_invalidates_old_generation`,
  `current_projection_is_separate_from_evictable_cache`, and
  `close_reopen_rebuilds_fresh_miller_state`.
- [ ] Observe one RED at a time. Expected failure is the named bound/lifecycle
  assertion, not compilation, temp-directory setup, or unrelated FM behavior.
- [ ] Commit RED tests as
  `test: define bounded Miller history and cache contracts`.

### Task FM1.2: Implement logical chain and resident projections

**Files:** `src/fm/miller.rs`, `src/fm/mod.rs`, create
`src/app/file_manager_miller.rs`, modify `src/app/mod.rs`.

- [ ] Seed the nearest ancestor chain from opening cwd without canonicalizing
  through inaccessible/missing path components. Preserve user-visible path
  identity and classify inaccessible projections explicitly.
- [ ] Reuse `read_directory_snapshot`, natural sort, hidden filtering,
  capability, and status semantics. Keep the existing current
  `FmState.entries` vector as the operational authority and store at most four
  non-current vectors. When the current directory changes, move the old vector
  into or out of the cache with ownership transfer rather than cloning a
  duplicate complete list.
- [ ] Use deterministic LRU by last visible/focused transition; ties resolve by
  chain order. Never evict current.
- [ ] If a requested ancestor is missing/inaccessible, keep a typed unavailable
  projection and allow horizontal traversal without action authority.
- [ ] Run new tests plus existing FM directory/status/hidden/root tests.
- [ ] Commit GREEN as `feat: add bounded Miller history projections`.

### Task FM1.3: Project and render horizontal viewport

**Files:** create `src/ui/file_manager/miller.rs`; modify
`src/ui/file_manager.rs`, `src/ui.rs`, `src/app/state.rs`.

- [ ] Add RED geometry tests at Stage widths 0, 15, 16, 31, 32, 56, 84,
  140, and 400. Assert at most five columns, every visible column complete,
  divider rects disjoint, focused column visible, and all rects within Stage.
- [ ] Add RED tests:
  `horizontal_viewport_clamps_after_path_shrink`,
  `horizontal_viewport_clamps_after_terminal_resize`,
  `horizontal_scroll_changes_only_miller_window`,
  `vertical_wheel_does_not_pan_horizontal_window`,
  `zero_area_clears_column_and_divider_hits`, and
  `render_uses_precomputed_projections_without_filesystem_reads`.
- [ ] Support `MouseEventKind::ScrollLeft/ScrollRight` where Crossterm exposes
  them, Shift+wheel, and bounded header arrows. Plain wheel remains column-local
  vertical navigation.
- [ ] Render unavailable/truncated-path context explicitly; never silently
  substitute another directory.
- [ ] Run all FM UI geometry/render tests and Shell scroll/input tests.
- [ ] Commit RED as `test: define horizontal Miller viewport`; GREEN as
  `feat: add horizontal Miller viewport`.

### FM1 verification

- [ ] Run:

  ```bash
  cargo nextest run --locked -E 'test(/(miller|file_manager|shell.*scroll)/)' --status-level fail --final-status-level fail --failure-output final --success-output never
  ```

- [ ] Measure resident projection count while traversing a synthetic 100-level
  directory path; expected logical chain <=32 and resident count <=5.
- [ ] Run Linux/Windows Clippy and full direct `just check` equivalent before
  phase publication.

## FM2 — Miller Column Resize

### Task FM2.1: Reuse Shell resize transaction with column targets

**Files:** `src/fm/miller.rs`, `src/ui/file_manager/miller.rs`,
`src/app/input/file_manager.rs`, `src/app/input/shell.rs`,
`src/app/state.rs`.

- [ ] Add RED tests:
  `miller_divider_down_captures_exact_adjacent_columns`,
  `miller_resize_preview_clamps_sixteen_to_sixty_four`,
  `miller_resize_preview_does_not_dirty_persist_reload_or_resize_pty`,
  `miller_resize_commit_updates_one_preference_revision`,
  `focused_miller_divider_keyboard_resize_matches_mouse_transaction`,
  `miller_resize_cancel_restores_both_widths`,
  `terminal_resize_cancels_stale_miller_transaction`,
  `evicted_or_reordered_divider_generation_fails_closed`, and
  `mouse_up_without_miller_capture_is_inert`.
- [ ] Drive the existing SF3 `ResizeTransaction` through a typed Miller
  constraint adapter. Do not create a second drag state machine.
- [ ] During preview, recompute only Files view geometry and clip existing text
  or image output. Do not read directories, persist widths, change history, or
  submit an image decode/placement request.
- [ ] On commit, update at most two adjacent segment preferences, increment one
  Files revision, and schedule at most one active preview target refresh.
- [ ] Widths are client-local and vanish on Files close; snapshot v4 remains
  unchanged.
- [ ] Commit RED as `test: define transactional Miller column resize`; GREEN as
  `feat: add transactional Miller column resize`.

### Task FM2.2: Responsive and preview-worker cross-layer gates

**Files:** FM UI/worker tests; product only if an exact adapter is missing.

- [ ] Add tests for resized columns across 1-5 visible-column layouts, Stage
  collapse/expand, AppDock/LeftPanel resize, and mobile/tiny terminal.
- [ ] Add image/text tests proving preview content clips during drag and one
  final target is requested only after commit. Stale worker completion for the
  pre-commit geometry must remain rejected by generation.
- [ ] Simulate 1,000 mouse-move previews. Assert constant resident/history
  bounds, zero persistence, zero PTY resize, zero filesystem load, and no
  unbounded event allocation.
- [ ] Run FM/Shell/image/text families, full gates, then publish atomically.

## FM3 — Mouse Ownership in Every Miller Column

### Interaction contract

- A plain click in any rendered directory column selects the exact live entry
  in that column and focuses the column.
- A directory selection prepares/replaces the next projection; a file selection
  prepares file preview. FM4 later adds retained growing-chain semantics.
- Ctrl/Shift multi-selection remains scoped to the current operational
  directory. Modified selection in an ancestor/preview column is consumed with
  an explicit inert result; it never grants cross-directory operation authority.
- Right-click on a live row selects according to existing single/current policy
  and opens the same typed file context menu anchored to that row.
- Plain wheel over a column moves/clamps only that column cursor/viewport;
  horizontal gestures move only the horizontal viewport.
- Double-click retains existing semantics for the current operational column;
  ancestor/preview activation first revalidates and establishes that directory
  as current before enter.

### Task FM3.1: Generate complete stable column hit targets

**Files:** `src/ui/file_manager/miller.rs`, `src/app/state.rs`.

- [ ] Add RED tests:
  `every_visible_directory_row_has_one_stable_target`,
  `file_preview_has_no_row_targets`,
  `column_targets_include_directory_entry_and_generations`,
  `column_and_divider_hits_do_not_overlap`,
  `hidden_or_clipped_rows_have_no_targets`, and
  `surface_switch_or_zero_area_clears_all_miller_hits`.
- [ ] Project hits once in row order from resident state. Never build path
  identity in render.
- [ ] Commit RED as `test: define all-column Miller hit identity`; GREEN as
  `feat: project stable targets for every Miller column`.

### Task FM3.2: Route/revalidate all-column mouse actions

**Files:** `src/app/input/file_manager.rs`,
`src/app/file_manager_miller.rs`, `src/fm/miller.rs`.

- [ ] Add table-driven RED tests for parent/current/preview/older-ancestor
  columns and plain/Ctrl/Shift/right/double/wheel gestures.
- [ ] Add adversarial tests:
  `same_index_different_path_is_consumed_stale`,
  `same_path_different_column_generation_is_consumed_stale`,
  `ancestor_deleted_between_view_and_click_is_inert`,
  `ancestor_reordered_between_view_and_click_uses_exact_path`,
  `evicted_column_click_is_inert`,
  `overlay_blocks_all_column_actions`, and
  `hidden_terminal_receives_no_files_mouse_bytes`.
- [ ] Revalidate non-current directories in the App state refresh path before
  transition. A stale result may refresh the projection but may not replay the
  original action automatically; the user must act on the fresh view.
- [ ] Preserve current multi-selection operation authority and context-menu
  capability checks. No cross-directory bulk selection is introduced.
- [ ] Commit RED as `test: define Miller all-column mouse ownership`; GREEN as
  `feat: route mouse ownership across Miller columns`.

### FM3 verification

- [ ] Run every FM input/context/operation/selection test, Shell router/overlay
  tests, and an isolated SGR-mouse session. Use semantic targeted input only;
  no coordinate automation outside the throwaway test terminal.
- [ ] Prove left/middle/right visible directory columns respond, stale rows are
  inert, background terminal is inert, context overlays block Stage, and
  closing Files restores Terminal.
- [ ] Require zero test-owned process/socket/root residue and full gates.

## FM4 — Finder-Like Path-Stable Growing Navigation

### Frozen behavior

1. Each directory column represents one exact directory path and retains its
   exact focused child path, cursor, and vertical viewport.
2. Selecting a directory in column N creates or refreshes column N+1 and
   retains columns 0..N. Selecting a file creates a typed preview at N+1 and
   removes deeper directory descendants.
3. Selecting a different entry in an ancestor truncates all descendants after
   that ancestor before appending the new branch.
4. Returning to a retained segment restores focus by exact child path; if the
   child vanished or is hidden, use the nearest valid row and clamp viewport.
5. Current `FmState::leave`/`reload_focusing_path` N2.1 behavior stays green and
   becomes a transition of the same chain model.
6. Chain overflow evicts the farthest ancestor, never current/focused context;
   resident projections remain at most five.
7. Current-cwd watcher events preserve exact live path where possible and
   prune impossible descendants. Non-current columns revalidate on action.
8. Closing/reopening Files discards the chain and seeds a fresh one from the
   opening cwd.

### Task FM4.1: RED path-chain state machine

**Files:** `src/fm/miller.rs`, `src/fm/mod.rs` tests.

- [ ] Add RED tests:
  `directory_selection_appends_one_child_segment`,
  `repeated_deep_selection_grows_path_stably`,
  `ancestor_branch_change_truncates_descendants`,
  `file_selection_replaces_deeper_chain_with_preview`,
  `return_to_segment_restores_exact_child_path`,
  `missing_or_hidden_child_uses_nearest_safe_fallback`,
  `watcher_delete_prunes_impossible_descendants`,
  `watcher_reorder_preserves_exact_path_focus`,
  `history_overflow_evicts_farthest_ancestor_only`,
  `root_and_inaccessible_boundaries_are_total`, and
  `close_reopen_resets_branch_and_generations`.
- [ ] Re-run all published N2.1 leave tests with the RED commit; they must stay
  green while the new test alone fails.
- [ ] Commit RED as `test: define path-stable Miller navigation`.

### Task FM4.2: Implement minimal growing transitions

**Files:** `src/fm/miller.rs`, `src/fm/mod.rs`,
`src/app/file_manager_miller.rs`, `src/app/file_manager_watcher.rs`.

- [ ] Make `MillerState` the single chain authority while retaining
  compatibility accessors for existing `cwd`, entries, cursor, viewport,
  parent, and preview consumers during refactor.
- [ ] Centralize transitions in pure methods returning typed refresh requests:
  `select_entry`, `activate_directory`, `leave`, `reconcile_current_refresh`,
  and `reset_for_open`. App adapters perform requested I/O and apply results
  only when generation/path still match.
- [ ] Branch truncation clears stale selection, preview generations, context
  menu authority, and evicted hit IDs before new projection publication.
- [ ] Do not automatically execute a stale click after refresh. Do not retain
  cross-directory operation selection.
- [ ] Run exact new tests, every N2.1 test, watcher/preview/operation families,
  and invariants after adversarial sequences.
- [ ] Commit GREEN as `feat: add path-stable growing Miller navigation`.

### Task FM4.3: Property/adversarial and performance closure

**Files:** tests/instrumentation only unless a proven defect exists.

- [ ] Add deterministic generated action sequences of at least 10,000 steps
  over select, append, branch, leave, hidden toggle, watcher create/delete/
  rename/reorder, resize, horizontal scroll, cache eviction, close/reopen, and
  overlay open/close.
- [ ] After every step assert chain <=32, resident <=5, unique IDs, focused
  membership, current residency, valid cursors/viewports, no stale hit to an
  absent generation, and existing App/Workspace invariants.
- [ ] Measure transition p95 and full Files frame p95 at 120x40 and 240x80.
  Require program budgets and no growth with action count after bounds fill.
- [ ] Run full gates and isolated deep-navigation manual proof before publish.

## FM5 — Preview / Inspector Placement Decision

FM5 is evidence-first. It does not automatically activate `RightPanel` product
work.

### Options to compare

| Option | Definition | Primary risk |
|---|---|---|
| A: inline final Miller column | Keep file/directory preview as the rightmost scrollable Miller column | Preview competes with deep navigation width |
| B: Shell `RightPanel` inspector | Move metadata/text/image preview into the optional right outer region | Adds cross-surface responsive/focus/persistence coupling |
| C: adaptive hybrid | Inline at wide Stage; RightPanel at a measured breakpoint | Two placements increase state, test, and transition complexity |

### Task FM5.1: Build evidence fixtures without product mutation

**Files:** tests/bench/evidence docs already permitted by continuity; no
production file until decision.

- [ ] Record representative terminal sizes 80x24, 100x30, 120x40, 160x50,
  240x80; path depths 1, 3, 5, 12, 32; names with ASCII, CJK, emoji, and long
  graphemes; preview states none/directory/text/image/error/loading/truncated;
  AppDock and LeftPanel expanded/collapsed.
- [ ] Measure visible navigation columns, focused-path readability, preview
  width/height, image target churn, full-frame p95, outgoing bytes, focus
  transitions, and collapse frequency for A/B/C using deterministic projections
  or local prototypes outside product code.
- [ ] Run accessibility/interaction review: keyboard focus order, mouse travel,
  screen-reader/accessibility labels available to the TUI, color-independent
  status, overlay blocking, tiny terminal, and mobile behavior.
- [ ] Record failure behavior for missing/inaccessible file, stale preview
  generation, image worker failure, RightPanel collapsed, and Stage switch.

### Task FM5.2: Apply explicit decision gate

Choose GO only if one option satisfies all of:

- path/focused entry remains readable at the approved minimum terminal;
- text/image preview has a useful measured area at its activation breakpoint;
- no extra filesystem read occurs in render;
- no new wire/runtime authority is required;
- frame and outgoing-byte budgets remain within the program thresholds;
- focus, scroll, overlay, collapse, and close/reopen ownership is unambiguous;
- implementation fits one bounded independently revertible slice.

- [ ] Write a decision record with raw measurements, option matrix, selected
  option or NO-GO, breakpoint if any, exact requirements/tests, file boundary,
  rollback, and risks.
- [ ] If NO-GO, keep current inline preview and close FM5 honestly. Do not add
  speculative RightPanel product code.
- [ ] If GO requires production behavior, stop and create a separately
  user-approved micro implementation plan with RED/GREEN slices; FM5 evidence
  alone does not authorize it.
- [ ] Commit the decision/evidence separately as
  `docs: decide native Files preview placement`.

## Per-Phase Verification and Git Closure

For FM1-FM4:

- [ ] Run the exact focused RED and read its assertion.
- [ ] Commit only the RED test/test seam.
- [ ] Implement the complete named behavior and failure branch; run focused
  GREEN.
- [ ] Commit product separately; refactor only after GREEN in another commit.
- [ ] Run FM + Shell cross-layer family, failure/adversarial tests, performance
  measurements, Linux/Windows Clippy, full nextest, Bun/Python maintenance,
  formatting, and diff cleanliness.
- [ ] Perform isolated runtime proof when mouse/render/image behavior changes.
- [ ] Update continuity/evidence separately.
- [ ] Targeted-stage declared files only. Never stage `.superpowers/`.
- [ ] Fetch, prove fast-forward, push only CyPack refs, verify remote SHA, then
  refresh Codebase Memory and prove the exact phase symbols.

Canonical broad FM filter:

```bash
cargo nextest run --locked -E 'test(/(miller|file_manager|file_operation|file_preview|image_preview|watcher|shell_input|surface_host|app_dock)/)' --status-level fail --final-status-level fail --failure-output final --success-output never
```

Expected: a nonzero selected inventory, zero failed, zero retry-only green, and
only explicitly named ignored host probes. The full direct `just check`
equivalent from the program plan is mandatory before every phase publication.

FM-next is complete only when FM1-FM4 have fresh focused/full/failure/
performance/manual/Git/graph evidence, every bound and stale-identity invariant
holds, and FM5 has an evidence-backed GO or NO-GO. Apps/Desktop remains
unstarted and no speculative preview placement is smuggled into the closure.
