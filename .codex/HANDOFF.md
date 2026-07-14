# SESSION HANDOFF — Herdr Native FM — 2026-07-14

## 1. SONRAKI ADIM

Make TP-N4.2-BULK-AUTHORITY RED before adding bulk toolbar authority. Define
zero/one/many selection counts, mixed supported/unsupported or stale members,
read-only destinations, clipboard state, selection clear/select-all bounds,
range limits, and operation-in-flight precedence. N4.1 selection must not grant
real filesystem authority; C4 still owns operation-time TOCTOU and failures.

## 2. AKTİF PROJE

- Project: `/home/ayaz/projects/herdr`
- Branch: `feat/native-fm`
- Goal: native, Lua-free file manager on Herdr's composable TUI foundation.
- Acting GitHub identity: `CyPack` external contributor; fork-only writes.

## 3. KAYNAK OTURUM

- Claude resume ID: `f53c720f-f795-4778-970b-d227714ffb1a`
- Raw JSONL: `/home/ayaz/.claude/projects/-home-ayaz-projects-herdr/f53c720f-f795-4778-970b-d227714ffb1a.jsonl`
- SHA-256: `368fb0a5045d1435c64679c8d0dea2a4283d58891231c91bb6e30350b69c2d30`
- Span: `2026-07-14T00:19:57Z`–`02:41:40Z`
- Reconstruction: `.codex/evidence/claude-session-f53c720f.md`

## 4. CLAUDE OTURUMUNDA TAMAMLANANLAR

1. Onboarding, rust-dev lessons, codebase graph freshness audit, task restoration.
2. S2 named region extraction, commit `c043c1e`.
3. Isolated-development test documentation and Claude semantic hook.
4. S3 re-scope and A2.1 center-region directory render, commit `d026e94`.
5. `prefix+f` activation across the two-layer keybinding system, commit `74d3cc9`.
6. Keyboard navigation/input interception, commit `d2b27e6`.
7. Detailed native-FM next-session prompt generated.

## 5. BU CODEX OTURUMUNDA TAMAMLANANLAR

- Recovered and verified the Claude transcript directly from local storage.
- Implemented A2.2 responsive Miller columns with cached parent/preview state.
- Added RED tests first, then achieved full GREEN verification.
- Built this Codex CLI continuity, memory, skill, hook, launcher, and handoff package.
- Committed A2.2 as `6c7c58f`, reindexed it, and fast-forward pushed only the
  CyPack feature branch and fork master.
- Implemented A4 native file watching test-point-first: pure normalization,
  generation/lifecycle safety, bounded channel/coalescing, path-preserving
  refresh, real-filesystem convergence, explicit polling fallback, and a
  2-second reconciliation safety-net.
- Made two pre-existing wall-clock-sensitive tests deterministic after full
  nextest exposed them under parallel load.
- Committed A4 separately as `01ba91d` and the deterministic test-only fixes
  separately as `8cd4e89`, using targeted staging only.
- Completed B0 Image Path Beta test-point-first with generated exact RGBA,
  malformed decode, synthetic local placement, upload/display/dedup/view/
  replacement/removal lifecycle, cursor framing, and isolated real-host tests.
- Captured a visible local Path Beta pattern in throwaway Kitty and an
  independent Path Alpha Yazi preview baseline; closed only the test windows
  with targeted semantic input.
- Recorded a conditional GO for B2: reuse existing `kitty_graphics`, bound all
  decode/allocation work, keep I/O outside render, reject stale generations,
  and prove cleanup plus real-host output.
- Committed B0 separately as `bcba84d`, full-reindexed it, and fast-forward
  published only to CyPack feature/master.
- Closed B1.0 dependency research: minimal pure-Rust `syntect 5.3.0` is the
  B1.2 choice, but measured latency requires generation-safe bounded background
  preparation. B1.1 begins without adding it.
- Completed B1 text preview through strict RED/GREEN commits: 64 KiB bounded
  UTF-8 reader, explicit failures, state-refresh ownership, pure deterministic
  syntax preparation, one-active/one-pending generation-safe worker, reload
  identity preservation, lifecycle rejection, Ratatui style mapping, explicit
  error/truncation states, and bounded responsive render.
- Proved cursor navigation, watcher reload, selected-file replace/delete,
  hidden toggle, close/reopen, worker panic/disconnect, narrow/zero geometry,
  long lines, and stale-result rejection without touching stable Herdr.
- Re-ran the actual five-package dependency/OSV delta and the entire direct
  `just check` equivalent. B1 product/test head is `2b2dcd3`; continuity and
  graph/publication follow separately.
- Completed A3 remainder as seven atomic RED/GREEN/scope commits from
  `d713b71` through `9d69c82`: persistent cursor viewport, shared responsive
  CURRENT-row geometry, single/double-click and bounded wheel runtime routing,
  stale-target rejection, preview refresh, and explicit cursor-only v1 scope.
- Ran the full A3 gate and isolated real PTY SGR-mouse cross-check without
  stable Herdr/socket access; exited semantically and removed all throwaway
  state. Full-reindexed and published `9d69c82` to CyPack feature/master only.
- Completed B2 as an auditable range from dependency decision `de1eff5`
  through fallback fix `2989434`: bounded common-format decode/downscale,
  responsive client-local placement, generation-safe worker lifecycle,
  cached Kitty paint/dedup/transition cleanup, and explicit non-Kitty fallback.
- Ran B2/FM/Kitty 96/96, full nextest 2983/2983 plus one named B0 host-probe
  skip, Linux/Windows clippy, Bun 17/17, Python 64/64, fmt, and diff-check.
- Proved the production FM path in isolated Kitty: `assets/logo.png` rendered
  in PREVIEW with 0/271425 image-compare pixel differences, FM close cleared
  the same region to one background color, semantic exit returned code 0, and
  no test process/socket/temp root remained. Stable Herdr was untouched.
- Completed C1.1 test-first: RED contract `0ed5e51`, GREEN product `c9bfbf9`.
  Added client-local Copy/Paste/NewFolder/Delete tags, one responsive pure
  geometry seam, desktop/mobile `ViewState` snapshots, complete-button hiding,
  stale-area clearing, and render consumption without filesystem work.
- Full nextest exposed a pre-existing lifecycle fixture clock race. Proved the
  mixed real/synthetic timing boundary, fixed only the test base clock, and
  committed it separately as `9aa1e59`; no C1 product code was mixed into the
  stability commit.
- Ran the complete direct `just check` equivalent at the C1.1 tip: targeted
  4/4, lifecycle family 27/27, full nextest 2986/2986 plus one named B0 probe
  skip, Linux/Windows clippy, Bun 17/17, Python 64/64, fmt, and diff-check.
- Completed C1.2 test-first: RED `dbc6798`, GREEN `7fd01de`. Added a private
  `NotHandled`/`Consumed`/`HeaderAction(tag)` dispatch seam, exact unmodified
  left-click tag mapping, and fail-closed identity/gap/outside/hidden/zero/
  stale/non-left behavior without AppState/protocol/filesystem mutation.
- Ran exact dispatch 2/2, all FM input 13/13, full nextest 2988/2988 plus one
  named B0 probe skip, Linux/Windows clippy, Bun 17/17, Python 64/64, fmt, and
  diff-check. Cwd, cursor, and fixture disk entries remain unchanged by header
  tag dispatch.
- Completed N3.1 test-first: RED `b5cc95c`, GREEN `510eebc`. Added a pure
  selection/clipboard action-bar ViewState model, client-local clipboard paths,
  selected file/directory identity, explicit empty content, desktop/mobile
  refresh, and render fallback without filesystem or protocol coupling.
- Proved navigation, reload-selected-delete, empty transition, close/reopen,
  selected-name rendering, and clipboard-summary persistence. Gates: 3/3,
  FM 135/135, full 2991/2991 plus one named B0 skip, Linux/Windows clippy,
  Bun 17/17, Python 64/64, fmt/diff clean.
- Completed N3.2 test-first: RED `446613a`, GREEN `267ad91`. Added explicit
  per-action enabled/disabled authority and reasons, prepared cwd writability
  and regular-target support, distinct disabled rendering, and fail-closed
  input dispatch with no disabled-click state/filesystem mutation.
- Proved missing cwd, read-only reload, Unix special target, empty clipboard,
  absent selection, unsupported target, and in-flight precedence. Gates:
  exact authority/preparation/render/dispatch 7/7, broad FM/input/render/Kitty
  165/165, full 2996/2996 plus one named B0 skip, Linux/Windows clippy, Bun
  17/17, Python 64/64, fmt/diff clean.
- Completed C2.1 test-first: RED `d4d404e`, GREEN `9a15328`. Added client-local
  SendAgent/Rename/Delete row tags, one pure responsive name/action geometry
  snapshot, desktop/mobile ViewState lifecycle, and pure render consumption.
- The first 3-cell-per-action prototype passed focused tests but failed two
  broad readability characterizations by truncating ordinary names. Reworked
  complete buttons to one-cell `>`, `r`, and `x` targets; the focused plus
  readability set passed 8/8 and the FM impact set passed 71/71.
- Ran the complete direct `just check` equivalent at the C2.1 tip: full
  nextest 2998/2998 plus one named B0 probe skip, Linux all-target and canonical
  Windows MSVC bin clippy, Bun 17/17, Python 64/64, fmt, and diff-check clean.
  Fast graph reindex is fresh at 18,042 nodes / 84,123 edges and returns the
  new geometry/action symbols; freshness was not inferred from `ready` alone.
- Completed C2.2 test-first: RED `94e4a02`, GREEN `9ef90c6`. Row action
  snapshots now carry stable path identity, and exact unmodified-left dispatch
  requires the live index to match that path and remain operation-supported.
- Proved exact SendAgent/Rename/Delete tags, unchanged name selection,
  non-left/modifier/outside/hidden/closed fail-closed behavior, watcher-style
  reorder rejection, unsupported-target rejection, and zero cwd/cursor/
  clipboard/filesystem side effects. The outer router consumes tags before
  hidden terminal input but deliberately executes no real operation.
- C2.2 gates: exact 3/3, all FM input 17/17, FM impact 74/74, full nextest
  3001/3001 plus one named B0 probe skip, Linux/Windows clippy, Bun 17/17,
  Python 64/64, fmt/diff clean. Fast graph reindex is fresh at 18,049 nodes /
  83,839 edges and returned the current path-bearing area plus validation
  handler snippets; freshness was not inferred from `ready` alone.
- Completed N4.1 test-first as seven atomic commits: state `e876223`/`590e376`,
  lifecycle `1789bbd`/`5c14439`, gesture/render RED `699a6a6`, stable row
  identity RED `fc19237`, and integrated GREEN `86b618a`.
- Added a cursor-independent deduplicated path set and stable anchor, current-
  order inclusive range selection, reload/hidden pruning, enter/leave clearing,
  close/reopen reset, exact plain/Ctrl/Shift mouse gestures, Space and
  Shift+Up/Down keyboard equivalents, and distinct pure multi-row rendering.
- `FileManagerRowArea` now carries stable path identity; same-index watcher
  reorder is consumed without selecting the wrong live entry. Combined or
  unknown modifiers and stale targets fail closed. N4.1 performs no filesystem
  operation and adds no server or wire-protocol state.
- N4.1 gates: focused 7/7, broad FM/watcher/input/render/Kitty 137/137, full
  nextest 3015/3015 plus one named B0 probe skip, Linux/Windows clippy, Bun
  17/17, Python 64/64, fmt/diff clean. Fast graph reindex is fresh at 18,078
  nodes / 83,865 edges and returned live model/input/render connections.

## 6. KOD DURUMU

Previously published product/test history through `bcba84d`:

- `c043c1e`: `src/ui/shell.rs`, `src/ui.rs`, `src/app/state.rs`, `src/app/mod.rs`.
- `d026e94`: `src/ui/file_manager.rs`, `src/ui.rs`, `src/app/state.rs`, `src/app/mod.rs`, `src/main.rs`.
- `74d3cc9`: `src/app/actions.rs`, `src/app/input/navigate.rs`, `src/config/keybinds.rs`, `src/config/model.rs`, `src/ui/keybind_help.rs`.
- `d2b27e6`: `src/app/input/file_manager.rs`, `src/app/input/mod.rs`, `src/fm/mod.rs`, `src/main.rs`.
- `6c7c58f`: `src/fm/mod.rs`, `src/ui/file_manager.rs`.
- `01ba91d`: `Cargo.toml`, `Cargo.lock`,
  `src/app/file_manager_watcher.rs`, `src/app/mod.rs`, `src/app/runtime.rs`,
  `src/fm/watcher.rs`, `src/fm/mod.rs`.
- `8cd4e89`: `src/server/headless.rs`, `src/terminal/state.rs`.
- `bcba84d`: `src/kitty_graphics.rs`.

B1 is an auditable 21-commit test/feature sequence from `439ff2c` through
`2b2dcd3`. Product paths are `Cargo.toml`, `Cargo.lock`,
`src/app/file_preview_worker.rs`, `src/app/mod.rs`, `src/app/runtime.rs`,
`src/fm/mod.rs`, `src/fm/text_preview.rs`, and `src/ui/file_manager.rs`.
Intermediate RED commits are intentional TDD checkpoints; the range tip is
fully green. Continuity/task commit `a0f82a3` and the complete B1 range were
fast-forward published to both CyPack `feat/native-fm` and fork `master`; exact
remote SHAs matched. Upstream was not pushed.

A3 is an auditable seven-commit sequence:

- `d713b71` / `027c364`: viewport RED/GREEN.
- `1fea0e7` / `6727342`: shared hit-geometry RED/GREEN.
- `3aa267b` / `33b585a`: runtime mouse dispatch RED/GREEN.
- `9d69c82`: cursor-only v1 selection-scope characterization.

All seven commits are published to both CyPack `feat/native-fm` and fork
`master`; upstream was not pushed.

B2 product/test history is an auditable sequence:

- `de1eff5`: dependency decision/evidence.
- `be200b9` / `e24cda8`: bounded decoder RED/GREEN.
- `054708f` / `983b6b3`: client-local placement RED/GREEN.
- `cf40a06` / `11f26eb`: generation-safe worker lifecycle RED/GREEN.
- `5c51f1a` / `c6b4762`: cached paint/cleanup RED/GREEN.
- `2989434`: width-safe fallback characterization and fix.

The range is fully green. The continuity/graph commits containing this handoff
are part of the publication unit; both CyPack heads are verified at that same
fast-forward branch tip. Upstream is not pushed.

C1.1 history is an auditable test/product pair plus an independent stability
fix:

- `0ed5e51`: compile-failing RED header geometry/ViewState contract.
- `9aa1e59`: deterministic process-generation suppression fixture.
- `c9bfbf9`: responsive header action geometry and pure render/ViewState seam.

The intermediate RED commit was never pushed alone. The publication unit adds
this continuity/graph commit and fast-forwards both CyPack heads only after all
fresh gates pass. Upstream is not pushed.

C1.2 is an auditable RED/GREEN pair:

- `dbc6798`: compile-failing exact-tag and fail-closed dispatch contract.
- `7fd01de`: private side-effect-free header action tag dispatch.

The intermediate RED commit was never pushed alone. This continuity/graph
commit completes the C1.2 publication unit.

N3.1 is an auditable RED/GREEN pair:

- `b5cc95c`: compile-failing selection/clipboard/lifecycle/render contract.
- `510eebc`: pure selection-sensitive persistent action-bar content.

The intermediate RED commit was never pushed alone. This continuity/graph
commit completes the N3.1 publication unit.

N3.2 is an auditable RED/GREEN pair:

- `446613a`: compile-failing explicit action-authority contract.
- `267ad91`: prepared fail-closed authority, disabled render, and input gate.

The intermediate RED commit was never pushed alone. This continuity/graph
commit completes the N3.2 publication unit.

C2.1 is an auditable RED/GREEN pair:

- `d4d404e`: compile-failing row action geometry/render/ViewState contract.
- `9a15328`: pure responsive row action geometry and ViewState/render seam.

The intermediate RED commit was never pushed alone. This continuity/graph
commit completes the C2.1 publication unit.

C2.2 is an auditable RED/GREEN pair:

- `94e4a02`: compile-failing stable-path row dispatch contract.
- `9ef90c6`: exact fail-closed path-and-tag dispatch before terminal input.

The intermediate RED commit was never pushed alone. This continuity/graph
commit completes the C2.2 publication unit.

N4.1 is an auditable seven-commit sequence:

- `e876223` / `590e376`: selection state RED/GREEN.
- `1789bbd` / `5c14439`: lifecycle reconcile RED/GREEN.
- `699a6a6`: gesture and render RED.
- `fc19237`: stable row identity RED.
- `86b618a`: exact input, stable identity, and pure visual projection GREEN.

The compile-failing RED checkpoints were never pushed alone. This continuity/
graph commit completes the N4.1 publication unit.

## 7. TEST KANITI

- B1/FM targeted: 64/64.
- Final full nextest: 2948/2948 passed, one explicit B0 interactive host probe
  skipped, no retry.
- Linux all-target and canonical Windows MSVC binary-target clippy passed with
  `-D warnings`.
- Bun 17/17; Python maintenance 64/64; fmt and diff-check clean.
- Doctests are N/A because Herdr has no library target.
- Actual B1 lock delta is five packages with no existing-version upgrade.
  Exact OSV batch returned only severity-less `RUSTSEC-2025-0141` for
  unmaintained `bincode 1.3.3`; no security-severity advisory.
- A3 targeted broad regression: 164/164; scope: 4/4.
- Final A3 full nextest: 2966/2966, one named B0 host probe skipped, no retry.
- Linux/Windows clippy, Bun 17/17, Python 64/64, fmt/diff clean.
- Isolated `--no-session` PTY proved three columns, cursor click, directory
  double-click enter, long-list wheel down/up viewport clamp, semantic exit 0,
  and zero temp/process residue.
- B2/FM/Kitty targeted: 96/96; full nextest: 2983/2983 plus one named B0
  interactive probe skip; no retry.
- B2 Linux all-target and canonical Windows MSVC bin clippy passed with
  `-D warnings`; Bun 17/17; Python 64/64; fmt/diff clean.
- B2 isolated Kitty source-to-host comparison: 0/271425 pixel difference.
  Closing FM removed the image from the host, and semantic exit left zero
  test process/socket/temp residue.
- C1.1 geometry/render/ViewState targeted: 4/4; suppression/process-exit/stale
  lifecycle family: 27/27; final full nextest: 2986/2986 plus one named B0
  interactive probe skip, no retry-only closure.
- C1.1 Linux all-target and canonical Windows MSVC bin clippy passed with
  `-D warnings`; Bun 17/17; Python maintenance 64/64; fmt/diff clean.
- C1.2 exact dispatch: 2/2; full FM input: 13/13; final full nextest:
  2988/2988 plus one named B0 interactive probe skip, no retry-only closure.
- C1.2 Linux all-target and canonical Windows MSVC bin clippy passed with
  `-D warnings`; Bun 17/17; Python maintenance 64/64; fmt/diff clean.
- N3.1 targeted: 3/3; FM regression: 135/135; final full nextest: 2991/2991
  plus one named B0 interactive probe skip, no retry-only closure.
- N3.1 Linux all-target and canonical Windows MSVC bin clippy passed with
  `-D warnings`; Bun 17/17; Python maintenance 64/64; fmt/diff clean.
- N3.2 exact authority/preparation/render/dispatch: 7/7; broad FM/input/render/
  Kitty regression: 165/165; final full nextest: 2996/2996 plus one named B0
  interactive probe skip, no retry-only closure.
- N3.2 Linux all-target and canonical Windows MSVC bin clippy passed with
  `-D warnings`; Bun 17/17; Python maintenance 64/64; fmt/diff clean.
- C2.1 focused invariant/readability set: 8/8; FM impact: 71/71; final full
  nextest: 2998/2998 plus one named B0 interactive probe skip, no retry-only
  closure. Linux/Windows clippy, Bun 17/17, Python 64/64, fmt/diff clean.
- C2.2 exact dispatch/stale/no-side-effect: 3/3; all FM input: 17/17; FM impact:
  74/74; final full nextest: 3001/3001 plus one named B0 interactive probe
  skip, no retry-only closure. Linux/Windows clippy, Bun 17/17, Python 64/64,
  fmt/diff clean.
- N4.1 focused input/render: 7/7; broad FM/watcher/input/render/Kitty: 137/137;
  final full nextest 3015/3015 plus one named B0 interactive probe skip, no
  retry-only closure. Linux/Windows clippy, Bun 17/17, Python 64/64, fmt/diff
  clean.

## 8. KRİTİK KARARLAR

- Pure render is non-negotiable.
- S3 registry deferred to S5; use concrete content swap until abstraction is earned.
- S2 persistence deferred to S6.
- A2.2 caches parent/preview in `FmState`.
- A4 uses stable `notify-debouncer-full 0.7.0` / `notify 8.2.0`.
- Native watching is primary; startup/runtime errors enter explicit polling
  fallback, and all active watchers reconcile every 2 seconds to cover silent
  FUSE/NFS/exFAT-class delivery failures.
- A4, B0, B1, A3, and B2 are implementation-complete, fully verified,
  graph-indexed, and published to the CyPack fork.
- C1, N3, and C2 are implementation-complete, fully verified, and
  graph-indexed.
  N3.2 supplies explicit selection/clipboard/target/in-flight authority and
  disabled-click no-side-effect behavior. C4 must still revalidate every real
  filesystem operation at execution time for TOCTOU and partial failure.
- N4.1 is implementation-complete, fully verified, and graph-indexed. Its
  path set and anchor are client-local selection facts only; N4.2 must derive
  explicit bulk authority before C4 can execute any operation.
- B1 uses minimal pure-Rust syntect outside input/render in a dedicated bounded
  worker. Plain prepared content remains availability authority; highlighting
  is optional enhancement and stale generations never mutate current state.
- B0's conditional B2 GO is satisfied: B2 retains bounded decode, generation,
  cleanup, and real-host evidence constraints.
- The user granted standing authorization for autonomous atomic commits and
  CyPack fork-only fast-forward pushes. Do not repeatedly ask for alignment;
  never relax targeted staging, verification, ancestry, or remote-SHA checks.

## 9. GÜVENLİK

- Never kill user processes.
- Never touch `/home/ayaz/.local/bin/herdr` or the stable socket.
- Clear inherited socket variables and use throwaway XDG directories for runtime tests.
- Never stage ignored `.local` files into product commits.
- Never push `upstream`.

## 10. AÇIK GÖREVLER

See `.codex/TASKS.md` for the completed A3/B2/C1/N3/C2/N4.1 contracts and the
complete N4.2/C3–C6, S5–S7, N2, and M1–M3 roadmap. A4, B0, B1, A3, B2, C1,
N3, C2, and N4.1 are closed. The immediate product task is
TP-N4.2-BULK-AUTHORITY RED; then follow N4.2 → C3 → C4 → C5 → C6 without
skipping modules.

## 11. ORTAM

- `codex-cli 0.144.1` is installed.
- `just` is absent; direct recipe execution is required unless installed later.
- Full post-N3.1 graph reindex completed at 18,009 nodes / 83,964 edges.
  Freshness query returned current action-bar model/selection/kind types and
  `compute_file_manager_action_bar_model`; the builder is connected to
  desktop/mobile view computation, render fallback, and model tests. Lifecycle
  and render tests are present. Freshness was not inferred from `ready` alone.
- Full post-N3.2 graph reindex completed at 18,026 nodes / 84,120 edges.
  Freshness queries returned current `miller_layout`, action state, authority
  builder, entry-capability preparation, and mouse handler symbols. The pure
  builder is connected to desktop/mobile view computation, render fallback,
  and authority tests. Freshness was not inferred from `ready` alone.
- Fast post-C2.1 graph reindex completed at 18,042 nodes / 84,123 edges.
  Freshness queries returned `compute_file_manager_row_geometry`,
  `FileManagerRowAction`, and `FileManagerRowActionArea` from their current
  source files. Freshness was not inferred from `ready` alone.
- Fast post-C2.2 graph reindex completed at 18,049 nodes / 83,839 edges.
  Freshness queries and snippets returned the current path-bearing
  `FileManagerRowActionArea` and live path/support validation in
  `handle_file_manager_mouse`. Freshness was not inferred from `ready` alone.
- Fast post-N4.1 graph reindex completed at 18,078 nodes / 83,865 edges.
  Freshness queries returned `replace_selection`, `toggle_selection`,
  `extend_selection`, `reconcile_multi_selection`, stable row identity input,
  and the multi-selection render test with live connections. Freshness was not
  inferred from `ready` alone.
- `mcp-proxy.service` cold start measured 54 seconds for 26 servers. Readiness
  now has a 120-second internal and 150-second systemd budget; live proof was
  `expected=26 observed=26 critical_tools=14`.
- `~/.codex/skills/rust-dev` points to the canonical Claude `rust-dev` skill; parity reports no broken skill links.
- Global Codex hooks support SessionStart and UserPromptSubmit; Herdr context routing is scoped to this repo.

## 12. BAŞLATMA

Run:

```bash
herdr-codex
```

The new agent must read `AGENTS.md`, `CLAUDE.md`, `.codex/BOOTSTRAP.md`, `.codex/CURRENT.md`, and `.codex/TASKS.md`, then verify graph and Git state before acting.
