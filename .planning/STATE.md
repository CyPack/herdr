# Herdr Planning State

- Updated: 2026-07-14
- Branch: `feat/native-fm`
- B2 product/test checkpoint: `2989434`; the continuity commit containing this
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
- Continuity/tooling is versioned by the separate commit containing this state
  file.
- Active increment: begin C1.1 named header-button geometry test-first.
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
- First next code action: make C1.1 named header-button geometry RED before
  production header/action code.
- Next code order: C1 through C6. Deferred
  architecture and north-star missions remain recorded in `.codex/TASKS.md`.
