# Herdr Files Content Locations Rail Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:subagent-driven-development` (recommended) or
> `superpowers:executing-plans` to execute this plan task by task.

**Goal:** Keep Herdr's global agent/workspace panel visible while Native Files
owns the center, move Favorites/Locations into a responsive Files-local rail,
make the selected root explicit, and remove directory enumeration from
render/input/scheduled apply paths.

**Architecture:** `StageState` remains the singleton Files owner while
`SidebarTab` continues to own only the global Spaces/Projects presentation.
The Native Files body is projected once into disjoint locations-rail,
separator, and Trail rectangles. Exact row identities emit generation-bound
location requests. Root switching, Miller navigation, and current refresh are
prepared by one bounded latest-pending read worker and applied atomically
through the existing pure generation checks.

**Tech Stack:** Rust 2021, Ratatui, Crossterm, Tokio `Notify`, standard-library
thread/`Mutex`/`Condvar`, existing `notify_debouncer_full`, cargo-nextest,
Playwright 1.54 Chromium, deterministic Ratatui ASCII cell fixtures.

## Global Constraints

- Never access or stop the installed stable Herdr server, inherited stable
  socket, Ghostty, terminal, browser, editor, or any user session.
- Runtime checks use `.local/ISOLATED-DEV-TEST.md` with every inherited
  `HERDR_*` override unset.
- Never edit, stage, delete, or otherwise touch user-owned `.superpowers/`.
- Never push to `ogulcancelik/herdr`; publish only fast-forward commits to the
  CyPack fork's `feat/native-fm` and `master`.
- Use targeted staging only. Ignored plan/spec/cartography files require
  exact-path `git add -f`; `git add -A` is forbidden.
- Prefix every Cargo invocation with
  `export PATH="$HOME/.local/bin:$PATH"`.
- Run `cargo fmt --check` as a separate command and verify its exit status
  before every commit.
- Full Nextest always uses `--no-fail-fast`.
- Production Rust must not add `unwrap()`. New items precede a file's
  `#[cfg(test)] mod tests`.
- Render and `compute_view` remain filesystem-free; input emits typed intents;
  runtime adapters own filesystem work.
- New Chromium baselines are updated only with the exact new spec. Existing
  baselines are never globally rewritten.
- Each behavior change follows RED commit, GREEN commit, then refactor only
  while the same focused and adversarial tests remain green.

## File Structure and Ownership

### New files

- `src/app/file_manager_io_worker.rs`
  - owns the one-executing/one-latest-pending read-only worker;
  - accepts root, navigation, and current-refresh jobs;
  - catches worker preparation panics and produces typed failures;
  - carries no `AppState`, workspace, terminal, or protocol authority.
- `src/app/file_manager_locations.rs`
  - owns pure client-local location origin, pending/error/focus/drawer state;
  - validates exact prepared-model identity without reading the filesystem.
- `src/ui/file_manager/locations.rs`
  - owns responsive body layout and pure locations row projection/render;
  - publishes disjoint current-frame hit geometry.
- `tests/visual/files-locations.spec.ts`
  - owns the new wide, standard, compact/drawer, origin, loading, and failure
    Chromium snapshots.

### Modified files

- `src/fm/mod.rs`
  - adds typed root preparation request/result and a pure root apply seam;
  - retains existing `FmNavigationRequest` and `FmCurrentRefreshRequest`.
- `src/app/mod.rs`
  - registers the new modules and initializes the worker handle.
- `src/app/state.rs`
  - stores pure locations state and current-frame locations geometry;
  - removes the old global-sidebar request/row ownership after the swap.
- `src/app/file_manager_miller.rs`
  - submits Miller navigation to the shared worker instead of preparing it
    synchronously.
- `src/app/file_manager_watcher.rs`
  - submits root/current-refresh jobs and applies drained results;
  - watcher rebinding remains downstream of a successful atomic apply.
- `src/app/runtime.rs`
  - drains worker results before synchronizing watcher/render state;
  - contains no directory preparation.
- `src/server/headless.rs`
  - mirrors the same non-blocking drain/apply ordering.
- `src/ui.rs`
  - projects the Files-local rail and Trail from one body split;
  - stops projecting Files rows inside global `LeftPanel`.
- `src/ui/file_manager.rs`
  - passes the Files body to the new locations layout and renders the shared
    header/status around rail + Trail.
- `src/ui/sidebar.rs`
  - removes the Files-owned global body and cwd-derived location highlight;
  - preserves the Files launcher control.
- `src/app/input/mouse.rs`
  - makes Files activation stage-only and preserves Spaces/Projects;
  - removes global-sidebar location hit handling.
- `src/app/input/file_manager.rs`
  - routes rail/drawer click, wheel, keyboard, stale-hit, and overlay behavior.
- `src/ui/visual_fixture.rs`
  - exports deterministic ASCII fixtures for the new composition/states.
- `.codex/TASKS.md`, `.codex/CURRENT.md`, `.codex/HANDOFF.md`
  - record FCL dependencies, evidence, and exact unchecked-task continuity.
- `.cartography/files-content-locations-rail-SYSTEM-MAP.json`
  - freezes the verified graph surfaces and final relation counts; it is
    generated only from real codebase-memory evidence.

### Deliberately unchanged

- server wire/API/persistence types;
- file operations and destructive-action authority;
- current one-third Trail horizontal scroll algorithm;
- stable/release documentation and release assets;
- optional plugin adoption program.

## Exact Types and Interfaces

Add the root-preparation seam in `src/fm/mod.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FmRootNavigationRequest {
    pub files_generation: u32,
    pub location_model_revision: u64,
    pub target_root: PathBuf,
    pub show_hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FmPreparedRootNavigation {
    pub request: FmRootNavigationRequest,
    pub file_manager: FmState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FmRootNavigationError {
    Missing,
    PermissionDenied,
    ChangedType,
    Unavailable,
}

pub(crate) fn prepare_root_navigation_io(
    request: FmRootNavigationRequest,
) -> Result<FmPreparedRootNavigation, FmRootNavigationError>;
```

`prepare_root_navigation_io` performs the live type check, calls the existing
Trail construction seam, rechecks the final type, and maps failure into the
stable typed error. The App-side apply validates Files generation, location
model revision, exact accessible item identity, and pending target before
installing `FmState`.

Add pure presentation state in `src/app/file_manager_locations.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FileManagerLocationOrigin {
    Location(PathBuf),
    Direct(PathBuf),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileManagerLocationsFocus {
    Trail,
    Rail,
    Drawer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileManagerLocationPending {
    pub path: PathBuf,
    pub files_generation: u32,
    pub model_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileManagerLocationsState {
    pub origin: Option<FileManagerLocationOrigin>,
    pub pending: Option<FileManagerLocationPending>,
    pub error: Option<FmRootNavigationError>,
    pub scroll: usize,
    pub focus: FileManagerLocationsFocus,
    pub drawer_open: bool,
}
```

The location model receives a monotonically increasing `revision: u64` when
its prepared sections change. `item_for_path` remains exact, not prefix-based.
The state exposes pure helpers:

```rust
pub(crate) fn highlighted_path(
    &self,
    model: &FileManagerSidebarModel,
) -> Option<&Path>;
pub(crate) fn begin_location(
    &mut self,
    path: &Path,
    files_generation: u32,
    model: &FileManagerSidebarModel,
) -> Option<FmRootNavigationRequest>;
pub(crate) fn retire_for_closed_files(&mut self);
```

Add worker messages in `src/app/file_manager_io_worker.rs`:

```rust
#[derive(Debug)]
pub(super) enum FileManagerIoRequest {
    Root(FmRootNavigationRequest),
    Navigate {
        files_generation: u32,
        request: FmNavigationRequest,
    },
    Refresh(FmCurrentRefreshRequest),
}

#[derive(Debug)]
pub(super) enum FileManagerIoOutcome {
    Root(Result<FmPreparedRootNavigation, FmRootNavigationError>),
    Navigate {
        files_generation: u32,
        prepared: Option<FmPreparedNavigation>,
    },
    Refresh(FmPreparedCurrentRefresh),
    Panicked(FileManagerIoIdentity),
}
```

The worker state is a standard-library `Mutex` + `Condvar` with:

```rust
pending: Option<FileManagerIoRequest>,
result: Option<FileManagerIoOutcome>,
alive: bool,
closed: bool,
```

Submitting replaces only `pending`; it never interrupts the executing
processor and never grows a queue. The processor wraps each preparation in
`std::panic::catch_unwind(std::panic::AssertUnwindSafe(...))`. `Drop` marks
closed, clears pending/result, notifies, and joins only its own thread.

Add pure UI geometry in `src/ui/file_manager/locations.rs`:

```rust
pub(crate) enum FileManagerLocationsMode {
    Wide,
    Standard,
    Compact,
}

pub(crate) struct FileManagerContentLayout {
    pub mode: FileManagerLocationsMode,
    pub rail: Option<Rect>,
    pub separator: Option<Rect>,
    pub trail: Rect,
}

pub(crate) fn file_manager_content_layout(body: Rect) -> FileManagerContentLayout;
pub(crate) fn project_location_rows(
    app: &AppState,
    rail: Rect,
) -> Vec<FileManagerLocationRowArea>;
```

Constants:

```rust
const WIDE_RAIL_TARGET: u16 = 24;
const WIDE_RAIL_MIN: u16 = 18;
const WIDE_RAIL_MAX: u16 = 28;
const STANDARD_RAIL_MIN: u16 = 16;
const STANDARD_RAIL_MAX: u16 = 20;
const LOCATIONS_SEPARATOR_WIDTH: u16 = 1;
```

The mode threshold derives from
`STANDARD_RAIL_MIN + LOCATIONS_SEPARATOR_WIDTH +
MILLER_COLUMN_MIN_WIDTH`; it is not hard-coded from a device name.

Extend `ViewState` with:

```rust
pub(crate) file_manager_content_layout: FileManagerContentLayout,
pub(crate) file_manager_location_row_areas: Vec<FileManagerLocationRowArea>,
pub(crate) file_manager_locations_action_area: Option<Rect>,
pub(crate) file_manager_locations_drawer_area: Option<Rect>,
```

Every row area carries `files_generation`, `model_revision`, and exact `path`.
Input revalidates all three identities before emitting a request.

## Test Point Allocation

| Task | Test IDs |
|---|---|
| FCL-0 characterization | `SHELL-03`, existing Trail scroll/operation/handoff families |
| FCL-1 shell + explicit origin | `AUTH-01..04`, `SHELL-01..02` |
| FCL-2 worker core | `IO-01..04` |
| FCL-2 worker integration | `IO-05..06` |
| FCL-3 geometry | `GEO-01..03` |
| FCL-4 input ownership | `INPUT-01..03` |
| FCL-5 compact drawer | `DRAWER-01` |
| FCL-6 Chromium | `VIS-01..04` |
| FCL-7 closure | `GATE-01` |

All 25 IDs from the approved design are allocated exactly once.

## Task 1: FCL-0 Characterize Protected Behavior

**Files:**

- Modify: `src/app/input/mouse.rs`
- Modify: `src/ui/sidebar.rs`
- Modify: `src/app/file_manager_watcher.rs`
- Modify: `src/app/file_manager_miller.rs`
- Modify: `.codex/TASKS.md`

**Step 1 — Record the test matrix before code**

Add FCL-0 through FCL-7 with their dependencies and exact `TP-FCL-*` IDs to
`.codex/TASKS.md`. Mark only FCL-0 in progress.

**Step 2 — Add characterization tests**

Pin these current invariants without changing production behavior:

- Files activation uses the singleton stage generation;
- `TP-FCL-SHELL-03`: Spaces/Projects switching restores terminal stage and
  exact workspace/tab/pane identity;
- Trail horizontal scroll remains fractional and does not mutate location
  model state;
- file operation, agent-handoff, and watcher selected-path sources remain the
  active Trail selection;
- `AppState::assert_invariants_for_test()` stays green under
  `test_with_adversarial_identity_state()`.

Mark old global-Files-body tests with `// FCL-5 teardown` but do not delete
them in this task.

**Step 3 — Run focused characterization**

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo nextest run file_manager --locked --no-fail-fast
```

Expected: all characterization tests pass because this task protects existing
behavior. A failing characterization is investigated before FCL-1.

**Step 4 — Commit**

```text
test: characterize files shell before locations rail
```

## Task 2: FCL-1 Decouple Shell Ownership and Add Explicit Origin

**Files:**

- Create: `src/app/file_manager_locations.rs`
- Modify: `src/app/mod.rs`
- Modify: `src/app/state.rs`
- Modify: `src/app/input/mouse.rs`
- Modify: `src/ui/sidebar.rs`
- Modify: `src/ui.rs`

**Step 1 — RED tests**

Add:

- `TP-FCL-SHELL-01`: Files activation from Spaces preserves global workspace
  cards while center surface becomes `NativeFiles`;
- `TP-FCL-SHELL-02`: Files activation from Projects preserves Projects as the
  global content owner;
- `TP-FCL-AUTH-01`: `Location(Home)` remains highlighted after the file
  manager descends below Home;
- `TP-FCL-AUTH-02`: explicit nested favorite activation transfers the only
  highlight;
- `TP-FCL-AUTH-03`: `Direct(child)` does not infer a favorite;
- `TP-FCL-AUTH-04`: removed/inaccessible/model-revision/close-reopen
  identities clear or reject origin.

Run:

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo nextest run fcl_ --locked --no-fail-fast
```

Expected RED: shell tests observe `SidebarTab::Files`; origin type/helpers do
not exist.

Commit:

```text
test: define files locations shell and origin contract
```

**Step 2 — GREEN implementation**

- Make the Files tab hit activate `BuiltInAppId::Files` without assigning
  `sidebar_tab = Files`.
- Normalize any transitional in-memory `SidebarTab::Files` to `Spaces` at the
  stage-activation boundary.
- Add the pure locations state and exact highlight helper.
- Initialize it in `AppState::test_new()` and normal App state construction.
- Do not yet render the content rail; the old Files body may remain only as
  marked compatibility code until FCL-4.

Run:

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo nextest run fcl_ --locked --no-fail-fast
cargo nextest run sidebar --locked --no-fail-fast
cargo nextest run surface_host --locked --no-fail-fast
```

Expected GREEN: all new shell/origin tests and prior singleton stage tests
pass.

Commit:

```text
feat: separate files stage from global sidebar ownership
```

## Task 3: FCL-2 Add the Bounded File Manager I/O Worker

**Files:**

- Create: `src/app/file_manager_io_worker.rs`
- Modify: `src/app/mod.rs`
- Modify: `src/fm/mod.rs`

**Step 1 — RED worker tests**

Use an injected processor and deterministic gates, not wall-clock sleeps:

- `TP-FCL-IO-01`: submit returns while the processor is blocked and old model
  data remains readable;
- `TP-FCL-IO-02`: with request 1 executing, requests 2 then 3 leave only
  request 3 pending;
- `TP-FCL-IO-03`: mismatched Files/source/model generations are rejected by
  pure apply seams;
- `TP-FCL-IO-04`: panic becomes `Panicked(identity)`, then a later request
  completes successfully.

Run:

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo nextest run file_manager_io_worker --locked --no-fail-fast
```

Expected RED: module/types/worker do not exist.

Commit:

```text
test: define bounded file manager io worker contract
```

**Step 2 — GREEN worker and root preparation**

- Implement the exact interfaces above.
- Use one worker thread and one pending slot.
- Notify the render loop after success, failure, panic, or disconnect.
- Do not store `AppState` inside the worker.
- Map root failures without leaking platform-specific error strings into
  state.

Run:

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo nextest run file_manager_io_worker --locked --no-fail-fast
cargo nextest run prepare_root_navigation --locked --no-fail-fast
```

Expected GREEN: latest-pending, stale identity, panic recovery, changed-type,
missing, and permission fixtures pass.

Commit:

```text
feat: add bounded file manager io worker
```

## Task 4: FCL-2 Route Root, Miller, and Refresh Reads Through the Worker

**Files:**

- Modify: `src/app/file_manager_miller.rs`
- Modify: `src/app/file_manager_watcher.rs`
- Modify: `src/app/runtime.rs`
- Modify: `src/server/headless.rs`
- Modify: `src/app/input/file_manager.rs`

**Step 1 — RED integration tests**

Add:

- location click creates one pending root request and scheduled sync returns
  before an injected blocked reader (`TP-FCL-IO-01`);
- stale root completion after close/reopen or model change preserves old
  Trail/origin/watcher (`TP-FCL-IO-03`);
- active resident root click resets from loaded snapshots and increments no
  injected directory-read counter (`TP-FCL-IO-05`);
- Miller enter and watcher refresh submit worker messages, while their
  input/scheduled functions execute no processor callback on the calling
  thread (`TP-FCL-IO-06`);
- root failure clears pending, preserves prior state, and leaves worker
  reusable (`TP-FCL-IO-04`).

Run:

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo nextest run fcl_io_ --locked --no-fail-fast
```

Expected RED: the existing adapters invoke `prepare_*_io` synchronously.

Commit:

```text
test: expose blocking file manager navigation adapters
```

**Step 2 — GREEN adapter swap**

- Replace `execute_file_manager_navigation` preparation with worker submit.
- Replace `execute_file_manager_current_refresh` preparation with submit.
- Replace `sync_file_manager_sidebar_navigation` with
  `sync_file_manager_location_request` that validates prepared model identity
  and either executes the resident fast path or submits root work.
- Add `sync_file_manager_io_results` to apply at most the bounded drain.
- Apply results before watcher synchronization in both monolithic and
  headless scheduled loops.
- Rebind watcher only after successful current-result application.
- On stale/failure/panic, preserve Trail/origin and expose one stable bounded
  status.

Run:

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo nextest run fcl_io_ --locked --no-fail-fast
cargo nextest run file_manager_watcher --locked --no-fail-fast
cargo nextest run file_manager_miller --locked --no-fail-fast
cargo nextest run headless --locked --no-fail-fast
```

Expected GREEN: all structural non-blocking and stale-result tests pass.

Commit:

```text
fix: move file manager directory reads off scheduled loops
```

## Task 5: FCL-3 Project Responsive Content Geometry

**Files:**

- Create: `src/ui/file_manager/locations.rs`
- Modify: `src/ui/file_manager.rs`
- Modify: `src/ui.rs`
- Modify: `src/app/state.rs`

**Step 1 — RED geometry tests**

Add:

- `TP-FCL-GEO-01`: wide and standard rail/separator/Trail rectangles are
  bounded and pairwise intersections are `.is_empty()`;
- `TP-FCL-GEO-02`: compact mode is deterministic at one cell below, exactly
  at, and one cell above the derived threshold;
- `TP-FCL-GEO-03`: zero/tiny/short areas and Unicode labels never panic and
  publish only complete rows/actions.

Run:

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo nextest run fcl_geometry_ --locked --no-fail-fast
```

Expected RED: content-local layout and row snapshot do not exist.

Commit:

```text
test: define files locations responsive geometry
```

**Step 2 — GREEN projection**

- Split only the body returned by `file_manager_frame_areas`.
- Give useful Miller width precedence over decorative rail width.
- Project row identities from the exact prepared model and current files/model
  generations.
- Store one current-frame snapshot in `ViewState`.
- Pass only `layout.trail` to the existing Trail projector; do not change
  fractional horizontal scroll math.

Run:

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo nextest run fcl_geometry_ --locked --no-fail-fast
cargo nextest run trail_view --locked --no-fail-fast
```

Expected GREEN: exact boundary, disjointness, clipping, and Trail preservation
tests pass.

Commit:

```text
feat: project responsive files locations rail
```

## Task 6: FCL-4 Render and Route Content-Owned Locations

**Files:**

- Modify: `src/ui/file_manager/locations.rs`
- Modify: `src/ui/file_manager.rs`
- Modify: `src/app/input/file_manager.rs`
- Modify: `src/app/input/mouse.rs`
- Modify: `src/ui/sidebar.rs`
- Modify: `src/ui.rs`

**Step 1 — RED input/render tests**

Add:

- rail renders Favorites/Locations from the prepared model and explicit
  origin, including pending/error markers;
- primary click on one fresh complete row emits one exact request;
- `TP-FCL-INPUT-01`: vertical rail wheel changes only rail scroll;
- `TP-FCL-INPUT-02`: Trail horizontal/Shift-wheel preserves existing
  one-third cell offset and never changes rail scroll;
- `TP-FCL-INPUT-03`: separator/header/status/stale/hidden hits are inert.

Run:

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo nextest run fcl_input_ --locked --no-fail-fast
cargo nextest run fcl_render_ --locked --no-fail-fast
```

Expected RED: rows are still owned by the global sidebar.

Commit:

```text
test: define content owned locations input
```

**Step 2 — GREEN ownership swap**

- Render locations inside the Files body before the Trail.
- Route content clicks before terminal fallback and after blocking overlays.
- Require unmodified primary button and fresh row identity.
- Route vertical wheel in rail and horizontal/fallback wheel in Trail using
  their disjoint rectangles.
- Keep global sidebar workspace/agent body visible.
- Remove global Files row projection and input handling.

Run:

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo nextest run fcl_input_ --locked --no-fail-fast
cargo nextest run fcl_render_ --locked --no-fail-fast
cargo nextest run sidebar --locked --no-fail-fast
cargo nextest run file_manager --locked --no-fail-fast
```

Expected GREEN: rail owns only its cells; Trail scrolling and shell tracking
remain unchanged.

Commit:

```text
feat: move file locations into native files content
```

## Task 7: FCL-5 Add Compact Drawer and Remove Legacy Global Files Body

**Files:**

- Modify: `src/app/file_manager_locations.rs`
- Modify: `src/app/state.rs`
- Modify: `src/ui/file_manager/locations.rs`
- Modify: `src/ui/file_manager.rs`
- Modify: `src/app/input/file_manager.rs`
- Modify: `src/ui/sidebar.rs`
- Modify: `src/app/input/mouse.rs`
- Modify: `src/server/headless.rs`

**Step 1 — RED drawer test**

`TP-FCL-DRAWER-01` covers open, exact select, outside click, Esc,
wide/standard resize, close/reopen, stale row, and background input blocking.

Run:

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo nextest run fcl_drawer_ --locked --no-fail-fast
```

Expected RED: compact header action/drawer do not exist.

Commit:

```text
test: define compact files locations drawer
```

**Step 2 — GREEN drawer and teardown**

- Publish a complete `Locations` header action only in compact mode.
- Clear and paint a bounded drawer using the existing popup/picker shell
  language.
- Give drawer input priority above Trail and restore prior Files focus.
- Close safely when resized out of compact mode.
- Delete every `// FCL-5 teardown` global Files body test and production seam.
- Rename prepared presentation types from `FileManagerSidebar*` to
  `FileManagerLocation*` in one mechanical refactor after behavior is green.
- Retain `SidebarTab::Files` only if needed for backward in-memory enum
  exhaustiveness; it must have no render/input body owner.

Run:

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo nextest run fcl_drawer_ --locked --no-fail-fast
cargo nextest run sidebar --locked --no-fail-fast
cargo nextest run headless --locked --no-fail-fast
```

Expected GREEN: drawer contract passes and no global Files row geometry or
navigation request remains.

Commit:

```text
refactor: retire global files sidebar body
```

## Task 8: FCL-6 Add Playwright Chromium Oracles

**Files:**

- Modify: `src/ui/visual_fixture.rs`
- Create: `tests/visual/files-locations.spec.ts`
- Add exact PNGs under:
  `tests/visual/files-locations.spec.ts-snapshots/`

**Step 1 — Export deterministic fixtures**

Use ASCII icon profile and fixed UTC timestamps. Export:

- wide composition with active agents, 24-cell rail, four Trail columns and
  detail (`TP-FCL-VIS-01`);
- Home origin and nested favorite last-explicit-origin states
  (`TP-FCL-VIS-02`);
- standard rail and compact closed/open drawer (`TP-FCL-VIS-03`);
- pending and failed root states with prior Trail retained
  (`TP-FCL-VIS-04`).

Run the fixture generator's existing Rust test. Expected: generated JSON files
exist and contain non-empty disjoint rail/Trail regions.

**Step 2 — RED Chromium spec**

```bash
cd tests/visual
npx playwright test files-locations.spec.ts
```

Expected RED: exact new baseline PNGs are absent.

Commit:

```text
test: pin files locations chromium oracle
```

**Step 3 — Create only the new baselines**

```bash
cd tests/visual
npx playwright test files-locations.spec.ts --update-snapshots
npx playwright test files-locations.spec.ts
```

Expected GREEN: every new snapshot passes with `maxDiffPixels: 0`.

**Step 4 — Mutation proof**

Temporarily shift one deterministic separator cell in the fixture source,
regenerate, and run only `files-locations.spec.ts`.

Expected RED: at least the intended wide/standard snapshot differs and the raw
PNG digest changes.

Revert only the temporary mutation, regenerate, and rerun:

```bash
cd tests/visual
npx playwright test files-locations.spec.ts
npx playwright test
```

Expected GREEN: new spec and full Chromium suite pass. No existing baseline is
updated.

Commit:

```text
test: add files locations chromium baselines
```

## Task 9: FCL-7 Full Closure, Continuity, Graph, and Publication

**Files:**

- Modify: `.codex/TASKS.md`
- Modify: `.codex/CURRENT.md`
- Modify: `.codex/HANDOFF.md`
- Add: `.cartography/files-content-locations-rail-SYSTEM-MAP.json`

**Step 1 — Focused safety and hygiene**

This task is the executable `TP-FCL-GATE-01` acceptance point.

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo fmt
```

Then, in separate commands:

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo fmt --check
export PATH="$HOME/.local/bin:$PATH"
cargo nextest run fcl_ --locked --no-fail-fast
env -u RIPGREP_CONFIG_PATH rg -n 'unwrap\\(' src/app/file_manager_io_worker.rs src/app/file_manager_locations.rs src/ui/file_manager/locations.rs
env -u RIPGREP_CONFIG_PATH rg -n 'file_manager_sidebar_row_areas|request_file_manager_sidebar_navigation|render_file_manager_sidebar' src
```

Expected: fmt and focused tests pass; both hygiene searches return no
production residue. Test-only `unwrap()` outside the new production modules is
not a failure.

**Step 2 — Full Rust and lint gates**

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo nextest run --locked --no-fail-fast
```

Expected: all Rust tests pass with only pre-declared real-host skips.

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo clippy --all-targets --locked -- -D warnings
```

Expected: zero Linux warnings.

```bash
export PATH="$HOME/.local/bin:$PATH"
LIBGHOSTTY_VT_SIMD=false cargo clippy --bin herdr --target x86_64-pc-windows-msvc --locked -- -D warnings
```

Expected: zero Windows-target warnings.

**Step 3 — Maintenance and Chromium**

Run the repository's direct maintenance equivalents because `just` is not
installed in this environment:

```bash
python3 -m unittest discover -s scripts/tests
cd integrations/claude && bun test
cd ../../tests/visual && npx playwright test
```

Expected: Python, Bun, and Chromium suites all pass.

**Step 4 — Optional isolated manual acceptance**

Follow `.local/ISOLATED-DEV-TEST.md`. The helper must clean only its owned test
root before launch and after exit. Verify:

- global agents/workspaces remain visible;
- Home remains highlighted after deep descent;
- nested favorite explicit click transfers highlight;
- rapid location switching does not freeze;
- MX Master/trackpad horizontal input still advances fractional Trail cells;
- cleanup leaves no helper-owned server/socket/root.

This step never touches stable Herdr and is reported separately from automated
acceptance.

**Step 5 — Continuity and exact OPEN_TASKS**

- Mark FCL-0..FCL-7 complete with RED/GREEN/evidence SHAs.
- Update `.codex/CURRENT.md` and HANDOFF section 0 with honest product status.
- Rebuild HANDOFF's `OPEN_TASKS` block from `.codex/TASKS.md` and
  `.codex/CHANGE-PIPELINE-TASKS.md`.
- Every multi-occurrence Python marker search uses
  `s.index(marker, start)`.
- Reparse the generated block and require byte-exact equality with both
  sources before commit.

**Step 6 — Refresh graph and freeze map**

```bash
CBM_WORKERS=1 codebase-memory-mcp cli index_repository '{"repo_path":"/home/ayaz/projects/herdr","project":"home-ayaz-projects-herdr","mode":"fast","persistence":false}'
```

Expected: ready graph includes `FileManagerIoWorker`,
`FileManagerLocationOrigin`, `file_manager_content_layout`, and the new
worker/result call paths. Record actual node/edge counts and verified
relations in the map; never invent MCP data.

**Step 7 — Commit closure**

Run standalone fmt check, exact force-add the ignored cartography file, and
target-stage only continuity files.

Commit:

```text
docs: close files content locations rail program
```

**Step 8 — Fast-forward publish**

```bash
git fetch origin
git merge-base --is-ancestor origin/feat/native-fm HEAD
git merge-base --is-ancestor origin/master HEAD
git push origin HEAD:feat/native-fm HEAD:master
git ls-remote origin refs/heads/feat/native-fm refs/heads/master
```

Expected: both ancestry checks succeed before push and both remote refs equal
the exact local HEAD afterward.

## Plan Self-Review

- Approved spec requirements mapped: 25/25 unique `TP-FCL-*`.
- Root, Miller, and watcher reads share one bounded lane; no symptom-only fix.
- Explicit origin is client-local and exact; no prefix inference.
- Global runtime tracking remains visible while Files owns the center.
- Compact behavior has a complete action and bounded topmost drawer.
- Render/input/scheduled apply contain no directory enumeration.
- Existing fractional horizontal scroll is protected, not rewritten.
- Failure, panic, stale generation, close/reopen, changed type, tiny geometry,
  Unicode, backpressure, and worker reuse are explicit.
- No placeholder text, unspecified error handling, new dependency, protocol,
  persistence, release, upstream, stable runtime, or user-session mutation is
  authorized.
