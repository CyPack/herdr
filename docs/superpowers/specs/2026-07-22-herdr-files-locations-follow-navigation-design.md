# Herdr Files Locations Follow Navigation Design

**Date:** 2026-07-22
**Status:** Behavior approved; written design awaiting final review
**Scope:** Native Files client interaction state only
**Selected approach:** Follow-first Locations Rail navigation using the existing bounded root I/O lane

## 1. Decision

The Files Locations Rail will obey the same interaction law as the Miller Trail while remaining a distinct root owner:

- `Up` and `Down` move the keyboard cursor by exactly one actionable item within the focused owner.
- Cursor movement may prepare the next view asynchronously, but never transfers keyboard focus by itself.
- `Right` or `Enter` explicitly crosses one hierarchy edge and immediately highlights the first actionable entry in the destination column.
- `Left` crosses exactly one parent edge. From the first Trail column, one more `Left` transfers focus to the Locations Rail.

For the Locations Rail, follow movement prepares an entire root `FmState`. For a Miller column, follow movement prepares the selected directory's child branch. The underlying operation differs, but movement, preview, activation, focus, stale-result, and failure semantics are intentionally the same.

This design tests Follow mode first. It does not add a manual-mode setting. A configuration choice will be considered only if isolated measurements and user testing show that the bounded asynchronous design still harms responsiveness or usability.

## 2. User Experience Contract

### 2.1 Focus owners

Files has two keyboard ownership domains:

1. **Locations Rail** — Home, Desktop, Downloads, and other accessible location rows.
2. **Miller Trail** — the root column and any visible descendant columns.

Exactly one domain owns directional keyboard input at a time. Showing a preview never silently changes that owner.

### 2.2 Locations Rail behavior

When the Rail owns focus:

- `Up` and `Down` move by one accessible location row.
- Headers, separators, blank rows, and inaccessible locations are not cursor targets.
- Movement clamps at the first and last actionable row; a clamped key is inert.
- Landing on a different location queues a bounded asynchronous root preview.
- Until that preview succeeds, the previous Trail remains visible and usable as the last accepted state.
- When the preview succeeds, the Trail displays the selected root, but no Trail row is highlighted because focus remains in the Rail.
- `Right` or `Enter` transfers focus into that root and immediately highlights its first actionable entry.
- `Left` at the Rail boundary is inert.

When the accepted origin is an exact accessible Rail location, the Rail cursor initializes to that exact origin when focus returns from the first Trail column. For example, after entering Downloads and navigating its descendants, repeated `Left` reaches the Downloads root and the next `Left` highlights Downloads in the Rail.

A `Direct(path)` origin may not exist in the Locations model. `Left` must still transfer focus to the Rail, but it must not infer an ancestor location or display a false origin marker. It preserves the last valid Rail cursor when one exists; otherwise it chooses the first accessible location deterministically. If the model has no accessible location, Rail focus is still valid with no cursor and its entry-dependent keys are inert.

### 2.3 Miller Trail behavior

When a Trail column owns focus:

- `Up` and `Down` move by one entry within that same column.
- Landing on a directory may queue its child preview asynchronously; focus remains in the current column.
- Landing on a file does not enter another column.
- `Right` on a directory transfers focus to its child column and immediately highlights that child's first actionable entry.
- `Right` on a file is inert.
- `Left` transfers focus to the immediate parent column without changing the parent's selected row.
- `Left` from the root column transfers focus to the Rail and applies the exact-Location or non-inferred Direct-origin cursor rule defined above.

If the first destination entry is itself a directory, Herdr may queue that entry's child preview through the existing bounded preview lane. This preparation must not move focus beyond the newly entered destination column.

### 2.4 First-entry rule

An explicit `Right` or `Enter` transition always initializes the destination to its first actionable entry under the existing sort, filter, and visibility rules. It does not preserve a hidden historical cursor as the active highlight.

- A non-empty destination highlights entry zero in its actionable sequence.
- The next `Down` highlights entry one, not entry zero again.
- An empty or failed destination has no synthetic row and no highlight.
- Further vertical or horizontal navigation that requires an entry is inert until a real entry exists.

### 2.5 Mouse compatibility

Existing mouse activation and selection semantics remain intact. A successful mouse activation of a Locations row must synchronize the Rail cursor and accepted location origin so that later keyboard navigation starts from what the user can see. Direct-path activation retains a `Direct(path)` origin and never invents a matching Rail row. This work must not reintroduce multi-row wheel movement or cross-column vertical focus transfer.

## 3. Current Architecture and Reuse Boundary

The current Miller path already separates cursor movement from asynchronous preparation:

1. `FmState::move_trail_cursor_in_column` changes the cursor and installs the resident projection.
2. `handle_file_manager_key` derives `FileManagerKeyDispatch::PreviewDirectory` for a directory without transferring focus.
3. `App::handle_focused_file_manager_key` calls `queue_file_manager_trail_directory_preview_identity`.
4. `queue_file_manager_trail_directory_request(..., preview_only = true)` submits `FileManagerIoRequest::TrailPreview`.
5. `sync_file_manager_io_results` applies a completion only when generation, source column, source index, source path, and current cursor still agree.

The Locations root path already has the required asynchronous lane:

1. `App::sync_file_manager_location_request` consumes the requested root navigation.
2. A resident root can take a zero-worker fast path.
3. A non-resident root submits `FileManagerIoRequest::Root` to the bounded worker.
4. Root completion is accepted only for the exact pending root identity and live Files model.

The selected design extends these existing seams. It does not create a second worker, timer, debounce scheduler, or cache policy.

## 4. State and Authority Model

Locations needs three separate concepts:

| Concept | Meaning | Authority rule |
|---|---|---|
| `cursor` | The Rail row currently targeted by keyboard movement | Changes synchronously on a valid one-row step |
| `origin` | The root represented by the last successfully accepted Trail state | Changes only after a resident activation or accepted root completion |
| `pending` | Exact identity of an in-flight root request and its local completion intent | Cleared, replaced, promoted, failed, or accepted only by exact identity |

These values must not be collapsed. While Downloads is pending and Home is still resident, the valid state is:

```text
focus = Rail
cursor = Downloads
origin = Home
Trail = accepted Home state
pending = Downloads root request
```

The Rail cursor is a client-local presentation/interaction fact. It does not belong in the server protocol, private socket protocol, or shared runtime model.

### 4.1 Pending identity

A root request must carry or be matched against enough identity to reject obsolete work:

- Files open/model generation,
- root I/O generation,
- requested path,
- location/model revision or equivalent exact location identity,
- current Rail cursor,
- active request lifecycle state.

The completion intent is local mutable authority associated with that exact pending identity:

- `FollowPreview` — install the root if still current, retain Rail focus, and show no Trail highlight.
- `EnterTrail` — install the root if still current, then transfer focus to the Trail and initialize its first actionable entry.

Intent promotion must not weaken the request identity checks.

### 4.2 Invariants

The implementation must preserve these invariants:

1. Input and render paths perform zero filesystem enumeration.
2. `origin` always describes the accepted Trail; `cursor` may temporarily describe a different pending location.
3. A completion can apply only to the exact current pending identity and current cursor target.
4. Preview never transfers focus.
5. Explicit activation crosses at most one owner/column edge.
6. A visible destination highlight exists only for a real actionable entry.
7. Failure preserves the last accepted Trail and origin.
8. Render remains a pure projection of `AppState`; it does not mutate navigation state or start I/O.

## 5. Interaction Flows

### 5.1 Rail follow preview

```text
Rail Up/Down
  -> move cursor one actionable location
  -> keep focus in Rail
  -> resident root available?
       yes: install accepted root without worker I/O
       no: submit/replace bounded Root request
  -> accepted result updates origin + Trail
  -> Trail highlight remains visually inactive while Rail owns focus
```

A clamped move, repeated selection of the same accepted location, or request replacement that changes no visible state must not trigger an unnecessary render.

### 5.2 Right or Enter from Rail

```text
Rail Right/Enter
  -> target already resident/prepared?
       yes: transfer to Trail with zero filesystem I/O
            initialize first actionable root entry
       no, exact root request already pending:
            promote pending intent to EnterTrail
            do not submit a duplicate request
       no request pending:
            submit exact Root request with EnterTrail intent
  -> on accepted completion, transfer exactly once
  -> if first entry is a directory, optionally queue its child preview
  -> remain focused in the root column
```

If the user moves the Rail cursor, leaves Files, changes focus, or changes the Files model before completion, the old activation intent cannot transfer focus later.

### 5.3 Right from a Miller directory

```text
Trail Right on directory
  -> activate the already prepared/resident child when valid
  -> transfer exactly one column right
  -> initialize first actionable child entry
  -> optionally queue a preview for that entry if it is a directory
  -> keep focus in the newly entered child column
```

The same first-entry helper or state transition law should serve root entry and child entry so the two owners cannot drift semantically.

### 5.4 Left from the Trail root

```text
Trail Left at root
  -> focus Rail
  -> Location origin: Rail cursor = exact accepted origin
  -> Direct origin: preserve valid Rail cursor, otherwise first accessible row or None
  -> Trail and origin remain unchanged
  -> no filesystem I/O
```

## 6. Bounded I/O, Stale Results, and Failures

### 6.1 Queue bound

The existing latest-wins worker contract remains the hard limit: at most one request is running and one latest request is pending. Rapid Rail movement replaces obsolete pending work instead of building an unbounded queue.

For a controlled 100-move burst while the first enumeration is blocked, the structural expectation is at most two enumerations: the already-running first request and the final retained request. Intermediate paths must not all reach disk.

### 6.2 Stale rejection

A root completion is rejected without visible mutation when any required authority has changed, including:

- Rail cursor moved to another location,
- a newer root request replaced it,
- focus or explicit activation lifecycle invalidated its intent,
- Files closed and reopened,
- model or generation changed,
- the path changed type or no longer resolves to the requested directory,
- the response belongs to an old worker lifecycle.

Stale rejection must not change focus, origin, Trail, cursor, failure UI, or render counters except for diagnostics that are not themselves rendered.

### 6.3 Failure behavior

Missing paths, permission errors, directory-to-file changes, worker panic, and channel disconnect are typed failures. On failure:

- retain the previous accepted Trail and origin,
- retain Rail focus and the user's cursor target,
- expose a bounded, location-specific failure state,
- do not fabricate an empty successful directory,
- do not transfer focus because of a failed `EnterTrail` intent,
- clear or retire the exact failed pending identity,
- leave the worker lane reusable for the next request.

Moving to another Rail target retires a previous target's failure presentation. A failure can render only when its exact path and model authority still correspond to the current Rail cursor.

An empty directory is a successful root with zero entries, not an I/O failure. It displays no highlight and all entry-dependent navigation is inert.

## 7. Rendering, Compact Layout, and Accessibility

The renderer must visually distinguish:

- the strong focused Rail cursor,
- the subdued accepted-origin marker when it differs from the cursor,
- a pending or failed cursor target without implying that the Trail already represents it,
- an active Trail row only when the Trail owns focus.

The distinction must survive ANSI/no-color rendering. Use existing semantic style primitives such as focus reverse/bold versus origin accent/bold; do not rely on color alone. Styling must not change cell width, row height, column geometry, or hit targets.

In compact layouts, the Locations drawer is the Rail owner:

- `Left` from the root opens/focuses the drawer using the same exact-Location or non-inferred Direct-origin cursor rule as the wide Rail.
- Follow movement uses the same root request lifecycle.
- `Right` or `Enter` closes the drawer only when transferring into a resident or successfully completed root.
- `Esc` or outside-click restores the existing compact interaction contract and invalidates any focus-after-completion intent that can no longer apply.

No state mutation, I/O start, or cursor correction may occur inside `render(&AppState)`.

## 8. Performance Study and Decision Gate

Follow mode is accepted for implementation only with measurement before and after the behavior change. The study will use isolated throwaway XDG state and the debug `herdr-dev` runtime; it will never connect to or stop the stable Herdr server.

### 8.1 Fixtures

Measure at least:

- a small representative directory,
- a synthetic directory with approximately 10,000 entries,
- a synthetic directory with approximately 100,000 entries,
- an inaccessible or removed directory,
- rapid switching among Home, Desktop, and Downloads.

Fixture roots must be temporary, bounded, and removed by the isolated test harness.

### 8.2 Observations

Record:

- key-dispatch latency (`p50`, `p95`, and maximum),
- directory enumeration latency (`p50`, `p95`, and maximum),
- submitted, replaced, processed, accepted, failed, and stale-rejected request counts,
- time until the final cursor target settles,
- loop-tick continuity while a worker read is blocked,
- render attempts and full renders during a burst,
- settled memory after repeated switching.

### 8.3 Hard gates

The following are structural release gates, independent of machine speed:

- zero filesystem I/O on the input path,
- one running plus one latest pending request at most,
- no more than first and final enumeration in the controlled blocked 100-event burst,
- zero stale completion applies,
- visible changes, not raw input count, determine render attempts,
- a failed or panicked request does not poison the lane,
- final accepted Trail, origin, cursor, and focus agree with the final user intent.

The study will report observed timings rather than inventing an arbitrary wall-clock CI threshold before measurement. After isolated automated evidence and the user's live test, the team will decide whether Follow remains the sole behavior or whether a manual/configurable mode has demonstrated value.

## 9. Test-First Verification Matrix

Production code may begin only after the relevant characterization and failing behavior tests exist.

| ID | Test point | Expected result | Why it is protected |
|---|---|---|---|
| `TP-FLF-CHAR-01` | Characterize current Miller cursor preview | Directory cursor movement queues preview while focus remains in source column | Locks the interaction law being extended |
| `TP-FLF-FOCUS-01` | `Left` from root to Rail | Rail receives focus with exact Location origin; Direct origin uses deterministic non-inferred fallback; no I/O | Prevents both the frozen-column UX and false ancestor authority |
| `TP-FLF-STEP-01` | Rail vertical movement | Exactly one accessible location per key; structural rows skipped; boundaries inert | Prevents multi-row jumps and invalid targets |
| `TP-FLF-PREVIEW-01` | Rail follow request | Cursor change queues asynchronous root preview and Rail retains focus | Separates movement from activation |
| `TP-FLF-NO-HIGHLIGHT-01` | Accepted follow preview | New root is visible with no active Trail row highlight while Rail is focused | Makes ownership visually truthful |
| `TP-FLF-ENTER-01` | Rail `Right`/`Enter` | Trail receives focus and its first actionable root entry is highlighted immediately | Removes the extra-Down defect |
| `TP-FLF-CHILD-01` | Miller directory `Right` | Child column receives focus and first actionable child is highlighted immediately | Keeps root and child transitions identical |
| `TP-FLF-SECOND-01` | First `Down` after entry | Selection advances from first entry to second entry | Prevents duplicate first-step behavior |
| `TP-FLF-BOUNDED-01` | 100 rapid Rail moves with blocked first read | Maximum one running plus one latest pending; at most first and final enumeration | Proves queue and disk work are bounded |
| `TP-FLF-LATEST-01` | Out-of-order/latest-wins completion | Only the final cursor path can update origin and Trail | Prevents obsolete views flashing in |
| `TP-FLF-BLOCKED-01` | Worker blocked for 500 ms | Cursor, input processing, and render loop remain responsive | Proves disk latency is outside input loop |
| `TP-FLF-RESIDENT-01` | Resident root follow/entry | Accepted transition performs zero worker reads | Protects the fast path |
| `TP-FLF-FAIL-01` | Missing, changed-type, and permission failures | Old Trail/origin remain; failure is visible; cursor/focus remain Rail | Makes failure non-destructive and truthful |
| `TP-FLF-PANIC-01` | Worker panic or disconnect | Typed failure is produced and a later request succeeds on a reusable lane | Covers lifecycle failure, not only I/O errors |
| `TP-FLF-STALE-01` | Cursor, focus, model, close/reopen invalidation | Every obsolete completion is rejected without focus or view mutation | Protects exact authority under races |
| `TP-FLF-EMPTY-01` | Empty or failed destination | Destination Trail cursor is `None`; no fake row is highlighted; failure retains the Rail target cursor | Protects boundary correctness without contradicting failure authority |
| `TP-FLF-RENDER-01` | Render neutrality | Accepted visible cursor/completion renders; clamped, replaced, and stale events do not | Prevents a new render storm |
| `TP-FLF-COMPACT-01` | Compact Locations drawer lifecycle | Same cursor, request, promotion, failure, and focus semantics as wide Rail | Prevents responsive-layout divergence |
| `TP-FLF-VIS-01` | Cell-level visual fixture | Rail cursor and accepted origin are distinguishable with color and no-color styles and no geometry drift | Makes split state understandable and accessible |
| `TP-FLF-E2E-01` | Isolated live rapid navigation | Repeated Home/Desktop/Downloads follow plus Trail entry has no freeze, wrong-root apply, or implicit focus jump | Validates the complete runtime path |

The automated sequence is RED, GREEN, REFACTOR. Narrow tests run first; then the full project gate is `just check`, including nextest and maintenance checks. Platform lint remains part of the established final gate where the branch workflow requires it.

## 10. Likely Ownership Surfaces

Implementation planning may refine exact symbols, but ownership should remain within these existing client surfaces:

- `fm/` state for Rail cursor, origin, request intent, and Trail destination initialization,
- `app/input/file_manager.rs` for key-to-semantic-dispatch mapping,
- Files app actions for request submission, intent promotion, and accepted-result transitions,
- the existing file-manager I/O worker for bounded Root requests,
- `ui/` Files rendering for pure focus/cursor/origin projection,
- nearby unit, worker, render-cell, and isolated runtime tests.

No server protocol, shared runtime state, platform API, terminal parser, or vendored libghostty-vt change is expected.

## 11. Non-Goals

This change does not add:

- a manual navigation mode or config toggle,
- a new cache, unbounded LRU, pinned-location pre-warm policy, or dependency,
- a second root worker, debounce timer, or general scheduler,
- background recursive enumeration,
- protocol or socket fields,
- new layout geometry,
- changes to stable Herdr runtime/configuration,
- unrelated mouse, preview-format, or filesystem feature work.

Pinned Home/Desktop/Downloads pre-warming remains a separate measurement-first proposal. It is not required for Follow mode because the bounded root worker keeps filesystem latency outside the input loop.

## 12. Alternatives Considered

### 12.1 Selected: extend the existing Miller interaction law

Reuse the current bounded Root worker and exact stale-result authority while adding a distinct Rail cursor and explicit completion intent. This has the smallest lifecycle surface and makes root and child navigation predictable.

### 12.2 Rejected: separate Locations worker or debounce scheduler

A second lane would duplicate cancellation, priority, failure, teardown, observability, and stale-authority logic. Time-based debounce would also make behavior machine-dependent and could still submit obsolete work at the wrong boundary.

### 12.3 Rejected: synchronous root switching on every Rail step

This would put directory enumeration back on the input path and recreate the freeze class already removed from file selection and preview handling.

### 12.4 Deferred: manual mode or user configuration

Providing both modes before measuring Follow would add settings, documentation, state combinations, and test burden without evidence that users need the alternative. The decision is deliberately deferred until automated performance evidence and live user testing exist.

## 13. Yazi Architecture Transfer

The transferable Yazi principle is the separation of movement, speculative preparation, and explicit activation:

- movement updates a cheap cursor,
- preparation is asynchronous, bounded, and replaceable,
- activation alone transfers navigation ownership,
- obsolete results have no authority.

Herdr should transfer this interaction and authority law, not assume Yazi's cache topology or import an unbounded cache into a long-lived agent runtime.

## 14. Delivery and Safety Gates

Implementation is complete only when:

1. all 20 `TP-FLF-*` points have fresh evidence,
2. structural performance gates pass,
3. `just check` and the established platform lint gate are clean,
4. the isolated live test confirms no freeze or implicit focus jump,
5. stable Herdr, its socket, and `~/.config/herdr` were never targeted,
6. user-owned `.superpowers/` remains untouched and unstaged,
7. product code, tests, docs, and evidence are staged by exact path in atomic commits,
8. commit messages are aligned before commit and publication stays on the CyPack fork.

The next artifact after this design is an implementation plan produced with the project planning workflow. Rust edits are out of scope until this written design is reviewed and approved.
