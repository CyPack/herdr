# SF5 — AppDock (progress)

Plan contract: `docs/superpowers/plans/2026-07-15-herdr-shell-foundation-v0-implementation.md`
"Task SF5.1" / "Task SF5.2". Predecessor SF4 fully closed at `f973740e`.
Program guide (local, gitignored):
`docs/superpowers/specs/2026-07-17-herdr-custom-layout-architecture-guide.md`.

## SF5.1 — Dock model, geometry, and pure render (CLOSED)

Seven-row plan catalog, one RED + one GREEN commit per the plan's naming.

- RED `64d5dd5e` (`test: define native app dock presentation`): compile-valid
  via the new-module skeleton pattern (typed API stubs with empty behavior in
  the same commit — SF4.1/C6.1 precedent); ran 7 tests with EXACTLY the four
  behavior rows failing at runtime (model projection, icon render, hit
  areas, narrow targets) and the three already-frozen rows passing as
  characterizations (template five-cell policy from SF2's `dock_track()`,
  solver dock-collapses-before-stage ladder from `degrade_workspace_requests`,
  degenerate inertness). Also widened `ValidatedShellLayout::as_layout` to
  `pub(crate)` so tests exercise the validated template path.
- GREEN `cb0c77fd` (`feat: render bounded native app dock`):
  - `src/ui/app_dock.rs`: `AppDockModel::for_state` projects exactly
    Terminal+Files in stable order from the TYPED stage authority
    (`stage.surface_view()` for active; live workspaces / open FM for
    running). Single-cell icons with ASCII fallbacks (`❯`/`>`, `▤`/`#`),
    width-1 asserted via `unicode_width`.
  - `app_dock_entry_areas`: one full-width single-row target per entry,
    stacked from the top, clipped to available rows; degenerate geometry
    produces no target.
  - `render_app_dock`: pure — ownership bar `▎` + accent for the active
    entry, `palette.text` for running, `palette.overlay0` for idle; icon
    centered after the bar column; no clock/random/I-O.
  - Production wiring: `BaseLayer` renders the dock ONLY when the current
    shell projects a non-empty `RegionId::AppDock` region — the legacy
    default template projects none, so live behavior is unchanged until a
    dock-bearing template activates (SF5.2+); the code path is live, not
    dead.
- Gates: dock table 7/7 (run `1693575c`); full Nextest `--no-fail-fast`
  3,322/3,322 plus only the named B0 skip; fmt; Linux all-target and
  Windows MSVC bin Clippy clean with `-D warnings`; diff and
  added-production-`unwrap()` clean.
- Publication: FF pushes to CyPack only; both refs equal exact SHA
  `cb0c77fd4d831d41f2e85cb2cebd80f589d9c078`; `upstream` untouched.

## SF5.2 — Dock interaction and anchored name popover (CLOSED)

- RED `406db487` (`test: define app dock interaction and popover`):
  compile-valid skeleton (new `ContextMenuKind::AppDock` variant + `items`
  arm + `enabled` on entry/area + `view.app_dock_entry_areas` computed from
  the live shell region) with the seven-row catalog; exactly the five
  interaction rows failed at runtime (clicks reached nothing), while
  `dock_resize_and_collapse_use_shared_transaction` passed as a
  characterization (the SF3 `ResizeTransaction` is region-generic by
  construction — dock divider clamps 3..=9 and stays inert on a stale
  generation through the SAME machinery, no dock-specific drag state) and
  the disabled row guarded vacuously until dispatch existed.
- GREEN `d031ef26` (`feat: activate dock apps with bounded name popover`):
  - `AppState::activate_dock_app` — one shared activation authority: Files
    opens (or keeps) its singleton surface, Terminal restores the terminal
    stage; both the dock left-click and the popover row use it, so the two
    paths cannot drift.
  - `App::handle_app_dock_mouse` — consumes EVERY event over live dock
    terrain (enabled left press activates; enabled right press opens the
    `ContextMenuKind::AppDock` popover through `enter_overlay_mode`, so
    SF4.2 blocking/restore/outside-close arrive free; disabled targets and
    modified/other events are consumed fail-closed). Runs in the
    non-overlay pre-branch beside the divider chrome; with the legacy
    default template the dock region is empty, so live behavior is
    unchanged until a dock-bearing template activates.
  - `apply_context_menu_action` gained the AppDock arm (activate +
    `leave_modal`); the test-only context-menu invariant checker classifies
    the new kind explicitly.
- Popover geometry reuses the existing clamped `context_menu_rect`
  (C3.2), which the reanchor-after-resize row pins.
- Gates: catalog 7/7 (run `287a41b6`); full Nextest `--no-fail-fast`
  3,329/3,329 plus only the named B0 skip; fmt; Linux all-target and
  Windows MSVC bin Clippy with `-D warnings` (one collapsible-if fixed);
  diff and added-production-`unwrap()` clean.
- SF5 CLOSURE gate at `d031ef26`: Bun 5/5 + 12/12, Python 64/64 rerun and
  green. Publication: FF pushes to CyPack only; both refs equal exact SHA
  `d031ef26d65b26967ac758a28da9dc478d996ae0`; `upstream` untouched.

## SF5 PHASE CLOSED — SF5.1 + SF5.2 GREEN

Test inventory grew 3,315 -> 3,329 across the phase.

## Exact Next Microtask

SF6.1: move the Files render projection out of the terminal curtain onto
the Workspace Stage (plan "Task SF6.1") — first recon the plan's RED
catalog and the current curtain seams (`render_file_manager` under
`terminal_area`, `sync_file_manager_view`), then RED/GREEN per slice. Then
SF6.2 lifecycle/input migration, SF6.3 perf/failure/isolated closure, then
FM1 (horizontal Miller viewport) and FM2 (column drag-resize — the user's
custom-layout target).
