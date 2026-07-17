# SF6 — Migrate Files into Workspace Stage (progress)

Plan contract: `docs/superpowers/plans/2026-07-15-herdr-shell-foundation-v0-implementation.md`
"Task SF6.1/6.2/6.3". Predecessors: SF4 closed `f973740e`, SF5 closed
`d031ef26`. Target chain continues into FM1 (horizontal Miller viewport)
and FM2 (column drag-resize — the user's custom-layout target).

## SF6.1 — Files render projection out of the terminal curtain (CLOSED)

- RED `8a256d37` (`test: define Files workspace stage rendering`): per the
  plan, the SF1 curtain characterization
  (`files_curtain_currently_replaces_terminal_surface`) was REPLACED by the
  target `files_renders_as_native_workspace_stage_surface`. The control
  phase proves the terminal surface owns a real non-empty tab bar; the
  target rows then failed exactly at `terminal_area == WorkspaceStage`
  (the curtain carved a tab-bar row out of the stage: y=1/h=29 versus
  y=0/h=30). Rows: stage ownership + no tab-bar chrome + sidebar separate
  + FM content spans the stage + no terminal text leak + runtime survives
  + collapsed-sidebar wider stage + tiny-terminal panic-free + explicit
  mobile contract (empty desktop shell regions, dedicated full-width
  content area).
- GREEN `8472f14b` (`feat: host Files in the native workspace stage`): the
  tab bar is terminal-app chrome — while NativeFiles is active the desktop
  projection carves NO tab-bar row and Files owns the COMPLETE
  WorkspaceStage (`terminal_area == stage`, `tab_bar_rect` empty, tab hit
  areas empty by construction). The surface flag moved above the chrome
  split so one binding governs chrome, hit geometry, and renderer choice.
  The plan's "delete the `file_manager.is_some()` curtain branch" was
  already satisfied by SF4.3-06's typed `SurfaceHost` match; SF6.1
  completed the geometry side. FM input's `in_center` follows
  `view.terminal_area`, so mouse coverage widened together with the render
  — no split authority.
- Blast radius bounded by `--no-fail-fast`: exactly TWO fixtures pinned
  the old curtain arithmetic (viewport-normalization and row-area
  snapshots assumed the tab-bar row); both were updated to the verified
  stage arithmetic (one more visible list row) with expectations derived
  from the new frozen geometry, and the direct `file_manager = None` close
  in one fixture migrated onto `close_file_manager`.
- Gates: exact 3/3 (run `348056d8`); full Nextest `--no-fail-fast`
  3,329/3,329 plus only the named B0 skip; fmt; Linux all-target and
  Windows MSVC bin Clippy with `-D warnings`; diff and
  added-production-`unwrap()` clean.
- Publication: FF pushes to CyPack only; both refs equal exact SHA
  `8472f14b057e4e83180fe1a37cd8983d853f563d`; `upstream` untouched.

## Exact Next Microtask

SF6.2: migrate Files lifecycle and input authority (plan "Task SF6.2").
Recon note recorded now: much of the catalog is ALREADY delivered by
earlier phases — lifecycle singleton/close-restoration/failed-open
rollback (SF4.1), hidden-terminal input seal (SF4.2-08), stale-hit
retirement on switch (SF4.3-02), watcher/worker/operation authority
(C4-C6). Verify RED-ability per row; the genuinely open contract is
routing Files keyboard/mouse from the TYPED `AppSurfaceRef::NativeFiles`
authority instead of the legacy `file_manager.is_some()` boolean
(SF4.3-06 precedent, adversarial divergent-state RED), plus the plan's
composite regression command. Then SF6.3
(perf/failure/migration/isolated-runtime closure per
`.local/ISOLATED-DEV-TEST.md`), then FM1/FM2.
