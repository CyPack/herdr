# Current State — 2026-07-16

## Repository

- Path: `/home/ayaz/projects/herdr`
- Branch: `feat/native-fm`
- Published SF0 planning artifact checkpoint: `32856f7`
  (`docs: plan shell foundation and files workspace`).
- Published SF1 characterization checkpoint: `7b9b626d`
  (`test: characterize shell foundation baseline`).
- The artifact checkpoint was fast-forward published to both CyPack
  `feat/native-fm` and fork `master`; exact remote SHA equality was verified.
- Published CyPack M3 evidence checkpoint: `e9f2fe0`
  (`docs: close general UI interface evaluation`).
- Verified M2.1 chain: RED `dab1e20`; GREEN `0ae6175`
  (`feat: add focused-agent worktree launcher`).
- Verified C6.4 product/test head: `f52cb85`
  (`test: align image resize target with status geometry`).
- M3.0 evidence is published to both CyPack refs with exact remote-SHA
  equality. This continuity-only follow-up records that result; no product
  publication or implementation task remains pending.
- `origin` is the `CyPack/herdr` fork. `upstream` is `ogulcancelik/herdr` and must never be pushed.

## Active Product Lane — Shell Foundation and Files Workspace

- The user explicitly approved the bounded twelve-phase direction: SF0-SF6,
  followed by FM1-FM5. Apps/Desktop remains a later independent program.
- Approved design:
  `docs/superpowers/specs/2026-07-15-herdr-shell-foundation-v0-design.md`.
- Approved execution package:
  `docs/superpowers/plans/2026-07-15-herdr-shell-file-manager-program-plan.md`,
  `docs/superpowers/plans/2026-07-15-herdr-shell-foundation-v0-implementation.md`,
  and
  `docs/superpowers/plans/2026-07-15-herdr-file-manager-post-shell-implementation.md`.
- Plan review evidence:
  `.codex/evidence/shell-foundation-plan-review.md`.
- SF0, SF1, and SF2 are closed. A0-A7 and delivery gates I0-I14 are complete
  for the geometry phase. Named regions, bounded validation, typed templates,
  deterministic track allocation, responsive degradation, one aggregate
  cached `ShellView`, and generation-safe flattened semantic hits are live.
  The next product slice is SF3.1 transactional divider resize; it starts with
  a fresh I2-I6 drift/characterization pass before entering I7 RED.
- The SF2.4 RED/GREEN pair is `2a440478` / `07133b8b`. CyPack
  `feat/native-fm` and fork `master` both resolve to exact SHA
  `07133b8b9e9cf10b9b3dea0febe22a8389457164`. Fresh closure is cached-view
  7/7, broad shell 88/88, every `src/ui.rs` test 41/41, frozen SF1 11/11,
  full Nextest 3239/3239 plus only the named B0 skip, Linux/Windows Clippy,
  Bun 17/17, Python 64/64, fmt, and diff clean.
- Product post-publication single-worker graph evidence is 20,017 nodes /
  91,917 edges; after the separate continuity checkpoint the current graph is
  20,018 / 91,918. CLI and built-in MCP status/search/snippet calls return the
  current by-value `compute_shell_view`, both new fail-closed
  characterizations, `miller_layout`, and the SF3.1 RED task; freshness was
  verified by content rather than `ready` alone. No proxy or process was
  restarted.
- SF1 commit `7b9b626d` is fast-forward published to both CyPack refs. Exact
  SHA equality at its product/test checkpoint was
  `7b9b626d3e84b4ba03856a8f54b81196985d3f48`. Focused characterization is
  11/11; full nextest is 3203/3203 plus only the named B0 real-host probe skip;
  Linux/Windows Clippy, Bun 17/17, Python 64/64, fmt, diff, and temp-residue
  gates are clean.
- Post-publication graph evidence is 19,809 nodes / 91,610 edges. Freshness is
  proved through exact snippets for both
  `files_curtain_currently_replaces_terminal_surface` and `miller_layout`;
  `ready` alone was not accepted.
- The prior P4.0 S6 and dynamic-N2 NO-GO decisions were correct for the then-
  available evidence. The later explicit user demand now supplies the missing
  triggers: a real AppDock/WorkspaceStage/Files consumer, resizable/collapsible
  shell regions, overlay input blocking, horizontal Miller traversal, column
  resize, and all-column mouse ownership. The new design remains bounded and
  does not authorize the rejected arbitrary component registry, unbounded
  Miller state, visual editor, or Apps/Desktop expansion.
- Product and non-product tooling commits must remain separate. The change-
  pipeline T3.1 lane is paused while this sequential product lane closes its
  current phase. The untracked user-owned `.superpowers/` tree remains outside
  every stage/commit.

## Active Non-Product Tooling Lane — Change Pipeline

- Design commits `86a25e8`, `0ea0f77`, and `600c0d6` define Ratatui reference
  intelligence v2.1 plus generalized A0-A7 change intelligence and I0-I14
  delivery governance.
- Durable human/agent task state is in
  `.codex/CHANGE-PIPELINE-TASKS.md`; T0, T1, and T2 are complete. The queued
  tooling micro action is T3.1, paused until the sequential product lane closes
  its current phase: write the RED identity/version/authority contracts for the
  sibling `herdr-change-pipeline` module.
- The user-approved execution contracts are the shared program
  plan plus the separate Ratatui v2.1 and change-pipeline v1 TDD plans under
  `docs/superpowers/plans/2026-07-15-*.md`.
- The registry includes a mid-flight adoption path for an existing file-manager
  feature/bugfix session: inspect and preserve current work, reconstruct A0-A7,
  classify the current I-phase, then enforce the remaining gates.
- Ratatui Design Intelligence v2.1 is locally implemented as the atomic chain
  `7622dde` through `2517353`: preserved baseline, observed identity/stack/
  phase/governance RED tests, GREEN manifest/schema/template/validator/phase
  bindings, and human/agent governance. Fresh closure is 59/59 tests, 15
  phases/101 jobs, both validators PASS, JSON parse PASS, skill validation
  PASS, and product isolation PASS.
- The new `herdr-change-pipeline` skill is not yet built. Product-code
  authorization remains false; no push or graph refresh has been performed for
  this tooling lane.
- This lane does not activate S5, S6, S7, or N2.2 and never authorizes touching
  stable Herdr or inherited stable sockets.

## Verified Checkpoint — Native-FM Completion Audit

- `.codex/evidence/native-fm-completion-audit.md` reconciles the complete
  A1–C6/N1/N3/N4/N2.1/M1/M2 scope, M3 NO-GO, gate evidence, graph state, and
  fork-only Git baseline.
- The apparent empty/incorrect queue was continuity drift in ignored local PRD
  files: B0, A3 remainder, and B1–C6 were still marked pending despite their
  published closure. The root tree, module checklists, and local next-session
  prompt now match tracked truth.
- No Rust, dependency, protocol, persisted state, filesystem operation,
  process, pane, socket, or stable Herdr resource changed during the audit.
- Fresh exact-head characterization is 16/16, zero retry, nextest run
  `c3e40137-6400-4547-9eb8-729f29fd6583`.
- At this completion-audit checkpoint no speculative production increment was
  active. Later explicit user demand now satisfies bounded S6/N2.2 triggers as
  recorded in the active product lane; S5 registry and S7 popup stack remain
  inactive.

## Completed Checkpoint

- A2.2 responsive Miller columns were committed as `6c7c58f`, full graph-indexed,
  and fast-forward pushed to the CyPack feature branch and fork master only.

## Verified Checkpoint — M2 Focused-Agent Worktree Launcher

- M2.0 rejected duplicate List/Create/Remove/Switch implementations and froze
  five M2.1 failure-aware test points before Rust production work.
- RED `dab1e20`; GREEN `0ae6175`. Pure `[w]` geometry sits beside M1 `[+]` and
  carries exact workspace ID, `PaneId`, and `TerminalId`.
- Input revalidates current focused agent plus cached non-linked root Git/
  worktree capability, then emits only the existing open-dialog intent.
- Exact 5/5; worktree/attachment 131/131; full nextest 3202/3202 plus one named
  B0 skip; Linux/Windows clippy; Bun 17/17; Python 64/64; fmt/diff clean.
- Fresh graph is 19,534 nodes / 91,017 edges and returns current
  `miller_layout`, action geometry, and cached-capability symbols.

## Verified Checkpoint — M3 General UI Interface Evaluation

- M3.0–M3.3 are closed implementation NO-GO by
  `.codex/evidence/m3-general-ui-interface.md`; no Rust production code was
  edited.
- M1 `[+]` and M2 `[w]` are two real action consumers, but the common part is
  limited to focused-agent/border geometry and two 21-line bounded ASCII render
  templates. M1 owns a private attachment-picker lifecycle; M2 owns cached
  workspace/Git authority and routes into an existing dialog.
- `BaseLayer` remains one terminal/FM content swap, `OverlayLayer` remains one
  active `Mode` owner, and `ShellLayout` has no additional persisted region.
  The action areas and transient request do not enter persistence or protocol.
- Fresh characterization is 16/16 with zero retry, nextest run
  `32ca7f37-b65c-45ef-9dbf-548e8263d383`. It protects Base/content swap,
  desktop/mobile shell geometry, M1/M2 disjoint geometry/render, exact stale-
  identity input, dialog/picker cleanup, context focus/close, and old snapshot
  compatibility.
- The already-current graph is 19,534 nodes / 91,017 edges and was verified
  with `miller_layout` plus current M1/M2 compute/render/input symbols; `ready`
  alone was not accepted.
- Future triggers remain explicit: S5 needs a second independently owned page/
  component lifecycle, S6 a real persisted resizable region, S7 a nested popup
  retaining parent ownership, and N2.2 independent finite history demand.

## Verified Checkpoint — C5 Agent Handoff

- C5.1 classified FM authority as TUI/client-local and reused neutral terminal,
  pane, split, agent identity, process-exit, and runtime-cleanup seams.
- C5.2 RED/GREEN `65c3928`/`ec7539d` prepares one exact path plus focused agent
  terminal identity without side effects. C5.3 RED/GREEN `00664c7`/`66b00d7`
  sends one literal UTF-8 path plus one Enter, with no shell interpolation,
  wrong-pane fallback, duplicate send, or hot retry.
- C5.4 RED/GREEN `6c6a409`/`f744e4d` handles the non-agent route: exact source
  workspace/pane/terminal and FM cwd are revalidated at the scheduled boundary;
  one direct-argv `["claude"]`, empty-extra-env `Down` split is created. The FM
  closes and focus moves only after the first literal path send succeeds.
- Split/spawn failure, stale/cancelled authority, first-send failure, partial
  setup, early `PaneDied`, and retry remove only the newly owned pane/terminal/
  runtime. Existing panes, source focus, FM state on failure, stable Herdr, and
  stable sockets remain untouched.
- Fresh gates: C5.4 4/4; related handoff/agent/pane-exit 17/17; full nextest
  3143/3143 with only the named B0 host probe skipped, run
  `418dc969-0218-42f7-8ef3-26ed6c12ec3b`; Linux all-target and canonical Windows
  clippy; Bun 17/17; Python 64/64; fmt/diff/production-unwrap clean.
- Fresh graph is 18,854 nodes / 88,064 edges. Queries returned both
  `miller_layout` and the new typed request, scheduled launch, exact ownership,
  rollback, completion, and current-authority symbols; freshness was not
  inferred from `ready` alone.
- Next product module: C6.3 integrated header/row/context actions, then C6.4.

## Verified Checkpoint — C6.1 Native Sectioned Sidebar

- Durable plan `6464668` and RED contracts `4a65c15`, `4836b32`, `1236f57`
  precede GREEN `2bcdf14`; no compile-failing RED checkpoint is published
  alone.
- `FileManagerSidebarModel` lives in its own client-local module and caps the
  complete source set at 256 exact paths. FAVORITES, optional PINNED, and
  LOCATIONS retain stable order; duplicate paths grant authority only to the
  first row. Startup preparation reads live directory metadata outside render;
  missing favorites are omitted and inaccessible configured pins remain
  visible but inert.
- Desktop `compute_view` produces complete item-only hit rectangles. Headers,
  blank separators, clipped/tiny/collapsed/non-Files rows, and stale prior
  geometry carry no navigation authority. Render consumes only the prepared
  model and removes the old `(files — soon)` placeholder.
- Mouse input performs no filesystem I/O and stores only the latest exact path.
  The scheduled App boundary consumes once, revalidates current Files tab,
  current model accessibility, and live directory type, then opens the exact
  `FmState`; the existing watcher binds that cwd in the same scheduled flow.
  Missing, regular-file, model-stale, tab-stale, close/reopen, and same-frame
  request replacement paths fail closed without changing the prior FM.
- Fresh gates: exact C6.1 9/9; broad sidebar/FM nextest 239/239, run
  `d7202d9b-ffbc-409d-82f8-76ec191429d3`; full nextest 3151/3151 plus only the
  named B0 probe skipped, run `c5232427-adc0-49b9-9898-daf479b623cd`; Linux
  all-target and canonical Windows MSVC clippy; Bun 17/17; Python 64/64;
  fmt/diff/production-unwrap/temp-artifact checks clean.
- Fresh graph is 18,899 nodes / 90,094 edges. `ready` was cross-checked with
  `miller_layout`, `FileManagerSidebarModel`, exact geometry/hit-test, and
  scheduled navigation symbols.

## Verified Checkpoint — C6.2 Current-Location Sidebar Styling

- Durable plan `c3dfa6f` and RED contract `ac4eecb` precede GREEN product
  `b88fc12`; test-only lifecycle closure `a078d98` explicitly covers the
  non-Files and close/reopen boundaries. The RED checkpoint was not published
  alone.
- Exact current authority is derived every frame from open `FmState.cwd` plus
  one exact prepared accessible sidebar item. Closed FM, non-Files tab,
  inaccessible/model-missing/stale path, cwd transition, and close/reopen
  cannot retain an accent pill; no highlight cache or render-time I/O exists.
- The pure row builder reserves the final cell for warning/eject affordances,
  gives warning precedence when an inaccessible ejectable path carries both
  facts, truncates by display cells, and emits both Powerline caps together or
  neither. Zero and narrow widths remain bounded and panic-free.
- Fresh gates: focused sidebar/FM groups 11/11, 60/60, and 56/56; full nextest
  3154/3154 with only the named B0 host probe skipped, run
  `3ffc29fb-d053-4a6c-bbda-86b63745fc64`; Linux all-target and canonical
  Windows MSVC clippy; Bun 17/17; Python 64/64; fmt/diff/production-unwrap/
  artifact checks clean.
- Fresh graph is 18,909 nodes / 90,194 edges. MCP parallel refresh failed on
  the known native extraction path, so the recorded `CBM_WORKERS=1` CLI
  fallback completed with zero extraction errors. `ready` was cross-checked with
  `miller_layout`, `file_manager_sidebar_item_is_current`,
  `file_manager_sidebar_marker`, and `file_manager_sidebar_item_line`.

## Verified Checkpoint — C6.3 Integrated Action Authority

- Durable cross-surface matrix `2648a08` and compile-valid RED contracts
  `a12a870`, `9aad978`, `ab27caa`, and `0905e49` precede product commits
  `40c7ab9`, `dd00f25`, `e7614aa`, and `8b21442`; scheduled-delete test closure
  is `2d974da`. No RED checkpoint is published alone.
- New Folder and Compress remain visible but explicitly disabled because v1
  has no mutation owner. Header and exact-row Copy/Rename/Delete/Send actions
  converge on the existing typed context intent and C4/C5 scheduled authority;
  invalid row projection is checked on a clone before current selection changes.
- Scheduled Open revalidates current exact paths and enabled action state before
  calling `FmState::enter`. Plugin intents revalidate the current installed
  manifest, platform support, selection, and enabled action before invoking the
  existing App-owned plugin command runtime exactly once.
- Stale/reordered/missing/unsupported selection, operation in flight, popup
  close, unsupported intent, FM close/reopen, and manifest drift are consumed
  fail-closed without duplicate mutation, filesystem work in input/render, or
  stale focus/action authority.
- Fresh gates: focused C6.3 nextest 118/118, run
  `41e5dbf8-576c-4e6b-a7eb-eedd9897121b`; full nextest 3160/3160 with only the
  named B0 host probe skipped, run `ec91fccd-12fc-49b9-ae92-0d464de19552`;
  Linux all-target and canonical Windows MSVC clippy; Bun 17/17; Python 64/64;
  fmt/diff/production-unwrap/temp-artifact checks clean.
- Fresh graph is 18,922 nodes / 89,277 edges. Current source snippets for
  `miller_layout`, `dispatch_file_manager_row_action`,
  `consume_file_manager_context_open`, and
  `sync_file_manager_plugin_action` prove freshness beyond `ready`.
- Historical next module C6.4 is now complete at the checkpoint below.

## Verified Checkpoint — C6.4 Visual Polish and Failure Truthfulness

- Durable semantic-role plan `5b8f327` precedes semantic RED/GREEN
  `2362751`/`3e73351`, directory-state RED/GREEN `4ed210e`/`37f760d`, operation
  status RED/GREEN `04b8070`/`792c4d8`, preview-state RED/GREEN
  `3f9a0cd`/`101809c`, and full-frame composition/test closure `03aeb6d` plus
  `f52cb85`. No RED checkpoint is published alone.
- Existing palette roles now provide one pure FM visual seam for canvas,
  identity, headings, dividers, focus, explicit selection, disabled actions,
  warning/error, and terminal operation states. No literal color or render-time
  filesystem/runtime work was added.
- `FmState` prepares typed Available/Missing/PermissionDenied/Unavailable cwd
  state during construction/reload. Empty, missing, denied, unavailable,
  read-only, preview warning/error, running/completed/cancelled/partial/failed,
  and first recovery-path evidence have distinct stable bounded presentation.
- One status row is included in the shared FM geometry used by compute, render,
  and Kitty preview preparation. One/two/three-column layouts, expanded and
  collapsed sidebar, desktop/mobile, alternate palette, context menu, delete
  modal, progress, and image resize targets are covered by exact buffers.
- Fresh gates: full nextest 3171/3171 with only the named B0 real-host probe
  skipped, run `339242c5-a4d2-4989-9583-8e904c6d7b1e`; Linux all-target and
  canonical Windows MSVC clippy; Bun 17/17; Python 64/64; fmt/diff/production-
  unwrap/artifact checks clean.
- Throwaway-XDG headless API smoke proved protocol 16 and typed empty tab JSON.
  A separate 120x30 real PTY capture proved sidebar/header plus
  PARENT/CURRENT/PREVIEW and row-action composition. Both exited semantically
  with zero test process/socket/temp residue; stable Herdr was untouched.
- Fresh graph is 18,974 nodes / 89,775 edges. `ready` was cross-checked with
  current source snippets for `miller_layout`, `FmDirectoryStatus`,
  `file_manager_visual_styles`, and `file_manager_status_line`.
- P4.0, N2.0, and N2.1 are closed below. Their then-current NO-GO evidence did
  not automatically activate a future lane; the later active lane exists only
  because the user supplied and approved new concrete product demand.

## Verified Checkpoint — P4.0 Architecture Evidence Gate

- Live graph/source evidence compared four alternatives per the system-design
  decision process. No production code, dependency, persisted state, protocol,
  runtime, or stable Herdr resource changed.
- S5 is implementation NO-GO: `Compositor` has two fixed layers and `BaseLayer`
  has one explicit terminal/FM swap, but no second dynamic component/page owns
  duplicated event and lifecycle seams.
- At the P4.0 evidence checkpoint, S6 was implementation NO-GO:
  `ShellLayout` already had nested/serde fixtures,
  yet production computes only LeftPanel/CenterContent and `SessionSnapshot`
  persists the concrete sidebar fields. No RightPanel/BottomBar consumer or
  general restore/migration pressure exists.
- S7 is implementation NO-GO: one `Mode` selects one `OverlayLayer`, while
  `render_modal_shell` and `modal_stack_areas` already serve eight and ten
  callers respectively. Existing context-to-confirmation tests cover staged
  focus/close behavior; no simultaneous nested popup requires ownership stack.
- N2.0 is complete in
  `.codex/evidence/n2-path-stable-miller-navigation.md`. Pinned Yazi and Joshuto
  source plus Ranger/Yazi primary docs confirm a bounded parent/current/preview
  projection, not an arbitrarily growing visible column chain. The original
  dynamic/unbounded state-machine idea is implementation NO-GO.
- Both independent source references preserve the path identity of the child
  just exited when leaving to its parent. Herdr alone forces current cursor zero
  in `FmState::leave()`, which selects an unrelated sibling whenever the child
  is not first. N2.1 receives a narrow implementation GO for that observable
  path-stable parent-return delta.
- N2.1 adds zero retained fields/history, no extra directory read, and no
  protocol/server/render/worker surface. Missing/hidden/raced child paths use
  the existing deterministic top/clamp fallback. Per-directory cursor history,
  back/forward, and parent-column sibling controls remain deferred as N2.2.
- Exact RED tests, transition/failure behavior, resource budgets, and complete
  gates are frozen in the evidence file and `.codex/TASKS.md` before Rust work.
- This decision activated N2.1; its completed RED/GREEN/gate checkpoint follows.

## Verified Checkpoint — N2.1 Path-Stable Parent Return

- N2.1 follows the precommitted contract in
  `.codex/evidence/n2-path-stable-miller-navigation.md`. RED `e433a2f` failed
  exactly four path-focus cases because `leave()` selected `alpha` at cursor
  zero instead of departed `target`; two missing/hidden/root characterizations
  passed. RED run: `eeef105d-a35a-4e68-92f6-885a80c3cee1`.
- GREEN `c530836` introduces one private `reload_focusing_path` seam. Ordinary
  reload preserves its existing selected-path/nearest-row behavior; leave
  passes one local departed `PathBuf` as preferred focus and retains zero new
  state/history. The directory snapshot and context refresh still execute once.
- Exact tests are 6/6 and all FM model tests are 65/65. Reorder preserves exact
  path, delete/missing/hidden use bounded fallback, root is a complete no-op,
  selection clears, preview generation converges on the departed child, and
  viewport normalization contains the restored cursor.
- Fresh full nextest is 3177/3177 with only the named real-host probe ignored,
  run `ac096bcc-80aa-45bb-9a78-954c73543cbe`. Linux all-target and canonical
  Windows MSVC clippy pass with `-D warnings`; Bun 17/17; Python 64/64; fmt,
  diff, added-production-unwrap/debug-marker, and ignored inventory clean.
- Fresh graph is 18,997 nodes / 89,826 edges. Direct graph search/snippets return
  current `FmState::reload_focusing_path` and `FmState::leave`; `ready` alone was
  not used as freshness evidence.
- No input/render/protocol/server/persistence/dependency/worker surface changed,
  so a stable runtime was neither touched nor required. Unreleased release docs
  need no new command documentation because keys and visible controls are
  unchanged.
- N2.1 is closed. At this checkpoint N2.2 retained cursor/back-forward history
  remained deferred behind independent demand and a finite-eviction contract;
  the later approved FM1-FM4 program now supplies that bounded trigger.

## Verified Checkpoint — C1.1 Header Action Geometry

- RED contract commit: `0ed5e51` (`test: define file manager header action
  geometry`). GREEN product commit: `c9bfbf9` (`feat: add file manager header
  action geometry`). Intermediate compile-failing RED was never pushed alone.
- `FileManagerHeaderAction` names Copy, Paste, NewFolder, and Delete as
  client-local presentation/input tags; no server or wire-protocol state was
  added.
- One pure geometry seam produces complete, disjoint, priority-ordered,
  right-aligned button rectangles while reserving a readable cwd identity
  width. Narrow layouts progressively hide whole lower-priority actions;
  zero/degenerate areas fail closed.
- Desktop/mobile `compute_view` snapshot the same rectangles into `ViewState`;
  render reads that snapshot and uses the same pure fallback for isolated
  component tests. Closing FM clears the areas. Render performs no mutation or
  filesystem work.
- Full nextest exposed a pre-existing 1–4 ms synthetic/real clock race in the
  multiple-process-generation suppression fixture. Root cause was proven and
  the test-only base clock was moved one second ahead in separate commit
  `9aa1e59`; the exact test and 27-test lifecycle family passed before the full
  suite.
- Fresh gates: C1 geometry/render/ViewState 4/4; full nextest 2986/2986 with
  one named B0 host probe skipped; Linux all-target and canonical Windows bin
  clippy clean with `-D warnings`; Bun 17/17; Python 64/64; fmt/diff clean.

## Verified Checkpoint — C1.2 Header Action Dispatch

- RED contract commit: `dbc6798` (`test: define file manager header action
  dispatch`). GREEN product commit: `7fd01de` (`feat: dispatch file manager
  header action tags`). Intermediate compile-failing RED was never pushed
  alone.
- Private `FileManagerMouseDispatch` distinguishes `NotHandled`, `Consumed`,
  and `HeaderAction(tag)` without adding App/AppState fields or server/wire
  state. The outer mouse router consumes visible header tags before the hidden
  terminal path.
- Only an unmodified left press inside a current named rectangle returns its
  exact Copy/Paste/NewFolder/Delete tag. Identity/gap/outside/hidden/zero,
  stale-closed-FM, right/middle, and modified-left paths cannot invent an
  action.
- C1.2 intentionally executes no filesystem mutation and preserves cwd,
  cursor, and disk entries. N3 must provide explicit selection-sensitive
  content and enablement authority before any action tag can cause a side
  effect.
- Fresh gates: exact dispatch 2/2; full FM input 13/13; full nextest 2988/2988
  with one named B0 host probe skipped; Linux all-target and canonical Windows
  bin clippy clean with `-D warnings`; Bun 17/17; Python 64/64; fmt/diff clean.

## Verified Checkpoint — N3.1 Selection-Sensitive Action-Bar Content

- RED contract commit: `b5cc95c` (`test: define selection-sensitive file
  manager action bar`). GREEN product commit: `510eebc` (`feat: add
  selection-sensitive file manager action bar`). Intermediate compile-failing
  RED was never pushed alone.
- `FileManagerActionBarModel` is a pure ViewState snapshot of selected path,
  display label, file/directory kind, and clipboard count. Source clipboard
  paths remain client-local AppState and persist across FM close/reopen; no
  server or wire state was added.
- Desktop/mobile `compute_view` rebuild the model from current prepared
  `FmState`; navigation and reload-selected-delete transitions cannot leave a
  stale label. Close clears the model; reopen restores the current empty or
  selected state plus clipboard summary.
- The persistent header visibly carries selected name, explicit empty state,
  and non-empty clipboard count while retaining the existing four action tags
  and responsive geometry. Render performs no filesystem or metadata I/O.
- Fresh gates: targeted 3/3; FM regression 135/135; full nextest 2991/2991
  with one named B0 host probe skipped; Linux all-target and canonical Windows
  bin clippy clean with `-D warnings`; Bun 17/17; Python 64/64; fmt/diff clean.

## Verified Checkpoint — N3.2 Explicit Action Authority

- RED contract commit: `446613a` (`test: define file manager action
  authority`). GREEN product commit: `267ad91` (`feat: add explicit file
  manager action authority`). Intermediate compile-failing RED was never
  pushed alone.
- `FileManagerActionState` names both boolean authority and a deterministic
  disabled reason for Copy, Paste, NewFolder, and Delete. Operation-in-flight
  overrides every other reason; selection/clipboard absence, unsupported
  special or broken targets, and read-only destinations then fail closed.
- `FmState` prepares cwd writability and regular-file/directory support during
  state refresh. Render remains pure, and C4 remains responsible for
  operation-time TOCTOU, permission, and partial-failure handling.
- Disabled actions render with a distinct dim style. Mouse input returns an
  action tag only when the current ViewState model explicitly enables it;
  disabled or malformed/stale authority is consumed without changing cwd,
  cursor, clipboard, or filesystem entries.
- Fresh gates: exact authority/preparation/render/dispatch 7/7; broad FM/input/
  render/Kitty regression 165/165; full nextest 2996/2996 with one named B0
  host probe skipped; Linux all-target and canonical Windows bin clippy clean
  with `-D warnings`; Bun 17/17; Python 64/64; fmt/diff clean.

## Verified Checkpoint — C2.1 Row Action Geometry

- RED contract commit: `d4d404e` (`test: define file manager row action
  geometry`). GREEN product commit: `9a15328` (`feat: add file manager row
  action geometry`). The compile-failing RED was kept local until GREEN and
  the complete gate passed.
- `FileManagerRowAction` names SendAgent, Rename, and Delete as client-local
  presentation/input tags. `compute_file_manager_row_geometry` produces the
  visible name rows and row-action areas together from the current responsive
  Miller column and persisted viewport.
- Every action is a complete one-cell symbol (`>`, `r`, `x`) with an exact
  disjoint rectangle. This preserves existing directory/file readability in
  two-column Miller layouts; the first broad run caught and rejected a
  nine-cell prototype that truncated ordinary names.
- Desktop/mobile `compute_view` snapshot the same geometry in `ViewState`;
  component render uses the same pure fallback. Closing FM clears both name
  and action areas. C2.1 adds no server/wire/filesystem behavior.
- Fresh gates: focused invariant plus readability regression 8/8; FM impact
  71/71; full nextest 2998/2998 with one named B0 host probe skipped; Linux
  all-target and canonical Windows MSVC bin clippy clean with `-D warnings`;
  Bun 17/17; Python 64/64; fmt/diff clean.

## Verified Checkpoint — C2.2 Stable Row Action Dispatch

- RED contract commit: `94e4a02` (`test: define file manager row action
  dispatch`). GREEN product commit: `9ef90c6` (`feat: dispatch file manager row
  action tags`). The compile-failing RED was kept local until GREEN and the
  complete gate passed.
- Every `FileManagerRowActionArea` snapshots both absolute visible index and
  stable `entry_path`. An unmodified left press emits `RowAction { action,
  entry_path }` only when the live entry at that index still matches the path
  and remains operation-supported.
- Watcher-style reorder, unsupported targets, hidden/cleared areas, closed FM,
  outside/empty regions, non-left presses, and modifiers fail closed. Row-name
  selection/double-click behavior remains separate and unchanged.
- The outer mouse router consumes row tags before hidden terminal input. C2.2
  deliberately performs no SendAgent/Rename/Delete side effect; C4/C5 must
  provide operation-time authority and TOCTOU/failure handling.
- Fresh gates: exact dispatch/stale/no-side-effect 3/3; all FM input 17/17; FM
  impact 74/74; full nextest 3001/3001 with one named B0 host probe skipped;
  Linux all-target and canonical Windows MSVC bin clippy clean with
  `-D warnings`; Bun 17/17; Python 64/64; fmt/diff clean.

## Verified Checkpoint — N4.1 Cursor-Independent Multi-Selection

- N4.1 is an auditable seven-commit test/product sequence: state RED/GREEN
  `e876223`/`590e376`, lifecycle RED/GREEN `1789bbd`/`5c14439`, gesture/render
  RED `699a6a6`, stable row-identity RED `fc19237`, and integrated GREEN
  `86b618a`. No compile-failing RED checkpoint was pushed alone.
- `FmState` owns a client-local `BTreeSet<PathBuf>` plus stable path anchor,
  separate from cursor focus/preview. Plain selection replaces, Ctrl toggles,
  and Shift rebuilds an inclusive range from the current visible entry order;
  duplicate identities cannot inflate the set and stale targets fail closed.
- Reload/watch reconciliation preserves live path identities across reorder,
  prunes deleted or hidden paths and missing anchors, and successful enter/
  leave clears the old directory selection. No-op file enter preserves state;
  close/reopen starts empty.
- Mouse and keyboard use the same state methods: plain/Ctrl/Shift clicks,
  Space toggle, and Shift+Up/Down range. Combined/unrecognized modifiers fail
  closed. `FileManagerRowArea` now snapshots stable path identity so a valid
  but reordered index cannot select another entry.
- Pure render paints explicit non-cursor rows with `surface1` and keeps the
  single cursor/preview focus at `surface0`. N4.1 grants no filesystem,
  destructive, server, or wire-protocol authority; N4.2 owns bulk authority.
- Fresh gates: focused 7/7, broad FM/watcher/input/render/Kitty 137/137, full
  nextest 3015/3015 with only the named B0 real-host probe ignored, Linux
  all-target and canonical Windows MSVC bin clippy clean with `-D warnings`,
  Bun 17/17, Python 64/64, fmt/diff clean. Fast graph reindex is fresh at
  18,078 nodes / 83,865 edges and returns the new model/input/render symbols.

## Verified Checkpoint — N4.2 Bounded Bulk Authority

- N4.2 is an auditable seven-commit sequence: bulk-model RED `d5e027f`,
  full-frame lifecycle RED `0c76017`, authority GREEN `0302b10`, bounded
  state/input RED `36c815f`, bounded GREEN `57e2a44`, keyboard-atomicity RED
  `50619ff`, and keyboard-atomicity GREEN `cb5a43e`. No RED checkpoint was
  pushed alone.
- Cursor focus alone grants no bulk authority. The action bar derives only
  from explicit path identities and carries zero/one/many content, paths in
  current visible order, file/directory/multiple/unavailable kind, and stable
  disabled reasons.
- Copy/Delete fail closed for one stale or ambiguous path and for any
  unsupported member. Operation-in-flight overrides all actions. Read-only
  target disables Delete/Paste/NewFolder but not Copy; empty clipboard
  disables Paste. C4 still owns execution-time TOCTOU and partial failures.
- Ctrl+A selects all only when the complete unique set fits the 4,096-path
  ceiling; Ctrl+Shift+A clears. Oversized, duplicate, stale-anchor, ambiguous,
  and oversized-range attempts reject atomically without silently selecting a
  subset. Keyboard Shift range cannot move cursor when the selection rejects.
- Render remains pure and filesystem-free. Header identity distinguishes no
  explicit selection, one selected name, and `N selected`; disabled authority
  retains its distinct style. No server or wire-protocol state was added.
- Fresh gates: focused N4.2 12/12 across the staged runs, broad FM/input/render
  112/112, full nextest 3020/3020 with only the named B0 real-host probe
  ignored, Linux all-target and canonical Windows MSVC bin clippy clean with
  `-D warnings`, Bun 17/17, Python 64/64, fmt/diff clean. Fast graph reindex is
  fresh at 18,091 nodes / 84,102 edges and returns current `select_all`,
  `MAX_MULTI_SELECTION_PATHS`, action builder, and keyboard route source.

## Verified Checkpoint — C3.1 File Context-Menu Model

- C3.1 begins with durable test-point plan `d56e3db`, then model RED/GREEN
  `5d6fc1d`/`02c60e7` and adversarial in-flight precedence RED/GREEN
  `d9f28b5`/`0832ccc`. No RED checkpoint was pushed alone.
- `ContextMenuKind::File` reuses the established global popup state. Its pure
  `FileManagerContextMenuModel` derives only from the prepared N4.2 action-bar
  snapshot and carries exact ordered paths plus six typed items: Open, Copy,
  Rename, Delete, Compress, and Send to Agent.
- Cursor-only/no explicit selection produces no model. Single file/directory
  permits supported actions; read-only disables Rename/Delete/Compress only.
  Multiple permits Copy/Delete/Compress and disables single-target actions.
  Unavailable, stale, ambiguous, or unsupported authority fails closed.
- Explicit disabled precedence is OperationInFlight > StaleSelection >
  UnsupportedSelection; malformed/missing action state maps to stale rather
  than granting authority. C3.1 performs no popup opening or action execution.
- Existing workspace/tab/pane/project menu item behavior and static-label API
  remain unchanged. The temporary C3.1 dead-code allowances named their C3.2
  removal condition and were removed when production routing became live.
- Fresh gates: C3.1 5/5, combined existing/new menu models 7/7, full nextest
  3025/3025 with only the named B0 host probe ignored, Linux all-target and
  Windows MSVC bin clippy clean, Bun 17/17, Python 64/64, fmt/diff clean. Fast
  graph reindex is fresh at 18,115 nodes / 84,003 edges and returns current
  model constructor and File kind source.

## Verified Checkpoint — C3.2 File Context-Menu Popup and Lifecycle

- C3.2 is an auditable six-commit RED/GREEN sequence: popup geometry
  `69864d6`/`ad5f8a5`, lifecycle and typed intent `73df647`/`45c151f`, and
  disabled render `1078215`/`0915964`. No RED checkpoint was pushed alone.
- Exact unmodified right-click on a current Miller row or its row-action cells
  requires matching live index/path identity. A selected member preserves the
  explicit bulk set and moves cursor focus; an unselected row becomes the one
  exact selection before the menu model is prepared. Stale, modified,
  non-row, hidden, and zero geometry fail closed.
- The existing global `ContextMenuState` owns placement, hover, keyboard,
  outside-click, and close lifecycle. Popup geometry stays inside the complete
  screen at one/two/three-column breakpoints and keeps all six rows plus
  borders reachable.
- Disabled items stay visibly dim even while highlighted and cannot be taken
  by keyboard or mouse. Enabled activation emits only a client-local typed
  `{ action, paths }` intent. Reorder, delete, unsupported/in-flight authority,
  or FM close is revalidated/cleared before intent; C3 performs no filesystem
  or agent side effect.
- Fresh gates: popup 4/4, lifecycle 3/3, disabled render 1/1, broad FM/global
  menu 51/51, menu/render regression 26/26, full nextest 3033/3033 with only
  the named B0 host probe skipped, Linux all-target and Windows MSVC bin
  clippy clean, Bun 17/17, Python 64/64, fmt/diff clean.
- Parallel graph extraction exposed a native codebase-memory 0.8.1
  `munmap_chunk()` crash. The proxy was not restarted or killed. The supported
  one-shot `CBM_WORKERS=1` CLI fallback completed with zero extraction errors;
  the fresh graph is 18,139 nodes / 86,595 edges and returns the current
  validation function plus right-click and disabled-render tests.

## Verified Checkpoint — C3.3 Plugin File-Action Surface

- RED `0e06181` defined manifest, registry, model, exact-path serialization,
  wrong/unknown context, disabled plugin, ordering, and non-UTF-8 fail-closed
  behavior. GREEN `3c11369` implemented the contract; neither was pushed
  alone.
- Existing neutral plugin types now include `PluginActionContext::File` and
  optional `PluginInvocationContext.file_paths`. The generated next API schema
  is current. This is a backward-compatible JSON API extension; private TUI
  transport protocol remains 16, matching release `v0.7.3` and repository
  precedent for API-only schema additions.
- The file menu appends only enabled, manifest-available, host-supported file
  actions in deterministic qualified-ID order. Duplicate qualified identities
  fail closed. Built-ins remain first; multi-path order is preserved.
- Right-click and activation both rebuild current registry/path authority.
  Disable/reorder/removal races emit no intent. Exact UTF-8 paths produce typed
  public invocation params; non-UTF-8 paths expose no plugin action rather
  than using lossy conversion. C3.3 starts no plugin command and performs no
  filesystem/agent mutation.
- Dynamic labels use terminal display width, including CJK titles. Focused
  C3.3 8/8, plugin/context 35/35, FM/watcher/global-menu 112/112, full nextest
  3041/3041 with only `path_beta_real_host_probe` skipped, Linux and Windows
  clippy clean, Bun 17/17, Python 64/64, schema/fmt/diff clean.
- Sequential graph refresh completed with zero extraction errors at 18,246
  nodes / 85,535 edges. Search and snippets returned current selector,
  typed-param method, Unicode geometry test, and disable-race input test.

## Verified Checkpoint — C4.1 Safe Copy/Move and App Lifecycle

- C4.1 is five atomic RED/GREEN pairs: immutable preflight
  `386ddce`/`a9f022b`, staged COPY `47c753e`/`2848d97`, safe MOVE
  `e422d03`/`606d7ea`, bounded worker `f1590be`/`88cda7f`, and App lifecycle
  `626b7c3`/`98c51e4`. No RED checkpoint is published alone.
- Preflight snapshots exact file identity and destination authority, rejects
  collision/same-path/descendant/symlink/non-UTF-8/read-only/in-flight cases,
  and revalidates before the first write. COPY publishes staged complete trees
  with platform no-replace primitives. MOVE uses atomic same-filesystem rename
  and EXDEV copy-before-delete; partial source-removal failure is explicit.
- One App-owned persistent worker lane converts panic/cancel/disconnect into
  terminal state. Header and context Copy share exact clipboard authority;
  Paste starts no second lane, runs outside render, and reloads only a matching
  destination cwd, so close/reopen cannot project stale entries.
- Gates: operation core 15/15, App/worker 8/8, broad FM/watcher/preview
  147/147, full nextest 3064/3064 with only the named B0 probe skipped,
  Linux/Windows clippy, Bun 17/17, Python 64/64, fmt/diff/temp clean. Supported
  single-worker graph refresh is fresh at 18,453 nodes / 86,399 edges and
  returns current operation state, dispatch, sync, and exact source snippets.

## Verified Checkpoint — C4.2 Safe Trash and Permanent Delete

- C4.2 is an auditable seventeen-commit test/product chain from confirmation
  RED `733d423` through unnamed/root-path hardening GREEN `917cd57`; no RED
  checkpoint is published alone.
- Header and context Delete share exact ordered-path authority and a typed
  modal. Trash is the default. Permanent Delete requires `d` and then a
  separate Enter. Modified keys, stale/reordered targets, closed/reopened FM,
  unsupported paths, no explicit selection, and in-flight work fail closed.
- `src/fm/delete.rs` snapshots immutable symlink metadata plus file identity,
  rejects roots and unnamed paths, and revalidates immediately before each
  mutation. Trash moves one exact entry at a time through the restricted
  `trash 5.2.6` backend; permanent deletion chooses file/symlink versus
  directory primitives without following symlinks.
- Delete and transfer share the existing single worker lane. Ordered item
  states distinguish pending, completed, retained, and failed; partial
  backend errors, cancellation, panic, and disconnect all reach an explicit
  aggregate and per-item terminal projection. Matching-cwd reload remains the
  sole App reconciliation path.
- Gates: focused 29/29, broad FM/watcher/preview/context/plugin 321/321, full
  nextest 3086/3086 with only the named B0 host probe skipped, Linux all-target
  and Windows MSVC bin clippy clean, Bun 17/17, Python 64/64, fmt/diff/temp
  clean. An isolated child used throwaway HOME/XDG trash and proved file plus
  symlink trash while preserving the symlink target. Exact OSV query for
  `trash 5.2.6` returned no vulnerability record.
- Fast graph refresh is fresh at 18,576 nodes / 86,769 edges and returns
  `miller_layout`, `FileManagerDeleteConfirmation`, `DeleteOperationPlan`,
  `execute_delete_operation`, and `FileOperationWorkerTask`; `ready` alone was
  not accepted as freshness evidence.

## Verified Checkpoint — C4.3 Safe Single and Bulk Rename

- C4.3 is an auditable eighteen-commit test/product chain from intent RED
  `2028bce` through shared validation GREEN `c7043e2`; no compile-failing RED
  checkpoint is published alone.
- Context-menu and row Rename converge on one exact current single-target
  modal. Stale/reordered, multi-selected, unsupported, closed/reopened, and
  in-flight authority fails closed before a worker plan exists. The header has
  no Rename control, and the single-name modal deliberately remains
  single-target.
- `src/fm/rename.rs` owns the common platform-aware component validator and
  immutable source identity snapshot. Empty/path-like/NUL/reserved/over-limit
  names fail before scheduling; unchanged input is an explicit no-op. Single
  rename revalidates immediately and uses the strongest available platform
  no-replace primitive for files, directories, and symlinks.
- Bounded typed bulk mappings validate all sources and outputs before the first
  mutation. Chains, swaps, and cycles stage through private collision-safe
  paths, then publish deterministically. Injected staging, publish, rollback,
  panic, disconnect, and cancellation paths distinguish renamed, unchanged,
  restored, retained, and uncertain items; uncertain evidence includes the
  exact recovery path.
- Single and bulk rename reuse the existing one-operation worker lane. App
  completion reloads only the matching current cwd and rejects stale
  close/reopen generations. The typed App bulk boundary is ready for a future
  editor surface without silently converting a multi-selection into the
  single-name dialog. Render remains pure.
- Gates: focused rename/bulk/worker/App regression 163/163; full nextest
  3109/3109 with only `path_beta_real_host_probe` ignored; Linux all-target and
  canonical Windows MSVC bin clippy clean with `-D warnings`; Bun 17/17;
  Python 64/64; fmt/diff/temp-artifact checks clean. Real temporary-filesystem
  tests cover file/directory/symlink, destination/source races, cycles, swaps,
  and recovery failure without leaving `.herdr-rename-stage-*` artifacts.
- `ready` was explicitly rejected when the pre-refresh graph lacked every new
  rename symbol. Fast reindex completed at 18,722 nodes / 88,526 edges and
  returned `miller_layout`, `RenameOperationPlan`, `BulkRenameOperationPlan`,
  `validate_rename_name_component`, and
  `consume_file_manager_bulk_rename_request` from current source.

## Verified Increment — C4.4 Bounded Operation Progress

- TP-C4.4-PROGRESS is an auditable ten-commit RED/GREEN chain from worker/App
  contract RED `aa9c894` through bulk-rename adapter GREEN `cd4368a`; no
  compile-failing RED checkpoint is published alone.
- `FileOperationWorkerProgress` is a latest-value, same-generation worker slot,
  not an event queue. Repeated updates coalesce, `started_items` remains
  monotonic and bounded, completion clears progress, and stale/closed/
  completed generations fail closed.
- Transfer, delete, single rename, and bulk rename report the item entering
  execution through one production `execute_worker_task_with_progress` seam.
  App applies matching-generation progress before completion and projects only
  the bounded started prefix from Pending to Running; render performs no
  filesystem or worker mutation.
- The complete progress chain is worker/App `aa9c894`/`da46bfb`, transfer
  `84db86a`/`2141593`, delete `edc1588`/`d0a0c8a`, single rename
  `3469883`/`94465e2`, and bulk rename `f5ea272`/`cd4368a`.
- The first full-suite run exposed an unrelated OMP lifecycle fixture that
  mixed real and synthetic `Instant` values. Root-cause analysis moved the
  whole fixture onto one explicit monotonic clock in separate test-only commit
  `30d99bd`; the exact test, 33-test lifecycle family, and second full suite
  all passed.
- Fresh evidence: C4 progress/operation regression 57/57; full nextest
  3115/3115 with only `path_beta_real_host_probe` ignored; Linux all-target and
  canonical Windows MSVC bin clippy clean with `-D warnings`; Bun 17/17;
  Python 64/64; fmt/diff/temp-artifact checks clean. The ignored B0 probe was
  separately proven as `1 ignored / 0 failed` without execution.
- The stale pre-refresh graph reported `ready` but had no
  `FileOperationWorkerProgress`. Fast reindex completed at 18,745 nodes /
  87,178 edges and returned the progress type, common worker seam, all four
  observer adapters, and the existing `miller_layout` symbol.

## Verified Increment — C4.4 Generation-Safe Cancellation

- TP-C4.4-CANCEL is an auditable fourteen-commit RED/GREEN chain from single
  rename boundary RED `29572ab` through bulk cancellation precedence GREEN
  `d77858a`; every gap was observed failing before its production fix.
- Transfer retains its existing before-start, staging, between-commit rollback,
  and idempotent token contracts. Delete now checks the token after progress
  but before its irreversible backend call; completed deletes remain Deleted
  while untouched items remain NotStarted.
- Single and bulk rename give an already-observed cancellation precedence over
  later revalidation races and check again at their final safe boundary.
  External replacements remain visible; no publish/staging artifact is
  invented and committed work is never reported rolled back without proof.
- Running operations route `Esc` through typed `FileManagerKeyDispatch` to the
  exact active worker generation. Repeated Esc is idempotent, stale App
  generations fail closed, the FM remains open, and normal Esc/q close behavior
  remains unchanged when no operation runs.
- Buffered completion wins the cancel/completion race: cancellation is rejected
  once the sole terminal result exists, even before App drains it. Correctly
  cancelled work projects `Cancelled` with exact per-item evidence and leaves
  the single lane idle.
- Atomic chain: single rename `29572ab`/`d246f09`, delete
  `43d573b`/`1cf0ca4`, typed key intent `eef9a9b`/`a0d91ec`, end-to-end App
  route `699f21c`/`9eb7f4b`, single rename precedence `f0a8280`/`26484ed`,
  completion race `a66bef7`/`dfe21e6`, and bulk precedence
  `15c7a27`/`d77858a`.
- Fresh evidence: broad C4/input regression 98/98; full nextest 3122/3122 with
  only `path_beta_real_host_probe` ignored; Linux all-target and canonical
  Windows MSVC bin clippy clean; Bun 17/17; Python 64/64;
  fmt/diff/temp-artifact checks clean; B0 skip separately proved as
  `1 ignored / 0 failed` without execution.
- The stale graph said `ready` but lacked `cancel_file_manager_operation`.
  Reindex completed at 18,756 nodes / 87,282 edges and returned the key intent,
  App cancel seam, generation-safe worker method, cancellation tests, and
  existing `miller_layout`.

## Verified Increment — C4.4 Deterministic Watcher Reconciliation

- TP-C4.4-RECONCILE is an auditable RED/GREEN chain covering queued,
  watcher-first, delayed, polling-fallback, selection-pruning, cwd/rebind, and
  same-cwd close/reopen orderings. Every production fix followed an observed
  failure; no compile-failing RED was published alone.
- Matching worker completion and watcher events now converge through one
  watcher-owned reload. A runtime-only baseline binds operation generation,
  cwd, watcher generation/revision, and the exact planned path set without
  adding state to `AppState` or render.
- Own-operation events already queued at completion coalesce with the immediate
  request. A watcher refresh observed after publish owns later completion.
  Exact delayed native bursts are absorbed for one bounded two-second window,
  while mixed or external paths reload immediately and the existing periodic
  reconciliation remains the eventual-correctness safety net.
- Watcher rebind clears requested/owned state. A prior completion cannot use
  path equality to reload a same-cwd reopened FM generation. Polling fallback
  uses the same single lane and immediate reconciliation; selected rename
  paths are pruned by exact identity with a safe cursor fallback.
- Atomic chain: queued ownership `0b04e73`/`9a22d1e`, watcher-first ownership
  `411de3d`/`6bdb97c`, delayed burst ownership `e9361ab`/`38280fb`, same-cwd
  reopen generation `1d7350a`/`779d771`, and fallback/selection cross-check
  `d1a2d2e`.
- Fresh evidence: broad C4/FM regression 126/126; full nextest 3128/3128 with
  only `path_beta_real_host_probe` ignored; Linux all-target and canonical
  `LIBGHOSTTY_VT_SIMD=false` Windows MSVC bin clippy clean with `-D warnings`;
  Bun 17/17; Python 64/64; fmt/diff and operation/staging artifact checks
  clean. Ignored-only inventory listed the single B0 probe without running it.
- The pre-refresh graph reported `ready` at 18,756 / 87,282 but returned no
  `own_operation_reconcile`. Fast reindex completed at 18,786 nodes / 87,697
  edges and returned the production ownership seam, delayed and same-cwd
  lifecycle tests, and existing `miller_layout`.

## Verified Increment — C4.4 Terminal Recovery And Closure Gates

- TP-C4.4-RECOVERY is an auditable seven-commit chain: disconnected-worker
  RED/GREEN `0881976`/`7847a6c`, progress-then-panic coverage `8974f4c`,
  cancellation-to-next-generation coverage `bcc9ef5`, uncertain private bulk
  recovery evidence `7e2af79`, disconnect cleanup idempotence `03b9395`, and
  fixture lint closure `c674296`.
- A worker channel disconnect after bounded progress now terminalizes every
  remaining item, clears runtime-only reconciliation ownership, and replaces
  the dead worker at the prior generation floor. The next operation therefore
  receives a strictly newer generation and the second sync is a no-op rather
  than a hot retry.
- Caught worker panic and generation-safe cancellation already preserved the
  single lane; new App-level tests now prove progress cleanup, exact terminal
  item state, stale-cancel rejection, and successful next-generation reuse.
- Injected bulk staging plus rollback failure leaves one private
  `.herdr-rename-stage-*` artifact during the test. Its exact surviving path and
  payload remain visible in `FileManagerOperationItemState::recovery_path`, the
  operation is `Partial`, and the same lane remains reusable. Test teardown and
  the post-gate scan leave no private artifact behind.
- No second worker field, scheduler, thread name, render-time filesystem work,
  `AppState` runtime authority, or wire-protocol surface was added.
- Fresh evidence: focused recovery 46/46; C4 core 67/67; broad C4/FM 218/218;
  final full nextest 3131/3131 with only `path_beta_real_host_probe` skipped;
  Linux all-target and canonical `LIBGHOSTTY_VT_SIMD=false` Windows MSVC bin
  clippy clean with `-D warnings`; Bun 17/17; Python 64/64; fmt/diff and
  operation/staging artifact checks clean. Ignored-only inventory listed the
  single B0 probe without executing it.
- The pre-refresh graph reported `ready` at 18,786 / 87,697 but returned only
  `miller_layout`, not the new recovery symbols. Fast reindex completed at
  18,793 nodes / 87,788 edges and returned `new_after_generation`, the
  disconnect/panic/private-recovery tests, and `miller_layout`.

## Completed Checkpoint — B2 Native Image Preview

- B2 is an auditable dependency decision plus four RED/GREEN increments and a
  fallback fix from `de1eff5` through `2989434`.
- `image 0.25.10` is restricted to `png/jpeg/gif/webp`. Encoded bytes,
  dimensions, checked pixels, decoder allocation, RGBA output, and target
  placement are independently bounded before untrusted allocation can grow.
- Decode/downscale supports PNG, JPEG, GIF, and WebP; preserves alpha; applies
  orientation-aware aspect fit without upscaling; and maps corrupt,
  unsupported, oversized, non-regular, missing, and decoder-panic paths to
  explicit states.
- A dedicated generation-safe worker owns filesystem/decode work outside
  render. Path, model generation, and pixel target must all match before a
  result can publish; navigation, watcher reload, resize, close/reopen, and
  worker panic cannot paint stale pixels.
- The client-local FM preview uses the existing responsive preview geometry,
  synthetic local placement identity, and existing Kitty encoder/cache. It
  uploads once, repositions without re-upload, replaces and deletes
  superseded content, clears on terminal/FM surface transitions, and leaves
  generic terminal-image reuse semantics intact.
- Non-Kitty hosts get the width-safe `(Kitty graphics req.)` fallback. Ready
  Kitty frames have no text underlay.

## Published Checkpoint — A3 Navigation Remainder

- A3 is an auditable seven-commit RED/GREEN sequence from `d713b71` through
  `9d69c82`; intermediate compile-failing RED commits were never pushed alone.
- `FmState.viewport_start` has explicit cursor-visible and clamp invariants for
  long lists, resize, reload shrink, zero-height, enter, and leave.
- `compute_file_manager_row_geometry` is the shared responsive one/two/three-
  column CURRENT-row geometry consumed by render and snapshotted in `ViewState`
  for input. Degenerate geometry and stale indices fail closed.
- Real runtime mouse dispatch selects on single click, enters only a directory
  on same-path double-click, leaves files selected, bounds wheel navigation,
  refreshes preview state, and consumes center input before hidden panes.
- v1 intentionally has one cursor-owned visual selection. Multi-select and bulk
  authority remain deferred to N4/C2 and require their own RED tests.

## Published Checkpoint — A4 Watcher

- A4 product commit: `01ba91d` (`feat: add live filesystem watching to native
  file manager`).

- Stable dependency: `notify-debouncer-full 0.7.0` with `notify 8.2.0`.
- Pure event normalization, generation filtering, burst coalescing, bounded
  channel drain, overflow recovery, and watcher lifecycle are implemented.
- Runtime ownership lives in `App`; render remains filesystem-free.
- Native watcher is primary. Init/runtime failure enters explicit
  `PollingFallback`; all active watchers also receive a bounded 2-second
  reconciliation safety-net for silent FUSE/NFS/exFAT-class backends.
- `FmState::reload()` preserves the exact selected path when possible and
  safely clamps deleted/renamed/hidden-filtered selections while rebuilding
  parent/preview context.
- Real filesystem create, rename, delete, and 16-file burst convergence is
  covered by a bounded-deadline integration test.
- Product paths: `Cargo.toml`, `Cargo.lock`, `src/app/file_manager_watcher.rs`,
  `src/app/mod.rs`, `src/app/runtime.rs`, `src/fm/watcher.rs`, `src/fm/mod.rs`.

## Published Test-Stability Work

- Test-only commit: `8cd4e89` (`test: make timing-sensitive lifecycle tests
  deterministic`).

- `src/server/headless.rs`: the metadata-expiry test now uses a long real TTL
  and still expires via its existing synthetic deadline.
- `src/terminal/state.rs`: the late lifecycle-hook test now uses one synthetic
  clock for authority, process exit, and late report.
- These fixes were required after full-suite parallel load exposed two existing
  wall-clock races. Keep them separate from the A4 feature commit.

## Published Checkpoint — B0 Image Path Beta

- B0 product/test commit: `bcba84d` (`test: prove native image path beta
  feasibility`).
- A generated 2×2 RGBA PNG round-trips byte-for-byte through existing
  `png 0.17`; truncated input is rejected without panic and no dependency was
  added.
- A synthetic local `HostPlacement` proves stable content/placement identity,
  RGBA upload, display, full-frame deduplication, view redisplay, content
  replacement, placement removal, and superseded-image cleanup through the
  existing `kitty_graphics` lifecycle.
- `paint_local_pane_graphics` now uses the private `frame_graphics_bytes` seam;
  behavior remains cursor save + existing Kitty bytes + cursor restore.
- The ignored probe rendered the four-color/alpha pattern in an isolated Kitty
  0.46.2 X11 window with throwaway XDG and cleared inherited Herdr socket
  variables. A separate Yazi 26.5.6 Path Alpha baseline rendered the source
  image in another throwaway window. Test windows were closed with targeted
  semantic `q`; no stable Herdr process/socket was touched.
- B2 decision: conditional GO. Reuse the existing encoder/cache; require
  bounded decode/allocation, state-refresh-only I/O, selection generation
  safety, cleanup on every transition, and fresh real-host evidence.

## Fresh B0 Verification Evidence

- Path Beta targeted: 4/4 passed; `kitty_graphics` scope: 25/25 passed.
- Full nextest final run: 2916/2916 passed, one intentionally ignored
  real-host probe skipped, no retry.
- `cargo fmt --check`: passed.
- Linux `cargo clippy --all-targets --locked -- -D warnings`: passed.
- Canonical Windows MSVC binary-target clippy with
  `LIBGHOSTTY_VT_SIMD=false`: passed. A stronger exploratory `--all-targets`
  command is not an applicable repo gate because Unix-only integration tests
  import `std::os::unix`/Unix `libc`; the Justfile intentionally uses `--bin
  herdr`.
- Bun integration assets: 5/5; plugin marketplace: 12/12.
- Python maintenance: 64/64.
- `git diff --check`: passed.
- Doctest probe reported no library target, so doctests are not applicable to
  this binary-only package.
- `just` is absent; every applicable `just check` command was run directly.

## Graph and Publication Evidence

- `mcp-proxy.service` readiness was repaired without killing/restarting a user
  process. A 26-server cold start measured 54 seconds; the bounded readiness
  budget is now 120 seconds inside a 150-second systemd start budget.
- Readiness passed with `expected=26 observed=26 critical_tools=14`.
- Full post-A3 graph reindex completed at 17,818 nodes / 83,121 edges.
- `sync_viewport`, `compute_file_manager_row_areas`, and
  `handle_file_manager_mouse` were found as current production graph symbols
  with their call/test connections; freshness was not inferred from `ready`
  alone.
- Full post-N3.1 graph reindex completed at 18,009 nodes / 83,964 edges.
  `FileManagerActionBarModel`, selection/kind types, and
  `compute_file_manager_action_bar_model` are current graph symbols; the pure
  model builder is connected to desktop/mobile compute, render fallback, and
  model tests. Render and lifecycle tests are present. Freshness was not
  inferred from `ready` alone.
- Full post-N3.2 graph reindex completed at 18,026 nodes / 84,120 edges.
  `miller_layout`, `FileManagerActionState`,
  `compute_file_manager_action_bar_model`, `entry_capabilities`, and
  `handle_file_manager_mouse` are current production symbols. The builder is
  connected to desktop/mobile view computation, render fallback, and its
  authority tests; freshness was not inferred from `ready` alone.
- Publication uses sequential fast-forward pushes to both CyPack heads and
  exact remote-SHA equality. `upstream` is never pushed.
- Post-C3.2 graph refresh completed through the supported single-worker CLI
  fallback after parallel extraction crashed the proxy-owned child. Freshness
  was proven at 18,139 nodes / 86,595 edges with
  `validated_file_context_action`, the exact right-click selection test, the
  disabled-render test, and a current source snippet; `ready` alone was not
  accepted.
- Post-C3.3 sequential refresh completed at 18,246 nodes / 85,535 edges and
  returned current `file_manifest_actions`, `plugin_invocation_params`, Unicode
  popup geometry, and end-to-end disable-race evidence; `ready` alone was not
  accepted.
- Post-C4.1 sequential refresh completed at 18,453 nodes / 86,399 edges and
  returned current `FileManagerOperationState`, header dispatch, scheduled
  sync, and exact source snippet; `ready` alone was not accepted.

## Standing Git Authorization

- The user explicitly authorized autonomous commits and pushes for this
  project. Do not repeatedly ask for commit-message alignment.
- Still require targeted staging, lowercase conventional commits, fresh
  proportional verification, fast-forward ancestry, remote SHA verification,
  and CyPack fork-only writes. Never force and never push `upstream`.

## Exact Next Action

1. Begin SF3.1 at delivery gates I2-I6: graph-first trace the existing sidebar
   divider drag, mouse-capture, persistence-dirty, and PTY-resize seams; name
   the protected legacy behavior and verify the current characterization set.
2. Enter I7 only after that drift pass. Add compile-valid RED contracts
   `divider_down_captures_original_constraints` and
   `drag_preview_clamps_without_dirty_or_pty_resize`; run them and require
   missing resize-transaction behavior assertions to fail. Compile/setup
   errors are not RED evidence. The planned RED commit is
   `test: define transactional shell resize`.
3. After valid RED evidence, implement only the smallest bounded transaction
   state needed for capture and preview. Preview must not write persistence or
   resize a PTY; production code waits until the behavior RED is observed.
4. Execute phases sequentially through the approved child plans. Do not mix
   tooling T3.1, Apps/Desktop, S5 registry, S7 popup stack, or unrelated
   refactors into a product commit.
5. Preserve targeted staging, atomic RED/GREEN/refactor commits, direct full
   `just check` equivalent, isolated runtime safety, CyPack-only FF writes, and
   post-publication exact-symbol graph freshness at every phase.

## Verified B2.0 Dependency Decision

- Selected `image 0.25.10` with default features disabled and only
  `png/jpeg/gif/webp`; default `image` was rejected because it adds 78 packages
  including unnecessary rayon/AVIF/EXR surfaces.
- Exact selected delta is 12 packages with no existing-version upgrade, no
  build script, and no proc macro. All license metadata is compatible.
- Package-registry advisories found only two historical `image` ranges, both
  fixed long before `0.25.10`; the other selected packages returned no result.
- Rust 1.96.1 Windows MSVC check passed. Three clean compile samples showed no
  material RSS/wall penalty versus `image` PNG-only; common formats add seven
  packages and about 2.43 MB of check artifacts.
- `image::Limits::max_alloc` is best-effort, so TP-B2.1 additionally hard-
  bounds input bytes, dimensions, checked pixels, decoder total bytes, RGBA
  output, and target placement allocation. Full evidence is in
  `.codex/evidence/b2-image-dependency.md`.

## Fresh B2 Verification Evidence

- B2/FM/Kitty targeted expression: 96/96 passed.
- Full nextest: 2983/2983 passed; one named B0 interactive real-host probe was
  skipped; no fail or retry.
- `cargo fmt --check`, Linux all-target clippy, and canonical Windows MSVC
  binary-target clippy passed with `-D warnings`.
- Bun integration assets 5/5 plus plugin marketplace 12/12; Python maintenance
  64/64; diff-check clean. `just` is absent, so every applicable `just check`
  command was executed directly.
- Isolated Kitty X11 used a unique throwaway XDG root, cleared socket and
  session identity variables, `experimental.kitty_graphics=true`, and a
  workspace rooted at `assets/`. Selecting `logo.png` produced a 517×525 host
  preview whose resized source comparison was exactly 0/271425 pixels
  different. Closing FM returned the same region to one background color.
  `prefix+q` exited semantically; the test process, sockets, and temp root were
  absent afterward. Stable Herdr and its socket were untouched.

## Verified Checkpoint — B1 Text Preview

- B1.1 adds a 64 KiB hard-capped regular-file reader with four-byte UTF-8
  sentinel, exact CRLF/content preservation, explicit truncation metadata, and
  stable missing/permission/non-regular/binary/invalid-UTF-8 states.
- `FmState` prepares content outside render, binds it to `source_path`, and
  preserves a highlight across reload only when path, visible bytes, and
  truncation identity all match.
- B1.2 uses `syntect 5.3.0` with default features disabled and only
  `default-syntaxes`, `default-themes`, and pure-Rust `regex-fancy` enabled.
- Measured synchronous highlighting is too slow for the input/render path
  (64 sample lines: ~460 ms debug / ~40 ms release), so B1.2 requires a
  generation-safe dedicated worker with one active and one replaceable pending
  request. Stale navigation/reload/close generations are rejected; worker
  failure degrades to plain text without App panic or dirty-loop.
- Highlighting and render each cap at 128 lines; Ratatui clips rendered columns.
  Prepared RGB/bold/italic/underline spans map to terminal styles; plain text,
  empty/error, and byte/line truncation states remain explicit.
- Actual lock delta is five packages and no existing-version upgrade:
  `syntect 5.3.0`, `fancy-regex 0.16.2`, `bit-set 0.8.0`, `bit-vec 0.8.0`, and
  `bincode 1.3.3`. Exact OSV rerun found only severity-less
  `RUSTSEC-2025-0141` for unmaintained bincode, with no patched version or new
  security-severity advisory.

## Fresh B1 Verification Evidence

- B1/FM targeted: 64/64 passed.
- Final full nextest: 2948/2948 passed; one named ignored B0 real-host probe
  skipped; no retry.
- `cargo fmt --check`: passed.
- Linux all-target clippy and canonical Windows MSVC bin clippy: passed with
  `-D warnings`.
- Bun integration assets 5/5 plus plugin marketplace 12/12; Python maintenance
  64/64.
- Metadata has only bin/custom-build/test targets, so doctests are N/A.
- `git diff --check`: passed for product paths; `just` is absent, so every
  applicable `just check` command was executed directly.

## Fresh A3 Verification Evidence

- Targeted viewport/geometry/input/render regressions: 164/164 passed at the
  broadest A3-targeted run; dedicated scope tests: 4/4 passed.
- Final full nextest: 2966/2966 passed; one named ignored B0 real-host probe
  skipped; no retry.
- Linux all-target clippy and canonical Windows MSVC bin clippy passed with
  `-D warnings`.
- Bun integration assets 5/5 plus plugin marketplace 12/12; Python maintenance
  64/64; fmt and diff-check clean.
- Isolated real PTY used cleared Herdr socket/identity variables, throwaway XDG,
  and `--no-session`: three Miller columns rendered; single click changed the
  cursor; same-row directory double-click entered `/home/ayaz/2027 weeks`; 25
  wheel-down events moved the viewport to `WEEK_7…WEEK_27`; 40 wheel-up events
  returned to the top clamp. `q` then `prefix+q` exited with code 0; the unique
  temp tree and process were absent afterward.
- `just` is absent; every applicable `just check` command was run directly.
