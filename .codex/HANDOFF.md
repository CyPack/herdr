# SESSION HANDOFF — Herdr Native FM — 2026-07-14

## 1. SONRAKI ADIM

Make TP-B2.1-DECODE RED before manifest or production code changes. Then add
the B2.0-selected common-format dependency and execute decode/placement/
lifecycle/paint/real-host test points in `.codex/TASKS.md` sequentially.

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

## 8. KRİTİK KARARLAR

- Pure render is non-negotiable.
- S3 registry deferred to S5; use concrete content swap until abstraction is earned.
- S2 persistence deferred to S6.
- A2.2 caches parent/preview in `FmState`.
- A4 uses stable `notify-debouncer-full 0.7.0` / `notify 8.2.0`.
- Native watching is primary; startup/runtime errors enter explicit polling
  fallback, and all active watchers reconcile every 2 seconds to cover silent
  FUSE/NFS/exFAT-class delivery failures.
- A4, B0, B1, and A3 are implementation-complete, fully verified,
  graph-indexed, and published to the CyPack fork through A3 product/test head
  `9d69c82`.
- B1 uses minimal pure-Rust syntect outside input/render in a dedicated bounded
  worker. Plain prepared content remains availability authority; highlighting
  is optional enhancement and stale generations never mutate current state.
- B0's B2 decision is conditional GO; B2 remains ordered after the A3
  remainder and must retain bounded decode, generation, cleanup, and real-host
  evidence constraints.
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

See `.codex/TASKS.md` for the completed A3 contract plus the now-expanded B2
test-point table, C1–C6, S5–S7, N2, and M1–M3 roadmap. A4, B0, B1, and A3 are
closed. B2.0 dependency evidence is complete; immediate task is
TP-B2.1-DECODE RED. Do not skip directly to image placement or painting.

## 11. ORTAM

- `codex-cli 0.144.1` is installed.
- `just` is absent; direct recipe execution is required unless installed later.
- Full post-A3 graph reindex completed at 17,818 nodes / 83,121 edges and
  returned `sync_viewport`, `compute_file_manager_row_areas`, and
  `handle_file_manager_mouse` with current connections; freshness was not
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
