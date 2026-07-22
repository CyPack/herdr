# Herdr Files Focus Ownership Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the most recently accepted Locations Rail or Miller Trail interaction the single truthful owner of directional input, active-row styling, and File Action Bar authority.

**Architecture:** Keep ownership entirely in the existing client-local `FileManagerLocationsFocus` seam. `FileManagerLocationsState` exposes named Rail/Trail transitions; current-frame typed Files input transfers that owner only after validation; `TrailState::active_col()` remains subordinate to Trail ownership; render projects one shared active-cursor style; and every File Action Bar preparation and dispatch boundary rejects authority while Rail owns focus.

**Tech Stack:** Rust, ratatui, crossterm, cargo-nextest, the existing bounded File Manager I/O worker, deterministic Ratatui `TestBackend` fixtures, Playwright Chromium, Codebase Memory MCP, and the repository's `just` recipes or their exact children when `just` is unavailable.

**Product contract:** `docs/superpowers/specs/2026-07-22-herdr-files-focus-ownership-design.md` at commit `bf9fcf46`.

## Global Constraints

- Never connect to, stop, restart, reinstall, update, signal, or otherwise touch the stable Herdr process, stable socket, `~/.config/herdr`, or any user-session process.
- Every runtime check uses `.local/ISOLATED-DEV-TEST.md`, throwaway XDG roots, and an environment with `HERDR_SOCKET_PATH`, `HERDR_CLIENT_SOCKET_PATH`, `HERDR_PANE_ID`, `HERDR_WORKSPACE_ID`, `HERDR_TAB_ID`, and `HERDR_ENV` unset.
- Never touch, delete, stage, or commit the user-owned untracked `.superpowers/` directory.
- Work only on `feat/native-fm`; publish only with `git push origin HEAD:feat/native-fm` to the CyPack fork. Never push, open an issue, or open a PR against upstream.
- Preserve all published and local predecessor commits. Historical A2.2 is complete; checkpoint `b4ac62a0` and the approved FFO spec commit are rollback anchors, not reset targets.
- Use exact-path staging. `git add -A`, directory-wide staging, broad globs, force push, and unrelated cleanup are forbidden.
- The commit messages listed in this plan become aligned only when the user approves this plan. Never push a RED head.
- Use `cargo nextest run`, never `cargo test`, for automated Rust behavior evidence. A zero-test filter, filter parse error, compilation-only failure, retry-only green, SIGTERM, or unrun test is not RED or PASS evidence.
- Add every behavior-changing assertion before its production code, observe an assertion-level RED, implement the minimum GREEN, then refactor only while green.
- Re-read every source immediately before editing it. Use Codebase Memory graph tools before filesystem search for code discovery.
- `compute_view()` may normalize geometry-owned state. `render(&AppState)` remains immutable and performs no filesystem access, request submission, cursor correction, selection mutation, or focus transfer.
- Add no dependency, server/protocol field, platform API, worker, channel, cache, timer, debounce, hover-focus model, or global focus manager.
- Keep vertical cursor movement, speculative preview, and explicit activation separate. FFO changes region ownership only; it does not reopen accepted FMN/FLF/FMH movement laws.
- Never blindly regenerate Playwright PNGs. First prove exporter determinism twice, run snapshots with updates disabled, inspect each changed image, and update only FFO-explained pixels in exact named snapshots.
- Pinned Home/Desktop/Downloads pre-warming remains the separate measurement-first FMN-6 lane.

---

## Verified Current Architecture

Codebase Memory reports 24,217 nodes and 128,975 edges. Freshness is established by resolving `flf_scale_locations_follow_navigation`, introduced at predecessor HEAD `b4ac62a0`; `ready` alone is not proof.

~~~text
HeadlessServer::handle_client_input_events
  -> App::route_client_events
  -> App::handle_mouse_without_agent_frame_action
  -> App::handle_file_manager_mouse_at
  -> current Files/Locations/Trail generation checks
  -> typed row/header/action/body target
  -> focus-owner transition
  -> exact Rail or Trail reducer
  -> render-needed projection
~~~

~~~text
current focus owner + Trail selection + clipboard + writable/operation state
  -> compute_file_manager_action_bar_model
  -> prepared current-frame enabled/disabled state
  -> header/context/plugin/rename request
  -> dispatch-time recomputation from current AppState
  -> exact intent or fail closed
~~~

| Seam | Current owner | Planned change |
|---|---|---|
| Top-level Files focus | `src/app/file_manager_locations.rs:26-30,56-65,221-265` | Keep `FileManagerLocationsFocus`; add named boolean-returning `focus_rail`/`focus_trail` transitions and centralize direct writes |
| Keyboard routing | `src/app/input/file_manager.rs:188-287,301-466` | Preserve owner-first routing and FMH horizontal laws; add regressions only |
| Trail row/wheel routing | `src/app/input/file_manager.rs:689-877,910-1236` | Transfer owner only after live typed acceptance for row, modified row, right click, row action, wheel, horizontal scroll, and empty/detail body |
| Trail subordinate focus | `src/fm/trail.rs`, `src/fm/mod.rs` | No semantic change; `active_col()` remains subordinate to Trail ownership |
| Action state types | `src/app/state.rs:890-914` | Add `InactiveFocusOwner`; preserve the selection payload and four-action catalog |
| Action model | `src/ui/file_manager.rs:298-341` | Accept current owner; Rail reason outranks operation/selection/clipboard reasons |
| Action consumers | `src/ui.rs`, `src/ui/file_manager.rs`, `src/app/input/file_manager.rs`, `src/app/input/modal.rs`, `src/app/api/plugins/mod.rs`, `src/app/file_operation_worker.rs`, `src/app/file_rename.rs` | Pass current focus at paint, context creation, validation, plugin, rename, copy/delete, and paste boundaries |
| Header hit dispatch | `src/app/input/file_manager.rs:1089-1217` | Require current Trail owner plus prepared enabled state; stale enabled geometry is consumed |
| Rail renderer | `src/ui/file_manager/locations.rs:338-448` | Reuse central active cursor; replace underlined origin with bold accent context |
| Trail renderer | `src/ui/file_manager/trail_view.rs:657-733` | Keep Trail-focus gating and consume the shared cursor style |
| Shared styles | `src/ui/file_manager.rs:53-113` | Add origin-marker style and make cursor accent-filled, bold, and reversed |
| Visual oracle | `src/ui/visual_fixture.rs:614-725`, `tests/visual/files-locations.spec.ts` | Update VIS-26 and add VIS-27; inspect every broader cursor-style delta individually |
| Runtime/protocol/platform | `src/server/`, `src/protocol/`, `src/platform/` | No change authorized |

## Target Interfaces

`FileManagerLocationsState` remains the single top-level focus owner:

~~~rust
impl FileManagerLocationsState {
    pub(crate) fn focus_rail(&mut self) -> bool {
        let changed = self.focus != FileManagerLocationsFocus::Rail;
        self.focus = FileManagerLocationsFocus::Rail;
        changed
    }

    pub(crate) fn focus_trail(&mut self) -> bool {
        let pending_changed = self.pending.take().is_some();
        let focus_changed = self.focus != FileManagerLocationsFocus::Trail;
        self.focus = FileManagerLocationsFocus::Trail;
        pending_changed || focus_changed
    }
}
~~~

`focus_rail()` preserves an accepted FollowPreview pending request. `focus_trail()` retires incompatible Rail pending state. The input adapter also retires the App-level deferred request:

~~~rust
fn focus_file_manager_trail(state: &mut AppState) -> bool {
    let request_changed = state
        .request_file_manager_location_navigation
        .take()
        .is_some();
    let owner_changed = state.file_manager_locations.focus_trail();
    request_changed || owner_changed
}
~~~

The File Action Bar model takes explicit owner authority:

~~~rust
pub(crate) fn compute_file_manager_action_bar_model(
    file_manager: &FmState,
    focus_owner: FileManagerLocationsFocus,
    clipboard: &[std::path::PathBuf],
    operation_in_flight: bool,
) -> FileManagerActionBarModel;
~~~

The typed reason and precedence are:

~~~rust
pub enum FileManagerActionDisabledReason {
    InactiveFocusOwner,
    NoSelection,
    EmptyClipboard,
    ReadOnlyTarget,
    MultipleSelection,
    StaleSelection,
    UnsupportedSelection,
    UnsupportedAction,
    OperationInFlight,
}
~~~

1. Rail owner -> `InactiveFocusOwner` for all four actions.
2. Trail owner plus operation running -> `OperationInFlight`.
3. Trail owner -> existing selection, clipboard, writability, stale-kind, unsupported-kind, and unsupported-action rules.

Selection metadata may remain resident for a fast truthful return to Trail, but `enabled=false` at every Rail-owned boundary gives it zero operation authority.

The shared visual styles converge on:

~~~rust
cursor: Style::default()
    .fg(palette.accent)
    .bg(palette.panel_bg)
    .add_modifier(Modifier::BOLD | Modifier::REVERSED),
origin_marker: Style::default()
    .fg(palette.accent)
    .bg(palette.panel_bg)
    .add_modifier(Modifier::BOLD),
~~~

`cursor` wins over entry-kind and multi-selection paint. `origin_marker` never contains `REVERSED` or `UNDERLINED`.

## Test-Point Coverage Map

| Approved ID | Owning task |
|---|---|
| `TP-FFO-CHAR-01` | Task 0 |
| `TP-FFO-MOUSE-01` | Tasks 1-2 |
| `TP-FFO-MOUSE-02` | Tasks 1-2 |
| `TP-FFO-MOUSE-03` | Tasks 1-2 |
| `TP-FFO-MOUSE-04` | Tasks 1-2 |
| `TP-FFO-WHEEL-01` | Tasks 1-2 |
| `TP-FFO-WHEEL-02` | Tasks 1-2 and 8 |
| `TP-FFO-KEY-01` | Tasks 2 and 8 |
| `TP-FFO-ACTION-01` | Tasks 3-4 |
| `TP-FFO-ACTION-02` | Tasks 3-4 |
| `TP-FFO-ACTION-03` | Tasks 3-4 and 8 |
| `TP-FFO-VIS-01` | Tasks 5-7 |
| `TP-FFO-VIS-02` | Tasks 5-7 |
| `TP-FFO-VIS-03` | Tasks 5-6 |
| `TP-FFO-VIS-04` | Tasks 5-7 |
| `TP-FFO-LIFE-01` | Tasks 2 and 8 |
| `TP-FFO-ASYNC-01` | Tasks 2 and 8 |
| `TP-FFO-RENDER-01` | Tasks 1-2 and 8 |
| `TP-FFO-IO-01` | Tasks 1-2 and 8 |
| `TP-FFO-E2E-01` | Task 8 |
| `TP-FFO-GATE-01` | Task 8 |

## Standard Commands and Evidence Rules

~~~bash
export PATH="$HOME/.local/bin:$PATH"
~~~

Focused FFO tests:

~~~bash
cargo nextest run --locked -E 'test(/ffo_/)' --no-fail-fast \
  --status-level fail --final-status-level fail \
  --failure-output final --success-output never
~~~

Owner/input regressions:

~~~bash
cargo nextest run --locked \
  -E 'test(/(ffo_|flf_|fmn_|fmh_|vertical_wheel|plain_wheel|single_click|file_manager_key)/)' \
  --no-fail-fast --status-level fail --final-status-level fail \
  --failure-output final --success-output never
~~~

Action regressions:

~~~bash
cargo nextest run --locked \
  -E 'test(/(ffo_|action_bar|header_action|context_action|file_manager_context|paste|rename|plugin_action)/)' \
  --no-fail-fast --status-level fail --final-status-level fail \
  --failure-output final --success-output never
~~~

Visual semantic tests:

~~~bash
cargo nextest run --locked \
  -E 'test(/(ffo_|flf_render|cursor_style|multi_selection_rows|production_trail_render)/)' \
  --no-fail-fast --status-level fail --final-status-level fail \
  --failure-output final --success-output never
~~~

Canonical gate is `just check`. If `just` is absent, run every current child recipe explicitly:

~~~bash
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo nextest run --locked -E 'all()' --no-fail-fast \
  --status-level fail --final-status-level slow \
  --failure-output final --success-output never
bun test src/integration/assets/herdr-agent-state.test.ts
(cd workers/plugin-marketplace && bun test)
rustup target add x86_64-pc-windows-msvc
LIBGHOSTTY_VT_SIMD=false cargo clippy --bin herdr --locked \
  --target x86_64-pc-windows-msvc -- -D warnings
python3 -m unittest \
  scripts.test_agent_detection_manifest_check \
  scripts.test_changelog \
  scripts.test_docs_translation_parity \
  scripts.test_preview \
  scripts.test_trail_t7_teardown \
  scripts.test_vendor_libghostty_vt \
  scripts.test_vendor_portable_pty
~~~

Every command must exit 0. Record exact selected/passed/skipped counts and Nextest run IDs. A failed first run remains in evidence after correction.

---

## Task 0: Freeze Characterization and Commit This Plan

**Files:**

- Verify: `src/app/input/file_manager.rs`
- Verify: `src/ui/file_manager.rs`
- Verify: `docs/superpowers/specs/2026-07-22-herdr-files-focus-ownership-design.md`
- Modify: `docs/superpowers/plans/2026-07-22-herdr-files-focus-ownership-implementation.md`

- [ ] **Step 0.1: Re-prove the protected current behavior**

Run the four pre-existing characterization tests before changing Rust:

~~~bash
cargo nextest run --locked -E 'test(/(single_click_selects_current_row_and_refreshes_preview|flf_mouse_location_click_synchronizes_cursor_and_typed_intent|flf_render_rail_focus_suppresses_trail_cursor_style|flf_render_rail_cursor_wins_and_origin_remains_subdued)/)' --no-fail-fast --status-level fail --final-status-level fail --failure-output final --success-output never
~~~

Expected: `4 passed`, no retries, no skipped selected tests. This is `TP-FFO-CHAR-01`; it protects exact row activation, typed Rail intent, Trail-style suppression while Rail owns focus, and the deliberately weaker origin marker.

- [ ] **Step 0.2: Self-review the plan against the approved contract**

~~~bash
env -u RIPGREP_CONFIG_PATH rg -o 'TP-FFO-[A-Z0-9-]+' docs/superpowers/specs/2026-07-22-herdr-files-focus-ownership-design.md | sort -u > /tmp/herdr-ffo-spec-ids
env -u RIPGREP_CONFIG_PATH rg -o 'TP-FFO-[A-Z0-9-]+' docs/superpowers/plans/2026-07-22-herdr-files-focus-ownership-implementation.md | sort -u > /tmp/herdr-ffo-plan-ids
diff -u /tmp/herdr-ffo-spec-ids /tmp/herdr-ffo-plan-ids
git diff --check -- docs/superpowers/plans/2026-07-22-herdr-files-focus-ownership-implementation.md
~~~

Expected: no diff between ID sets and no whitespace error. Manually verify that every production edit is preceded by its RED task, every action consumer is enumerated, stable runtime paths are absent, and no task authorizes cache/protocol/platform work.

- [ ] **Step 0.3: Commit only the reviewed plan**

~~~bash
git status --short
git add -f -- docs/superpowers/plans/2026-07-22-herdr-files-focus-ownership-implementation.md
git diff --cached --name-only
git diff --cached --check
git commit -m "docs: plan file manager focus ownership"
~~~

Expected cached set: exactly the plan path. `.superpowers/` remains untracked and unstaged.

## Task 1: RED — Specify Accepted Trail Input Ownership

**Files:**

- Modify tests only: `src/app/input/file_manager.rs`
- Test: `src/app/input/file_manager.rs`

- [ ] **Step 1.1: Add deterministic Trail-click ownership tests**

Add `ffo_trail_click_transfers_focus_and_next_down_stays_in_clicked_column` beside the current Trail mouse tests. Build a `TempDir` containing `00.txt`, `01.txt`, and `02.txt`; equalize their mtimes with the existing `TempDir::set_equal_modified` helper. Open a runtime Trail, set `app.state.file_manager_locations.focus = Rail`, snapshot the Rail cursor, click the exact live `00.txt` Trail row, and assert:

~~~rust
assert_eq!(
    app.state.file_manager_locations.focus,
    FileManagerLocationsFocus::Trail
);
assert_eq!(
    handle_file_manager_key(&mut app.state, key(KeyCode::Down)),
    FileManagerKeyDispatch::Changed
);
assert_eq!(
    app.state.file_manager.as_ref().expect("open FM").selected()
        .map(|entry| entry.name.as_str()),
    Some("01.txt")
);
assert_eq!(app.state.file_manager_locations.cursor, rail_cursor_before);
~~~

This test must prove the exact reported bug: after the mouse chooses Trail, the next vertical key remains in that Trail column and does not mutate the Rail cursor.

Add `ffo_live_empty_trail_click_transfers_owner_without_mutating_trail`. Use an empty directory, compute a current frame with a live nonzero Trail body, set Rail focus, clone the `FmState`, click a body coordinate that is not a row/action/divider/header, then assert Trail ownership and exact `FmState` equality. This separates region focus from row activation and proves no synthetic selection or I/O.

- [ ] **Step 1.2: Add wheel ownership/render-law tests**

Add `ffo_clamped_trail_wheel_transfers_owner_and_requests_one_render`. Use one equalized file, Rail focus, and `ScrollUp` over its live Trail row while already clamped at index 0. Assert:

~~~rust
assert_eq!(app.state.file_manager_locations.focus, FileManagerLocationsFocus::Trail);
assert_eq!(app.state.file_manager.as_ref().expect("open FM").cursor, 0);
assert_eq!(app.file_manager_mouse_render_override, Some(true));
~~~

The first accepted wheel event changes ownership even when its cursor delta clamps, so one repaint is necessary. Then send a second human-scale accepted `ScrollUp` after the burst-gate interval while Trail already owns focus; assert cursor 0 and `Some(false)`. A coalesced duplicate must remain `Some(false)` and must not acquire authority by itself.

Add `ffo_stale_trail_frame_cannot_transfer_focus`. Capture a row from the current view, invalidate the Files generation or row identity by recomputing state without recomputing that view, dispatch the old coordinate, and assert Rail focus, unchanged selection, no navigation request, and no operation intent. This negative can pass before production code and remains a fail-closed regression; RED is established by the positive ownership assertions above.

- [ ] **Step 1.3: Extend existing accepted-input characterizations**

Without changing production code, add Rail-before/Trail-after assertions to the existing tests that already prove:

- modified live Trail row selection,
- live right click of a non-current Trail row,
- enabled inline row action,
- accepted horizontal Trail navigation.

Keep current stale, modified-invalid, disabled-header, divider, and outside-surface tests Rail-owned. Do not weaken their existing state/intent assertions.

- [ ] **Step 1.4: Run RED and record the assertion failures**

~~~bash
cargo nextest run --locked -E 'test(/ffo_/)' --no-fail-fast --status-level fail --final-status-level fail --failure-output final --success-output never
~~~

Expected RED: the new positive click/wheel ownership tests fail because focus remains `Rail`; the stale negative stays green. Compilation success plus assertion-level failures is required. Save exact failing test names and mismatch values in the eventual closure evidence.

- [ ] **Step 1.5: Commit the tests-only RED**

~~~bash
git add -- src/app/input/file_manager.rs
git diff --cached --name-only
git diff --cached --check
git commit -m "test: specify file manager focus ownership"
~~~

Expected cached set: exactly `src/app/input/file_manager.rs`. The branch may temporarily contain a RED commit locally, but it must not be pushed and Task 2 must immediately restore green.

## Task 2: GREEN — Centralize and Apply Trail Ownership Transitions

**Files:**

- Modify: `src/app/file_manager_locations.rs`
- Modify: `src/app/input/file_manager.rs`
- Test: `src/app/file_manager_locations.rs`
- Test: `src/app/input/file_manager.rs`

- [ ] **Step 2.1: Add named, observable owner transitions**

In `FileManagerLocationsState`, implement the approved `focus_rail() -> bool` and convert `focus_trail()` to `-> bool`. The return value means externally visible authority or pending-state changed, not merely that assignment ran.

Replace direct top-level focus writes in state-owned transitions where a named transition expresses the same law. Keep these semantic exceptions explicit:

- `activate_location` and `select_cursor` establish Rail authority after a validated location identity.
- `activate_direct` and closed-Files retirement establish Trail authority.
- `open_drawer`/`close_drawer` preserve their restore-owner contract.
- `scroll_rail` keeps Rail authority even if the scroll delta clamps; its boolean continues to report scroll change only unless its callers separately account for owner change.

Add unit tests `ffo_focus_helpers_report_only_real_changes` and `ffo_focus_trail_retires_pending_authority`. Prove idempotence, Rail pending preservation, Trail pending retirement, and truthful booleans.

- [ ] **Step 2.2: Add one input-boundary helper**

Immediately above the Files mouse reducers in `src/app/input/file_manager.rs`, add:

~~~rust
fn focus_file_manager_trail(state: &mut AppState) -> bool {
    let deferred_changed = state
        .request_file_manager_location_navigation
        .take()
        .is_some();
    state.file_manager_locations.focus_trail() || deferred_changed
}
~~~

If borrow ordering requires two local booleans, preserve both operations and OR after both execute; do not short-circuit the second cleanup. This helper is pure state mutation and performs no directory read or worker request.

- [ ] **Step 2.3: Transfer ownership only after a live typed Trail acceptance**

Update the exact current-frame paths in `App::handle_file_manager_mouse_at` and its helpers:

1. Plain or modified Trail row: validate generation/path/column/entry first; only after `activate_trail_entry` is not `Rejected`, call `focus_file_manager_trail`.
2. Right click: validate the exact row, apply the exact selection policy, then transfer before building the context model.
3. Inline row action: validate the projected and live exact target, transfer before writing its typed context intent.
4. Vertical wheel: first prove the coordinate belongs to a live Trail column and the burst gate accepted it; OR owner change with cursor change when choosing the render override.
5. Horizontal wheel/key transition already accepted by the Trail reducer: use the helper rather than a direct focus assignment.
6. Live empty/detail Trail body primary click: after higher-priority row/action/divider/header targets decline and the current Trail frame is live, transfer owner without selection mutation.

Never transfer from a stale generation, stale path, missing target, coalesced wheel packet, modified invalid event, resize drag, divider hit, Rail hit, disabled header action, overlay interception, or coordinate outside the live Trail body.

- [ ] **Step 2.4: Preserve lifecycle, async, render, and I/O laws**

Add or extend assertions showing:

- closing Files restores the existing default owner and leaves no pending/deferred location authority (`TP-FFO-LIFE-01`);
- a late location result cannot reclaim Rail focus after Trail retired that request (`TP-FFO-ASYNC-01`);
- ownership-only transition produces one render, idempotent repeated Trail input produces none (`TP-FFO-RENDER-01`);
- click/wheel ownership transition does not increment the test directory-read counter or submit a new I/O request (`TP-FFO-IO-01`).

Use existing operation counters/hooks. Do not introduce a new global counter solely for these tests.

- [ ] **Step 2.5: Run GREEN and adjacent regressions**

~~~bash
cargo nextest run --locked -E 'test(/ffo_/)' --no-fail-fast --status-level fail --final-status-level fail --failure-output final --success-output never
cargo nextest run --locked -E 'test(/(ffo_|flf_|fmn_|fmh_|vertical_wheel|plain_wheel|single_click|file_manager_key)/)' --no-fail-fast --status-level fail --final-status-level fail --failure-output final --success-output never
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
~~~

Expected: all selected tests pass with no retry; fmt and Linux clippy exit 0. Specifically, the old `clamped_vertical_wheel_declines_render` test must remain true when Trail already owns focus, while the new Rail-to-Trail clamped case renders exactly once.

- [ ] **Step 2.6: Commit the minimal GREEN**

~~~bash
git add -- src/app/file_manager_locations.rs src/app/input/file_manager.rs
git diff --cached --name-only
git diff --cached --check
git commit -m "fix: transfer file manager focus on trail input"
~~~

Expected cached set: exactly the two named Rust files.

## Task 3: RED — Specify Trail-Owned File Action Authority

**Files:**

- Modify tests only: `src/ui/file_manager.rs`
- Modify tests only: `src/app/input/file_manager.rs`
- Modify tests only: `src/app/file_operation_worker.rs`

- [ ] **Step 3.1: Specify model-level precedence without adding the enum yet**

Update existing action-model calls to pass `FileManagerLocationsFocus::Trail` as the explicit owner once the test compilation requires the future signature. Add `ffo_rail_owner_disables_every_file_action_with_owner_precedence` using a valid selected file, nonempty clipboard, writable cwd, and both `operation_in_flight = false` and `true` cases.

During the RED-only commit, avoid referencing a not-yet-created enum variant by asserting its debug representation:

~~~rust
for action in FileManagerHeaderAction::ALL {
    let state = model.action_state(action).expect("catalog action");
    assert!(!state.enabled);
    assert_eq!(format!("{:?}", state.disabled_reason), "Some(InactiveFocusOwner)");
}
~~~

Expected RED may initially be a compile failure because the model has no focus parameter. That is acceptable only after the test expresses the approved API and no production file has changed; once the signature exists in Task 4, the behavior assertion must also demonstrate the precedence.

- [ ] **Step 3.2: Specify stale prepared-header rejection**

Add `ffo_prepared_enabled_header_action_fails_closed_after_rail_takes_focus`:

1. Build current-frame Trail-owned geometry with Copy enabled for one explicit selection.
2. Switch current owner to Rail without recomputing the prepared view.
3. Click the old Copy rectangle.
4. Assert `Consumed`, Rail remains owner, clipboard unchanged, no context-action intent, no operation request, no selection change, and no filesystem mutation.

This proves prepared paint is not current dispatch authority (`TP-FFO-ACTION-02`).

- [ ] **Step 3.3: Specify direct dispatch fail-closed behavior**

In `src/app/file_operation_worker.rs`, add Rail-owned tests for `Copy`, `Paste`, and `Delete` through the existing direct header/context dispatch seams. Seed states that would otherwise be valid. Assert each returns false/declines, no clipboard or request changes, no worker starts, and source/destination directory listings remain byte-for-byte equivalent.

Extend one plugin or rename caller characterization only if its existing direct seam bypasses the shared model during compilation analysis; otherwise Task 4's callsite compilation plus focused regressions provide the fail-closed proof.

- [ ] **Step 3.4: Run and preserve RED evidence**

~~~bash
cargo nextest run --locked -E 'test(/ffo_/)' --no-fail-fast --status-level fail --final-status-level fail --failure-output final --success-output never
~~~

Expected RED: missing focus parameter/variant or action still enabled under Rail, plus stale header/direct dispatch side effects. Record the exact first failure class; do not weaken assertions to obtain green.

- [ ] **Step 3.5: Commit tests-only RED**

~~~bash
git add -- src/ui/file_manager.rs src/app/input/file_manager.rs src/app/file_operation_worker.rs
git diff --cached --name-only
git diff --cached --check
git commit -m "test: require trail-owned file actions"
~~~

Expected cached set: only these three test-containing source files. Do not push the RED head.

## Task 4: GREEN — Bind Every File Action Boundary to Current Trail Focus

**Files:**

- Modify: `src/app/state.rs`
- Modify: `src/ui.rs`
- Modify: `src/ui/file_manager.rs`
- Modify: `src/app/input/file_manager.rs`
- Modify: `src/app/input/modal.rs`
- Modify: `src/app/api/plugins/mod.rs`
- Modify: `src/app/file_operation_worker.rs`
- Modify: `src/app/file_rename.rs`
- Test: all of the above modules' existing action suites

- [ ] **Step 4.1: Add the typed disabled reason and owner parameter**

Add `FileManagerActionDisabledReason::InactiveFocusOwner` in `src/app/state.rs`. Extend `compute_file_manager_action_bar_model` with the explicit `focus_owner` argument. Compute every action's reason with this total precedence:

~~~rust
let disabled_reason = if focus_owner != FileManagerLocationsFocus::Trail {
    Some(FileManagerActionDisabledReason::InactiveFocusOwner)
} else if operation_in_flight {
    Some(FileManagerActionDisabledReason::OperationInFlight)
} else {
    // the existing per-action rules, unchanged
};
~~~

Do not erase resident selection/clipboard metadata; only operation authority becomes disabled. This lets the header remain truthful and re-enable without an artificial selection rebuild after Trail regains focus.

- [ ] **Step 4.2: Update every model producer with current owner**

Pass `state.file_manager_locations.focus` (or the equivalent borrowed current `AppState`) at every compiler-reported callsite:

- desktop/mobile `compute_view` in `src/ui.rs`,
- fallback/fixture render in `src/ui/file_manager.rs`,
- right-click and inline row preparation in `src/app/input/file_manager.rs`,
- modal/context revalidation in `src/app/input/modal.rs`,
- plugin action discovery/dispatch in `src/app/api/plugins/mod.rs`,
- copy/delete/paste/context execution in `src/app/file_operation_worker.rs`,
- rename preparation/dispatch in `src/app/file_rename.rs`.

Do not pass a constant `Trail` outside tests. Any caller that lacks access to current focus must gain a narrow explicit parameter rather than reading global state or duplicating the owner rule.

- [ ] **Step 4.3: Revalidate at action dispatch time**

For a header hit, require both:

1. the prepared frame contains the exact enabled action rectangle, and
2. the current `AppState` still has `FileManagerLocationsFocus::Trail` and current recomputation still enables that action.

Consume stale/disabled coordinates without starting an action. For right click and inline row actions, transfer Trail ownership only after exact target validation and before recomputing the model, so the initiating live Trail interaction can legitimately authorize the requested Trail-scoped action.

Direct header/context/plugin/rename entry points must independently decline under Rail even if called without mouse geometry. This prevents internal callers from bypassing the presentation gate (`TP-FFO-ACTION-03`).

- [ ] **Step 4.4: Update all existing action tests explicitly**

Every existing unit test that intentionally exercises action behavior must state its owner. Use `Trail` for legacy behavior expectations and add Rail variants only where authority is the subject. Do not hide the new argument inside a default helper that makes ownership invisible.

Assert the real enum variant in GREEN:

~~~rust
assert_eq!(
    state.disabled_reason,
    Some(FileManagerActionDisabledReason::InactiveFocusOwner)
);
~~~

Keep operation, no-selection, empty-clipboard, read-only, unsupported selection, unsupported action, stale identity, and multiple-selection precedence unchanged under Trail.

- [ ] **Step 4.5: Run focused and cross-surface GREEN gates**

~~~bash
cargo nextest run --locked -E 'test(/ffo_/)' --no-fail-fast --status-level fail --final-status-level fail --failure-output final --success-output never
cargo nextest run --locked -E 'test(/(ffo_|action_bar|header_action|context_action|file_manager_context|paste|rename|plugin_action)/)' --no-fail-fast --status-level fail --final-status-level fail --failure-output final --success-output never
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
~~~

Expected: all selected tests pass without retry; no filesystem mutation in any Rail-owned negative; all compiler-discovered action model callsites pass current focus.

- [ ] **Step 4.6: Commit the minimal GREEN**

~~~bash
git add -- src/app/state.rs src/ui.rs src/ui/file_manager.rs src/app/input/file_manager.rs src/app/input/modal.rs src/app/api/plugins/mod.rs src/app/file_operation_worker.rs src/app/file_rename.rs
git diff --cached --name-only
git diff --cached --check
git commit -m "fix: bind file actions to trail focus"
~~~

Expected cached set: exactly the eight named Rust files.

## Task 5: RED — Specify One Cross-Region Focus Visual Language

**Files:**

- Modify tests only: `src/ui/file_manager.rs`
- Test: `src/ui/file_manager.rs`

- [ ] **Step 5.1: Reject the underlined origin marker**

Change `flf_render_rail_cursor_wins_and_origin_remains_subdued` so the accepted origin is accent + bold, but contains neither `REVERSED` nor `UNDERLINED`. Keep the cursor assertion accent-filled + bold + reversed.

Update `flf_render_no_color_distinguishes_cursor_from_origin_by_modifiers` so monochrome users receive:

- active cursor: `BOLD | REVERSED`, no underline;
- origin context: `BOLD`, no reverse, no underline.

The modifier difference, not color alone, remains the no-color discriminator.

- [ ] **Step 5.2: Specify cross-region cursor equality and one-owner paint**

Add `ffo_rail_and_trail_active_rows_share_the_same_cursor_style`:

1. Render a wide current frame with a known Rail cursor and resident Trail selection while Rail owns focus.
2. Capture the Rail cursor cell's foreground, background, and modifiers; prove the resident Trail row is not reversed.
3. Switch owner to Trail without changing selection identities, recompute the view, and render again.
4. Capture the active Trail row; assert its foreground, background, and modifiers exactly equal the former Rail cursor's active-focus tuple.
5. Prove the Rail cursor is no longer reversed and at most the accepted origin receives the subdued origin-marker tuple.

This is semantic cell evidence; it must not infer focus from label text alone.

- [ ] **Step 5.3: Specify focused-row precedence over multi-selection**

Extend `multi_selection_rows_are_distinct_from_cursor_focus` or add `ffo_active_cursor_style_wins_over_multi_selection_membership`. Make the Trail-focused cursor also a member of explicit multi-selection and assert:

- model membership remains present;
- the painted cursor uses the active cursor tuple, not `multi_selection`;
- other selected members keep the multi-selection tuple;
- only one rendered row contains `REVERSED` active-focus authority.

- [ ] **Step 5.4: Run assertion-level RED**

~~~bash
cargo nextest run --locked -E 'test(/(ffo_|flf_render|multi_selection_rows)/)' --no-fail-fast --status-level fail --final-status-level fail --failure-output final --success-output never
~~~

Expected RED: old origin still contains underline and Trail cursor still uses the old surface/text tuple. Preserve exact expected/actual style evidence.

- [ ] **Step 5.5: Commit the visual-semantic RED**

~~~bash
git add -- src/ui/file_manager.rs
git diff --cached --name-only
git diff --cached --check
git commit -m "test: specify unified file manager focus styling"
~~~

Expected cached set: exactly `src/ui/file_manager.rs`. Do not push the RED head.

## Task 6: GREEN — Reuse One Active Cursor and One Subdued Origin Marker

**Files:**

- Modify: `src/ui/file_manager.rs`
- Modify: `src/ui/file_manager/locations.rs`
- Verify only unless compiler requires it: `src/ui/file_manager/trail_view.rs`

- [ ] **Step 6.1: Centralize the semantic styles**

Add `origin_marker: Style` to `FileManagerVisualStyles` and implement the approved style tuples in `file_manager_visual_styles`. Keep `multi_selection`, `directory`, `file`, warning, error, and operation status styles unchanged.

The active cursor style is:

~~~rust
Style::default()
    .fg(palette.accent)
    .bg(palette.panel_bg)
    .add_modifier(Modifier::BOLD | Modifier::REVERSED)
~~~

The accepted origin marker is:

~~~rust
Style::default()
    .fg(palette.accent)
    .bg(palette.panel_bg)
    .add_modifier(Modifier::BOLD)
~~~

- [ ] **Step 6.2: Make Rail consume the same styles as Trail**

In `render_file_manager_locations`, obtain `super::file_manager_visual_styles(&app.palette)` once and use:

- `styles.cursor` only for `rail_focused && cursor == item.path`;
- `styles.origin_marker` only for accepted origin that is not the focused cursor;
- existing accessible/inaccessible base styles otherwise.

Do not mutate focus or cursor during render. Do not add a second location-specific cursor definition. Trail already receives `styles.cursor` through `render_trail_view` and `render_entry_row_clipped`; edit `trail_view.rs` only if a test proves a real precedence leak.

- [ ] **Step 6.3: Preserve cursor-over-selection precedence**

Confirm `render_trail_view` and `render_entry_row_clipped` both choose `styles.cursor` before `styles.multi_selection`. If duplicate selection-style application is found in timestamps/actions, route those cells through the same already-computed `row_style`; do not change selection membership.

- [ ] **Step 6.4: Run semantic GREEN and renderer regressions**

~~~bash
cargo nextest run --locked -E 'test(/(ffo_|flf_render|cursor_style|multi_selection_rows|production_trail_render)/)' --no-fail-fast --status-level fail --final-status-level fail --failure-output final --success-output never
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
~~~

Expected: all selected semantic tests pass; exactly one active owner row is reversed in each fixture; no render-time mutation or I/O test regresses.

- [ ] **Step 6.5: Commit the minimal style GREEN**

~~~bash
git add -- src/ui/file_manager.rs src/ui/file_manager/locations.rs
git diff --cached --name-only
git diff --cached --check
git commit -m "fix: unify file manager focus cursor styling"
~~~

If `src/ui/file_manager/trail_view.rs` genuinely required a production correction, stop and add it to the proposed cached set with an evidence note before staging. Otherwise it remains untouched.

## Task 7: Deterministic Visual Oracle and Reviewed Snapshots

**Files:**

- Modify: `src/ui/visual_fixture.rs`
- Modify: `tests/visual/files-locations.spec.ts`
- Generated/ignored: `tests/visual/fixtures/generated/vis-26-files-locations-follow-focus.json`
- Generated/ignored: `tests/visual/fixtures/generated/vis-27-files-locations-trail-focus.json`
- Modify only after inspection: `tests/visual/files-locations.spec.ts-snapshots/vis-26-files-locations-follow-focus-linux.png`
- Add only after inspection: `tests/visual/files-locations.spec.ts-snapshots/vis-27-files-locations-trail-focus-linux.png`

- [ ] **Step 7.1: Extend the focused exporter without touching VIS-01..13**

Rename the targeted exporter/test only if a truthful name improves clarity; it must continue to export VIS-26 and additionally export VIS-27 from the same deterministic filesystem fixture.

- VIS-26: Rail owns focus, Downloads is the filled active cursor, Home is the bold non-underlined accepted origin, resident Trail row is inactive.
- VIS-27: Trail owns focus, the active Trail row is filled with the exact shared cursor style, no Rail row is reversed, and the accepted Home origin is subdued.

Assert the semantic cells inside Rust before writing JSON. Keep fixed mtimes/calendar anchors and test-owned `/tmp/herdr-vis26-flf-root` teardown. Ordinary test runs must never write fixtures.

- [ ] **Step 7.2: Add the VIS-27 Playwright contract**

Append the exact `vis-27-files-locations-trail-focus` case to `tests/visual/files-locations.spec.ts` with width 96 and height 24. Update the range comment to VIS-18..27. Do not change the harness, CSS, device, locale, timezone, scale, retry policy, or `updateSnapshots: "none"`.

- [ ] **Step 7.3: Prove exporter determinism twice before replacing anything**

~~~bash
FFO_VIS_A="$(mktemp -d /tmp/herdr-ffo-vis-a.XXXXXX)"
FFO_VIS_B="$(mktemp -d /tmp/herdr-ffo-vis-b.XXXXXX)"
HERDR_VISUAL_FIXTURE_DIR="$FFO_VIS_A" cargo nextest run --locked --run-ignored only -E 'test(/write_locations_follow_visual_fixture/)' --status-level fail --final-status-level fail --failure-output final --success-output never
HERDR_VISUAL_FIXTURE_DIR="$FFO_VIS_B" cargo nextest run --locked --run-ignored only -E 'test(/write_locations_follow_visual_fixture/)' --status-level fail --final-status-level fail --failure-output final --success-output never
diff -ru -- "$FFO_VIS_A" "$FFO_VIS_B"
~~~

Expected: both targeted exporter runs pass and recursive diff is empty. If output differs, stop snapshot work and diagnose the nondeterministic source; never update PNGs from unstable fixtures.

Copy only the two proven-identical JSON files into the ignored generated fixture directory. Validate each temp root matches `/tmp/herdr-ffo-vis-a.*` or `/tmp/herdr-ffo-vis-b.*` before removing it; never use a broad `/tmp` target.

- [ ] **Step 7.4: Run Playwright with updates disabled and inspect failures**

~~~bash
cd tests/visual
npx --no-install playwright test files-locations.spec.ts --grep 'vis-2[67]-files-locations'
~~~

Expected first result: VIS-26 mismatch and VIS-27 missing snapshot are acceptable only if their exact actual images express the approved style. Inspect each baseline (when present), `*-actual.png`, and `*-diff.png` with the image viewer. Check:

- one and only one filled active cursor;
- Rail/Trail owner swaps truthfully;
- no origin underline;
- no clipping, label shift, timestamp shift, icon change, unexpected row reorder, or unrelated palette delta.

Any unrelated pixel is a test failure to diagnose, not an approval to regenerate.

- [ ] **Step 7.5: Update exact reviewed snapshots one at a time**

Only after inspection, run an exact anchored grep for VIS-26, inspect the written PNG, then repeat for VIS-27. Do not run a suite-wide `--update-snapshots` command.

~~~bash
npx --no-install playwright test files-locations.spec.ts --grep '^vis-26-files-locations-follow-focus:' --update-snapshots
npx --no-install playwright test files-locations.spec.ts --grep '^vis-27-files-locations-trail-focus:' --update-snapshots
npx --no-install playwright test files-locations.spec.ts
~~~

Expected final focused suite: 10/10 after adding VIS-27, zero retry, zero diff. Return to the repository root before Git operations.

- [ ] **Step 7.6: Commit only visual source and reviewed PNGs**

~~~bash
cd /home/ayaz/projects/herdr
git status --short
git add -- src/ui/visual_fixture.rs tests/visual/files-locations.spec.ts tests/visual/files-locations.spec.ts-snapshots/vis-26-files-locations-follow-focus-linux.png tests/visual/files-locations.spec.ts-snapshots/vis-27-files-locations-trail-focus-linux.png
git diff --cached --name-only
git diff --cached --check
git commit -m "test: cover file manager focus ownership visuals"
~~~

Expected cached set: exactly four paths. Ignored generated JSON and Playwright results are never staged; `.superpowers/` remains untouched.

## Task 8: Full Gates, Isolated Acceptance, Knowledge Capture, and Fork Publication

**Files:**

- Add: `.codex/evidence/files-focus-ownership-closure.md`
- Modify: `.codex/CURRENT.md`
- Modify: `.codex/TASKS.md`
- Modify: `.codex/HANDOFF.md`
- Modify if active: `.planning/STATE.md`
- Modify when a genuinely reusable rule exists: `docs/patterns/rust-engineering.md`
- Modify: `docs/references/README.md`
- Modify lessons as evidence warrants: `.codex/skills/herdr-native-fm/lessons/errors.md`
- Modify lessons as evidence warrants: `.codex/skills/herdr-native-fm/lessons/golden-paths.md`
- Modify lessons as evidence warrants: `.codex/skills/herdr-native-fm/lessons/edge-cases.md`
- Local-only verification: `.local/ISOLATED-DEV-TEST.md`, `.local/herdr-trail-test.sh`
- External knowledge state: Codebase Memory project `home-ayaz-projects-herdr`

- [ ] **Step 8.1: Run the complete automated gate before documentation claims**

Run `just check`. If and only if `just` is unavailable, run every exact child command listed in Standard Commands. Then run the full visual suite with snapshot updates disabled:

~~~bash
cd tests/visual
npx --no-install playwright test
~~~

Expected: every Rust, integration, maintenance, Linux clippy, Windows clippy, formatting, and visual test passes; record exact totals, skips, run IDs, durations, and any corrected first failure. A green focused filter cannot substitute for the full gate (`TP-FFO-GATE-01`).

- [ ] **Step 8.2: Prove architecture boundaries mechanically**

~~~bash
git diff b4ac62a0..HEAD -- src/server src/protocol src/platform Cargo.toml Cargo.lock
env -u RIPGREP_CONFIG_PATH rg -n "read_dir|metadata\(|spawn|channel|sleep|debounce|cache" src/app/file_manager_locations.rs src/app/input/file_manager.rs src/ui/file_manager.rs src/ui/file_manager/locations.rs
~~~

Expected first command: no FFO changes in server/protocol/platform/dependencies. Review the second command's pre-existing hits; the FFO diff must add no filesystem enumeration, async worker, cache, timer, or blocking path. Use source diff plus existing test counters as `TP-FFO-IO-01`, not timing alone.

- [ ] **Step 8.3: Build and run only the isolated debug runtime**

~~~bash
export PATH="$HOME/.local/bin:$PATH"
cargo build --locked
env -u HERDR_SOCKET_PATH \
    -u HERDR_CLIENT_SOCKET_PATH \
    -u HERDR_PANE_ID \
    -u HERDR_WORKSPACE_ID \
    -u HERDR_TAB_ID \
    -u HERDR_ENV \
    HERDR_RENDER_PROF=1 \
    ./.local/herdr-trail-test.sh run
~~~

Before recommending the command, re-read `.local/herdr-trail-test.sh` and prove it targets only its exact throwaway `/tmp` XDG root, performs cleanup-first, traps normal exit/error/Ctrl-C, and never resolves the stable socket/config. Do not terminate any existing user process. If interactive acceptance must be user-driven, provide this as one copy/paste block after automated gates.

Manual `TP-FFO-E2E-01` acceptance matrix:

1. Click Locations Rail, then press Up/Down: only Rail cursor moves one row.
2. Click an ordinary Trail row, then press Up/Down: only that active Trail column moves one row.
3. From Rail, perform one slow wheel detent over Trail: Trail owns focus and advances at most one row; an upward detent at the top transfers focus without skipping.
4. Right/Left continue the approved Miller column laws; Right immediately highlights the first row of an enterable child, Left returns to the prior/ancestor column including the formerly “frozen” root column.
5. Rail-owned header Copy/Paste/New Folder/Delete look disabled and clicking them is inert.
6. Trail click restores eligible action state for the exact current Trail selection.
7. Exactly one active row has the filled accent focus style; the accepted Rail origin has no underline.
8. Dense click/wheel movement remains smooth; `render.prof` shows no inert render storm and no synchronous directory-read burst.
9. Exit normally and verify the throwaway test root/socket are absent.

- [ ] **Step 8.4: Write evidence before conclusions**

Create `.codex/evidence/files-focus-ownership-closure.md` with:

- commit SHAs and exact file sets;
- root cause and rejected hypotheses;
- RED test names plus assertion failures;
- GREEN/focused/full gate commands and exact totals;
- visual determinism hashes/diff result and inspected PNG list;
- isolated runtime root/socket/log path and manual acceptance results;
- performance-neutrality evidence;
- unchanged server/protocol/platform/dependency statement with command evidence;
- known exclusions: pinned cache, unrelated global focus architecture, drag-and-drop, hover focus.

Every conclusion must be `claim + evidence + confidence`. Mark manual acceptance pending rather than claiming it if the user has not yet run the build.

- [ ] **Step 8.5: Capture reusable engineering and Yazi-transfer lessons**

Update project docs only where new evidence exists:

- `docs/patterns/rust-engineering.md`: add the reusable nested-authority law — prepared data extent, selected identity, active child column, top-level region focus, visual projection, and destructive action authority are distinct; the last accepted live intent may transfer ownership, stale paint/data may not.
- `docs/references/README.md`: register the FFO closure evidence and the existing Yazi transfer study with tier/confidence; do not invent a Yazi feature that was not source-verified.
- `golden-paths.md`: add the successful workflow “characterize owner seams → RED accepted/stale input → central transition → revalidate destructive boundaries → semantic render tests → deterministic visual oracle”.
- `edge-cases.md`: record clamped accepted input that changes only ownership, coalesced input that cannot change ownership, and prepared-enabled geometry that becomes unauthorized before dispatch.
- `errors.md`: add only new encountered error + cause + verified fix in the required table-row format. Do not duplicate an existing known error.

If a lesson is broadly Rust-specific rather than Herdr-specific, propagate a compact table row to the active `rust-dev` lesson file per its skill protocol; otherwise keep it project-local.

- [ ] **Step 8.6: Refresh continuity and Codebase Memory truth**

Update `.codex/CURRENT.md`, `.codex/TASKS.md`, `.codex/HANDOFF.md`, and active `.planning/STATE.md` from live Git/test evidence. Remove stale “FMH at 787bb96b” claims rather than layering contradictory status. Keep future pinned-cache work explicitly separate.

For Codebase Memory:

1. call `manage_adr(mode="get", project="home-ayaz-projects-herdr")` and retain the result before any reindex;
2. call `detect_changes` against the pre-FFO anchor, but distrust zero changes;
3. prove freshness by resolving at least `focus_file_manager_trail` and `InactiveFocusOwner` through `search_graph`/`get_code_snippet`;
4. if stale, reindex in the proven single-worker mode only after preserving ADR content, then restore/update it;
5. call `manage_adr(mode="update", ...)` with the approved focus decision, alternatives, consequences, action-authority rule, test evidence, and final SHAs;
6. rerun `index_status`, `get_architecture`, `trace_path`, and exact symbol queries and record node/edge counts and freshness evidence.

Do not claim MCP-backed architecture state unless these real calls succeed.

- [ ] **Step 8.7: Commit closure documentation after alignment**

Review actual changed paths, then propose any necessary adjustment to this intended message/set before staging:

~~~bash
git add -- .codex/evidence/files-focus-ownership-closure.md .codex/CURRENT.md .codex/TASKS.md .codex/HANDOFF.md docs/patterns/rust-engineering.md docs/references/README.md .codex/skills/herdr-native-fm/lessons/errors.md .codex/skills/herdr-native-fm/lessons/golden-paths.md .codex/skills/herdr-native-fm/lessons/edge-cases.md
git diff --cached --name-only
git diff --cached --check
git commit -m "docs: record file manager focus ownership"
~~~

Stage `.planning/STATE.md` only if it is tracked and actually updated. Omit unchanged lesson/doc paths from the exact `git add` list. Never stage `.superpowers/`, ignored generated fixtures, `.local/`, or Playwright results.

- [ ] **Step 8.8: Re-run post-documentation gate and publish only to the fork**

Because documentation maintenance checks can fail after Task 8 edits, run `just check` again (or every exact child), plus the full Playwright suite with updates disabled. Then:

~~~bash
git status --short --branch
git log --oneline --decorate -12
git push origin HEAD:feat/native-fm
git rev-parse HEAD
git rev-parse origin/feat/native-fm
~~~

Expected: local and origin SHAs match; no tracked changes remain; only user-owned `.superpowers/` may remain untracked. Never push upstream and never force push.

---

## Commit Matrix and Rollback Anchors

| Concern | Planned commit | Required predecessor state |
|---|---|---|
| Approved design | `bf9fcf46 docs: define file manager focus ownership` | Existing rollback anchor |
| Executable plan | `docs: plan file manager focus ownership` | Characterization 4/4 |
| Input RED | `test: specify file manager focus ownership` | Assertion-level RED, local only |
| Input GREEN | `fix: transfer file manager focus on trail input` | FFO/input green |
| Action RED | `test: require trail-owned file actions` | Assertion/compile RED, local only |
| Action GREEN | `fix: bind file actions to trail focus` | FFO/action green |
| Visual RED | `test: specify unified file manager focus styling` | Assertion-level RED, local only |
| Visual GREEN | `fix: unify file manager focus cursor styling` | Semantic visual green |
| Visual oracle | `test: cover file manager focus ownership visuals` | Twice-deterministic fixtures + inspected exact PNGs |
| Closure | `docs: record file manager focus ownership` | Full gates + evidence + continuity + ADR |

Each GREEN is an ordinary forward commit; never rewrite published predecessor history. The safe rollback unit is the corresponding GREEN/visual commit, while the RED test preserves the rejected behavior contract. No RED head may be pushed.

## Plan Self-Review Checklist

- [ ] All 21 approved `TP-FFO-*` IDs appear in both spec and plan with no orphan or duplicate semantic owner.
- [ ] Positive, stale, clamped, coalesced, overlay/lifecycle, async, destructive-action, render, no-I/O, accessibility, and visual paths are covered.
- [ ] Every production task follows an observed RED task.
- [ ] All action-model producers and direct dispatch consumers are enumerated.
- [ ] Stable Herdr, stable XDG paths, user sessions, `.superpowers/`, upstream, and broad staging are excluded.
- [ ] No dependency, protocol, server, platform, cache, worker, timer, or global focus manager is authorized.
- [ ] Visual updates are targeted, deterministic, update-disabled first, and human/image inspected.
- [ ] Full Linux + Windows lint, complete Nextest, maintenance, integration, visual, isolated runtime, docs, lessons, continuity, graph, ADR, and fork SHA evidence have explicit closure steps.
