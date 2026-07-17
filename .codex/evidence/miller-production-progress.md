# Miller Production Progress — P0–P7

Date: 2026-07-17

Branch: `feat/native-fm`

Starting checkpoint: `cbac59bb`

Verified P6 product head: `9e1d63bf`

Verified P7 product head: `d7997d0d`

## Current Override — P7 Verification Closed, Publication Pending

This section supersedes the older P6-only next-phase statement below.

P7 implementation and verification are closed at product head `d7997d0d`.
The isolated physical matrix found two real server/client-path ownership bugs
that unit-only P6 evidence had not exposed:

- headless key routing used `Mode::Terminal` instead of the typed
  `ShellInputOwner`, so Files-owned `q` and capture-owned Escape reached the
  hidden PTY;
- monolithic and headless bracketed paste paths also forwarded input to the
  hidden PTY while Native Files was visible.

Both defects were closed through independent RED/GREEN pairs:

- `714849bc` / `53f73f23` — headless Files keyboard and capture ownership;
- `5844db41` / `d7997d0d` — hidden-terminal paste isolation plus visible
  terminal positive controls.

### Isolated Physical Matrix

The matrix used a unique throwaway XDG config/state/runtime root and socket, a
test-owned foreground server, and a test-owned tmux shell/client. It unset
`HERDR_ENV`, `HERDR_PANE_ID`, `HERDR_TAB_ID`, `HERDR_WORKSPACE_ID`,
`HERDR_SOCKET_PATH`, `HERDR_CLIENT_SOCKET_PATH`, `HERDR_SERVER_SOCKET`, and
`HERDR_SESSION`. Stable Herdr and every user-owned process remained untouched.

| Scenario | Fresh evidence | Result |
|---|---|---|
| Deep path / wide | Exactly 32 segments at 240x80; five columns and four Miller dividers; focused/current remained visible | PASS |
| Deep path / narrow | 80x24 degraded to exactly CURRENT + PREVIEW with one divider; no overlap | PASS |
| Horizontal input | Shift-wheel moved only `first_visible` where movement was possible and clamped at both ends; plain wheel changed only vertical row/preview state | PASS |
| First divider | `x=55 -> 65`, outside release retained capture and committed the clamped preview | PASS |
| Second divider | `x=84 -> 96`, left overshoot clamped back to `84`; unrelated divider positions did not move | PASS |
| Resize-mid-drag | 240x80 -> 180x60 canceled the transaction, restored committed geometry, and made the old release inert | PASS |
| Capture Escape | Raw client Escape restored the pre-drag width, kept Files open, retired capture, and did not reach the PTY | PASS |
| Close/reopen | Raw `q` closed Files without entering the shell; reopen reset ephemeral width preferences; stale release was inert | PASS |
| Paste isolation | Bracketed marker sent while Files was visible was absent from the retained shell after close | PASS |
| Permission failure | Entering a mode-000 directory left Files active and preserved the deep current path/header | PASS |
| Unicode / preview | CJK text rendered without overlap; text preview stayed bounded; image state used the non-Kitty `(Kitty graphics req.)` fallback without crash | PASS |
| Terminal return | The retained terminal remained usable after Files close; hidden Files-owned key/paste input was absent | PASS |

Some PRD risk cases require deterministic generation, watcher, overlay, or
worker-state injection and were therefore not represented as misleading
physical clicks. Their fresh exact substitution gate is run
`f8373441-9190-4b97-a7cc-5e1b7c1da335`, 10/10:

- overlay blocks every typed Miller-row gesture;
- stale/reordered/deleted/evicted/reopened targets are inert;
- renamed non-current targets are consumed without model mutation;
- plain wheel changes only the hovered resident column;
- terminal resize cancels the stale Miller transaction;
- 1,000 preview moves remain bounded;
- stale text-worker completion after scroll is rejected;
- committed resize requests at most one image target;
- the Files Stage blocks hidden-terminal input;
- headless Files blocks bracketed paste.

### Cleanup and Stable-Runtime Boundary

- test client exited semantically; the Herdr client detached semantically; the
  hosting test shell exited normally;
- the test server stopped through `herdr server stop` and its foreground
  process exited with code 0;
- test sockets: 0; open file descriptors under the root: 0; throwaway root:
  absent;
- the permission fixture was restored before guarded deletion;
- stable socket before and after:
  inode `21223953`, mode `600`, mtime `1783871657`.

### P7 Gates

- headless owner/paste regression: 35/35;
- input/platform normalization: 146/146, run
  `dc7f12c9-adb4-4781-b974-26c39406052c`;
- Miller/Files/Stage/watcher/preview/resize: 245/245, run
  `8dff60f2-bd07-445f-b180-5508b43e8c08`;
- deterministic physical-substitution gate: 10/10, run
  `f8373441-9190-4b97-a7cc-5e1b7c1da335`;
- release 1,000-move workload: 1/1, run
  `4a4b59cd-cad8-4c8b-a4b9-aa0e0ffbde45`;
- release performance workload: 1/1, run
  `376e9530-90ea-4f82-8e4a-4ab50da48d6a`;
- full Nextest: 3,443/3,443, one named real-host probe skipped, run
  `8ea53580-4f2f-408d-8629-3a56748b0df1`;
- Linux all-target Clippy and Windows MSVC bin Clippy: PASS;
- Bun 5/5 + 12/12; Python 64/64; `cargo fmt --all -- --check`: PASS.

Fresh release p95:

| Metric | Result | Frozen budget |
|---|---:|---:|
| 120x40 compute p95 | 10 us | <= 500 us |
| 240x80 compute p95 | 14 us | <= 500 us |
| 120x40 full-frame p95 | 1,153 us | <= 8,000 us |
| 240x80 full-frame p95 | 4,115 us | <= 16,000 us |

### Graph Truth

The safe sequential CLI refresh reparsed eight changed files with zero
extraction errors and produced 21,056 nodes / 97,919 edges. Fresh CLI search
and exact snippets prove `App::handle_key_headless`,
`App::visible_terminal_owns_paste`, the inbound `src/app/mod.rs` caller, and
the retained `miller_layout` symbol. The long-lived built-in MCP channel still
serves the stale 21,041 / 97,701 store and returns zero results for both new
symbols. It was not restarted and is not represented as fresh.

P7 publication remains intentionally open until this continuity unit is
committed, the final clean-diff/stable-socket checks pass, and both CyPack refs
are proven fast-forwarded. FM5 remains a separate evidence-only placement
decision and is not part of the Miller production-completion PRD.

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
