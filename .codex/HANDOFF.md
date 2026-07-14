# SESSION HANDOFF — Herdr Native FM — 2026-07-14

## 1. SONRAKI ADIM

Execute B0 Image Path Beta test-point-first. Keep B2 blocked until B0 records
a go decision.

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

## 6. KOD DURUMU

Committed history through `8cd4e89`:

- `c043c1e`: `src/ui/shell.rs`, `src/ui.rs`, `src/app/state.rs`, `src/app/mod.rs`.
- `d026e94`: `src/ui/file_manager.rs`, `src/ui.rs`, `src/app/state.rs`, `src/app/mod.rs`, `src/main.rs`.
- `74d3cc9`: `src/app/actions.rs`, `src/app/input/navigate.rs`, `src/config/keybinds.rs`, `src/config/model.rs`, `src/ui/keybind_help.rs`.
- `d2b27e6`: `src/app/input/file_manager.rs`, `src/app/input/mod.rs`, `src/fm/mod.rs`, `src/main.rs`.
- `6c7c58f`: `src/fm/mod.rs`, `src/ui/file_manager.rs`.
- `01ba91d`: `Cargo.toml`, `Cargo.lock`,
  `src/app/file_manager_watcher.rs`, `src/app/mod.rs`, `src/app/runtime.rs`,
  `src/fm/watcher.rs`, `src/fm/mod.rs`.
- `8cd4e89`: `src/server/headless.rs`, `src/terminal/state.rs`.

`01ba91d` and `8cd4e89` are fast-forward published to
both CyPack `feat/native-fm` and fork `master`; both remote refs were verified
at local `8cd4e89`. The separate continuity/tooling commit containing this
handoff is published after them. Product staging is empty.

## 7. TEST KANITI

- A4 broad FM/file-manager filter: 69/69 passed.
- Final full nextest: 2912/2912 passed, 0 skipped, no retry in the final run.
- Linux all-target and Windows MSVC-target clippy passed with `-D warnings`.
- Bun 17/17; Python maintenance 64/64; fmt and diff-check clean.
- Exactly 22 new watcher/target packages; no existing version upgrades; the
  exact-version OSV batch returned zero advisories.
- Real filesystem create/rename/delete/16-file burst convergence passed under
  bounded deadlines. No stable Herdr process or socket was touched.

## 8. KRİTİK KARARLAR

- Pure render is non-negotiable.
- S3 registry deferred to S5; use concrete content swap until abstraction is earned.
- S2 persistence deferred to S6.
- A2.2 caches parent/preview in `FmState`.
- A4 uses stable `notify-debouncer-full 0.7.0` / `notify 8.2.0`.
- Native watching is primary; startup/runtime errors enter explicit polling
  fallback, and all active watchers reconcile every 2 seconds to cover silent
  FUSE/NFS/exFAT-class delivery failures.
- A4 is implementation-complete, verified, and published. B0 remains
  independent and mandatory before B2.
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

See `.codex/TASKS.md` for the full B0, B1, A3, B2, C1–C6, S5–S7, N2, and
M1–M3 roadmap. A4 and continuity publication are closed. B0 is active.

## 11. ORTAM

- `codex-cli 0.144.1` is installed.
- `just` is absent; direct recipe execution is required unless installed later.
- Full A4 graph reindex completed at 17,613 nodes / 83,598 edges and returned
  `miller_layout`, `NativeFileManagerWatcher`, and `normalize_watch_events`.
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
