# Herdr Miller Trail Fractional Scroll Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace whole-column Miller wheel jumps with deterministic cell-based movement of approximately one third of the relevant column while rendering and hit-testing partially visible columns correctly.

**Architecture:** Keep horizontal viewport state client-local in `MillerHorizontalViewport`, but replace mutable column-index authority with one absolute `offset_cells`. A pure Miller geometry core projects logical column/divider intervals through that offset into clipped Ratatui rectangles; `TrailViewSnapshot` becomes the shared immutable authority for render and horizontal input. Navigation/resize auto-follow is computed in `sync_miller_view`, while manual wheel input disables follow and only changes the bounded offset.

**Tech Stack:** Rust 1.96.1, Ratatui, Crossterm mouse events, cargo-nextest, Playwright 1.54.1 with Chromium, Bun, Python unittest, codebase-memory-mcp.

## Global Constraints

- Canonical UX: `docs/superpowers/specs/2026-07-18-herdr-miller-trail-ux-contract.md`.
- Approved design: `docs/superpowers/specs/2026-07-18-herdr-miller-fractional-scroll-design.md`.
- One event advances `max(1, ceil(reference_column_width / 3))` cells; no timer, easing, momentum, or animation state.
- Partially visible columns are clipped, and invisible cells create no mouse/action target.
- Render remains pure and performs no filesystem I/O or state mutation.
- State is TUI/client-local; no server, protocol, socket, persistence, or dependency change.
- Production Rust contains no new `unwrap()`.
- Tests are RED before GREEN; nextest RED evidence uses `--no-fail-fast`.
- Visual acceptance is Playwright Chromium; snapshot updates are VIS-12 spec-scoped only.
- Every Cargo command uses `PATH="$HOME/.local/bin:$PATH"`.
- Manual testing starts and ends with `.local/herdr-trail-test.sh` semantic cleanup and never touches stable Herdr/socket or user processes.
- `.superpowers/` is user-owned and never staged, edited, or deleted.
- Publication is fast-forward to CyPack `feat/native-fm` and `master` only; upstream is read-only.

---

## Dependency Chain

```text
MillerHorizontalViewport.offset_cells
  -> pure logical interval geometry + clamp/auto-follow helpers
  -> sync_miller_view commits bounded auto/manual offset
  -> project_trail_view_at_offset publishes clipped TrailViewSnapshot
  -> render_trail_view paints clipped label/action cells
  -> handle_miller_horizontal_scroll consumes the same TrailViewSnapshot
  -> VIS-12 + full regression gates
```

The state/geometry task must land before render/input. Render and input may be reviewed
independently once the shared snapshot interface exists. VIS-12 depends on both.

## File Ownership

- Modify `src/fm/miller.rs`: client-local absolute offset and lifecycle/invariant rules.
- Modify `src/ui/file_manager/miller.rs`: pure logical interval, clamp, auto-follow, and clipped viewport geometry.
- Modify `src/ui/file_manager/trail_view.rs`: cell-offset projection, clipped row/action geometry, Trail snapshot scroll authority, render clipping.
- Modify `src/ui/file_manager.rs`: reusable display-cell slice renderer for clipped entry labels.
- Modify `src/ui.rs`: compute-time auto-follow/clamp and Trail snapshot projection.
- Modify `src/app/file_manager_miller.rs`: horizontal input consumes Trail snapshot and writes `offset_cells`.
- Modify `src/app/input/file_manager.rs`: compile-valid behavior REDs, stale authority, bounds, resize/navigation, and invariant regressions.
- Modify `src/ui/visual_fixture.rs`: deterministic VIS-12 partial-column fixture.
- Create `tests/visual/fractional-scroll.spec.ts` and its Chromium snapshot.
- Modify `.codex/TASKS.md`, `.codex/CURRENT.md`, `.codex/HANDOFF.md`: durable task and closure evidence.

### Task 1: Compile-valid RED behavior contract

**Files:**
- Modify: `src/app/input/file_manager.rs`
- Modify: `src/ui/file_manager/trail_view.rs`

**Interfaces:**
- Consumes: current `handle_miller_horizontal_scroll`, `compute_view`, `TrailViewSnapshot`.
- Produces: failing tests named below without referencing nonexistent APIs.

- [x] **Step 1: Add the one-third movement RED**

Create `horizontal_wheel_moves_by_fractional_cells_and_keeps_partial_column_visible`.
Build an eight-level Trail with 30-cell per-index widths, compute a narrow frame, scroll left
once from auto-follow, recompute, and assert:

```rust
assert!(after_left_offset > before_left_offset);
assert_eq!(after_left_offset - before_left_offset, 10);
assert!(leading.rect.width > 0 && leading.rect.width < 30);
assert_eq!(after.trail.selected_path(), before.trail.selected_path());
```

The offsets are recovered from the visible logical column identity and its clipped width so
the test compiles against the current snapshot. Current behavior must fail because it advances
one complete column and cannot produce a partial leading rect.

- [x] **Step 2: Add clipping and mixed-width REDs**

Add:

```text
fractional_scroll_uses_each_leading_columns_own_width
partial_trail_rows_and_actions_never_escape_the_stage
partial_leading_column_renders_the_visible_label_suffix
navigation_rearms_fractional_scroll_auto_follow
manual_fractional_offset_reclamps_without_rearming_on_resize
```

Use 18/30/48-cell preferences. Assert 6/10/16-cell steps at their corresponding leading
columns, exact stage intersection for rows/actions/dividers, a leading label slice that omits
the clipped prefix, navigation returning the active column to view, and manual resize preserving
`follow_active == false`.

- [x] **Step 3: Run RED**

Run:

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo nextest run --locked --no-fail-fast fractional_scroll
```

Expected: behavior assertion failures showing whole-column movement/no partial column; no
compile error, panic, empty selection, or unrelated failure is accepted as RED.

- [x] **Step 4: Run protected characterization**

Run:

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo nextest run --locked --no-fail-fast horizontal_wheel
cargo nextest run --locked --no-fail-fast ten_thousand_miller_actions_preserve_all_invariants
```

Expected: existing tests pass before production edits.

- [x] **Step 5: Commit RED**

Run `cargo fmt`, then run `cargo fmt --check` separately. Target-stage only the two Rust test
files and commit:

```bash
git commit -m "test: require fractional miller trail scrolling"
```

### Task 2: Absolute offset state and pure clipped geometry

**Files:**
- Modify: `src/fm/miller.rs`
- Modify: `src/ui/file_manager/miller.rs`
- Test: inline `#[cfg(test)] mod tests` in both files

**Interfaces:**
- Produces:

```rust
pub(crate) struct MillerHorizontalViewport {
    pub offset_cells: u32,
    pub follow_active: bool,
}

pub(crate) struct MillerColumnRect {
    pub chain_index: usize,
    pub rect: Rect,
    pub logical_width: u16,
    pub source_x: u16,
}

pub(crate) struct MillerViewportGeometry {
    pub columns: Vec<MillerColumnRect>,
    pub dividers: Vec<MillerDividerRect>,
    pub offset_cells: u32,
    pub max_offset_cells: u32,
    pub first_visible: usize,
}

pub(crate) fn miller_auto_follow_offset(
    stage_width: u16,
    preferred_widths: &[u16],
    focused_index: usize,
) -> u32;

pub(crate) fn miller_viewport_geometry_at_offset(
    stage: Rect,
    preferred_widths: &[u16],
    requested_offset_cells: u32,
) -> MillerViewportGeometry;
```

- [x] **Step 1: Replace the mutable index**

Initialize `offset_cells = 0`. On Trail visit/sync keep the current bounded offset but set
`follow_active = true`; do not derive or store a mutable first-visible index. Update the
test-only invariant to require:

```rust
assert!(self.horizontal.offset_cells <= miller_total_content_width(&widths));
```

- [x] **Step 2: Implement logical interval geometry**

Build every clamped column/divider interval in `u32`, clamp requested offset to
`total_width.saturating_sub(stage.width)`, intersect each interval with the viewport, and emit
only nonempty visible rectangles. Convert to `u16` only after subtracting the viewport origin.
Derive `first_visible` from the first emitted column.

- [x] **Step 3: Implement auto-follow**

For a focused interval `[start, end)`, return:

```rust
if width > stage_width { start } else { end.saturating_sub(stage_width) }
```

then clamp to `max_offset`. This keeps the active column complete when possible and allows a
partial ancestor at the left edge.

- [x] **Step 4: Preserve resize compatibility**

Keep `miller_viewport_geometry(stage, widths, focused, requested_first_visible)` as a pure
compatibility wrapper that converts the requested column to its logical start offset and calls
the new core. It is not mutable viewport authority.

- [x] **Step 5: Run focused GREEN**

Run:

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo nextest run --locked --no-fail-fast miller_viewport
cargo nextest run --locked --no-fail-fast auto_follow_origin
```

Expected: all selected tests pass.

### Task 3: Trail projection, clipped render, and single snapshot authority

**Files:**
- Modify: `src/ui/file_manager/trail_view.rs`
- Modify: `src/ui/file_manager.rs`
- Modify: `src/ui.rs`

**Interfaces:**
- Produces:

```rust
pub(crate) struct TrailViewSnapshot {
    pub files_generation: Option<u32>,
    pub model_revision: u64,
    pub offset_cells: u32,
    pub max_offset_cells: u32,
    pub scroll_step_left: u32,
    pub scroll_step_right: u32,
    // existing columns/dividers/detail_panel
}

pub(crate) fn project_trail_view_at_offset(
    stage: Rect,
    trail: &TrailState,
    snaps: &TrailSnapshots,
    preferred_widths: &[u16],
    detail_preferred_width: u16,
    requested_offset_cells: u32,
) -> TrailViewSnapshot;
```

- [x] **Step 1: Project clipped columns**

Use `miller_viewport_geometry_at_offset`. Carry `logical_width` and `source_x` into
`TrailColumnView`. Build logical name/action intervals first, intersect them with the visible
column interval, and publish only nonempty stage-contained rects. A partially clipped action
must be omitted rather than become a smaller clickable command.

- [x] **Step 2: Add Unicode display-cell slicing**

Extract the complete logical entry label at `logical_width`, then slice it by
`source_x..source_x+visible_width` using `UnicodeWidthChar`. Never split a double-width glyph;
replace a clipped continuation cell with one blank cell. Keep the existing non-clipped
`render_entry_row` wrapper for other consumers.

- [x] **Step 3: Publish scroll steps**

Derive the right reference column at `offset_cells` and the left reference column at
`offset_cells.saturating_sub(1)`. Divider positions choose the nearest column in the movement
direction. Store `max(1, width.div_ceil(3))` for both directions.

- [x] **Step 4: Make compute the state transition authority**

Change `sync_trail_view` to accept the offset. In mutable `sync_miller_view`, compute and commit
auto-follow or manual clamp before both resize and Trail snapshots are projected. Set
`TrailViewSnapshot.files_generation` and `model_revision` from the same Files instance.

- [x] **Step 5: Run focused projection/render tests**

Run:

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo nextest run --locked --no-fail-fast trail_view
cargo nextest run --locked --no-fail-fast entry_row
```

Expected: fractional, clipping, Unicode, purity, and existing Trail render tests pass.

### Task 4: Horizontal input cutover and stale protection

**Files:**
- Modify: `src/app/file_manager_miller.rs`
- Modify: `src/ui/file_manager/miller.rs`
- Modify: `src/app/input/file_manager.rs`

**Interfaces:**
- Consumes: generation/revision-bound `TrailViewSnapshot`.
- Produces:

```rust
impl TrailViewSnapshot {
    pub(crate) fn horizontal_scroll_target(
        &self,
        file_manager: &FmState,
        delta: i8,
    ) -> Option<u32>;
}
```

- [x] **Step 1: Move input authority to Trail snapshot**

Reject absent generation, revision mismatch, empty columns, and zero-width geometry. Read the
current `offset_cells`, apply `scroll_step_left/right`, and clamp to
`0..=max_offset_cells`.

- [x] **Step 2: Update the App handler**

Read `self.state.view.file_manager_trail`, validate the active Files generation, write only
`file_manager.miller.horizontal.offset_cells`, and set `follow_active = false`. Remove the
scroll-target method from `MillerViewSnapshot`.

- [x] **Step 3: Update existing regressions**

Convert existing `first_visible` assertions to absolute offset/derived first-visible
assertions. Mutate the Trail snapshot generation/revision in stale tests. Preserve cursor,
entries, multi-selection, preview, chain, revision, overlay precedence, and outside-stage
assertions.

- [x] **Step 4: Run input GREEN**

Run:

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo nextest run --locked --no-fail-fast fractional_scroll
cargo nextest run --locked --no-fail-fast horizontal_wheel
cargo nextest run --locked --no-fail-fast ten_thousand_miller_actions_preserve_all_invariants
```

Expected: all selected tests pass with no warning or skipped selected test.

- [x] **Step 5: Commit GREEN**

Run `cargo fmt`, then `cargo fmt --check` separately. Review the production diff and added
`unwrap()` scan, target-stage only owned Rust files, and commit:

```bash
git commit -m "fix: add fractional miller trail scrolling"
```

### Task 5: Playwright Chromium VIS-12

**Files:**
- Modify: `src/ui/visual_fixture.rs`
- Create: `tests/visual/fractional-scroll.spec.ts`
- Create: `tests/visual/fractional-scroll.spec.ts-snapshots/vis-12-fractional-miller-scroll-chromium-linux.png`

**Interfaces:**
- Consumes: real Ratatui `TrailViewSnapshot` and `render_trail_view`.
- Produces: deterministic cell fixture `vis-12-fractional-miller-scroll.json`.

- [x] **Step 1: Add deterministic fixture**

Create a narrow ASCII-profile Trail with at least four columns, different widths, and an
`offset_cells` value inside the second column. Export the actual Ratatui buffer; do not draw an
HTML mock.

- [x] **Step 2: Add Chromium assertion**

Load the fixture through the existing cell-grid harness. Assert the fixture metadata identifies
VIS-12 and screenshot the exact terminal grid. The screenshot must visibly contain a clipped
leading column and the beginning of the trailing column.

- [x] **Step 3: Prove mutation sensitivity**

Temporarily alter one exported cell in the generated artifact, run only
`fractional-scroll.spec.ts`, require screenshot failure, restore/regenerate the exact fixture,
and rerun green. Do not commit the mutation.

- [x] **Step 4: Approve only the new baseline**

Run:

```bash
cd tests/visual
npx playwright test fractional-scroll.spec.ts --update-snapshots
npx playwright test fractional-scroll.spec.ts
npx playwright test
```

Expected: the focused VIS-12 spec passes, then the full Chromium suite reports 20/20 if no
other spec count changed.

- [x] **Step 5: Commit visual evidence**

Target-stage only the exporter, VIS-12 spec, and VIS-12 PNG. Commit:

```bash
git commit -m "test: approve fractional miller scroll visual"
```

### Task 6: Production closure, continuity, graph, and publication

**Files:**
- Modify: `.codex/TASKS.md`
- Modify: `.codex/CURRENT.md`
- Modify: `.codex/HANDOFF.md`
- Create: `.codex/evidence/miller-fractional-scroll-closure.md`

**Interfaces:**
- Consumes: completed RED/GREEN/VIS commits and fresh gate output.
- Produces: exact closure SHAs, counts, graph evidence, and clean publication state.

- [x] **Step 1: Run complete gates**

Run in this order with no concurrent Cargo process:

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo fmt --check
cargo nextest run --locked --no-fail-fast
cargo clippy --all-targets --locked -- -D warnings
LIBGHOSTTY_VT_SIMD=false cargo clippy --bin herdr --target x86_64-pc-windows-msvc --locked -- -D warnings
python3 -m unittest scripts.test_agent_detection_manifest_check scripts.test_changelog scripts.test_docs_translation_parity scripts.test_preview scripts.test_trail_t7_teardown scripts.test_vendor_libghostty_vt scripts.test_vendor_portable_pty
bun test src/integration/assets/herdr-agent-state.test.ts
(cd workers/plugin-marketplace && bun test)
(cd tests/visual && npx playwright test)
git diff --check
```

Expected: zero failures/warnings; record exact pass/skip counts.

- [x] **Step 2: Run source and safety audits**

Require no added production `unwrap()`, no production read of mutable
`horizontal.first_visible`, no stage-external Trail hit rect, and only `.superpowers/` as
untracked user-owned state. Run `.local/herdr-trail-test.sh --help` or its documented
noninteractive verification path only if it does not address stable Herdr.

- [x] **Step 3: Record closure**

Mark T7.8 subtasks complete, add exact commits and gate counts to continuity/evidence, run the
OPEN_TASKS exact-copy synchronizer with offset-aware marker search, and require exact diff.
Run `cargo fmt --check` separately before the docs commit:

```bash
git commit -m "docs: record fractional miller scroll closure"
```

- [x] **Step 4: Refresh Codebase Memory**

Run only:

```bash
CBM_WORKERS=1 codebase-memory-mcp cli index_repository '{"repo_path":"/home/ayaz/projects/herdr","project":"home-ayaz-projects-herdr","mode":"fast","persistence":false}'
```

Require `ready`, record node/edge counts, and retrieve fresh snippets for
`MillerHorizontalViewport`, `miller_viewport_geometry_at_offset`, and
`handle_miller_horizontal_scroll`.

- [x] **Step 5: Publish CyPack fast-forward**

Fetch origin, require both remote refs are ancestors of local HEAD, then:

```bash
git push origin HEAD:feat/native-fm HEAD:master
git ls-remote origin refs/heads/feat/native-fm refs/heads/master
```

Expected: both remote SHAs exactly equal local HEAD. Never push upstream.

## Rollback

- Planning commit: documentation/task-only rollback.
- RED commit: safe failing-test checkpoint.
- GREEN commit: functional checkpoint; before publication it may be reverted atomically.
- VIS commit: fixture/spec/baseline only.
- After publication use forward-fix; do not reset published history.

## Completion Definition

T7.8 is complete only when one event advances by the appropriate one-third cell step, partial
columns render clipped and hit-test safely, navigation/resize follow semantics remain exact,
stale snapshots fail closed, VIS-12 and the full Chromium suite pass, all repository/platform
gates pass, continuity and graph are current, and both CyPack refs equal local HEAD.
