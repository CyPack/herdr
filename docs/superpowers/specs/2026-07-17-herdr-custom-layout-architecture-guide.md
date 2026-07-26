# Herdr Custom Layout Architecture Guide

Target: the user-approved custom Files layout — a horizontally scrollable
Miller area whose columns can be narrowed/widened by dragging their edges —
delivered on an architecture that stays fast over remote (SSH) connections
and stays bounded when the layout holds many objects.

This guide is DERIVED, not invented: every contract below cites the frozen
program plans. Authority order on conflict: the plans win.

- Shell program: `docs/superpowers/plans/2026-07-15-herdr-shell-foundation-v0-implementation.md`
- FM program: `docs/superpowers/plans/2026-07-15-herdr-file-manager-post-shell-implementation.md`
- Design spec: `docs/superpowers/specs/2026-07-15-herdr-shell-foundation-v0-design.md`

## 1. The target experience (what "done" means)

1. Files owns the Workspace Stage as a typed surface (no terminal curtain).
2. The Miller area is a horizontal viewport over a growing path chain:
   entering a child APPENDS a column (Finder-like), the viewport scrolls
   left/right (wheel, Shift+wheel, header arrows), and the focused column is
   always visible (FM1 catalog).
3. Every visible column edge is a drag divider: press-drag narrows/widens the
   column between `MILLER_COLUMN_MIN_WIDTH = 16` and
   `MILLER_COLUMN_MAX_WIDTH = 64` cells (preferred 28), reusing the SF3
   shell resize transaction with column targets (FM2.1 — no new drag state).
4. Mouse works in EVERY visible column, not only the current one (FM3), with
   generation-checked authority: stale coordinates are consumed inert.

## 2. Why the layered chain (SF5 -> SF6 -> FM1 -> FM2) is the path

Each phase supplies one load-bearing property the target cannot skip:

| Phase | Property the target consumes |
|---|---|
| SF4 (closed) | One typed surface owns hit geometry per frame; hidden surfaces project nothing; stale generations resolve to nothing; render is pure and deterministic |
| SF5 AppDock | The stage gains a visible app switcher so Files-as-a-Stage is reachable/leavable without the sidebar; dock resize/collapse reuses the SF3 reducer (the same transaction family the Miller dividers will reuse) |
| SF6 Files Stage | Files render/lifecycle/input leave the legacy curtain; the Miller area becomes a first-class stage surface with its own geometry authority |
| FM1 | The bounded horizontal viewport (chain + resident cache + scroll) |
| FM2 | Column drag-resize on that viewport (the user's target interaction) |

## 3. Bounded-state model — how "many objects" stays managed

The layout NEVER holds unbounded live objects. Frozen bounds (FM plan,
"Frozen Interfaces and Bounds"):

- `MAX_MILLER_HISTORY_DEPTH = 32` path segments — a deep cwd seeds at most
  the nearest 32 ancestors; the logical chain is a `VecDeque` of cheap
  `MillerPathSegment`s (path + cursor + viewport + preferred width), NOT
  loaded directories.
- `MAX_RESIDENT_MILLER_COLUMNS = 5` complete directory projections resident
  at once; the active current column is never evicted; at most four unique
  non-current cached projections. Everything else re-loads on demand through
  the existing state-refresh path (never in render/input).
- Selection stays under the existing `MAX_MULTI_SELECTION_PATHS = 4096`
  atomic ceiling; the Stage keeps its 16-instance bound.

Precedent: this is the same bounded-authority pattern already proven by the
Stage (16 instances, generation exhaustion fail-closed) and bulk selection
(atomic 4,096 rejection). Growth changes NUMBERS, never the shape.

Identity and staleness: every column is a `MillerColumnId { directory,
generation }`; every hit target (`MillerRowTarget`, `MillerDividerTarget`)
carries BOTH the shell generation and the files generation plus the exact
entry path. The controller revalidates all of them before acting; any
mismatch returns `ConsumedStale` with zero mutation. This is the same
current-generation rule SF4.2-07 wired for shell hits
(`ShellView::region_hit_at`).

## 4. Remote-connection (SSH) performance architecture

Herdr renders server-side and ships frames to thin clients, so remote
performance = "how little work per event, how few changed cells per frame".
The architecture already enforces the right invariants; the Miller layout
must ride them, never bypass them:

1. **Retained shell path.** Unchanged geometry keys return the cached
   `ShellView` with the SAME generation — a dirty PTY row never re-solves
   the shell (frozen by `terminal_dirty_row_keeps_retained_path_with_static_shell`,
   SF4.3-05). Miller adds its own key: recompute column geometry only when
   `(stage area, chain revision, widths, first-visible)` changes; a
   keystroke that only moves a cursor must not re-lay-out five columns.
2. **Exactly one surface computes.** Hidden surfaces project no geometry and
   receive no resize side effects (SF4.3-01), so a background terminal costs
   ~zero while the user works in Files, and vice versa.
3. **Pure, deterministic render.** Identical state renders byte-identical
   buffers and mutates nothing (SF4.3-03/04) — the terminal diff layer then
   ships only genuinely changed cells over the wire. Corollary: NO clock, NO
   randomness, NO filesystem reads in Miller render (the noted
   `render_projects_list` `SystemTime::now()` exception is recorded as a
   cleanup candidate, not a precedent).
4. **O(visible) per frame, O(1) amortized per event.** Render cost scales
   with the 1-5 VISIBLE columns times their visible rows (viewport
   windows), never with chain depth (<=32) or directory size. Hit tests scan
   the bounded flattened target lists (same shape as `ShellView.hits`).
5. **Latest-value coalescing, not event queues.** Watcher/preview/progress
   updates coalesce into latest-value slots per generation (the C4.4
   `FileOperationWorkerProgress` pattern); a burst of filesystem events
   costs one refresh, not N.
6. **Drag preview stays local-transactional.** Divider drags reuse the SF3
   resize transaction: preview width lives in the transaction, panes are not
   re-resized during preview (`resize_panes_during_shell_preview`), and the
   commit lands once on release. Over SSH this means a drag ships small
   preview frames and exactly one committed relayout.
7. **Budgets are tested, not hoped.** SF6.3 and FM1.3/FM2.2 carry explicit
   render/queue/retained-PTY budget gates in the plans; a Miller frame that
   exceeds them fails the phase, exactly like a failing unit test.

## 5. Input ownership in the custom layout

The SF4.2 frozen precedence stays the single authority: topmost overlay ->
active capture -> z-ordered topmost hit -> focused component -> page ->
global -> fail-closed. Concretely for Miller:

- A divider drag is an ACTIVE CAPTURE: it owns move/up everywhere until
  release (SF4.2-04 characterization), so no column click can steal a
  half-finished resize.
- Column rows/dividers are TOPMOST HITS resolved only against the exact
  current generation (SF4.2-07); after any relayout the same pixel
  re-resolves against the new geometry.
- While Files owns the stage, the hidden terminal receives NOTHING
  (SF4.2-08 seal + SF4.3 exclusivity), and every overlay above Files blocks
  the Miller area completely (SF4.2-02/03).

## 6. Anti-patterns (rejected with evidence)

| Rejected | Why | Evidence |
|---|---|---|
| Unbounded visible column chain | Memory/latency multiplier; refuted by pinned Yazi/Joshuto/Ranger sources | N2.0 evidence, FM plan bounds |
| Per-column bespoke drag state | Duplicates SF3 reducer; two resize authorities drift | FM2.1 "do not add dock-specific drag state" precedent (SF5.2) |
| Rect-only hit authority (no generation) | Stale coordinates become authority after relayout | SF4.2-07, FM `ConsumedStale` contract |
| Filesystem or metadata reads in render/input | Blocks the render loop; kills SSH latency | C6.x invariants, SF4.3-03/04 |
| Arbitrary component registry for the layout | Over-engineering without a second consumer | P4.0 S5 NO-GO |

## 7. Status against the target (kept honest, update per phase)

| Step | State |
|---|---|
| SF4 typed surface + input + projection foundations | CLOSED (`f973740e`) |
| SF5.1 dock model/geometry/render | NEXT |
| SF5.2 dock interaction/popover | pending |
| SF6.1-6.3 Files-to-Stage migration | pending |
| FM1.1-1.3 horizontal Miller viewport (scrollable area) | pending |
| FM2.1-2.2 column edge drag-resize (the target interaction) | pending |
| FM3+ all-column mouse, growing navigation | pending |

The target is DONE when: an isolated dev build shows Files on the Stage
with a horizontally scrollable Miller chain whose visible column edges
drag-resize within 16..=64 cells under the SF3 transaction, all FM1/FM2
gates green, and the SSH-facing budgets in SF6.3/FM2.2 hold.
