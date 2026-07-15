# Herdr Native-FM Planning State

- Updated: 2026-07-15
- Branch: `feat/native-fm`
- Published CyPack base: `918f4fc`.
- Verified M2.1 chain: RED `dab1e20`; GREEN product head `0ae6175`.
- Current publication unit: M2.1 closure/continuity. Push
  only CyPack `feat/native-fm` and fork `master` after targeted staging,
  fast-forward ancestry, and exact remote-SHA verification.

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

## Active Next Increment

M3.0 read-only architecture evidence gate. M1 `[+]` and M2 `[w]` now provide
two concrete frame-action consumers, but production refactoring remains NO-GO
unless measured duplication proves a smaller shared ownership contract.

## Ordered Roadmap

1. M3.0 inventory M1/M2 frame-action duplication and rerun the P4 decision
   matrix; conclude GO/NO-GO before production edits.
2. If and only if M3.0 is GO, freeze characterization tests before M3.1.
3. N2.2 and S5–S7 remain independently evidence-gated.

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

## Non-Negotiable Boundaries

- Pure render; filesystem preparation and execution stay outside render.
- No production `unwrap()` and no hidden authority from labels/coordinates.
- C3 emits intent only; C4/C5 own all side effects and execution-time checks.
- Never touch stable Herdr/socket or kill/restart user processes.
- Targeted staging only; never `git add -A`; never push `upstream` or force.

Canonical sources: `.codex/CURRENT.md`, `.codex/TASKS.md`, and
`.codex/HANDOFF.md`.
