# Herdr Planning State

- Updated: 2026-07-14
- Branch: `feat/native-fm`
- Last published continuity checkpoint: `68abf61`
- Completed local commits:
  - `6c7c58f` A2.2 responsive Miller columns (already pushed)
  - `01ba91d` A4 live filesystem watching (pushed to CyPack feature/master)
  - `8cd4e89` deterministic lifecycle tests (pushed to CyPack feature/master)
  - `bcba84d` B0 native image Path Beta feasibility (pushed to CyPack
    feature/master)
  - `439ff2c..2b2dcd3` B1 bounded, generation-safe text/syntax preview
    (fully verified; graph/FF publication is the current close-out action)
- Continuity/tooling is versioned by the separate commit containing this state
  file.
- Active increment: close B1 publication, then execute A3 remainder
  test-point-first.
- Canonical current state: `/home/ayaz/projects/herdr/.codex/CURRENT.md`
- Durable tasks: `/home/ayaz/projects/herdr/.codex/TASKS.md`
- Full handoff: `/home/ayaz/projects/herdr/.codex/HANDOFF.md`
- B1 is implementation-complete at `2b2dcd3`: 64 KiB reader, explicit
  failures, minimal pure-Rust syntect behind one-active/one-pending worker,
  path/content generation safety, 128-line and rendered-column limits, and
  pure responsive render. Final gates: targeted 64/64, full 2948/2948,
  Linux/Windows clippy, Bun 17/17, Python 64/64.
- First code action after publication: make TP-A3.2-VIEWPORT RED; expected
  cursor-visible/clamped behavior and reasons are in `.codex/TASKS.md`.
- Next code order: A3 navigation remainder; B2 image preview under B0's
  conditional GO; then C1 through C6. Deferred
  architecture and north-star missions remain recorded in `.codex/TASKS.md`.
