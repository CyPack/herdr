# Files Sidebar Headless Mouse Forward-Fix Plan

Date: 2026-07-19
Status: completed (`dbfa55be` RED / `72cdce83` GREEN)
Spec:
`docs/superpowers/specs/2026-07-19-herdr-files-sidebar-headless-mouse-forward-fix.md`

## Layer 1 — Characterize the Real Runtime Boundary

- Add a `HeadlessServer` test with one app client and a real accessible temp
  shortcut target.
- Render/compute the production view at the client size and obtain the prepared
  shortcut row only to encode the host's 1-based SGR coordinates.
- Send raw `\x1b[<0;<col>;<row>M` through `ServerEvent::ClientInput`.
- Assert the request exists before the scheduled tick so parser/input/hit-test
  failures are distinguished from consumer failures.

RED commit: `test: reproduce headless files shortcut navigation gap`

## Layer 2 — Restore the Missing Consumer

- Call the existing `App::sync_file_manager_sidebar_navigation()` from
  `HeadlessServer::handle_scheduled_tasks_headless()`.
- Preserve its exact-path, accessible-item, live-directory, generation, and
  race revalidation.
- Do not duplicate the consumer or add direct I/O to mouse routing.
- Assert the scheduled tick consumes exactly once and loads the target Trail.

GREEN commit: `fix: consume files shortcut navigation in headless mode`

## Layer 3 — Regression and Visual Gates

- Focused server-level raw-input test.
- Existing `sidebar_shortcut_mouse`, `clicking_file_sidebar_item`, and
  `sidebar_navigation` families.
- Playwright Chromium full visual suite; update no baseline unless the
  deterministic fixture intentionally changes.
- Full Rust nextest with `--no-fail-fast`, Linux clippy, Windows clippy,
  formatting, maintenance Python/Bun gates, diff check, and production
  `unwrap()` scan.

## Layer 4 — Continuity and Delivery

- Mark FMR-2A and its subtasks complete with RED/GREEN SHAs.
- Regenerate `.codex/HANDOFF.md` OPEN_TASKS from both registries and require an
  exact diff.
- Reindex `home-ayaz-projects-herdr` with `CBM_WORKERS=1`.
- Fetch, verify fast-forward ancestry, push only CyPack
  `feat/native-fm` and `master`, then verify both remote SHAs.

Closure graph: `23,556 nodes / 125,078 edges`, fresh symbol
`headless_raw_mouse_shortcut_navigation_loads_exact_trail`.
