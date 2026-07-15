# Herdr Native-FM Planning State

- Updated: 2026-07-15
- Branch: `feat/native-fm`
- Published CyPack base: `821506e`
- Verified C4.4 terminal-recovery product head: `c674296`
- Publication unit: C4.4.4 seven-commit test/product chain from `0881976`
  through `c674296`, plus the continuity/graph commit
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
- C4.4.4 terminal recovery for disconnect-after-progress, caught panic,
  cancel-to-next-generation, uncertain private bulk-recovery paths, generation
  floor preservation, lane reuse, baseline cleanup, and no-hot-retry sync.
- C4.4.5 complete closure gate across focused recovery, broad C4/FM, full Rust,
  Linux/Windows lint, Bun/Python maintenance, ignored inventory, graph
  freshness, and artifact/diff cleanliness.

## Active Next Increment

C5.1 graph-first pane/agent runtime-boundary verification, followed by
TP-C5-AUTHORITY RED before production changes.

Test points:

- Current exact path and uniquely resolved current terminal/agent identity are
  mandatory; stale selection, reordered rows, closed FM, unsupported path,
  missing/duplicate target, or in-flight conflict fails closed.
- Literal handoff preserves spaces, quotes, metacharacters, Unicode, and exact
  target identity without shell interpolation, duplicate sends, or wrong-pane
  delivery.
- Split-and-Claude launch either owns one new pane/process or removes only its
  newly created resources on split/spawn/early-exit/cancel/stale failure.
- Runtime proof uses `.local/ISOLATED-DEV-TEST.md`; stable Herdr, inherited
  stable sockets, existing panes, and user processes remain untouched.
- Complete C5 gates precede C6; exact expected results and reasons are in
  `.codex/TASKS.md` TP-C5-AUTHORITY/SEND/SPLIT/ISOLATION/GATES.

## Ordered Roadmap

1. C5 exact pane/agent handoff, quoting/identity, split-and-launch failure
   cleanup, and isolated-session safety.
2. C6 Finder-fidelity sidebar, highlight/location marker, integrated actions,
   theme/spacing/empty/error states, and visual review.
3. Deferred evidence-gated architecture: S5 ComponentRegistry, S6 persisted
   resizable shell, S7 popup stack, N2 dynamic Miller navigation.
4. North-star backlog: M1 interactive CLI attachments, M2 git-worktree
   controls, M3 general panel/page/button interface evaluation.

## Fresh C4.4 Closure Evidence

- Focused recovery 46/46; C4 core 67/67; broad C4/FM 218/218.
- Final full nextest 3131/3131; only `path_beta_real_host_probe` ignored. A
  separate ignored-only inventory listed that exact test without executing it.
- Linux all-target and canonical Windows MSVC bin clippy clean with
  `-D warnings`.
- Bun 17/17; Python maintenance 64/64; fmt and diff-check clean.
- No `.herdr-operation-*` or `.herdr-rename-stage-*` artifact remains.
- Graph refresh: 18,793 nodes / 87,788 edges. Freshness queries returned
  `new_after_generation`, disconnect/panic/private-recovery tests, and
  `miller_layout` after proving the prior `ready` graph was stale.

## Non-Negotiable Boundaries

- Pure render; filesystem preparation and execution stay outside render.
- No production `unwrap()` and no hidden authority from labels/coordinates.
- C3 emits intent only; C4/C5 own all side effects and execution-time checks.
- Never touch stable Herdr/socket or kill/restart user processes.
- Targeted staging only; never `git add -A`; never push `upstream` or force.

Canonical sources: `.codex/CURRENT.md`, `.codex/TASKS.md`, and
`.codex/HANDOFF.md`.
