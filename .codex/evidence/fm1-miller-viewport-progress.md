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

## FM1.2 — Logical chain and resident projections (CORE CLOSED; App integration NEXT)

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

## Exact Next Microtask

FM1.2 App integration: `FmState` gains a `miller: MillerState` seeded in
`FmState::new(cwd)`; `enter()`/`leave()` call `visit` with the departing
current projection built by MOVING the old `entries` vector (ownership
transfer, no clone) under a fresh `next_column_id`; missing/inaccessible
ancestors keep a typed unavailable projection (plan bullet). Then FM1.3:
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
