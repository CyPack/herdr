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
- Vertical cursor movement and directory activation are separate product
  intents. Up/Down, `j/k`, Shift+Up/Down, and row wheel must remain in the exact
  owner column. Left moves one resident parent edge. Right/`l` owns
  directory-only child traversal and is inert on files/non-entries; Enter or
  explicit primary click retains file/directory activation.
- Stable installed Herdr and development Herdr must remain isolated. Use `.local/ISOLATED-DEV-TEST.md` for runtime checks.
- The acting GitHub account is `CyPack`; this is external-contributor/fork work. Never push upstream or open upstream issues/PRs for the user.

## Current Decision Ledger

- The main Native Files rapid-click and inert-mouse stutter is closed and
  human-accepted at `d8583d3a`. The final residual commits are `b2accbb4`
  (resident file projection), `8851b5e0` (inert-move render gate), `ed329058`
  (background text preview), and `d8583d3a` (deterministic filesystem-time
  fixtures). Closure publication adds `8f4b2acc` (test-only path-identity
  fixture) and `d52b4417` (evidence, Yazi transfer reference, and lessons).
  Canonical evidence:
  `.codex/evidence/files-performance-fix-closure-and-navigation-followups.md`.
- `FMN — Files Movement Semantics and Wheel Normalization` is closed and
  physically accepted by the user at published continuity head `787bb96b`.
  Raw isolated Ghostty evidence captured 333 vertical
  packets and 226 same-direction deltas below 2 ms in identical-coordinate
  triplets/occasional sextuplets; one-to-one routing rejected duplicate Herdr
  dispatch. Vertical keys/wheel now move an exact owner-column cursor without
  branching; Right/`l` owns directory traversal and Enter/click owns explicit
  activation; directory preview reuses
  the bounded latest worker with current cursor authority; a narrow `<2 ms`
  owner/direction/coordinate gate coalesces only the measured host burst.
  FMN-5 E2E/publication gates are complete.
- `FMH — Horizontal Miller Focus Navigation` is the active local lane. Its
  behavioral RED proved Right on a file fell through to `SelectedFile`,
  truncating the resident Trail and rendering. The minimum GREEN returns
  `Inert` for a non-directory cursor while preserving directory worker,
  Enter/click, Left, vertical, and wheel semantics. Automated closure is green:
  FMH 3/3, cross-layer 10/10, broad FM 190/190, full 3,622/3,622 + 4 skip,
  both Clippy targets, Python 68/68, Bun 5/5 + 12/12, exporter 1/1, Chromium
  33/33, zero JSON/PNG delta, and clean source/dependency/vendor/diff audits.
  Graph CLI is current at 24,078 / 129,027 with exact FMH snippets; the
  long-lived built-in channel remains a documented stale 24,072 / 129,520.
  Only isolated physical E2E and publication remain.
- Resident Trail depth and focus are distinct: `deepest()` is prepared-data
  extent; initialization, auto-follow, render/hit geometry, resize projection,
  and watcher binding use `active_col()`. Explicit Right/Left changes the
  owner; vertical movement never does.
- Yazi source commit `6d84921e7004eb8d49ba13a4acc97c6cfeb094b4`
  proves cursor/activation separation, discardable async folder preview,
  ticketed stale-result rejection, change-gated rendering, and an unbounded
  directory history. Transfer the first four laws; reject the unbounded cache.
  Canonical reference:
  `.codex/references/yazi-file-manager-performance-transfer.md`.
- Home/Desktop/Downloads pre-warm is not authorized without a separate
  first-entry latency RED. Any future implementation is allowlisted,
  background, mtime-invalidated, and capped per directory by entries and bytes;
  no general LRU.
- The user approved the bounded Files Interaction Polish program on
  2026-07-17. Its canonical design is
  `docs/superpowers/specs/2026-07-17-herdr-files-interaction-polish-design.md`.
  Drag-and-drop is excluded. The next task is FIP-G.1 planning through
  `superpowers:writing-plans`, not an immediate Rust edit.
- The FIP no-submit invariant is absolute: the selected safe UTF-8 file or
  directory path is inserted once into an explicitly selected live agent
  terminal with no CR/LF/Enter, submit, implicit whitespace, or implicit
  split/chat. All stale identity, path-kind, control-character, and
  backpressure cases fail closed.
- FIP visual acceptance requires Playwright Chromium driven by deterministic
  Ratatui cell fixtures, while Rust and isolated PTY tests retain semantic and
  byte-level authority.
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
- Native Files performance closure and next input contract:
  `.codex/evidence/files-performance-fix-closure-and-navigation-followups.md`.
- Pinned Yazi performance transfer reference:
  `.codex/references/yazi-file-manager-performance-transfer.md`.
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
- A human “stutter is gone” report is qualitative symptom acceptance. Keep it
  distinct from profiler counts, structural tests, and fresh publication gates.
- Under `set -euo pipefail`, guards inside `if`, `!`, traps, or conditional
  function calls still need explicit `|| return 1` or an explicit failure
  return before destructive cleanup can continue.
- An MCP proxy with 26 serially initialized servers takes about 54 seconds on
  this machine. Keep the readiness probe bounded at 120 seconds and the
  systemd start budget at 150 seconds; require exact server-set equality plus a
  real codebase-memory initialize/tools probe.
