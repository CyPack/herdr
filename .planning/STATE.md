# Herdr Native-FM Planning State

- Updated: 2026-07-15
- Branch: `feat/native-fm`
- Published CyPack base: `445add3`
- Verified C4.1 product head: `98c51e4`
- Publication unit: C4.1 five RED/GREEN pairs through `98c51e4`, plus the continuity/graph commit
  containing this file. Push only CyPack `feat/native-fm` and fork `master`
  after fast-forward ancestry and exact remote-SHA verification.

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

## Active Next Increment

TP-C4.2-CONFIRM must be RED before production changes.

Test points:

- Trash versus permanent delete must be explicit and confirmed from current
  exact path/order authority; a stale dialog or closed FM fails closed.
- Trash is the recoverable default; symlinks are moved as links, never followed.
- Missing/replaced/read-only/backend/permission failures and partial multi-item
  results retain exact per-item state; no destructive success is inferred.
- Permanent delete is a separately gated irreversible path with stronger
  confirmation, immediate identity revalidation, and cancellation boundaries.
- Completion and watcher bursts converge without stale selection, duplicate
  entries, hot retry, or leaked temp artifacts.

## Ordered Roadmap

1. C4 safe copy/move, trash/delete, rename/bulk rename, bounded progress/cancel,
   TOCTOU, collision, permission, cross-filesystem, partial-failure, and watcher
   reconciliation tests.
2. C5 exact pane/agent handoff, quoting/identity, split-and-launch failure
   cleanup, and isolated-session safety.
3. C6 Finder-fidelity sidebar, highlight/location marker, integrated actions,
   theme/spacing/empty/error states, and visual review.
4. Deferred evidence-gated architecture: S5 ComponentRegistry, S6 persisted
   resizable shell, S7 popup stack, N2 dynamic Miller navigation.
5. North-star backlog: M1 interactive CLI attachments, M2 git-worktree
   controls, M3 general panel/page/button interface evaluation.

## Fresh C4.1 Evidence

- Operation core 15/15; App/worker 8/8; FM/watcher/preview 147/147.
- Full nextest 3064/3064; only `path_beta_real_host_probe` skipped.
- Linux all-target and canonical Windows MSVC bin clippy clean with
  `-D warnings`.
- Bun 17/17; Python maintenance 64/64; fmt and diff-check clean.
- Isolated real-filesystem COPY/MOVE/failure/cancel/reopen tests left no
  operation, staging, or preflight temp artifact.
- Graph refresh: 18,453 nodes / 86,399 edges. Supported `CBM_WORKERS=1`
  one-shot CLI completed with zero extraction errors and returned current
  operation state, dispatch/sync methods, and exact source snippet.

## Non-Negotiable Boundaries

- Pure render; filesystem preparation and execution stay outside render.
- No production `unwrap()` and no hidden authority from labels/coordinates.
- C3 emits intent only; C4/C5 own all side effects and execution-time checks.
- Never touch stable Herdr/socket or kill/restart user processes.
- Targeted staging only; never `git add -A`; never push `upstream` or force.

Canonical sources: `.codex/CURRENT.md`, `.codex/TASKS.md`, and
`.codex/HANDOFF.md`.
