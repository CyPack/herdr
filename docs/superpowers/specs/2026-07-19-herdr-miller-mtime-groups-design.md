# Herdr Miller Mtime Sorting and Groups Design

## Status

- Design date: 2026-07-19
- Product area: Native Files / Miller Trail columns
- Change class: filesystem metadata, ordering, grouped projection, row rendering
- User decision: approved
- Approved ordering: directories and files are mixed in one strict
  modification-time-descending order
- Required visual verifier: Playwright Chromium
- Source branch: `feat/native-fm`
- Source checkpoint: `d009c6456aa93935520a1e47284d0e0d0554046e`
- Stable runtime safety: this design authorizes no access to the installed
  Herdr process, inherited socket, or user-owned `.superpowers/`

## Product Decision

Every Miller column becomes a Finder-like chronological list:

1. all directories and files participate in one mixed ordering;
2. the newest modification time appears first;
3. entries are divided by non-overlapping local-calendar sections;
4. every entry displays a compact modification time at the right side of the
   name area;
5. section headers are visible but never become file-operation or navigation
   authority;
6. metadata, calendar grouping, and display text are prepared before render.

The approved section vocabulary is:

- `Future` only when clock skew or a future filesystem timestamp exists;
- `Today`;
- `Yesterday`;
- `Previous 7 Days`;
- `Older`;
- `Unknown Date` only for entries whose modification time cannot be read.

Ordinary users normally see the middle four groups. Exceptional timestamps
remain truthful instead of being silently mislabeled.

## User-Visible Contract

### Ordering

The primary comparator is modification time descending across every entry
kind. A newer file sorts above an older directory, and a newer directory sorts
above an older file.

Ties are deterministic:

1. natural case-insensitive name order;
2. raw name order;
3. exact path bytes where the platform exposes two otherwise identical
   display names.

Missing modification time always sorts after every known time. The old
directory-first rule is deliberately replaced for Miller snapshots.

### Calendar Boundaries

Groups use local calendar dates, not rolling 24-hour durations:

- `Today`: same local date as the prepared clock anchor;
- `Yesterday`: exactly one local calendar date earlier;
- `Previous 7 Days`: two through seven local dates earlier;
- `Older`: every earlier known date;
- `Future`: a local date after the clock anchor;
- `Unknown Date`: no trustworthy modification time.

This keeps midnight and daylight-saving transitions truthful. Tests inject an
explicit clock/timezone anchor; production obtains one anchor before view
projection and never reads the clock during render.

### Row Timestamp

The timestamp is right-aligned immediately before row action cells:

- `Future`, `Today`, and `Yesterday`: `HH:MM`;
- `Previous 7 Days`: abbreviated weekday plus time when it fits, otherwise
  abbreviated day and month;
- `Older`: day and abbreviated month, adding the year when it differs from the
  anchor year;
- `Unknown Date`: `—`.

Responsive priority is:

1. preserve complete row-action cells;
2. preserve a usable icon and filename;
3. render the complete timestamp only when its reserved cells fit;
4. omit the whole timestamp rather than clipping or overlapping it.

The filename remains the exact path identity. Timestamp text is presentation
only and never grants action authority.

## Scope

### In Scope

- prepare one optional modification time for every visible `FileEntry`;
- strict mixed entry ordering by modification time descending;
- deterministic tie and unknown-time ordering;
- local-calendar group classification;
- section-header projection inside every Trail column;
- header-aware vertical viewport calculations;
- right-aligned responsive timestamp cells;
- header-aware mouse/wheel behavior;
- exact-path cursor and multi-selection preservation after watcher reorder;
- deterministic Ratatui buffer tests;
- Playwright Chromium visual snapshots and mutation proof;
- Linux and canonical Windows lint/build coverage;
- full repository gates, graph refresh, continuity, and CyPack-only
  fast-forward publication.

### Non-Goals

- creation-time, birth-time, access-time, or Git commit-time sorting;
- user-configurable sort modes in this increment;
- persisted sort preferences or protocol fields;
- server/runtime ownership;
- filesystem or clock reads during render;
- locale framework or general translation system;
- sticky section headers;
- changing preview, Kitty image, file-operation, agent-reference, or Trail
  horizontal-scroll authority;
- adopting a new package without a separate dependency delta and platform
  audit;
- touching stable Herdr, upstream, release assets, or `.superpowers/`.

## Current-State Evidence

Fresh Codebase Memory evidence at the design checkpoint:

- project: `home-ayaz-projects-herdr`;
- graph: 23,556 nodes / 125,078 edges, status `ready`;
- freshness: current `read_directory_snapshot`,
  `project_trail_view_inner`, and `render_trail_view` bodies were returned.

Current ownership:

- `FileEntry` stores `name`, `path`, and semantic `kind`; it has no mtime;
- `collect_directory_entries` prepares entries outside render;
- `read_directory_snapshot` calls one `sort_entries` seam;
- `sort_entries` currently orders directories first, then natural name;
- `TrailSnapshots` owns aligned prepared directory snapshots;
- `project_trail_view_inner` maps entry indices directly to one-cell rows;
- `TrailRowView` carries exact path/index/rect and action geometry;
- `render_trail_view` consumes the prepared snapshot and shared entry renderer;
- `render_entry_row_clipped` owns icon/name display-cell clipping;
- watcher refresh already preserves selection by exact path.

The required change therefore crosses prepared model, pure projection,
render/input geometry, and watcher reconciliation, but adds no runtime or wire
state.

## Options Considered

### Option A — Strict Mixed Mtime, Finder-Like Groups

Selected. It directly matches the user's decision, provides one auditable
ordering rule, and avoids misleading directory-first exceptions.

### Option B — Directory-First Within Each Date Group

Rejected. It preserves an old invariant but violates strict chronological
order whenever a file is newer than a directory.

### Option C — Keep Name Sort and Add Visual Date Labels

Rejected. Labels would imply chronological grouping while row order remained
unrelated to the displayed metadata.

### Option D — Compute Metadata and Relative Time During Render

Rejected. It violates Herdr's pure-render contract, makes frame cost depend on
filesystem latency, and destabilizes Chromium fixtures.

## Target Architecture

### Layer 0 — Metadata Authority

`FileEntry` gains one optional modification-time value prepared during
directory collection. The value is filesystem evidence, not presentation
text.

The metadata read must be integrated with existing entry classification where
possible so symlink classification and mtime do not perform duplicate
unbounded work. Exact symlink policy is characterized before implementation:
the displayed path's own metadata is preferred unless current supported-target
classification already requires followed metadata.

Metadata failure does not hide an entry. The entry remains visible with
`Unknown Date`, preserving the existing FMR visibility contract.

### Layer 1 — Canonical Ordering

One pure comparator replaces directory-first ordering for prepared Miller
snapshots:

```text
known mtime descending
  -> natural case-insensitive name
  -> raw name
  -> exact path
  -> unknown mtime last
```

Sorting occurs once per snapshot/reload. Render and input never sort.

### Layer 2 — Calendar Projection

A pure classifier accepts:

- optional entry modification time;
- explicit prepared `now`;
- explicit local offset/calendar conversion evidence.

It returns a stable section enum and preformatted timestamp label. Production
acquires a single clock anchor outside render. Unit and visual fixtures inject
fixed anchors.

The existing lockfile contains `time 0.3.47`, but it is not a direct
dependency. The implementation plan must first audit:

- the locally installed crate API and required features;
- direct-dependency and lockfile delta;
- license and advisory status;
- Unix/macOS/Windows local-offset behavior;
- failure behavior when local offset cannot be determined.

No API claim from an unavailable documentation service is accepted as
evidence. If the locked crate cannot satisfy the contract safely, the plan
uses a small platform-isolated conversion seam built from existing platform
dependencies.

### Layer 3 — Grouped Trail View

The snapshot entry vector remains the model and selection authority. Section
headers exist only in the projected view.

Each projected column contains a logical display sequence:

```text
section header
entry row
entry row
next section header
entry row
...
optional omission status
```

Entry rows retain:

- exact `trail_index`;
- exact `entry_index`;
- exact `entry_path`;
- full row rectangle;
- name, timestamp, and action rectangles.

Section headers retain only section identity and rectangle. They carry no
entry index/path and cannot be activated, selected, renamed, deleted, or sent
to an agent.

Vertical viewport math uses logical display rows while cursor movement stays
over the entry vector. A selected entry must remain visible after accounting
for preceding headers. Empty, zero-height, and status-row layouts remain
panic-free.

### Layer 4 — Render and Responsive Geometry

`render_trail_view` renders section headers from prepared view data and entry
timestamps from prepared labels. It performs no filesystem or clock I/O.

For entry rows:

- action cells remain complete and rightmost;
- timestamp cells sit immediately to their left;
- icon/name receives the remaining logical width;
- partial horizontal column clipping uses the same source-cell coordinates for
  render and hit testing;
- a timestamp is complete or omitted;
- section labels truncate by display cells and never spill into a divider.

Existing palette roles are reused. Section headers use subdued/bold visual
hierarchy and remain readable without color-only semantics.

### Layer 5 — Input Authority

Entry hit testing remains exact path plus generation/revision authority.

- clicking an entry timestamp is still a click on that exact entry row;
- clicking a section header is consumed inert;
- right/middle/modified header input remains inert;
- plain wheel over a section header uses the column's vertical navigation
  behavior and must not fall through to horizontal empty-body scrolling;
- row actions and timestamps never overlap;
- stale projected rows remain fail closed after watcher reorder.

### Layer 6 — Watcher and Lifecycle

Watcher refresh can reorder every entry. Reconciliation must preserve:

- cursor by exact selected path;
- explicit multi-selection by exact paths;
- Trail selected child by exact path;
- active column and horizontal offset rules;
- preview/detail generation for the still-selected path.

If the selected path disappears, existing deterministic fallback rules remain
authoritative. Mtime grouping does not create a second selection lifecycle.

### Layer 7 — Verification and Publication

Semantic authority remains Rust tests. Playwright verifies human-visible
composition from real Ratatui cell fixtures.

The feature is not complete until:

- focused metadata/sort/group/projection/input/watcher families pass;
- a controlled visual mutation fails;
- Playwright Chromium passes;
- full Rust, Linux/Windows Clippy, maintenance, fmt, diff, and unwrap gates
  pass;
- graph is reindexed single-worker and current changed symbols are readable;
- continuity is exact;
- only CyPack `feat/native-fm` and `master` are fast-forwarded;
- remote SHAs equal local final GREEN;
- stable Herdr/socket and `.superpowers/` remain untouched.

## Dependency Chain

```text
M0 research and characterization
  -> M1 prepared mtime metadata
  -> M2 strict mixed-mtime sorting
  -> M3 local-calendar group projection
  -> M4 grouped Trail geometry and timestamp render
  -> M5 header-aware input and watcher reconciliation
  -> M6 Playwright Chromium visual oracle
  -> M7 full gates, continuity, graph, and publication
```

No layer may start production code before its behavior-specific RED is
observed and committed locally.

## Test-Point Catalog

| ID | What is tested | Expected result | Reason |
|---|---|---|---|
| `TP-MTIME-01-METADATA` | regular file, directory, supported symlink, broken/special entry, metadata error | every visible entry keeps one optional prepared mtime; failure stays visible as unknown | metadata failure must not recreate invisible entries |
| `TP-MTIME-02-MIXED-SORT` | newer file versus older directory and inverse | all kinds are mixed strictly by mtime descending | direct approved behavior |
| `TP-MTIME-03-TIES` | equal mtimes, case variants, numeric names, duplicate display names, unknown mtimes | deterministic natural/raw/path tie order; unknowns last | watcher refresh and screenshots require stable order |
| `TP-MTIME-04-CALENDAR` | local midnight, yesterday, days 2 and 7, day 8, future, unknown, DST transition | exact non-overlapping section enum | rolling seconds misclassify calendar days |
| `TP-MTIME-05-LABEL` | same day, recent week, older same/different year, unknown | compact complete timestamp strings | right-side date must be truthful and bounded |
| `TP-MTIME-06-PROJECTION` | multiple groups, empty groups, selected deep row, omission status, zero/tiny height | headers consume logical rows; selected entry remains visible; no overlap/panic | headers change vertical geometry |
| `TP-MTIME-07-GEOMETRY` | normal/narrow/partially clipped columns with row actions | disjoint name/time/action rects; timestamp complete or absent | timestamp must not corrupt actions or horizontal clipping |
| `TP-MTIME-08-INPUT` | click timestamp, click/header buttons/modifiers, wheel over header, stale row | exact entry acts; headers never act; header wheel stays vertical; stale fails closed | new visible cells must have explicit ownership |
| `TP-MTIME-09-WATCHER` | mtime change reorders selected and multi-selected rows; delete/rebranch/refresh | exact paths retain authority or use existing disappearance fallback | indices change whenever metadata changes |
| `TP-MTIME-10-PURITY` | repeated render and profiling counters | byte-identical fixed-anchor render; zero filesystem/clock reads in render | Herdr render invariant |
| `TP-MTIME-VIS-01` | normal mixed directory/file groups | Chromium shows chronological sections and right timestamps | primary visual acceptance |
| `TP-MTIME-VIS-02` | narrow and partially scrolled column | names/actions remain usable; timestamps omit cleanly | responsive visual acceptance |
| `TP-MTIME-VIS-03` | selected entry after mtime-driven reorder | highlight follows exact path in the new group | human-visible identity proof |
| `TP-MTIME-GATES` | full repository/platform/visual/graph/Git gates | all applicable gates green with exact counts and zero residue | production closure |

## Task and Subtask Tree

### MTIME-0 — Research, Design, and Baseline

- `MTIME-0.1`: graph current metadata → sort → Trail projection → render/input
  → watcher chain;
- `MTIME-0.2`: characterize current directory-first/natural-name behavior and
  exact-path reorder preservation;
- `MTIME-0.3`: audit local calendar dependency/platform choices;
- `MTIME-0.4`: freeze test points, dependency order, rollback, and gate matrix;
- `MTIME-0.5`: obtain design approval and write the code-level TDD plan.

### MTIME-1 — Prepared Metadata

- `MTIME-1.1 RED`: require optional mtime on visible entries;
- `MTIME-1.2 GREEN`: collect bounded metadata outside render;
- `MTIME-1.3`: characterize symlink and metadata-failure policy;
- `MTIME-1.4`: prove no render-time metadata access.

### MTIME-2 — Strict Mixed Sorting

- `MTIME-2.1 RED`: newer file must sort above older directory;
- `MTIME-2.2 GREEN`: replace directory-first comparator;
- `MTIME-2.3`: add tie, unknown, future, Unicode, and reload determinism;
- `MTIME-2.4`: migrate the old directory-first characterization to the
  approved contract.

### MTIME-3 — Calendar Groups

- `MTIME-3.1 RED`: classify fixed timestamps into all section buckets;
- `MTIME-3.2 GREEN`: add pure fixed-anchor calendar classifier/formatter;
- `MTIME-3.3`: cover midnight, DST, offset failure, future, and unknown;
- `MTIME-3.4`: add prepared group/timestamp fields to pure view data.

### MTIME-4 — Grouped Projection and Render

- `MTIME-4.1 RED`: section headers must consume logical rows while selection
  remains visible;
- `MTIME-4.2 GREEN`: project header and entry display rows;
- `MTIME-4.3 RED`: timestamp/name/action geometry must be disjoint and
  responsive;
- `MTIME-4.4 GREEN`: render headers and right-aligned timestamps;
- `MTIME-4.5`: preserve fractional horizontal clipping, omission status,
  detail panel, Unicode, and zero/tiny geometry.

### MTIME-5 — Input and Watcher Reconciliation

- `MTIME-5.1 RED`: header click inert and header wheel vertically owned;
- `MTIME-5.2 GREEN`: add typed header terrain without entry authority;
- `MTIME-5.3 RED`: mtime reorder must preserve exact cursor/multi-selection/
  Trail child identity;
- `MTIME-5.4 GREEN`: reuse existing exact-path reconciliation;
- `MTIME-5.5`: stale generation, delete, rebranch, preview/detail, and
  10,000-action invariants.

### MTIME-6 — Playwright Chromium

- `MTIME-6.1`: export fixed-clock ASCII-profile fixtures;
- `MTIME-6.2`: add `VIS-01` normal mixed groups;
- `MTIME-6.3`: add `VIS-02` narrow/partial-column behavior;
- `MTIME-6.4`: add `VIS-03` exact-path selection after reorder;
- `MTIME-6.5`: prove a controlled timestamp/header cell mutation fails;
- `MTIME-6.6`: run Chromium spec-scoped baseline creation and full suite.

### MTIME-7 — Production Closure

- `MTIME-7.1`: run focused and full Nextest with `--no-fail-fast`;
- `MTIME-7.2`: run fmt separately, Linux Clippy, canonical Windows Clippy,
  maintenance, Bun/Python, diff, unwrap, and artifact gates;
- `MTIME-7.3`: run isolated manual runtime only if automated evidence exposes
  a host-specific gap, using cleanup-first/cleanup-last test ownership;
- `MTIME-7.4`: synchronize `.codex` continuity and exact open-task copy;
- `MTIME-7.5`: reindex Codebase Memory with `CBM_WORKERS=1` and re-read changed
  symbols;
- `MTIME-7.6`: verify targeted atomic history, fast-forward ancestry, CyPack
  push, and exact remote SHA equality.

## Commit and Rollback Strategy

Planned atomic subjects:

```text
docs: design miller mtime groups
test: pin miller entry modification metadata
feat: prepare miller entry modification metadata
test: pin mixed miller mtime ordering
feat: sort miller entries by modification time
test: pin miller calendar groups
feat: project miller calendar groups
test: pin miller timestamp geometry
feat: render miller mtime groups
test: pin grouped miller input and refresh
fix: preserve grouped miller interaction authority
test: add miller mtime chromium oracle
docs: record miller mtime closure
```

Each RED/GREEN pair is independently reviewable. No failing RED is pushed as a
remote branch head. If a layer fails its exit gate, the last remote GREEN
remains deployable and the layer is forward-fixed locally.

## Acceptance Criteria

- files and directories are strictly mixed by mtime descending;
- equal and unknown times have deterministic order;
- `Today`, `Yesterday`, `Previous 7 Days`, and `Older` groups use local
  calendar boundaries;
- exceptional future/unknown timestamps remain truthful;
- every visible entry shows a complete right-side timestamp when space allows;
- narrow layouts omit timestamps before harming identity/actions;
- section headers never become entry authority;
- header wheel behavior remains vertical and does not trigger horizontal
  fallback;
- watcher reorders preserve exact path cursor, multi-selection, and Trail
  child identity;
- render performs zero filesystem and clock I/O;
- existing horizontal fractional scroll, detail preview, row actions, image
  placement, omission status, and sidebar navigation stay green;
- Playwright Chromium snapshots and mutation proof pass;
- full Rust/Linux/Windows/maintenance gates pass;
- graph, continuity, Git ancestry, and CyPack remote SHA evidence are fresh;
- stable Herdr/socket, upstream, and `.superpowers/` remain untouched.
