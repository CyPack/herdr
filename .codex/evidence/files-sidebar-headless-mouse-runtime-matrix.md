# Files Sidebar Headless Mouse Runtime Matrix

Date: 2026-07-19

## Root Cause

The earlier FMR-2 closure covered:

```text
AppState mouse hit
  -> typed sidebar navigation request
  -> App::handle_scheduled_tasks
  -> exact Trail load
```

Remote app clients use `HeadlessServer::handle_scheduled_tasks_headless()`
instead. That loop did not call the existing
`App::sync_file_manager_sidebar_navigation()` consumer. Raw SGR input reached
the exact shortcut hit and prepared the request, but the remote/server-owned
runtime never consumed it. Miller content clicks remained functional because
they use a separate direct typed-navigation path.

## TDD Evidence

| Layer | Commit | Evidence |
|---|---|---|
| RED | `dbfa55be` | raw SGR input prepared the exact target, then failed at the headless scheduled-consumer assertion |
| GREEN | `72cdce83` | headless scheduled tasks call the existing revalidating one-shot consumer |

Production behavior added no new parser, hitbox, filesystem authority,
protocol field, socket message, or dependency.

## Fresh Gates

- Headless raw SGR mouse: 1/1 PASS.
- Existing shortcut family: 4/4 PASS.
- Existing typed request and consumer families: 3/3 PASS.
- Full Rust: 3,528/3,528 PASS, 2 skipped.
- Playwright Chromium: 22/22 PASS with unchanged baselines.
- Linux and Windows clippy: clean with `-D warnings`.
- Python maintenance: 68/68 PASS.
- Bun integration assets: 5/5 PASS.
- Bun plugin marketplace: 12/12 PASS.
- Formatting and diff checks: clean.
- Codebase graph: 23,556 nodes / 125,078 edges; fresh RED test symbol found.

Stable Herdr, stable sockets, user sessions, and `.superpowers/` were not
touched.
