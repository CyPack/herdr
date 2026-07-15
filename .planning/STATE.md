# Herdr Native-FM Planning State

- Updated: 2026-07-15
- Branch: `feat/native-fm`
- Published CyPack base: `a7bca0e`
- Verified C4.2 product head: `917cd57`
- Publication unit: C4.2 seventeen-commit test/product chain through `917cd57`, plus the continuity/graph commit
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
- C4.2 exact-path Trash/Permanent confirmation, immutable symlink-safe delete
  preflight, immediate identity revalidation, restricted platform-trash
  backend, permanent file/directory deletion, shared bounded worker lane,
  ordered per-item recovery evidence, and matching-cwd reconciliation.

## Active Next Increment

TP-C4.3-INTENT must be RED before production changes.

Test points:

- Header, context, and row Rename must converge on one exact current single
  target; stale, reordered, multi-selected, closed, or in-flight intent fails closed.
- Name validation must reject empty, path-like, reserved, non-UTF-8, and
  over-limit components before scheduling; unchanged input is an explicit no-op.
- Exact, case-fold, duplicate-output, and replacement-race collisions never
  overwrite. Single rename revalidates identity and uses no-replace commit.
- Bulk mapping validates completely before mutation; chains and swaps/cycles
  use bounded private staging with explicit rollback/partial recovery evidence.
- The existing worker generation and watcher reconciliation must terminalize
  every item, reject stale completion, and leave no private staging artifact.

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

## Fresh C4.2 Evidence

- Focused delete 29/29; broad FM/watcher/preview/context/plugin 321/321.
- Full nextest 3086/3086; only `path_beta_real_host_probe` skipped.
- Linux all-target and canonical Windows MSVC bin clippy clean with
  `-D warnings`.
- Bun 17/17; Python maintenance 64/64; fmt and diff-check clean.
- Isolated throwaway-HOME/XDG Trash proved file and symlink deletion while
  preserving the symlink target; no delete/staging/preflight artifact remains.
- Graph refresh: 18,576 nodes / 86,769 edges. Freshness queries returned both
  `miller_layout` and all current delete confirmation/core/worker symbols.

## Non-Negotiable Boundaries

- Pure render; filesystem preparation and execution stay outside render.
- No production `unwrap()` and no hidden authority from labels/coordinates.
- C3 emits intent only; C4/C5 own all side effects and execution-time checks.
- Never touch stable Herdr/socket or kill/restart user processes.
- Targeted staging only; never `git add -A`; never push `upstream` or force.

Canonical sources: `.codex/CURRENT.md`, `.codex/TASKS.md`, and
`.codex/HANDOFF.md`.
