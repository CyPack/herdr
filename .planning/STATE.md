# Herdr Native-FM Planning State

- Updated: 2026-07-16
- Branch: `feat/native-fm`
- Current verified product head: `efe6446b` (`feat: give blocking overlays
  keyboard ownership over captures`); matching RED `bb6f8970`. Prior heads
  this session: SF4.2-02 `41362e89`/`017ba97f`, SF4.2-01
  `92777e23`/`f4f5e3cb`, SF4.1-08 `784fdc2e`/`944a9d4c`, and test-stability
  `3c853a70`.
- Published SF0 planning artifact checkpoint: `32856f7`
  (`docs: plan shell foundation and files workspace`).
- Published SF1 characterization checkpoint: `7b9b626d`
  (`test: characterize shell foundation baseline`).
- Published SF3.1 product checkpoint: `336fa3de`
  (`feat: add keyboard shell resize routing`).
- Published SF3.2 product checkpoint: `45a2e87e`
  (`feat: add bounded shell scroll ownership`).
- Published SF3.3 product checkpoint: `90be6893`
  (`feat: persist shell collapse preference`).
- Exact remote SHA equality was verified for CyPack `feat/native-fm` and fork
  `master` at product SHA `90be689359988424b2a7c6206ff45a3207422196`.
- Approved product program: SF0-SF6 followed by FM1-FM5; Apps/Desktop remains
  a later independent program.
- Current phase: SF0-SF3 closed through I14. SF4.1 is CLOSED with 8/8 typed
  Stage behavior slices GREEN: default, activation history, singleton,
  16-instance bound, checked generation exhaustion, close restoration,
  failed-open Stage/focus rollback, and terminal-runtime preservation across
  stage switches (`AppDefinition`/`LaunchPolicy` + pure
  `StageState::surface_view()`). SF4.2 is the ACTIVE next slice; its first
  RED is table-driven `shell_input_router_follows_frozen_precedence`.
- Published SF2.4 RED/GREEN: `2a440478` / `07133b8b`; both CyPack refs equal
  exact SHA `07133b8b9e9cf10b9b3dea0febe22a8389457164`.
- Current sequential CLI graph: 20,396 nodes / 93,372 edges after the
  post-SF4.1 reindex. Fresh exact search proves `StageState.surface_view`,
  `activate_files` consulting `BuiltInAppId::Files.definition().launch`, and
  `miller_layout`; freshness was not inferred from status alone. The built-in
  MCP channel now serves the fresh store (verified by exact symbol and
  snippet, not `ready` alone).
- Published CyPack M3 evidence checkpoint: `e9f2fe0`.
- Verified M2.1 chain: RED `dab1e20`; GREEN product head `0ae6175`.
- M3.0 evidence is published with exact SHA equality to CyPack
  `feat/native-fm` and fork `master`. No product publication is pending.
- Completion audit: all approved native-FM core/post-core scope is reconciled
  in `.codex/evidence/native-fm-completion-audit.md`; ignored local PRD drift
  was repaired without product changes. The later explicit user demand now
  activates bounded S6/N2.2-derived work through the approved SF/FM program;
  S5 registry and S7 popup stack remain trigger-gated.

## Completed

- A2.2 responsive Miller columns.
- A3 viewport, stable row geometry, keyboard/mouse navigation.
- A4 native watcher plus bounded reconciliation fallback.
- B0 image Path Beta, B1 bounded text preview, B2 bounded native image preview.
- C1/N3 header actions, prepared selection authority, and clipboard model.
- C2/N4 stable row actions, cursor-independent bounded multi-selection, and
  bulk authority.
- C3.1 deterministic file context-menu model.
- C3.2 global-popup reuse, exact path-stable right-click selection policy,
  bounded placement, keyboard/mouse lifecycle, disabled styling, activation-
  time authority revalidation, and typed client-local intent only.
- C3.3 enabled/available/host-supported plugin file actions, deterministic
  ordering, exact public path context, activation-time disable-race checks,
  non-UTF-8 fail-closed behavior, and display-width-aware dynamic labels.
- C4.1 immutable exact-path preflight, staged no-replace COPY, atomic/EXDEV-safe
  MOVE, bounded single-lane worker, pure generation/terminal state, header and
  context Copy authority, Paste dispatch, and matching-cwd reconciliation.
- C4.2 exact-path Trash/Permanent confirmation, immutable symlink-safe delete
  preflight, immediate identity revalidation, restricted platform-trash
  backend, permanent file/directory deletion, shared bounded worker lane,
  ordered per-item recovery evidence, and matching-cwd reconciliation.
- C4.3 exact single-target Rename intent, shared platform-aware component
  validation, immutable identity revalidation, no-replace file/directory/
  symlink commit, bounded cycle-safe bulk staging, explicit recovery paths,
  shared worker lane, and matching-generation App reconciliation.
- C4.4.1 one bounded latest-value worker progress slot, monotonic started-item
  projection, same-generation App consumption, and production progress
  adapters shared by transfer, delete, single rename, and bulk rename.
- C4.4.2 explicit reversible/irreversible cancellation boundaries, typed Esc
  intent, matching-generation App/worker routing, repeated-cancel idempotence,
  revalidation-race precedence, and buffered-completion authority.
- C4.4.3 deterministic worker/watcher reconciliation for queued,
  watcher-first, delayed, polling-fallback, selection-pruning, cwd/rebind, and
  same-cwd close/reopen races.
- C4.4.4 terminal recovery for disconnect-after-progress, caught panic,
  cancel-to-next-generation, uncertain private bulk-recovery paths, generation
  floor preservation, lane reuse, baseline cleanup, and no-hot-retry sync.
- C4.4.5 complete closure gate across focused recovery, broad C4/FM, full Rust,
  Linux/Windows lint, Bun/Python maintenance, ignored inventory, graph
  freshness, and artifact/diff cleanliness.
- C5 exact existing-agent literal path handoff and non-agent Claude split with
  exact new-resource rollback.
- C6 Finder-fidelity sidebar, integrated action authority, visual polish,
  explicit directory/preview/operation failure truth, and isolated runtime
  composition proof.
- P4.0 architecture evidence gate: S5 component registry, S6 persisted shell,
  and S7 popup stack remain implementation NO-GO without concrete consumers.
- N2.0 rejected an arbitrary/unbounded Miller history state machine; N2.1
  preserves the exact departed child on parent navigation through a bounded
  path-focus refresh seam.
- M1.0 proved a genuine product delta and selected the narrow existing-agent,
  single-file attachment overlay described in
  `.codex/evidence/m1-agent-attachment-picker.md`.
- M1.1–M1.4 add the pure focused-agent `[+]` action, `prefix+a`, Clear-first
  private picker, exact workspace/`PaneId`/`TerminalId` authority, path-stable
  mouse/keyboard input, and one-shot literal path-plus-CR delivery.
- M2.0–M2.4 are closed: `[w]` uses pure cached capability and exact identity
  to enter the existing open dialog; duplicate Create/Remove/List/Switch
  implementations remain NO-GO.
- M3.0–M3.3 are closed implementation NO-GO: M1/M2 repeat only small pure
  geometry/render mechanics, not lifecycle, authority, focus, event, or
  cleanup ownership. No trait, registry, migration, or Rust product diff was
  created. Evidence: `.codex/evidence/m3-general-ui-interface.md`.

## Active Next Increment

- SF4.1 is CLOSED. Eight atomic RED/GREEN pairs end at `784fdc2e`/`944a9d4c`;
  detailed evidence is
  `.codex/evidence/shell-foundation-sf4-stage-progress.md`.
- SF4.2-01 is GREEN (`92777e23`/`f4f5e3cb`): the seven-tier frozen precedence
  is pure `route_shell_input`, and `App::handle_key` selects its keyboard
  tier through `AppState::shell_key_input_owner()`.
- SF4.2-02 is GREEN (`41362e89`/`017ba97f`): topmost blocking overlays now
  own every background mouse route — pre-branches stay inert behind
  `shell_mouse_input_owner()`, and one total early `Mode::ContextMenu` block
  consumes wheel/drag fail-closed while preserving item dispatch, hover,
  outside-close, and right-click re-targeting.
- SF4.2-03 is GREEN (`bb6f8970`/`efe6446b`): blocking overlays own keyboard
  input ahead of the active capture and the focused FM through one shared
  exhaustive `blocking_overlay_active()` classifier; `AttachFile` joined the
  overlay tier; one unrealistic Onboarding-mode fixture was corrected.
- SF4.2-04 is GREEN as an explicit characterization (`119e4a2d`): valid RED
  was refuted with source evidence (drag/up route by `DragState` without
  coordinate re-resolution; every left-down clears selection before a
  capture can begin), so the test freezes capture ownership outside the
  origin rect end-to-end (SF1 precedent).
- SF4.2-05 is GREEN and CLOSED (`8b1882eb`/`5eb63763` scoped core, then
  `27f8699f`/`3880c66b` full sweep): `leave_modal` consumes
  `overlay_return_mode` and restores a still-valid `Resize`/`Copy` owner;
  EVERY production overlay entry (24 call sites, ContextMenu x5 including a
  project new-chat menu the original inventory missed, Rename x5, Confirm
  x4, worktree dialogs x3, full-screen overlays x5 plus
  GlobalMenu/KeybindHelp) now goes through `enter_overlay_mode`, with a
  structural sweep proving zero direct overlay assignments remain in
  production. The ContextMenu-from-Copy row keeps a live copy session
  through an overlay episode. Direct exits that bypass `leave_modal` skip
  the restore nicety but cannot produce stale restores (recorded as an
  SF4.3 candidate). Current verified product head: `3880c66b`.
- SF4.2-06 is GREEN as a characterization (`3580ff19`): valid RED was
  refuted with source evidence (empty-rect hit filter, generation +
  containment in `hit_at`, collapsed-divider guard, degenerate toggle
  rects); the test freezes Hidden/zero-area inertness, compact-rail
  interactivity, and pins the previously unpinned adversarial
  collapsed-divider guard.
- SF4.2-07 is GREEN (`bb3ac54d`/`c6b024ce`): `shell_mouse_input_owner`
  takes the event position and resolves the topmost-hit tier through the
  new crate seam `ShellView::region_hit_at` against the EXACT current
  generation — old coordinates re-resolve to their current owner and never
  grant vanished authority; the single caller still consumes only the
  overlay comparison, so dispatch stays bit-identical until SF4.3/SF6
  consume semantic targets. Current verified head: `c6b024ce`.
- Next microtask: SF4.2-08 hidden-terminal blocking
  (`files_stage_blocks_hidden_terminal_input`), then the SF4.2 closure
  gate. See
  `.codex/evidence/shell-foundation-sf4-input-router-progress.md`.
- Then the remaining SF4.2 REDs (overlay blocking, capture ownership, focus
  restore, inert regions, stale generation, hidden-terminal blocking), one
  bounded focus/capture router shared by mouse and keyboard, and recovery
  proofs before SF4.3.
- Preserve frozen SF1 and complete SF2/SF3/SF4.1 baselines. Do not mix SF5
  AppDock render, SF6 Files rendering migration, FM1 history, or
  change-pipeline T3.1 into SF4.2 increments.
- After SF4 full closure, execute SF5, SF6, and FM1-FM5 sequentially through
  the approved plans. Product, continuity, and tooling commits remain separate.

The separate non-product change-pipeline lane has completed T0-T2. Ratatui
Design Intelligence v2.1 is locally committed (`7622dde` through `2517353`)
and freshly verified at 59/59 tests plus 15 phases/101 jobs. Its next exact
micro action is T3.1 RED module-contract tests for `herdr-change-pipeline`,
paused while the sequential product lane closes its current phase. This does
not authorize product code, stable runtime access, push, or reindex.

## Ordered Roadmap

1. SF0 design and baseline freeze.
2. SF1 characterization tests.
3. SF2 shell geometry foundation.
4. SF3 resize, collapse, scroll, and snapshot v4 persistence.
5. SF4 SurfaceHost and fail-closed input routing.
6. SF5 icon-only bounded AppDock.
7. SF6 Files migration to the native Workspace Stage.
8. FM1 horizontal Miller viewport.
9. FM2 Miller column resize.
10. FM3 all-column mouse ownership.
11. FM4 Finder-like path-stable growing navigation.
12. FM5 preview/Inspector placement evidence and GO/NO-GO.

S5 arbitrary registry and S7 popup ownership stack remain independently
deferred. Apps/Desktop expansion begins only after this twelve-phase program
closes and receives its own authorization.

## Fresh M2 Closure Evidence

- Exact M2.1 5/5; worktree/attachment regression 131/131.
- Full nextest 3202/3202; only `path_beta_real_host_probe` ignored.
- Linux all-target and canonical Windows MSVC clippy clean with `-D warnings`.
- Bun 17/17; Python maintenance 64/64; fmt, diff, production-unwrap,
  debug-marker, and ignored-inventory checks clean.
- Graph refresh: 19,534 nodes / 91,017 edges. Current symbols for
  `miller_layout`, `compute_agent_worktree_action_area`, and cached capability
  proved freshness beyond `ready`.
- No dependency, protocol, persisted runtime state, Git/filesystem mutation,
  process, socket, watcher, worker, pane, or stable Herdr resource changed.

## Fresh M3 Evidence

- Graph-first inventory at 19,534 nodes / 91,017 edges returned current
  `miller_layout`, M1/M2 compute/render areas, and input ownership; `ready`
  alone was not accepted.
- Characterization set: 16/16, zero retry, run
  `32ca7f37-b65c-45ef-9dbf-548e8263d383`.
- Protected Base terminal/FM swap, desktop/mobile shell regions, responsive and
  disjoint M1/M2 geometry/render, exact stale-identity dispatch, picker/dialog
  cleanup, modal/context focus/close, and old snapshot compatibility.
- Documentation/continuity only: no Rust, dependency, protocol, persistence,
  worker, watcher, filesystem/Git operation, process, pane, or socket change.

## Non-Negotiable Boundaries

- Pure render; filesystem preparation and execution stay outside render.
- No production `unwrap()` and no hidden authority from labels/coordinates.
- C3 emits intent only; C4/C5 own all side effects and execution-time checks.
- Never touch stable Herdr/socket or kill/restart user processes.
- Targeted staging only; never `git add -A`; never push `upstream` or force.

Canonical sources: `.codex/CURRENT.md`, `.codex/TASKS.md`, and
`.codex/HANDOFF.md`. Approved execution artifacts:

- `docs/superpowers/specs/2026-07-15-herdr-shell-foundation-v0-design.md`
- `docs/superpowers/plans/2026-07-15-herdr-shell-file-manager-program-plan.md`
- `docs/superpowers/plans/2026-07-15-herdr-shell-foundation-v0-implementation.md`
- `docs/superpowers/plans/2026-07-15-herdr-file-manager-post-shell-implementation.md`
- `.codex/evidence/shell-foundation-plan-review.md`
