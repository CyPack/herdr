# Herdr Native-FM Planning State

- Updated: 2026-07-15
- Branch: `feat/native-fm`
- Published CyPack base: `ed47da8`
- Verified C4.3 product head: `c7043e2`
- Publication unit: C4.3 eighteen-commit test/product chain through `c7043e2`, plus the continuity/graph commit
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

## Active Next Increment

TP-C4.4-PROGRESS must be RED before production changes.

Test points:

- Transfer, delete, single rename, and bulk rename must expose one bounded,
  monotonic aggregate/per-item progress model with coalesced worker updates.
- Cancellation must be idempotent before work, during reversible staging/copy,
  and at irreversible publish/delete boundaries; committed work stays explicit.
- Worker completion, watcher bursts, polling fallback, selection pruning, cwd
  changes, and close/reopen must converge under one matching generation.
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

## Fresh C4.3 Evidence

- Focused/broad rename, bulk, worker, App, and watcher regression 163/163.
- Full nextest 3109/3109; only `path_beta_real_host_probe` ignored.
- Linux all-target and canonical Windows MSVC bin clippy clean with
  `-D warnings`.
- Bun 17/17; Python maintenance 64/64; fmt and diff-check clean.
- Real temporary-filesystem tests proved file/directory/symlink rename,
  source/destination races, cycles, swaps, injected rollback failure, and exact
  recovery paths; no private staging artifact remains.
- Graph refresh: 18,722 nodes / 88,526 edges. Freshness queries returned
  `miller_layout` plus current single/bulk/name/App rename symbols after proving
  the prior `ready` graph was stale.

## Non-Negotiable Boundaries

- Pure render; filesystem preparation and execution stay outside render.
- No production `unwrap()` and no hidden authority from labels/coordinates.
- C3 emits intent only; C4/C5 own all side effects and execution-time checks.
- Never touch stable Herdr/socket or kill/restart user processes.
- Targeted staging only; never `git add -A`; never push `upstream` or force.

Canonical sources: `.codex/CURRENT.md`, `.codex/TASKS.md`, and
`.codex/HANDOFF.md`.
