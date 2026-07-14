# Herdr Project Memory

## Stable Facts

- Herdr is a terminal-based coding-agent runtime written primarily in Rust.
- Rendering must remain pure. Filesystem reads for the native file manager belong in state/model refresh paths, not render.
- The active native-FM implementation uses `AppState.file_manager: Option<FmState>` as a TUI content swap in the center region.
- The UI composition foundation is `Compositor` plus named `ShellLayout` regions. Do not add a speculative component registry before a second real component proves the need.
- The native-FM activation key is `prefix+f`; navigation uses arrows/hjkl, Enter, Backspace, `.`, Esc/q.
- Stable installed Herdr and development Herdr must remain isolated. Use `.local/ISOLATED-DEV-TEST.md` for runtime checks.
- The acting GitHub account is `CyPack`; this is external-contributor/fork work. Never push upstream or open upstream issues/PRs for the user.

## Current Decision Ledger

- S2 `ShellLayout` persistence is deferred to S6; serializable types already exist.
- The former S3 ComponentRegistry design is deferred to S5 because Herdr's current grain is central content-swap plus `ViewState` hit areas.
- A2.2 uses cached `FmParent`/`FmPreview` state so render performs no filesystem I/O.
- Responsive Miller thresholds: three columns when the area can hold three 12-cell panels plus dividers, two columns when it can hold two, otherwise current-only.
- A4 native watching is published on the CyPack fork. B0 is the active
  independent risk spike required before B2.
- The user granted standing authorization for autonomous targeted commits and
  CyPack fork-only fast-forward pushes. Preserve all verification and atomicity
  gates, but do not repeatedly ask for commit-message alignment.

## Evidence Pointers

- Claude session reconstruction: `.codex/evidence/claude-session-f53c720f.md`.
- Detailed native-FM/UI-ARCH history: `.local/prd/native-fm/MASTER-HANDOFF.md`.
- Current CLI handoff: `.codex/HANDOFF.md`.
- Current code/task truth: `.codex/CURRENT.md` and `.codex/TASKS.md`.
- Research vault: `~/.cartography/native-fm-research-INDEX.md`.

## Known Operational Lessons

- `index_status=ready` can still be stale; verify a recent symbol.
- `cargo test` can show false failures from shared process state; prefer nextest.
- A full nextest load may expose the known late-lifecycle flaky test; distinguish retry evidence from a clean first pass.
- `just` may be absent. Read `justfile` and run the entire recipe directly rather than claiming a partial gate.
- `RIPGREP_CONFIG_PATH` may reference a missing file; use `env -u RIPGREP_CONFIG_PATH rg ...`.
- Never bulk-stage with `git add -A`; local cartography and continuity artifacts may be present.
- An MCP proxy with 26 serially initialized servers takes about 54 seconds on
  this machine. Keep the readiness probe bounded at 120 seconds and the
  systemd start budget at 150 seconds; require exact server-set equality plus a
  real codebase-memory initialize/tools probe.
