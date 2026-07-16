# SF4.3 — Cross-Layer Surface Projection and Render Purity (progress)

Plan contract: `docs/superpowers/plans/2026-07-15-herdr-shell-foundation-v0-implementation.md`
"Task SF4.3". Predecessor microphase SF4.2 closed 8/8 at `20f659c1`
(evidence: `shell-foundation-sf4-input-router-progress.md`).

## Slice contracts (frozen test-point catalog)

| ID | Test | Expected result | Reason | Status |
|---|---|---|---|---|
| SF4.3-01 | `active_surface_alone_populates_stage_hits` | Exactly one stage surface owns projected hit geometry per frame (terminal pane/split only under `TerminalWorkspace`; Files geometry only under `NativeFiles`) | Hidden surfaces must not project hit rectangles or side effects | GREEN |
| SF4.3-02 | `hidden_surface_has_no_stale_hits_or_cursor` | Switching surfaces leaves no stale view artifacts (rects/cursor) from the hidden surface | Stale geometry = invisible interactive surface | GREEN |
| SF4.3-03 | `surface_render_is_deterministic_for_identical_state` | Identical state renders byte-identical buffers | Render purity contract | PENDING |
| SF4.3-04 | `surface_render_does_not_mutate_app_state` | Render leaves `AppState` bit-identical | "Never mutate state during render" | PENDING |
| SF4.3-05 | `terminal_dirty_row_keeps_retained_path_with_static_shell` | A dirty terminal row under a static shell keeps the cached `ShellView` generation (cheap path) | Every PTY row must not re-solve the shell | PENDING |
| SF4.3-06 | `compute_view_internal` structural split + `SurfaceHost` typed renderer selection (production refactor; parts landed in 01) | Shell projection separated from active-surface projection; `Compositor` stays pure ordered layers | Plan bullet 2-3 | PARTIAL (01 landed the surface gate; renderer selection pending) |

## Reconnaissance (recorded before the first RED)

- `compute_pane_infos` (`src/ui/panes.rs:316`) had NO surface guard: pane
  geometry AND `rt.resize` PTY side effects ran while the Files surface
  covered the stage. Files geometry was already `file_manager.is_some()`
  gated — the asymmetry was the genuine gap.
- SF4.1's typed `StageState::surface_view()` sat behind
  `#[allow(dead_code)]` with a named consumption condition; SF4.3-01 is
  that consumer (allow removed).
- The cached `ShellView` (geometry-key equality returns the previous view)
  is the existing retained-path foundation for SF4.3-05.
- The mobile projection (`compute_mobile_view`) computed the same
  unguarded pane/split geometry; the same gate was applied there for
  contract consistency.

## SF4.3-01 Atomic TDD Evidence

- RED `7796d855` (`test: require exclusive stage surface hit geometry`):
  failed exactly at "a hidden terminal surface must project no pane hit
  geometry" — with NativeFiles active, `view.pane_infos` was populated.
  Control rows proved the terminal surface projects pane+split geometry
  and that Files geometry appears when Files is active, so the slice
  cannot pass vacuously; a return row pins the same-frame restoration.
- GREEN `acc82ffd` (`feat: grant stage hit geometry to active surface
  only`): both desktop and mobile projections gate `split_borders` and
  `compute_pane_infos` behind
  `app.stage.surface_view() == StageSurfaceView::TerminalWorkspace`.
  Under NativeFiles the hidden terminal now projects NO pane/split
  rectangles and receives NO `rt.resize` side effects (they resume in the
  same frame as the first terminal recompute after close). The
  `#[allow(dead_code)]` on `surface_view()` was removed with its named
  consumption condition satisfied. Deliberately unchanged: the Files-side
  `file_manager.is_some()` guards (equivalent to NativeFiles under the
  SF4.1 transactional invariant), the AttachFile picker projection (its
  overlay state is independent of the stage), and
  `resize_background_tab_panes_*` (background lifecycle, not stage hit
  geometry).
- Gates: exact 1/1 (run `fc3c03dd`); full Nextest `--no-fail-fast`
  3,310/3,310 plus only the named B0 skip, ZERO regressions from the
  split (run `ff247ffd`); fmt; Linux all-target and Windows MSVC bin
  Clippy with `-D warnings`; diff and added-production-`unwrap()` clean.
- Publication: FF pushes to CyPack only; both refs equal exact SHA
  `acc82ffdd272885150278dc0ce941828e4db68cd`; `upstream` untouched.

## SF4.3-02 Atomic TDD Evidence

- RED-ability verified first: `try_open_file_manager_with` and
  `close_file_manager` cleared domain state but NOT view projections, so
  the window between a surface switch and the next compute carried stale
  hit rectangles for the hidden surface. Concrete hazard found during
  recon: the `[+]` agent-attachment pre-branch checks only the view rect
  at its INPUT site (its FM guard is compute-level), so a stale `[+]`
  rectangle in that window could open the picker while Files owns the
  stage.
- RED `bb5a6899` (`test: require stale projection retirement on surface
  switch`): failed exactly at "the switch itself must retire stale pane
  hit geometry" — after opening Files without an intervening compute,
  `view.pane_infos` still carried the prior terminal frame. Control rows
  prove both surfaces project geometry when computed; the close direction
  pins Files row/action/header/action-bar retirement. There is no
  dedicated view cursor field — terminal cursor placement derives from
  `pane_infos`, so its retirement is covered structurally.
- GREEN `1bc69cf5` (`feat: retire hidden surface projection on stage
  switch`): the open transaction retires `pane_infos`, `split_borders`,
  and both agent frame action areas in the same mutation that activates
  Files; the close transaction retires the Files row/row-action/header
  geometry and the action-bar model. Sidebar Files-tab rows are
  deliberately NOT retired (they belong to the sidebar tab, not the stage
  surface).
- Gates: exact 1/1 (run `681916e4`); full Nextest `--no-fail-fast`
  3,311/3,311 plus only the named B0 skip, zero regressions; fmt, Linux
  all-target and Windows MSVC bin Clippy clean with `-D warnings`; diff
  and added-production-`unwrap()` clean.
- Publication: FF pushes to CyPack only; both refs equal exact SHA
  `1bc69cf5d8f1118468d4a1adffd3b33956ebcf14`; `upstream` untouched.

## Exact Next Microtask

SF4.3-03: `surface_render_is_deterministic_for_identical_state` and
SF4.3-04: `surface_render_does_not_mutate_app_state` — verify RED-ability
first (render already takes `&AppState`; these are LIKELY
characterizations pinning purity, per the SF1 precedent — confirm by
source inspection of the render entry and any interior mutability before
writing). Then SF4.3-05 retained path, SF4.3-06 SurfaceHost typed
renderer selection, and the SF4.3 closure gate. Do not start SF5 before
SF4.3 closes.
