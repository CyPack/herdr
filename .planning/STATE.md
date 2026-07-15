# Herdr Native-FM Planning State

- Updated: 2026-07-15
- Branch: `feat/native-fm`
- Published CyPack base: `fb43091`
- Verified C4.4 reconciliation product head: `d1a2d2e`
- Publication unit: C4.4.3 nine-commit test/product chain through `d1a2d2e`,
  plus the continuity/graph commit
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

## Active Next Increment

TP-C4.4-RECOVERY must be RED before production changes.

Test points:

- Panic/disconnect/cancel recovery must leave the existing lane reusable,
  preserve uncertain recovery paths, and never orphan in-flight state.
- Run focused progress/cancel/reconcile/recovery tests, all C4 regressions, the
  full cross-platform gate, graph freshness, and artifact/diff checks before
  publication.

## Ordered Roadmap

1. C4.4 bounded progress/cancel, watcher reconciliation, terminal recovery,
   lane reuse, and the complete C4 closure gate.
2. C5 exact pane/agent handoff, quoting/identity, split-and-launch failure
   cleanup, and isolated-session safety.
3. C6 Finder-fidelity sidebar, highlight/location marker, integrated actions,
   theme/spacing/empty/error states, and visual review.
4. Deferred evidence-gated architecture: S5 ComponentRegistry, S6 persisted
   resizable shell, S7 popup stack, N2 dynamic Miller navigation.
5. North-star backlog: M1 interactive CLI attachments, M2 git-worktree
   controls, M3 general panel/page/button interface evaluation.

## Fresh C4.4.3 Evidence

- Broad C4/FM regression 126/126.
- Full nextest 3128/3128; only `path_beta_real_host_probe` ignored. A separate
  ignored-only inventory listed that exact test without executing it.
- Linux all-target and canonical Windows MSVC bin clippy clean with
  `-D warnings`.
- Bun 17/17; Python maintenance 64/64; fmt and diff-check clean.
- No `.herdr-operation-*` or `.herdr-rename-stage-*` artifact remains.
- Graph refresh: 18,786 nodes / 87,697 edges. Freshness queries returned
  `own_operation_reconcile`, delayed and same-cwd lifecycle tests, and
  `miller_layout` after proving the prior `ready` graph was stale.

## Non-Negotiable Boundaries

- Pure render; filesystem preparation and execution stay outside render.
- No production `unwrap()` and no hidden authority from labels/coordinates.
- C3 emits intent only; C4/C5 own all side effects and execution-time checks.
- Never touch stable Herdr/socket or kill/restart user processes.
- Targeted staging only; never `git add -A`; never push `upstream` or force.

Canonical sources: `.codex/CURRENT.md`, `.codex/TASKS.md`, and
`.codex/HANDOFF.md`.
