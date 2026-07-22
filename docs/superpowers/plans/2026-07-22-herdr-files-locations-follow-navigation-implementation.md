# Herdr Files Locations Follow Navigation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the Locations Rail and Miller Trail obey one predictable Follow-first keyboard law: vertical input moves exactly one actionable row in the current owner, asynchronous preparation never steals focus, and Right/Enter transfers exactly one level while immediately highlighting the destination's first real entry.

**Architecture:** Keep the change entirely client-local. Add a Rail cursor distinct from accepted origin and pending root work; route Rail keys before Trail keys; carry FollowPreview versus EnterTrail as typed local intent through the existing one-running-plus-one-latest file-manager worker; reject results against Files generation, model revision, worker ticket, current Rail cursor, focus lifecycle, and exact path; initialize entered roots/children through one pure resident-snapshot helper; render cursor, origin, pending, failure, and Trail ownership from immutable AppState.

**Tech Stack:** Rust, ratatui, crossterm, tokio Notify, cargo-nextest, the existing render profiler, deterministic Ratatui TestBackend fixtures, Playwright Chromium, Bash isolation helpers, and the existing just recipes.

**Product contract:** docs/superpowers/specs/2026-07-22-herdr-files-locations-follow-navigation-design.md at commit 3c099ea4.

## Global Constraints

- Never connect to, stop, reinstall, update, signal, or otherwise touch the stable Herdr process, stable socket, or ~/.config/herdr. Every runtime check uses throwaway XDG roots and clears inherited HERDR_SOCKET_PATH, HERDR_CLIENT_SOCKET_PATH, HERDR_PANE_ID, HERDR_WORKSPACE_ID, HERDR_TAB_ID, and HERDR_ENV.
- Preserve the verified FMH/A2.2 predecessor diff. Before the first FLF Rust edit, publish or checkpoint that predecessor as its own already-reviewed concern; never stage predecessor and FLF paths together.
- Never touch or stage the user-owned untracked .superpowers/ directory.
- Work only on feat/native-fm and publish only to the CyPack fork with git push origin HEAD:feat/native-fm. Never push upstream.
- Use exact-path staging. git add -A and broad globs are forbidden.
- Propose every commit message and wait for alignment before committing. RED and GREEN may be separate local commits, but never push a RED head.
- Use cargo nextest, not cargo test, for automated Rust evidence. A compilation error is only wiring feedback: add the smallest test-only stub necessary, then capture an assertion-level RED before production behavior.
- Re-read every modified source immediately before editing it. Re-check Codebase Memory symbols after each structural phase because ready or detect_changes=0 alone is not freshness evidence.
- compute_view may normalize geometry-owned state; render(&AppState) remains immutable and performs no filesystem access, request submission, cursor correction, or focus transfer.
- Add no dependency, server/protocol field, concurrent second worker lane, cache, LRU, timer, debounce scheduler, recursive enumeration, or manual-mode setting. Sequentially replacing a confirmed-dead lane is lifecycle recovery, not a second lane.
- Do not regenerate existing Playwright PNGs. Add and inspect only the new FLF fixture/snapshot; exporter mtime drift makes bulk regeneration invalid evidence.
- Pinned Home/Desktop/Downloads pre-warming remains FMN-6 and is not part of this plan.

---

## Verified Current-Owner Map

Graph evidence on 2026-07-22 resolved 24,078 nodes and 129,027 edges for home-ayaz-projects-herdr.

| Seam | Current owner | Planned change |
|---|---|---|
| Locations model | src/app/file_manager_locations_model.rs:1-196 | expose bounded accessible ordering, pure content-line identity, and exact path-to-line lookup |
| Rail authority | src/app/file_manager_locations.rs:12-196 | add cursor; keep cursor, accepted origin, pending identity, failure, focus, and drawer lifecycle separate |
| App request | src/app/state.rs:2147-2155 | replace Option<PathBuf> with one typed path plus FollowPreview/EnterTrail intent |
| Key reducer | src/app/input/file_manager.rs:138-290 | owner-first Rail reducer; root Left transfers to Rail; Trail keys cannot mutate while Rail owns focus |
| Mouse reducer | src/app/input/file_manager.rs:730-876 | synchronize Rail cursor; wide click follows; compact explicit activation stays lifecycle-safe |
| Key dispatch bridge | src/app/input/mod.rs:169-212 | suppress immediate render for deferred activation while preserving renders for visible cursor/focus changes |
| Scheduled order | src/app/runtime.rs:198-210 | retain result-before-request order and explicitly defend the old-result/new-cursor race |
| Root I/O | src/app/file_manager_io_worker.rs:602-868 | typed intent, pending promotion, exact cursor/focus guard, panic reuse, one-shot disconnect recovery, profiler counters |
| Miller request | src/app/file_manager_miller.rs:20-84 | distinguish keyboard first-entry activation from mouse-preserving activation |
| First-entry state | src/fm/trail.rs:51-146, src/fm/trail_snapshots.rs:420-510, src/fm/mod.rs:995-1069 | clear hidden destination history and move to first real entry without disk I/O |
| Locations render | src/ui/file_manager/locations.rs:110-420 | strong Rail cursor plus subdued accepted-origin marker with unchanged geometry |
| Trail render | src/ui/file_manager/trail_view.rs:654-729 | paint active row only while Trail owns keyboard focus |
| Compact geometry | src/ui.rs:245-255 | retire deferred activation when resize invalidates the drawer |
| Visual oracle | src/ui/visual_fixture.rs:228-620, tests/visual/files-locations.spec.ts | add one deterministic FLF-only fixture and PNG |
| Runtime harness | .local/herdr-files-v1-profile.sh, .local/herdr-trail-test.sh | reuse cleanup-first isolated execution; never target a stable socket |

## Concrete Type and Method Contract

The implementation should converge on these signatures. Names may move only when a compile-checked existing owner makes the alternative materially smaller; semantic fields must not be collapsed.

~~~rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileManagerLocationNavigationIntent {
    FollowPreview,
    EnterTrail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileManagerLocationNavigationRequest {
    pub(crate) path: PathBuf,
    pub(crate) intent: FileManagerLocationNavigationIntent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileManagerLocationPending {
    pub(crate) path: PathBuf,
    pub(crate) files_generation: u32,
    pub(crate) model_revision: u64,
    pub(crate) io_generation: u64,
    pub(crate) intent: FileManagerLocationNavigationIntent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileManagerLocationFailure {
    pub(crate) path: PathBuf,
    pub(crate) files_generation: u32,
    pub(crate) model_revision: u64,
    pub(crate) error: FileManagerLocationLoadError,
}
~~~

FileManagerLocationsState gains cursor: Option<PathBuf>. Its pure API must include:

~~~rust
pub(crate) enum FileManagerLocationCursorMove {
    Inert,
    Moved(PathBuf),
}

pub(crate) fn cursor_path<'a>(
    &'a self,
    model: &FileManagerLocationsModel,
) -> Option<&'a Path>;

pub(crate) fn normalize_cursor_for_rail(
    &mut self,
    model: &FileManagerLocationsModel,
) -> bool;

pub(crate) fn move_cursor(
    &mut self,
    model: &FileManagerLocationsModel,
    delta: isize,
) -> FileManagerLocationCursorMove;

pub(crate) fn ensure_cursor_visible(
    &mut self,
    model: &FileManagerLocationsModel,
    viewport_height: u16,
) -> bool;

pub(crate) fn retire_navigation_authority(&mut self);
~~~

Normalization rules are exact:

1. Location(path) plus an accessible exact item chooses path.
2. Direct(path) never infers path or an ancestor; a still-accessible prior cursor survives.
3. Otherwise choose the first accessible item.
4. No accessible item produces cursor=None while Rail focus remains valid.

The model stays bounded at 256 items and exposes allocation-free iteration:

~~~rust
pub(crate) fn accessible_items(
    &self,
) -> impl DoubleEndedIterator<Item = &FileManagerLocationItem> {
    self.sections
        .iter()
        .flat_map(|section| section.items.iter())
        .filter(|item| item.accessible)
}

pub(crate) fn content_line_count(&self) -> usize;
pub(crate) fn line_index_for_path(&self, path: &Path) -> Option<usize>;
~~~

content_line_count and line_index_for_path must use the same section-header and inter-section blank-line law as the renderer. Add a parity test so input auto-scroll and painted rows cannot drift.

The destination helper is disk-free and clears hidden history before selecting entry zero:

~~~rust
pub(crate) fn focus_first_active_trail_entry(
    &mut self,
) -> trail_snapshots::TrailCursorMoveOutcome {
    let owner_col = self.trail.active_col();
    if !self
        .trail_snapshots
        .reset_active_column_cursor(&mut self.trail)
    {
        return trail_snapshots::TrailCursorMoveOutcome::Rejected;
    }
    self.move_trail_cursor_in_column(owner_col, 1)
}
~~~

reset_active_column_cursor returns bool, truncates TrailState and TrailSnapshots after the active column, clears that column's selection/cursor and detail, and performs no read. An empty destination remains cursor=None; the caller still keeps the legitimate focus transfer.

Keyboard Trail activation carries a typed destination policy; mouse activation retains its existing selected-row behavior:

~~~rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FileManagerTrailDestinationPolicy {
    PreserveMouseSelection,
    FocusFirstActionable,
}
~~~

Root completion acceptance is conjunctive, never path-only:

~~~rust
let root_is_current = pending.path == target_root
    && pending.files_generation == files_generation
    && pending.model_revision == model_revision
    && pending.io_generation == result.generation
    && locations.cursor_path(model) == Some(target_root.as_path())
    && locations.focus == FileManagerLocationsFocus::Rail;
~~~

Closing Files, closing an invalid compact drawer, moving the Rail cursor away, changing the model, or otherwise leaving the Rail lifecycle clears pending/request authority before a same-path A-to-B-to-A sequence can revive an old EnterTrail completion.

Processor panic and worker disconnect are distinct lifecycle cases. `catch_unwind` converts a processor panic into `FileManagerIoOutcome::Panicked`, so that request fails but the existing thread remains reusable. A true disconnect fails only the exact current pending identity, discards any result co-returned from that dead lifecycle as stale, logs once, and replaces the dead worker with `FileManagerIoWorker::new(self.render_notify.clone())`; it never replays the failed request. The next explicit user request uses the fresh lane. Both the result-drain path and the submit-time disconnect race call the same replacement helper, and repeated scheduled drains must not create a restart loop.

## Test-Point Coverage Map

| Approved ID | Owning task |
|---|---|
| TP-FLF-CHAR-01 | Task 0 |
| TP-FLF-FOCUS-01 | Tasks 1-2 |
| TP-FLF-STEP-01 | Tasks 1-2 |
| TP-FLF-PREVIEW-01 | Tasks 2-3 |
| TP-FLF-NO-HIGHLIGHT-01 | Task 5 |
| TP-FLF-ENTER-01 | Tasks 3-4 |
| TP-FLF-CHILD-01 | Task 4 |
| TP-FLF-SECOND-01 | Task 4 |
| TP-FLF-BOUNDED-01 | Tasks 3 and 6 |
| TP-FLF-LATEST-01 | Task 3 |
| TP-FLF-BLOCKED-01 | Tasks 3 and 6 |
| TP-FLF-RESIDENT-01 | Tasks 3-4 |
| TP-FLF-FAIL-01 | Task 3 |
| TP-FLF-PANIC-01 | Tasks 3 and 6 |
| TP-FLF-STALE-01 | Tasks 2-3 |
| TP-FLF-EMPTY-01 | Tasks 3-4 |
| TP-FLF-RENDER-01 | Tasks 2, 3, and 5 |
| TP-FLF-COMPACT-01 | Tasks 2, 3, and 5 |
| TP-FLF-VIS-01 | Task 5 |
| TP-FLF-E2E-01 | Task 7 |

## Standard Gate Commands

Set the vendored Zig path before commands that build libghostty-vt:

~~~bash
export PATH="$HOME/.local/bin:$PATH"
~~~

Focused RED/GREEN:

~~~bash
cargo nextest run --locked -E 'test(/flf_/)' --no-fail-fast --status-level fail --final-status-level fail --failure-output final --success-output never
~~~

Full project gate:

~~~bash
just check
~~~

Targeted visual gate:

~~~bash
cd tests/visual
npx playwright test files-locations.spec.ts --grep 'vis-26-files-locations-follow-focus'
~~~

Explicit release-profile calibration:

~~~bash
cargo nextest run --release --locked --run-ignored only -E 'test(flf_scale_locations_follow_navigation)' --status-level all --final-status-level slow --failure-output immediate-final --success-output immediate
~~~

A filter that runs zero tests is a failure of the verification procedure. Record exact passed/skipped counts and nextest run IDs.

---

### Task 0: Separate the predecessor and freeze characterization

**Files:**
- Read only: src/app/input/file_manager.rs
- Read only: .codex/CURRENT.md, .codex/TASKS.md, .codex/HANDOFF.md, .planning/STATE.md
- Create after the tree is separated: .codex/evidence/files-locations-follow-baseline.md

- [ ] **Step 1: Verify branch, remotes, checkpoint refs, and exact dirty ownership**

~~~bash
git status --short --branch
git log --oneline --decorate -8
git remote -v
git for-each-ref --format='%(refname:short) %(objectname:short) %(subject)' refs/checkpoints/
git diff --check
~~~

Expected: feat/native-fm, origin is CyPack, upstream is ogulcancelik, HEAD contains design commit 3c099ea4, and the known FMH/A2.2 paths are still the only tracked dirty predecessor. .superpowers/ remains untracked.

- [ ] **Step 2: Finish the predecessor as a separate concern**

Run its already-defined focused and full gates, propose its existing commit message(s), exact-stage only its paths, commit, and push to origin. If the predecessor differs from its handoff or any test fails, stop FLF work and resolve that mismatch first. Do not invent a new combined commit.

- [ ] **Step 3: Create the FLF rollback ref**

~~~bash
git update-ref refs/checkpoints/herdr-flf-pre-implementation-20260722 HEAD
git show --no-patch --oneline refs/checkpoints/herdr-flf-pre-implementation-20260722
~~~

Expected: the checkpoint resolves to the clean post-predecessor HEAD. The only remaining untracked path may be .superpowers/.

- [ ] **Step 4: Re-verify graph freshness by symbols, not status alone**

Run index_status, search_graph, trace_path, and get_code_snippet for:

- handle_file_manager_key
- sync_file_manager_location_request
- sync_file_manager_io_results
- render_file_manager_locations
- render_trail_view
- focus_first_active_trail_entry after it exists

If current source does not resolve, first save manage_adr get output, then perform a single-worker reindex. Record node/edge totals.

- [ ] **Step 5: Run the existing interaction characterization**

~~~bash
cargo nextest run --locked -E 'test(/keyboard_directory_cursor_schedules_bounded_preview_without_focus_transfer|directory_preview_after_horizontal_focus_change_is_rejected|missing_directory_preview_preserves_cursor_and_resident_branch|fcl_io_worker_keeps_only_latest_pending_request|fcl_io_location_request_is_async_and_generation_safe|fcl_io_resident_root_activation_performs_zero_worker_reads/)' --no-fail-fast --status-level fail --final-status-level fail --failure-output final --success-output never
~~~

Expected: 6 selected tests pass. This is the movement-versus-preview law being extended; do not edit until it is green.

- [ ] **Step 6: Record baseline evidence**

Write .codex/evidence/files-locations-follow-baseline.md with HEAD, graph totals, command, run ID, exact count, stable-runtime-touch=false, and the dirty-tree boundary. Propose and align:

~~~text
docs: freeze locations follow baseline
~~~

Then exact-stage and commit only that file.

---

### Task 1: Add pure Rail cursor and ordering authority

**Files:**
- Modify: src/app/file_manager_locations_model.rs:50-196
- Modify: src/app/file_manager_locations.rs:12-196 and adjacent unit tests

- [ ] **Step 1: Write assertion-level RED tests**

Add these tests beside the pure state:

- flf_cursor_normalizes_exact_location_without_inferred_direct_ancestor
- flf_cursor_steps_accessible_items_one_at_a_time_and_clamps
- flf_cursor_reconcile_retires_obsolete_pending_and_failure
- flf_cursor_scroll_reveals_exact_model_line
- flf_model_line_identity_matches_render_section_law

The fixture must contain multiple sections, inaccessible items, and a Direct path nested below Home. If new signatures initially fail compilation, add test-only inert stubs, then run until assertions fail for absent cursor behavior.

~~~bash
cargo nextest run --locked -E 'test(/flf_cursor_|flf_model_line_identity/)' --no-fail-fast --status-level fail --final-status-level fail --failure-output final --success-output never
~~~

Expected RED: one-step/cursor assertions fail; no panic, timeout, or compile-only failure counts.

- [ ] **Step 2: Commit the RED after message alignment**

Proposed message:

~~~text
test: specify locations rail cursor authority
~~~

Stage exactly the two files. Do not push.

- [ ] **Step 3: Implement bounded model helpers**

Implement accessible_items, content_line_count, and line_index_for_path without filesystem calls or unbounded allocation. Use the current section order and count one header per section plus one blank between sections.

- [ ] **Step 4: Implement cursor normalization, stepping, reconciliation, and reveal**

Add cursor to FileManagerLocationsState and replace the loose failure tuple with FileManagerLocationFailure. A changed cursor clears obsolete pending/failure authority; a clamped step changes nothing. activate_location synchronizes the exact cursor. activate_direct preserves the prior valid cursor. retire_for_closed_files clears cursor.

- [ ] **Step 5: Run GREEN and a render-purity regression**

~~~bash
cargo nextest run --locked -E 'test(/flf_cursor_|flf_model_line_identity|fcl_origin_|render_entry_row_performs_no_filesystem_io/)' --no-fail-fast --status-level fail --final-status-level fail --failure-output final --success-output never
~~~

Expected: all selected tests pass and no existing exact-origin behavior changes.

- [ ] **Step 6: Refactor only after GREEN**

Remove test-only stubs, run cargo fmt, then rerun the same filter. Propose:

~~~text
feat: add locations rail cursor authority
~~~

Exact-stage the two files and commit. Do not push a failing intermediate HEAD.

---

### Task 2: Route Rail keys before Trail keys

**Files:**
- Modify: src/app/state.rs:2147-2155 and constructor initialization
- Modify: src/app/input/file_manager.rs:85-290, 730-876, and nearby tests
- Modify: src/app/input/mod.rs:169-212
- Modify: src/app/actions.rs:530-544 and lifecycle test
- Modify: src/ui.rs:245-255

- [ ] **Step 1: Add typed request scaffolding and behavioral RED tests**

Replace the path-only request type with FileManagerLocationNavigationRequest. Add tests:

- flf_root_left_focuses_rail_with_exact_or_direct_fallback
- flf_rail_up_down_move_one_and_never_mutate_trail
- flf_rail_owner_swallows_shift_ctrl_and_hidden_trail_actions
- flf_rail_right_and_enter_queue_enter_without_immediate_focus
- flf_compact_root_left_opens_drawer_and_escape_invalidates_entry
- flf_mouse_location_click_synchronizes_cursor_and_typed_intent
- flf_clamped_and_deferred_keys_decline_immediate_render

Use a prepared in-memory Locations model and resident Trail snapshots. Wrap key handling with render_prof::observe_for_test when asserting zero filesystem reads.

Expected RED command:

~~~bash
cargo nextest run --locked -E 'test(/flf_root_left_|flf_rail_|flf_compact_root_left|flf_mouse_location|flf_clamped_and_deferred/)' --no-fail-fast --status-level fail --final-status-level fail --failure-output final --success-output never
~~~

- [ ] **Step 2: Commit assertion-level RED after alignment**

~~~text
test: specify locations follow input ownership
~~~

Stage only the five files listed for this task. Do not push.

- [ ] **Step 3: Add the owner-first Rail reducer**

Immediately after drawer Esc handling and before Trail selection shortcuts, branch on FileManagerLocationsFocus::Rail. Support unmodified Up/k, Down/j, Right/l, Enter, Left/h/Backspace, Esc, and q. Up/Down step one actionable location, reveal it, and stage FollowPreview. Right/Enter stage or promote EnterTrail. Left is inert at Rail. Shift/Ctrl/unknown keys are swallowed without touching Trail.

Use a semantic dispatch for deferred work:

~~~rust
FileManagerKeyDispatch::DeferredLocationNavigation
~~~

App::handle_focused_file_manager_key maps it to file_manager_key_render_override=Some(false). Cursor/focus changes return Consumed and render once.

- [ ] **Step 4: Implement root Left and compact lifecycle**

When TrailState::move_active_left returns false at active_col zero:

1. clear obsolete root request/pending authority,
2. wide layout: normalize cursor and set Rail focus,
3. compact layout: open the drawer first so drawer_restore_focus records Trail, then normalize cursor,
4. perform no I/O.

Esc, outside click, Files close, and resize-invalid drawer closure clear both the typed request and pending focus-after-completion authority. compute_view may perform this geometry-owned invalidation in src/ui.rs; render remains immutable.

- [ ] **Step 5: Preserve mouse semantics with explicit surface intent**

Wide Rail click sets the exact cursor and stages FollowPreview while Rail remains focused. Compact drawer row click sets the cursor and stages EnterTrail; keep the drawer open while pending and close it only on accepted entry. Stale row/model/generation hits remain inert.

- [ ] **Step 6: Run GREEN plus existing wheel/keyboard regressions**

~~~bash
cargo nextest run --locked -E 'test(/flf_root_left_|flf_rail_|flf_compact_root_left|flf_mouse_location|flf_clamped_and_deferred|vertical_navigation|wheel|right_navigation_on_file_is_inert_without_side_effects|fcl_drawer_/)' --no-fail-fast --status-level fail --final-status-level fail --failure-output final --success-output never
~~~

Expected: all FLF tests pass; one wheel detent remains one row; vertical input never crosses columns; Right on a file remains inert.

- [ ] **Step 7: Commit GREEN after alignment**

~~~text
feat: route locations rail keyboard ownership
~~~

Run cargo fmt --check, exact-stage only this task's paths, and commit.

---

### Task 3: Make root Follow/Enter bounded and lifecycle-exact

**Files:**
- Modify: src/app/file_manager_locations.rs
- Modify: src/app/state.rs
- Modify: src/app/file_manager_io_worker.rs:33-280, 300-580, 602-868, tests
- Modify: src/app/file_manager_watcher.rs:403-410 test helper
- Modify: src/app/input/file_manager.rs for synchronous pending-intent promotion

- [ ] **Step 1: Write race, bound, failure, and resident RED tests**

Add:

- flf_follow_request_keeps_rail_focus_until_exact_success
- flf_enter_promotes_exact_pending_without_duplicate_submission
- flf_result_before_request_rejects_old_root_after_cursor_move
- flf_same_path_a_b_a_cannot_revive_old_enter_intent
- flf_blocked_hundred_move_burst_processes_first_and_final_only
- flf_blocked_root_keeps_cursor_input_and_render_loop_responsive
- flf_latest_root_only_updates_cursor_origin_trail_and_focus
- flf_resident_follow_and_enter_perform_zero_worker_reads
- flf_missing_changed_type_permission_preserve_last_accepted_trail
- flf_root_panic_reports_failure_and_lane_remains_reusable
- flf_worker_disconnect_reports_failure_restarts_once_and_next_request_succeeds
- flf_close_model_focus_and_generation_invalidate_completion
- flf_empty_root_succeeds_without_synthetic_cursor

The blocked test uses the existing Gate and with_processor seam. Count processed target paths in an Arc<Mutex<Vec<PathBuf>>>; block the first, stage/sync 99 later cursor requests, release, then assert the processor saw exactly first and final.

The fast blocked-responsiveness unit test uses channels/barriers rather than elapsed-time assertions: while the processor gate is still closed, cursor/focus changes and scheduled-loop progress must already be observable, with no filesystem work on the caller. Hold the same gate closed for the approved 500 ms observation only inside the ignored release calibration in Task 6, where timing is reported rather than used as a flaky pass threshold. Inject `PermissionDenied` through the custom processor so the failure test is independent of host user privileges; use temporary missing and changed-to-file paths for the other typed failures.

Test panic and disconnect separately. Panic uses the existing `with_processor` seam and proves a second request is processed by the same lane. Add a test-only disconnect seam that exposes the worker's observed-dead state without terminating any external process; draining it must fail the exact pending identity and replace the old worker once, a second idle drain must not replace again, and a later explicit request must succeed. Also force the narrower race where `submit` observes disconnect after an earlier drain, proving both detection sites share the same recovery helper.

~~~rust
assert_eq!(processed.as_slice(), [first.as_path(), final_path.as_path()]);
assert_eq!(app.state.file_manager_locations.cursor_path(&model), Some(final_path.as_path()));
assert_eq!(app.state.file_manager_locations.origin, Some(FileManagerLocationOrigin::Location(final_path)));
~~~

Run RED:

~~~bash
cargo nextest run --locked -E 'test(/flf_follow_request|flf_enter_promotes|flf_result_before_request|flf_same_path_a_b_a|flf_blocked_|flf_latest_root|flf_resident_follow|flf_missing_changed|flf_root_panic|flf_worker_disconnect|flf_close_model_focus|flf_empty_root/)' --no-fail-fast --status-level fail --final-status-level fail --failure-output final --success-output never
~~~

- [ ] **Step 2: Commit assertion-level RED after alignment**

~~~text
test: require bounded locations follow loading
~~~

Stage only this task's files. Do not push.

- [ ] **Step 3: Carry intent in request and pending authority**

Add FollowPreview/EnterTrail to the typed AppState request and FileManagerLocationPending. Moving away clears pending before staging the next request. Right/Enter synchronously promotes an exact pending FollowPreview to EnterTrail so the result-before-request scheduled order cannot apply the weaker intent first. Promotion never submits a duplicate and does not render.

- [ ] **Step 4: Preserve one-running-plus-one-latest worker behavior**

sync_file_manager_location_request:

1. take the typed request,
2. validate active Files generation, exact model revision/item accessibility, current cursor path, and Rail focus,
3. use the resident root fast path first,
4. promote an exact pending request without submission,
5. otherwise submit one Root request and store the returned worker generation with intent,
6. return true only when visible pending/failure/Trail/focus state changed.

Do not move filesystem calls into this method.

If `submit` returns `Disconnected`, fail the exact request as unavailable, replace the dead worker through the shared recovery helper, and do not replay the request. A later user action is the only authority that may submit it again.

- [ ] **Step 5: Harden completion acceptance**

For Root success, error, panic, and disconnect, require exact pending identity plus current Rail cursor and valid Rail lifecycle before mutating presentation state. Capture pending intent before clearing it. Store failures with the pending Files generation and model revision so a replacement model containing the same path cannot display an obsolete error. FollowPreview installs the prepared root, origin, and cursor but retains Rail focus and leaves the Trail cursor empty. EnterTrail installs the root then delegates first-entry initialization to Task 4 before focus transfer.

When `drain` reports a true disconnect, call one App helper that (1) applies failure only when the captured pending authority is still current, (2) discards `drain.current` even if present because the old lifecycle is no longer authoritative, (3) replaces the dead worker with `FileManagerIoWorker::new(self.render_notify.clone())`, and (4) logs the lifecycle failure once. Return after recovery rather than falling through to result application. The helper must never auto-resubmit, retry, recurse, or replace a healthy worker. Reuse it from the submit-time disconnect branch so there is one recovery law.

Every rejected root completion increments only a non-rendered diagnostic counter and returns false. It must not alter failure UI.

- [ ] **Step 6: Update the test-only wait helper**

sync_file_manager_location_navigation must wait for the latest accepted worker generation, not merely any pending flag. Resident and promotion-only transitions return without blocking. Add a regression that a replaced generation cannot satisfy the helper early.

- [ ] **Step 7: Run GREEN and current worker regressions**

~~~bash
cargo nextest run --locked -E 'test(/flf_|fcl_io_|directory_preview_after_horizontal_focus_change_is_rejected|missing_directory_preview_preserves_cursor_and_resident_branch/)' --no-fail-fast --status-level fail --final-status-level fail --failure-output final --success-output never
~~~

Expected: all selected pass; panic recovery and latest-wins worker tests remain green.

- [ ] **Step 8: Commit GREEN after alignment**

~~~text
perf: bound locations follow root loading
~~~

Run cargo fmt --check and cargo clippy --all-targets --locked -- -D warnings before exact staging.

---

### Task 4: Initialize the first real entry on explicit hierarchy crossing

**Files:**
- Modify: src/fm/trail_snapshots.rs
- Modify: src/fm/mod.rs
- Modify: src/app/file_manager_miller.rs
- Modify: src/app/file_manager_io_worker.rs
- Modify: src/app/input/file_manager.rs
- Modify: src/app/input/mod.rs

- [ ] **Step 1: Write first-entry RED tests**

Add:

- flf_entered_root_highlights_first_actionable_entry
- flf_entered_child_highlights_first_actionable_entry
- flf_next_down_after_entry_selects_second_entry
- flf_empty_entered_destination_keeps_none_cursor
- flf_keyboard_activation_discards_hidden_destination_history
- flf_mouse_activation_preserves_existing_mouse_selection_contract
- flf_first_entry_initialization_performs_zero_filesystem_reads

Build fixtures whose first row is a directory and whose destination previously had a deeper selected branch. Assert active_col changes by exactly one and no automatic second column transfer occurs.

~~~bash
cargo nextest run --locked -E 'test(/flf_entered_|flf_next_down|flf_empty_entered|flf_keyboard_activation|flf_mouse_activation|flf_first_entry/)' --no-fail-fast --status-level fail --final-status-level fail --failure-output final --success-output never
~~~

- [ ] **Step 2: Commit assertion-level RED after alignment**

~~~text
test: specify first entry focus on file manager entry
~~~

- [ ] **Step 3: Add the shared snapshot-only helper**

TrailSnapshots::reset_active_column_cursor truncates snapshots after the active column, clears detail, calls TrailState::clear_selection_at(active_col), and returns bool (`true` only when the active column exists and was reset). FmState::focus_first_active_trail_entry then moves by +1 and installs the existing operation projection. Wrap it in render_prof::observe_for_test and require fm.filesystem.read == 0.

- [ ] **Step 4: Distinguish keyboard and mouse activation**

Carry FileManagerTrailDestinationPolicy through TrailActivate request, identity, and outcome. Keyboard Right/Enter uses FocusFirstActionable. Mouse row activation uses PreserveMouseSelection. Include the policy in exact result identity so a newer intent cannot accept an older result.

- [ ] **Step 5: Apply the helper at both entry boundaries**

- Resident Right into an already prepared child: move active_col right, then initialize first entry.
- Worker-backed keyboard child activation: initialize in the prepared FmState before installing it.
- Root EnterTrail success/resident fast path: initialize root before setting Trail focus.
- FollowPreview: never initialize a Trail cursor.
- Empty success: install/focus the empty column with cursor=None.

Do not queue a preview for the first directory automatically in this change. The design permits it but does not require it; deferring avoids a second implicit transition and keeps the proof surface minimal.

- [ ] **Step 6: Run GREEN and mouse compatibility**

~~~bash
cargo nextest run --locked -E 'test(/flf_entered_|flf_next_down|flf_empty_entered|flf_keyboard_activation|flf_mouse_activation|flf_first_entry|mouse_activation|trail_row_hit|right_navigation_on_file/)' --no-fail-fast --status-level fail --final-status-level fail --failure-output final --success-output never
~~~

- [ ] **Step 7: Commit GREEN after alignment**

~~~text
feat: focus first entry on file manager transitions
~~~

Exact-stage the six files and commit only after fmt and the focused filter pass.

---

### Task 5: Render truthful focus, origin, failure, and compact state

**Files:**
- Modify: src/ui/file_manager/locations.rs
- Modify: src/ui/file_manager/trail_view.rs
- Modify: src/ui/file_manager.rs
- Modify: src/ui/visual_fixture.rs
- Modify: tests/visual/files-locations.spec.ts
- Create: tests/visual/files-locations.spec.ts-snapshots/vis-26-files-locations-follow-focus-linux.png

- [ ] **Step 1: Write cell-level RED tests**

Add:

- flf_render_rail_cursor_wins_and_origin_remains_subdued
- flf_render_rail_focus_suppresses_trail_cursor_style
- flf_render_no_color_distinguishes_cursor_from_origin_by_modifiers
- flf_render_pending_failure_apply_only_to_current_cursor
- flf_render_is_state_pure_and_geometry_identical
- flf_compact_drawer_focus_matches_wide_rail

Render before/after buffers at identical dimensions. Assert symbols, row rects, and widths are unchanged; only semantic styles/markers may differ. Set a test palette with equal colors and require the Rail cursor to retain REVERSED plus BOLD while the origin uses BOLD or UNDERLINED without REVERSED.

~~~bash
cargo nextest run --locked -E 'test(/flf_render_|flf_compact_drawer_focus/)' --no-fail-fast --status-level fail --final-status-level fail --failure-output final --success-output never
~~~

- [ ] **Step 2: Commit assertion-level RED after alignment**

~~~text
test: specify locations follow rendering
~~~

- [ ] **Step 3: Implement pure semantic styles**

In render_file_manager_locations compute cursor and accepted origin separately:

~~~rust
let cursor = app
    .file_manager_locations
    .cursor_path(&app.file_manager_locations_model);
let origin = app
    .file_manager_locations
    .highlighted_path(&app.file_manager_locations_model);
let rail_focused =
    app.file_manager_locations.focus == FileManagerLocationsFocus::Rail;
~~~

Style precedence:

1. focused cursor: accent plus BOLD and REVERSED,
2. accepted origin when different: accent foreground plus BOLD/UNDERLINED,
3. accessible row,
4. inaccessible row.

Pending/failure markers remain width-neutral. Failure renders only when its exact path equals the current cursor and current model authority.

- [ ] **Step 4: Suppress only painted Trail focus**

In render_trail_view keep selected_entry for viewport projection but set selected styling only when focus is Trail. Do not mutate TrailState or erase its selected/cursor identity. Existing ancestor focus returns byte-for-byte when focus transfers back.

- [ ] **Step 5: Add one deterministic FLF fixture**

Add a dedicated ignored exporter that writes only vis-26-files-locations-follow-focus. It must:

- use ASCII icons,
- set fixed mtimes before opening snapshots,
- render Rail cursor on Downloads while accepted origin/Trail remain Home,
- expose both cursor and origin modifiers in JSON,
- leave existing VIS-18..25 fixture files untouched.

Extend files-locations.spec.ts with exactly one new case. First generate into the ignored generated directory:

~~~bash
HERDR_VISUAL_FIXTURE_DIR=/home/ayaz/projects/herdr/tests/visual/fixtures/generated cargo nextest run --locked --run-ignored only -E 'test(write_locations_follow_visual_fixture)' --status-level all --success-output immediate
~~~

- [ ] **Step 6: Inspect before creating the PNG**

Run the new Playwright test without snapshot update. Expected RED is one missing snapshot after the cell contract passes. Open the actual image with the local image viewer and verify cursor/origin distinction, no glyph shift, no clipped marker, and inactive Trail highlight. Only then run:

~~~bash
cd tests/visual
npx playwright test files-locations.spec.ts --grep 'vis-26-files-locations-follow-focus' --update-snapshots
~~~

Verify git status shows only the new PNG, never modifications to VIS-18..25. Re-run without update; expected 1/1 pass.

- [ ] **Step 7: Run GREEN plus all Locations visuals**

~~~bash
cargo nextest run --locked -E 'test(/flf_render_|flf_compact_drawer_focus|fcl_render_prepared_locations|selected_rows_highlight/)' --no-fail-fast --status-level fail --final-status-level fail --failure-output final --success-output never
cd tests/visual
npx playwright test files-locations.spec.ts
~~~

- [ ] **Step 8: Commit GREEN after alignment**

~~~text
feat: render locations follow focus
~~~

Exact-stage only the five source/test files and the new PNG. Confirm existing PNG hashes are unchanged.

---

### Task 6: Measure Follow mode and lock structural performance gates

**Files:**
- Modify: src/app/file_manager_io_worker.rs
- Modify locally: .local/herdr-files-v1-profile.sh
- Create locally: .local/perf/locations-follow/ result directories
- Create: .codex/evidence/files-locations-follow-performance.md

- [ ] **Step 1: Add fixed-label profiler counters and tests**

Use the existing bounded profiler with static labels:

- fm.locations.root.submitted
- fm.locations.root.replaced
- fm.locations.root.processed
- fm.locations.root.accepted
- fm.locations.root.failed
- fm.locations.root.stale
- fm.locations.root.enumeration

Do not include paths in labels. Add a test that success, failure, replacement, and stale rejection increment the exact counters and that the label set stays bounded.

- [ ] **Step 2: Add an ignored release-profile calibration**

Create flf_scale_locations_follow_navigation beside the worker tests. It creates temporary small, 10k, and 100k flat roots, measures dispatch and root enumeration p50/p95/max, executes the blocked 100-event proof, holds the worker gate closed for the approved 500 ms loop-continuity observation, records final settle time, and reports Linux test-process VmRSS when available. Structural assertions are mandatory; timing values are observations, not pre-invented CI limits.

The TempDir Drop guard removes every synthetic root. No fixture may use the user's Home/Desktop/Downloads.

- [ ] **Step 3: Run focused structural tests first**

~~~bash
cargo nextest run --locked -E 'test(/flf_blocked_|flf_root_panic|flf_worker_disconnect|flf_profiler/)' --no-fail-fast --status-level fail --final-status-level fail --failure-output final --success-output never
~~~

Expected: at most first/final processing, reusable lane after panic, exactly one replacement after injected disconnect, and bounded labels.

- [ ] **Step 4: Run the explicit release calibration**

~~~bash
cargo nextest run --release --locked --run-ignored only -E 'test(flf_scale_locations_follow_navigation)' --status-level all --final-status-level slow --failure-output immediate-final --success-output immediate
~~~

Record filesystem, build profile, fixture counts, samples, p50/p95/max, processed targets, final state, and settled RSS. A host timing regression triggers diagnosis, not an arbitrary threshold edit.

- [ ] **Step 5: Make the local live profiler one-command and cleanup-first**

Extend .local/herdr-files-v1-profile.sh locally so run prints the saved report after normal exit while preserving:

- exact TEST_ROOT ownership marker,
- semantic server stop through the isolated socket,
- no kill/pkill,
- refusal to delete while the socket remains,
- trap cleanup on normal exit/error/Ctrl-C,
- inherited HERDR variables unset,
- stable_socket_touched=false in evidence.

The user command remains one line:

~~~bash
cd /home/ayaz/projects/herdr && HERDR_RENDER_PROF=1 ./.local/herdr-files-v1-profile.sh run
~~~

Do not stage .local paths.

- [ ] **Step 6: Write evidence and commit after alignment**

Populate .codex/evidence/files-locations-follow-performance.md with raw commands, run IDs, counts, timings, structural verdicts, and confidence. Proposed message:

~~~text
perf: measure locations follow navigation
~~~

Exact-stage src/app/file_manager_io_worker.rs and the evidence file. Never stage .local/perf.

---

### Task 7: Complete isolated runtime acceptance, gates, docs, and publication

**Files:**
- Modify: docs/patterns/rust-engineering.md
- Modify: .codex/references/yazi-file-manager-performance-transfer.md
- Modify as evidence requires: .codex/skills/herdr-native-fm/lessons/errors.md
- Modify as evidence requires: .codex/skills/herdr-native-fm/lessons/golden-paths.md
- Modify as evidence requires: .codex/skills/herdr-native-fm/lessons/edge-cases.md
- Modify: .codex/CURRENT.md, .codex/TASKS.md, .codex/HANDOFF.md, .codex/NEXT-SESSION-PROMPT.md
- Modify: .planning/STATE.md
- Modify: .codex/evidence/files-locations-follow-performance.md

- [ ] **Step 1: Run the user's isolated live acceptance**

The user launches the cleanup-first helper in a new Ghostty window. Test:

1. Left from Trail root reaches the visible Rail/drawer.
2. Up/Down move Home/Desktop/Downloads one row at a time.
3. Follow preview never jumps focus into Trail.
4. Right/Enter focuses the root and highlights its first entry immediately.
5. The next Down selects the second entry.
6. Right on a directory focuses its child and first child entry.
7. Right on a file is inert.
8. Empty/missing roots show no fake row and preserve the previous accepted Trail on failure.
9. Rapid switching produces no freeze, wrong-root flash, or late focus jump.
10. Normal exit removes the test-owned socket/root.

Profiler evidence must show loop ticks continue while root I/O is blocked and render attempts correspond to visible cursor/pending/completion changes, not raw/clamped input.

- [ ] **Step 2: Run all automated gates fresh**

~~~bash
export PATH="$HOME/.local/bin:$PATH"
cargo nextest run --locked -E 'test(/flf_/)' --no-fail-fast --status-level fail --final-status-level fail --failure-output final --success-output never
cd tests/visual
npx playwright test
cd /home/ayaz/projects/herdr
just check
~~~

Expected: every FLF test, the full visual suite, fmt, Linux clippy, Windows clippy, full nextest, integration assets, plugin marketplace, and maintenance tests pass. Record exact counts and run IDs after reading the output.

- [ ] **Step 3: Run static safety and diff audits**

~~~bash
git diff refs/checkpoints/herdr-flf-pre-implementation-20260722..HEAD --check
env -u RIPGREP_CONFIG_PATH rg -n 'read_dir|metadata|canonicalize' src/app/input/file_manager.rs src/ui/file_manager/locations.rs src/ui/file_manager/trail_view.rs
git status --short
git diff --name-only refs/checkpoints/herdr-flf-pre-implementation-20260722..HEAD
git diff refs/checkpoints/herdr-flf-pre-implementation-20260722..HEAD -- tests/visual/files-locations.spec.ts-snapshots/
~~~

Expected: no filesystem call in input/render; only the new VIS-26 PNG differs; .superpowers/ remains untracked and unstaged.

- [ ] **Step 4: Re-run graph impact and freshness**

Verify updated symbols and trace:

- Rail key -> typed request -> sync_file_manager_location_request
- worker Root -> sync_file_manager_io_results -> accepted root
- keyboard Trail activation -> FocusFirstActionable -> focus_first_active_trail_entry
- AppState -> pure Locations/Trail render

Record final graph totals and any high-risk inbound callers in evidence.

- [ ] **Step 5: Save generalized engineering lessons**

Add a concise verified pattern to docs/patterns/rust-engineering.md:

- cursor, accepted origin, and pending work are separate authority,
- movement is cheap and synchronous,
- preparation is bounded/latest-wins,
- activation alone transfers ownership,
- old result plus same returned path still requires lifecycle invalidation.

Update the Yazi transfer reference with measured Herdr evidence, not a claim that Herdr copied Yazi's cache topology.

Append skill lesson rows only for facts actually encountered:

~~~text
| error | cause | fix | 2026-07-22 |
| scenario | steps | precondition | 2026-07-22 |
| situation | solution | 2026-07-22 |
~~~

Do not duplicate an existing lesson row.

- [ ] **Step 6: Close continuity truthfully**

Update CURRENT, TASKS, HANDOFF, NEXT-SESSION-PROMPT, STATE, and evidence with:

- all 20 test-point verdicts,
- exact commit SHAs,
- local/remote SHA relationship,
- live acceptance status,
- gate counts/run IDs,
- stable runtime untouched,
- .superpowers untouched,
- FMN-6 pinned pre-warm still separate,
- any genuinely open failure.

- [ ] **Step 7: Commit docs/closure after message alignment**

Proposed message:

~~~text
docs: close locations follow navigation
~~~

Exact-stage only the listed docs/continuity/evidence paths. Do not stage .local or .superpowers.

- [ ] **Step 8: Final publication gate**

Before push:

~~~bash
just check
git status --short
git diff --cached --name-only
git log --oneline --decorate -12
~~~

Push only after every commit message has been aligned and HEAD is green:

~~~bash
git push origin HEAD:feat/native-fm
git rev-parse HEAD
git rev-parse origin/feat/native-fm
~~~

Expected: local and origin SHAs are identical. Never push upstream and never open an upstream issue or PR.

## Final Completion Checklist

- [ ] The predecessor was separated before FLF edits.
- [ ] All 20 approved test IDs have fresh evidence.
- [ ] Rail cursor, accepted origin, and pending intent remain distinct.
- [ ] Input/render perform zero filesystem enumeration.
- [ ] The blocked 100-event proof processes at most first and final requests.
- [ ] Processor panic reuses the existing lane; true disconnect replaces it once and never replays the failed request.
- [ ] A-to-B-to-A cannot revive an old EnterTrail completion.
- [ ] Root and child entry highlight the first real row immediately.
- [ ] The next Down selects the second row.
- [ ] Empty/failure states never fabricate a highlight.
- [ ] Mouse behavior remains explicitly tested.
- [ ] Compact and wide ownership laws match.
- [ ] Only the new FLF PNG was created; old snapshots are unchanged.
- [ ] Release-profile observations and isolated live evidence are recorded.
- [ ] just check and the full Playwright suite pass fresh.
- [ ] Stable Herdr and .superpowers/ remain untouched.
- [ ] Every commit was exact-staged, message-aligned, and pushed only to origin.
