# Current State — 2026-07-14

## Repository

- Path: `/home/ayaz/projects/herdr`
- Branch: `feat/native-fm`
- Published B1 continuity checkpoint: `a0f82a3`
  (`docs: close text preview and plan navigation remainder`).
- CyPack `feat/native-fm` and fork `master` were both fast-forwarded from
  `68abf61` to `a0f82a3` and verified at the exact remote SHA.
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

## Published Checkpoint — B0 Image Path Beta

- B0 product/test commit: `bcba84d` (`test: prove native image path beta
  feasibility`).
- A generated 2×2 RGBA PNG round-trips byte-for-byte through existing
  `png 0.17`; truncated input is rejected without panic and no dependency was
  added.
- A synthetic local `HostPlacement` proves stable content/placement identity,
  RGBA upload, display, full-frame deduplication, view redisplay, content
  replacement, placement removal, and superseded-image cleanup through the
  existing `kitty_graphics` lifecycle.
- `paint_local_pane_graphics` now uses the private `frame_graphics_bytes` seam;
  behavior remains cursor save + existing Kitty bytes + cursor restore.
- The ignored probe rendered the four-color/alpha pattern in an isolated Kitty
  0.46.2 X11 window with throwaway XDG and cleared inherited Herdr socket
  variables. A separate Yazi 26.5.6 Path Alpha baseline rendered the source
  image in another throwaway window. Test windows were closed with targeted
  semantic `q`; no stable Herdr process/socket was touched.
- B2 decision: conditional GO. Reuse the existing encoder/cache; require
  bounded decode/allocation, state-refresh-only I/O, selection generation
  safety, cleanup on every transition, and fresh real-host evidence.

## Fresh B0 Verification Evidence

- Path Beta targeted: 4/4 passed; `kitty_graphics` scope: 25/25 passed.
- Full nextest final run: 2916/2916 passed, one intentionally ignored
  real-host probe skipped, no retry.
- `cargo fmt --check`: passed.
- Linux `cargo clippy --all-targets --locked -- -D warnings`: passed.
- Canonical Windows MSVC binary-target clippy with
  `LIBGHOSTTY_VT_SIMD=false`: passed. A stronger exploratory `--all-targets`
  command is not an applicable repo gate because Unix-only integration tests
  import `std::os::unix`/Unix `libc`; the Justfile intentionally uses `--bin
  herdr`.
- Bun integration assets: 5/5; plugin marketplace: 12/12.
- Python maintenance: 64/64.
- `git diff --check`: passed.
- Doctest probe reported no library target, so doctests are not applicable to
  this binary-only package.
- `just` is absent; every applicable `just check` command was run directly.

## Graph and Publication Evidence

- `mcp-proxy.service` readiness was repaired without killing/restarting a user
  process. A 26-server cold start measured 54 seconds; the bounded readiness
  budget is now 120 seconds inside a 150-second systemd start budget.
- Readiness passed with `expected=26 observed=26 critical_tools=14`.
- Full post-B1 graph reindex completed at 17,774 nodes / 84,033 edges.
- `miller_layout`, `highlight_text_preview`, `sync_file_preview_worker`, and
  `render_file_preview` were found as current graph symbols with B1 call/test
  connections; freshness was not inferred from `ready` alone.
- `bcba84d` was fast-forward pushed sequentially to both CyPack
  `feat/native-fm` and fork `master`; both remote SHA checks matched local
  `bcba84d`. `upstream` was not pushed.

## Standing Git Authorization

- The user explicitly authorized autonomous commits and pushes for this
  project. Do not repeatedly ask for commit-message alignment.
- Still require targeted staging, lowercase conventional commits, fresh
  proportional verification, fast-forward ancestry, remote SHA verification,
  and CyPack fork-only writes. Never force and never push `upstream`.

## Exact Next Action

1. Begin A3 remainder with TP-A3.2-VIEWPORT RED. Follow the complete A3 test
   contract in `.codex/TASKS.md`: viewport/clamp, shared mouse hit geometry,
   click/double-click/wheel dispatch, and explicit v1 single-selection scope.
2. B2 follows A3 and remains bound by B0's conditional-GO constraints.

## Verified Checkpoint — B1 Text Preview

- B1.1 adds a 64 KiB hard-capped regular-file reader with four-byte UTF-8
  sentinel, exact CRLF/content preservation, explicit truncation metadata, and
  stable missing/permission/non-regular/binary/invalid-UTF-8 states.
- `FmState` prepares content outside render, binds it to `source_path`, and
  preserves a highlight across reload only when path, visible bytes, and
  truncation identity all match.
- B1.2 uses `syntect 5.3.0` with default features disabled and only
  `default-syntaxes`, `default-themes`, and pure-Rust `regex-fancy` enabled.
- Measured synchronous highlighting is too slow for the input/render path
  (64 sample lines: ~460 ms debug / ~40 ms release), so B1.2 requires a
  generation-safe dedicated worker with one active and one replaceable pending
  request. Stale navigation/reload/close generations are rejected; worker
  failure degrades to plain text without App panic or dirty-loop.
- Highlighting and render each cap at 128 lines; Ratatui clips rendered columns.
  Prepared RGB/bold/italic/underline spans map to terminal styles; plain text,
  empty/error, and byte/line truncation states remain explicit.
- Actual lock delta is five packages and no existing-version upgrade:
  `syntect 5.3.0`, `fancy-regex 0.16.2`, `bit-set 0.8.0`, `bit-vec 0.8.0`, and
  `bincode 1.3.3`. Exact OSV rerun found only severity-less
  `RUSTSEC-2025-0141` for unmaintained bincode, with no patched version or new
  security-severity advisory.

## Fresh B1 Verification Evidence

- B1/FM targeted: 64/64 passed.
- Final full nextest: 2948/2948 passed; one named ignored B0 real-host probe
  skipped; no retry.
- `cargo fmt --check`: passed.
- Linux all-target clippy and canonical Windows MSVC bin clippy: passed with
  `-D warnings`.
- Bun integration assets 5/5 plus plugin marketplace 12/12; Python maintenance
  64/64.
- Metadata has only bin/custom-build/test targets, so doctests are N/A.
- `git diff --check`: passed for product paths; `just` is absent, so every
  applicable `just check` command was executed directly.
