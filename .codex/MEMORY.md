# Herdr Project Memory

## Stable Facts

- Herdr is a terminal-based coding-agent runtime written primarily in Rust.
- Rendering must remain pure. Filesystem reads for the native file manager belong in state/model refresh paths, not render.
- The visible native-FM implementation still uses
  `AppState.file_manager: Option<FmState>` as a legacy TUI center-content
  curtain. SF4.1 now has typed client-local Terminal/Files Stage state; SF6 is
  the only phase authorized to remove the curtain and migrate Files rendering.
- The UI composition foundation is `Compositor`, named bounded `ShellLayout`
  regions, transactional resize/collapse/scroll, and snapshot-v4 shell
  presentation persistence. Do not add an arbitrary ComponentRegistry before
  its separate S5 trigger is proven.
- The native-FM activation key is `prefix+f`; navigation uses arrows/hjkl, Enter, Backspace, `.`, Esc/q.
- Stable installed Herdr and development Herdr must remain isolated. Use `.local/ISOLATED-DEV-TEST.md` for runtime checks.
- The acting GitHub account is `CyPack`; this is external-contributor/fork work. Never push upstream or open upstream issues/PRs for the user.

## Current Decision Ledger

- The user approved twelve ordered phases: SF0-SF6 followed by FM1-FM5.
  Apps/Desktop is a later independent program.
- SF0-SF3 are closed. Named/typed bounded shell geometry, cached semantic hits,
  transactional resize, collapse/restore, two-axis scroll ownership, and
  snapshot-v4 persistence with v3 migration are published.
- SF4.1 is active and 7/8 behavior slices are GREEN through product head
  `f0f32075`. The next RED is
  `stage_surface_switch_does_not_destroy_terminal_runtime`.
- Typed Stage identity is TUI/client-local presentation state. It must not own,
  create, destroy, or rename server/runtime terminal identity.
- The former S6 and N2.2 gates are superseded only by the explicitly approved
  bounded SF/FM program. Arbitrary S5 ComponentRegistry and S7 popup stack
  remain independently trigger-gated NO-GO.
- The non-product change-pipeline lane is paused at T3.1 until the current
  sequential product phase closes. Product and tooling commits never mix.
- A2.2 uses cached `FmParent`/`FmPreview` state so render performs no filesystem I/O.
- Responsive Miller thresholds: three columns when the area can hold three 12-cell panels plus dividers, two columns when it can hold two, otherwise current-only.
- A3, A4, B0, B1, B2, C1-C6, N1/N3/N4/N2.1, M1, and M2 are published and
  closed with full evidence. M3 is an evidence-backed implementation NO-GO.
- Text/image preview, watcher, operation, selection, context, plugin, and agent
  handoff behavior must be preserved intact during SF6 migration.
- The user granted standing authorization for autonomous targeted commits and
  CyPack fork-only fast-forward pushes. Preserve all verification and atomicity
  gates, but do not repeatedly ask for commit-message alignment.

## Evidence Pointers

- Claude session reconstruction: `.codex/evidence/claude-session-f53c720f.md`.
- Detailed native-FM/UI-ARCH history: `.local/prd/native-fm/MASTER-HANDOFF.md`.
- Current CLI handoff: `.codex/HANDOFF.md`.
- Canonical next-session trigger: `.codex/NEXT-SESSION-PROMPT.md`.
- Current code/task truth: `.codex/CURRENT.md` and `.codex/TASKS.md`.
- Current SF4.1 evidence:
  `.codex/evidence/shell-foundation-sf4-stage-progress.md`.
- Research vault: `~/.cartography/native-fm-research-INDEX.md`.

## Known Operational Lessons

- `index_status=ready` can still be stale; verify a recent symbol.
- The proven current graph refresh is sequential:
  `CBM_WORKERS=1 codebase-memory-mcp cli index_repository ...`. The long-lived
  built-in channel may keep an older snapshot; do not restart the proxy or
  mislabel CLI evidence as built-in MCP evidence.
- `cargo test` can show false failures from shared process state; prefer nextest.
- A full nextest load may expose the known late-lifecycle flaky test; distinguish retry evidence from a clean first pass.
- `just` may be absent. Read `justfile` and run the entire recipe directly rather than claiming a partial gate.
- `RIPGREP_CONFIG_PATH` may reference a missing file; use `env -u RIPGREP_CONFIG_PATH rg ...`.
- Never bulk-stage with `git add -A`; local cartography and continuity artifacts may be present.
- An MCP proxy with 26 serially initialized servers takes about 54 seconds on
  this machine. Keep the readiness probe bounded at 120 seconds and the
  systemd start budget at 150 seconds; require exact server-set equality plus a
  real codebase-memory initialize/tools probe.
