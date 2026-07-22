# Files Locations Follow Navigation Baseline

Date: 2026-07-22 CEST

## Boundary

- Branch: `feat/native-fm`
- Clean post-predecessor HEAD: `49e872347ae0ad94a18e33fe481a77f5c7cd4112`
- Fork publication: `origin/feat/native-fm` resolves to the same SHA.
- Rollback ref: `refs/checkpoints/herdr-flf-pre-implementation-20260722`
- Tracked worktree at the boundary: clean.
- Preserved user-owned untracked path: `.superpowers/`.
- Stable Herdr process/socket/config touched: `false`.

The predecessor remains an independent concern:

- `e279ecf9` — `fix: make file manager horizontal navigation directional`
- `49e87234` — `docs: record file manager horizontal navigation closure`

No FLF Rust change is included in either predecessor commit.

## Physical RED

The user's isolated 2026-07-22 trial established both target gaps without
reclassifying them as completed behavior:

1. Left from the Trail root does not transfer focus or highlight to the
   visible Locations Rail.
2. Right on a directory enters the child column, but the child has no
   immediate highlighted row; the first Down/Up supplies the missing cursor.

These observations match `TP-FLF-FOCUS-01`, `TP-FLF-ENTER-01`, and
`TP-FLF-CHILD-01`. They are the RED behavior to replace, not acceptance
evidence for the target implementation.

## Graph Evidence

Codebase Memory project: `home-ayaz-projects-herdr`

- Status: `ready`
- Nodes: `24,078`
- Edges: `129,027`
- Current symbols resolved from `src/`:
  - `handle_file_manager_key`
  - `sync_file_manager_location_request`
  - `sync_file_manager_io_results`
  - `render_file_manager_locations`
  - `render_trail_view`
- Planned symbol `focus_first_active_trail_entry`: absent, as expected before
  the FLF implementation.

Fresh snippets show the current ownership mismatch: root Left stops when
`TrailState::move_active_left()` returns false, `FileManagerLocationsState`
has focus but no independent Rail cursor, and a directory branch creates the
new `TrailCol` with `selected: None`. Rendering therefore has no immediate
child-row selection until a later vertical cursor move.

## Characterization Gate

Command:

```bash
cargo nextest run --locked -E 'test(/keyboard_directory_cursor_schedules_bounded_preview_without_focus_transfer|directory_preview_after_horizontal_focus_change_is_rejected|missing_directory_preview_preserves_cursor_and_resident_branch|fcl_io_worker_keeps_only_latest_pending_request|fcl_io_location_request_is_async_and_generation_safe|fcl_io_resident_root_activation_performs_zero_worker_reads/)' --no-fail-fast --status-level fail --final-status-level fail --failure-output final --success-output never
```

Result:

- Nextest run: `2d38eaab-b99a-4ad9-a05a-323459c9bb44`
- Selected: `6`
- Passed: `6`
- Failed: `0`
- Skipped by filter: `3,620`

This freezes the behavior being extended: cursor preview uses the existing
bounded/latest worker, stale horizontal completions are rejected, failures
preserve the resident branch, and a resident root activation performs zero
worker reads.
