# Miller Production Progress — P0–P6

Date: 2026-07-17

Branch: `feat/native-fm`

Starting checkpoint: `cbac59bb`

Verified P6 product head: `9e1d63bf`

## Current Override — P6 Closed, P7 Open

This section supersedes the older P0/P1-only scope and next-phase statements
below. Those sections remain as historical slice evidence.

P0–P6 of
`.local/prd/native-fm/MILLER-PRODUCTION-COMPLETION-PRD.md` are closed:

- one prepared, generation-safe 1–5-column Miller snapshot owns compute,
  render, image geometry, horizontal input, all-column rows, and dividers;
- the legacy resize authority is removed; mouse and keyboard use the shared
  typed `ResizeTransaction`, with 16..=64 clamps, one model commit, zero PTY
  resize, zero persistence request, and zero filesystem/worker work during
  preview;
- all-column mouse targets fail closed across overlays, stale generations,
  rename/delete/reorder, close/reopen, and branch truncation;
- growing navigation is bounded to 32 history segments and five resident/
  visible projections, with exact parent focus and current-only watcher
  ownership;
- render transport skips identical frames, keeps one pending payload per
  client, rejects stale worker results, and keeps invalid previews stable;
- the opt-in profiler has fixed label cardinality, fixed-size duration
  histograms, p95 output, and test-readable thread-local observation.

P7 remains open. No isolated runtime/manual matrix or release-readiness claim
is made by this checkpoint.

## P6 Verification

- P6 exact matrix: run `97be87ad-8f24-4300-bd4f-5e16a0f50f3d`,
  18/18 passed.
- Targeted Miller/Files/Stage/watcher/preview/resize gate:
  run `996dfa4d-803e-48eb-a183-012d9c8f0a90`, 244/244 passed.
- Full Nextest: run `d6bc5091-7c32-4027-a3e0-0d436d57c5ba`,
  3,437/3,437 passed, one named real-host probe skipped.
- Linux all-target Clippy and canonical Windows MSVC bin Clippy: PASS.
- Bun integration assets 5/5; plugin marketplace 12/12.
- Python maintenance: 64/64.
- `cargo fmt --all -- --check` and `git diff --check`: PASS.
- Stable Herdr, inherited sockets, terminals, browsers, editors, and every
  user-owned process were untouched. `.superpowers/` remains untracked and
  unstaged.

Final release-profile named workload:

| Metric | Result | Frozen budget |
|---|---:|---:|
| 120x40 compute p95 | 17 us | <= 500 us |
| 240x80 compute p95 | 9 us | <= 500 us |
| 120x40 full-frame p95 | 1,057 us | <= 8,000 us |
| 240x80 full-frame p95 | 4,804 us | <= 16,000 us |

The profiler also exposes bounded counters for geometry cache hit/miss, resize
preview/commit, filesystem reads, watcher rebinds, text/image worker
submission/completion/rejection, image-target refresh, PTY-resize requests,
debounced persistence-write requests, identical-frame skips, outgoing bytes,
and queue-full outcomes. The persistence counter represents the logical
debounced write request, not completion of the background disk thread.

## Graph and Publication Truth

The safe one-worker CLI refresh indexed the P6 head with zero extraction
errors: 21,041 nodes / 97,701 edges. CLI search returns the new production
`render_prof::duration_guard` symbol. The long-lived MCP transport failed on a
later call and was not restarted; it is not claimed fresh or available.

At this evidence-writing point, both CyPack refs still equal the last published
P5 head `c26244709ab55a6f73b30415299a9de4df0fc27a`. The P6 chain is green but
unpublished until the continuity commit and final pre-push verification
complete. `upstream` remains untouched.

## Exact Next Phase

P7 only:

1. follow `.local/ISOLATED-DEV-TEST.md` with unique throwaway XDG roots and
   sockets;
2. execute the practical deep-path, fast-drag, scroll, branch, Unicode,
   preview, close/reopen, Terminal-return, and non-Kitty matrix without
   touching stable Herdr;
3. verify cleanup residue is zero;
4. rerun proportionate final gates, reconcile release/continuity evidence,
   and only then fast-forward the two CyPack refs.

## Scope and Truth Boundary

P0 reconciled the canonical task registry and froze the published legacy-trio
behavior before cutover. P1 adds the first production consumer of the bounded
Miller viewport geometry without changing visible render output.

P1 owns:

- active typed Files singleton generation;
- Miller model revision and clamped `first_visible`;
- focused logical chain index;
- at most five complete column path/rect identities;
- at most four adjacent divider path/rect identities;
- desktop/mobile compute ownership;
- close, foreign-surface, zero-body, and reopen retirement.

P1 explicitly does not own:

- rendering the snapshot;
- horizontal key/wheel/header input;
- shared `ResizeTransaction` cutover;
- all-column row targets or mouse actions;
- growing branch navigation;
- production p95, outgoing-byte, or isolated-runtime closure.

`render_file_manager` therefore still draws the characterized fixed
parent/current/preview compatibility trio. P2 must remove that visible gap.

## Commit Chain

- `2812c5ac` — `docs: reconcile miller production task status`
- `345d32a5` — `test: define production miller projection`
- `35cfbc00` — `feat: project bounded miller viewport into view state`

No RED-only or Clippy-dirty head was pushed. `.superpowers/` was never staged.

Product publication was a verified fast-forward:

- `refs/heads/feat/native-fm` =
  `35cfbc0074fdf2a1319fb7f5954b5eb7953a95be`
- `refs/heads/master` =
  `35cfbc0074fdf2a1319fb7f5954b5eb7953a95be`
- `upstream` was not pushed or otherwise mutated

## RED Evidence

Initial production-consumer RED:

- `compute_view_projects_one_to_five_miller_columns`
- run `80527c6a-f601-4c42-99ad-6abbeaa39e9d`: missing production projection
- corrected body-oracle run
  `bc53a8f2-fc66-471f-bc68-62b4b0c0d08d`: same intended missing-projection RED

Files lifecycle-generation RED:

- run `0593ac75-d3d5-4e2d-83e4-49189102fe28`
- selected 1, passed 0, failed 1
- exact failure: the production snapshot did not carry the active Files
  singleton generation

Additional transition REDs exercised during the slice:

- close retirement run `56002e76-847a-474b-a177-9ca018de0383`
- typed foreign-surface authority run `a03f8d62-0d6b-41b7-ad17-5316102b58f6`

Each failed at its intended behavior assertion, not at test setup, filtering,
compilation, or transport.

## GREEN Test Points

| Test | Expected result | Reason |
|---|---|---|
| `compute_view_projects_one_to_five_miller_columns` | Breakpoints exercise exactly 1/2/3/4/5 visible complete columns; dividers are `n-1`; paths and rects match the logical chain; focus stays visible | Proves the pure geometry core has a production consumer |
| `windowed_projection_uses_model_first_visible` | Nonzero requested origin produces indices 2/3/4 and does not change chain, focus, or model revision | Establishes viewport authority |
| `zero_files_area_retires_windowed_miller_targets` | Desktop zero width and mobile header-only frames expose no column/divider/row/action/header targets | Prevents stale hidden hits |
| `windowed_projection_does_not_read_filesystem` | Intentionally absent logical directories remain prepared identities; cwd, entries, revision, and chain remain unchanged | Preserves compute/render purity |
| `focused_column_remains_visible_after_projection_shrink` | Five columns shrink to two; focused index 6 remains present and every rect is bounded | Covers normal responsive degradation |
| `reopened_files_projection_uses_fresh_instance_generation` | Close clears projection identity; reopen produces a strictly newer Files generation | Prevents close/reopen ABA aliasing |
| `windowed_projection_requires_typed_files_surface` | A retained FM model under a foreign typed Stage projects no Miller geometry or generation | Prevents split-brain authority |

Runs:

- single strengthened GREEN:
  `52a9a580-e047-47c4-8b5b-6a9fd463fefe` — 1/1
- P1 catalog:
  `f9beb264-87f3-4db0-9ed7-e5a2690ba0f6` — 7/7
- post-Clippy-fix P1 catalog:
  `ed1ef867-9770-42ec-a591-c483f0f56ca1` — 7/7
- Miller/Files/Stage/watcher/preview/resize cross-layer family:
  `9db7cad1-ec42-4d76-855a-67a0693a2201` — 189/189

The characterization set including both legacy dividers, runaway capture,
close retirement, pure viewport clamps, and typed projection ran 11/11 in
`91478ef4-a9e8-4732-a0eb-bce7323e34a6`.

## Full Gate

`just` is not installed in this environment. Every command in the lowercase
`justfile` `check` recipe was run directly with `set -euo pipefail`.

- `cargo fmt --check`: PASS
- Linux `cargo clippy --all-targets --locked -- -D warnings`: PASS
- Windows:
  `LIBGHOSTTY_VT_SIMD=false cargo clippy --bin herdr --locked --target
  x86_64-pc-windows-msvc -- -D warnings`: PASS
- Full Nextest run `3fc79b68-0c85-430a-8be1-e6ac10dce45d`:
  3,354 run, 3,354 passed, 1 named real-host B0 probe skipped, zero retry
- `bun test src/integration/assets/herdr-agent-state.test.ts`: 5/5
- `workers/plugin-marketplace` Bun suite: 12/12
- Python maintenance suites: 64/64
- `git diff --check`: PASS
- added production `unwrap(` audit: zero
- stable Herdr/socket and all user applications: untouched

One intermediate Linux Clippy run correctly rejected four redundant borrowed
test paths. The fixtures were corrected, the 7/7 P1 catalog reran, and both
platform Clippy gates then passed. The failed gate was not hidden or counted
as completion.

## Graph Freshness

Safe sequential refresh:

```bash
CBM_WORKERS=1 codebase-memory-mcp cli index_repository \
  '{"repo_path":"/home/ayaz/projects/herdr","mode":"fast","persistence":false}'
```

Result:

- extraction errors: 0
- nodes: 20,649
- edges: 94,370
- built-in MCP finds `project_miller_view`, `sync_miller_view`, the production
  projection tests, and the reopen-generation test
- inbound call proof:
  `project_miller_view <- sync_miller_view <- compute_view_internal |
  compute_mobile_view`
- exact source snippet shows the three-parameter
  `(stage, prepared FmState, files_generation)` seam

Freshness is proven by current symbols and source, not by `ready` alone. No
proxy or user process was restarted or killed.

## Exact Next Phase

P2 begins with compile-valid RED tests before render production edits:

1. render 1–5 snapshot columns from prepared resident/current projections;
2. keep every render rect identical to the P1 snapshot;
3. move image target geometry onto the same snapshot authority;
4. route ScrollLeft/Right, Shift+wheel, and bounded header arrows by mutating
   only `MillerState.horizontal.first_visible`;
5. preserve ordinary vertical wheel ownership for the hovered column;
6. prove byte-identical double render, no filesystem/runtime mutation, Unicode
   cell correctness, overlay blocking, tiny/zero areas, and explicit
   unavailable-ancestor states.

P3–P7 remain open and must not be summarized as completed.
