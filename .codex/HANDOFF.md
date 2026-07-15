# SESSION HANDOFF — Herdr Native FM — 2026-07-15

## 1. SONRAKI ADIM

N2.1 is complete through RED `e433a2f` and GREEN `c530836`; preserve its exact
path-focus contract and verified fork publication. Start M1.0 discovery only:
compare C5 native handoff, CLI attach/send, pane focus, and plugin file actions,
then publish a terminating delta or NO-GO before UI/runtime code. The complete
M1 → M2 → M3 macro/micro roadmap and expected-result/reason test tables are in
`.codex/TASKS.md`; N2.2 and S5–S7 remain independently gated. Never touch
stable Herdr/socket/processes.

## 2. AKTİF PROJE

- Project: `/home/ayaz/projects/herdr`
- Branch: `feat/native-fm`
- Goal: native, Lua-free file manager on Herdr's composable TUI foundation.
- Acting GitHub identity: `CyPack` external contributor; fork-only writes.

## 3. KAYNAK OTURUM

- Claude resume ID: `f53c720f-f795-4778-970b-d227714ffb1a`
- Raw JSONL: `/home/ayaz/.claude/projects/-home-ayaz-projects-herdr/f53c720f-f795-4778-970b-d227714ffb1a.jsonl`
- SHA-256: `368fb0a5045d1435c64679c8d0dea2a4283d58891231c91bb6e30350b69c2d30`
- Span: `2026-07-14T00:19:57Z`–`02:41:40Z`
- Reconstruction: `.codex/evidence/claude-session-f53c720f.md`

## 4. CLAUDE OTURUMUNDA TAMAMLANANLAR

1. Onboarding, rust-dev lessons, codebase graph freshness audit, task restoration.
2. S2 named region extraction, commit `c043c1e`.
3. Isolated-development test documentation and Claude semantic hook.
4. S3 re-scope and A2.1 center-region directory render, commit `d026e94`.
5. `prefix+f` activation across the two-layer keybinding system, commit `74d3cc9`.
6. Keyboard navigation/input interception, commit `d2b27e6`.
7. Detailed native-FM next-session prompt generated.

## 5. BU CODEX OTURUMUNDA TAMAMLANANLAR

- Recovered and verified the Claude transcript directly from local storage.
- Implemented A2.2 responsive Miller columns with cached parent/preview state.
- Added RED tests first, then achieved full GREEN verification.
- Built this Codex CLI continuity, memory, skill, hook, launcher, and handoff package.
- Committed A2.2 as `6c7c58f`, reindexed it, and fast-forward pushed only the
  CyPack feature branch and fork master.
- Implemented A4 native file watching test-point-first: pure normalization,
  generation/lifecycle safety, bounded channel/coalescing, path-preserving
  refresh, real-filesystem convergence, explicit polling fallback, and a
  2-second reconciliation safety-net.
- Made two pre-existing wall-clock-sensitive tests deterministic after full
  nextest exposed them under parallel load.
- Committed A4 separately as `01ba91d` and the deterministic test-only fixes
  separately as `8cd4e89`, using targeted staging only.
- Completed B0 Image Path Beta test-point-first with generated exact RGBA,
  malformed decode, synthetic local placement, upload/display/dedup/view/
  replacement/removal lifecycle, cursor framing, and isolated real-host tests.
- Captured a visible local Path Beta pattern in throwaway Kitty and an
  independent Path Alpha Yazi preview baseline; closed only the test windows
  with targeted semantic input.
- Recorded a conditional GO for B2: reuse existing `kitty_graphics`, bound all
  decode/allocation work, keep I/O outside render, reject stale generations,
  and prove cleanup plus real-host output.
- Committed B0 separately as `bcba84d`, full-reindexed it, and fast-forward
  published only to CyPack feature/master.
- Closed B1.0 dependency research: minimal pure-Rust `syntect 5.3.0` is the
  B1.2 choice, but measured latency requires generation-safe bounded background
  preparation. B1.1 begins without adding it.
- Completed B1 text preview through strict RED/GREEN commits: 64 KiB bounded
  UTF-8 reader, explicit failures, state-refresh ownership, pure deterministic
  syntax preparation, one-active/one-pending generation-safe worker, reload
  identity preservation, lifecycle rejection, Ratatui style mapping, explicit
  error/truncation states, and bounded responsive render.
- Proved cursor navigation, watcher reload, selected-file replace/delete,
  hidden toggle, close/reopen, worker panic/disconnect, narrow/zero geometry,
  long lines, and stale-result rejection without touching stable Herdr.
- Re-ran the actual five-package dependency/OSV delta and the entire direct
  `just check` equivalent. B1 product/test head is `2b2dcd3`; continuity and
  graph/publication follow separately.
- Completed A3 remainder as seven atomic RED/GREEN/scope commits from
  `d713b71` through `9d69c82`: persistent cursor viewport, shared responsive
  CURRENT-row geometry, single/double-click and bounded wheel runtime routing,
  stale-target rejection, preview refresh, and explicit cursor-only v1 scope.
- Ran the full A3 gate and isolated real PTY SGR-mouse cross-check without
  stable Herdr/socket access; exited semantically and removed all throwaway
  state. Full-reindexed and published `9d69c82` to CyPack feature/master only.
- Completed B2 as an auditable range from dependency decision `de1eff5`
  through fallback fix `2989434`: bounded common-format decode/downscale,
  responsive client-local placement, generation-safe worker lifecycle,
  cached Kitty paint/dedup/transition cleanup, and explicit non-Kitty fallback.
- Ran B2/FM/Kitty 96/96, full nextest 2983/2983 plus one named B0 host-probe
  skip, Linux/Windows clippy, Bun 17/17, Python 64/64, fmt, and diff-check.
- Proved the production FM path in isolated Kitty: `assets/logo.png` rendered
  in PREVIEW with 0/271425 image-compare pixel differences, FM close cleared
  the same region to one background color, semantic exit returned code 0, and
  no test process/socket/temp root remained. Stable Herdr was untouched.
- Completed C1.1 test-first: RED contract `0ed5e51`, GREEN product `c9bfbf9`.
  Added client-local Copy/Paste/NewFolder/Delete tags, one responsive pure
  geometry seam, desktop/mobile `ViewState` snapshots, complete-button hiding,
  stale-area clearing, and render consumption without filesystem work.
- Full nextest exposed a pre-existing lifecycle fixture clock race. Proved the
  mixed real/synthetic timing boundary, fixed only the test base clock, and
  committed it separately as `9aa1e59`; no C1 product code was mixed into the
  stability commit.
- Ran the complete direct `just check` equivalent at the C1.1 tip: targeted
  4/4, lifecycle family 27/27, full nextest 2986/2986 plus one named B0 probe
  skip, Linux/Windows clippy, Bun 17/17, Python 64/64, fmt, and diff-check.
- Completed C1.2 test-first: RED `dbc6798`, GREEN `7fd01de`. Added a private
  `NotHandled`/`Consumed`/`HeaderAction(tag)` dispatch seam, exact unmodified
  left-click tag mapping, and fail-closed identity/gap/outside/hidden/zero/
  stale/non-left behavior without AppState/protocol/filesystem mutation.
- Ran exact dispatch 2/2, all FM input 13/13, full nextest 2988/2988 plus one
  named B0 probe skip, Linux/Windows clippy, Bun 17/17, Python 64/64, fmt, and
  diff-check. Cwd, cursor, and fixture disk entries remain unchanged by header
  tag dispatch.
- Completed N3.1 test-first: RED `b5cc95c`, GREEN `510eebc`. Added a pure
  selection/clipboard action-bar ViewState model, client-local clipboard paths,
  selected file/directory identity, explicit empty content, desktop/mobile
  refresh, and render fallback without filesystem or protocol coupling.
- Proved navigation, reload-selected-delete, empty transition, close/reopen,
  selected-name rendering, and clipboard-summary persistence. Gates: 3/3,
  FM 135/135, full 2991/2991 plus one named B0 skip, Linux/Windows clippy,
  Bun 17/17, Python 64/64, fmt/diff clean.
- Completed N3.2 test-first: RED `446613a`, GREEN `267ad91`. Added explicit
  per-action enabled/disabled authority and reasons, prepared cwd writability
  and regular-target support, distinct disabled rendering, and fail-closed
  input dispatch with no disabled-click state/filesystem mutation.
- Proved missing cwd, read-only reload, Unix special target, empty clipboard,
  absent selection, unsupported target, and in-flight precedence. Gates:
  exact authority/preparation/render/dispatch 7/7, broad FM/input/render/Kitty
  165/165, full 2996/2996 plus one named B0 skip, Linux/Windows clippy, Bun
  17/17, Python 64/64, fmt/diff clean.
- Completed C2.1 test-first: RED `d4d404e`, GREEN `9a15328`. Added client-local
  SendAgent/Rename/Delete row tags, one pure responsive name/action geometry
  snapshot, desktop/mobile ViewState lifecycle, and pure render consumption.
- The first 3-cell-per-action prototype passed focused tests but failed two
  broad readability characterizations by truncating ordinary names. Reworked
  complete buttons to one-cell `>`, `r`, and `x` targets; the focused plus
  readability set passed 8/8 and the FM impact set passed 71/71.
- Ran the complete direct `just check` equivalent at the C2.1 tip: full
  nextest 2998/2998 plus one named B0 probe skip, Linux all-target and canonical
  Windows MSVC bin clippy, Bun 17/17, Python 64/64, fmt, and diff-check clean.
  Fast graph reindex is fresh at 18,042 nodes / 84,123 edges and returns the
  new geometry/action symbols; freshness was not inferred from `ready` alone.
- Completed C2.2 test-first: RED `94e4a02`, GREEN `9ef90c6`. Row action
  snapshots now carry stable path identity, and exact unmodified-left dispatch
  requires the live index to match that path and remain operation-supported.
- Proved exact SendAgent/Rename/Delete tags, unchanged name selection,
  non-left/modifier/outside/hidden/closed fail-closed behavior, watcher-style
  reorder rejection, unsupported-target rejection, and zero cwd/cursor/
  clipboard/filesystem side effects. The outer router consumes tags before
  hidden terminal input but deliberately executes no real operation.
- C2.2 gates: exact 3/3, all FM input 17/17, FM impact 74/74, full nextest
  3001/3001 plus one named B0 probe skip, Linux/Windows clippy, Bun 17/17,
  Python 64/64, fmt/diff clean. Fast graph reindex is fresh at 18,049 nodes /
  83,839 edges and returned the current path-bearing area plus validation
  handler snippets; freshness was not inferred from `ready` alone.
- Completed N4.1 test-first as seven atomic commits: state `e876223`/`590e376`,
  lifecycle `1789bbd`/`5c14439`, gesture/render RED `699a6a6`, stable row
  identity RED `fc19237`, and integrated GREEN `86b618a`.
- Added a cursor-independent deduplicated path set and stable anchor, current-
  order inclusive range selection, reload/hidden pruning, enter/leave clearing,
  close/reopen reset, exact plain/Ctrl/Shift mouse gestures, Space and
  Shift+Up/Down keyboard equivalents, and distinct pure multi-row rendering.
- `FileManagerRowArea` now carries stable path identity; same-index watcher
  reorder is consumed without selecting the wrong live entry. Combined or
  unknown modifiers and stale targets fail closed. N4.1 performs no filesystem
  operation and adds no server or wire-protocol state.
- N4.1 gates: focused 7/7, broad FM/watcher/input/render/Kitty 137/137, full
  nextest 3015/3015 plus one named B0 probe skip, Linux/Windows clippy, Bun
  17/17, Python 64/64, fmt/diff clean. Fast graph reindex is fresh at 18,078
  nodes / 83,865 edges and returned live model/input/render connections.
- Completed N4.2 test-first as seven atomic commits: bulk authority RED
  `d5e027f`, ViewState lifecycle RED `0c76017`, bulk authority GREEN `0302b10`,
  bounded selection RED/GREEN `36c815f`/`57e2a44`, and keyboard overflow
  atomicity RED/GREEN `50619ff`/`cb5a43e`.
- Cursor focus no longer grants Copy/Delete authority. Prepared bulk paths
  retain current visible order; zero/one/many labels and file/directory/
  multiple/unavailable kinds are explicit. Stale/ambiguous or any unsupported
  member disables the complete selection; operation-in-flight has precedence.
- Added atomic 4,096-path select-all/range ceilings, Ctrl+A select-all,
  Ctrl+Shift+A clear, stale/duplicate rejection, and keyboard overflow cursor
  preservation. No partial set is silently selected and render remains pure.
- N4.2 gates: focused staged runs 6/6 + 4/4 + 2/2, broad FM/input/render
  112/112, full nextest 3020/3020 plus one named B0 probe skip, Linux/Windows
  clippy, Bun 17/17, Python 64/64, fmt/diff clean. Fast graph reindex is fresh
  at 18,091 nodes / 84,102 edges and returned current selection, builder, and
  keyboard source rather than relying on `ready`.
- Defined the complete C3 test-point contract in `d56e3db`, then completed
  C3.1 model RED/GREEN `5d6fc1d`/`02c60e7` and adversarial in-flight
  precedence RED/GREEN `d9f28b5`/`0832ccc`.
- Added the existing-stack `ContextMenuKind::File` model with deterministic
  Open/Copy/Rename/Delete/Compress/Send-to-Agent order, exact prepared paths,
  file/directory/multiple/unavailable kinds, and explicit item authority.
  No explicit selection produces no menu; mixed invalid selection fails closed.
- C3.1 gates: focused 5/5, combined menu models 7/7, full nextest 3025/3025
  plus one named B0 skip, Linux/Windows clippy, Bun 17/17, Python 64/64,
  fmt/diff clean. Fast graph reindex is fresh at 18,115 nodes / 84,003 edges
  and returned current constructor/variant source rather than relying on ready.
- Completed C3.2 as six atomic RED/GREEN commits: popup geometry
  `69864d6`/`ad5f8a5`, lifecycle/typed intent `73df647`/`45c151f`, and
  disabled render `1078215`/`0915964`.
- Exact current-row and row-action-cell right click now uses stable path
  identity, preserves selected bulk members, replaces unselected targets, and
  opens the existing globally bounded popup at every Miller breakpoint.
- Context-menu keyboard/mouse focus precedes the visible FM. Disabled rows
  remain dim and inert; enabled rows emit only typed client-local intent after
  current path/order/action authority revalidation. Reorder, delete,
  operation-in-flight, outside click, Esc, and FM close are fail-closed. C3.2
  executes no filesystem or agent operation.
- C3.2 gates: popup 4/4, lifecycle 3/3, render 1/1, broad FM/global-menu
  51/51, menu/render 26/26, full nextest 3033/3033 plus one named B0 skip,
  Linux/Windows clippy, Bun 17/17, Python 64/64, fmt/diff clean.
- Fast parallel graph refresh crashed codebase-memory 0.8.1 in native
  Tree-sitter cleanup. No service/process was restarted or killed. A supported
  one-shot `CBM_WORKERS=1` CLI refresh completed with zero extraction errors;
  the current graph is 18,139 nodes / 86,595 edges with fresh C3.2 symbols and
  source snippet evidence.
- Completed C3.3 as RED `0e06181` and GREEN `3c11369`. Enabled, available,
  host-supported `contexts=["file"]` actions append after built-ins in stable
  qualified-ID order; duplicate identities, unknown/wrong contexts, disabled
  plugins, and non-UTF-8 path conversion fail closed.
- Added neutral `PluginActionContext::File` and optional exact
  `PluginInvocationContext.file_paths`; generated next API schema is current.
  Right-click/activation revalidate registry plus path authority, disable races
  emit no intent, and CJK titles use display-cell geometry. No plugin command,
  filesystem operation, agent action, stable socket, or user process was
  touched.
- C3.3 gates: focused 8/8, plugin/context 35/35, FM/watcher/global-menu
  112/112, full 3041/3041 plus only `path_beta_real_host_probe` skipped,
  Linux/Windows clippy, Bun 17/17, Python 64/64, schema/fmt/diff clean. Graph
  is fresh at 18,246 nodes / 85,535 edges with current snippets.
- Completed C4.1 as five RED/GREEN pairs: preflight `386ddce`/`a9f022b`,
  staged COPY `47c753e`/`2848d97`, safe MOVE `e422d03`/`606d7ea`, bounded
  worker `f1590be`/`88cda7f`, and App lifecycle `626b7c3`/`98c51e4`.
  Preflight revalidates exact identities before writes; COPY staged-publishes
  without replacement; MOVE is atomic on one filesystem and copy-before-delete
  on EXDEV. Panic/cancel/partial results are explicit and render stays pure.
- Header/context Copy share exact clipboard authority. Paste owns one App
  worker lane and matching-cwd reconciliation; close/reopen cannot project old
  entries. C4.1 gates: core 15/15, App/worker 8/8, broad 147/147, full
  3064/3064 plus one named B0 skip, Linux/Windows clippy, Bun 17/17, Python
  64/64, fmt/diff/temp clean. Fresh graph: 18,453 / 86,399.
- Completed C4.2 as seventeen atomic test/product commits from `733d423`
  through `917cd57`. Header/context Delete converge on typed exact-path
  confirmation; Trash is default and Permanent requires a separate stage.
  Modified keys, stale/reordered/no-selection/closed/in-flight authority fail
  closed before a worker plan exists.
- Immutable delete preflight snapshots symlink metadata and file identity,
  rejects roots/unnamed paths, and revalidates before every mutation. Trash
  and permanent delete share the C4 worker lane and preserve ordered per-item
  completed/retained/failed evidence across partial errors, cancellation,
  panic, and disconnect. Matching-cwd completion owns reconciliation.
- C4.2 gates: focused 29/29, broad 321/321, full nextest 3086/3086 plus one
  named B0 skip, Linux/Windows clippy, Bun 17/17, Python 64/64, fmt/diff/temp
  clean. Isolated real Trash used throwaway HOME/XDG. Fresh graph: 18,576 /
  86,769 with both `miller_layout` and current delete symbols.
- Completed C4.3 as eighteen atomic test/product commits from `2028bce`
  through `c7043e2`. Context-menu and row Rename require one exact current
  target; stale/reordered/multi-selected/unsupported/closed/in-flight intent
  fails closed. The header has no Rename control and the single-name modal
  deliberately remains single-target.
- Common platform-aware component validation and immutable identity snapshots
  protect both single and typed bounded bulk plans. Single rename uses
  immediate revalidation plus platform no-replace; bulk chains/swaps/cycles
  use private collision-safe staging and deterministic publish. Failure and
  rollback preserve renamed/unchanged/restored/retained/uncertain per-item
  evidence, including exact recovery paths.
- Single and bulk rename reuse the existing operation worker and matching-cwd
  App reconciliation. The typed bulk App boundary is present for a future
  editor surface; it does not silently reinterpret multi-selection in the
  single modal. Render remains pure.
- C4.3 gates: focused/broad rename lifecycle 163/163, full nextest 3109/3109
  plus only `path_beta_real_host_probe` ignored, Linux/Windows clippy, Bun
  17/17, Python 64/64, fmt/diff/temp clean. Real temporary-filesystem tests
  cover file/directory/symlink, races, cycles, swaps, and rollback failure.
  Fresh graph: 18,722 / 88,526 with `miller_layout` and current single, bulk,
  shared-validator, and App-consumer symbols.
- Completed TP-C4.4-PROGRESS as ten atomic RED/GREEN commits from `aa9c894`
  through `cd4368a`: worker/App, transfer, delete, single rename, and bulk
  rename each have an observed RED then minimal GREEN. One latest-value
  same-generation worker slot coalesces updates; started count is monotonic and
  bounded; App projects Pending items to Running before exact completion.
- The first full suite exposed an unrelated OMP fixture mixing real and
  synthetic `Instant` values. Separate test-only `30d99bd` moved its complete
  lifecycle to one explicit monotonic clock; exact and 33-test family probes
  plus the second full suite passed.
- C4.4 progress gates: focused C4 operations 57/57, full nextest 3115/3115 plus
  only `path_beta_real_host_probe` ignored, Linux/Windows clippy, Bun 17/17,
  Python 64/64, fmt/diff/temp clean. Fresh graph: 18,745 / 87,178 with the
  progress type, common worker seam, four observer adapters, and
  `miller_layout` after the stale `ready` graph was disproven.
- Completed TP-C4.4-CANCEL as fourteen atomic RED/GREEN commits from `29572ab`
  through `d77858a`. Transfer rollback remains protected; delete checks before
  irreversible mutation; single/bulk rename prioritize already-observed cancel
  over later revalidation races. Repeated Esc routes only to the matching
  active generation and keeps FM open; buffered completion rejects late cancel.
- C4.4 cancellation gates: broad C4/input 98/98, full nextest 3122/3122 plus
  only `path_beta_real_host_probe` ignored, Linux/Windows clippy, Bun 17/17,
  Python 64/64, fmt/diff/temp clean. Fresh graph: 18,756 / 87,282 with typed
  input, App/worker cancel seams, exact tests, and `miller_layout` after stale
  `ready` was disproven.
- Completed TP-C4.4-RECONCILE as nine atomic test/product commits from
  `0b04e73` through `d1a2d2e`. Queued, watcher-first, and delayed own-operation
  events share one active watcher generation/revision/path owner; external
  paths remain immediately visible, polling fallback stays single-lane, and a
  stale completion cannot reload a same-cwd reopened FM.
- C4.4 reconciliation gates: broad C4/FM 126/126, full nextest 3128/3128 plus
  only `path_beta_real_host_probe` ignored, Linux/canonical Windows clippy, Bun
  17/17, Python 64/64, fmt/diff and operation/staging artifact checks clean.
  Fresh graph: 18,786 / 87,697 with the production ownership seam, delayed and
  same-cwd lifecycle tests, and `miller_layout` after stale `ready` proof.
- Completed TP-C4.4-RECOVERY as seven atomic commits from `0881976` through
  `c674296`. Disconnect-after-progress was observed RED before
  `new_after_generation` recovery: every remaining item terminalizes, runtime
  reconciliation ownership clears, the dead channel is replaced at its prior
  generation floor, and the next sync does not hot retry. Progress-then-panic,
  cancel-to-next-generation, stale cancel rejection, uncertain private staging
  evidence, and lane reuse are covered at App level without a second scheduler.
- C4.4 closure gates: focused recovery 46/46, C4 core 67/67, broad C4/FM
  218/218, final full nextest 3131/3131 plus only
  `path_beta_real_host_probe` skipped, Linux/canonical Windows clippy, Bun
  17/17, Python 64/64, fmt/diff and operation/staging artifact checks clean.
  Fresh graph: 18,793 / 87,788 with `new_after_generation`, exact recovery
  tests, and `miller_layout` after stale `ready` proof.
- Completed C5 existing-agent and non-agent handoff through product head
  `f744e4d`, then published the continuity checkpoint `f23dbc7` to both CyPack
  heads only. Existing agents receive one exact literal UTF-8 path plus Enter;
  non-agent sources use one direct-argv `claude` Down split with exact owned-
  resource rollback on every failure path.
- Completed C6.1 as durable plan `6464668`, RED contracts `4a65c15`, `4836b32`,
  `1236f57`, and GREEN `2bcdf14`. The new client-local module prepares a
  bounded 256-item FAVORITES/optional PINNED/LOCATIONS model outside render,
  deduplicates exact path authority, and keeps inaccessible pins visible but
  inert. `compute_view` owns item-only hit areas; input only replaces one typed
  path; the scheduled App boundary revalidates Files tab/model/live directory
  before opening exact `FmState` and rebinding the existing watcher. Tab change
  and FM open/close clear stale intent.
- C6.1 gates: exact 9/9; sidebar/FM 239/239 (run
  `d7202d9b-ffbc-409d-82f8-76ec191429d3`); full nextest 3151/3151 plus only
  `path_beta_real_host_probe` skipped (run
  `c5232427-adc0-49b9-9898-daf479b623cd`); Linux/Windows clippy, Bun 17/17,
  Python 64/64, fmt/diff/production-unwrap/temp clean. Fresh graph: 18,899 /
  90,094 with `miller_layout` and all new sidebar symbols.
- Completed C6.2 as durable plan `c3dfa6f`, RED contract `ac4eecb`, GREEN
  product `b88fc12`, and test-only lifecycle closure `a078d98`. Current
  authority is derived every frame from exact open
  `FmState.cwd` plus a prepared accessible row. The responsive accent pill is
  complete-or-omitted; Unicode truncation is display-cell safe; inaccessible
  warning overrides accessible eject, and every marker stays in the final row
  cell. Render remains pure and owns no filesystem/runtime mutation.
- C6.2 gates: focused sidebar/FM groups 11/11, 60/60, and 56/56; full nextest
  3154/3154 plus only `path_beta_real_host_probe` skipped (run
  `3ffc29fb-d053-4a6c-bbda-86b63745fc64`); Linux/Windows clippy, Bun 17/17,
  Python 64/64, fmt/diff/production-unwrap/artifact checks clean. Fresh graph:
  18,909 / 90,194 with `miller_layout` and the three C6.2 helper symbols. The
  known MCP extraction failure recovered via `CBM_WORKERS=1` CLI with zero
  extraction errors and no service/process restart.
- Completed C6.3 as matrix `2648a08`, RED contracts `a12a870`, `9aad978`,
  `ab27caa`, and `0905e49`, product commits `40c7ab9`, `dd00f25`, `e7614aa`,
  and `8b21442`, plus test closure `2d974da`. Unsupported New Folder/Compress
  are explicitly disabled; header, row, context, Open, and plugin paths
  converge on current typed and scheduled authority. Stale, reordered,
  unsupported, in-flight, popup-close, FM-close/reopen, and manifest-drift
  cases consume fail-closed without duplicate execution or focus corruption.
- C6.3 gates: focused 118/118 (run
  `41e5dbf8-576c-4e6b-a7eb-eedd9897121b`); full nextest 3160/3160 plus only
  `path_beta_real_host_probe` skipped (run
  `ec91fccd-12fc-49b9-ae92-0d464de19552`); Linux/Windows clippy, Bun 17/17,
  Python 64/64, fmt/diff/production-unwrap/temp clean. Fresh graph: 18,922 /
  89,277 with current `miller_layout`, row dispatcher, Open scheduler, and
  plugin scheduler snippets.

## 6. KOD DURUMU

Previously published product/test history through `bcba84d`:

- `c043c1e`: `src/ui/shell.rs`, `src/ui.rs`, `src/app/state.rs`, `src/app/mod.rs`.
- `d026e94`: `src/ui/file_manager.rs`, `src/ui.rs`, `src/app/state.rs`, `src/app/mod.rs`, `src/main.rs`.
- `74d3cc9`: `src/app/actions.rs`, `src/app/input/navigate.rs`, `src/config/keybinds.rs`, `src/config/model.rs`, `src/ui/keybind_help.rs`.
- `d2b27e6`: `src/app/input/file_manager.rs`, `src/app/input/mod.rs`, `src/fm/mod.rs`, `src/main.rs`.
- `6c7c58f`: `src/fm/mod.rs`, `src/ui/file_manager.rs`.
- `01ba91d`: `Cargo.toml`, `Cargo.lock`,
  `src/app/file_manager_watcher.rs`, `src/app/mod.rs`, `src/app/runtime.rs`,
  `src/fm/watcher.rs`, `src/fm/mod.rs`.
- `8cd4e89`: `src/server/headless.rs`, `src/terminal/state.rs`.
- `bcba84d`: `src/kitty_graphics.rs`.

B1 is an auditable 21-commit test/feature sequence from `439ff2c` through
`2b2dcd3`. Product paths are `Cargo.toml`, `Cargo.lock`,
`src/app/file_preview_worker.rs`, `src/app/mod.rs`, `src/app/runtime.rs`,
`src/fm/mod.rs`, `src/fm/text_preview.rs`, and `src/ui/file_manager.rs`.
Intermediate RED commits are intentional TDD checkpoints; the range tip is
fully green. Continuity/task commit `a0f82a3` and the complete B1 range were
fast-forward published to both CyPack `feat/native-fm` and fork `master`; exact
remote SHAs matched. Upstream was not pushed.

A3 is an auditable seven-commit sequence:

- `d713b71` / `027c364`: viewport RED/GREEN.
- `1fea0e7` / `6727342`: shared hit-geometry RED/GREEN.
- `3aa267b` / `33b585a`: runtime mouse dispatch RED/GREEN.
- `9d69c82`: cursor-only v1 selection-scope characterization.

All seven commits are published to both CyPack `feat/native-fm` and fork
`master`; upstream was not pushed.

B2 product/test history is an auditable sequence:

- `de1eff5`: dependency decision/evidence.
- `be200b9` / `e24cda8`: bounded decoder RED/GREEN.
- `054708f` / `983b6b3`: client-local placement RED/GREEN.
- `cf40a06` / `11f26eb`: generation-safe worker lifecycle RED/GREEN.
- `5c51f1a` / `c6b4762`: cached paint/cleanup RED/GREEN.
- `2989434`: width-safe fallback characterization and fix.

The range is fully green. The continuity/graph commits containing this handoff
are part of the publication unit; both CyPack heads are verified at that same
fast-forward branch tip. Upstream is not pushed.

C1.1 history is an auditable test/product pair plus an independent stability
fix:

- `0ed5e51`: compile-failing RED header geometry/ViewState contract.
- `9aa1e59`: deterministic process-generation suppression fixture.
- `c9bfbf9`: responsive header action geometry and pure render/ViewState seam.

The intermediate RED commit was never pushed alone. The publication unit adds
this continuity/graph commit and fast-forwards both CyPack heads only after all
fresh gates pass. Upstream is not pushed.

C1.2 is an auditable RED/GREEN pair:

- `dbc6798`: compile-failing exact-tag and fail-closed dispatch contract.
- `7fd01de`: private side-effect-free header action tag dispatch.

The intermediate RED commit was never pushed alone. This continuity/graph
commit completes the C1.2 publication unit.

N3.1 is an auditable RED/GREEN pair:

- `b5cc95c`: compile-failing selection/clipboard/lifecycle/render contract.
- `510eebc`: pure selection-sensitive persistent action-bar content.

The intermediate RED commit was never pushed alone. This continuity/graph
commit completes the N3.1 publication unit.

N3.2 is an auditable RED/GREEN pair:

- `446613a`: compile-failing explicit action-authority contract.
- `267ad91`: prepared fail-closed authority, disabled render, and input gate.

The intermediate RED commit was never pushed alone. This continuity/graph
commit completes the N3.2 publication unit.

C2.1 is an auditable RED/GREEN pair:

- `d4d404e`: compile-failing row action geometry/render/ViewState contract.
- `9a15328`: pure responsive row action geometry and ViewState/render seam.

The intermediate RED commit was never pushed alone. This continuity/graph
commit completes the C2.1 publication unit.

C2.2 is an auditable RED/GREEN pair:

- `94e4a02`: compile-failing stable-path row dispatch contract.
- `9ef90c6`: exact fail-closed path-and-tag dispatch before terminal input.

The intermediate RED commit was never pushed alone. This continuity/graph
commit completes the C2.2 publication unit.

N4.1 is an auditable seven-commit sequence:

- `e876223` / `590e376`: selection state RED/GREEN.
- `1789bbd` / `5c14439`: lifecycle reconcile RED/GREEN.
- `699a6a6`: gesture and render RED.
- `fc19237`: stable row identity RED.
- `86b618a`: exact input, stable identity, and pure visual projection GREEN.

The compile-failing RED checkpoints were never pushed alone. This continuity/
graph commit completes the N4.1 publication unit.

C3.2 is an auditable six-commit sequence:

- `69864d6` / `ad5f8a5`: exact path-stable popup geometry RED/GREEN.
- `73df647` / `45c151f`: keyboard/mouse lifecycle and typed intent RED/GREEN.
- `1078215` / `0915964`: disabled/highlight-safe render RED/GREEN.

No RED checkpoint is published alone. This continuity/graph commit completes
the C4.1 publication unit before both CyPack heads are fast-forwarded.

C4.4 terminal recovery is an auditable seven-commit sequence:

- `0881976` / `7847a6c`: disconnected worker lane RED/GREEN.
- `8974f4c`: progress-then-panic terminalization and lane reuse coverage.
- `bcc9ef5`: cancellation-to-next-generation and stale-cancel coverage.
- `7e2af79`: uncertain private bulk-recovery path evidence and lane reuse.
- `03b9395`: real baseline cleanup plus no-hot-retry idempotence coverage.
- `c674296`: test-fixture Clippy closure after the warning-as-error gate found
  `field_reassign_with_default`.

The only production change is in `src/app/file_operation_worker.rs`: a dead
channel is replaced through the existing single worker constructor while
preserving the generation floor. No second scheduler, server field, protocol,
render mutation, dependency, or public docs surface was added. This continuity
commit closes the C4.4 publication unit before both CyPack heads are
fast-forwarded; no RED checkpoint is pushed alone.

C6.1 is an auditable plan/RED/GREEN sequence:

- `6464668`: durable sidebar test points and ordered C6.1–C6.4 decomposition.
- `4a65c15`: model/geometry/render/navigation RED contract.
- `4836b32`: live discovery and bounded-source RED contract.
- `1236f57`: stale-hit and hidden-Spaces input isolation RED contract.
- `2bcdf14`: sectioned model, pure geometry/render, typed input intent,
  lifecycle invalidation, scheduled revalidation, and watcher convergence.
- `c3dfa6f`: durable C6.2 current/pill/marker/lifecycle/gate test points.
- `ac4eecb`: C6.2 exact-authority, responsive Unicode pill, and marker RED
  contracts.
- `b88fc12`: pure current-location derivation plus bounded pill and
  warning/eject rendering.
- `a078d98`: explicit non-Files and close/reopen lifecycle coverage.

No RED checkpoint is published alone. Product/test paths are
`src/app/file_manager_sidebar.rs`, App state/runtime/actions/watcher/input,
desktop/mobile view computation, and pure sidebar render. This continuity/
graph commit closes C6.2 before both CyPack heads are fast-forwarded; upstream
is never pushed.

C6.4 is an auditable plan/RED/GREEN sequence:

- `5b8f327`: durable semantic roles, state precedence, and visual gate plan.
- `2362751` / `3e73351`: semantic palette and canvas RED/GREEN.
- `4ed210e` / `37f760d`: typed cwd availability RED/GREEN.
- `04b8070` / `792c4d8`: operation/recovery status RED/GREEN.
- `3f9a0cd` / `101809c`: preview warning/error RED/GREEN.
- `03aeb6d` / `f52cb85`: full-frame composition and image-target closure.

No RED checkpoint is published alone. Render remains pure; filesystem-derived
status is prepared only during FM refresh. This continuity/graph commit closes
C6.4 and the v1 A-C visual gate before both CyPack heads are fast-forwarded;
upstream is never pushed.

## 7. TEST KANITI

- B1/FM targeted: 64/64.
- Final full nextest: 2948/2948 passed, one explicit B0 interactive host probe
  skipped, no retry.
- Linux all-target and canonical Windows MSVC binary-target clippy passed with
  `-D warnings`.
- Bun 17/17; Python maintenance 64/64; fmt and diff-check clean.
- Doctests are N/A because Herdr has no library target.
- C6.1 focused model/geometry/render/input/navigation/lifecycle: 9/9.
- Broad sidebar/native-FM nextest: 239/239, run
  `d7202d9b-ffbc-409d-82f8-76ec191429d3`.
- Final C6.1 full nextest: 3151/3151, one named B0 real-host probe skipped,
  run `c5232427-adc0-49b9-9898-daf479b623cd`; no retry.
- C6.1 exact-head Linux all-target and canonical Windows MSVC bin clippy,
  Bun 17/17, Python 64/64, fmt/diff/production-unwrap/temp-artifact checks are
  clean. `just` remains absent; the complete recipe was executed directly.
- C6.2 focused sidebar/FM groups: 11/11, 60/60, and 56/56. Final full nextest:
  3154/3154 plus one named B0 real-host probe skipped, run
  `3ffc29fb-d053-4a6c-bbda-86b63745fc64`; no retry-only closure.
- C6.2 exact-head Linux all-target and canonical Windows MSVC bin clippy,
  Bun 17/17, Python 64/64, fmt/diff/production-unwrap/artifact checks are clean.
  `just` remains absent; the complete recipe was executed directly.
- C6.3 focused 118/118; full nextest 3160/3160 plus one named B0 probe skip;
  Linux/Windows clippy, Bun 17/17, Python 64/64, fmt/diff/production-unwrap/
  artifact checks clean.
- C6.4 focused semantic/directory/status/preview/composition sets passed;
  broad FM/Kitty and UI composition sets passed. Final full nextest 3171/3171
  plus only `path_beta_real_host_probe` skipped, run
  `339242c5-a4d2-4989-9583-8e904c6d7b1e`; no retry-only closure.
- C6.4 Linux all-target and canonical Windows MSVC bin clippy, Bun 17/17,
  Python 64/64, fmt/diff/production-unwrap/artifact checks are clean. Isolated
  headless API and 120x30 real PTY checks used throwaway XDG/socket state,
  exited semantically, and left zero process/socket/temp residue.
- Actual B1 lock delta is five packages with no existing-version upgrade.
  Exact OSV batch returned only severity-less `RUSTSEC-2025-0141` for
  unmaintained `bincode 1.3.3`; no security-severity advisory.
- A3 targeted broad regression: 164/164; scope: 4/4.
- Final A3 full nextest: 2966/2966, one named B0 host probe skipped, no retry.
- Linux/Windows clippy, Bun 17/17, Python 64/64, fmt/diff clean.
- Isolated `--no-session` PTY proved three columns, cursor click, directory
  double-click enter, long-list wheel down/up viewport clamp, semantic exit 0,
  and zero temp/process residue.
- B2/FM/Kitty targeted: 96/96; full nextest: 2983/2983 plus one named B0
  interactive probe skip; no retry.
- B2 Linux all-target and canonical Windows MSVC bin clippy passed with
  `-D warnings`; Bun 17/17; Python 64/64; fmt/diff clean.
- B2 isolated Kitty source-to-host comparison: 0/271425 pixel difference.
  Closing FM removed the image from the host, and semantic exit left zero
  test process/socket/temp residue.
- C1.1 geometry/render/ViewState targeted: 4/4; suppression/process-exit/stale
  lifecycle family: 27/27; final full nextest: 2986/2986 plus one named B0
  interactive probe skip, no retry-only closure.
- C1.1 Linux all-target and canonical Windows MSVC bin clippy passed with
  `-D warnings`; Bun 17/17; Python maintenance 64/64; fmt/diff clean.
- C1.2 exact dispatch: 2/2; full FM input: 13/13; final full nextest:
  2988/2988 plus one named B0 interactive probe skip, no retry-only closure.
- C1.2 Linux all-target and canonical Windows MSVC bin clippy passed with
  `-D warnings`; Bun 17/17; Python maintenance 64/64; fmt/diff clean.
- N3.1 targeted: 3/3; FM regression: 135/135; final full nextest: 2991/2991
  plus one named B0 interactive probe skip, no retry-only closure.
- N3.1 Linux all-target and canonical Windows MSVC bin clippy passed with
  `-D warnings`; Bun 17/17; Python maintenance 64/64; fmt/diff clean.
- N3.2 exact authority/preparation/render/dispatch: 7/7; broad FM/input/render/
  Kitty regression: 165/165; final full nextest: 2996/2996 plus one named B0
  interactive probe skip, no retry-only closure.
- N3.2 Linux all-target and canonical Windows MSVC bin clippy passed with
  `-D warnings`; Bun 17/17; Python maintenance 64/64; fmt/diff clean.
- C2.1 focused invariant/readability set: 8/8; FM impact: 71/71; final full
  nextest: 2998/2998 plus one named B0 interactive probe skip, no retry-only
  closure. Linux/Windows clippy, Bun 17/17, Python 64/64, fmt/diff clean.
- C2.2 exact dispatch/stale/no-side-effect: 3/3; all FM input: 17/17; FM impact:
  74/74; final full nextest: 3001/3001 plus one named B0 interactive probe
  skip, no retry-only closure. Linux/Windows clippy, Bun 17/17, Python 64/64,
  fmt/diff clean.
- N4.1 focused input/render: 7/7; broad FM/watcher/input/render/Kitty: 137/137;
  final full nextest 3015/3015 plus one named B0 interactive probe skip, no
  retry-only closure. Linux/Windows clippy, Bun 17/17, Python 64/64, fmt/diff
  clean.
- C3.2 focused popup 4/4, lifecycle 3/3, disabled render 1/1; broad
  FM/global-menu 51/51 and menu/render 26/26. Final full nextest 3033/3033
  plus one named B0 host-probe skip; Linux all-target and canonical Windows
  MSVC bin clippy, Bun 17/17, Python 64/64, fmt/diff clean.
- C3.3 focused 8/8; plugin/context 35/35; FM/watcher/global-menu 112/112;
  final full nextest 3041/3041 plus only the named B0 host probe skip. Linux
  all-target and canonical Windows MSVC bin clippy, Bun 17/17, Python 64/64,
  generated schema, fmt, and diff-check are clean.
- C4.1 operation core 15/15, App/worker 8/8, broad FM/watcher/preview 147/147;
  final full nextest 3064/3064 plus only the named B0 host probe skip. Linux
  all-target and canonical Windows MSVC bin clippy, Bun 17/17, Python 64/64,
  fmt/diff/temp checks, and graph freshness are clean.
- C4.2 focused delete 29/29 and broad FM/watcher/preview/context/plugin
  321/321; final full nextest 3086/3086 plus only the named B0 host probe skip.
  Linux/Windows clippy, Bun 17/17, Python 64/64, isolated real Trash,
  fmt/diff/temp checks, and graph freshness are clean.
- C4.3 focused/broad rename, bulk, worker, App, and watcher regression 163/163;
  final full nextest 3109/3109 plus only the named B0 host probe ignored.
  Linux/Windows clippy, Bun 17/17, Python 64/64, real temporary-filesystem
  rename/recovery coverage, fmt/diff/temp checks, and graph freshness are
  clean.
- C4.4 terminal recovery focused 46/46, C4 core 67/67, broad C4/FM 218/218;
  final full nextest 3131/3131 plus only `path_beta_real_host_probe` skipped.
  Linux all-target and canonical Windows MSVC bin clippy passed with
  `-D warnings`; Bun 17/17; Python 64/64; ignored-only inventory, fmt, diff,
  operation/staging artifact scan, and graph freshness are clean. The final
  graph is 18,793 nodes / 87,788 edges and returns the production recovery
  seam, exact failure tests, and `miller_layout`.

## 8. KRİTİK KARARLAR

- Pure render is non-negotiable.
- S3 registry deferred to S5; use concrete content swap until abstraction is earned.
- S2 persistence deferred to S6.
- A2.2 caches parent/preview in `FmState`.
- A4 uses stable `notify-debouncer-full 0.7.0` / `notify 8.2.0`.
- Native watching is primary; startup/runtime errors enter explicit polling
  fallback, and all active watchers reconcile every 2 seconds to cover silent
  FUSE/NFS/exFAT-class delivery failures.
- A4, B0, B1, A3, and B2 are implementation-complete, fully verified,
  graph-indexed, and published to the CyPack fork.
- C1, N3, and C2 are implementation-complete, fully verified, and
  graph-indexed.
  N3.2 supplies explicit selection/clipboard/target/in-flight authority and
  disabled-click no-side-effect behavior. C4 must still revalidate every real
  filesystem operation at execution time for TOCTOU and partial failure.
- N4.2 is implementation-complete, fully verified, and graph-indexed. Its
  prepared path vector and action states are client-local presentation/input
  authority only; C3 may consume them for menu modeling, while C4 must still
  revalidate every target at execution time.
- C3.1 is implementation-complete, fully verified, and graph-indexed. It
  models context intent only and deliberately leaves popup routing/render/
  disabled dispatch to C3.2 and plugin extension to C3.3.
- C3.2 is implementation-complete, fully verified, and graph-indexed. The
  popup reuses the global modal stack, disabled actions are inert, and emitted
  typed intents are revalidated against current prepared path/order authority.
  C4/C5 remain the sole owners of real side effects.
- B1 uses minimal pure-Rust syntect outside input/render in a dedicated bounded
  worker. Plain prepared content remains availability authority; highlighting
  is optional enhancement and stale generations never mutate current state.
- B0's conditional B2 GO is satisfied: B2 retains bounded decode, generation,
  cleanup, and real-host evidence constraints.
- The user granted standing authorization for autonomous atomic commits and
  CyPack fork-only fast-forward pushes. Do not repeatedly ask for alignment;
  never relax targeted staging, verification, ancestry, or remote-SHA checks.
- C6.4 and the v1 A-C visual gate are implementation-complete, fully verified,
  and graph-indexed. P4.0 is read-only evidence gathering; it may activate at
  most one deferred architecture candidate and cannot itself justify product
  refactoring.
- P4.0 and N2.0 are complete. S5/S6/S7 and the original dynamic/unbounded N2
  state machine are implementation NO-GO. Pinned Yazi/Joshuto source proves one
  narrow missing behavior: leave should focus the exact child just exited.
  N2.1 receives implementation GO under zero-new-state/no-extra-read budgets;
  N2.2 retained history and parent-column sibling navigation remain deferred.
- N2.1 is implementation-complete as RED `e433a2f` and GREEN `c530836`.
  Exact 6/6, FM 65/65, full nextest 3177/3177 plus one named ignored host probe,
  Linux/Windows clippy, Bun 17/17, Python 64/64, and fresh graph 18,997 / 89,826
  are clean. Ordinary reload behavior is preserved through the shared helper;
  leave holds only one local departed path and performs no additional read.

## 9. GÜVENLİK

- Never kill user processes.
- Never touch `/home/ayaz/.local/bin/herdr` or the stable socket.
- Clear inherited socket variables and use throwaway XDG directories for runtime tests.
- Never stage ignored `.local` files into product commits.
- Never push `upstream`.
- C3.3 extends the neutral public JSON plugin context, not private TUI
  transport frames; `PROTOCOL_VERSION` remains 16 per `v0.7.3` comparison and
  repository precedent. C4 owns every filesystem side effect.

## 10. AÇIK GÖREVLER

See `.codex/TASKS.md` for the completed A3/B2/C1/N3/C2/N4 contracts and the
complete C3–C6, S5–S7, N2, and M1–M3 roadmap. A4, B0, B1, A3, B2, C1, N3,
C2, N4, C3.1, C3.2, C3.3, C4.1, C4.2, C4.3, C4.4.1 PROGRESS, C4.4.2
CANCEL, C4.4.3 RECONCILE, C4.4.4 RECOVERY, C4.4.5 GATES, C5.1–C5.5, and
C6.1–C6.4 are closed. C5's atomic product chain ends at `f744e4d`: exact existing-agent send
and non-agent `Down` split → direct `claude` argv → first literal path send,
with exact new-resource rollback on every failure path. The complete gate is
3143/3143 Rust, Bun 17/17, Python 64/64, Linux/Windows clippy, and fresh graph
18,854 / 88,064. C6.1 then closes at product head `2bcdf14`, full Rust
3151/3151 plus one named skip, and fresh graph 18,899 / 90,094. C6.2 closes at product head
`b88fc12` plus test closure `a078d98`, full Rust 3154/3154 plus one named skip,
and fresh graph 18,909 / 90,194. C6.3 closes at product head `8b21442` plus
test closure `2d974da`, full Rust 3160/3160 plus one named skip, and fresh
graph 18,922 / 89,277. C6.4 closes at test/product head `f52cb85`, full Rust
3171/3171 plus one named skip, isolated API/PTY residue zero, and fresh graph
18,974 / 89,775. P4.0 closes as a documentation-only evidence matrix with
S5–S7 implementation NO-GO. N2.0 then rejects the original dynamic/unbounded
state machine and N2.1 closes as RED `e433a2f` → GREEN `c530836` with complete
gates. M1–M3 now have durable macro/micro and failure-path test contracts; M1.0
is the next discovery-only lane, followed by M2.0, while M3.0 requires a real
second consumer. N2.2 and S5–S7 remain independently deferred.

## 11. ORTAM

- `codex-cli 0.144.1` is installed.
- `just` is absent; direct recipe execution is required unless installed later.
- Full post-N3.1 graph reindex completed at 18,009 nodes / 83,964 edges.
  Freshness query returned current action-bar model/selection/kind types and
  `compute_file_manager_action_bar_model`; the builder is connected to
  desktop/mobile view computation, render fallback, and model tests. Lifecycle
  and render tests are present. Freshness was not inferred from `ready` alone.
- Full post-N3.2 graph reindex completed at 18,026 nodes / 84,120 edges.
  Freshness queries returned current `miller_layout`, action state, authority
  builder, entry-capability preparation, and mouse handler symbols. The pure
  builder is connected to desktop/mobile view computation, render fallback,
  and authority tests. Freshness was not inferred from `ready` alone.
- Fast post-C2.1 graph reindex completed at 18,042 nodes / 84,123 edges.
  Freshness queries returned `compute_file_manager_row_geometry`,
  `FileManagerRowAction`, and `FileManagerRowActionArea` from their current
  source files. Freshness was not inferred from `ready` alone.
- Fast post-C2.2 graph reindex completed at 18,049 nodes / 83,839 edges.
  Freshness queries and snippets returned the current path-bearing
  `FileManagerRowActionArea` and live path/support validation in
  `handle_file_manager_mouse`. Freshness was not inferred from `ready` alone.
- Fast post-N4.1 graph reindex completed at 18,078 nodes / 83,865 edges.
  Freshness queries returned `replace_selection`, `toggle_selection`,
  `extend_selection`, `reconcile_multi_selection`, stable row identity input,
  and the multi-selection render test with live connections. Freshness was not
  inferred from `ready` alone.
- Fast post-N4.2 graph reindex completed at 18,091 nodes / 84,102 edges.
  Freshness queries and source snippets returned current `select_all`,
  `MAX_MULTI_SELECTION_PATHS`, `compute_file_manager_action_bar_model`, and
  `handle_file_manager_key`, including the 4,096 ceiling and atomic Shift
  routing. Freshness was not inferred from `ready` alone.
- Fast post-C3.1 graph reindex completed at 18,115 nodes / 84,003 edges.
  Freshness queries and source snippets returned current
  `FileManagerContextMenuModel::from_action_bar` precedence/item mapping and
  `ContextMenuKind::File`. Freshness was not inferred from `ready` alone.
- Post-C3.2 graph refresh completed at 18,139 nodes / 86,595 edges through
  supported single-worker CLI fallback after the proxy-owned child crashed in
  parallel native extraction. CLI status, graph search, and source snippet
  returned `validated_file_context_action`, right-click selection tests, and
  disabled-render tests. No process/service was restarted or killed, and
  freshness was not inferred from `ready` alone.
- Post-C3.3 sequential graph refresh completed at 18,246 nodes / 85,535 edges
  with zero extraction errors. CLI status/search/snippet proof returned
  `file_manifest_actions`, `plugin_invocation_params`, Unicode popup geometry,
  and the end-to-end plugin disable-race test; `ready` alone was not accepted.
- Post-C4.3 fast refresh completed at 18,722 nodes / 88,526 edges. The stale
  pre-refresh graph said `ready` but returned none of the new rename symbols;
  after indexing, searches returned `miller_layout`, `RenameOperationPlan`,
  `BulkRenameOperationPlan`, `validate_rename_name_component`, and
  `consume_file_manager_bulk_rename_request` from current source.
- Post-C4.4.1 fast refresh completed at 18,745 nodes / 87,178 edges. The stale
  pre-refresh graph said `ready` but returned no `FileOperationWorkerProgress`;
  after indexing it returned the progress type,
  `execute_worker_task_with_progress`, all four operation observer adapters,
  and the prior `miller_layout` symbol.
- Post-C4.4.2 fast refresh completed at 18,756 nodes / 87,282 edges. The stale
  graph said `ready` but lacked `cancel_file_manager_operation`; after indexing
  it returned `FileManagerKeyDispatch`, the App cancel seam, generation-safe
  worker cancel, exact cancellation tests, and `miller_layout`.
- Post-C4.4.3 fast refresh completed at 18,786 nodes / 87,697 edges. The stale
  graph said `ready` but lacked `own_operation_reconcile`; after indexing it
  returned the exact ownership seam, delayed/same-cwd lifecycle tests, and
  `miller_layout`.
- Post-C4.4.4 fast refresh completed at 18,793 nodes / 87,788 edges. The stale
  graph said `ready` but returned only `miller_layout`, not the new recovery
  symbols; after indexing it returned `new_after_generation`, the disconnect,
  panic, and private-recovery tests, plus `miller_layout`.
- Post-C5.4 fast refresh completed at 18,854 nodes / 88,064 edges. Status was
  cross-checked with `miller_layout` and nine current split/ownership/rollback
  symbols including `sync_file_manager_claude_split`,
  `launch_file_manager_claude_split`, and
  `complete_file_manager_claude_split`; `ready` alone was not accepted.
- Post-C6.1 moderate refresh completed at 18,899 nodes / 90,094 edges. Status
  was cross-checked with `miller_layout`, `FileManagerSidebarModel`,
  `compute_file_manager_sidebar_row_areas`, `file_manager_sidebar_path_at`,
  and `sync_file_manager_sidebar_navigation`; `ready` alone was not accepted.
- Post-C6.4 moderate refresh completed at 18,974 nodes / 89,775 edges. Status
  was cross-checked with current `miller_layout`, `FmDirectoryStatus`,
  `file_manager_visual_styles`, and `file_manager_status_line` source snippets;
  `ready` alone was not accepted.
- `mcp-proxy.service` cold start measured 54 seconds for 26 servers. Readiness
  now has a 120-second internal and 150-second systemd budget; live proof was
  `expected=26 observed=26 critical_tools=14`.
- `~/.codex/skills/rust-dev` points to the canonical Claude `rust-dev` skill; parity reports no broken skill links.
- Global Codex hooks support SessionStart and UserPromptSubmit; Herdr context routing is scoped to this repo.

## 12. BAŞLATMA

Run:

```bash
herdr-codex
```

The new agent must read `AGENTS.md`, `CLAUDE.md`, `.codex/BOOTSTRAP.md`, `.codex/CURRENT.md`, and `.codex/TASKS.md`, then verify graph and Git state before acting.
