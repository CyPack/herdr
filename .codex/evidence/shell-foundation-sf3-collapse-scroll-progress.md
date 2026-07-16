# Shell Foundation SF3.2 Collapse and Scroll Evidence

Date: 2026-07-16

## Decision

Result: **PASS; SF3.2 is closed. SF3.3 snapshot-v4 persistence is next.**

SF3.2 adds bounded client-local collapse/restore state, routes the existing
sidebar mouse and keyboard toggles through one application adapter, keys shell
geometry by a monotonic collapse revision, and establishes a pure two-axis
scroll reducer with topmost fail-closed ownership. It does not add a protocol,
runtime resource, filesystem operation, render mutation, or new dependency.

## Test Points and Expected Results

| Test point | Expected result | Reason |
|---|---|---|
| Collapse committed width | One collapse remembers the last committed width, advances revision once, and marks persistence dirty once | Restore must preserve explicit user intent |
| Collapse no-op/failure paths | Repeated collapse, mandatory Stage collapse, revision exhaustion, and infeasible expand are inert | Invalid or duplicate intent must not corrupt layout state |
| Expand after shrink | Restore width clamps to current min/max and available total | Terminal shrink must not recreate impossible geometry |
| Shared input adapter | Mouse and both keyboard toggle routes use one state transition; one changed transition marks dirty | Input paths must not drift |
| Geometry revision | Collapse and expand each invalidate shell geometry once; repeated compute does not | Stale hit generations must not be authorized by a cycling boolean |
| Axis clamp | Horizontal and vertical offsets saturate independently at `content - viewport` | Miller and panel scrolling share one bounded primitive |
| Stale/zero viewport | Content shrink reconciles offsets; zero area consumes without scrolling | Watcher/resize/tiny-terminal states must stay panic-free |
| Topmost owner | Only the final owner changes; boundary and stale top owners consume inert | Input must never leak into a background surface |

Compile, formatter, filter, or setup failures were not counted as RED. The two
formatter-only interruptions were corrected mechanically before the behavior
tests were run.

## Atomic Product Chain

- Collapse reducer RED/GREEN: `deb8ca45` / `08a7d42b`.
- App adapter and real-route REDs: `71d79894`, `a316422e`; GREEN `0ede6fd8`.
- Monotonic geometry revision RED/GREEN: `5b007728` / `79e50983`.
- Scroll ownership REDs: `3faca061`, `d833081c`; GREEN `45a2e87e`.

Every RED compiled and failed at the intended behavior assertion. The final
product head is exact SHA
`45a2e87ec0b69b14c2a19348d09f85a2c7568191`.

## Architecture and Performance

- `ShellPresentationState` owns committed client-local collapse preferences;
  transient resize capture remains in `ShellInteractionState`.
- Legacy Compact/Hidden rendering behavior remains the compatibility
  projection; collapse state does not turn the existing compact rail into a
  different product behavior.
- `ShellGeometryKey.collapse_revision` uses the aggregate monotonic revision,
  not the cycling `sidebar_collapsed` boolean.
- `ScrollViewportState` stores only stable `(RegionId, slot)` identity and a
  two-axis offset. Derived content/viewport metrics and bottom-to-top ownership
  projections are supplied separately.
- Offset update and reconcile are `O(1)`. Top-owner state resolution is `O(n)`
  over the bounded visible component set (Foundation limit 64), with no
  allocation in the routing path.
- Zero-area, boundary, and stale-owner paths consume fail-closed. Render remains
  pure and no PTY resize, persistence write, or SSH/network message is emitted
  by scroll preview/update.

The Ratatui Design Intelligence tiling/layout and Herdr adaptation guides were
applied as behavior/architecture references: cached/derived geometry remains
separate from persistent presentation state, and no third-party code or new
dependency was copied.

## Fresh Verification

- Collapse/scroll exact contracts: 6/6 scroll GREEN, run
  `a6edfeb3-d96a-452d-b4c5-63d324314b57`.
- Broad shell/sidebar/input regression: 202/202, run
  `fe842cf7-4605-4235-a7dd-51f3e15d4ddd`.
- Frozen SF1 characterization: 11/11, run
  `b9e1af2d-d959-4b14-8219-685f85653c52`.
- Full repository Nextest: 3281/3281 pass and one intentional skip, run
  `9e4d2954-4a35-46b2-8256-4c541d02982e`.
- Ignored inventory contains exactly
  `kitty_graphics::tests::path_beta_real_host_probe`; it was listed and not
  executed.
- `cargo fmt --check`: PASS.
- Linux all-target Clippy with `-D warnings`: PASS.
- Canonical Windows MSVC binary Clippy with
  `LIBGHOSTTY_VT_SIMD=false`: PASS.
- Bun integration assets 5/5 and plugin marketplace 12/12: PASS (17/17).
- Python maintenance modules: 64/64 PASS.
- Commit-range diff, added production `unwrap()`, temp fixture residue, and
  worktree boundary checks: PASS. Only user-owned `.superpowers/` is untracked.

No stable Herdr process/socket, installed binary, user process, or real-host B0
probe was contacted, restarted, terminated, or executed.

## Git and Graph Evidence

All product commits were targeted-stage only. CyPack `feat/native-fm` and fork
`master` were fetched, proven ancestors, fast-forward pushed, and verified at
exact SHA `45a2e87ec0b69b14c2a19348d09f85a2c7568191`. `upstream` was not pushed;
no force push occurred.

The supported single-worker graph refresh completed with zero extraction
errors:

```text
CBM_WORKERS=1 codebase-memory-mcp cli index_repository '{"repo_path":"/home/ayaz/projects/herdr","mode":"fast","persistence":true}'
```

The fresh graph is 20,236 nodes / 94,402 edges. Exact graph search and source
snippets return:

- `src.ui.shell.interaction.route_scroll_to_topmost`;
- `src.app.input.shell.AppState.set_sidebar_collapsed`;
- `src.ui.file_manager.miller_layout`.

Freshness was verified by current symbols and source, not by `indexed`/`ready`
alone. No proxy or process was restarted.

## Next Gate

SF3.3 begins with a graph/drift/ownership pass over
`SessionSnapshot`, `parse_snapshot`, `migrate_snapshot`, save projection, and
restore application. The first behavior RED will require a v3 snapshot to
derive bounded shell-presentation defaults from its existing sidebar facts
without losing sidebar-section or unrelated session state. Snapshot v4 product
code must wait for that compile-valid assertion RED.
