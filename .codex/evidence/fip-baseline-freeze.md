# FIP-0.1 — Baseline Freeze Evidence

Date: 2026-07-18 CEST · Branch: `feat/native-fm` · HEAD at freeze: `c23aebfa`

## Characterization set (design "Characterization Gate" families)

Command:

```bash
cargo nextest run --locked -E 'test(/sidebar_tab|activate_dock|current_row_mouse|directory_chain|agent_handoff|attachment_target|symlink|special_entry|truncat|unicode_row_action/)' \
  --status-level fail --final-status-level fail --failure-output final --success-output never
```

Result: **50 passed, 0 failed** (3,394 skipped by filter), run ID
`831894cc-5262-4e1c-8403-7e886f9feb7a`, 1.163s.

This freezes the adjacent green behavior the FIP corrections must preserve:
sidebar tab switching, AppDock Files activation, current-row mouse focus,
directory-chain append, current `path + Enter` handoff, vanished
attachment-target rejection, symlink/special classification at the operation
level, truncation, and Unicode row-action isolation.

## Full suite

Command: full `cargo nextest run --locked ...` (justfile `test` recipe, direct).

Result: **3,443 passed, 1 skipped** (named `path_beta_real_host_probe`), run ID
`df00a924-f57b-4bc9-8702-e15839bc053d`, 40.188s. Zero retries.

## Graph freshness

`index_status(project="home-ayaz-projects-herdr")` = 21,064 nodes / 98,009
edges. The 2026-07-18 planning session verified `MillerPathSegment.focused_child`,
`sync_file_manager_agent_handoff_send` (payload `\r` at
`src/app/file_agent_handoff.rs:172`), `StageState::activate_files`/`close_files`,
`AppState::close_file_manager`, `entry_capabilities`, `render_entry_row`, and
`agent_panel_entries` from CURRENT source snippets, not `ready` alone. The three
commits since the graph refresh (`dd81ef59`, `ce605adb`, `c23aebfa`) are
docs/continuity only; no Rust changed, so the graph remains current for code.

## Environment notes

- `just` binary absent; justfile recipes executed directly (recorded per HP3).
- Global `rust-dev` skill is a broken symlink (`~/.codex/skills/rust-dev` →
  missing `~/.claude/skills/rust-dev`); no copy exists in vault/backup/factory.
  Per the plan's recorded fallback, work proceeds on the herdr-local catalog
  (`docs/patterns/rust-engineering.md` HP1-HP10 + FM/TUI pattern catalogs),
  which takes precedence for herdr work anyway. Restoring the global layer
  remains a user-level repair item.

No product behavior changed in FIP-0.1.
