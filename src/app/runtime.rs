use std::time::{Duration, Instant};

use crossterm::terminal;

use super::{
    background_update_check_enabled, pressed_key_identity, App, ANIMATION_INTERVAL,
    AUTO_UPDATE_CHECK_INTERVAL, GIT_REMOTE_STATUS_REFRESH_INTERVAL, MIN_RENDER_INTERVAL,
    RESIZE_POLL_INTERVAL, SELECTION_AUTOSCROLL_INTERVAL,
};
use crate::events::AppEvent;
use crate::workspace::{GitStatusCacheEntry, Workspace, WorkspaceGitStatus};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WorkspaceGitRefreshItem {
    pub(crate) workspace_id: String,
    pub(crate) resolved_identity_cwd: std::path::PathBuf,
    pub(crate) cache_key: std::path::PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WorkspaceGitRefreshTarget {
    pub(crate) workspace_id: String,
    pub(crate) resolved_identity_cwd: std::path::PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WorkspaceGitRefreshJob {
    pub(crate) cache_key: std::path::PathBuf,
    pub(crate) status_cwd: std::path::PathBuf,
    pub(crate) cached: Option<GitStatusCacheEntry>,
    pub(crate) targets: Vec<WorkspaceGitRefreshTarget>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WorkspaceGitRefreshOutput {
    pub(crate) results: Vec<WorkspaceGitStatus>,
    pub(crate) cache_updates: Vec<(std::path::PathBuf, GitStatusCacheEntry)>,
}

fn retain_custom_command_after_wait(
    pid: u32,
    result: std::io::Result<Option<std::process::ExitStatus>>,
) -> bool {
    match result {
        Ok(None) => true,
        Ok(Some(_)) => false,
        Err(err) if err.kind() == std::io::ErrorKind::Interrupted => true,
        Err(err) => {
            tracing::warn!(pid, err = %err, "failed to reap detached custom command");
            false
        }
    }
}

impl App {
    pub(crate) fn reap_finished_custom_commands(&mut self) {
        self.detached_custom_command_children
            .retain_mut(|child| retain_custom_command_after_wait(child.id(), child.try_wait()));
    }

    pub(crate) fn shutdown_detached_terminal_runtimes(&mut self) {
        let terminal_ids = std::mem::take(&mut self.state.terminal_runtime_shutdowns);
        for terminal_id in terminal_ids {
            if let Some(runtime) = self.terminal_runtimes.remove(&terminal_id) {
                runtime.shutdown();
            }
        }
    }

    pub(crate) fn drain_api_requests(&mut self) -> bool {
        let mut changed = false;
        while let Ok(msg) = self.api_rx.try_recv() {
            changed |= self.handle_api_request_message(msg);
            self.shutdown_detached_terminal_runtimes();
        }
        changed
    }

    pub(super) fn handle_api_request_message(
        &mut self,
        msg: crate::api::ApiRequestMessage,
    ) -> bool {
        let previous_mode = self.state.mode;
        let mut changed = self.expire_due_metadata(Instant::now());
        changed |= crate::api::request_changes_ui(&msg.request);
        let skip_default_workspace = matches!(
            &msg.request.method,
            crate::api::schema::Method::ServerStop(_)
                | crate::api::schema::Method::ServerLiveHandoff(_)
        );
        if matches!(
            &msg.request.method,
            crate::api::schema::Method::WorktreeCreate(_)
                | crate::api::schema::Method::WorktreeRemove(_)
        ) {
            self.drain_all_internal_events();
            let deferred_changed =
                self.handle_deferred_worktree_api_request(msg.request, msg.respond_to);
            if !skip_default_workspace {
                changed |= self.ensure_default_workspace();
            }
            self.sync_prefix_input_source(previous_mode);
            return changed | deferred_changed;
        }
        let response = self.handle_api_request(msg.request);
        if !skip_default_workspace {
            changed |= self.ensure_default_workspace();
        }
        let _ = msg.respond_to.send(response);
        self.sync_prefix_input_source(previous_mode);
        changed
    }

    pub(super) async fn handle_raw_input_batch(
        &mut self,
        first: crate::raw_input::RawInputEvent,
    ) -> bool {
        let mut changed = self.handle_raw_input_event(first).await;

        while let Some(rx) = self.input_rx.as_mut() {
            match rx.try_recv() {
                Ok(event) => changed |= self.handle_raw_input_event(event).await,
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    self.input_rx = None;
                    break;
                }
            }
        }

        changed
    }

    pub(super) async fn handle_raw_input_event(
        &mut self,
        event: crate::raw_input::RawInputEvent,
    ) -> bool {
        let previous_mode = self.state.mode;
        let changed = match event {
            crate::raw_input::RawInputEvent::Key(key) => {
                let pressed_key_id = pressed_key_identity(super::LOCAL_INPUT_SOURCE, &key);
                match key.kind {
                    crossterm::event::KeyEventKind::Press => {
                        if self.state.popup_pane.is_some()
                            || self.state.mode == crate::app::Mode::Terminal
                        {
                            self.suppressed_repeat_keys.remove(&pressed_key_id);
                        } else {
                            self.suppressed_repeat_keys.insert(pressed_key_id);
                        }
                        if let Some(target) = self.handle_key(key).await {
                            if !key.is_text_commit {
                                self.pressed_terminal_keys.insert(
                                    pressed_key_id,
                                    super::PressedTerminalKey { target, key },
                                );
                            }
                        } else {
                            self.pressed_terminal_keys.remove(&pressed_key_id);
                        }
                        true
                    }
                    crossterm::event::KeyEventKind::Repeat => {
                        if let Some(pressed) =
                            self.pressed_terminal_keys.get(&pressed_key_id).cloned()
                        {
                            if !self
                                .forward_terminal_key_to_target(&pressed.target, key)
                                .await
                            {
                                self.pressed_terminal_keys.remove(&pressed_key_id);
                            }
                            true
                        } else if (self.state.popup_pane.is_some()
                            || self.state.mode == crate::app::Mode::Terminal)
                            && !self.suppressed_repeat_keys.contains(&pressed_key_id)
                        {
                            self.handle_key(key).await;
                            true
                        } else {
                            false
                        }
                    }
                    crossterm::event::KeyEventKind::Release => {
                        self.suppressed_repeat_keys.remove(&pressed_key_id);
                        if let Some(pressed) = self.pressed_terminal_keys.remove(&pressed_key_id) {
                            let _ = self
                                .forward_terminal_key_to_target(&pressed.target, key)
                                .await;
                        }
                        false
                    }
                }
            }
            crate::raw_input::RawInputEvent::Paste(text) => {
                self.handle_paste(text).await;
                true
            }
            crate::raw_input::RawInputEvent::Mouse(mouse) => {
                if self.state.popup_pane.is_some() || self.state.mouse_capture {
                    self.handle_mouse(mouse);
                } else {
                    self.state
                        .handle_pane_mouse_only(&self.terminal_runtimes, mouse);
                }
                true
            }
            crate::raw_input::RawInputEvent::OuterFocusGained => {
                self.send_outer_focus_event(crate::ghostty::FocusEvent::Gained);
                if self.state.redraw_on_focus_gained {
                    self.request_repaint();
                }
                self.state.outer_terminal_focus = Some(true);
                self.state.mark_active_tab_seen();
                true
            }
            crate::raw_input::RawInputEvent::OuterFocusLost => {
                self.release_input_source(super::LOCAL_INPUT_SOURCE).await;
                self.send_outer_focus_event(crate::ghostty::FocusEvent::Lost);
                self.state.outer_terminal_focus = Some(false);
                false
            }
            crate::raw_input::RawInputEvent::HostDefaultColor { kind, color } => {
                self.update_host_terminal_theme(kind, color)
            }
            crate::raw_input::RawInputEvent::HostPaletteColors { colors } => {
                self.update_host_terminal_palette_colors(&colors)
            }
            crate::raw_input::RawInputEvent::HostColorSchemeChanged(appearance) => {
                self.query_host_terminal_theme();
                self.set_host_terminal_appearance(appearance, true)
            }
            crate::raw_input::RawInputEvent::Unsupported => false,
        };
        self.sync_prefix_input_source(previous_mode);
        self.shutdown_detached_terminal_runtimes();
        changed
    }

    fn handle_resize_poll(&mut self) -> bool {
        let Ok(size) = terminal::size() else {
            return false;
        };
        if self.last_terminal_size != Some(size) {
            self.last_terminal_size = Some(size);
            return true;
        }
        false
    }

    /// Runs scheduled work once for every display, inside that display's view.
    ///
    /// The workers themselves are unchanged and know nothing about displays:
    /// inside the window, `state.file_manager` is the browser of the display
    /// being served, exactly as it is during that display's render and input.
    /// This is the only place that has to know there is more than one.
    ///
    /// TP-SUR-FM-02
    pub(crate) fn for_each_display(&mut self, mut body: impl FnMut(&mut Self) -> bool) -> bool {
        let mut changed = false;
        for display in self.state.displays_to_serve() {
            let previous = self.state.enter_viewer(display);
            changed |= body(self);
            self.state.restore_viewer(previous);
        }
        changed
    }

    pub(crate) fn handle_scheduled_tasks(&mut self, now: Instant, geometry_dirty: bool) -> bool {
        let mut changed = false;
        let mut resized = false;

        changed |= self.sync_file_operation_worker();
        changed |= self.sync_file_manager_agent_handoff_send();
        changed |= self.sync_agent_attachment_delivery();
        changed |= self.for_each_display(|app| {
            let mut changed = false;
            // Three consumers compete for one context-action field, and the
            // order between them is the precedence: send-to-agent, then a
            // plugin, then the ordinary actions. All three resolve against
            // the raising display's browser, so all three belong in its view.
            changed |= app.sync_file_manager_agent_handoff();
            changed |= app.sync_file_manager_plugin_action();
            changed |= app.sync_agent_reference_picker();
            changed |= app.sync_file_manager_requests();
            changed |= app.sync_file_manager_io_results();
            changed |= app.sync_file_manager_location_request();
            changed |= app.sync_file_manager_watcher_at(now);
            changed |= app.sync_file_preview_worker();
            changed |= app.sync_image_preview_worker();
            changed
        });
        self.sync_animation_timer(now);
        changed |= self.refresh_projects_if_due(now);
        changed |= self.refresh_tab_branches_if_due(now);
        // No presentation surface reads preview_bindings yet, so a refresh
        // must not dirty the frame (flip to `changed |=` once a marker renders).
        let _ = self.refresh_preview_bindings_if_due(now);

        if now >= self.next_resize_poll {
            resized = self.handle_resize_poll();
            changed |= resized;
            self.next_resize_poll = now + RESIZE_POLL_INTERVAL;
        }

        if self
            .config_diagnostic_deadline
            .is_some_and(|deadline| now >= deadline)
        {
            self.config_diagnostic_deadline = None;
            self.state.config_diagnostic = None;
            changed = true;
        }

        if self.toast_deadline.is_some_and(|deadline| now >= deadline) {
            self.toast_deadline = None;
            self.state.toast = None;
            changed = true;
        }

        if self
            .state
            .next_pending_agent_notification_deadline()
            .is_some_and(|deadline| now >= deadline)
        {
            let previous_toast = self.state.toast.clone();
            let mut deliveries = self.state.drain_due_agent_notifications(now);
            if !deliveries.is_empty() {
                self.refresh_agent_notification_delivery_contexts(&mut deliveries);
                self.emit_delayed_client_local_agent_notifications(&deliveries);
                self.sync_toast_deadline(previous_toast);
                changed = true;
            }
        }

        if self
            .state
            .next_managed_agent_deadline()
            .is_some_and(|deadline| now >= deadline)
        {
            let panes = self.state.reconcile_managed_agents_at(now);
            if !panes.is_empty() {
                for (ws_idx, pane_id) in panes {
                    self.emit_pane_updated(ws_idx, pane_id);
                }
                self.schedule_session_save();
                changed = true;
            }
        }

        if self
            .copy_feedback_deadline
            .is_some_and(|deadline| now >= deadline)
        {
            self.copy_feedback_deadline = None;
            self.state.copy_feedback = None;
            changed = true;
        }

        if self
            .next_animation_tick
            .is_some_and(|deadline| now >= deadline)
        {
            self.state.spinner_tick = self.state.spinner_tick.wrapping_add(1);
            self.next_animation_tick = Some(now + ANIMATION_INTERVAL);
            changed = true;
        }

        if self
            .selection_autoscroll_deadline
            .is_some_and(|deadline| now >= deadline)
        {
            self.tick_selection_autoscroll(now);
            changed = true;
        }

        changed |= self.clear_due_selection_highlight(now);

        // A new reading changes what a section says, so it is a reason to draw
        // — and the only reason. The screen follows the data's clock, not the
        // other way round.
        changed |= self.tick_resource_sample(now);

        self.start_git_status_refresh_if_due(now);

        if self
            .next_auto_update_check
            .is_some_and(|deadline| now >= deadline)
        {
            self.run_auto_update_check();
        }

        if self
            .next_agent_manifest_update_check
            .is_some_and(|deadline| now >= deadline)
        {
            self.run_agent_manifest_update_check();
        }

        if self
            .session_save_deadline
            .is_some_and(|deadline| now >= deadline)
        {
            self.start_background_session_save();
        }

        changed |= self.expire_due_metadata(now);

        if geometry_dirty || resized {
            self.pending_agent_resume_deadline = None;
        } else {
            self.sync_pending_agent_resume_deadline(now);
            changed |= self.start_pending_agent_resumes(self.pending_agent_resume_due(now));
        }
        self.sync_animation_timer(now);
        changed
    }

    /// Clears temporary copied-token highlights, such as after double-click copy.
    pub(crate) fn clear_due_selection_highlight(&mut self, now: Instant) -> bool {
        if self
            .selection_highlight_clear_deadline
            .is_none_or(|deadline| now < deadline)
        {
            return false;
        }

        self.selection_highlight_clear_deadline = None;
        if self
            .state
            .selection
            .as_ref()
            .is_some_and(|selection| !selection.is_in_progress())
        {
            self.state.clear_selection();
            return true;
        }
        false
    }

    pub(crate) fn sync_agent_metadata_deadline(&mut self) {
        self.agent_metadata_deadline = self.state.next_agent_metadata_expiry();
    }

    pub(crate) fn expire_due_metadata(&mut self, now: Instant) -> bool {
        let Some(deadline) = self
            .agent_metadata_deadline
            .filter(|deadline| now >= *deadline)
        else {
            return false;
        };
        self.expire_metadata_at(deadline, now);
        true
    }

    pub(crate) fn expire_metadata_at(&mut self, deadline: Instant, now: Instant) {
        let previous_toast = self.state.toast.clone();
        for update in self.state.expire_agent_metadata_at(deadline, now) {
            self.refresh_new_herdr_toast_context_for_update(&update, &previous_toast);
            self.emit_pane_state_update(&update);
        }
        let (panes, workspaces) = self.state.expire_metadata_tokens(now);
        for (ws_idx, pane_id) in panes {
            self.emit_pane_updated(ws_idx, pane_id);
        }
        for ws_idx in workspaces {
            self.emit_workspace_token_updated(ws_idx);
        }
        self.sync_agent_metadata_deadline();
    }

    pub(crate) fn sync_animation_timer(&mut self, now: Instant) {
        self.sync_animation_timer_with_interval(now, ANIMATION_INTERVAL);
    }

    pub(crate) fn sync_headless_animation_timer(&mut self, now: Instant) {
        self.sync_animation_timer_with_interval(now, crate::app::HEADLESS_ANIMATION_INTERVAL);
    }

    fn sync_animation_timer_with_interval(&mut self, now: Instant, interval: Duration) {
        if self.agent_panel_has_animation() || self.any_tab_flash_active(now) {
            self.next_animation_tick.get_or_insert(now + interval);
        } else {
            self.next_animation_tick = None;
        }
    }

    fn agent_panel_has_animation(&self) -> bool {
        self.state
            .workspaces
            .iter()
            .any(|ws| ws.has_working_pane(&self.state.terminals))
    }

    /// A freshly spawned tab's flash window drives frames the same way a
    /// working spinner does — without this the headless renderer never ticks
    /// and the flash is simply never drawn. TP-TAB-FLASH-03
    fn any_tab_flash_active(&self, now: Instant) -> bool {
        self.state
            .workspaces
            .iter()
            .any(|ws| ws.tabs.iter().any(|tab| tab.flash_window_active(now)))
    }

    pub(crate) fn tick_selection_autoscroll(&mut self, now: Instant) {
        let Some(autoscroll) = self.state.selection_autoscroll.clone() else {
            // Self-heal: state cleared but deadline leaked
            self.selection_autoscroll_deadline = None;
            return;
        };

        // Selection must still be in progress for autoscroll to continue
        let Some(pane_id) = self.state.selection.as_ref().map(|s| s.pane_id) else {
            self.stop_selection_autoscroll();
            return;
        };
        if !self
            .state
            .selection
            .as_ref()
            .is_some_and(|s| s.is_dragging())
        {
            self.stop_selection_autoscroll();
            return;
        }

        // Rect-change detection: if inner_rect changed since drag, stop
        let current_rect = self
            .state
            .pane_info_by_id(pane_id)
            .map(|info| info.inner_rect);
        if current_rect != Some(autoscroll.inner_rect) {
            self.stop_selection_autoscroll();
            return;
        }

        // Scrollback boundary detection via ScrollMetrics — fail-closed if unavailable
        let Some(metrics) = self
            .state
            .pane_scroll_metrics(&self.terminal_runtimes, pane_id)
        else {
            self.stop_selection_autoscroll();
            return;
        };
        match autoscroll.direction {
            crate::app::state::SelectionAutoscrollDirection::Up => {
                let at_top = metrics.offset_from_bottom >= metrics.max_offset_from_bottom;
                if at_top {
                    self.stop_selection_autoscroll();
                    return;
                }
                self.state
                    .scroll_pane_up(&self.terminal_runtimes, pane_id, 1);
            }
            crate::app::state::SelectionAutoscrollDirection::Down => {
                let at_bottom = metrics.offset_from_bottom == 0;
                if at_bottom {
                    self.stop_selection_autoscroll();
                    return;
                }
                self.state
                    .scroll_pane_down(&self.terminal_runtimes, pane_id, 1);
            }
        }

        // Extend selection cursor to last known mouse position
        self.state.update_selection_cursor(
            &self.terminal_runtimes,
            pane_id,
            autoscroll.last_mouse_screen_col,
            autoscroll.last_mouse_screen_row,
        );

        // Reschedule
        self.selection_autoscroll_deadline = Some(now + SELECTION_AUTOSCROLL_INTERVAL);
    }

    pub(crate) fn stop_selection_autoscroll(&mut self) {
        self.state.stop_selection_autoscroll_state();
        self.selection_autoscroll_deadline = None;
    }

    pub(crate) fn can_render_now(&self, now: Instant) -> bool {
        match self.last_render_at {
            Some(last_render_at) => now.duration_since(last_render_at) >= MIN_RENDER_INTERVAL,
            None => true,
        }
    }

    pub(crate) fn run_auto_update_check(&mut self) {
        if !background_update_check_enabled(self.no_session, self.update_version_check_enabled) {
            self.next_auto_update_check = None;
            return;
        }

        self.next_auto_update_check = self
            .state
            .update_available
            .is_none()
            .then_some(Instant::now() + AUTO_UPDATE_CHECK_INTERVAL);

        if self.state.update_available.is_some() {
            return;
        }

        let update_tx = self.event_tx.clone();
        std::thread::spawn(move || crate::update::auto_update(update_tx));
    }

    pub(crate) fn run_agent_manifest_update_check(&mut self) {
        if !background_update_check_enabled(self.no_session, self.update_manifest_check_enabled) {
            self.next_agent_manifest_update_check = None;
            return;
        }

        self.next_agent_manifest_update_check = Some(Instant::now() + AUTO_UPDATE_CHECK_INTERVAL);

        let manifest_update_tx = self.event_tx.clone();
        std::thread::spawn(move || crate::detect::manifest_update::auto_update(manifest_update_tx));
    }

    pub(crate) fn start_git_status_refresh_if_due(&mut self, now: Instant) {
        let Some(deadline) = self.git_refresh_deadline() else {
            return;
        };

        if now < deadline {
            return;
        }

        let workspaces = self.workspace_git_refresh_items();

        if workspaces.is_empty() {
            self.last_git_remote_status_refresh = now;
            return;
        }

        self.git_refresh_in_flight = true;
        let event_tx = self.event_tx.clone();
        let cache = self.git_status_cache.clone();
        std::thread::spawn(move || {
            let output = refresh_workspace_git_statuses_with_cache(workspaces, &cache);
            let _ = event_tx.blocking_send(AppEvent::GitStatusRefreshed {
                results: output.results,
                cache_updates: output.cache_updates,
            });
        });
    }

    pub(crate) fn mark_git_status_refresh_due(&mut self, now: Instant) {
        if self.git_refresh_in_flight {
            self.git_refresh_due_after_in_flight = true;
            return;
        }
        self.last_git_remote_status_refresh = now
            .checked_sub(GIT_REMOTE_STATUS_REFRESH_INTERVAL)
            .unwrap_or(now);
        self.git_refresh_due_after_in_flight = false;
    }

    pub(crate) fn git_refresh_deadline(&self) -> Option<Instant> {
        (!self.git_refresh_in_flight && !self.state.workspaces.is_empty())
            .then_some(self.last_git_remote_status_refresh + GIT_REMOTE_STATUS_REFRESH_INTERVAL)
    }

    /// How often the machine's counters are re-read.
    ///
    /// Two seconds by default: slow enough that the cost is unmeasurable next
    /// to a single keystroke's worth of terminal work, fast enough that a build
    /// starting is visible before it finishes. It became configurable once it
    /// had the three things a settable number needs — a floor, a refusal
    /// message and a place in the config reference — which is the layer
    /// `shell.resource_interval_ms` added.
    ///
    /// Read from state rather than held as a constant so a config reload
    /// changes the pace of a running herdr; a constant would have meant
    /// restarting to try a number.
    fn resource_sample_interval(&self) -> Duration {
        self.state.resource_sample_interval
    }

    /// When the next reading is due, or None when nothing on screen wants one.
    ///
    /// The `wants_resources` gate is the whole cost argument: with no resource
    /// section configured this returns None, the loop never wakes for it, and
    /// `/proc` is never opened. Sampling unconditionally and discarding the
    /// result would be simpler and would make every herdr on every machine pay
    /// for a widget almost nobody has turned on.
    // TP-RES-08: no resource section means no deadline and no reading.
    pub(crate) fn resource_sample_deadline(&self) -> Option<Instant> {
        if !self.state.shell_bar_chrome.wants_resources() {
            return None;
        }
        // Only ever consulted to decide how long to sleep. Whether a reading is
        // actually owed is decided by `tick_resource_sample` against the `now`
        // it is given, so the two cannot disagree — an earlier draft asked this
        // getter, which called `Instant::now()` internally, and the first
        // reading was therefore due at a moment that had already passed by the
        // time the caller compared against it. It never fired.
        Some(
            self.last_resource_sample_at
                .map_or_else(Instant::now, |last| last + self.resource_sample_interval()),
        )
    }

    /// Reads the machine, if a reading is due.
    ///
    /// Called from the loop and from nowhere else. The first call can only
    /// establish a baseline — a percentage is a difference, and there is
    /// nothing yet to differ from — so CPU stays unknown until the second,
    /// which is honest: `--` for two seconds, not a fabricated 0%.
    // TP-RES-09: the loop samples, the renderer never does.
    pub(crate) fn tick_resource_sample(&mut self, now: Instant) -> bool {
        if !self.state.shell_bar_chrome.wants_resources() {
            return false;
        }
        if self
            .last_resource_sample_at
            .is_some_and(|last| now < last + self.resource_sample_interval())
        {
            return false;
        }

        self.resource_samples_taken = self.resource_samples_taken.saturating_add(1);
        self.last_resource_sample_at = Some(now);

        let times = crate::platform::read_cpu_times();
        self.state.resources.cpu = match (self.previous_cpu_times, times) {
            (Some(prev), Some(current)) => crate::resource::cpu_percent(prev, current),
            _ => None,
        };
        if times.is_some() {
            self.previous_cpu_times = times;
        }

        let (mem, swap) = crate::platform::read_memory();
        self.state.resources.mem = mem;
        self.state.resources.swap = swap;
        true
    }

    pub(crate) fn next_loop_deadline(&self, now: Instant, needs_render: bool) -> Option<Instant> {
        self.next_loop_deadline_with_resize_poll(now, needs_render, true, true)
    }

    pub(crate) fn next_headless_loop_deadline_with_git_refresh(
        &self,
        now: Instant,
        needs_render: bool,
        include_git_refresh: bool,
    ) -> Option<Instant> {
        self.next_loop_deadline_with_resize_poll(now, needs_render, false, include_git_refresh)
    }

    fn next_loop_deadline_with_resize_poll(
        &self,
        now: Instant,
        needs_render: bool,
        include_resize_poll: bool,
        include_git_refresh: bool,
    ) -> Option<Instant> {
        let render_deadline = if needs_render {
            self.last_render_at
                .map(|last_render_at| last_render_at + MIN_RENDER_INTERVAL)
                .filter(|deadline| *deadline > now)
        } else {
            None
        };

        [
            include_resize_poll.then_some(self.next_resize_poll),
            self.config_diagnostic_deadline,
            self.toast_deadline,
            self.state.next_pending_agent_notification_deadline(),
            self.state.next_managed_agent_deadline(),
            self.copy_feedback_deadline,
            self.next_animation_tick,
            self.resource_sample_deadline(),
            include_git_refresh
                .then(|| self.git_refresh_deadline())
                .flatten(),
            self.next_auto_update_check,
            self.next_agent_manifest_update_check,
            self.agent_metadata_deadline,
            self.pending_agent_resume_deadline,
            self.session_save_deadline,
            self.selection_autoscroll_deadline,
            self.selection_highlight_clear_deadline,
            render_deadline,
        ]
        .into_iter()
        .flatten()
        .min()
    }

    fn workspace_git_refresh_items(&self) -> Vec<WorkspaceGitRefreshItem> {
        self.state
            .workspaces
            .iter()
            .filter_map(|ws| {
                let cwd =
                    ws.resolved_identity_cwd_from(&self.state.terminals, &self.terminal_runtimes)?;
                let git_key = crate::workspace::git_status_cache_key(&cwd);
                let cache_key = git_key.unwrap_or_else(|| cwd.clone());
                Some(WorkspaceGitRefreshItem {
                    workspace_id: ws.id.clone(),
                    resolved_identity_cwd: cwd,
                    cache_key,
                })
            })
            .collect()
    }

    pub(crate) fn drain_internal_events(&mut self) -> bool {
        self.drain_internal_events_up_to(super::APP_EVENT_DRAIN_LIMIT)
    }

    pub(crate) fn drain_all_internal_events(&mut self) -> bool {
        let mut had_event = false;
        while self.drain_internal_events_up_to(super::APP_EVENT_DRAIN_LIMIT) {
            had_event = true;
        }
        had_event
    }

    fn drain_internal_events_up_to(&mut self, limit: usize) -> bool {
        let mut had_event = false;
        for _ in 0..limit {
            let Ok(ev) = self.event_rx.try_recv() else {
                break;
            };
            had_event = true;
            self.handle_internal_event_with_prefix_sync(ev);
        }
        had_event
    }
}

pub(crate) fn deduplicate_git_refresh_items(
    items: Vec<WorkspaceGitRefreshItem>,
    cache: &HashMap<std::path::PathBuf, GitStatusCacheEntry>,
) -> Vec<WorkspaceGitRefreshJob> {
    let mut indexes = HashMap::<std::path::PathBuf, usize>::new();
    let mut jobs = Vec::<WorkspaceGitRefreshJob>::new();

    for item in items {
        let target = WorkspaceGitRefreshTarget {
            workspace_id: item.workspace_id,
            resolved_identity_cwd: item.resolved_identity_cwd.clone(),
        };
        if let Some(&index) = indexes.get(&item.cache_key) {
            jobs[index].targets.push(target);
            continue;
        }

        let status_cwd = item.cache_key.clone();
        let cached = cache.get(&item.cache_key).cloned();
        indexes.insert(item.cache_key, jobs.len());
        jobs.push(WorkspaceGitRefreshJob {
            cache_key: status_cwd.clone(),
            status_cwd,
            cached,
            targets: vec![target],
        });
    }

    jobs
}

pub(crate) fn refresh_workspace_git_statuses_with_cache(
    items: Vec<WorkspaceGitRefreshItem>,
    cache: &HashMap<std::path::PathBuf, GitStatusCacheEntry>,
) -> WorkspaceGitRefreshOutput {
    let mut results = Vec::new();
    let mut cache_updates = Vec::new();

    for job in deduplicate_git_refresh_items(items, cache) {
        let (snapshot, cache_entry) =
            Workspace::git_status_snapshot_for_cwd_with_cache(&job.status_cwd, job.cached.as_ref());
        if let Some(cache_entry) = cache_entry {
            cache_updates.push((job.cache_key.clone(), cache_entry));
        }
        results.extend(job.targets.into_iter().map(move |target| {
            snapshot
                .clone()
                .into_workspace_status(target.workspace_id, target.resolved_identity_cwd)
        }));
    }

    WorkspaceGitRefreshOutput {
        results,
        cache_updates,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state;
    use crate::workspace::Workspace;
    use std::path::PathBuf;

    /// A bar whose only section is a live CPU meter.
    fn resource_bars_config() -> crate::config::ShellBarsConfig {
        let mut section = crate::config::ShellBarSectionConfig {
            kind: "fixed".to_string(),
            cells: 12,
            ..Default::default()
        };
        section.widget.kind = "resource".to_string();
        section.widget.metric = "cpu".to_string();
        crate::config::ShellBarsConfig {
            top: crate::config::ShellBarConfig {
                enabled: true,
                size: 1,
                border: false,
                color: String::new(),
                gradient: Vec::new(),
                sections: vec![section],
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn app_with_resource_section() -> super::super::App {
        let mut app = super::super::App::new(
            &crate::config::Config::default(),
            true,
            None,
            tokio::sync::mpsc::unbounded_channel().1,
            crate::api::EventHub::default(),
        );
        let config = resource_bars_config();
        app.state.workspaces.push(Workspace::test_new("test"));
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.shell_presentation = crate::ui::shell::ShellPresentationState::from_restored(
            app.state.sidebar_width,
            false,
            None,
            crate::ui::shell::ShellBars::from_config(&config),
        );
        app.state.shell_bar_chrome = crate::ui::shell::ShellBarChrome::from_config(&config, true);
        app
    }

    /// An app whose shell config is whatever the caller wants it to be.
    fn app_with_shell(shell: crate::config::ShellConfig) -> super::super::App {
        let config = crate::config::Config {
            shell,
            ..Default::default()
        };
        let mut app = super::super::App::new(
            &config,
            true,
            None,
            tokio::sync::mpsc::unbounded_channel().1,
            crate::api::EventHub::default(),
        );
        app.state.workspaces.push(Workspace::test_new("test"));
        app.state.active = Some(0);
        app.state.selected = 0;
        app
    }

    // TP-RES-16: a config that says nothing keeps the pace it always had.
    //
    // Adding a setting must not change a machine nobody touched: every file
    // written before this key existed has to go on meaning what it meant.
    #[test]
    fn a_config_that_never_mentions_the_interval_keeps_the_two_second_default() {
        let app = app_with_shell(crate::config::ShellConfig::default());
        assert_eq!(
            app.state.resource_sample_interval,
            Duration::from_secs(2),
            "the default pace moved"
        );
    }

    // TP-RES-12: a number inside the range is the pace, and it reaches the loop.
    //
    // The one test that proves the key is wired rather than merely stored. A
    // setting parsed into a field nothing reads is the quietest way for a
    // feature to be "done" and do nothing.
    #[test]
    fn an_interval_inside_the_range_is_the_pace_the_loop_keeps() {
        let shell = crate::config::ShellConfig {
            bars: resource_bars_config(),
            resource_interval_ms: 500,
            ..Default::default()
        };
        let mut app = app_with_shell(shell);
        app.state.shell_bar_chrome =
            crate::ui::shell::ShellBarChrome::from_config(&resource_bars_config(), true);

        assert_eq!(
            app.state.resource_sample_interval,
            Duration::from_millis(500)
        );

        let start = Instant::now();
        assert!(app.tick_resource_sample(start), "the first reading is owed");
        assert!(
            !app.tick_resource_sample(start + Duration::from_millis(499)),
            "a reading was taken before the configured interval had passed"
        );
        assert!(
            app.tick_resource_sample(start + Duration::from_millis(500)),
            "no reading was taken once the configured interval had passed"
        );
    }

    // TP-RES-13: a number outside the range is refused by name, and the meter
    // keeps running at the default.
    //
    // Two halves that must both hold. Refusing without saying so would leave
    // somebody reading their own file for a mistake herdr already found; and
    // answering a bad number by stopping the meter would be a worse answer than
    // a working meter and a sentence about the number.
    #[test]
    fn an_interval_outside_the_range_is_refused_and_the_default_keeps_running() {
        for millis in [
            crate::config::ShellConfig::RESOURCE_INTERVAL_MS_MIN - 1,
            crate::config::ShellConfig::RESOURCE_INTERVAL_MS_MAX + 1,
        ] {
            let shell = crate::config::ShellConfig {
                resource_interval_ms: millis,
                ..Default::default()
            };

            let problems = crate::ui::shell::shell_config_problems(&shell)
                .into_iter()
                .map(|problem| problem.to_string())
                .collect::<Vec<_>>();
            assert!(
                problems.iter().any(|problem| {
                    problem.contains("shell.resource_interval_ms")
                        && problem.contains(&millis.to_string())
                }),
                "{millis} was not refused by name: {problems:?}"
            );

            let app = app_with_shell(shell);
            assert_eq!(
                app.state.resource_sample_interval,
                Duration::from_secs(2),
                "a refused interval did not fall back to the default"
            );
        }
    }

    // TP-RES-14: the range's own edges are accepted.
    //
    // A bound stated in a message is a promise, and an off-by-one at either end
    // makes the message a lie in the one place somebody is most likely to test
    // it — right at the number the refusal just told them about.
    #[test]
    fn both_ends_of_the_accepted_range_are_accepted() {
        for millis in [
            crate::config::ShellConfig::RESOURCE_INTERVAL_MS_MIN,
            crate::config::ShellConfig::RESOURCE_INTERVAL_MS_MAX,
        ] {
            let shell = crate::config::ShellConfig {
                resource_interval_ms: millis,
                ..Default::default()
            };
            assert!(
                crate::ui::shell::shell_config_problems(&shell).is_empty(),
                "{millis} is an edge of the accepted range and was refused"
            );
            assert_eq!(
                app_with_shell(shell).state.resource_sample_interval,
                Duration::from_millis(millis)
            );
        }
    }

    // TC-D1 · the seam, stated as the only thing that can be measured about it:
    // drawing does not read the machine. A sampler called from render looks
    // identical on screen and costs a `/proc` open per frame, which is exactly
    // the shape the user's "never put load on herdr" rules out. Counting reads
    // and drawing many frames is the only way to tell the two apart.
    // TP-RES-09: the renderer never samples.
    #[test]
    fn drawing_a_live_section_many_times_never_reads_the_machine() {
        use ratatui::{backend::TestBackend, Terminal};

        let mut app = app_with_resource_section();
        let frame = ratatui::layout::Rect::new(0, 0, 80, 24);
        crate::ui::compute_view(&mut app.state, frame);

        let mut terminal =
            Terminal::new(TestBackend::new(frame.width, frame.height)).expect("test backend");
        for _ in 0..60 {
            terminal
                .draw(|f| crate::ui::render(&app.state, f))
                .expect("a bar with a live section draws");
        }

        assert_eq!(
            app.resource_samples_taken, 0,
            "sixty frames must not have read /proc even once"
        );
    }

    // TC-D2 · and the other half: something does read it, on the loop's clock,
    // and not more often than it said it would.
    #[test]
    fn the_loop_reads_the_machine_once_per_interval_and_not_per_call() {
        let mut app = app_with_resource_section();
        let start = Instant::now();

        // Nothing has been read, so the first reading is owed now.
        app.tick_resource_sample(start);
        assert_eq!(app.resource_samples_taken, 1);

        // Called again immediately — and again, and again — it refuses.
        for _ in 0..10 {
            app.tick_resource_sample(start);
        }
        assert_eq!(
            app.resource_samples_taken, 1,
            "a tick that is not due must not read"
        );

        app.tick_resource_sample(start + Duration::from_secs(2));
        assert_eq!(
            app.resource_samples_taken, 2,
            "the next interval reads once"
        );
    }

    // TC-D3 · a machine nobody is asking about is never read at all. This is
    // the difference between a feature that costs nothing when off and one that
    // samples always and throws the answer away.
    // TP-RES-08: no resource section, no deadline, no reading.
    #[test]
    fn a_bar_with_no_live_section_never_reads_the_machine_and_asks_for_no_wakeup() {
        let mut app = super::super::App::new(
            &crate::config::Config::default(),
            true,
            None,
            tokio::sync::mpsc::unbounded_channel().1,
            crate::api::EventHub::default(),
        );
        let now = Instant::now();

        assert_eq!(app.resource_sample_deadline(), None);
        app.tick_resource_sample(now);
        assert_eq!(app.resource_samples_taken, 0);
    }

    // TC-D4 · the first reading cannot produce a percentage, because a
    // percentage is a difference and there is nothing yet to differ from.
    // Reporting 0% there would be a fabricated number that looks exactly like
    // an idle machine.
    #[test]
    fn the_first_reading_leaves_the_percentage_unknown_rather_than_inventing_zero() {
        let mut app = app_with_resource_section();
        let start = Instant::now();

        app.tick_resource_sample(start);
        assert_eq!(
            app.state.resources.cpu, None,
            "one reading is a baseline, not a measurement"
        );

        // Memory needs no baseline, so on a platform with a reader the very
        // first reading already has it. This is also the only test that proves
        // the platform read works at all rather than silently returning None —
        // every other one feeds the parsers fixtures.
        #[cfg(target_os = "linux")]
        assert!(
            app.state.resources.mem.is_some_and(|mem| mem.total > 0),
            "a Linux box has readable memory: {:?}",
            app.state.resources.mem
        );
    }

    #[test]
    fn interrupted_custom_command_wait_keeps_child_for_retry() {
        let interrupted = std::io::Error::new(std::io::ErrorKind::Interrupted, "test interrupt");

        assert!(retain_custom_command_after_wait(42, Err(interrupted)));
    }

    // TP-TAB-FLASH-03: the flash exists only if frames keep coming. The
    // headless renderer draws on animation ticks; without this coupling the
    // timer never arms (no working pane), no tick arrives, and the flash is
    // simply never drawn even though every phase computation is correct.
    #[test]
    fn a_fresh_tabs_flash_window_keeps_the_animation_timer_alive() {
        let (mut app, _pane) = test_app_with_pane();
        let now = Instant::now();

        app.sync_animation_timer(now);
        assert!(
            app.next_animation_tick.is_none(),
            "control: no working pane and no flash means no timer"
        );

        app.state.workspaces[0].tabs[0].spawned_at = Some(now);
        app.sync_animation_timer(now);
        assert!(
            app.next_animation_tick.is_some(),
            "an open flash window must drive frames"
        );

        app.state.workspaces[0].tabs[0].spawned_at = now.checked_sub(Duration::from_secs(3));
        app.next_animation_tick = None;
        app.sync_animation_timer(now);
        assert!(
            app.next_animation_tick.is_none(),
            "a closed flash window must stop driving frames"
        );
    }

    fn test_app_with_pane() -> (super::super::App, crate::layout::PaneId) {
        let mut app = super::super::App::new(
            &crate::config::Config::default(),
            true,
            None,
            tokio::sync::mpsc::unbounded_channel().1,
            crate::api::EventHub::default(),
        );
        let ws = Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        app.state.workspaces.push(ws);
        app.state.active = Some(0);
        app.state.view.pane_infos.push(crate::layout::PaneInfo {
            id: pane_id,
            rect: ratatui::layout::Rect::new(0, 0, 80, 24),
            inner_rect: ratatui::layout::Rect::new(0, 0, 80, 24),
            scrollbar_rect: None,
            borders: ratatui::widgets::Borders::NONE,
            is_focused: true,
        });
        (app, pane_id)
    }

    #[test]
    fn git_refresh_deduplicates_workspaces_with_same_cache_key() {
        let repo =
            std::env::temp_dir().join(format!("herdr-git-refresh-dedupe-{}", std::process::id()));
        let nested = repo.join("nested");
        let other = repo.join("other");
        std::fs::create_dir_all(&nested).expect("create nested dir");
        std::fs::create_dir_all(&other).expect("create other dir");
        std::process::Command::new("git")
            .arg("-C")
            .arg(&repo)
            .arg("init")
            .output()
            .expect("run git init");

        let output = refresh_workspace_git_statuses_with_cache(
            vec![
                WorkspaceGitRefreshItem {
                    workspace_id: "one".into(),
                    resolved_identity_cwd: nested.clone(),
                    cache_key: repo.clone(),
                },
                WorkspaceGitRefreshItem {
                    workspace_id: "two".into(),
                    resolved_identity_cwd: other.clone(),
                    cache_key: repo.clone(),
                },
            ],
            &HashMap::new(),
        );

        assert_eq!(output.cache_updates.len(), 1);
        assert_eq!(output.cache_updates[0].0, repo);
        assert_eq!(output.results.len(), 2);
        assert_eq!(output.results[0].workspace_id, "one");
        assert_eq!(
            output.results[0].resolved_identity_cwd,
            PathBuf::from(&nested)
        );
        assert_eq!(output.results[1].workspace_id, "two");
        assert_eq!(
            output.results[1].resolved_identity_cwd,
            PathBuf::from(&other)
        );

        let _ = std::fs::remove_dir_all(repo);
    }

    #[test]
    fn git_refresh_items_use_cwd_cache_key_for_non_git_cwd() {
        let mut app = super::super::App::new(
            &crate::config::Config::default(),
            true,
            None,
            tokio::sync::mpsc::unbounded_channel().1,
            crate::api::EventHub::default(),
        );
        let cwd = std::env::temp_dir().join(format!("herdr-non-git-cwd-{}", std::process::id()));
        std::fs::create_dir_all(&cwd).expect("create temp cwd");
        let mut ws = Workspace::test_new("test");
        ws.identity_cwd = cwd.clone();
        ws.tabs.clear();
        app.state.workspaces.push(ws);

        let items = app.workspace_git_refresh_items();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].cache_key, cwd);
        let _ = std::fs::remove_dir_all(&cwd);
    }

    #[test]
    fn headless_deadline_can_suppress_git_refresh_timer() {
        let mut app = super::super::App::new(
            &crate::config::Config::default(),
            true,
            None,
            tokio::sync::mpsc::unbounded_channel().1,
            crate::api::EventHub::default(),
        );
        app.state.workspaces.push(Workspace::test_new("test"));
        let now = Instant::now();
        app.last_git_remote_status_refresh = now - super::super::GIT_REMOTE_STATUS_REFRESH_INTERVAL;

        assert_eq!(
            app.next_headless_loop_deadline_with_git_refresh(now, false, false),
            None
        );
        assert_eq!(
            app.next_headless_loop_deadline_with_git_refresh(now, false, true),
            Some(now)
        );
    }

    #[test]
    fn git_refresh_due_request_survives_in_flight_refresh() {
        let mut app = super::super::App::new(
            &crate::config::Config::default(),
            true,
            None,
            tokio::sync::mpsc::unbounded_channel().1,
            crate::api::EventHub::default(),
        );
        let now = Instant::now();
        app.git_refresh_in_flight = true;

        app.mark_git_status_refresh_due(now);
        assert!(app.git_refresh_due_after_in_flight);

        app.handle_internal_event(crate::events::AppEvent::GitStatusRefreshed {
            results: Vec::new(),
            cache_updates: Vec::new(),
        });

        assert!(!app.git_refresh_in_flight);
        assert!(!app.git_refresh_due_after_in_flight);
        assert_eq!(app.git_refresh_deadline(), None);

        app.state.workspaces.push(Workspace::test_new("test"));
        let deadline = app
            .git_refresh_deadline()
            .expect("refresh should be due once a workspace exists");
        assert!(deadline <= Instant::now());
    }

    #[test]
    fn tick_selection_autoscroll_stops_when_metrics_unavailable() {
        // Without a runtime, pane_scroll_metrics returns None.
        // Fail-closed: stop autoscroll instead of rescheduling forever.
        let (mut app, pane_id) = test_app_with_pane();
        let now = Instant::now();
        let mut sel = crate::selection::Selection::anchor(pane_id, 0, 0, None);
        // Drag to a different cell so it becomes Dragging
        sel.drag(5, 5, ratatui::layout::Rect::new(0, 0, 80, 24), None);
        app.state.selection = Some(sel);
        app.state.selection_autoscroll = Some(state::SelectionAutoscroll {
            direction: state::SelectionAutoscrollDirection::Down,
            last_mouse_screen_col: 5,
            last_mouse_screen_row: 23,
            inner_rect: ratatui::layout::Rect::new(0, 0, 80, 24),
        });
        app.selection_autoscroll_deadline = Some(now);
        app.tick_selection_autoscroll(now);
        // Should stop because no runtime metrics available
        assert!(app.state.selection_autoscroll.is_none());
        assert!(app.selection_autoscroll_deadline.is_none());
    }

    #[test]
    fn tick_selection_autoscroll_stops_when_selection_done() {
        let (mut app, pane_id) = test_app_with_pane();
        let now = Instant::now();
        // Create a selection that is already finished (not in progress)
        let mut sel = crate::selection::Selection::anchor(pane_id, 0, 0, None);
        // Drag to a different cell so it becomes visible, then finish
        sel.drag(5, 5, ratatui::layout::Rect::new(0, 0, 80, 24), None);
        sel.finish(); // now it's Done, not in progress
        app.state.selection = Some(sel);
        app.state.selection_autoscroll = Some(state::SelectionAutoscroll {
            direction: state::SelectionAutoscrollDirection::Down,
            last_mouse_screen_col: 0,
            last_mouse_screen_row: 23,
            inner_rect: ratatui::layout::Rect::new(0, 0, 80, 24),
        });
        app.selection_autoscroll_deadline = Some(now);
        app.tick_selection_autoscroll(now);
        assert!(app.state.selection_autoscroll.is_none());
        assert!(app.selection_autoscroll_deadline.is_none());
    }

    #[test]
    fn tick_selection_autoscroll_stops_when_selection_cleared() {
        let (mut app, _pane_id) = test_app_with_pane();
        let now = Instant::now();
        app.state.selection = None;
        app.state.selection_autoscroll = Some(state::SelectionAutoscroll {
            direction: state::SelectionAutoscrollDirection::Down,
            last_mouse_screen_col: 0,
            last_mouse_screen_row: 23,
            inner_rect: ratatui::layout::Rect::new(0, 0, 80, 24),
        });
        app.selection_autoscroll_deadline = Some(now);
        app.tick_selection_autoscroll(now);
        assert!(app.state.selection_autoscroll.is_none());
        assert!(app.selection_autoscroll_deadline.is_none());
    }

    #[test]
    fn tick_selection_autoscroll_stops_when_selection_anchored() {
        // Anchored (click, no drag) should not keep the timer running.
        let (mut app, pane_id) = test_app_with_pane();
        let now = Instant::now();
        app.state.selection = Some(crate::selection::Selection::anchor(pane_id, 0, 0, None));
        app.state.selection_autoscroll = Some(state::SelectionAutoscroll {
            direction: state::SelectionAutoscrollDirection::Down,
            last_mouse_screen_col: 0,
            last_mouse_screen_row: 23,
            inner_rect: ratatui::layout::Rect::new(0, 0, 80, 24),
        });
        app.selection_autoscroll_deadline = Some(now);
        app.tick_selection_autoscroll(now);
        assert!(app.state.selection_autoscroll.is_none());
        assert!(app.selection_autoscroll_deadline.is_none());
    }

    /// Creates an app with a real TerminalRuntime (no PTY) so scroll_metrics
    /// returns meaningful data. Uses test_with_scrollback_bytes.
    fn test_app_with_runtime(
        cols: u16,
        rows: u16,
        bytes: &[u8],
    ) -> (super::super::App, crate::layout::PaneId) {
        let mut app = super::super::App::new(
            &crate::config::Config::default(),
            true,
            None,
            tokio::sync::mpsc::unbounded_channel().1,
            crate::api::EventHub::default(),
        );
        let mut ws = Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        let runtime =
            crate::terminal::TerminalRuntime::test_with_scrollback_bytes(cols, rows, 0, bytes);
        ws.tabs[0].runtimes.insert(pane_id, runtime);
        app.state.workspaces.push(ws);
        app.state.active = Some(0);
        app.state.view.pane_infos.push(crate::layout::PaneInfo {
            id: pane_id,
            rect: ratatui::layout::Rect::new(0, 0, cols, rows),
            inner_rect: ratatui::layout::Rect::new(0, 0, cols, rows),
            scrollbar_rect: None,
            borders: ratatui::widgets::Borders::NONE,
            is_focused: true,
        });
        (app, pane_id)
    }

    #[tokio::test]
    async fn tick_selection_autoscroll_stops_at_scrollback_top() {
        // Create a runtime with no scrollback content — we're already at
        // the top (offset_from_bottom == max_offset_from_bottom).
        let (mut app, pane_id) = test_app_with_runtime(80, 24, &[]);
        let now = Instant::now();
        let mut sel = crate::selection::Selection::anchor(pane_id, 5, 5, None);
        sel.drag(0, 0, ratatui::layout::Rect::new(0, 0, 80, 24), None);
        app.state.selection = Some(sel);
        app.state.selection_autoscroll = Some(state::SelectionAutoscroll {
            direction: state::SelectionAutoscrollDirection::Up,
            last_mouse_screen_col: 0,
            last_mouse_screen_row: 0,
            inner_rect: ratatui::layout::Rect::new(0, 0, 80, 24),
        });
        app.selection_autoscroll_deadline = Some(now);
        app.tick_selection_autoscroll(now);
        // At scrollback top, can't scroll further up — should stop
        assert!(app.state.selection_autoscroll.is_none());
        assert!(app.selection_autoscroll_deadline.is_none());
    }

    #[tokio::test]
    async fn tick_selection_autoscroll_stops_at_scrollback_bottom() {
        // Create a runtime with no scrollback content — we're already at
        // the bottom (offset_from_bottom == 0).
        let (mut app, pane_id) = test_app_with_runtime(80, 24, &[]);
        let now = Instant::now();
        let mut sel = crate::selection::Selection::anchor(pane_id, 0, 0, None);
        sel.drag(5, 5, ratatui::layout::Rect::new(0, 0, 80, 24), None);
        app.state.selection = Some(sel);
        app.state.selection_autoscroll = Some(state::SelectionAutoscroll {
            direction: state::SelectionAutoscrollDirection::Down,
            last_mouse_screen_col: 5,
            last_mouse_screen_row: 23,
            inner_rect: ratatui::layout::Rect::new(0, 0, 80, 24),
        });
        app.selection_autoscroll_deadline = Some(now);
        app.tick_selection_autoscroll(now);
        // At scrollback bottom, can't scroll further down — should stop
        assert!(app.state.selection_autoscroll.is_none());
        assert!(app.selection_autoscroll_deadline.is_none());
    }

    #[tokio::test]
    async fn raw_input_batch_does_not_start_pending_agent_resume_before_render() {
        let (mut app, pane_id) = test_app_with_pane();
        app.state.ensure_test_terminals();
        let terminal_id = app.state.workspaces[0]
            .terminal_id(pane_id)
            .cloned()
            .expect("test pane should have a terminal");
        app.state
            .terminals
            .get_mut(&terminal_id)
            .expect("test terminal should exist")
            .pending_agent_resume_plan = Some(crate::agent_resume::AgentResumePlan {
            agent: "codex".into(),
            argv: vec!["/bin/sh".into(), "-c".into(), "sleep 5".into()],
            dedupe_key: "herdr:codex\0codex\0Id\0codex-session".into(),
        });

        assert!(
            app.handle_raw_input_batch(crate::raw_input::RawInputEvent::HostDefaultColor {
                kind: crate::terminal_theme::DefaultColorKind::Foreground,
                color: crate::terminal_theme::RgbColor {
                    r: 220,
                    g: 220,
                    b: 220,
                },
            })
            .await
        );
        assert!(
            app.terminal_runtimes.get(&terminal_id).is_none(),
            "raw input can mutate active geometry; pending resumes must wait for render to refresh pane_infos"
        );
        assert!(app
            .state
            .terminals
            .get(&terminal_id)
            .expect("test terminal should still exist")
            .pending_agent_resume_plan
            .is_some());
    }

    #[tokio::test]
    async fn scheduled_tasks_do_not_start_pending_agent_resume_when_geometry_dirty() {
        let (mut app, pane_id) = test_app_with_pane();
        app.state.ensure_test_terminals();
        app.state.host_terminal_theme = crate::terminal_theme::TerminalTheme {
            foreground: Some(crate::terminal_theme::RgbColor {
                r: 220,
                g: 220,
                b: 220,
            }),
            background: Some(crate::terminal_theme::RgbColor {
                r: 20,
                g: 20,
                b: 20,
            }),
            ..Default::default()
        };
        let terminal_id = app.state.workspaces[0]
            .terminal_id(pane_id)
            .cloned()
            .expect("test pane should have a terminal");
        app.state
            .terminals
            .get_mut(&terminal_id)
            .expect("test terminal should exist")
            .pending_agent_resume_plan = Some(crate::agent_resume::AgentResumePlan {
            agent: "codex".into(),
            argv: vec!["/bin/sh".into(), "-c".into(), "sleep 5".into()],
            dedupe_key: "herdr:codex\0codex\0Id\0codex-session".into(),
        });
        app.pending_agent_resume_deadline = Some(Instant::now() - Duration::from_millis(1));

        assert!(!app.handle_scheduled_tasks(Instant::now(), true));
        assert!(app.terminal_runtimes.get(&terminal_id).is_none());
        assert!(app
            .state
            .terminals
            .get(&terminal_id)
            .expect("test terminal should still exist")
            .pending_agent_resume_plan
            .is_some());
        assert!(app.pending_agent_resume_deadline.is_none());
    }
}

#[cfg(test)]
mod per_display_worker_tests {
    // TP-SUR-FM-02
    #[test]
    fn scheduled_work_visits_each_display_in_its_own_view() {
        let mut state = crate::app::state::AppState::test_new();
        state
            .workspaces
            .push(crate::workspace::Workspace::test_new("only"));
        state.active = Some(0);

        // No display has been served: the session is the whole list, which is
        // every monolithic run and every test that never attaches one.
        assert_eq!(state.displays_to_serve(), vec![None]);

        state.enter_viewer(Some(7));
        state.restore_viewer(None);
        state.enter_viewer(Some(3));
        state.restore_viewer(None);

        // Stable order, so which display wins a race does not change between
        // runs the way hash order would.
        assert_eq!(state.displays_to_serve(), vec![Some(3), Some(7)]);

        state.forget_client(3);
        assert_eq!(state.displays_to_serve(), vec![Some(7)]);
    }
}
