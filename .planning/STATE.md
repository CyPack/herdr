# Herdr Planning State

- Updated: 2026-07-14
- Branch: `feat/native-fm`
- C1.2 product checkpoint: `7fd01de`. The continuity commit containing this
  state file is part of the CyPack publication unit.
- Completed local commits:
  - `6c7c58f` A2.2 responsive Miller columns (already pushed)
  - `01ba91d` A4 live filesystem watching (pushed to CyPack feature/master)
  - `8cd4e89` deterministic lifecycle tests (pushed to CyPack feature/master)
  - `bcba84d` B0 native image Path Beta feasibility (pushed to CyPack
    feature/master)
  - `439ff2c..2b2dcd3` B1 bounded, generation-safe text/syntax preview
    (fully verified, graph-indexed, and published through continuity commit
    `a0f82a3` to CyPack feature/master)
  - `d713b71..9d69c82` A3 cursor viewport, shared row hit geometry, runtime
    mouse dispatch, and cursor-only v1 selection scope (fully verified,
    graph-indexed, and published to CyPack feature/master)
  - `de1eff5..2989434` B2 dependency decision, bounded decoder, client-local
    placement, generation-safe worker, cached Kitty paint/cleanup, and
    width-safe non-Kitty fallback (fully verified, graph-indexed, and published
    to CyPack feature/master)
  - `0ed5e51` / `c9bfbf9` C1.1 header-action geometry RED/GREEN plus
    independent test-stability fix `9aa1e59` (fully verified and graph-indexed;
    published to CyPack feature/master)
  - `dbc6798` / `7fd01de` C1.2 side-effect-free exact header action dispatch
    RED/GREEN (fully verified and graph-indexed; publication completes with
    this continuity commit)
- Continuity/tooling is versioned by the separate commit containing this state
  file.
- Active increment: begin TP-N3.1-CONTENT test-first; define a pure,
  selection-sensitive persistent action-bar model before action authority.
- Canonical current state: `/home/ayaz/projects/herdr/.codex/CURRENT.md`
- Durable tasks: `/home/ayaz/projects/herdr/.codex/TASKS.md`
- Full handoff: `/home/ayaz/projects/herdr/.codex/HANDOFF.md`
- B1 is implementation-complete at `2b2dcd3`: 64 KiB reader, explicit
  failures, minimal pure-Rust syntect behind one-active/one-pending worker,
  path/content generation safety, 128-line and rendered-column limits, and
  pure responsive render. Final gates: targeted 64/64, full 2948/2948,
  Linux/Windows clippy, Bun 17/17, Python 64/64.
- A3 final gates: targeted 164/164, full nextest 2966/2966 plus one named B0
  host-probe skip, Linux/Windows clippy, Bun 17/17, Python 64/64, and isolated
  real-PTY mouse/viewport/double-click closure with no temp/process residue.
- B2 final gates: targeted 96/96; full nextest 2983/2983 plus one named B0
  interactive probe skip; Linux/Windows clippy; Bun 17/17; Python 64/64;
  source-to-host image comparison 0/271425 pixel difference; host cleanup and
  semantic exit with zero process/socket/temp residue.
- C1.1 final gates: geometry/render/ViewState 4/4; lifecycle family 27/27;
  full nextest 2986/2986 plus one named B0 host-probe skip; Linux/Windows
  clippy; Bun 17/17; Python 64/64; fmt/diff clean. Full graph: 17,986 nodes /
  83,818 edges with the header action types and geometry seam verified.
- C1.2 final gates: exact dispatch 2/2; FM input 13/13; full nextest 2988/2988
  plus one named B0 host-probe skip; Linux/Windows clippy; Bun 17/17; Python
  64/64; fmt/diff clean. Full graph: 17,993 nodes / 84,009 edges with dispatch,
  handler, outer input connection, and tests verified.
- First next code action: make TP-N3.1-CONTENT RED for directory/file/empty,
  clipboard, watcher/navigation, and close/reopen action-bar state.
- Next code order: N3.1, N3.2, then C2 through C6. Deferred
  architecture and north-star missions remain recorded in `.codex/TASKS.md`.
