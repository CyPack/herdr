# Herdr Files Rapid Navigation Latency PRD

## Status

- Program: `FMP — Files Mouse and Navigation Performance`
- Date: 2026-07-19
- Product baseline: locked `Files Layout V1`
- Trigger: user-observed delayed mouse response and temporary freezing during
  rapid reverse/ancestor and locations-rail clicking
- Current phase: closed at `d8583d3a`; human live acceptance recorded
- Product optimization authorization: approved for the two measured slices
  below, each behind a behavior-specific RED and preserved Layout V1 oracles
- Required visual verifier: Playwright Chromium

## 2026-07-21 Closure Addendum

The original rapid-click freeze and inert-motion stutter are closed. The final
residual chain after the FMP foundation is:

- `b2accbb4` — resident file selection reuses its owning parent snapshot;
- `8851b5e0` — inert file-manager mouse moves can decline render;
- `ed329058` — file text preview preparation leaves the input loop;
- `d8583d3a` — filesystem-time-sensitive fixtures use deterministic time
  authority.

Local HEAD and `origin/feat/native-fm` were both
`d8583d3ab564d42f880e94e0462f9d12ab61d391` when this addendum was prepared.
The user then completed the isolated live trial and reported that the original
freezing/stutter appears completely gone and interaction works very well.

Two newly reported behaviors are separate follow-ups and do not reopen this
latency program:

1. one physical wheel gesture can appear to skip three to five entries;
2. Up/Down or wheel landing on a directory can implicitly move navigation into
   its child column without Right/Enter.

The second behavior is source-confirmed: vertical movement currently calls the
same activation reducer as an explicit action. The next program must first
separate cursor movement from directory activation, then measure physical wheel
events before choosing any burst-normalization policy. Canonical closure and
follow-up evidence:

- `.codex/evidence/files-performance-fix-closure-and-navigation-followups.md`;
- `.codex/references/yazi-file-manager-performance-transfer.md`.

## Problem

When the user rapidly clicks backward through already visible Miller columns,
or quickly clicks several rows/locations in succession, Native Files can feel
temporarily frozen. Mouse events appear to be recognized late and visible
selection/navigation catches up after the interaction burst.

The isolated 2026-07-19 profile is now evidence of two independent defects.
The investigation distinguished:

1. input arrival latency;
2. input dispatch and Trail mutation latency;
3. bounded worker request/result latency;
4. view projection and Ratatui render latency;
5. frame encoding, queueing, and client presentation latency.

The primary trigger is synchronous filesystem work performed inside directory
click dispatch. The secondary amplifier is client-side head-of-line blocking:
input, resize, and server presentation share one FIFO and one blocking
consumer. Neither finding authorizes a visual Layout V2 change.

## Protected Behavior

All work is constrained by
`docs/superpowers/specs/2026-07-19-herdr-files-layout-v1-lock.md`.

In particular:

- the global agent/workspace tracker remains visible;
- the Files-local locations rail and compact drawer remain the location owner;
- explicit `Location(path)` / `Direct(path)` origin remains authoritative;
- the Trail retains exact path, active-end follow, partial columns, per-path
  widths, one-third horizontal scroll, grouped mtime rows, actions, and detail;
- all existing visual baselines remain unchanged unless the user explicitly
  approves Layout V2.

## Current Architecture Evidence

Fresh Codebase Memory reports 23,863 nodes / 124,101 edges and returns the
current FCL and client-loop symbols.

The current hot path is serial across both client and server boundaries:

```text
terminal mouse bytes
  -> client stdin thread
  -> shared client FIFO (input + resize + every server message, capacity 256)
  -> blocking client socket write
  -> server socket reader
  -> server event FIFO (capacity 64)
  -> HeadlessServer::handle_client_input_events
  -> App::route_client_events
  -> App::handle_file_manager_mouse
  -> FmState::activate_trail_entry
  -> synchronous child listing read
  -> synchronous legacy parent/preview projection reads
  -> full-render decision
  -> HeadlessServer::render_and_stream
  -> one-slot client render queue
  -> shared client FIFO
  -> synchronous encode + stdout write + flush
```

Confirmed properties:

- plain-left Trail directory activation bypasses the bounded FM worker;
- `TrailSnapshots::select_dir` re-reads the selected child even when the exact
  child snapshot is already resident in the next Trail column;
- accepted activation then reconstructs the legacy operation projection,
  synchronously enumerating the parent and selected-directory preview again;
- one folder click can therefore enumerate the selected child twice plus the
  owner parent, with per-entry classification, `symlink_metadata`, mtime, and
  sorting;
- every interactive client batch requests a post-input semantic frame;
- server events and changed scheduled tasks force a full render;
- `render_and_stream` builds a Ratatui frame for each target client;
- a full render uses a one-slot non-blocking writer queue and records
  `full_render.queue_full` when it cannot send the current frame;
- the client uses one FIFO for stdin, resize, all server messages, disconnect,
  and timer coordination; its single consumer also performs blocking socket
  writes and blocking stdout presentation;
- a `SemanticFrame` is a complete logical frame and may be latest-wins;
  `TerminalAnsi` deltas, graphics, clipboard, shutdown, title, mouse-capture,
  and input-source control are ordered/lossless and may not be blindly
  coalesced;
- already resident location roots reset synchronously without filesystem I/O;
- non-resident root, Miller navigation, and refresh preparation use the
  bounded one-executing/one-latest-pending FM worker;
- the existing opt-in profiler records render, encode, queue, PTY, watcher,
  and filesystem counters, but not click dispatch, Trail activation, resident
  reset, FM submit latency, or result-apply duration.

## Measured Evidence

The cleanup-first/cleanup-last isolated run is archived at:

```text
.local/perf/files-layout-v1/20260719-165338-run
```

Client and server logs contain 7,411 parsed input events. Their raw byte
signatures are exactly equal and in the same order: there is no mouse-byte
loss. Pairing the identical sequences gives client-main-loop parse to server
parse latency:

| Metric | Result |
|---|---:|
| p50 | 9.872 ms |
| p95 | 7,587.645 ms |
| p99 | 9,303.526 ms |
| max | 9,867.817 ms |

Every major server-log silence begins immediately after `Mouse Down(Left)`.
Observed stalls include 9.945 s, 9.886 s, 9.837 s, 9.230 s, 8.024 s, and
7.024 s. The 312.762-second profile contains 1,699 filesystem reads and 4,158
sent full frames (500,967,844 serialized bytes), while
`full_render.queue_full` remains zero. In a 10.910-second worst window, 62
filesystem reads occur; full render averages 16.4 ms and peaks at 18.9 ms.

The several-second hole is therefore inside click dispatch/filesystem work,
not frame composition. When the serial server loop blocks, its 64-event
channel and socket receive path back up. The client's blocking socket write
then stalls, so its shared FIFO and blocking presentation path amplify the
delay.

## Hypothesis Decision

### H4 — Synchronous Trail filesystem work

Decision: accepted as primary.

The previous assumption that resident ancestor clicks perform zero filesystem
reads was false. Production code unconditionally reads the child and then
rehydrates parent/preview state with more reads on the input loop.

Required boundary: resident rebranch is entirely disk-free; nonresident or
stale directory activation is a typed request on the existing bounded worker.

### H1 — Full-frame work and client queue pressure

Decision: accepted as an independent amplifier, not the primary 10-second
trigger.

The server writer already retains at most one pending render, but the client
destroys that property by placing every frame in the same FIFO as input and
rendering/flushing synchronously. Semantic frames require a latest-only
mailbox; lossless input and ordered controls require independent bounded
lanes with explicit fairness.

The measured low render duration and zero server queue-full events falsify
frame computation as the primary trigger, not the client HOL defect itself.

### H2 — Synchronous resident Trail projection cost

Decision: rejected as the primary cause.

Backward/ancestor clicks operate on resident snapshots, so they bypass
filesystem I/O but synchronously rebranch/reproject Trail state. The current
activation path also performs exact-path linear searches. Cost may grow with
deep Trails, large columns, grouped rows, and repeated intermediate clicks.

Linear searches and projection clones may be optimized only after a focused
benchmark proves they violate a budget. They do not explain multi-second
server silence.

### H3 — Serial scheduled work competes with new input

Decision: not evidenced as primary.

The headless loop drains client events, then synchronizes FM results and other
scheduled work before rendering. Worker completion, watcher reconciliation,
preview work, and repeated full-render decisions may consume the same serial
loop budget during an interaction burst even though directory reads themselves
are off-thread.

Scheduled work is not the click-local trigger. Worker completion, watcher, and
preview races remain required adversarial gates for the fix.

## Observability Contract

Profiling remains opt-in through `HERDR_RENDER_PROF=1` and is disabled by
default. New metrics must:

- use static bounded labels;
- add no path, filename, terminal content, or user data to logs;
- avoid allocations and locks when profiling is disabled;
- preserve the existing 128-label cap and one-second flush window;
- distinguish counts from durations;
- remain client/TUI performance facts, not server protocol fields.

Required correlation stages:

```text
stdin bytes framed
  -> client input dequeued
  -> client message written
  -> server input batch received
  -> mouse event dispatched
  -> Trail/location mutation accepted or rejected
  -> FM request submitted/replaced
  -> FM result applied/rejected
  -> compute_view complete
  -> frame prepared
  -> frame sent, deferred, or skipped
```

The first implementation slice records aggregate counters and durations, not
per-event identifiers or unbounded traces.

## Initial Budgets

Budgets are frozen only after the deterministic fixture is calibrated on the
same host/profile. Until then they are provisional diagnostic thresholds:

- mouse dispatch p95: at most 2 ms;
- resident Trail activation p95: at most 4 ms;
- resident location reset p95: at most 4 ms;
- FM result apply p95: at most 4 ms;
- `shell.compute_view` p95: at most 12 ms;
- full-render total p95: at most 20 ms;
- render writer queue-full ratio during a 100-click burst: less than 5%;
- accepted latest destination visible within two rendered frames after input
  stops;
- zero filesystem reads for the resident ancestor/active-root fixture;
- bounded worker: one executing and one latest pending request only.

Final regression budgets require warm-up, fixed geometry, fixed fixture data,
fixed sample count, debug/release profile identification, and recorded host
conditions. A wall-clock assertion alone is not sufficient.

## Test-Point Catalog

| ID | Test | Expected result | Reason |
|---|---|---|---|
| `TP-FMP-OBS-01` | profiler disabled, run click and render workload | no metric lock/allocation path is entered; behavior and buffers are unchanged | observability must be free by default |
| `TP-FMP-OBS-02` | profiler enabled, one accepted row click | input, dispatch, activation, compute, render, and send stages increment bounded metrics | separates event arrival from presentation |
| `TP-FMP-OBS-03` | profiler enabled, stale/inert click | rejected/inert outcome is counted without state mutation or path data | failure paths must be visible without leaking identity |
| `TP-FMP-OBS-04` | 100-event deterministic burst | counter totals are exact, labels remain bounded, p95/max are emitted | proves the probe itself remains bounded |
| `TP-FMP-RES-01` | alternate through deep resident ancestor columns | final exact row/path wins; child, parent, preview, and file-body reads are all zero; invariants hold | matches the primary reported reproduction and catches the worker bypass |
| `TP-FMP-RES-02` | repeatedly activate current resident location | zero worker reads and bounded reset time | isolates the location fast path |
| `TP-FMP-TRAIL-01` | exact child snapshot is resident at `col + 1` | activation reuses it, truncates the right branch, focuses the child column, and performs zero filesystem work | resident back-navigation must be immediate |
| `TP-FMP-TRAIL-02` | nonresident directory reader is deterministically blocked | click dispatch returns without waiting; a newer click replaces the pending intent; only the latest exact result applies | proves all cold listing I/O left the server input loop |
| `TP-FMP-TRAIL-03` | prepared activation races with watcher, newer click, location switch, and close/reopen | every stale generation/source/column/path completion is inert | responsiveness cannot weaken authority |
| `TP-FMP-TRAIL-04` | worker panics, disconnects, or returns missing/permission/changed-type | resident Trail remains intact and failure is bounded | failure paths must fail closed |
| `TP-FMP-IO-01` | blocked executing root plus two newer locations | only latest pending destination runs/applies; input dispatch stays responsive | preserves bounded latest-wins behavior |
| `TP-FMP-IO-02` | worker result races with a new click, watcher tick, and close/reopen | stale result rejects; latest exact state remains authoritative | race handling cannot trade correctness for speed |
| `TP-FMP-SYM-01` | compare resident/non-resident real directories with local directory symlinks; include dangling/changed-type failure | resident paths perform zero reads, bounded worker owns non-resident reads, exact symlink identity survives, and failures preserve the Trail | separates Native Files symlink I/O from unrelated Claude/Codex pointer health |
| `TP-FMP-CLIENT-01` | exact click sequence competes with hundreds of semantic frames | clicks remain lossless/in-order and are serviced before stale presentation backlog | prevents the measured client HOL amplifier |
| `TP-FMP-CLIENT-02` | semantic frame burst while presentation is busy | at most one latest semantic frame remains pending and the final frame renders | complete frames are state, not a FIFO event log |
| `TP-FMP-CLIENT-03` | continuous input plus pending frame/control | bounded input quantum prevents frame starvation; shutdown/disconnect/control remain bounded and lossless | priority needs explicit fairness |
| `TP-FMP-CLIENT-04` | Terminal ANSI, graphics, clipboard, title, capture, and input-source messages | no delta/control message is coalesced, dropped, or reordered | only semantic frames are safe latest-wins data |
| `TP-FMP-RENDER-01` | rapid clicks with semantic client and one-slot server writer | intermediate frames coalesce; final frame is sent; queue does not grow | preserves the server's existing bounded render contract |
| `TP-FMP-RENDER-02` | slow client/backpressure fixture | server state and input processing remain bounded and newest semantic frame remains pending | client pressure must not freeze server state |
| `TP-FMP-SCALE-01` | large files inside a high-entry-count directory | listing opens no file body and serializes only name/path/kind/mtime/icon key metadata | rejects the “hundreds of GB transferred” hypothesis |
| `TP-FMP-SCALE-02` | calibrated 100k-entry directory | first useful rows, total memory, cancellation, and final sorted result meet an explicit budget before chunking is authorized | Yazi-style `Part/Done` is conditional, not speculative |
| `TP-FMP-VIS-01` | complete Files Layout V1 Playwright Chromium suite | all existing PNGs remain byte/semantic-equivalent | performance work cannot silently create Layout V2 |
| `TP-FMP-E2E-01` | isolated real mouse rapid ancestor/location burst with profile enabled | raw interaction, profiler windows, final visible path, and cleanup evidence are captured | deterministic tests cannot prove terminal/client presentation alone |
| `TP-FMP-GATE-01` | focused/full Rust, both Clippy targets, maintenance, Chromium, hygiene, graph, Git | all pass with fresh evidence | cross-loop optimization has broad regression risk |

## Dependency Chain

```text
FMP-0 Layout V1 freeze
  -> FMP-1 observability + archived isolated evidence
  -> FMP-2 deterministic reproduction matrix
  -> FMP-3 measured root-cause decision
  -> FMP-4A resident disk-free Trail activation
  -> FMP-4B nonresident Trail activation on bounded latest worker
  -> FMP-4C client input/control/semantic-frame lane separation
  -> FMP-4D optional directory chunk/cache work only after SCALE-02 RED
  -> FMP-5 adversarial, Chromium, full-gate, graph, and publication closure
```

FMP-4A through FMP-4C are separate RED/GREEN and rollback boundaries. FMP-4D
is not authorized by directory size alone: it requires a calibrated
high-entry-count RED. No recursive file contents are ever part of listing
delivery.

## FMP-4D Scale Decision

The explicit debug-profile calibration at `631f213f` passed without a separate
scale RED:

- 100,000 immediate-child entries;
- four 256 GiB sparse files, 1 TiB logical body size in total;
- one warm-up plus five measured complete snapshots;
- samples `763/764/781/788/802 ms`, p95 `802 ms`;
- retained metadata lower bound `14,800,000` bytes;
- budgets: p95 `2,000 ms`, metadata `64 MiB`.

Partial listing, viewport-first enrichment, and a directory cache are therefore
not authorized in this slice. The worker boundary already keeps input
responsive while the final globally mtime-sorted snapshot is prepared. Reopen
FMP-4D only if a reproducible target filesystem exceeds a budget or a separate
time-to-first-row RED is established. Full evidence and host conditions are in
`.codex/evidence/files-rapid-navigation-scale-calibration.md`.

## Candidate Optimization Classes

These are options to measure, not pre-approved implementations:

1. reuse exact resident Trail snapshots without filesystem or preview reads;
2. send nonresident Trail activation through the existing
   one-executing/one-latest-pending worker and reject stale completion;
3. split lossless input, ordered control, and latest-only semantic-frame
   client lanes with a bounded fairness quantum;
4. move file detail/body preview preparation off mouse dispatch;
5. only after a measured scale RED, add bounded `Start/Part/Done` listing,
   cancellation tickets, viewport-first enrichment, and a byte/entry-budgeted
   directory cache.

## Production Reference Diff

The strongest reference is Yazi v26.5.6:

- directory listing and previews are asynchronous and discardable;
- partial batches carry a ticket so stale results are rejected;
- expensive MIME/preview work is page-oriented and lower priority;
- render requests are merged rather than emitted for every intermediate
  action.

Herdr already exceeds some reference implementations in stale-result safety:
its FM worker is bounded and validates generation, source, target, and model
revision. The defect is that Trail click activation bypasses that worker.

Ranger contributes bounded cooperative work slices and cached directory
objects; Broot contributes the rule that new input invalidates older expensive
work. Joshuto and Superfile are useful comparisons but both still contain
full synchronous directory enumeration paths and weaker generation
contracts, so they are not the target architecture.

Primary sources:

- https://yazi-rs.github.io/blog/why-is-yazi-fast/
- https://github.com/sxyazi/yazi/tree/v26.5.6
- https://github.com/ranger/ranger/blob/master/ranger/core/loader.py
- https://github.com/Canop/broot
- https://github.com/kamiyaa/joshuto
- https://github.com/yorukot/superfile

Every selected optimization requires a behavior-specific failing test. A
change that merely lowers a benchmark without reproducing the user's delayed
interaction is not sufficient.

## Failure Memory

- Do not classify delayed presentation as dropped mouse input without raw
  input and dispatch evidence.
- Do not blame filesystem enumeration for resident-only clicks when the read
  counter has not been measured. The archived run proved the old zero-read
  assumption false.
- Do not add an unbounded event/frame queue to avoid dropping intermediate
  states.
- Do not coalesce click, Terminal ANSI delta, graphics, or ordered control
  messages as if they were complete semantic frames.
- Do not introduce an unbounded path cache. Any future cache requires entry
  and byte budgets, invalidation, and eviction tests.
- Do not sleep in deterministic tests to manufacture timing.
- Do not rewrite existing Playwright snapshots to make an optimization pass.
- Do not profile the installed stable Herdr process or inherited socket.

## Runtime Investigation

Use the durable local helper:

```bash
cd /home/ayaz/projects/herdr
bash .local/herdr-files-v1-profile.sh run
```

The helper must clean its owned throwaway root before start, use every required
`HERDR_*` unset, enable `HERDR_LOG=herdr=debug` and
`HERDR_RENDER_PROF=1`, preserve captured logs under the repository's
`.local/` area, semantically stop only its own server, and clean its throwaway
root after exit.

Stable Herdr, its socket, existing Ghostty windows, browsers, editors, and
unrelated user processes remain outside the investigation's authority.

The Native Files versus Claude/Codex symlink classification is frozen in
`.codex/evidence/files-layout-v1-symlink-audit.md`. Machine-global continuity
or skill symlinks are not modified as part of FMP.

## Acceptance

The performance program closes only when:

1. the lag is reproduced or explicitly bounded under a documented workload;
2. the violated stage is identified from real metrics;
3. the chosen optimization has observed RED and minimal GREEN evidence;
4. final destination/path authority remains exact under burst, stale, worker,
   watcher, close/reopen, and slow-client cases;
5. Layout V1 Chromium baselines remain unchanged;
6. fresh full cross-platform and maintenance gates pass;
7. stable runtime and user sessions remain untouched;
8. continuity, graph, Git ancestry, and CyPack remote SHA evidence are exact.
