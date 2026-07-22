# Files Performance Fix Closure and Navigation Follow-ups

## Status

- Date: 2026-07-21
- Branch: `feat/native-fm`
- Product head: `d8583d3ab564d42f880e94e0462f9d12ab61d391`
- CyPack branch at capture: same SHA
- Main rapid-click/mouse-motion stutter: **closed and human-accepted**
- Follow-up program: `FMN — Files Movement Semantics and Wheel Normalization`
- Stable Herdr and user-owned `.superpowers/`: outside authority and untouched

The user completed a live Native Files trial and reported that the freezing and
stutter appear completely gone and that the interaction now works very well.
This is the required human acceptance signal for the reported symptom. It is
qualitative evidence, not a substitute for the executable gates below.

The same live-testing sequence previously recorded two bounded burst results:

- 200 rapid file clicks produced one full render;
- 600 inert mouse-move events produced one render attempt.

Those counts are session/operator evidence. The structural tests and profiler
contracts remain the regression authority.

## Complete root-cause stack

The final fix was not one cache or one throttle. Four distinct defects and one
test-harness weakness were separated:

1. Directory activation could perform synchronous child/parent/preview reads on
   the serial server input loop.
2. Input and presentation shared blocking client work, so a blocked server
   amplified delay into visible head-of-line latency.
3. Selecting an already resident file rebuilt its parent projection from disk.
4. Inert mouse movement requested full virtual frames even though the visible
   file-manager state did not change.
5. Text preview preparation could still read bytes while applying the file
   selection on the input loop.
6. Filesystem-time-sensitive fixtures could change order under full-suite
   parallelism and obscure the real result.

The original isolated profile remains the root-cause evidence: all 7,411 client
and server input signatures matched, but parse-to-server p95 reached
7,587.645 ms and major server silences followed primary mouse-down. The mouse
and Ghostty were not dropping events; the serial server loop was blocked and the
client queue amplified it.

## Atomic production and test chain

### Earlier FMP foundation

| Boundary | RED | GREEN/decision | Result |
|---|---|---|---|
| Resident directory activation | `c1ced923` | `a030fa76` | Rebranch from resident Trail snapshots without filesystem reads |
| Cold directory activation | `8dfe03c2` | `abbaef91` | One-executing/one-latest-pending worker; stale completion rejected |
| Client head-of-line pressure | `f0b3964e` | `0b94447b` | Lossless input, ordered control, latest semantic frame, bounded fairness |
| 100k scale policy | `631f213f` | `e5dcbb20` | No cache/chunking RED; general cache rejected |

### Final residual fixes

| Commit | Change | Protected failure paths |
|---|---|---|
| `b2accbb4` — `fix: reuse resident parent snapshot for file selection projection` | File selection projects cwd/entries/status/writability from the owning resident Trail snapshot | exact column/index/path revalidation; root file has no fabricated parent; zero filesystem reads |
| `8851b5e0` — `perf: skip renders for inert file manager mouse moves` | `route_client_events` reports render need; inert pointer motion can return false | overlay hover still renders; every non-move mouse event renders; keyboard/paste renders; drag/resize/pane motion semantics remain owned |
| `ed329058` — `perf: load file selection previews off the input loop` | Text preview uses a discardable background worker and explicit pending/prepared/unavailable state | first+latest-only, stale Files generation/path/preview generation, missing, invalid UTF-8, panic/disconnect, close/reopen, worker teardown |
| `d8583d3a` — `test: stabilize filesystem-time-sensitive fixtures` | Order-insensitive fixtures receive explicit equal mtimes or wait for a distinct timestamp | production sorting and order validation stay strict; only nondeterministic fixture authority changes |

### Publication-gate discovery after `d8583d3a`

The documentation publication gate exposed one remaining fixture from the
same mtime class:

- full Nextest run `3357db2f-ce29-42cb-a2a4-794dfffcd1f1` failed
  `cursor_move_does_not_reuse_highlight_for_equal_content_at_new_path`;
- actual landed path was `alpha.rs`, while the fixture expected `beta.py`;
- the test created both files and assumed natural alpha-to-beta order, but
  production intentionally sorts modification time descending before name;
- one immediate rerun and 20/20 repeated exact runs passed, proving the
  filesystem/scheduler-sensitive ordering precondition rather than a
  deterministic stale-highlight production failure;
- the minimum test-only correction ties both mtimes with the existing
  `set_equal_modified` helper before constructing `FmState`;
- post-fix exact repetition is 20/20 and the five-test highlight/path family is
  5/5;
- fresh full Nextest run `4dd813f9-9d36-42c0-9ba3-d50dfbd33904` is
  3,599/3,599 passed with four intentional skips.

The test-only correction is commit `8f4b2acc` —
`test: stabilize text preview path identity fixture`. Production sort and
preview behavior are unchanged.

Notable new regression tests include:

- `resident_file_selection_projects_without_filesystem_reads`;
- `root_file_selection_is_disk_free_and_has_no_parent`;
- `inert_mouse_move_declines_render`;
- `mouse_move_over_blocking_overlay_requests_render`;
- `non_move_mouse_events_always_request_render`;
- `keyboard_and_paste_always_request_render`;
- `pending_preview_worker_executes_first_and_latest_only`;
- `app_resolves_pending_file_selection_off_thread_once`;
- missing, invalid-UTF-8, disconnect, scheduled-pump, loading-render, and
  inline-byte-read prevention tests.

## Architectural lessons now frozen

### L1 — Input occurred is not render authority

An event is evidence that the router must inspect state, not evidence that the
frame changed. Render authority comes from a visible mutation or a surface that
owns hover/motion. This matches Yazi's inert plain move and revision-driven
render requests without copying its render cadence.

### L2 — Resident model state is the first cache

If the exact owner column already contains entries, status, writability, and
path identity, selection projection must reuse it. Re-enumerating the same
directory is both slower and less coherent because watcher state can change
between two reads in one input transaction.

### L3 — Every filesystem byte/metadata read needs an explicit scheduler owner

Directory enumeration and text preview cannot hide inside a reducer named
“select,” “activate,” or “project.” The synchronous reducer owns identity and
state transition only. The bounded worker owns fallible filesystem work.

### L4 — Bounded latest-wins applies only to replaceable state

Complete semantic frames, directory previews, and file previews can be
superseded when their exact identity is stale. Input events, Terminal ANSI
deltas, graphics, clipboard, shutdown, title, capture, and input-source control
remain ordered/lossless.

### L5 — Cancellation without authority validation is insufficient

Abort is an optimization, not correctness. Completion must still validate the
current Files instance generation, model/source identity, owner column, exact
path, and request generation before applying.

### L6 — Performance gates need both structure and a human symptom check

Deterministic tests prove no read/no render/bounded queue properties. A
profiled isolated runtime proves the terminal-client-server path. The user's
live report closes the original felt-latency symptom. None of the three replaces
the others.

### L7 — Filesystem-time fixtures need deterministic time authority

Tests that care about identity but not time order must explicitly equalize
mtimes. Tests that care about order must create provably distinct mtimes. Never
weaken production ordering to stabilize a fixture.

## Isolated live-test error handling

The ignored local helper `.local/herdr-fm-live-test.sh` is the current one-command
operator harness. Its reusable safety pattern is:

1. hard-code a small allowlist of exact `/tmp` roots;
2. reject symlink roots, non-directory roots, foreign ownership, and missing
   ownership markers before deletion;
3. unset both Herdr socket variables and all inherited Herdr
   pane/workspace/tab/environment identities;
4. use throwaway XDG config/state/runtime roots with mode 0700;
5. semantically request `server stop` through the same isolated environment;
6. wait a bounded five seconds for the exact socket to disappear;
7. refuse deletion while the socket remains;
8. install an EXIT trap for normal exit, error, and Ctrl-C cleanup;
9. remove the trap before finalization to prevent recursive cleanup;
10. preserve the original command status unless cleanup itself fails;
11. print bounded aggregate profiler counters without path or terminal data;
12. make the next run cleanup-first so a machine crash is recoverable.

Shell functions used under `set -euo pipefail` explicitly return failure after a
failed guard. Relying on `set -e` alone inside `if`, `!`, traps, or conditional
function calls can silently continue after an unsafe precondition.

Stable `~/.config/herdr`, the installed stable binary, and user processes are
absent from the cleanup allowlist.

## New user-confirmed bugs

These are separate from the closed stutter. They do not reopen FMP.

### FMN-B1 — One wheel gesture appears to skip 3–5 files

Observed behavior: a small/rapid vertical wheel action can visibly advance
several entries at once.

Current evidence does not yet distinguish:

- H1: Ghostty/crossterm emits several `ScrollUp`/`ScrollDown` events for one
  physical high-resolution or momentum gesture;
- H2: one decoded event is dispatched more than once;
- H3: the first event lands on a directory, automatically branches, and the
  remaining burst is applied to the child column, creating a larger perceived
  jump.

H3 is strongly supported by the current reducer. H2 is lower probability
because the mouse match arms are mutually exclusive, but it must be falsified
with counters rather than assumed. H1 requires an isolated raw-event trace.

### FMN-B2 — Up/Down automatically continues into a directory child

Observed behavior: while the user holds Down or Up, landing on a directory
changes navigation into its right-side child column without pressing Right or
Enter.

Root cause is source-confirmed. `move_selection_in_column` computes the landed
row and immediately calls `activate_entry`; directory activation branches and
rearms horizontal follow. This is current intended-by-code behavior but wrong
product behavior under the user's clarified contract.

## Frozen interaction contract

1. Up/Down and `j/k` move exactly one row in the current active column.
2. Shift+Up/Down extends selection in that same column and never branches.
3. Vertical wheel over a row moves the cursor in that row's exact owner column;
   it never transfers the active column merely because the landed item is a
   directory.
4. Landing on a directory may update a right-side preview asynchronously, but
   the cursor and active-column authority stay in the owner column.
5. Right/`l`, Enter, or an explicit primary directory click is required to
   enter/rebranch.
6. Left/Backspace remains the explicit parent/previous-column action.
7. Clamped movement is inert and should not force unnecessary full renders.
8. A stale row/header geometry identity is consumed without mutation.
9. No navigation input performs directory or file-body I/O on the serial input
   loop.
10. A wheel normalization policy is authorized only after physical gesture,
    decoded-event, dispatch, and reducer-mutation counts identify the duplicate
    stage.

## Test-point catalog for FMN

| ID | RED / verification | Required result |
|---|---|---|
| `TP-FMN-OBS-01` | one isolated physical wheel gesture with raw parser, dispatch, and cursor-mutation counters | identify whether multiplicity begins at terminal decode, Herdr dispatch, or reducer apply; no path/content logging |
| `TP-FMN-OBS-02` | held Down/Up across file-directory-file fixtures | record owner column and cursor path for every accepted step; no implicit active-column change |
| `TP-FMN-NAV-01` | Down/`j` lands on a directory | cursor moves one row; owner/active column is unchanged; no branch |
| `TP-FMN-NAV-02` | Up/`k` lands on a directory | same cursor-only invariant in reverse |
| `TP-FMN-NAV-03` | Shift+vertical movement across a directory | selection extends in the owner column without branch or rollback corruption |
| `TP-FMN-NAV-04` | explicit Right/`l` and Enter on a directory | branch occurs exactly once and focuses the intended child |
| `TP-FMN-NAV-05` | explicit primary click on a directory | bounded worker owns cold activation; exact path applies once |
| `TP-FMN-NAV-06` | file, symlink-dir, broken symlink, special, empty and clamped edges | cursor semantics are deterministic; only explicit actionable directory activation branches |
| `TP-FMN-WHEEL-01` | one decoded row-wheel event | exactly one owner-column cursor step and no branch |
| `TP-FMN-WHEEL-02` | burst over rows containing directories | all accepted steps remain in the original owner column |
| `TP-FMN-WHEEL-03` | row/header/empty-body/detail/rail/outside geometry | only the exact current-frame owner handles the event; horizontal fallback remains disjoint |
| `TP-FMN-WHEEL-04` | deliberate continuous scroll versus one high-resolution gesture | normalization removes accidental burst multiplication without making intentional scrolling sticky or dropping direction changes |
| `TP-FMN-IO-01` | cursor lands on nonresident directory with blocked reader | input returns immediately; optional preview is bounded/latest; focus stays in owner column |
| `TP-FMN-IO-02` | preview races newer cursor, explicit activation, watcher, location switch, close/reopen | stale completion is inert and cannot steal focus |
| `TP-FMN-IO-03` | missing, permission, changed-type, panic, disconnect | resident Trail and cursor survive; bounded unavailable state; no retry loop |
| `TP-FMN-RENDER-01` | clamped/inert move and real cursor/preview transition | inert action declines render; visible transition requests bounded render |
| `TP-FMN-VIS-01` | Layout V1 Chromium fixtures plus cursor/preview cells | no unreviewed layout drift; any legacy PNG update is individually scoped and inspected |
| `TP-FMN-E2E-01` | isolated Ghostty wheel and held-arrow trial | no 3–5 accidental jump and no child focus transfer; cleanup leaves zero test residue |
| `TP-FMN-GATE-01` | focused/full Rust, Linux/Windows Clippy, maintenance, Chromium, graph, Git | all fresh and clean before CyPack-only publication |

## Dependency order

```text
FMN-0 freeze report, source diff, and test contract
  -> FMN-1 observe physical wheel -> decoded event -> dispatch -> mutation
  -> FMN-2 RED cursor-only vertical navigation and explicit activation
  -> FMN-3 minimal reducer/preview GREEN with bounded stale-result authority
  -> FMN-4 wheel normalization only if FMN-1 still proves a separate burst bug
  -> FMN-5 adversarial/full/Chromium/isolated-live closure
  -> FMN-6 pinned-location pre-warm only after an independent latency RED
```

Do not implement FMN-4 first. Decoupling movement from activation may eliminate
the cross-column part of the apparent skip, and raw-event evidence is required
before choosing coalescing, accumulation, rate limiting, or no normalization.

## FMN implementation and verification — 2026-07-21

### Observation result

The isolated Ghostty trace recorded 333 vertical wheel packets. Of those,
226 consecutive same-direction deltas were below 2 ms. The raw SGR packets
repeated the same coordinates in triplets and occasional sextuplets, commonly
around 0.02–0.4 ms apart; the next independent groups were normally at least
5 ms apart. The server/client call chain and route tests remained one-to-one.

Verdict:

- H1 terminal/host micro-burst: confirmed;
- H2 duplicate Herdr dispatch: rejected;
- H3 automatic branch amplification: confirmed by the old reducer and the
  original RED that transferred `active_col` on a directory landing.

The authorized normalization is therefore intentionally narrower than a
generic debounce: only the same Files generation, Trail owner column,
direction, pointer coordinates, and a delta strictly below 2 ms coalesce.
Reversal, owner change, coordinate change, the exact 2 ms boundary, and the
observed 5 ms next detent remain independent semantic input. Every raw packet
advances the gate timestamp so a dense train cannot leak every Nth duplicate.
The state is one fixed-size stamp; there is no sleep, queue, retry, history, or
unbounded accumulator.

### Implemented state and authority split

- `TrailState` carries an ephemeral exact-path cursor separately from
  `TrailCol::selected`, which remains the activated directory-chain identity.
- `TrailSnapshots::move_cursor[_in_column]` moves one clamped row and reports
  entry index/kind without calling activation, truncating the chain, or moving
  the active column.
- Up/Down/`j/k`, Shift+vertical, row wheel, and header wheel use that
  cursor-only reducer. Right/`l`, Enter, and primary click remain the only
  directory activation commands.
- A directory cursor landing schedules `TrailPreview` through the existing
  one-running/one-latest `FileManagerIoWorker`. The serial input loop performs
  no cold directory read.
- Preview apply requires the original worker identity plus the current Files
  generation, source, owner column, entry index, exact path, directory kind,
  and active cursor identity. A newer cursor, horizontal focus change,
  activation, location/model change, missing target, or failed preparation is
  inert and cannot steal focus.
- Render projection highlights the cursor exact path and temporarily hides a
  mismatched stale child until the matching preview arrives. Clamped keyboard
  or wheel input and coalesced packets explicitly decline render.
- Profiler counters `fm.vertical_wheel.accepted` and
  `fm.vertical_wheel.coalesced` expose the live normalization without logging
  user paths or content.

### RED/GREEN and failure evidence

- The first cursor-only navigation RED failed because a directory landing
  changed `active_col`; GREEN keeps every accepted vertical step in the owner.
- The async preview RED initially had no worker request, then proved bounded
  off-loop preparation and owner-focus preservation after the request path was
  wired.
- The stale-preview RED applied after a horizontal focus change; GREEN adds
  current active cursor/column authority.
- The wheel RED first failed on the absent gate, then the end-to-end triplet
  fixture proved one semantic row plus preserved 5 ms detent and reversal.
- Clamped wheel and keyboard REDs requested renders; GREEN consumes them while
  declining the frame.
- A final Shift+Up edge RED returned `Consumed` despite preserving the same
  cursor and selection. GREEN returns `Inert` when range extension rolls back
  or a clamped cursor leaves the explicit set unchanged, while a real
  directory-crossing range extension still renders.
- A worker test wait initially accepted an older result-slot generation. The
  test seam now waits for `latest_generation`, preventing false completion or
  hangs without changing production scheduling.
- Missing directory preview preserves the exact cursor, resident branch, and
  owner focus. Shift+vertical across a directory extends selection without
  branching.
- Initial/resync integration originally reused `select_dir`, which prepared the
  resident child but also left `active_col` on that child. The exact RED proved
  this recreated the reported auto-entry before the first keypress. GREEN keeps
  the child resident and explicitly restores the parent owner.
- Trail auto-follow and the compatibility resize projection initially used
  `deepest()` as focus authority. That value is only the resident-data extent;
  all rendered, hit-tested, and resize geometry now follows `active_col()`.
  The 10,000-action invariant caught the otherwise hidden divider mismatch.
- A watcher characterization expected an explicit Leave/Backspace to remain
  bound to the resident child. The clarified product law says Leave changes the
  owner to the parent, so the test now requires exactly one parent rebind and a
  stable second sync.
- The full suite rejected a legacy narrow-layout assertion that expected the
  resident child's `preview.txt` before Right. The corrected full-frame test
  proves the root rows remain visible, explicit Right reveals the child, and
  explicit Left restores the root owner.

### Fresh gate ledger

| Gate | Fresh result |
|---|---|
| Focused FMN/Files family | 302/302 passed; run `a718ae73-da90-4155-91a6-3f336f2149e5` |
| Active-owner/full-frame regression trio | 3/3 passed; run `4cb8b23e-94ec-4420-9b5a-0e9e8186aafc` |
| Full Nextest | 3,619/3,619 passed, 4 intentionally skipped; run `d42e9919-f7dd-4f43-b855-a0ed4fd6922e` |
| `cargo fmt --all -- --check` | clean |
| Linux all-target Clippy `-D warnings` | clean |
| Windows MSVC Clippy, SIMD disabled | clean |
| Python maintenance | 68/68 passed |
| Integration assets Bun | 5/5 passed |
| Plugin marketplace Bun | 12/12 passed |
| Deterministic Ratatui exporter | 1/1 passed; all generated JSON stayed clean |
| Full Chromium | 33/33 passed |
| Snapshot scope | exactly six reviewed legacy PNGs: VIS-01 through VIS-06; no other PNG changed |

Earlier exhaustive runs exposed real contract drift instead of being waved
away: first the watcher expectation, then the auxiliary resize projection, and
finally the legacy narrow-layout assertion. After those exact RED/GREEN loops,
run `d42e9919-f7dd-4f43-b855-a0ed4fd6922e` completed in 38.430 s with
3,619/3,619 passes and 4 intentional skips. The exporter now injects a fixed
calendar anchor, equalizes order-insensitive mtimes, settles async preview by
exact identity/generation, and uses no-follow timestamp handling for symlink/
FIFO fixtures. The six VIS-01..06 pixel updates were reviewed individually;
VIS-07..25 and every generated JSON remained unchanged.

`TP-FMN-E2E-01` remains intentionally open for the user's new isolated build:
slow physical wheel, held Up/Down across file-directory-file rows, explicit
Right/Enter activation, semantic exit, and zero throwaway residue. Graph
refresh, exact-path staging, commit, CyPack push, and remote SHA equality also
remain publication-time gates; none is represented as complete here.

Working-tree graph refresh completed at 24,072 nodes / 129,692 edges. Current
symbol queries resolve `move_trail_cursor_in_column`,
`FileManagerVerticalWheelBurstGate`,
`queue_file_manager_trail_directory_preview_identity`,
`project_miller_view_with_resize_preview`, and the active-owner regression
tests at their live source paths. A post-commit SHA-bound recheck is still
required before publication closure.

## Cache and pre-warm decision

Home/Desktop/Downloads pre-warm remains a separate measurement-first lane.
The current evidence does not show a first-entry latency violation, while the
100k-entry calibration does show a meaningful memory cost. If the new
cursor-only preview path is responsive, no pre-warm may be needed. If a live
profile establishes a RED, use an allowlisted background pre-warm with
per-directory entry/byte caps and mtime invalidation. Never adopt Yazi's
unbounded history map as a general Herdr LRU.

## Verification ledger

`just` is not installed on this host. The 2026-07-21 publication gate therefore
read `justfile` and executed the complete `just check` dependency recipe
directly; the missing wrapper itself is not reported as a pass.

| Gate | Fresh result |
|---|---|
| `cargo fmt --check` | clean |
| Linux `cargo clippy --all-targets --locked -- -D warnings` | clean |
| Full Nextest | 3,599/3,599 passed, 4 skipped; run `4dd813f9-9d36-42c0-9ba3-d50dfbd33904` |
| Integration asset Bun | 5/5 passed |
| Plugin marketplace Bun | 12/12 passed |
| Windows MSVC Clippy with `LIBGHOSTTY_VT_SIMD=false` | clean |
| Python maintenance | 68/68 passed |
| Gate-discovered exact test repetition | 20/20 passed |
| Related highlight/path family | 5/5 passed |

The earlier full run `3357db2f-ce29-42cb-a2a4-794dfffcd1f1` is retained above
as RED/failure evidence and is not counted as a pass. Final Markdown, task-copy,
Git diff, staging, ancestry, and remote-SHA checks remain publication-time
gates.

## Related evidence

- `.codex/references/yazi-file-manager-performance-transfer.md`
- `.codex/evidence/files-rapid-navigation-root-cause.md`
- `.codex/evidence/files-rapid-navigation-scale-calibration.md`
- `docs/superpowers/specs/2026-07-19-herdr-files-rapid-navigation-latency-prd.md`
- `.local/herdr-fm-live-test.sh` (ignored, machine-local operator helper)
