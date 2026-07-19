# Herdr Files Content Locations Rail and Non-Blocking Navigation Design

## Status

- Design date: 2026-07-19
- Product area: Native Files / global shell / Miller Trail
- Change class: presentation ownership, navigation authority, filesystem I/O
  scheduling, responsive geometry, mouse routing
- User decision: Option A approved
- Required visual verifier: Playwright Chromium
- Source branch: `feat/native-fm`
- Source checkpoint: `98cc72db937a550646b4d98f85f21c9b7aff0995`
- Stable runtime safety: this design authorizes no access to the installed
  Herdr process, inherited socket, existing user sessions, upstream, release
  assets, or user-owned `.superpowers/`

## Product Decision

The global Herdr left panel remains the agent/workspace runtime tracker while
Native Files is active. `Files` becomes a stage launcher, not the owner of the
global sidebar body.

The Native Files content area becomes:

```text
global agent/workspace panel | locations rail | scrollable Miller Trail | detail
```

The locations rail is a full-height, file-surface-local navigation panel. It
contains the existing prepared Favorites and Locations items. The Miller Trail
to its right retains path-growing columns, fractional one-third horizontal
scrolling, active-end follow, per-path widths, row actions, and the detail
panel.

The selected location is explicit navigation-origin identity. It is not
derived from the current directory and is not guessed from the longest path
prefix.

Filesystem-backed directory preparation moves off the UI/headless scheduled
loop into one bounded, generation-safe read lane. Render, hit testing, and
scheduled tasks must not wait for directory enumeration or per-entry metadata
reads.

## User-Visible Contract

### Global Left Panel

Activating Native Files preserves the global left panel's agent/workspace
tracking content. It does not replace that content with Home, Desktop,
Downloads, configured favorites, removable locations, Trash, or Root.

The visible Files control still:

- opens or reactivates the singleton Native Files stage;
- preserves existing modifier, button, collapsed, overlay, and stale-hit
  fail-closed rules;
- does not create a second Files instance;
- does not mutate the active Spaces/Projects sidebar-content selection.

`SidebarTab::Files` is not persisted today. The approved change therefore
requires no session-snapshot or wire migration. Any transitional in-memory
`SidebarTab::Files` value is normalized to `Spaces` before the new projection
becomes authoritative.

### Content Locations Rail

The locations rail occupies the left edge of the Native Files body, below the
shared identity/action header and above the shared status row. It uses the
existing prepared location model and performs no filesystem access during
render.

Sections remain explicit:

- Favorites;
- Locations.

Every accessible item has one complete row hit target. Headers, separators,
clipped rows, inaccessible items, empty space, and stale frame geometry are
inert.

The rail owns vertical navigation only:

- primary click activates an exact prepared item;
- vertical wheel scrolls the rail when the rail needs scrolling;
- keyboard focus can move within the rail only when the rail owns focus;
- horizontal wheel, Shift-wheel, and Trail fallback scrolling do not mutate
  the rail.

The Trail owns horizontal navigation only inside its own rectangle. Rail and
Trail hit rectangles are computed from the same frame projection and never
overlap.

### Explicit Root Authority

One typed location-origin value records how the current Trail root was opened:

```text
Location(path)  exact accessible rail item selected by the user
Direct(path)    deep link, startup path, plugin action, or non-rail navigation
```

Highlight rules:

1. clicking Home records `Location(Home)` and highlights Home;
2. descending through any number of Home subdirectories keeps Home
   highlighted;
3. clicking a nested configured favorite records that exact favorite and
   transfers the single highlight to it;
4. a nested favorite never steals highlight merely because the current path
   happens to be beneath it;
5. a direct/deep-link root has no rail highlight unless its root exactly
   equals an accessible rail item;
6. inaccessible, removed, changed-type, stale-generation, or model-removed
   origins clear or reject highlight rather than selecting an ancestor;
7. close/reopen establishes a fresh origin and cannot reuse a stale one.

The location origin is TUI/client presentation state. It adds no server wire
fact and grants no filesystem authorization. Every action still revalidates
the prepared model and exact live path type at the effect boundary.

### Non-Blocking Navigation

All read-only directory preparation used by root switching, Miller directory
entry, and watcher/current refresh is executed through one bounded
file-manager I/O worker lane.

The lane has these rules:

- at most one request executes and at most one latest request waits;
- a newer navigation request replaces an older pending read request;
- every request carries the active Files generation and the exact source
  directory, directory generation, preview generation, and Miller revision
  needed by its apply path;
- every result is applied atomically only when all relevant identities still
  match;
- stale, cancelled, closed, replaced, panicked, missing, permission-denied,
  changed-type, or structurally invalid results preserve the current Trail;
- closing Files retires the lane generation and prevents late application;
- a worker panic becomes a typed failure and the lane remains reusable;
- no worker result submits terminal input, mutates a workspace/session
  runtime, or writes to the filesystem.

While a different root is loading:

- the existing Trail remains visible and interactive for non-conflicting
  actions;
- the requested rail row shows a bounded loading marker;
- a second location click supersedes the pending destination;
- successful completion atomically installs the new Trail, its explicit
  location origin, and one watcher binding;
- failure removes the loading marker, preserves the previous Trail and
  origin, and shows a stable bounded status message.

Same-root behavior:

- clicking the already active location resets/focuses its root from the
  already loaded Trail snapshot when available;
- this path performs no directory enumeration or metadata read;
- if the root snapshot is no longer resident or valid, the action uses the
  bounded worker rather than blocking the UI loop.

## Responsive Layout

Layout modes are driven by content minimums, not device names.

### Wide

- persistent locations rail;
- one-cell separator;
- remaining width belongs to the existing Trail/detail projector;
- target rail width is 24 cells, clamped to 18–28 cells.

### Standard

- persistent compact rail at 16–20 cells;
- section labels and item names truncate by display-cell width;
- markers remain complete-or-omitted;
- at least one useful Miller column retains priority over decorative rail
  width.

### Compact

- the persistent rail is omitted;
- a complete `Locations` action in the Files header opens a bounded
  locations drawer/overlay;
- the overlay clears its target area, owns input above the Trail, shows only
  complete rows, and restores prior Files focus on close;
- the Trail receives the complete body width while the overlay is closed.

Mode boundaries are calculated from:

- rail minimum;
- separator width;
- the existing useful Miller-column minimum;
- complete header action width.

Tests cover the exact boundary, one cell below, and one cell above. Zero,
tiny, and short-height areas are panic-free and publish no partial targets.

## Current-State Evidence and Root Causes

Fresh Codebase Memory evidence at the design checkpoint:

- project: `home-ayaz-projects-herdr`;
- graph: 23,656 nodes / 125,342 edges, status `ready`;
- freshness: current mtime grouping symbols and current sidebar/navigation
  bodies were returned.

### Root Cause 1 — Highlight Uses the Wrong Identity

`src/ui/sidebar.rs::file_manager_sidebar_item_is_current` currently requires:

```text
SidebarTab::Files
AND item.accessible
AND FmState.cwd == item.path
```

The existing test
`file_sidebar_current_pill_tracks_exact_accessible_open_cwd` deliberately pins
that contract. Descending below Home therefore removes Home's highlight.
Nested favorites cannot express the user's selected root without guessing
from path ancestry.

The approved design replaces cwd-derived presentation authority with explicit
location-origin identity.

### Root Cause 2 — Files Owns the Global Sidebar Body

`render_workspace_list` dispatches `SidebarTab::Files` to
`render_file_manager_sidebar` and returns before rendering workspace cards.
`compute_view_internal` similarly projects Files rows from the global
`LeftPanel`, and `file_manager_sidebar_path_at` hit-tests those global sidebar
rectangles.

The Native Files stage already has independent singleton authority through
`StageState::activate_files()`. Files activation and global sidebar-content
selection are therefore separable without a protocol or persisted-state
change.

### Root Cause 3 — Directory I/O Runs on the Scheduled/UI Loop

`sync_file_manager_sidebar_navigation` is called directly by both the
monolithic and headless scheduled loops. It synchronously:

1. performs live path metadata checks;
2. calls `FmState::open_trail_to`;
3. enumerates the root directory;
4. classifies entries;
5. reads symlink-preserving modification time for every visible entry;
6. sorts and constructs Trail snapshots;
7. performs another live path check;
8. swaps the state.

The mtime program correctly moved metadata outside render, but it increased
the amount of synchronous work performed by this already blocking path.
Large, cold, removable, remote, or otherwise slow directories can therefore
delay the entire scheduled/render response.

The codebase already has bounded latest-request-wins and stale-generation
patterns in `file_preview_worker`, `image_preview_worker`, and
`file_operation_worker`. The approved design adapts those established
patterns rather than introducing another concurrency model.

## Options Considered

### Option A — Content-Local Rail and Bounded I/O Lane

Selected. It preserves agentic runtime visibility, matches Finder's
file-window-local locations model, establishes unambiguous root identity, and
removes filesystem latency from UI scheduling.

### Option B — Keep the Global Files Sidebar

Rejected. Changing highlight and async loading alone would reduce defects but
would continue hiding the agent/workspace tracker while Files is active.

### Option C — Drawer-Only Locations

Rejected as the default. It maximizes Trail width but removes persistent
orientation on standard and wide terminals. It remains the compact fallback.

## Ownership and Module Boundaries

### TUI Presentation State

- active stage and singleton Files generation;
- active Spaces/Projects global sidebar content;
- location origin and pending location;
- locations rail scroll/focus;
- responsive rail/drawer mode;
- rail, separator, Trail, drawer, and row geometry;
- loading/error presentation.

### Pure File-Manager State

- exact current directory and Trail root;
- aligned Trail snapshots;
- directory/preview/Miller generations;
- prepared location model;
- pure apply/reject transitions for worker results.

### Runtime Handle

- bounded file-manager I/O worker;
- request/result channels and generation lifecycle;
- wake notification.

### Explicit Non-Owners

- render does not read the filesystem, clock, channels, or runtime handles;
- mouse coordinates do not authorize a path without model revalidation;
- the global sidebar does not own location navigation after the swap;
- the watcher does not establish location highlight identity;
- server protocol and private client socket gain no presentation fields.

## Dependency Chain

```text
Files launcher
  -> StageState::activate_files
  -> preserve Spaces/Projects global sidebar content
  -> compute Native Files body split
  -> project locations rail and Trail from disjoint rectangles
  -> render both from prepared state

location click
  -> exact frame row identity
  -> current prepared model revalidation
  -> typed location request + Files generation
  -> same-root resident fast path OR bounded I/O worker
  -> prepared root result
  -> generation/model/path revalidation
  -> atomic FmState + location-origin commit
  -> single watcher rebind
  -> next pure projection

Miller directory activation / watcher refresh
  -> existing typed request
  -> shared bounded I/O lane
  -> existing pure generation-safe apply seam
  -> exact-path Trail reconciliation
```

Implementation dependency order:

```text
FCL-0 baseline and characterization
  -> FCL-1 explicit location-origin authority
  -> FCL-2 bounded read-only FM I/O lane
  -> FCL-3 content rail geometry and responsive modes
  -> FCL-4 render/input ownership swap
  -> FCL-5 compact drawer and teardown
  -> FCL-6 Playwright Chromium and isolated runtime evidence
  -> FCL-7 full gates, continuity, graph, and publication
```

No later layer may begin with an earlier RED left unresolved.

## Test-Point Contract

| ID | Test | Expected result | Reason |
|---|---|---|---|
| `TP-FCL-AUTH-01` | click Home, then descend through several Miller directories | Home remains the only highlighted location | cwd equality is the reported highlight defect |
| `TP-FCL-AUTH-02` | Home and nested favorite both contain current path; activate each in turn | only the explicitly activated item is highlighted | longest-prefix selection would recreate ambiguity |
| `TP-FCL-AUTH-03` | direct/deep-link root under a favorite | no inferred highlight unless the root exactly equals an item | presentation must not invent origin identity |
| `TP-FCL-AUTH-04` | item removed, inaccessible, changed type, stale generation, close/reopen | request/result is rejected or origin cleared without partial mutation | stale paths must fail closed |
| `TP-FCL-SHELL-01` | activate Files from Spaces with active agents/workspaces | Native Files owns center while global sidebar keeps agent/workspace projection | primary UX goal |
| `TP-FCL-SHELL-02` | activate Files while Projects content is selected | Files stage opens without silently rewriting the global sidebar content owner | stage and sidebar selection are independent |
| `TP-FCL-SHELL-03` | click Spaces/Projects after Files activation | existing terminal-stage restoration and identity invariants remain exact | decoupling cannot break established shell lifecycle |
| `TP-FCL-IO-01` | location input and scheduled sync with a deliberately blocked directory reader | input/scheduled call returns without waiting; old Trail remains available | structural non-blocking proof is stronger than timing alone |
| `TP-FCL-IO-02` | first worker request blocks; second and third arrive | first completes, only the latest pending request executes/applies | bounded latest-request-wins contract |
| `TP-FCL-IO-03` | result completes after navigation, refresh, close/reopen, or model change | stale result is rejected with no partial state/origin/watcher mutation | async work must not resurrect retired state |
| `TP-FCL-IO-04` | worker preparation panics or returns missing/permission/changed-type failure | stable typed failure, old Trail preserved, worker accepts next generation | failure containment and lane reuse |
| `TP-FCL-IO-05` | click already active resident root | exact root is focused/reset with zero directory-read counter increase | repeat shortcut must not trigger avoidable I/O |
| `TP-FCL-IO-06` | Miller enter and watcher refresh use the shared lane | neither performs directory enumeration on the UI/headless scheduled thread | fixes the architectural latency source, not one symptom |
| `TP-FCL-GEO-01` | wide/standard body projection | rail, separator, Trail, and detail rectangles are bounded and pairwise disjoint | one geometry source prevents cross-surface clicks |
| `TP-FCL-GEO-02` | compact boundary, one cell below/at/above | persistent rail converts deterministically to a complete drawer action | prevents responsive flicker and partial targets |
| `TP-FCL-GEO-03` | zero/tiny/short body and long Unicode labels | panic-free; labels truncate by cells; clipped rows/actions publish no hit target | terminal geometry is adversarial |
| `TP-FCL-INPUT-01` | vertical wheel over rail | only rail scroll changes | rail and Trail have independent scroll authority |
| `TP-FCL-INPUT-02` | horizontal/Shift/plain fallback wheel over Trail | existing fractional Trail offset changes by its current contract; rail does not move | preserve proven one-third scrolling |
| `TP-FCL-INPUT-03` | wheel/click over separator, header, status, stale frame, or hidden rail | no navigation or scroll mutation | gaps and prior-frame geometry must stay inert |
| `TP-FCL-DRAWER-01` | compact drawer open, select, outside click, Esc, resize, close/reopen | top overlay owns input, exact selection routes once, prior focus restores safely | compact fallback must not leak input |
| `TP-FCL-VIS-01` | wide deterministic fixture with active agents, locations rail, deep Trail and detail | intended four-surface composition is visible | proves the requested agentic/Finder layout |
| `TP-FCL-VIS-02` | nested favorite/root highlight fixture | one explicit origin remains highlighted while cwd is deeper | visual oracle for the collision |
| `TP-FCL-VIS-03` | standard and compact/drawer fixtures | complete responsive transition with usable Miller width | catches cell-level clipping and hierarchy regressions |
| `TP-FCL-VIS-04` | loading and failure fixtures | old Trail remains visible; one requested row shows bounded state; failure is explicit | async correctness must be understandable |
| `TP-FCL-GATE-01` | focused families, full Nextest, Linux/Windows Clippy, maintenance, Playwright Chromium, hygiene | all pass with only pre-declared real-host skips; no production `unwrap()` or residue | cross-surface refactor requires full evidence |

Performance acceptance uses structural ownership as the primary oracle:
filesystem enumeration cannot be reached from input, render, or scheduled
apply code. A bounded deterministic blocked-reader test proves responsiveness.
Wall-clock measurements are supplemental and must not be the sole regression
gate.

## Visual Verification

Playwright Chromium consumes deterministic Ratatui cell fixtures in ASCII icon
profile. New baselines are created spec-scoped only.

Required fixtures:

1. wide: global agent/workspace sidebar + 24-cell locations rail + at least
   four Trail columns + detail;
2. standard: compact persistent rail + fractional leading/trailing Trail
   columns;
3. compact: full-width Trail with closed and open locations drawer;
4. deep root: Home origin highlighted while cwd is multiple levels deeper;
5. nested favorite: last explicit origin wins without dual highlight;
6. loading: old Trail plus one pending location;
7. failure: old Trail plus bounded explicit failure.

At least one controlled one-cell mutation must change the raw PNG and fail the
intended snapshot before the fixture and baseline are restored. Global
`--update-snapshots` is forbidden.

## Error Handling

- Missing/permission/changed-type target: retain current Trail and origin;
  publish one bounded error.
- Unreadable entry metadata: retain the entry under the existing Unknown Date
  contract; do not fail the whole root.
- Stale geometry/model/generation: inert, no error spam.
- Backpressure: replace only the pending read request; never grow a queue.
- Worker disconnect/panic: typed unavailable result, generation retired, lane
  recreated only through its normal lifecycle.
- Watcher event during pending root load: retire or coalesce by generation;
  never apply refresh data to a different root.
- Compact resize while drawer is open: recompute/clamp or close safely; no
  background hit ownership.

## Non-Goals

- changing mixed mtime ordering or Finder-like date groups;
- changing fractional one-third Trail scrolling;
- re-ranking the preserved scroll-version lab;
- adopting or shipping optional preview plugins;
- modifying file-operation ordering or destructive-action authority;
- drag and drop;
- a general-purpose tiling/split manager;
- persisted rail width in this increment;
- server wire/protocol fields;
- release documentation/assets;
- stable Herdr, inherited socket, existing user sessions, upstream, or
  `.superpowers/`.

## Reference Evidence

- Apple Finder sidebar: Favorites and Locations are file-window-local
  navigation and can be shown, hidden, reordered, and resized:
  <https://support.apple.com/en-gb/guide/mac-help/mchl83c9e8b8/mac>
- Apple Finder Column View: hierarchical columns are the file-content view to
  the right of navigation:
  <https://support.apple.com/en-euro/guide/mac-pro/apddf030866a/mac>
- Apple navigation guidance:
  <https://developer.apple.com/design/human-interface-guidelines/navigation-and-search>
- Yazi layout configuration: file-manager content is decomposed into explicit
  panel ratios and prepared metadata presentation:
  <https://yazi-rs.github.io/docs/configuration/yazi/>
- Ratatui core `Layout`/`Constraint` primitives are direct APIs; responsive
  mode selection, authority, worker lifecycle, and failure behavior are Herdr
  reimplementations, not copied third-party code.

## Acceptance

The program is complete only when:

1. activating Files preserves the agent/workspace global sidebar;
2. locations render and hit-test only inside the Native Files content area;
3. explicit origin identity resolves every nested-path highlight case;
4. no directory enumeration or per-entry metadata read occurs in render,
   input, or scheduled apply paths;
5. root, Miller-enter, and watcher read results are bounded and
   generation-safe;
6. existing Trail horizontal scroll, row/path authority, preview, operations,
   handoff, and watcher behavior remain green;
7. Playwright Chromium visual and mutation evidence is complete;
8. full Rust, Linux/Windows, maintenance, hygiene, continuity, graph, and
   CyPack publication gates pass;
9. stable Herdr/socket, user processes, upstream, release assets, and
   `.superpowers/` remain untouched.
