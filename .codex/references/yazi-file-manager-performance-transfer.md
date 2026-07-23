# Yazi File-Manager Performance Transfer Reference

## Record

| Field | Value |
|---|---|
| Verified | 2026-07-22 |
| Reference checkout | `/home/ayaz/.cartography/refpool/yazi-src` |
| Pinned commit | `6d84921e7004eb8d49ba13a4acc97c6cfeb094b4` |
| Commit date | `2026-07-13T02:01:37+08:00` |
| Checkout state | clean `main...origin/main` at verification time |
| Evidence tier | `source_code` |
| Overall confidence | high (`0.95`) for cited Yazi behavior and high (`0.95`) for the completed Herdr FMN transfer; physical human acceptance was recorded separately on 2026-07-22 |

This record answers a narrow question: which Yazi architectural properties
explain responsive cursor navigation and directory preview, and which of those
properties are safe to transfer into Herdr Native Files? It is not a claim that
Yazi and Herdr have the same state model, rendering protocol, or cache budget.

## Executive decision

The useful Yazi pattern is not “cache everything.” It is the separation of
three intents:

```text
vertical cursor movement
  -> update the cursor in the current directory
  -> schedule hover/preview/watch work
  -> render the resulting state once

explicit activation
  -> enter/open the selected directory or file

directory preview
  -> abort superseded work
  -> read asynchronously
  -> apply only matching ticket/revision results
```

Herdr already had the right bounded worker and generation-validation
primitives. FMN now applies the separation directly: Up/Down and row/header
wheel mutate a cursor-only exact-path identity; Right/`l` owns directory-only
traversal and Enter remains explicit activation. The 2026-07-23 DCLICK
clarification makes primary click exact cursor focus for both files and
directories; directory click schedules preview but does not enter. A directory
cursor landing schedules a bounded discardable
preview and cannot transfer focus. A separate isolated Ghostty trace also
proved identical-coordinate host packet triplets below 2 ms, so a narrow
owner/direction/coordinate-aware gate was authorized after the semantic split.
No directory history cache was added.

## Primary-source chain

### 1. Wheel input becomes cursor movement

`yazi-plugin/preset/components/current.lua:61-75` routes current-pane wheel
input to the `arrow` action:

```lua
function Current:scroll(event, step) ya.emit("arrow", { step }) end
```

`yazi-actor/src/mgr/arrow.rs:15-35` changes `tab.current.cursor`, then invokes
hover, peek, and watch. It does not enter the hovered directory. This is the
critical interaction distinction for Herdr.

Confidence: `0.99`, direct source.

### 2. A hovered directory is previewed asynchronously

`yazi-actor/src/mgr/peek.rs:15-54` resolves the hovered item. A directory calls
`Preview::go_folder`; a file calls `Preview::go`.

`yazi-core/src/tab/preview.rs:50-77` implements folder preview as discardable
work:

- identical `folder_lock` targets can reuse the current preview;
- a previous folder loader is aborted;
- the directory metadata is checked for staleness;
- `Entries::from_dir` runs asynchronously;
- results are emitted in chunks of at most 50,000 entries or 500 ms;
- every `Part` and `Done` carries the same ticket.

The concrete chunk size and timeout are Yazi policy, not Herdr requirements.
The transferable law is cancellation plus stale-result rejection.

Confidence: `0.99`, direct source.

### 3. Directory enumeration stays off the UI path

`yazi-vfs/src/entries.rs:17-37` obtains an asynchronous directory iterator,
spawns the metadata loop, and sends entries through a channel. Lines 69-77
compare fresh directory characteristics with the prior value and skip a reload
when the directory is unchanged and the partition is not timeless.

Confidence: `0.99`, direct source.

### 4. Partial results are ticketed and revisions represent real changes

`yazi-fs/src/entries.rs:37-70` rejects a partial update whose ticket no longer
matches. Its `revision` advances only when the visible item set actually
changes. This makes “state changed” a stronger repaint signal than “an input
event occurred.”

Confidence: `0.99`, direct source.

### 5. Plain mouse motion is inert and render work is coalesced

`yazi-plugin/preset/components/root.lua:79` defines plain `Root:move` as a
no-op. `yazi-fm/src/app/app.rs:34-93` drains the currently queued events before
rendering and enforces a 10 ms next-render interval. No mutation means no
render request; a burst of valid mutations is collapsed into bounded frame
work.

Herdr's `8851b5e0` applies the first law to inert file-manager motion while
preserving overlay hover, drag, resize, and non-move input behavior.

Confidence: `0.99` for Yazi; `0.95` for the verified Herdr mapping.

### 6. Yazi's directory history is unbounded

`yazi-core/src/tab/history.rs:8-25` is a plain
`HashMap<UrlBuf, Folder>`. The cited implementation has no entry cap, byte cap,
LRU eviction, or pinned-location budget.

This is explicitly not a safe cache model for Herdr. Herdr's calibrated 100k
snapshot retained a 14.8 MB metadata lower bound, so an unbounded directory
history can turn responsive navigation into unbounded process growth.

Confidence: `0.99`, direct source and Herdr executable calibration.

## Historical Herdr source comparison at `d8583d3a`

### Correct primitives already present

- `FileManagerIoWorker` is bounded to one executing request and one latest
  pending request.
- Files/model/source/generation/column/index/path identities reject stale
  completions.
- Direct directory clicks use
  `App::queue_file_manager_trail_directory_activation`
  (`src/app/file_manager_miller.rs:12-42`) and therefore do not block the input
  loop on a cold listing.
- Complete semantic frames are latest-wins while input and ordered controls
  remain lossless.
- File preview work is discardable and leaves the serial input loop after
  `ed329058`.
- Inert pointer motion can decline a render after `8851b5e0`.

### Semantic mismatch that FMN closed

`src/app/input/file_manager.rs:111-119` routes plain Up/Down and `j/k` to
`FmState::move_trail_selection`. Visible-row wheel input does the same at
`src/app/input/file_manager.rs:495-510`; section-header wheel uses
`move_trail_selection_in_column` at lines 754-768.

`src/fm/trail_snapshots.rs:388-429` computes the landed row and immediately
calls `activate_entry`. Its own comment freezes the current behavior:
directories branch and files mark. `src/fm/mod.rs:1015-1066` then installs the
operation projection and rearms active-column follow after a branch.

Consequences:

1. Up/Down is not cursor-only.
2. A directory under the cursor can change the active column without
   Right/Enter.
3. Remaining key-repeat or wheel events can then operate on the child column,
   making one gesture look like a multi-row or cross-column jump.
4. Keyboard/header-wheel directory movement still bypasses the click-specific
   bounded activation request and can retain synchronous projection work.

Confidence: `0.99`, historical Herdr source plus user reproduction.

## Applied Herdr transfer after FMN

The published FMN line through continuity head `787bb96b` implements the
reference laws with Herdr-native bounds:

- `TrailState::cursor` is separate from the activated `TrailCol::selected`
  chain. `TrailSnapshots::move_cursor[_in_column]` changes one exact row and
  never calls `activate_entry`.
- Up/Down/`j/k`, Shift+vertical, visible-row wheel, and header wheel retain the
  exact owner column. Right/`l` is directory-only traversal; Enter is explicit
  activation. Primary click is exact cursor focus; a directory click uses the
  preview lane and retains the clicked owner column.
- `FileManagerIoRequest::TrailPreview` reuses the one-running/one-latest
  worker. Apply validates Files generation, source, owner column, entry index,
  exact path, directory kind, and the active cursor. Horizontal focus change,
  newer cursor, model/location change, missing path, or failed preparation is
  stale/inert.
- Render consumes the cursor identity, hides an old mismatched child while the
  new preview is pending, renders a matching child when ready, and declines a
  frame for clamped/coalesced movement.
- The isolated Ghostty trace captured 333 vertical packets and 226
  same-direction deltas below 2 ms in identical-coordinate triplets or
  occasional sextuplets. The gate coalesces only the same generation, owner,
  direction, and coordinates strictly below 2 ms; reversal, pointer/owner
  changes, the 2 ms boundary, and 5 ms next-detent input remain independent.
- `fm.vertical_wheel.accepted` and `fm.vertical_wheel.coalesced` provide
  non-sensitive live evidence.
- A resident preview is data availability, not focus authority. Initialization,
  auto-follow, rendering, hit testing, resize projection, and watcher rebinding
  all use `active_col()` for ownership; `deepest()` only identifies the loaded
  chain extent.

Fresh automated evidence is focused 302/302, full Nextest 3,619/3,619 plus 4
intentional skips, fmt, Linux/Windows Clippy, Python 68/68, Bun 5/5 + 12/12,
one deterministic exporter pass, and full Chromium 33/33. The exporter uses a
fixed calendar anchor, exact async-preview settlement, equal order-insensitive
mtimes, and no-follow symlink/FIFO timestamp handling. Exactly six legacy
VIS-01..06 PNGs were inspected and updated; generated JSON and VIS-07..25 did
not drift. The user then physically accepted the isolated wheel/held-arrow
build on 2026-07-22; that qualitative signal remains separate from the
automated gate counts.

Confidence: `0.95` for the source/TDD transfer and high qualitative confidence
for the reported live UX acceptance.

## Transfer laws applied by the Herdr FMN slice

### YT-1 — Movement and activation are different commands

Up, Down, `j`, `k`, Shift+Up/Down, and vertical row wheel move the cursor only
inside the exact owning Trail column. They must not change `active_col`, truncate
or extend the branch, rearm horizontal follow for a different column, or
transfer selection authority to a child.

Right/`l` owns directory-only child traversal and Enter remains explicit
activation. A primary row click focuses the exact row in its owner column; for
a directory it may prepare a bounded child preview but cannot transfer focus.
No implicit activation is inferred from the selected entry kind, and Right/`l`
on a file is inert.

### FMH clarification — directional traversal is not file activation

The Yazi comparison supports separating cursor movement from activation, but
it does not require every horizontal key to activate every entry kind. Herdr's
2026-07-22 FMH RED proved that treating Right/`l` as a generic activation alias
was observably wrong: over a file it converted the ephemeral cursor into a
`SelectedFile` Trail selection, retired the resident child, and rendered.

Herdr therefore narrows the transferred law:

- Left is one resident parent-column focus step;
- Right/`l` is one directory-only child traversal/activation step;
- Right/`l` on files/non-entries is inert;
- Enter remains an explicit activation surface;
- primary click is exact file/directory cursor focus, with directory preview
  separate from child-column focus.

This is a Herdr product-contract refinement, not a claim about Yazi's keymap.
It preserves Yazi's deeper architectural lesson—movement, preview, and
activation must have distinct authority—without copying an unrelated binding
policy. Confidence: `0.98` for the Herdr RED/GREEN distinction and `0.90` for
the reference-fit inference.

### DCLICK clarification — pointer selection is not hierarchy traversal

The user's 2026-07-23 physical report exposed the remaining conflation: plain
directory click was routed through `TrailActivate`, so the strong cursor moved
to a child column before the user pressed Right. Herdr now treats primary
pointer intent like exact cursor movement, while still reusing the bounded
preview worker:

- `TrailSnapshots::focus_entry` revalidates `(column,index,path)`, updates only
  `active_col` plus the ephemeral cursor/detail, and preserves the resident
  branch byte-for-byte;
- `FmState::focus_trail_entry` installs the resident owner operation projection
  without filesystem reads;
- directory click submits `TrailPreview`, never mouse `TrailActivate`;
- a matching preview may prepare/replace the right-hand resident child but
  keeps the clicked owner active;
- Right/`l`/Enter is the explicit hierarchy boundary; Right immediately
  highlights the first child row;
- stale click, superseded preview, backpressure, and failure remain inert with
  respect to focus authority.

This is consistent with the transferred Yazi separation of cursor, preview,
and activation, while the exact pointer binding remains a Herdr product
decision. Confidence: `0.99` for Herdr behavior and `0.90` for reference fit.

### YT-2 — Preview may follow the cursor without focus transfer

Landing on a directory may schedule a right-side child preview, but the owner
column remains active. The preview request must use the existing bounded
latest-pending lane or an equivalently bounded discardable lane and must reject
stale Files generation, model revision, source, owner column, and exact path.

### YT-3 — A decoded event and a physical gesture are measured separately

A deterministic reducer must prove that one accepted vertical movement unit
changes the cursor by exactly one row. A live isolated trace must separately
record how many terminal wheel events one physical gesture produces. Do not
invent a debounce until that trace distinguishes terminal burst/momentum from
duplicate Herdr dispatch.

### YT-4 — Render only for visible state change

An inert move or clamped cursor action must not force a full frame. A real
cursor/preview/loading/error transition may request a render. Ordered input is
never dropped merely to reduce render work.

### YT-5 — Cache authorization remains measurement-first

Resident Trail snapshots are the first cache. Home/Desktop/Downloads pre-warm
is a separate optimization and requires a first-entry latency RED. If
authorized, it must have:

- an explicit directory allowlist;
- per-directory entry and byte caps;
- mtime/change invalidation;
- bounded startup/background concurrency;
- generation/ticket validation;
- permission/missing/changed-type handling;
- deterministic teardown and memory-budget tests.

No general or unbounded LRU is authorized by Yazi.

## Non-transfer decisions

| Yazi mechanism | Herdr decision | Reason |
|---|---|---|
| Unbounded `History(HashMap<UrlBuf, Folder>)` | Reject | Herdr has explicit memory budgets and large-directory calibration |
| 50,000 entry / 500 ms preview chunks | Do not copy literally | Needs a Herdr time-to-first-row RED and profile-specific calibration |
| Tokio task per folder preview | Reuse bounded Herdr worker semantics first | Existing worker already has stronger source/generation/path authority |
| Global 10 ms render cadence | Do not replace Herdr's 16 ms policy in this bug | The proven defect was unnecessary render requests, not the constant alone |
| Plain mouse move always inert | Transfer only with owner-aware exceptions | Menus, drag, resize, tooltip, and motion-reporting panes can mutate visible state |

## Reverification checklist

Before relying on this reference after either repository moves:

1. require the exact Yazi commit above or update every cited line;
2. verify the checkout is clean before treating file:line as source evidence;
3. run Codebase Memory `index_status`, then prove current Herdr symbols such as
   `move_trail_cursor_in_column`, `FileManagerVerticalWheelBurstGate`, or
   `queue_file_manager_trail_directory_preview_identity`;
4. inspect the exact current Herdr reducer before editing;
5. preserve the user contract in
   `.codex/evidence/files-performance-fix-closure-and-navigation-followups.md`.

## Related records

- `docs/superpowers/specs/2026-07-19-herdr-files-rapid-navigation-latency-prd.md`
- `.codex/evidence/files-rapid-navigation-root-cause.md`
- `.codex/evidence/files-rapid-navigation-scale-calibration.md`
- `.codex/evidence/files-performance-fix-closure-and-navigation-followups.md`
- `.codex/skills/herdr-native-fm/lessons/errors.md`
- `.codex/skills/herdr-native-fm/lessons/golden-paths.md`
- `.codex/skills/herdr-native-fm/lessons/edge-cases.md`
