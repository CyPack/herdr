# SF4.3 — Cross-Layer Surface Projection and Render Purity (progress)

Plan contract: `docs/superpowers/plans/2026-07-15-herdr-shell-foundation-v0-implementation.md`
"Task SF4.3". Predecessor microphase SF4.2 closed 8/8 at `20f659c1`
(evidence: `shell-foundation-sf4-input-router-progress.md`).

## Slice contracts (frozen test-point catalog)

| ID | Test | Expected result | Reason | Status |
|---|---|---|---|---|
| SF4.3-01 | `active_surface_alone_populates_stage_hits` | Exactly one stage surface owns projected hit geometry per frame (terminal pane/split only under `TerminalWorkspace`; Files geometry only under `NativeFiles`) | Hidden surfaces must not project hit rectangles or side effects | GREEN |
| SF4.3-02 | `hidden_surface_has_no_stale_hits_or_cursor` | Switching surfaces leaves no stale view artifacts (rects/cursor) from the hidden surface | Stale geometry = invisible interactive surface | GREEN |
| SF4.3-03 | `surface_render_is_deterministic_for_identical_state` | Identical state renders byte-identical buffers | Render purity contract | GREEN (characterization) |
| SF4.3-04 | `surface_render_does_not_mutate_app_state` | Render leaves `AppState` bit-identical | "Never mutate state during render" | GREEN (characterization) |
| SF4.3-05 | `terminal_dirty_row_keeps_retained_path_with_static_shell` | A dirty terminal row under a static shell keeps the cached `ShellView` generation (cheap path) | Every PTY row must not re-solve the shell | GREEN (characterization) |
| SF4.3-06 | `stage_renderer_follows_typed_surface_authority` + typed renderer selection | `BaseLayer` chooses the stage renderer from `stage.surface_view()`; `Compositor` stays pure ordered layers | Plan bullet 2-3 | GREEN |

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

## SF4.3-03/04 Evidence (characterizations — valid RED refuted with source)

- Source sweep BEFORE writing: the stage surface render path
  (`render_panes`/`render_file_manager` through the real `Compositor`)
  reads no clock and no randomness, and `render` takes `&AppState` so
  direct mutation is compile-impossible; the residual hazard is interior
  mutability via the runtime registry. One `test:` commit `08d73676`
  (`test: freeze stage surface render purity`) freezes both rows for BOTH
  surfaces: byte-identical double-draw buffers (03) and an observable
  state snapshot equal across a render (04). Exact 2/2 (run `25892970`);
  full suite 3,313/3,313.
- NOTED exception (recorded, outside the stage-surface scope): the
  sidebar Projects tab's `render_projects_list` reads
  `SystemTime::now()` for relative timestamps (`src/ui/sidebar.rs:1481`)
  inside a fn documented as pure render. Candidate follow-up: feed a
  clock through state; not an SF4.3 catalog row.

## SF4.3-05 Evidence (characterization)

- Valid RED refuted by source: the cached `ShellView` returns the
  previous projection on an unchanged geometry key (SF2.4). Commit
  `1f57ccbb` (`test: freeze retained shell path for dirty rows`) pins the
  retained path END-TO-END through `compute_view`: three identical-
  geometry recomputes keep the exact cached generation and regions, and a
  control geometry change advances the generation exactly once. Exact 1/1
  (run `77441e24`); full suite 3,314/3,314.

## SF4.3-06 Atomic TDD Evidence (typed renderer authority)

- RED `a9b67112` (`test: require typed stage authority for renderer
  choice`): the adversarial divergent state (Files domain state present
  while the typed stage says TerminalWorkspace) rendered the Files
  surface — the legacy `file_manager.is_some()` boolean chose the
  renderer. The aligned control proves the Files surface renders when the
  stage owns it.
- GREEN `f973740e` (`feat: choose stage renderer from typed surface
  authority`): `BaseLayer` now matches `app.stage.surface_view()` to pick
  `render_file_manager` versus `render_panes`; the `Compositor` stays
  pure ordered layers. The full `--no-fail-fast` sweep bounded the blast
  radius to exactly SEVEN test fixtures that set `app.file_manager`
  directly without the stage transaction (an unreachable divergent state
  in production since SF4.1); all were migrated onto
  `try_open_file_manager_with` with expectations unchanged.
- Gates: exact 1/1 (run `cc66b8df`); full Nextest `--no-fail-fast`
  3,315/3,315 plus only the named B0 skip; fmt, Linux all-target and
  Windows MSVC bin Clippy clean with `-D warnings`; diff and
  added-production-`unwrap()` clean.
- Publication: FF pushes to CyPack only; both refs equal exact SHA
  `f973740e2bda45c0cc39d8c9afd1061d8b47761d`; `upstream` untouched.

## SF4.3 MICROPHASE CLOSED — catalog 6/6 GREEN

Chain: 01 `7796d855`/`acc82ffd` surface-exclusive hit geometry ·
02 `bb5a6899`/`1bc69cf5` stale projection retirement ·
03+04 `08d73676` render purity characterizations ·
05 `1f57ccbb` retained-path characterization ·
06 `a9b67112`/`f973740e` typed renderer authority.
Closure gate at head `f973740e`: Rust 3,315/3,315 + B0 skip
(`--no-fail-fast`), Bun 5/5 + 12/12, Python 64/64, both Clippy targets,
fmt/diff/unwrap clean. Test inventory grew 3,309 -> 3,315.

Noted for later phases: sidebar Projects-tab `SystemTime::now()` render
read (above); input-side FM authority still keys off
`file_manager.is_some()` and migrates with SF6; SF4.2's recorded SF4.3
candidates (direct-exit `leave_modal` normalization, in-dispatch wheel
arms, FM double-click across overlay episodes, semantic `TopmostHit`
consumption in dispatch) remain candidates for SF4 closure review or SF6.

## Exact Next Microtask

SF4.3 is closed. SF4 has no further planned microphase in the
implementation plan (SF4.1/SF4.2/SF4.3 all closed) — run the SF4 closure
review against the plan's "Phase Completion and Publication" section,
then proceed per the frozen priority chain to SF5.1 (Dock model,
geometry, and pure render). Change-pipeline T3.1 stays paused.
