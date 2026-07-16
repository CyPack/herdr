# Shell Foundation SF4.2 Input Router Progress Evidence

Updated: 2026-07-16 CEST

## Scope and Boundary

SF4.2 introduces one frozen semantic input precedence shared by mouse and
keyboard. It does not render AppDock, migrate Files rendering, change
protocol/server identity, or touch watcher/preview/operation behavior. The
SF4.1 typed Stage closure (`944a9d4c`) and every earlier baseline stay frozen.

## Test-Point Contract (approved SF4.2 REDs)

| ID | Test | Expected result | Reason | Status |
|---|---|---|---|---|
| SF4.2-01 | `shell_input_router_follows_frozen_precedence` | Table-driven: overlay -> capture -> topmost hit -> focused component -> page -> global -> fail-closed, exactly one owner per row | Precedence must be a typed total authority, not implicit if-chain ordering | GREEN |
| SF4.2-02 | `overlay_blocks_every_background_mouse_action` | Every active overlay kind consumes background-targeted mouse events with zero background action | Topmost blocking overlays prevent every background route (design spec) | GREEN |
| SF4.2-03 | `overlay_blocks_background_keyboard_shortcut` | No background capture/shortcut acts while a blocking overlay owns focus | Keyboard parity for the same rule | GREEN |
| SF4.2-04 | `capture_owns_move_and_up_outside_original_rect` | Active capture receives move/up even outside its origin rect | Only one owner captures a pointer gesture | GREEN (characterization) |
| SF4.2-05 | `focus_restores_after_overlay_close` | Closing an overlay restores the previous valid focus owner or template fallback | Deterministic focus restoration | GREEN (SF4.2-05b done: every production overlay entry wired) |
| SF4.2-06 | `collapsed_or_inert_region_cannot_receive_focus` | Collapsed/zero regions expose no focusable target | Hidden geometry must not own input | GREEN (characterization) |
| SF4.2-07 | `stale_hit_generation_fails_closed` | A hit resolved against a non-current `ShellView` generation is consumed inert | Old coordinates must never become authority | GREEN |
| SF4.2-08 | `files_stage_blocks_hidden_terminal_input` | With Files active, no event reaches hidden terminal targets | Fixes the reported curtain/input leak class | GREEN (characterization) |

## SF4.2-01 Atomic TDD Evidence

- RED `92777e23` (`test: define shell focus and input ownership`), run
  `67730dbd-a529-4cae-96eb-25a0056f7473`: compiled, ran exactly one test, and
  failed only on the first frozen-precedence row (`LegacyImplicitChain` versus
  `TopmostOverlay`). The table encodes all seven design-spec tiers, including
  the fail-closed no-owner row.
- GREEN `f4f5e3cb` (`feat: route shell input through semantic ownership`):
  - Pure `route_shell_input(ShellInputRouteContext) -> ShellInputOwner` in
    `src/app/input/shell.rs`; total by construction, fail-closed default.
  - `AppState::shell_key_input_owner()` projects current keyboard ownership:
    topmost overlay = `ContextMenu | ConfirmFileDelete | RenameFile` modes,
    active capture = `shell_resize_active()`, focused component = open FM or
    `AttachFile`, remaining input = global dispatch. Keyboard has no
    positional hit; v0 has no page/template shortcut owner yet.
  - `App::handle_key` now selects its tier through the router with behavior
    preserved exactly; the former mode-dispatch tail became
    `handle_global_key_dispatch`. Granted-but-unmappable tiers consume
    fail-closed with `debug_assert` documentation.
- GREEN exact run `f3dc189a-5f79-477e-8ed5-8c1efad91b3a`: 1/1.
- Broad input regressions (app::input, handle_key, context menu, shell
  resize, FM keys, attachment): 401/401, run
  `fb209eec-6050-48f2-bbbb-f9a3c9c914da`.
- Full Nextest: 3,301/3,301 passed, one named B0 real-host probe skipped,
  zero retry. `cargo fmt --check`, Linux all-target Clippy, canonical Windows
  MSVC bin Clippy: PASS with `-D warnings`. Bun 5/5 + 12/12; Python 64/64;
  diff and added-production-`unwrap()` checks clean.
- Publication: sequential fast-forward pushes only to CyPack; both refs equal
  exact SHA `f4f5e3cbb65e391b073c57ebe750d10dddb5d9b1`; `upstream` untouched.
- Post-publication sequential graph refresh: 20,410 nodes / 93,605 edges.

## SF4.2-02 Reconnaissance (recorded before its RED)

Current mouse ownership is distributed, not total:

- `App::handle_overlay_mouse` consumes only six overlay modes (`AttachFile`,
  `ConfirmFileDelete`, `ReleaseNotes`, `ProductAnnouncement`, `Navigator`,
  `KeybindHelp`) and returns `false` for every other mode.
- `ContextMenu`, `GlobalMenu`, `Settings`, `Rename*`, `ConfirmClose`,
  `OpenExistingWorktree` mouse ownership lives inside
  `AppState::handle_mouse` (`src/app/input/mouse.rs`) behind per-branch mode
  guards.
- Leak candidates observed in `handle_mouse_without_agent_frame_action`
  (`src/app/input/mod.rs`): the sidebar-divider double-click branch has no
  mode/overlay guard, so a double left-click on the divider can execute
  `reset_sidebar_resize_to_preferred()` while `Mode::ContextMenu` is open and
  the same event also reaches the menu's outside-close path; wheel events in
  `Mode::ContextMenu` fall through `AppState::handle_mouse` past the
  left-down-only menu branch toward background scroll handlers.
- The SF4.2-02 RED must prove the blocking rule end-to-end for every overlay
  kind against background targets (sidebar row, divider, pane area, FM row);
  full gesture-family coverage (right/middle/modified, drag, double-click
  timing) remains SF4.3 scope.

## SF4.2-02 Atomic TDD Evidence

- RED `41362e89` (`test: require overlay blocking for background mouse
  actions`), run `e86799e4-9e32-49a8-9af8-3e87df750073`: the control phase
  proved the fixture reaches real background targets (sidebar wheel moved the
  selection without an overlay), then the overlay phase failed exactly on the
  leak — a background sidebar wheel moved `selected` 1 -> 0 while the context
  menu stayed open. The test also pins the divider contract: the outside
  click that closes the menu must not prime a double-click gesture, and a
  final control phase proves the normal divider double-click reset survives
  (over-blocking guard).
- GREEN `017ba97f` (`feat: block background mouse routes under topmost
  overlays`):
  - `AppState::shell_mouse_input_owner()` projects the mouse overlay tier
    through the frozen router; `mouse_blocking_overlay_active()` classifies
    all 23 modes exhaustively (Terminal/Prefix/Navigate/Copy/Resize are the
    only non-overlay modes) so a new mode must choose a side explicitly.
  - `handle_mouse_without_agent_frame_action` keeps every background
    pre-branch (file-manager surface, divider gestures, URL clicks) inert
    while a blocking overlay owns the mouse.
  - `AppState::handle_mouse` gained one total early `Mode::ContextMenu`
    ownership block: Moved hovers, left-down dispatches/closes, a
    re-targeting right-click falls through to the shared open arms, and
    every other gesture (wheel, drags, middle) is consumed fail-closed. The
    old scattered ContextMenu branches were removed.
- GREEN exact run `a7ffb0a9-dabc-4e86-9170-a74e1e29a2fa`: 1/1 with both
  control phases. Broad input/menu/FM/sidebar regressions: 555/555, run
  `e9d07eac-8437-4dfc-84f7-1ef2e77b2cde`.
- Full Nextest: 3,302/3,302 passed, one named B0 skip, zero retry; fmt,
  Linux all-target Clippy, canonical Windows MSVC bin Clippy with
  `-D warnings`, Bun 5/5 + 12/12, Python 64/64, diff and
  added-production-`unwrap()` checks clean.
- File-boundary note: the SF4.2 plan listed `mouse.rs` only implicitly
  ("subject to graph/source confirmation"); the source trace proved
  ContextMenu mouse ownership lives there, so the edit is inside the SF4.2
  semantic boundary.
- Publication: both CyPack refs equal exact SHA
  `017ba97f26ce111070f83ecf3c9306abfc756dcc`; `upstream` untouched. Fresh
  sequential graph: 20,421 nodes / 94,238 edges with current
  `shell_mouse_input_owner`, `mouse_blocking_overlay_active`, and the new
  blocking test.

## SF4.2-03 Atomic TDD Evidence

- RED-ability was verified before writing the test: a launcher click can open
  the global menu while a sidebar resize capture stays active, and the
  keyboard overlay tier only recognized three modes, so arrow keys resized
  the sidebar under the open menu and Esc cancelled the capture instead of
  closing the menu.
- RED `bb6f8970` (`test: require overlay keyboard ownership over active
  capture`), run `3c0df85b-3053-4574-8708-d591a0ca7bea`: the control phase
  proved the capture consumes keys without an overlay (preview 26 -> 27),
  then the overlay phase failed exactly on the leak (`Some(28)` — the menu
  key adjusted the capture). The test also pins Esc-closes-overlay-only and
  capture resumption after the overlay closes.
- GREEN `efe6446b` (`feat: give blocking overlays keyboard ownership over
  captures`):
  - The keyboard and mouse builders now share one exhaustive
    `blocking_overlay_active()` classifier; `AttachFile` moved to the
    keyboard overlay tier and the focused-component tier is the open file
    manager only.
  - `handle_key`'s TopmostOverlay arm dispatches ContextMenu/
    ConfirmFileDelete/RenameFile/AttachFile directly and routes every other
    overlay mode through the same mode-guarded global dispatch — inside the
    overlay tier, so captures and the FM can never intercept overlay keys.
  - One fixture correction:
    `app_esc_cancellation_is_generation_safe_and_lane_reusable` inherited
    `Mode::Onboarding` from a default-config `App::new` while driving Esc
    into an open FM — an unreachable production state; the fixture now sets
    the realistic `Mode::Terminal`. The no-fail-fast sweep proved this was
    the only affected test.
- GREEN exact run `4870639f` (2/2 with the corrected fixture); broad
  keyboard/overlay regressions 482/482 (run `85df300d`); full Nextest
  3,303/3,303 plus only the named B0 skip; fmt, Linux/Windows Clippy, Bun
  5/5 + 12/12, Python 64/64, diff/unwrap checks clean.
- Publication: both CyPack refs equal exact SHA
  `efe6446bc77495176e061c5d3231d28b7bc5fe04`; `upstream` untouched.

## SF4.2-04 Characterization Evidence (valid-RED impossible — recorded)

- RED-ability was investigated first and refuted with source evidence: the
  `Drag(Left)`/`Up(Left)` arms route by `DragState::SidebarDivider` with no
  coordinate re-resolution (`preview_sidebar_resize` at any position;
  `drag.take()` commit on any Up), pane forwarding is gated on
  `drag.is_none()`, and the theoretical selection-before-capture ordering
  hole is unreachable because every `Down(MouseButton::Left)` clears the
  selection (`src/app/input/mouse.rs` line ~256) before a capture can begin
  and selections are only anchored by pane downs.
- Test-only commit `119e4a2d`
  (`test: characterize capture ownership outside origin rect`), first run
  GREEN `9720468f-aace-4923-acdf-3b419acdf4d0` (SF1 precedent: the
  characterization freezes existing behavior). It proves end-to-end through
  `AppState::handle_mouse`: divider down begins the capture, drags over the
  pane area (preview 31), over sidebar workspace rows (min-clamp 18, no
  selection/press/reorder movement), and to the far corner (max-clamp 36)
  all stay capture-owned, and an Up far outside the origin rect commits
  exactly once (width 36, capture closed, one dirty mark).
- Full Nextest after the characterization: 3,304/3,304 plus only the named
  B0 skip; fmt and Linux/Windows Clippy clean. Both CyPack refs equal exact
  SHA `119e4a2d5026af2c4f4e2e23a9aaa27ac2134804`.

## SF4.2-05 Design Analysis (completed — implementation is the next unit)

RED-ability is CONFIRMED. The genuine gap: `leave_modal`
(`src/app/input/modal.rs:362`) always restores Terminal/Navigate, dropping a
non-default previous focus owner. Reachable flows: the launcher explicitly
allows `Mode::Resize` (`launcher_enabled` includes it), so
GlobalMenu-from-Resize is a real user path; the `Down(Right)` context-menu
arms have no mode guard, so ContextMenu-from-Resize and ContextMenu-from-Copy
are reachable too (the Copy case additionally strands `copy_mode: Some` under
`Mode::Terminal` — a state inconsistency).

Planned RED: open GlobalMenu from `Mode::Resize`, press Esc, assert the mode
returns to `Resize` (today: `Terminal`) — plus a ContextMenu-from-Copy row
asserting `Copy` restore while `copy_mode` is live.

Approved GREEN design (production-grade, NOT a one-field hack):
- One client-local `overlay_return_mode: Option<Mode>` on `AppState`, set
  FRESH at EVERY overlay entry site through one `enter_overlay_mode(Mode)`
  helper (entry inventory, non-test: `worktrees.rs:125/152/234/430`,
  `file_rename.rs:138`, `file_delete_confirmation.rs:34`, `mod.rs:1358`,
  `settings.rs:365`, `actions.rs:468/555/1975`, `modal.rs:100/105` plus the
  three right-click context-menu arms and FM context-menu opens in
  `mouse.rs`/C3 paths). Wiring every entry removes the stale-restore hazard
  by construction; partial wiring was analyzed and REJECTED (a lingering
  value from an unwired entry could restore a long-dead mode).
- `leave_modal` consumes (`take`) the value and restores it only if still
  valid (`Resize` requires an active workspace/pane; `Copy` requires
  `copy_mode.is_some()`); otherwise the existing Terminal/Navigate fallback.
- Pleasant emergent property: overlay-to-overlay chains (GlobalMenu ->
  Settings, ContextMenu -> Rename) naturally restore the original owner when
  the chain closes, matching user intent.
- Classification: refactor-risk (2+ core surfaces, UI/input state
  projection) — per HP4 run the characterization inventory over existing
  `leave_modal` expectations before edits, and expect several fixtures that
  pin `mode == Terminal` after close from Terminal-origin (unchanged
  behavior) to stay green; only non-default-origin flows change.

## SF4.2-05 Atomic TDD Evidence (scoped GREEN — deliberate deviation recorded)

- RED `8b1882eb` (`test: require focus restore after overlay close`), run
  `acc45dd5-2881-4b5e-b5d1-38bc2de8d531`: the Terminal-origin control row
  passed, the Resize-origin row failed exactly (`Terminal` versus `Resize`),
  and an invalid-owner row guards blind restoration.
- GREEN `5eb63763` (`feat: restore previous focus owner after overlay
  close`): client-local `AppState.overlay_return_mode` (never persisted) +
  `enter_overlay_mode()` (remembers `Resize`/`Copy` from non-overlay
  origins, preserves the original owner across overlay-to-overlay chains,
  clears stale values by construction) + `leave_modal` consumes (`take`) and
  restores only while valid (`Resize` needs an active workspace; `Copy`
  needs `copy_mode`).
- DEVIATION from the full-sweep design, recorded with justification: only
  `open_global_menu` and `open_keybind_help` are wired in this slice. Every
  GlobalMenu exit was audited — Esc/outside/launcher-toggle/Detach/
  ReloadConfig consume via `leave_modal`; Keybinds/Settings/WhatsNew chain
  into overlays whose dismissals also `leave_modal` (KeybindHelp Esc,
  Settings x3) — except `dismiss_release_notes`'s direct exit, which now
  drops the remembered owner explicitly. Lingering restore is therefore
  impossible within the wired scope. Wiring the remaining entry inventory
  (ContextMenu x4, Rename*, worktree modals, Settings/Navigator/AttachFile/
  ConfirmClose direct opens) is SF4.2-05b and reuses `enter_overlay_mode`.
- GREEN exact run `3c2df96c` (1/1, all three rows); full Nextest 3,305/3,305
  plus only the named B0 skip; fmt, Linux all-target and Windows MSVC bin
  Clippy clean; zero added production `unwrap()`. Both CyPack refs equal
  exact SHA `5eb63763ca5bbd00b3d7c100207462a3c8b18b02`.

## SF4.2-05b Atomic TDD Evidence (full entry sweep — SF4.2-05 now CLOSED)

- RED `27f8699f` (`test: require copy session restore after context menu
  close`), run `acc45dd5` follow-up: the new ContextMenu-from-Copy row
  failed exactly at the target assertion (`Terminal` versus `Copy`,
  shell.rs:630) with a live `copy_mode` fixture, entered through the real
  production entry `open_project_new_chat_menu` and closed through the real
  `leave_modal` (via the test Esc dispatcher that mirrors
  `handle_context_menu_key_via_api`). The three existing rows stayed green.
- GREEN `3880c66b` (`feat: wire every overlay entry through focus
  remembering`): all 22 remaining production overlay entries now call
  `enter_overlay_mode` — ContextMenu x5 (`mouse.rs` workspace/tab/pane
  right-click arms, FM file menu in `input/file_manager.rs`, project
  new-chat menu in `input/sidebar.rs` — an entry the original inventory had
  missed and this sweep found), Rename family x5 (`modal.rs`
  workspace/tab/pane/new-tab + `file_rename.rs` RenameFile), Confirm family
  x4 (`file_delete_confirmation.rs`, `open_confirm_close`,
  `confirm_implicit_worktree_group_close`, ConfirmRemoveWorktree), worktree
  dialogs x3 (NewLinked + OpenExisting x2), full-screen overlays x5
  (ProductAnnouncement chain, Settings, AttachFile, Navigator, ReleaseNotes
  chain). 22 insertions / 22 deletions — the mode transition itself is
  unchanged at every site.
- Structural completeness proof: a sweep over every overlay `Mode` variant
  (short and full `crate::app::Mode::` paths, test modules excluded by
  `mod tests` boundary) finds ZERO remaining production direct overlay
  assignments; the six remaining full-path assignments all set
  `Mode::Terminal` (exits, not entries). The stale-restore hazard is now
  closed by construction: every entry refreshes `overlay_return_mode`.
- Consciously out of scope (recorded, not hidden): direct-exit paths that
  bypass `leave_modal` (worktree accept flows,
  `close_agent_attachment_picker`, `close_files` context-menu teardown in
  `actions.rs`) do not restore a Resize/Copy owner — they cannot produce a
  stale restore (next entry overwrites), they only skip the nicety. Exit
  normalization onto `leave_modal` is a candidate follow-up for SF4.3.
- GREEN full gate: focused 1/1 (run `912ce012`); full Nextest
  `--no-fail-fast` 3,305/3,305 passed + only the named B0 skip, zero
  regressions (run `739e220a`); `cargo fmt --check`, Linux all-target
  Clippy, Windows MSVC bin Clippy all clean with `-D warnings`;
  `git diff --check` clean; zero added production `unwrap()`.
- Publication: FF-proof for both refs, sequential pushes to CyPack only;
  `feat/native-fm` == `master` == exact SHA
  `3880c66b1d3b58771f3f1dfc6aebc2ace6c396c7`; `upstream` untouched.

## SF4.2-06 Evidence (characterization — valid RED refuted with source proof)

- RED-ability was investigated BEFORE writing the test and refuted at every
  candidate seam: `flatten_region_hits` drops every empty rect before a hit
  can exist (`src/ui/shell/view.rs:161`, `!rect.is_empty()`); `hit_at`
  requires exact generation plus containment (`view.rs:87-96`);
  `on_sidebar_divider` fails closed for collapsed state before any rect math
  (`src/app/input/sidebar.rs:460`) and additionally requires
  `sidebar.width > 0`; both sidebar toggle rects return `Rect::default()`
  for degenerate areas (`src/ui/sidebar.rs:1800-1820`). No reachable state
  produces a hidden focusable target, so per the SF1/SF4.2-04 precedent the
  slice landed as one `test:` characterization commit `3580ff19`
  (`test: freeze inert region focus exclusion`).
- `collapsed_or_inert_region_cannot_receive_focus` (`src/ui/shell.rs`)
  freezes four rows end-to-end through the real
  `compute_shell_view`/`hit_at` path: expanded control (probe proves it hits
  the live sidebar), Hidden collapse (no LeftPanel hit; the former sidebar
  position resolves to the CURRENT WorkspaceStage; the pre-collapse
  generation resolves nothing), compact 4-cell rail (visible collapsed
  geometry KEEPS hit authority — over-inerting guard), and zero-area outer
  geometry (empty hit list, `hit_at` always None).
- Companion `collapsed_sidebar_exposes_no_divider_capture`
  (`src/app/input/shell.rs`) pins the previously UNPINNED collapse guard
  with an adversarial stale non-zero `view.sidebar_rect`: the control row
  proves the probe hits the live divider column, the collapsed row proves
  only the guard denies capture authority — deleting the guard now fails a
  test.
- Gates: exact 2/2 (run `f3cad287`); full Nextest `--no-fail-fast`
  3,307/3,307 plus only the named B0 skip; fmt, Linux all-target Clippy,
  Windows MSVC bin Clippy clean with `-D warnings`; `git diff --check` and
  added-production-`unwrap()` clean (test-only commit, zero production
  lines).
- Publication: FF pushes to CyPack only; both refs equal exact SHA
  `3580ff1986e524a67a6ff7c33bd6056afe75c2ea`; `upstream` untouched.

## SF4.2-07 Atomic TDD Evidence

- RED `bb3ac54d` (`test: require current generation hit resolution`): the
  first row failed exactly (`GlobalShortcut` versus
  `TopmostHit(LeftPanel)`) — the production mouse builder carried no
  positional tier. The RED compiled through a test-local
  `shell_mouse_owner_at` seam mirroring the then-current no-position
  builder (SF4.2-01 seam precedent); the GREEN replaced the seam body with
  the real position-taking call.
- GREEN `c6b024ce` (`feat: resolve mouse hits against current shell
  generation`):
  - New crate seam `ShellView::region_hit_at(generation, position)`
    projecting `hit_at` (whose `#[allow(dead_code)]` is now removed — the
    router consumes it); the generation gate means stale coordinates
    resolve to nothing by construction.
  - `shell_mouse_input_owner(position)` populates the topmost-hit tier
    from the EXACT current `ShellView` generation. Mouse captures stay
    routed through `DragState` (SF4.2-04 characterization) and the
    focused-component mouse tier is documented as arriving with SF4.2-08.
  - The single caller (`handle_mouse_without_agent_frame_action`) passes
    the event position; it still consumes only the `TopmostOverlay`
    comparison, so dispatch behavior is bit-identical for every existing
    flow — hit-owned events keep flowing through the legacy mode-guarded
    dispatch until SF4.3/SF6 consume semantic targets.
  - Test rows: live sidebar position → `TopmostHit(LeftPanel)`; blocking
    overlay outranks the hit; after a geometry change the SAME coordinates
    re-resolve to `TopmostHit(WorkspaceStage)` (vanished authority is gone
    with its generation); an outside miss falls through without inventing
    a hit.
- Gates: exact 1/1 (run `f5fe5087`); full Nextest `--no-fail-fast`
  3,308/3,308 plus only the named B0 skip; fmt (one mechanical caller
  reformat), Linux all-target Clippy, Windows MSVC bin Clippy clean with
  `-D warnings`; diff and added-production-`unwrap()` clean.
- Publication: FF pushes to CyPack only; both refs equal exact SHA
  `c6b024ced8116d88eaf04d1a660d41cd7a86afeb`; `upstream` untouched.

## SF4.2-08 Evidence (characterization — valid RED refuted empirically)

- Recon enumerated every candidate leak path with FM open BEFORE the test
  was written: in-center events all terminate in the FM dispatcher's
  fail-closed consumption (its final `_ => {}` plus tail `Consumed`);
  keyboard belongs to the `FocusedComponent` tier (SF4.2-01/03); the
  `[+]`/`[w]` agent-frame pre-branches are compute-guarded
  (`compute_agent_attachment_action_area` and the worktree twin both return
  `None` while `file_manager.is_some()`); pane double-click/focus/URL
  branches sit AFTER the FM dispatch return. No reachable leaking event was
  found, and the empirical matrix passed on first run — per the SF1/04/06
  precedent the slice landed as one `test:` characterization commit
  `20f659c1` (`test: seal hidden terminal input under files surface`).
- `files_stage_blocks_hidden_terminal_input`
  (`src/app/input/file_manager.rs`) drives the FULL production
  `App::handle_mouse` entry over a computed view with a real
  workspace/pane/agent-terminal underneath: an 8-kind event matrix
  (left down/drag/up, move, both wheels, middle, right) at covered
  non-FM-control terrain asserts no selection anchor, no mode change, no
  menu, and a surviving Files surface; the keyboard tier is asserted
  `FocusedComponent`. The CONTROL phase closes the Files surface,
  recomputes the view, proves the probe sits on the live pane, and shows
  the SAME press then anchors a terminal selection — the seal cannot pass
  vacuously.
- Gates at the SF4.2 CLOSURE checkpoint (head `20f659c1`): full Nextest
  `--no-fail-fast` 3,309/3,309 plus only the named B0 skip; fmt; Linux
  all-target Clippy and Windows MSVC bin Clippy with `-D warnings`; Bun
  5/5 + 12/12; Python 64/64; `git diff --check` clean; zero added
  production `unwrap()` (test-only commit).
- Publication: explicit FF proof (`merge-base --is-ancestor` for both
  refs), sequential pushes to CyPack only; both refs equal exact SHA
  `20f659c17c14a5988c15a02781eb017de5168ae9`; `upstream` untouched.

## SF4.2 MICROPHASE CLOSED — 8/8 slices GREEN

Chain: 01 `92777e23`/`f4f5e3cb` router · 02 `41362e89`/`017ba97f` overlay
mouse · 03 `bb6f8970`/`efe6446b` overlay keyboard · 04 `119e4a2d` capture
characterization · 05 `8b1882eb`/`5eb63763` + 05b `27f8699f`/`3880c66b`
focus restore with the full overlay-entry sweep · 06 `3580ff19` inert
regions characterization · 07 `bb3ac54d`/`c6b024ce` current-generation hit
tier · 08 `20f659c1` hidden-terminal seal characterization. Test inventory
grew 3,301 -> 3,309 across the microphase.

Noted for SF4.3 (recorded, not started): normalize direct overlay exits
(worktree accepts, `close_agent_attachment_picker`, `close_files` teardown)
onto `leave_modal` so they also restore a remembered Resize/Copy owner;
non-ContextMenu overlay modes' in-dispatch wheel arms; FM double-click
across overlay episodes; consume `TopmostHit` semantic targets in dispatch.

## Exact Next Microtask

SF4.2 is closed. Next per the frozen priority chain: SF4.3 cross-layer
surface projection and render purity (design/plan section "Task SF4.3",
`src/ui.rs`, `src/ui/surface_host.rs`, `src/ui/compose.rs`) — start with
its RED catalog and the SF4.3 candidates recorded above. Then SF4.4 + SF4
closure, SF5 AppDock, SF6 Files-to-NativeFiles-Stage, FM1-FM5.
Change-pipeline T3.1 stays paused until the product phase closes.
