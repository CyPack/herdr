# FM1 — Horizontal Miller Viewport (progress)

Plan contract:
`docs/superpowers/plans/2026-07-15-herdr-file-manager-post-shell-implementation.md`
"FM1". Predecessors: Shell Foundation closed in scoped form at `887471c2`
(deferred SF6.3 items recorded OPEN in the SF6 evidence). Target chain: FM1
scrollable horizontal viewport -> FM2 column edge drag-resize (the user's
custom-layout target interaction).

## FM1.1 — Bounded chain/cache RED contracts (CLOSED)

- RED `5e8616e0` (`test: define bounded Miller history and cache
  contracts`): the six plan tests landed compile-valid against the frozen
  interface set in the new `src/fm/miller.rs` (constants 32/5/16/28/64,
  `MillerColumnId`/`MillerPathSegment`/`MillerDirectoryProjection`/
  `MillerHorizontalViewport`/`MillerState` +
  `assert_miller_invariants_for_test`). The RED commit carried explicit
  "RED stub" bodies for the three bound-enforcement points (seed truncate,
  chain trim, cache eviction), so the bound/lifecycle assertions failed at
  RUNTIME exactly as the plan requires (observed: history-32, focused-drop,
  cache-5, eviction-generation rows failing; no compile/setup failure).

## FM1.2 — Logical chain and resident projections (CLOSED)

- GREEN `68ded90b` (`feat: add bounded Miller history projections`):
  - `MillerState::seed(cwd)` — nearest <=32 ancestors plus the cwd,
    root->child chain order, NO canonicalization through missing
    components (pure path arithmetic).
  - `MillerState::visit(directory, previous_current)` — chain trims from
    the ROOT side (never the focused tail), the previous current
    projection moves into the cache by ownership transfer at the call
    boundary, the new current is always removed from the cache, and LRU
    eviction (front = least recent visible/focused transition) keeps at
    most FOUR non-current projections. Generation allocation via
    `next_column_id`; `resident_projection` resolves ONLY the exact
    current identity — evicted generations resolve to nothing.
  - Module-level `#![allow(dead_code)]` with a named consumption
    condition (FmState seeding/visits + FM1.3 viewport), per the SF4.1
    precedent.
- Gates: miller family 13/13; full Nextest `--no-fail-fast` 3,337/3,337
  plus only the named B0 skip; fmt; Linux all-target and Windows MSVC bin
  Clippy with `-D warnings`; diff and added-production-`unwrap()` clean.
- Publication: FF pushes to CyPack only; both refs equal exact SHA
  `68ded90b86e9a32d0885332ae617d6474ea01d99`; `upstream` untouched.

- Integration `97710337` (`feat: seed and visit miller state through
  navigation`): `FmState.miller` seeded in `with_hidden`/`test_empty`;
  `enter()`/`leave()` route through one `departing_projection()` seam that
  MOVES the departing `entries` vector (ownership transfer, no clone)
  under a fresh `next_column_id` and then `visit`s the new cwd. The
  integration test drives a REAL temp tree end-to-end: the departed
  projection lands in the cache carrying the moved entries, returning to a
  directory removes it from the evictable cache, and the invariant checker
  holds after every transition. Miller family 14/14; full suite
  3,338/3,338; both Clippy targets clean. Both refs equal
  `977103371055dd4782bfefd07f10851bcabf6052`.
- Note (recorded, FM1.3/FM4 scope): typed unavailable-ancestor projections
  and the growing-chain traversal refinement land with the viewport and
  FM4 navigation slices.

## Exact Next Microtask

FM1.3:
`src/ui/file_manager/miller.rs` horizontal viewport geometry (<=5 complete
disjoint columns + dividers inside the Stage at widths
0/15/16/31/32/56/84/140/400, focused column visible) + scroll REDs
(`horizontal_viewport_clamps_after_path_shrink`, `..._terminal_resize`,
`horizontal_scroll_changes_only_miller_window`,
`vertical_wheel_does_not_pan_horizontal_window`,
`zero_area_clears_column_and_divider_hits`,
`render_uses_precomputed_projections_without_filesystem_reads`) +
ScrollLeft/Right + Shift+wheel + header arrows. Then the FM1 verification
command (`test(/(miller|file_manager|shell.*scroll)/)`) and phase
publication, then FM2.1-2.2 (SF3 transaction with column targets — the
drag-resize target).

## FM1.3 — Viewport geometry core (CLOSED at model level; render/scroll wiring NEXT)

- `0176e394` + clippy fix `3e8a50d0` (`feat: add horizontal miller viewport
  geometry`): pure `src/ui/file_manager/miller.rs` —
  `miller_viewport_geometry(stage, preferred_widths, focused, requested_first)`
  lays out <=5 COMPLETE columns (each clamped 16..=64) plus one-cell
  divider rects, clamps the window origin via a backward-greedy
  `first_visible_floor` so the focused column ALWAYS stays visible, and
  returns the clamped `first_visible` for callers to persist. The test loop
  covers the nine plan widths (0/15/16/31/32/56/84/140/400): bounded
  non-empty counts, complete/disjoint/in-stage rects, focused visibility;
  plus path-shrink clamp, terminal-resize clamp (focused wins the narrow
  window), scroll-changes-only-window, and zero-area inertness. The test
  loop CAUGHT two real algorithm bugs mid-slice (min-width capacity
  overcount hiding the focused column; end-of-chain window semantics) —
  fixed before publication. NOTE: an intermediate head `0176e394` was
  briefly published with a failing clippy gate (a pipe swallowed the exit
  code); the very next commit `3e8a50d0` restored the gate and both
  targets are clean — recorded honestly, and gate commands now echo exit
  codes explicitly.

## FM2.1 — Column resize core (CLOSED at model level)

- `d1002cac` (`feat: commit miller column widths through clamped seam`):
  `MillerState::commit_column_width(chain_index, width)` — the SINGLE
  write-back seam for divider resize commits: clamps to the frozen 16..=64
  bounds, bumps the revision (geometry recomputes), refuses stale chain
  indices. Combined with the proven region-generic SF3 `ResizeTransaction`
  (clamping + stale-generation inertness pinned at `d031ef26`) and the
  FM1.3 divider rects, the drag-resize MODEL chain is complete:
  divider rect -> transaction preview/commit -> clamped width write-back
  -> next-frame geometry. Full suite 3,344/3,344; both Clippy targets
  exit-code-verified clean. Both refs equal
  `d1002cacb8f2b6eb730d0a9ab6217cff9ac7f6a9`.

## Exact Next Microtask

FM1.3/FM2.2 WIRING (the remaining gap to the user-visible custom layout):
(1) render the Miller chain through `miller_viewport_geometry` in
`render_file_manager` (replace the fixed parent/current/preview trio with
the windowed chain; unavailable ancestors render explicitly), storing
column/divider rects + generations in `ViewState`; (2) horizontal scroll
input (ScrollLeft/Right, Shift+wheel, header arrows) mutating ONLY
`MillerState.horizontal.first_visible`; (3) divider mouse capture: press
on a `MillerDividerRect` begins an SF3 `ResizeTransaction` over
`[left_width, right_width]` with bounds 16..=64 (SF4.2 capture owns the
gesture), preview updates transient widths, release commits through
`commit_column_width`; (4) FM3 generation-checked `MillerRowTarget`
routing. Then the FM1/FM2 verification commands and phase publication.

## FM2.2 — Divider drag-resize LANDED (user-visible, end-to-end)

- `b1c4aec2` (`feat: drag miller trio dividers to resize columns`): pressing
  a Miller divider begins a capture that owns move/up EVERYWHERE (the
  SF4.2-04 principle — the E2E test CAUGHT the in-center gate swallowing a
  fast drag and forced the fix), each drag commits the clamped 16..=64
  width through the single `FmState::commit_trio_width` seam (mirrored into
  the FM1 chain model via `commit_column_width`), release ends the capture,
  and Files close clears it. ONE geometry authority: compute
  (`sync_file_manager_view`), render (`render_file_manager` + helpers),
  hit-testing, and the kitty image preview all consume the SAME
  `MillerTrioOverrides` through `_with` seams; default-override twins keep
  every legacy caller byte-identical (overrides None until the user drags).
- E2E test `divider_drag_resizes_trio_columns_end_to_end`: press ->
  capture, drag -> clamped commit, recompute honors the width, release ->
  capture ends, runaway drag clamps to 64. Full suite 3,345/3,345; both
  Clippy targets exit-code-verified clean; fmt/diff/unwrap clean. Both refs
  equal `b1c4aec2e034651ad3ceb8d74f2e4aa02426c4fa`.
- Recorded deviations/candidates: (1) drag commits live per move (no
  preview/commit split yet — client-local widths, zero persistence/PTY, so
  the SF3 purity contract is not violated; unify onto `ResizeTransaction`
  with a Miller `DividerId` extension at FM2 closure); (2) the horizontal
  GROWING-chain window render (FM1.3 full: >3 columns + ScrollLeft/Right/
  Shift+wheel over the chain) remains the next slice — the geometry
  (`miller_viewport_geometry`) and model are ready.

## Exact Next Microtask

FM1.3 chain-window render: replace the trio with the windowed chain render
consuming `miller_viewport_geometry` (columns from resident projections +
current FmState; unavailable ancestors explicit), horizontal scroll input
(ScrollLeft/Right, Shift+wheel, header arrows -> `horizontal.first_visible`
only), then FM3 generation-checked `MillerRowTarget` routing and the FM1/FM2
verification commands + phase publication.
