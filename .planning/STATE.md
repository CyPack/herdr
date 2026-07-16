# Herdr Native-FM Planning State

- Updated: 2026-07-16
- Branch: `feat/native-fm`
- Published SF0 planning artifact checkpoint: `32856f7`
  (`docs: plan shell foundation and files workspace`).
- Published SF1 characterization checkpoint: `7b9b626d`
  (`test: characterize shell foundation baseline`).
- Published SF3.1 product checkpoint: `336fa3de`
  (`feat: add keyboard shell resize routing`).
- Exact remote SHA equality was verified for CyPack `feat/native-fm` and fork
  `master` at the artifact checkpoint.
- Approved product program: SF0-SF6 followed by FM1-FM5; Apps/Desktop remains
  a later independent program.
- Current phase: SF0-SF2 and SF3.1 closed through I14. SF3.2 begins with a
  fresh graph/drift pass and the behavior-specific RED
  `collapse_remembers_last_committed_width`.
- Published SF2.4 RED/GREEN: `2a440478` / `07133b8b`; both CyPack refs equal
  exact SHA `07133b8b9e9cf10b9b3dea0febe22a8389457164`.
- Current single-worker CLI graph: 20,132 nodes / 93,587 edges. Fresh searches
  prove `handle_shell_resize_key`, both keyboard-step reducer symbols, and
  `miller_layout`. The built-in transport remains stale at 20,118 / 93,603 and
  was not restarted.
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

- Begin SF3.2 with a fresh ownership/drift pass, then write compile-valid RED
  `collapse_remembers_last_committed_width`.
- Expected: collapse remembers only the last committed bounded width, emits
  exactly one revision/dirty transition, and repeated collapse is inert.
- After collapse/restore closes, write separate owning-viewport REDs for
  horizontal/vertical scrolling; do not combine scroll or snapshot v4
  production into the first collapse slice.
- Preserve the SF1/SF2 baselines and SF3.1 zero-preview-effect contract in
  atomic RED/GREEN slices.
- Execute SF3-SF6 and then FM1-FM5 sequentially through the approved child
  plans. Product, continuity, and tooling concerns remain separate.

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
