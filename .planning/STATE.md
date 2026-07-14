# Herdr Native-FM Planning State

- Updated: 2026-07-15
- Branch: `feat/native-fm`
- Published CyPack base: `e4af288`
- Verified C3.3 product head: `3c11369`
- Publication unit: C3.3 RED `0e06181`, GREEN `3c11369`, plus the continuity/graph commit
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

## Active Next Increment

TP-C4.1-PREFLIGHT must be RED before production changes.

Test points:

- Exact prepared source identity must be revalidated immediately before work.
- Collision/same-path/descendant/read-only/permission/symlink cases fail before
  mutation; no implicit overwrite and no implicit symlink traversal.
- Copy publishes only complete staged destinations and cleans failure/cancel.
- Move uses same-filesystem rename where possible; cross-filesystem fallback
  commits copy before any source removal and reports partial terminal states.
- Worker/queue/progress/cancel are bounded, generation-safe, and outside render.
- Watcher completion/reconciliation converges without stale selection or
  duplicate entries; isolated tests leave no temp artifact.

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

## Fresh C3.3 Evidence

- Focused C3.3 8/8; plugin/context 35/35; FM/watcher/global-menu 112/112.
- Full nextest 3041/3041; only `path_beta_real_host_probe` skipped.
- Linux all-target and canonical Windows MSVC bin clippy clean with
  `-D warnings`.
- Bun 17/17; Python maintenance 64/64; fmt and diff-check clean.
- Generated next API schema is current; protocol stays 16 because the extension
  is optional JSON API data, not a private transport-frame change.
- Graph refresh: 18,246 nodes / 85,535 edges. Supported `CBM_WORKERS=1`
  one-shot CLI completed with zero extraction errors and returned current
  selector, typed params, Unicode geometry, and disable-race source evidence.

## Non-Negotiable Boundaries

- Pure render; filesystem preparation and execution stay outside render.
- No production `unwrap()` and no hidden authority from labels/coordinates.
- C3 emits intent only; C4/C5 own all side effects and execution-time checks.
- Never touch stable Herdr/socket or kill/restart user processes.
- Targeted staging only; never `git add -A`; never push `upstream` or force.

Canonical sources: `.codex/CURRENT.md`, `.codex/TASKS.md`, and
`.codex/HANDOFF.md`.
