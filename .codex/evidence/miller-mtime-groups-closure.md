# Miller mtime groups closure evidence

Date: 2026-07-19

## Product contract

Miller Trail columns use one strict descending modification-time order across
directories and files. Equal timestamps fall back to deterministic
natural/raw/path order; missing timestamps sort last. Prepared rows are grouped
by the local-calendar sections `Future`, `Today`, `Yesterday`,
`Previous 7 Days`, `Older`, and `Unknown Date`. Right-side timestamps are
complete or omitted; section headers have typed non-actionable terrain.

Canonical design:
`docs/superpowers/specs/2026-07-19-herdr-miller-mtime-groups-design.md`.
Execution plan:
`docs/superpowers/plans/2026-07-19-herdr-miller-mtime-groups-implementation.md`.

## Atomic history

- Design and dependency freeze: `c10e124c`, `1d400822`.
- Prepared mtime plus mixed sorting: `c8a8c4e3` RED / `7f6f9575` GREEN.
- Local-calendar grouping: `0831c855` RED / `86ac4cff` GREEN.
- Grouped projection/render: `9c1124c9` RED / `89e60144` GREEN.
- Typed header input and watcher reconciliation:
  `6e0460e8` RED / `9338cbbc` GREEN.
- Chromium oracle: `55516f50` RED / `3ff174ca` GREEN.
- Mtime-sensitive legacy fixture stabilization: `935c634f`.

## Fresh verification

- `cargo fmt --check`: pass.
- `cargo nextest run --locked --no-fail-fast`:
  3,551/3,551 pass, 2 skipped.
- Linux `cargo clippy --all-targets --locked -- -D warnings`: pass.
- Windows
  `LIBGHOSTTY_VT_SIMD=false cargo clippy --bin herdr --target
  x86_64-pc-windows-msvc --locked -- -D warnings`: pass.
- Maintenance Python suite: 68/68 pass.
- Agent-state Bun suite: 5/5 pass.
- Plugin-marketplace Bun suite: 12/12 pass.
- Playwright Chromium: 25/25 pass, update disabled for the final run.
- VIS-15/16/17 baselines were created only through the spec-scoped
  `mtime-groups.spec.ts --update-snapshots` command. Existing affected specs
  were reconciled one spec at a time; no global snapshot update was run.
- Controlled VIS-15 `Today` header-cell mutation:
  original PNG SHA-256
  `b3dfcd28d840712485d4cb8f59ab5c55600a981cd5d923fdfa78823bd4e7d15c`,
  mutated PNG SHA-256
  `338f8dda7facec886f04bb80949c6986043f79a8cd03f3e06a91d46f9b12f990`;
  raw buffers differed and the screenshot oracle failed by 14 pixels before
  the mutation was removed.
- `git diff c10e124c -- Cargo.lock`: only the expected direct `"time"` entry.
- `git diff --check`: pass.
- Program-added `src` lines containing `unwrap()`: zero.
- Working tree: clean except user-owned `?? .superpowers/`.

## Graph and safety

The required single-worker refresh completed at 23,656 nodes / 125,342 edges.
It reparsed the current mtime projection/render and stabilized test surfaces
without restarting the graph proxy. Stable Herdr, inherited sockets, user
sessions, upstream `ogulcancelik/herdr`, and `.superpowers/` were untouched.
