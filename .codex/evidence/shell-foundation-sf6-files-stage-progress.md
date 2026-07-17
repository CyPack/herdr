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

## SF6.2 — Files lifecycle and input authority migration (CLOSED)

- RED-ability was verified per catalog row BEFORE writing: lifecycle
  open/repeat/close/restore/failed-open rows are already frozen by the
  SF4.1 slices; hidden-terminal blocking by SF4.2-08; stale Files hits
  after a surface switch by SF4.3-02; watcher init failure, worker
  panic/disconnect, operation-running, context-menu, selected-path
  deletion, and reopen rows by the C4-C6 suites (all rerun green in the
  plan's composite command below). The genuinely open contract was input
  AUTHORITY: keyboard and mouse routed from the legacy
  `file_manager.is_some()` boolean.
- RED `1faff0e0` (`test: define Files stage lifecycle and ownership`):
  `files_input_routes_from_typed_surface_authority` — aligned control
  (Files owns stage -> `FocusedComponent` keyboard + in-center mouse
  `Consumed`), then the adversarial divergent state (Files domain state
  present, typed stage TerminalWorkspace) failed exactly: the boolean
  still granted `FocusedComponent`. The shared `runtime_app_with_fm`
  fixture was migrated onto the real open transaction to make the control
  phase truthful.
- GREEN `11c054b8` (`feat: migrate Files lifecycle to workspace stage`):
  - `shell_key_input_owner`: the focused-component tier now requires the
    TYPED `StageSurfaceView::NativeFiles` AND live Files domain state.
  - `handle_file_manager_mouse`: early NotHandled unless the typed stage
    authority owns Files (plus domain presence for safe access).
  - Fixture debt retired: 37 direct `app.state.file_manager = Some(...)`
    test assignments across nine test files (worker, watcher, previews,
    rename, delete-confirmation, agent-handoff, plugins, input) migrated
    onto `try_open_file_manager_with`; the SF4.2-08 control phase now
    closes through `close_file_manager()`. The production watcher rebind
    swap (`self.state.file_manager = Some(next)`) is deliberately
    untouched — it replaces state WITHIN an open Files surface.
- Gates: exact 1/1; the plan's composite regression command
  (`file_manager|file_operation|file_preview|image_preview|watcher|`
  `stage_surface|app_dock|shell_input`) 214/214 with zero retries; full
  Nextest `--no-fail-fast` 3,330/3,330 plus only the named B0 skip; fmt;
  Linux all-target and Windows MSVC bin Clippy with `-D warnings`; diff
  and added-production-`unwrap()` clean.
- Publication: FF pushes to CyPack only; both refs equal exact SHA
  `11c054b832db841bea7cb4c3180b85cc10b18674`; `upstream` untouched.

## SF6.3 — Performance, failure, and closure (SCOPED CLOSE — deferred items OPEN)

User-authorized rescope: the active goal directive prioritizes reaching
FM1/FM2 (the custom-layout target). SF6.3's TESTABLE contracts closed now;
its ceremony items are recorded OPEN below, not silently dropped.

- CLOSED `887471c2` (`test: verify shell foundation integration and
  performance`): the plan's core performance contract is now a TEST, not a
  benchmark claim — one hundred pointer preview moves inside a single
  resize transaction write ZERO persistence and leave the committed width
  untouched; exactly the commit marks persistence once. PTY purity is
  structural and separately frozen (preview returns no `ResizeUpdate` by
  type; `resize_panes_during_shell_preview` suppresses pane resizing).
- Regression families: all exist as tests and ran green inside the full
  `--no-fail-fast` sweep at this head — v3->v4 migration/invalid-v4/
  future-version (SF3.3 matrix), overlay leakage (SF4.2), retained
  dirty-row (SF4.3-05), watcher fallback + worker failure + Files
  close/reopen (C4.4 families). Full just-check equivalent at `887471c2`:
  Rust 3,331/3,331 + B0 skip, Bun 5/5 + 12/12, Python 64/64, both Clippy
  targets, fmt/diff/unwrap clean. Both CyPack refs equal
  `887471c23655d53e64211cdb9c29cd26cbfcb33f`.
- OPEN (deferred with named conditions, to close before the stable-release
  audit of this program):
  1. Bounded perf counters + named-workload p95 benchmark harness (plan
     bullet 1-3) — requires a bench/instrumentation lane; no counter
     exists yet.
  2. The ISOLATED manual runtime proof per `.local/ISOLATED-DEV-TEST.md`
     (open Files through the AppDock live) — BLOCKED on a dock-bearing
     shell template going live; the legacy default template still projects
     no dock region. The template activation decision belongs to the
     custom-layout program (FM-era) and is the natural moment for this
     proof.

## Exact Next Microtask

FM1 (plan `2026-07-15-herdr-file-manager-post-shell-implementation.md`):
FM1.1 bounded chain/cache RED tests -> FM1.2 logical chain + resident
projections -> FM1.3 horizontal viewport projection/render. Frozen bounds:
`MAX_MILLER_HISTORY_DEPTH=32`, `MAX_RESIDENT_MILLER_COLUMNS=5`, column
widths 16/28/64; `MillerState`/`MillerColumnId`/targets per the plan's
"Frozen Interfaces and Bounds". Then FM2.1-2.2 column drag-resize (SF3
transaction with column targets) — the custom-layout target interaction.
