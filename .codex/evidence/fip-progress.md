
## FIP-6 closure run — 2026-07-18 (session 3)

- 6.1 focused families: NAV 11/11 · FOCUS 9/9 · ICON 17/17 · REF/picker 35/35 · snapshot 20/20 (all non-zero filters).
- 6.2 visual: fresh `write_visual_fixtures` export → `npx playwright test` 14/14 (VIS-01..06).
- 6.4 gates: full nextest 3,494/3,494 + 2 skip · fmt clean · Linux clippy -D warnings clean · Windows MSVC bin clippy clean · python maintenance 64 OK · bun 5/5 + 12/12 · `git diff --check` clean · added-production-unwrap scan: none.
- 6.5 purity: `windowed_render_is_byte_identical_and_state_pure` (icon path) + `render_entry_row_performs_no_filesystem_io` PASS; picker recompute bounded O(rows), no new queue/cache/worker.
- 6.3 E2E — **OPEN (deferred to fresh session)**. Isolated throwaway-XDG instance + test-owned tmux ran cleanly; stable socket inode/mode/mtime IDENTICAL before/after (21223953 600 1783871657). Findings recorded:
  - zsh word-split trap: `tmux send-keys -H $seq` passed ONE arg; `${=seq}` required.
  - With correct injection, SGR mouse press/release REACH the server parser (herdr-server.log DEBUG: `Mouse(MouseEvent { kind: Down(Left), column: 20, row: 0 })`) but `handle_mouse` produces NO state change for sidebar-tab / new-workspace / new-tab targets, while keyboard (prefix C-b hint overlay) works. Dispatch-level no-op under SemanticFrame server mode needs a dedicated investigation (possible real server-mode mouse regression OR harness environmental gap). Do NOT claim E2E-01/02 until this is resolved.
