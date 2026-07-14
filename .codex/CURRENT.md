# Current State — 2026-07-14

## Repository

- Path: `/home/ayaz/projects/herdr`
- Branch: `feat/native-fm`
- Published product/test checkpoint: `8cd4e89`
  (`test: make timing-sensitive lifecycle tests deterministic`).
- `origin/feat/native-fm` and fork `origin/master` include `8cd4e89` and the
  separate continuity/tooling commit containing this state file.
- `origin` is the `CyPack/herdr` fork. `upstream` is `ogulcancelik/herdr` and must never be pushed.

## Completed Checkpoint

- A2.2 responsive Miller columns were committed as `6c7c58f`, full graph-indexed,
  and fast-forward pushed to the CyPack feature branch and fork master only.

## Published Checkpoint — A4 Watcher

- A4 product commit: `01ba91d` (`feat: add live filesystem watching to native
  file manager`).

- Stable dependency: `notify-debouncer-full 0.7.0` with `notify 8.2.0`.
- Pure event normalization, generation filtering, burst coalescing, bounded
  channel drain, overflow recovery, and watcher lifecycle are implemented.
- Runtime ownership lives in `App`; render remains filesystem-free.
- Native watcher is primary. Init/runtime failure enters explicit
  `PollingFallback`; all active watchers also receive a bounded 2-second
  reconciliation safety-net for silent FUSE/NFS/exFAT-class backends.
- `FmState::reload()` preserves the exact selected path when possible and
  safely clamps deleted/renamed/hidden-filtered selections while rebuilding
  parent/preview context.
- Real filesystem create, rename, delete, and 16-file burst convergence is
  covered by a bounded-deadline integration test.
- Product paths: `Cargo.toml`, `Cargo.lock`, `src/app/file_manager_watcher.rs`,
  `src/app/mod.rs`, `src/app/runtime.rs`, `src/fm/watcher.rs`, `src/fm/mod.rs`.

## Published Test-Stability Work

- Test-only commit: `8cd4e89` (`test: make timing-sensitive lifecycle tests
  deterministic`).

- `src/server/headless.rs`: the metadata-expiry test now uses a long real TTL
  and still expires via its existing synthetic deadline.
- `src/terminal/state.rs`: the late lifecycle-hook test now uses one synthetic
  clock for authority, process exit, and late report.
- These fixes were required after full-suite parallel load exposed two existing
  wall-clock races. Keep them separate from the A4 feature commit.

## Fresh Verification Evidence

- Broad FM/file-manager filter: 69/69 passed.
- Full nextest final run: 2912/2912 passed, 0 skipped, no retry in the final run.
- `cargo fmt --check`: passed.
- Linux `cargo clippy --all-targets --locked -- -D warnings`: passed.
- Windows MSVC target clippy with `LIBGHOSTTY_VT_SIMD=false`: passed.
- Bun integration assets: 5/5; plugin marketplace: 12/12.
- Python maintenance: 64/64.
- `git diff --check`: passed.
- Lock audit: exactly 22 new watcher/target packages, no existing version
  upgrades; exact-version OSV batch returned zero advisories.
- `just` is absent; every applicable `just check` command was run directly.

## Graph and Publication Evidence

- `mcp-proxy.service` readiness was repaired without killing/restarting a user
  process. A 26-server cold start measured 54 seconds; the bounded readiness
  budget is now 120 seconds inside a 150-second systemd start budget.
- Readiness passed with `expected=26 observed=26 critical_tools=14`.
- Full graph reindex completed at 17,613 nodes / 83,598 edges.
- `miller_layout`, `NativeFileManagerWatcher`, and `normalize_watch_events`
  were each found as a current graph symbol in their expected source file.
- `01ba91d` and `8cd4e89` were fast-forward pushed to both CyPack
  `feat/native-fm` and fork `master`; remote SHA verification matched local
  `8cd4e89`. `upstream` was not pushed.

## Standing Git Authorization

- The user explicitly authorized autonomous commits and pushes for this
  project. Do not repeatedly ask for commit-message alignment.
- Still require targeted staging, lowercase conventional commits, fresh
  proportional verification, fast-forward ancestry, remote SHA verification,
  and CyPack fork-only writes. Never force and never push `upstream`.

## Exact Next Action

1. Execute B0 Image Path Beta test-point-first: bounded decode, synthetic
   placement, graphics framing/lifecycle, then isolated real-host evidence.
2. Record the B2 go/no-go decision before starting B2. B0 does not block B1 or
   the A3 remainder.
