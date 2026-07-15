# Herdr Native-FM Planning State

- Updated: 2026-07-15
- Branch: `feat/native-fm`
- Published CyPack base: `5e67099`.
- Verified M1 product head: `7d3144e`; continuity closure is pending commit.
- Current publication unit: M1.1–M1.4 atomic RED/GREEN chain plus closure. Push
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

## Active Next Increment

M2.0 evidence matrix for Git worktree management actions. No product mutation
is permitted until existing dialogs/API/CLI/keybinds are compared action by
action and each candidate terminates GO or NO-GO.

Test points are the TP-M2-DELTA/IDENTITY/CREATE/REMOVE/RECOVERY/PLATFORM matrix
in `.codex/TASKS.md`. Expected result: duplicate controls terminate NO-GO;
only a proven missing lower-friction action may activate a RED production lane.

## Ordered Roadmap

1. M2.0 evidence matrix for missing Git worktree management actions; no product
   mutation until action-by-action GO/NO-GO terminates.
2. M3.0 only after M1/M2 creates a second concrete independently owned
   component/page/action family.
3. N2.2 and S5–S7 remain independently evidence-gated.

## Fresh M1 Closure Evidence

- Exact attachment family 20/20.
- Full nextest 3197/3197; only `path_beta_real_host_probe` ignored.
- Linux all-target and canonical Windows MSVC clippy clean with `-D warnings`.
- Bun 17/17; Python maintenance 64/64; fmt, diff, production-unwrap,
  debug-marker, and ignored-inventory checks clean.
- Graph refresh: 19,113 nodes / 91,118 edges. Current snippets for
  `miller_layout`, `sync_agent_attachment_delivery`, and picker row geometry
  proved freshness beyond `ready`.
- No dependency, protocol, persisted runtime state, process, socket, watcher,
  worker, pane, new agent, or stable Herdr resource changed.

## Non-Negotiable Boundaries

- Pure render; filesystem preparation and execution stay outside render.
- No production `unwrap()` and no hidden authority from labels/coordinates.
- C3 emits intent only; C4/C5 own all side effects and execution-time checks.
- Never touch stable Herdr/socket or kill/restart user processes.
- Targeted staging only; never `git add -A`; never push `upstream` or force.

Canonical sources: `.codex/CURRENT.md`, `.codex/TASKS.md`, and
`.codex/HANDOFF.md`.
