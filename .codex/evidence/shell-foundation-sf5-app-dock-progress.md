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

## Exact Next Microtask

SF5.2: dock interaction and anchored app-name popover (plan catalog:
`left_click_files_activates_existing_singleton_or_opens_one`,
`left_click_terminal_restores_terminal_stage`,
`right_click_opens_bounded_name_popover`, `popover_blocks_background_input`,
`popover_reanchors_or_closes_after_terminal_resize`,
`disabled_app_target_is_consumed_without_activation`,
`dock_resize_and_collapse_use_shared_transaction`). Reuse the SF3
resize/collapse reducer — no dock-specific drag state; the popover is a
topmost overlay entered through `enter_overlay_mode`. Then the SF5 closure
gate, then SF6.
