# Files Directory Primary-Click Focus — Closure Evidence

Date: 2026-07-23 CEST

Branch: `feat/native-fm`

Scope: Native Files client-local Miller Trail input, prepared state projection,
bounded directory preview, focus ownership, and related documentation. No
server protocol, platform, dependency, cache, terminal runtime, or stable Herdr
change is authorized by this slice.

## Outcome

Primary click on a live file or directory row means **focus this exact row in
this exact Miller column**. A directory click may prepare the right-hand child
through the existing bounded preview worker, but it does not enter or focus the
child. Up/Down continues in the clicked column. Right/`l`/Enter is the explicit
hierarchy transition; Right highlights the child's first actionable row in the
same dispatch.

This supersedes only the older Herdr binding that treated primary directory
click as activation. It preserves the accepted vertical/wheel, Left/Right,
Locations Rail, action-authority, and render-purity laws.

## Product vocabulary

| Term | Meaning |
|---|---|
| Locations Rail | Fixed Home/Desktop/Downloads/configured-location region |
| Miller Trail | Dynamic root/ancestor/current/child/detail region |
| Owner column | `TrailState::active_col()`; the one Trail column receiving vertical input |
| Row cursor | Exact file/directory path in the owner column; strong filled highlight while Trail owns top-level focus |
| Resident child | Prepared right-side directory data; availability is not focus authority |
| Preview | Bounded, discardable preparation that may replace resident child data but cannot cross the hierarchy focus boundary |
| Activation/traversal | Explicit Right/`l`/Enter transition into a directory child |

## User-observed defect

1. Primary click on a file painted the expected strong filled blue row.
2. Primary click on a directory appeared to leave focus as `[none]`.
3. The next Up/Down moved inside the directory's child column rather than the
   column that was clicked.
4. Returning to the clicked owner required Left, even though no explicit Right
   had been issued.

Expected: the clicked directory row is immediately the single strong cursor;
vertical movement remains in its column until explicit Right/`l`/Enter.

## Graph-first architecture and dependency chain

Built-in Codebase Memory was current at the pre-edit source with 24,327 nodes
and 129,874 edges. Freshness was proven by resolving the latest FFO symbol
`focus_file_manager_trail`, not by trusting `status=ready` alone.

Relevant module boundaries:

```text
ui/file_manager/trail_view.rs
  TrailRowView { trail_index, entry_index, entry_path, rects }
             |
             v
app/input/file_manager.rs
  handle_file_manager_row_mouse
             |
             +-- retired: queue_file_manager_trail_directory_activation
             |              -> FileManagerIoRequest::TrailActivate
             |              -> activate_trail_entry -> Branched
             |              -> child active_col
             |
             `-- current: focus_trail_row
                            -> FmState::focus_trail_entry
                            -> TrailSnapshots::focus_entry
                            -> resident operation projection
                            -> optional TrailPreview
                                           |
                                           v
                              matching completion prepares child
                              and restores/retains owner active_col

explicit Right/l/Enter
  -> active exact directory identity
  -> bounded TrailActivate or resident move_active_right
  -> focus_first_active_trail_entry
```

`ui/` remains pure projection/render. `fm/trail_snapshots.rs` owns exact
prepared row validation and cursor/detail semantics. `fm/mod.rs` owns the
temporary operation projection. `app/input/` owns pointer/key intent and worker
submission. No shared runtime or private socket protocol field was added.

## Root-cause hypotheses

| Hypothesis | Verdict | Evidence | Confidence |
|---|---|---|---|
| H1 plain directory click is routed as activation and transfers `active_col` to child | Confirmed | Graph/snippet showed `handle_file_manager_row_mouse -> queue_file_manager_trail_directory_activation`; `TrailActivateOutcome::Branched` focuses the child. RED observed `left: 1`, `right: 0`. | 0.99 |
| H2 state is correct but render paints the wrong column | Rejected | Render derives the strong cursor from Trail top-level focus, `active_col`, and exact `selected_entry`; existing semantic style tests are green. It faithfully painted the wrong child-owned state. | 0.99 |
| H3 row hit geometry resolves the wrong target | Rejected | The click opened the intended directory, and `TrailRowView` carries exact current-frame column/index/path identity before mutation. Stale-path and stale-frame tests remain inert. | 0.98 |

## Frozen test points

| ID | Stimulus | Expected result | Failure prevented |
|---|---|---|---|
| TP-DCLICK-01 | Primary click a directory in any visible Trail column | Exact row and exact owner column become current; one strong cursor | Hidden child focus transfer |
| TP-DCLICK-02 | Up/Down after directory click | Exactly one row in the clicked owner column | Vertical movement inside child |
| TP-DCLICK-03 | Right after matching preview | Child receives focus and first actionable row is highlighted immediately | Extra Down requirement / `[none]` child |
| TP-DCLICK-04 | Primary file click | Exact file selection/detail behavior remains intact | File preview regression |
| TP-DCLICK-05 | Stale frame/index/path | Consumed or rejected without model/focus/operation mutation | TOCTOU wrong-row focus |
| TP-DCLICK-06 | Directory click and preview | Zero serial filesystem reads; bounded latest request; failure/backpressure cannot erase clicked focus | UI stall / async authority theft |
| TP-DCLICK-07 | Render projection before and after preview | Same exact owner row qualifies for the shared strong style; resident child is weaker/non-owner | Missing or duplicate strong highlight |
| TP-DCLICK-08 | Rapid clicks on two directories | Latest exact cursor/preview wins; owner remains parent | Older completion focus theft |
| TP-DCLICK-09 | Rightmost visible owner with hidden child | Click keeps owner visible; Right reveals/focuses child | Click-implied viewport traversal |
| TP-DCLICK-10 | 10,000 mixed Miller actions | Trail/snapshot/focus invariants remain exact | Long-run authority drift |

## TDD evidence

### RED

Commit: `da413d1d` — `test: specify directory click focus ownership`

Run: `1fcd96df-30c4-4b39-b673-e7c43f178d37`

Result: 0/2 pass, 2/2 assertion failures, compilation successful.

Both tests failed at the reported defect:

```text
click must keep owner focus before preview completion
left: 1
right: 0
```

The tests cover immediate clicked state, no input-loop filesystem reads,
top-level Trail ownership, render projection identity, same-column Down,
post-preview owner focus, explicit Right, and first-child highlight.

### GREEN

Production commit: `b90a177d` —
`fix: keep directory clicks in the current column`.

Run `3f217ee8-9a05-4490-90f4-b6f9d1e28903`: 2/2 pass.

Old-contract audit `19492c0e-e982-4723-80a6-278edd3debbf`: 141/144 pass. The
three failures were all explicit retired expectations that click moves focus to
the child; file, stale, right-click, keyboard, wheel, and action tests passed.

Converted contract run `106666de-3b42-40e1-9796-60471c6daf2b`: 3/3 pass.

Reducer plus full input/invariant run
`6d4c0671-b18b-481a-8ebc-8d8c19f4666c`: 145/145 pass. It includes exact stale
identity, rapid click, preview race, hidden-child viewport, right-click,
keyboard, wheel, action ownership, and 10,000-action invariants.

## Implementation decisions

### Exact cursor focus is a prepared-state reducer

`TrailSnapshots::focus_entry` validates the immutable frame triple
`(trail_col, entry_index, expected_path)` before mutation. It changes the owner
column and ephemeral cursor/detail only. It never calls `select_dir`, truncates
the Trail, appends a child, or reads the filesystem. A directory clears file
detail; a file prepares the existing deferred detail state.

### Operation projection remains resident and disk-free

`FmState::focus_trail_entry` reuses `install_trail_operation_projection` so
File Action Bar and context operations resolve against the clicked row owner.
The projection clones already prepared owner entries/status/writability/parent
identity and performs no directory enumeration.

### Primary click uses preview, not activation

`focus_trail_row` transfers top-level owner only after exact row validation.
Plain directory click clears incompatible multi-selection and submits
`TrailPreview`. File click retains exact replace-selection semantics.

The retired mouse-only `queue_file_manager_trail_directory_activation` wrapper
was removed. Keyboard Right/`l`/Enter continues to use typed activation
identity and destination policy.

### Async completion cannot steal focus

Preview application requires matching Files generation, source projection,
owner column, entry index, exact path, directory kind, and current cursor.
The prepared clone branches only to load resident child data, then restores the
owner column. If the user moves, presses Right, switches location, closes Files,
or a newer request wins, the old completion is inert.

## Performance and bounds

- No new dependency, cache, collection, timer, sleep, debounce, retry, channel,
  or server message.
- No synchronous filesystem read was added to pointer dispatch.
- The only async directory path reuses the existing one-running/one-latest
  worker and exact result identity.
- Resident Trail depth remains bounded by the existing maximum.
- No Yazi-style unbounded directory history was introduced.
- Render remains pure and consumes prepared state only.

## Verification ledger

| Gate | Evidence | Status |
|---|---|---|
| RED exact behavior | Nextest `1fcd96df-30c4-4b39-b673-e7c43f178d37`, 0/2 at expected assertion | PASS as RED |
| GREEN exact behavior | Nextest `3f217ee8-9a05-4490-90f4-b6f9d1e28903`, 2/2 | PASS |
| Converted legacy contracts | Nextest `106666de-3b42-40e1-9796-60471c6daf2b`, 3/3 | PASS |
| Reducer/input/adversarial | Nextest `6d4c0671-b18b-481a-8ebc-8d8c19f4666c`, 145/145 | PASS |
| Post-commit related surface | Nextest `6162259b-0b7a-4bbe-a6ca-b065e88c727d`, 256/256, 3,433 skipped | PASS |
| Full Rust | Nextest `130f0c02-5a9e-4844-9667-9e72219d8a40`, 3,683/3,683, 6 intentional skips, 35.761 s | PASS |
| Fmt and platform lint | `cargo fmt --check`; Linux all-target Clippy; Windows `x86_64-pc-windows-msvc` Herdr Clippy; all `-D warnings` | PASS |
| Maintenance and integrations | Python 68/68; integration asset Bun 5/5; plugin marketplace Bun 12/12 | PASS |
| Visual oracle | Playwright Chromium 35/35, 1 worker, 18.8 s; `updateSnapshots` never requested and no PNG regenerated | PASS |
| Architecture/performance diff | `05b9ba70..b90a177d`: server/protocol/platform/Cargo boundary empty; no added `read_dir`, metadata I/O, spawn/channel/timer/cache primitive | PASS |
| Post-edit graph/ADR | Full graph 24,357 nodes / 129,888 edges; exact three focus symbols resolved; six-section 5,697-character FFO+DCLICK ADR written and read back | PASS |
| Exact commits and CyPack equality | To be filled after publication | PENDING |
| Isolated physical E2E | User-driven after publication | PENDING |

## Manual acceptance matrix

Run only the cleanup-first isolated helper after the build and publication
gates are green. Do not address stable Herdr.

1. Click a file: that row is immediately strong filled blue.
2. Click a directory in root/current/ancestor/rightmost visible columns: the
   exact clicked row is immediately strong filled blue.
3. Press Up/Down: move one row in that same column.
4. Click the directory again, then press Right: the child becomes visible and
   its first row is immediately strong filled blue without an extra Down.
5. Press Left: focus returns one column to the exact parent context.
6. Rapidly alternate file and directory clicks: no freeze, no blank focus, no
   child focus without Right/`l`/Enter.
7. Exit normally and require zero throwaway socket/root residue.

## Claim, evidence, confidence

| Claim | Evidence | Confidence |
|---|---|---|
| Root cause was click-to-activation routing, not render or hit geometry | Current graph/snippets plus exact RED `active_col` mismatch | 0.99 |
| Primary directory click now retains exact owner focus before and after preview | 2/2 GREEN, 3/3 converted contracts, 145/145 reducer/input suite | 0.99 |
| Right immediately focuses the child first row | Two independent route tests plus hidden-child viewport test | 0.99 |
| No serial filesystem read or unbounded cache was added | Profiler assertion plus clean source/boundary/hot-primitive diff audit | 0.99 |
| Physical UX is accepted | Must come from the user's isolated manual trial | Pending |
