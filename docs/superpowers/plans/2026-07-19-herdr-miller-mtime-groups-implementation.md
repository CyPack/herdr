# Herdr Miller Mtime Groups Implementation Plan

> **For Codex:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to
> implement this plan task-by-task. Every behavioral production change follows
> `superpowers:test-driven-development`: observe and commit a compile-valid RED,
> then implement and commit GREEN. Do not publish a failing RED.

**Goal:** Render every Miller column as one strict mixed folder/file
mtime-descending list, grouped by local calendar section with responsive
right-side timestamps, while preserving exact-path input and watcher
authority.

**Architecture:** Filesystem collection prepares `Option<SystemTime>` once on
each `FileEntry`. One canonical comparator sorts the prepared entries. A small
pure `entry_time` module turns a fixed clock anchor plus an entry mtime into a
section and bounded label. Trail projection expands entries into logical
header/row lines; render and input consume the same prepared rectangles.

**Tech Stack:** Rust 2021, `std::fs::symlink_metadata`, already-locked
`time = 0.3.47` with `local-offset`, Ratatui 0.30 `TestBackend`, Nextest,
Playwright 1.54.1 Chromium.

**Canonical design:**
`docs/superpowers/specs/2026-07-19-herdr-miller-mtime-groups-design.md`

**Dependency audit:**
`.codex/evidence/miller-mtime-dependency-audit.md`

**Publication boundary:** CyPack fork only. Never access or mutate stable
Herdr, inherited sockets, upstream refs, release assets, or `.superpowers/`.

---

## Dependency order and exit gates

```text
MTIME-0 plan/dependency freeze
  -> MTIME-1/2 prepared mtime + canonical mixed sort
  -> MTIME-3 pure local-calendar classifier
  -> MTIME-4 grouped projection + responsive render
  -> MTIME-5 typed header input + watcher identity
  -> MTIME-6 deterministic Chromium visuals
  -> MTIME-7 full gates + continuity + graph + CyPack FF
```

Each task exits only after its focused Nextest filter passes with
`--no-fail-fast`, standalone `cargo fmt --check` passes, and its diff contains
only the named concern.

## Test matrix

| Layer | Test points | Expected RED | GREEN exit |
|---|---|---|---|
| Metadata/sort | regular file, directory, symlink identity, deleted `DirEntry`, newer file vs older directory, inverse, tie | current directory-first/name comparator disagrees with mtime order and deleted entry is not unknown-last | visible entries retain optional mtime; strict mixed descending order; unknown last |
| Calendar | fixed local dates at future, today, yesterday, days 2/7/8, year boundary, DST offset difference, missing offset | no section/label module exists | pure exact section and compact label matrix |
| Projection | multiple groups, selected deep entry, omission status, zero/tiny height, partial horizontal clipping | current one-entry/one-line geometry has no headers/time rect | headers consume logical lines; selection visible; rects disjoint |
| Input | all mouse buttons/modifiers on headers, vertical wheel over header, timestamp click, stale row | header terrain is treated as empty horizontal-scroll body | header input inert except vertical navigation; row identity remains exact |
| Watcher | changed mtime reorders selected and multiselected paths, delete, child branch | index changes expose any index authority | cursor, bulk selection, Trail child/detail follow exact paths |
| Visual | normal, narrow/partial, reorder selection | new spec has no baseline | spec-scoped baselines and controlled pixel mutation proof pass |

---

### Task 1: Freeze direct dependency and baseline

**Files:**

- Modify: `Cargo.toml`
- Modify: `Cargo.lock` (root dependency list only)
- Modify: `.codex/TASKS.md`
- Modify: `.codex/CURRENT.md`
- Modify: `.codex/HANDOFF.md`
- Modify: `.planning/STATE.md`

**Step 1: add the exact direct dependency**

Add beside the other general dependencies:

```toml
time = { version = "=0.3.47", features = ["local-offset"] }
```

Do not run `cargo update`. Because Cargo records the root package dependency
list in `Cargo.lock`, generate the one-line metadata change offline:

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo tree -i time@0.3.47 --offline
```

**Step 2: prove zero dependency delta**

Run:

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo tree -i time@0.3.47 --locked
cargo tree -e features -i time@0.3.47 --locked
git diff -- Cargo.lock
```

Expected: exact 0.3.47 remains transitive and direct, existing
`local-offset/std` features remain, and the only lockfile diff is one `"time"`
entry in the existing `herdr` dependency list. No package, version, source, or
checksum record changes.

**Step 3: close MTIME-0 continuity**

Mark dependency audit and implementation plan complete. Regenerate
`.codex/HANDOFF.md` `OPEN_TASKS` from both registries and require exact equality.

**Step 4: validate and commit**

Run standalone:

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo fmt --check
```

Targeted stage only the named files, force-add the ignored plan/spec, inspect
`git diff --cached --check`, then commit:

```text
build: declare locked time dependency
```

---

### Task 2: RED — characterize prepared metadata and mixed order

**Files:**

- Modify: `src/fm/mod.rs` tests only

**Step 1: add deterministic filesystem-time helpers**

Inside the existing `#[cfg(test)] mod tests`, add helpers based on stable std:

```rust
fn set_modified(path: &Path, modified: SystemTime) {
    let file = std::fs::File::open(path).expect("open fixture for mtime");
    file.set_times(std::fs::FileTimes::new().set_modified(modified))
        .expect("set fixture mtime");
}
```

For directories, create all children first and set the directory timestamp
last. Use `UNIX_EPOCH + Duration::from_secs(...)`, never wall clock sleeps.

**Step 2: add compile-valid behavior tests**

Add these exact families against public/current seams:

- `newer_file_sorts_before_older_directory`;
- `newer_directory_sorts_before_older_file`;
- `deleted_dir_entry_stays_visible_and_sorts_as_unknown`;
- Unix-only `symlink_uses_its_own_modification_time`;
- equal timestamp numeric/case names remain deterministic.

The deleted-entry test must:

1. collect `DirEntry` values from `read_dir`;
2. remove one path;
3. pass the retained values to `collect_directory_entries`;
4. call `sort_entries`;
5. assert the deleted entry remains visible and sorts after known entries.

This proves metadata failure does not become omission.

**Step 3: run RED**

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo nextest run --locked --no-fail-fast newer_file_sorts_before_older_directory
cargo nextest run --locked --no-fail-fast deleted_dir_entry_stays_visible_and_sorts_as_unknown
```

Expected: assertions fail because current ordering is directory-first/name
based. Failure must be behavioral, not a missing symbol or compile error.

**Step 4: commit RED**

After standalone fmt:

```text
test: pin miller entry mtime ordering
```

---

### Task 3: GREEN — prepare mtime and replace canonical comparator

**Files:**

- Create: `src/fm/entry_time.rs`
- Modify: `src/fm/mod.rs`
- Mechanical test-fixture updates: every `FileEntry { ... }` under `src/`

**Step 1: define prepared metadata**

Add:

```rust
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub kind: entry_kind::FileEntryKind,
    pub modified: Option<std::time::SystemTime>,
}
```

Use `std::fs::symlink_metadata(entry.path()).and_then(|m| m.modified()).ok()`.
This preserves the listed path's identity, including a symlink's own timestamp.
Failure sets `None`; it never drops the entry.

Every synthetic fixture must explicitly state `modified: None` or a fixed
`Some(...)`. Do not add a hidden wall-clock default.

**Step 2: replace `sort_entries`**

The comparator is:

```rust
b.modified
    .cmp(&a.modified)
    .then_with(|| natsort::natsort(a.name.as_bytes(), b.name.as_bytes(), true))
    .then_with(|| a.name.cmp(&b.name))
    .then_with(|| a.path.cmp(&b.path))
```

`Option` ordering plus reversed operands gives known newest-first and `None`
last. No file kind participates.

**Step 3: replace old characterizations**

Change `dirs_first_then_natural_order` and
`snapshot_sort_and_symlink_classification_baseline` so they assert the
approved mtime contract. Keep semantic kind assertions independent from sort
position.

**Step 4: verify GREEN**

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo nextest run --locked --no-fail-fast mtime
cargo nextest run --locked --no-fail-fast snapshot_sort
cargo nextest run --locked --no-fail-fast trail_snapshots
cargo fmt --check
```

Expected: all metadata/sort families pass and no existing Trail test relies on
directory-first ordering.

**Step 5: commit GREEN**

```text
feat: sort miller entries by modification time
```

---

### Task 4: RED/GREEN — pure local-calendar sections and labels

**Files:**

- Create: `src/fm/entry_time.rs`
- Modify: `src/fm/mod.rs` module registration

`entry_time.rs` must appear before its `#[cfg(test)] mod tests`.

**Step 1: write RED tests first**

Define the intended API in tests:

```rust
pub(crate) enum FileTimeSection {
    Future,
    Today,
    Yesterday,
    Previous7Days,
    Older,
    UnknownDate,
}

pub(crate) struct FileTimePresentation {
    pub section: FileTimeSection,
    pub label: String,
}

pub(crate) struct LocalCalendarAnchor {
    now: SystemTime,
}

pub(crate) fn present_file_time(
    modified: Option<SystemTime>,
    anchor: LocalCalendarAnchor,
) -> FileTimePresentation;
```

The tests use exact `OffsetDateTime`/`UtcOffset` values converted to
`SystemTime`; no current time is read.

Cover:

- future local date;
- same date at 00:00 and 23:59;
- yesterday across midnight;
- exactly days 2 and 7;
- day 8;
- same/different year older labels;
- DST samples whose per-timestamp offsets differ;
- `None`;
- a test-only injected offset resolver failure.

Expected RED: module/API does not exist locally. Do not commit a
compile-failing state; add a minimal test-only compile seam in the same RED
worktree and preserve a behavioral failing assertion before the RED commit.

**Step 2: implement minimal pure projection**

Production `LocalCalendarAnchor::now()` captures `SystemTime::now()` once.
For both anchor and modified timestamps:

```rust
let utc = time::OffsetDateTime::from(system_time);
let offset = time::UtcOffset::local_offset_at(utc).ok()?;
let local = utc.to_offset(offset);
```

Compare `to_julian_day()` values. Resolve each timestamp separately for DST.
Use manual formatting from integer getters. Do not enable `time` formatting.

Section labels are exact:

```rust
"Future"
"Today"
"Yesterday"
"Previous 7 Days"
"Older"
"Unknown Date"
```

Timestamp labels:

- future/today/yesterday: `HH:MM`;
- previous seven: `Mon HH:MM`;
- older same year: `DD Mon`;
- older different year: `DD Mon YYYY`;
- unknown: `—`.

Month/weekday abbreviations are fixed English arrays local to the module.

**Step 3: verify and commit pair**

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo nextest run --locked --no-fail-fast entry_time
cargo fmt --check
```

Commit RED:

```text
test: pin miller calendar groups
```

Commit GREEN:

```text
feat: project miller calendar groups
```

---

### Task 5: RED — grouped Trail logical geometry

**Files:**

- Modify: `src/ui/file_manager/trail_view.rs` tests only

**Step 1: add fixed-anchor projection tests**

Introduce a test-only/projection-level API that accepts
`LocalCalendarAnchor`. Production wrappers may construct the anchor once, but
tests must inject it.

Add tests:

- `mtime_sections_insert_non_actionable_logical_rows`;
- `selected_entry_remains_visible_after_section_headers`;
- `omission_status_remains_last_after_grouped_rows`;
- `zero_and_tiny_heights_do_not_orphan_or_overflow_headers`;
- `timestamp_rect_is_complete_or_absent`;
- `name_timestamp_actions_are_disjoint`;
- `partial_column_never_exposes_partial_timestamp`;
- `unicode_name_and_horizontal_source_offsets_stay_cell_correct`.

Expected RED: no `section_headers` or `timestamp` geometry exists.

**Step 2: run and commit RED**

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo nextest run --locked --no-fail-fast mtime_sections
cargo nextest run --locked --no-fail-fast timestamp_rect
cargo fmt --check
```

Commit:

```text
test: pin miller timestamp geometry
```

---

### Task 6: GREEN — grouped projection and responsive render

**Files:**

- Modify: `src/ui/file_manager/trail_view.rs`
- Modify: `src/ui/file_manager.rs` only if a shared style/helper is needed
- Modify: `src/ui/mod.rs` exports

**Step 1: add typed view data**

```rust
pub(crate) struct TrailSectionHeaderView {
    pub trail_index: usize,
    pub section: FileTimeSection,
    pub label: &'static str,
    pub rect: Rect,
}

pub(crate) struct TrailTimestampView {
    pub rect: Rect,
    pub text: String,
}
```

Add `timestamp: Option<TrailTimestampView>` to `TrailRowView`, and
`section_headers: Vec<TrailSectionHeaderView>` plus `line_start: usize` to
`TrailColumnView`.

Headers intentionally contain no path or entry index.

**Step 2: project logical lines**

Build an internal sequence:

```rust
enum TrailLogicalLine {
    Header(FileTimeSection),
    Entry(usize, FileTimePresentation),
}
```

Only insert a header when the section changes in the already-sorted entry
vector. Calculate the selected entry's logical-line index, then choose
`line_start` so it is inside the height remaining after omission status.

Do not render an orphan header as the final visible line: if the window ends
on a header without one following entry from that section, omit that header.
Preserve `viewport_start` as the first visible entry index for compatibility.

**Step 3: project disjoint row geometry**

Logical order:

```text
[icon + name flexible][timestamp optional][actions complete]
```

Actions remain at the far right and retain current widths. Reserve timestamp
plus one separator only when:

- the filename zone retains at least the icon and one filename cell;
- the timestamp interval is fully inside the visible horizontal interval.

Otherwise omit the timestamp entirely and return the cells to the filename.
Never emit a clipped timestamp rect.

**Step 4: render prepared headers and labels**

Render headers with existing subdued foreground plus bold modifier. Render
timestamps with subdued foreground and the same selected/multiselected
background as the name/action cells. All text is already prepared; render
must not call filesystem, clock, or local-offset APIs.

Add:

```rust
pub(crate) fn trail_section_header_at(
    view: &TrailViewSnapshot,
    x: u16,
    y: u16,
) -> Option<&TrailSectionHeaderView>;
```

**Step 5: verify and commit GREEN**

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo nextest run --locked --no-fail-fast trail_view
cargo nextest run --locked --no-fail-fast fractional
cargo nextest run --locked --no-fail-fast directory_omissions
cargo fmt --check
```

Commit:

```text
feat: render miller mtime groups
```

---

### Task 7: RED/GREEN — header-aware input and exact-path refresh

**Files:**

- Modify: `src/app/input/file_manager.rs`
- Modify: `src/fm/trail_snapshots.rs`
- Modify: `src/fm/mod.rs`
- Modify: `src/app/file_manager_watcher.rs` tests

**Step 1: add input RED tests**

Create a fixed-mtime directory with at least two sections. Project the view,
obtain a real header rectangle, then assert:

- left/right/middle click changes no cursor, trail, detail, or selection;
- Ctrl/Shift/Alt header gestures are consumed without mutation;
- plain vertical wheel over a header moves selection in that header's owning
  column, not horizontal offset;
- timestamp click resolves the same exact row/path as name click;
- stale generation/revision header and row frames fail closed.

The current implementation should fail because a header is rowless empty
column body and vertical wheel is normalized into horizontal scroll.

Commit RED:

```text
test: pin grouped miller input and refresh
```

**Step 2: implement typed terrain routing**

In `handle_file_manager_mouse`, resolve both `trail_row_target` and
`trail_section_header_target` from the same live frame.

`plain_wheel_over_empty_trail` must require both targets to be absent.
For a live header plus plain vertical wheel, move selection in the header's
`trail_index`, not merely the active column. Add a bounded
`TrailSnapshots::move_selection_in_column` seam that reuses
`activate_entry`; `FmState` installs the same operation projection and
follow-active rule as existing Trail activation.

All header buttons/modifiers return `Consumed`; no header path can reach row
activation, context menu, resize, agent reference, rename, or delete.

**Step 3: add watcher RED/GREEN proof**

In `src/app/file_manager_watcher.rs`, set explicit file times, select a path
and multi-select another path, then change an mtime and inject the existing
owned watcher event. Assert:

- vector indices reorder;
- selected exact path is unchanged;
- multi-selection and anchor exact paths remain;
- Trail selected child and detail path remain;
- directory and preview generations advance exactly once.

No second selection lifecycle should be added. Existing
`current_refresh_cursor`, `reconcile_multi_selection`, and
`rebuild_trail_bridge` remain the implementation authority.

**Step 4: focused verification and GREEN commit**

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo nextest run --locked --no-fail-fast grouped_miller
cargo nextest run --locked --no-fail-fast watcher
cargo nextest run --locked --no-fail-fast trail
cargo fmt --check
```

Commit GREEN:

```text
fix: preserve grouped miller interaction authority
```

---

### Task 8: Playwright Chromium oracle

**Files:**

- Modify: `src/ui/visual_fixture.rs`
- Create: `tests/visual/mtime-groups.spec.ts`
- Create:
  `tests/visual/mtime-groups.spec.ts-snapshots/vis-15-mtime-groups-linux.png`
- Create:
  `tests/visual/mtime-groups.spec.ts-snapshots/vis-16-mtime-groups-narrow-linux.png`
- Create:
  `tests/visual/mtime-groups.spec.ts-snapshots/vis-17-mtime-reorder-selection-linux.png`

Use VIS-15..17 because VIS-01..14 already exist. Do not reuse IDs.

**Step 1: deterministic fixtures**

Extend the ignored fixture exporter with an ASCII-icon Trail tree. Create all
contents first, then apply fixed `FileTimes` last. Use the same injected fixed
calendar anchor as Rust tests. Export:

- `vis-15-mtime-groups`: mixed directories/files across Today, Yesterday,
  Previous 7 Days, Older;
- `vis-16-mtime-groups-narrow`: narrow plus partial horizontal clipping where
  timestamps omit and actions remain complete;
- `vis-17-mtime-reorder-selection`: selected exact path highlighted after its
  mtime moves it into a newer section.

Fixture cleanup starts before creation and runs after export. It owns only its
unique `/tmp/herdr-vis15-*` root and touches no server/process.

**Step 2: generate only these fixtures**

```bash
export PATH="$HOME/.local/bin:$PATH"
HERDR_VISUAL_FIXTURE_DIR="$PWD/tests/visual/fixtures/generated" \
  cargo test --locked write_visual_fixtures -- --ignored --exact
```

**Step 3: create baselines spec-scoped**

From `tests/visual/` only:

```bash
npx playwright test mtime-groups.spec.ts --update-snapshots
npx playwright test mtime-groups.spec.ts
```

Never run global `--update-snapshots`.

**Step 4: mutation proof**

Copy one generated JSON to a temporary file outside tracked paths, mutate one
visible header/timestamp cell, render both through the harness, and assert raw
PNG buffers differ. Also temporarily point one test at the mutated fixture and
observe one expected screenshot failure, then restore the spec and rerun
green. Do not commit failure artifacts.

**Step 5: full Chromium suite and commit**

```bash
cd tests/visual
npx playwright test
```

Expected: prior 22 tests plus the three new tests pass; report the fresh actual
count.

Commit:

```text
test: add miller mtime chromium oracle
```

---

### Task 9: Full production gates

**Files:** no product edits unless a gate finds a real defect; any defect gets
its own RED/GREEN forward-fix pair.

Run these as separate commands, reading every exit code:

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo fmt --check
```

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo nextest run --locked --no-fail-fast
```

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo clippy --all-targets --locked -- -D warnings
```

```bash
export PATH="$HOME/.local/bin:$PATH"
LIBGHOSTTY_VT_SIMD=false cargo clippy --bin herdr --target x86_64-pc-windows-msvc --locked -- -D warnings
```

```bash
python3 -m unittest \
  scripts.test_agent_detection_manifest_check \
  scripts.test_changelog \
  scripts.test_docs_translation_parity \
  scripts.test_preview \
  scripts.test_trail_t7_teardown \
  scripts.test_vendor_libghostty_vt \
  scripts.test_vendor_portable_pty
```

```bash
bun test src/integration/assets/herdr-agent-state.test.ts
```

```bash
cd workers/plugin-marketplace
bun test
```

```bash
cd tests/visual
npx playwright test
```

Additional gates:

```bash
git diff --check
git diff c10e124c -- Cargo.lock
env -u RIPGREP_CONFIG_PATH rg -n 'unwrap\\(\\)' src \
  --glob '*.rs'
git status --short
```

The unwrap search is an audit, not an automatic zero rule for pre-existing
test code. Inspect only newly added production lines and require zero new
production `unwrap()`.

---

### Task 10: Continuity, graph, and CyPack publication

**Files:**

- Modify: `.codex/TASKS.md`
- Modify: `.codex/CURRENT.md`
- Modify: `.codex/HANDOFF.md`
- Modify: `.planning/STATE.md`
- Create/update:
  `.codex/evidence/miller-mtime-groups-closure.md`

**Step 1: record exact evidence**

Record fresh counts for Rust, Chromium, maintenance, Linux/Windows Clippy,
dependency/lockfile, mutation, and diff gates. Mark MTIME-0..7 only when their
evidence exists.

**Step 2: exact OPEN_TASKS synchronization**

Parse unchecked blocks from `.codex/TASKS.md` and
`.codex/CHANGE-PIPELINE-TASKS.md`, regenerate only the bounded
`OPEN_TASKS_START/END` region using offset-aware
`s.index(marker, start)`, update the count line, reparse, and require exact
list equality before commit.

**Step 3: standalone fmt and closure commit**

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo fmt --check
```

Commit:

```text
docs: record miller mtime closure
```

**Step 4: single-worker graph refresh**

```bash
CBM_WORKERS=1 codebase-memory-mcp cli index_repository \
  '{"repo_path":"/home/ayaz/projects/herdr","project":"home-ayaz-projects-herdr","mode":"fast","persistence":false}'
```

Require `ready`, then re-read `FileEntry`, `sort_entries`,
`present_file_time`, `project_trail_view_inner`,
`trail_section_header_at`, and header input routing from the graph.

**Step 5: fast-forward publication**

```bash
git fetch origin feat/native-fm master
git merge-base --is-ancestor origin/feat/native-fm HEAD
git merge-base --is-ancestor origin/master HEAD
git push origin HEAD:feat/native-fm HEAD:master
git ls-remote origin refs/heads/feat/native-fm refs/heads/master
```

Expected: both remote refs equal exact local final HEAD. Never push upstream.

## Rollback points

- `c10e124c`: approved written design, no Rust changes;
- dependency commit: direct declaration only, lockfile unchanged;
- each GREEN commit: deployable and fully focused;
- remote always remains at a GREEN/closure checkpoint because RED heads are
  never pushed.

Published changes are forward-fixed. Destructive reset is not part of this
plan.
