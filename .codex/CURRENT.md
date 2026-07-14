# Current State — 2026-07-14

## Repository

- Path: `/home/ayaz/projects/herdr`
- Branch: `feat/native-fm`
- Active C1.2 product checkpoint: `7fd01de`
  (`feat: dispatch file manager header action tags`).
- The C1.2 publication unit is the RED/GREEN product/test pair and the
  continuity/graph commit containing this file. At publication, CyPack
  `feat/native-fm` and fork `master` are verified at that same fast-forward
  branch tip.
- `origin` is the `CyPack/herdr` fork. `upstream` is `ogulcancelik/herdr` and must never be pushed.

## Completed Checkpoint

- A2.2 responsive Miller columns were committed as `6c7c58f`, full graph-indexed,
  and fast-forward pushed to the CyPack feature branch and fork master only.

## Verified Checkpoint — C1.1 Header Action Geometry

- RED contract commit: `0ed5e51` (`test: define file manager header action
  geometry`). GREEN product commit: `c9bfbf9` (`feat: add file manager header
  action geometry`). Intermediate compile-failing RED was never pushed alone.
- `FileManagerHeaderAction` names Copy, Paste, NewFolder, and Delete as
  client-local presentation/input tags; no server or wire-protocol state was
  added.
- One pure geometry seam produces complete, disjoint, priority-ordered,
  right-aligned button rectangles while reserving a readable cwd identity
  width. Narrow layouts progressively hide whole lower-priority actions;
  zero/degenerate areas fail closed.
- Desktop/mobile `compute_view` snapshot the same rectangles into `ViewState`;
  render reads that snapshot and uses the same pure fallback for isolated
  component tests. Closing FM clears the areas. Render performs no mutation or
  filesystem work.
- Full nextest exposed a pre-existing 1–4 ms synthetic/real clock race in the
  multiple-process-generation suppression fixture. Root cause was proven and
  the test-only base clock was moved one second ahead in separate commit
  `9aa1e59`; the exact test and 27-test lifecycle family passed before the full
  suite.
- Fresh gates: C1 geometry/render/ViewState 4/4; full nextest 2986/2986 with
  one named B0 host probe skipped; Linux all-target and canonical Windows bin
  clippy clean with `-D warnings`; Bun 17/17; Python 64/64; fmt/diff clean.

## Verified Checkpoint — C1.2 Header Action Dispatch

- RED contract commit: `dbc6798` (`test: define file manager header action
  dispatch`). GREEN product commit: `7fd01de` (`feat: dispatch file manager
  header action tags`). Intermediate compile-failing RED was never pushed
  alone.
- Private `FileManagerMouseDispatch` distinguishes `NotHandled`, `Consumed`,
  and `HeaderAction(tag)` without adding App/AppState fields or server/wire
  state. The outer mouse router consumes visible header tags before the hidden
  terminal path.
- Only an unmodified left press inside a current named rectangle returns its
  exact Copy/Paste/NewFolder/Delete tag. Identity/gap/outside/hidden/zero,
  stale-closed-FM, right/middle, and modified-left paths cannot invent an
  action.
- C1.2 intentionally executes no filesystem mutation and preserves cwd,
  cursor, and disk entries. N3 must provide explicit selection-sensitive
  content and enablement authority before any action tag can cause a side
  effect.
- Fresh gates: exact dispatch 2/2; full FM input 13/13; full nextest 2988/2988
  with one named B0 host probe skipped; Linux all-target and canonical Windows
  bin clippy clean with `-D warnings`; Bun 17/17; Python 64/64; fmt/diff clean.

## Completed Checkpoint — B2 Native Image Preview

- B2 is an auditable dependency decision plus four RED/GREEN increments and a
  fallback fix from `de1eff5` through `2989434`.
- `image 0.25.10` is restricted to `png/jpeg/gif/webp`. Encoded bytes,
  dimensions, checked pixels, decoder allocation, RGBA output, and target
  placement are independently bounded before untrusted allocation can grow.
- Decode/downscale supports PNG, JPEG, GIF, and WebP; preserves alpha; applies
  orientation-aware aspect fit without upscaling; and maps corrupt,
  unsupported, oversized, non-regular, missing, and decoder-panic paths to
  explicit states.
- A dedicated generation-safe worker owns filesystem/decode work outside
  render. Path, model generation, and pixel target must all match before a
  result can publish; navigation, watcher reload, resize, close/reopen, and
  worker panic cannot paint stale pixels.
- The client-local FM preview uses the existing responsive preview geometry,
  synthetic local placement identity, and existing Kitty encoder/cache. It
  uploads once, repositions without re-upload, replaces and deletes
  superseded content, clears on terminal/FM surface transitions, and leaves
  generic terminal-image reuse semantics intact.
- Non-Kitty hosts get the width-safe `(Kitty graphics req.)` fallback. Ready
  Kitty frames have no text underlay.

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
- Full post-C1.2 graph reindex completed at 17,993 nodes / 84,009 edges.
  `FileManagerMouseDispatch` and `handle_file_manager_mouse` are current graph
  symbols; the handler remains connected to the outer input module, and both
  exact-tag/fail-closed tests are present with their fixture calls. Freshness
  was not inferred from `ready` alone.
- Publication uses sequential fast-forward pushes to both CyPack heads and
  exact remote-SHA equality. `upstream` is never pushed.

## Standing Git Authorization

- The user explicitly authorized autonomous commits and pushes for this
  project. Do not repeatedly ask for commit-message alignment.
- Still require targeted staging, lowercase conventional commits, fresh
  proportional verification, fast-forward ancestry, remote SHA verification,
  and CyPack fork-only writes. Never force and never push `upstream`.

## Exact Next Action

1. Begin TP-N3.1-CONTENT RED: define a pure, selection-sensitive persistent
   action-bar model for directory/file/empty selection, clipboard state, and
   watcher/navigation/close transitions. Render must stay filesystem-free and
   stale selection state must clear. Then TP-N3.2 must prove explicit disabled
   actions dispatch no side effect before C2 begins.

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
- `image::Limits::max_alloc` is best-effort, so TP-B2.1 additionally hard-
  bounds input bytes, dimensions, checked pixels, decoder total bytes, RGBA
  output, and target placement allocation. Full evidence is in
  `.codex/evidence/b2-image-dependency.md`.

## Fresh B2 Verification Evidence

- B2/FM/Kitty targeted expression: 96/96 passed.
- Full nextest: 2983/2983 passed; one named B0 interactive real-host probe was
  skipped; no fail or retry.
- `cargo fmt --check`, Linux all-target clippy, and canonical Windows MSVC
  binary-target clippy passed with `-D warnings`.
- Bun integration assets 5/5 plus plugin marketplace 12/12; Python maintenance
  64/64; diff-check clean. `just` is absent, so every applicable `just check`
  command was executed directly.
- Isolated Kitty X11 used a unique throwaway XDG root, cleared socket and
  session identity variables, `experimental.kitty_graphics=true`, and a
  workspace rooted at `assets/`. Selecting `logo.png` produced a 517×525 host
  preview whose resized source comparison was exactly 0/271425 pixels
  different. Closing FM returned the same region to one background color.
  `prefix+q` exited semantically; the test process, sockets, and temp root were
  absent afterward. Stable Herdr and its socket were untouched.

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
