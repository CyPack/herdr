# Herdr Native-FM Planning State

- Updated: 2026-07-14
- Branch: `feat/native-fm`
- Published CyPack base: `4bedd55`
- Verified C3.2 product head: `0915964`
- Publication unit: six C3.2 RED/GREEN commits plus the continuity/graph commit
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

## Active Next Increment

TP-C3.3-PLUGIN-SURFACE must be RED before production changes.

Test points:

- Accept only valid enabled manifest actions with `contexts=["file"]`.
- Reject wrong/unknown contexts, disabled plugins, malformed or duplicate IDs.
- Preserve deterministic built-in/plugin ordering for one and many exact paths.
- Serialize invocation context from prepared explicit path authority only.
- Do not add private TUI-only socket fields or execute filesystem/agent work.
- Preserve existing plugin, context-menu, FM/watcher, and full-repo regressions.

## Ordered Roadmap

1. C3.3 plugin file-action surface.
2. C4 safe copy/move, trash/delete, rename/bulk rename, bounded progress/cancel,
   TOCTOU, collision, permission, cross-filesystem, partial-failure, and watcher
   reconciliation tests.
3. C5 exact pane/agent handoff, quoting/identity, split-and-launch failure
   cleanup, and isolated-session safety.
4. C6 Finder-fidelity sidebar, highlight/location marker, integrated actions,
   theme/spacing/empty/error states, and visual review.
5. Deferred evidence-gated architecture: S5 ComponentRegistry, S6 persisted
   resizable shell, S7 popup stack, N2 dynamic Miller navigation.
6. North-star backlog: M1 interactive CLI attachments, M2 git-worktree
   controls, M3 general panel/page/button interface evaluation.

## Fresh C3.2 Evidence

- Focused popup 4/4; lifecycle 3/3; disabled render 1/1.
- Broad FM/global-menu 51/51; menu/render 26/26.
- Full nextest 3033/3033; only the named B0 interactive host probe skipped.
- Linux all-target and canonical Windows MSVC bin clippy clean with
  `-D warnings`.
- Bun 17/17; Python maintenance 64/64; fmt and diff-check clean.
- Graph refresh: 18,139 nodes / 86,595 edges. Parallel codebase-memory 0.8.1
  extraction crashed in native Tree-sitter cleanup; without restarting or
  killing any service, the supported `CBM_WORKERS=1` one-shot CLI fallback
  completed with zero extraction errors and returned current C3.2 production,
  test, and source-snippet evidence.

## Non-Negotiable Boundaries

- Pure render; filesystem preparation and execution stay outside render.
- No production `unwrap()` and no hidden authority from labels/coordinates.
- C3 emits intent only; C4/C5 own all side effects and execution-time checks.
- Never touch stable Herdr/socket or kill/restart user processes.
- Targeted staging only; never `git add -A`; never push `upstream` or force.

Canonical sources: `.codex/CURRENT.md`, `.codex/TASKS.md`, and
`.codex/HANDOFF.md`.
