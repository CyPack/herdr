# Herdr Files Rapid Navigation Latency PRD

## Status

- Program: `FMP — Files Mouse and Navigation Performance`
- Date: 2026-07-19
- Product baseline: locked `Files Layout V1`
- Trigger: user-observed delayed mouse response and temporary freezing during
  rapid reverse/ancestor and locations-rail clicking
- Current phase: investigation and observability
- Product optimization authorization: conditional on measured root cause,
  behavior-specific RED, and preserved Layout V1 oracles
- Required visual verifier: Playwright Chromium

## Problem

When the user rapidly clicks backward through already visible Miller columns,
or quickly clicks several rows/locations in succession, Native Files can feel
temporarily frozen. Mouse events appear to be recognized late and visible
selection/navigation catches up after the interaction burst.

This report is not yet evidence of dropped mouse bytes, slow filesystem I/O,
or a specific render defect. The investigation must distinguish:

1. input arrival latency;
2. input dispatch and Trail mutation latency;
3. bounded worker request/result latency;
4. view projection and Ratatui render latency;
5. frame encoding, queueing, and client presentation latency.

No optimization may be selected from the symptom alone.

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

Fresh Codebase Memory at intake reports 23,854 nodes / 124,093 edges and
returns the current FCL symbols.

The current hot path is serial at the server boundary:

```text
client input batch
  -> HeadlessServer::handle_client_input_events
  -> App::route_client_events
  -> App::handle_file_manager_mouse
  -> FmState::activate_trail_entry OR typed location request
  -> headless scheduled sync
  -> optional bounded FM worker result apply
  -> full-render decision
  -> HeadlessServer::render_and_stream
  -> one-slot client render queue
```

Confirmed properties:

- every interactive client batch requests a post-input semantic frame;
- server events and changed scheduled tasks force a full render;
- `render_and_stream` builds a Ratatui frame for each target client;
- a full render uses a one-slot non-blocking writer queue and records
  `full_render.queue_full` when it cannot send the current frame;
- Trail row activation mutates the resident Trail synchronously and then
  linearly locates the activated path in the operation projection;
- already resident location roots reset synchronously without filesystem I/O;
- non-resident root, Miller navigation, and refresh preparation use the
  bounded one-executing/one-latest-pending FM worker;
- the existing opt-in profiler records render, encode, queue, PTY, watcher,
  and filesystem counters, but not click dispatch, Trail activation, resident
  reset, FM submit latency, or result-apply duration.

## Ranked Hypotheses

### H1 — Full-frame work and client queue pressure

Probability: high.

Rapid clicks create a sequence of valid intermediate states. Each input batch
can force full view projection, frame construction, serialization, and a send
attempt. The single-slot writer queue may defer frames while the input stream
continues. The user then sees an older frame and perceives the pointer as late
even if raw mouse input arrived on time.

Falsifier: click bursts show low `full_render.total`, no queue-full events, and
presentation latency still grows before dispatch.

### H2 — Synchronous resident Trail projection cost

Probability: medium-high.

Backward/ancestor clicks operate on resident snapshots, so they bypass
filesystem I/O but synchronously rebranch/reproject Trail state. The current
activation path also performs exact-path linear searches. Cost may grow with
deep Trails, large columns, grouped rows, and repeated intermediate clicks.

Falsifier: click dispatch and activation p95 remain within budget under deep,
large deterministic fixtures while presentation still lags.

### H3 — Serial scheduled work competes with new input

Probability: medium.

The headless loop drains client events, then synchronizes FM results and other
scheduled work before rendering. Worker completion, watcher reconciliation,
preview work, and repeated full-render decisions may consume the same serial
loop budget during an interaction burst even though directory reads themselves
are off-thread.

Falsifier: scheduled/apply durations and counts remain bounded while input
dispatch latency grows independently.

### H4 — Filesystem read backlog

Probability: low for resident ancestor clicks, higher for rapid non-resident
location switching.

The FM worker is bounded and latest-pending, so it cannot grow an unbounded
directory-read queue. It can still spend time on one executing cold/large
directory while newer destinations replace only the pending request.

Falsifier: the reported resident-only reproduction produces zero
`fm.filesystem.read` events during the slow interval.

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
input batch received
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
| `TP-FMP-RES-01` | alternate through deep resident ancestor columns | final exact row/path wins; zero filesystem reads; invariants hold | matches the primary reported reproduction |
| `TP-FMP-RES-02` | repeatedly activate current resident location | zero worker reads and bounded reset time | isolates the location fast path |
| `TP-FMP-IO-01` | blocked executing root plus two newer locations | only latest pending destination runs/applies; input dispatch stays responsive | preserves bounded latest-wins behavior |
| `TP-FMP-IO-02` | worker result races with a new click, watcher tick, and close/reopen | stale result rejects; latest exact state remains authoritative | race handling cannot trade correctness for speed |
| `TP-FMP-SYM-01` | compare resident/non-resident real directories with local directory symlinks; include dangling/changed-type failure | resident paths perform zero reads, bounded worker owns non-resident reads, exact symlink identity survives, and failures preserve the Trail | separates Native Files symlink I/O from unrelated Claude/Codex pointer health |
| `TP-FMP-RENDER-01` | rapid clicks with semantic client and one-slot writer queue | intermediate frames may coalesce; final frame is sent; queue does not grow | tests the strongest hypothesis |
| `TP-FMP-RENDER-02` | slow client/backpressure fixture | input processing remains bounded and newest frame remains pending | client pressure must not freeze server state |
| `TP-FMP-VIS-01` | complete Files Layout V1 Playwright Chromium suite | all existing PNGs remain byte/semantic-equivalent | performance work cannot silently create Layout V2 |
| `TP-FMP-E2E-01` | isolated real mouse rapid ancestor/location burst with profile enabled | raw interaction, profiler windows, final visible path, and cleanup evidence are captured | deterministic tests cannot prove terminal/client presentation alone |
| `TP-FMP-GATE-01` | focused/full Rust, both Clippy targets, maintenance, Chromium, hygiene, graph, Git | all pass with fresh evidence | cross-loop optimization has broad regression risk |

## Dependency Chain

```text
FMP-0 Layout V1 freeze
  -> FMP-1 observability characterization
  -> FMP-2 isolated reproduction matrix
  -> FMP-3 root-cause decision
  -> FMP-4 one measured TDD optimization slice
  -> FMP-5 adversarial, Chromium, full-gate, graph, and publication closure
```

No production optimization begins before FMP-1 and FMP-2 identify which
latency stage violates its budget. If multiple stages fail independently,
each becomes a separate RED/GREEN slice and rollback boundary.

## Candidate Optimization Classes

These are options to measure, not pre-approved implementations:

1. coalesce superseded Files-only intermediate frame requests while preserving
   the final state and non-Files events;
2. remove redundant Trail/operation-projection scans by carrying validated
   indices through the existing typed frame identity;
3. cache or incrementally project unchanged Files geometry/rows;
4. cap scheduled FM apply work per loop tick while preserving latest-result
   correctness;
5. keep final-frame delivery independent of a slow client writer.

Every selected optimization requires a behavior-specific failing test. A
change that merely lowers a benchmark without reproducing the user's delayed
interaction is not sufficient.

## Failure Memory

- Do not classify delayed presentation as dropped mouse input without raw
  input and dispatch evidence.
- Do not blame filesystem enumeration for resident-only clicks when the read
  counter is zero.
- Do not add an unbounded event/frame queue to avoid dropping intermediate
  states.
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
