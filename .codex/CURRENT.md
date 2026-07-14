# Current State — 2026-07-14

## Repository

- Path: `/home/ayaz/projects/herdr`
- Branch: `feat/native-fm`
- Published A3 product/test checkpoint: `9d69c82`
  (`test: lock cursor-only file manager selection`).
- CyPack `feat/native-fm` and fork `master` were both fast-forwarded from
  `9ce9eae` to `9d69c82`.
- `origin` is the `CyPack/herdr` fork. `upstream` is `ogulcancelik/herdr` and must never be pushed.

## Completed Checkpoint

- A2.2 responsive Miller columns were committed as `6c7c58f`, full graph-indexed,
  and fast-forward pushed to the CyPack feature branch and fork master only.

## Published Checkpoint — A3 Navigation Remainder

- A3 is an auditable seven-commit RED/GREEN sequence from `d713b71` through
  `9d69c82`; intermediate compile-failing RED commits were never pushed alone.
- `FmState.viewport_start` has explicit cursor-visible and clamp invariants for
  long lists, resize, reload shrink, zero-height, enter, and leave.
- `compute_file_manager_row_areas` is the shared responsive one/two/three-column
  CURRENT-row geometry consumed by render and snapshotted in `ViewState` for
  input. Degenerate geometry and stale indices fail closed.
- Real runtime mouse dispatch selects on single click, enters only a directory
  on same-path double-click, leaves files selected, bounds wheel navigation,
  refreshes preview state, and consumes center input before hidden panes.
- v1 intentionally has one cursor-owned visual selection. Multi-select and bulk
  authority remain deferred to N4/C2 and require their own RED tests.

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
- Full post-A3 graph reindex completed at 17,818 nodes / 83,121 edges.
- `sync_viewport`, `compute_file_manager_row_areas`, and
  `handle_file_manager_mouse` were found as current production graph symbols
  with their call/test connections; freshness was not inferred from `ready`
  alone.
- `9d69c82` was fast-forward pushed sequentially to both CyPack
  `feat/native-fm` and fork `master`. `upstream` was not pushed.

## Standing Git Authorization

- The user explicitly authorized autonomous commits and pushes for this
  project. Do not repeatedly ask for commit-message alignment.
- Still require targeted staging, lowercase conventional commits, fresh
  proportional verification, fast-forward ancestry, remote SHA verification,
  and CyPack fork-only writes. Never force and never push `upstream`.

## Exact Next Action

1. Make TP-B2.1-DECODE RED before changing the manifest or adding production
   decode/downscale code.
2. Add the selected dependency only when that RED test requires it, implement
   the bounded common-format decoder, then follow the remaining B2 test points
   in `.codex/TASKS.md` sequentially.

## Verified B2.0 Dependency Decision

- Selected `image 0.25.10` with default features disabled and only
  `png/jpeg/gif/webp`; default `image` was rejected because it adds 78 packages
  including unnecessary rayon/AVIF/EXR surfaces.
- Exact selected delta is 12 packages with no existing-version upgrade, no
  build script, and no proc macro. All license metadata is compatible.
- Package-registry advisories found only two historical `image` ranges, both
  fixed long before `0.25.10`; the other selected packages returned no result.
- Rust 1.96.1 Windows MSVC check passed. Three clean compile samples showed no
  material RSS/wall penalty versus `image` PNG-only; common formats add seven
  packages and about 2.43 MB of check artifacts.
- `image::Limits::max_alloc` is best-effort, so TP-B2.1 must additionally hard
  bound input bytes, dimensions, checked pixels, decoder total bytes, RGBA
  output, and target placement allocation. Full evidence is in
  `.codex/evidence/b2-image-dependency.md`.

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

## Fresh A3 Verification Evidence

- Targeted viewport/geometry/input/render regressions: 164/164 passed at the
  broadest A3-targeted run; dedicated scope tests: 4/4 passed.
- Final full nextest: 2966/2966 passed; one named ignored B0 real-host probe
  skipped; no retry.
- Linux all-target clippy and canonical Windows MSVC bin clippy passed with
  `-D warnings`.
- Bun integration assets 5/5 plus plugin marketplace 12/12; Python maintenance
  64/64; fmt and diff-check clean.
- Isolated real PTY used cleared Herdr socket/identity variables, throwaway XDG,
  and `--no-session`: three Miller columns rendered; single click changed the
  cursor; same-row directory double-click entered `/home/ayaz/2027 weeks`; 25
  wheel-down events moved the viewport to `WEEK_7…WEEK_27`; 40 wheel-up events
  returned to the top clamp. `q` then `prefix+q` exited with code 0; the unique
  temp tree and process were absent afterward.
- `just` is absent; every applicable `just check` command was run directly.
