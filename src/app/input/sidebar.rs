use ratatui::layout::Rect;

use crate::app::state::{AppState, SidebarTab, ViewLayout};

use super::ScrollbarClickTarget;

impl AppState {
    // TP-CHROME-14: hit tests read the same chrome the renderer drew through.
    pub(super) fn workspace_list_rect(&self) -> Rect {
        let sidebar = self.view.sidebar_rect;
        if self.sidebar_collapsed || sidebar.width <= 1 || sidebar.height == 0 {
            return Rect::default();
        }
        // The frame is drawn out of this same rectangle, so the hit test has to
        // read the same chrome the renderer did -- passing NONE here would put
        // every row one cell off the row the user can see.
        crate::ui::workspace_list_rect(sidebar, self.sidebar_section_split, self.sidebar_chrome)
    }

    pub(super) fn agent_panel_rect(&self) -> Rect {
        let sidebar = self.view.sidebar_rect;
        if self.sidebar_collapsed || sidebar.width <= 1 || sidebar.height == 0 {
            return Rect::default();
        }
        let (_, detail_area) = crate::ui::expanded_sidebar_sections(
            sidebar,
            self.sidebar_section_split,
            self.sidebar_chrome,
        );
        detail_area
    }

    pub(super) fn workspace_list_scrollbar_target_at(
        &self,
        col: u16,
        row: u16,
    ) -> Option<ScrollbarClickTarget> {
        let area = self.workspace_list_rect();
        let metrics = crate::ui::workspace_list_scroll_metrics(self, area);
        let track = crate::ui::workspace_list_scrollbar_rect(self, area)?;
        if col < track.x
            || col >= track.x + track.width
            || row < track.y
            || row >= track.y + track.height
        {
            return None;
        }
        if let Some(grab_row_offset) = crate::ui::scrollbar_thumb_grab_offset(metrics, track, row) {
            Some(ScrollbarClickTarget::Thumb { grab_row_offset })
        } else {
            Some(ScrollbarClickTarget::Track {
                offset_from_bottom: crate::ui::scrollbar_offset_from_row(metrics, track, row),
            })
        }
    }

    pub(super) fn workspace_list_offset_for_drag_row(
        &self,
        row: u16,
        grab_row_offset: u16,
    ) -> Option<usize> {
        let area = self.workspace_list_rect();
        let metrics = crate::ui::workspace_list_scroll_metrics(self, area);
        let track = crate::ui::workspace_list_scrollbar_rect(self, area)?;
        Some(crate::ui::scrollbar_offset_from_drag_row(
            metrics,
            track,
            row,
            grab_row_offset,
        ))
    }

    pub(super) fn set_workspace_list_offset_from_bottom(&mut self, offset_from_bottom: usize) {
        let area = self.workspace_list_rect();
        let metrics = crate::ui::workspace_list_scroll_metrics(self, area);
        self.workspace_scroll = metrics
            .max_offset_from_bottom
            .saturating_sub(offset_from_bottom);
        self.workspace_scroll = crate::ui::normalized_workspace_scroll(
            self,
            self.view.sidebar_rect,
            self.workspace_scroll,
        );
    }

    pub(super) fn scroll_workspace_list(&mut self, delta: i16) {
        if delta.is_negative() {
            self.workspace_scroll = self
                .workspace_scroll
                .saturating_sub(delta.unsigned_abs() as usize);
            self.workspace_scroll = crate::ui::normalized_workspace_scroll(
                self,
                self.view.sidebar_rect,
                self.workspace_scroll,
            );
            return;
        }

        let area = self.workspace_list_rect();
        let metrics = crate::ui::workspace_list_scroll_metrics(self, area);
        self.workspace_scroll = self
            .workspace_scroll
            .saturating_add(delta as usize)
            .min(metrics.max_offset_from_bottom);
        self.workspace_scroll = crate::ui::normalized_workspace_scroll(
            self,
            self.view.sidebar_rect,
            self.workspace_scroll,
        );
    }

    pub(super) fn agent_panel_scrollbar_target_at(
        &self,
        col: u16,
        row: u16,
    ) -> Option<ScrollbarClickTarget> {
        let area = self.agent_panel_rect();
        let metrics = crate::ui::agent_panel_scroll_metrics(self, area);
        let track = crate::ui::agent_panel_scrollbar_rect(self, area)?;
        if col < track.x
            || col >= track.x + track.width
            || row < track.y
            || row >= track.y + track.height
        {
            return None;
        }
        if let Some(grab_row_offset) = crate::ui::scrollbar_thumb_grab_offset(metrics, track, row) {
            Some(ScrollbarClickTarget::Thumb { grab_row_offset })
        } else {
            Some(ScrollbarClickTarget::Track {
                offset_from_bottom: crate::ui::scrollbar_offset_from_row(metrics, track, row),
            })
        }
    }

    pub(super) fn agent_panel_offset_for_drag_row(
        &self,
        row: u16,
        grab_row_offset: u16,
    ) -> Option<usize> {
        let area = self.agent_panel_rect();
        let metrics = crate::ui::agent_panel_scroll_metrics(self, area);
        let track = crate::ui::agent_panel_scrollbar_rect(self, area)?;
        Some(crate::ui::scrollbar_offset_from_drag_row(
            metrics,
            track,
            row,
            grab_row_offset,
        ))
    }

    pub(super) fn set_agent_panel_offset_from_bottom(&mut self, offset_from_bottom: usize) {
        let area = self.agent_panel_rect();
        let metrics = crate::ui::agent_panel_scroll_metrics(self, area);
        self.agent_panel_scroll = metrics
            .max_offset_from_bottom
            .saturating_sub(offset_from_bottom);
    }

    pub(super) fn scroll_agent_panel(&mut self, delta: i16) {
        let area = self.agent_panel_rect();
        let max_scroll = crate::ui::agent_panel_scroll_metrics(self, area).max_offset_from_bottom;
        if delta.is_negative() {
            self.agent_panel_scroll = self
                .agent_panel_scroll
                .saturating_sub(delta.unsigned_abs() as usize);
        } else {
            self.agent_panel_scroll = self
                .agent_panel_scroll
                .saturating_add(delta as usize)
                .min(max_scroll);
        }
    }

    pub(super) fn scroll_projects_list(&mut self, delta: i16) {
        let area = self.workspace_list_rect();
        let max_scroll = crate::ui::projects_scroll_metrics(self, area).max_offset_from_bottom;
        if delta.is_negative() {
            self.projects_scroll = self
                .projects_scroll
                .saturating_sub(delta.unsigned_abs() as usize);
        } else {
            self.projects_scroll = self
                .projects_scroll
                .saturating_add(delta as usize)
                .min(max_scroll);
        }
    }

    pub(super) fn projects_scrollbar_target_at(
        &self,
        col: u16,
        row: u16,
    ) -> Option<ScrollbarClickTarget> {
        let area = self.workspace_list_rect();
        let metrics = crate::ui::projects_scroll_metrics(self, area);
        let track = crate::ui::projects_scrollbar_rect(self, area)?;
        if col < track.x
            || col >= track.x + track.width
            || row < track.y
            || row >= track.y + track.height
        {
            return None;
        }
        if let Some(grab_row_offset) = crate::ui::scrollbar_thumb_grab_offset(metrics, track, row) {
            Some(ScrollbarClickTarget::Thumb { grab_row_offset })
        } else {
            Some(ScrollbarClickTarget::Track {
                offset_from_bottom: crate::ui::scrollbar_offset_from_row(metrics, track, row),
            })
        }
    }

    pub(super) fn projects_offset_for_drag_row(
        &self,
        row: u16,
        grab_row_offset: u16,
    ) -> Option<usize> {
        let area = self.workspace_list_rect();
        let metrics = crate::ui::projects_scroll_metrics(self, area);
        let track = crate::ui::projects_scrollbar_rect(self, area)?;
        Some(crate::ui::scrollbar_offset_from_drag_row(
            metrics,
            track,
            row,
            grab_row_offset,
        ))
    }

    pub(super) fn set_projects_offset_from_bottom(&mut self, offset_from_bottom: usize) {
        let area = self.workspace_list_rect();
        let metrics = crate::ui::projects_scroll_metrics(self, area);
        self.projects_scroll = metrics
            .max_offset_from_bottom
            .saturating_sub(offset_from_bottom);
    }

    pub(crate) fn sidebar_footer_rect(&self) -> Rect {
        let ws_area = self.workspace_list_rect();
        if ws_area == Rect::default() {
            return Rect::default();
        }
        // Chips are frames, so the footer is as tall as `footer_rows` says --
        // never a locally chosen number, or the list and the buttons would
        // disagree about where one ends and the other begins.
        let rows = self.sidebar_chrome.footer_rows().min(ws_area.height);
        let y = ws_area.y + ws_area.height.saturating_sub(rows);
        Rect::new(ws_area.x, y, ws_area.width, rows)
    }

    pub(crate) fn sidebar_new_button_rect(&self) -> Rect {
        let footer = self.sidebar_footer_rect();
        let width = self
            .sidebar_chrome
            .control_width(5, "new")
            .min(footer.width.max(1));
        Rect::new(footer.x, footer.y, width, footer.height)
    }

    /// The footer "actives" toggle, centered between the chat and menu
    /// buttons; collapses to an empty rect when the footer is too narrow to
    /// keep all three hit areas disjoint.
    pub(crate) fn sidebar_actives_toggle_rect(&self) -> Rect {
        self.sidebar_filter_toggle_rect(7, "actives")
    }

    /// The Spaces tab's own filter toggle, in the same footer slot the
    /// Projects tab keeps its "actives" in (TP-FOCUS-SW-04). One slot, one
    /// meaning — "narrow the list above me" — and the tab decides which list
    /// that is; a second control position for the same idea would read as a
    /// second idea.
    pub(crate) fn sidebar_focus_toggle_rect(&self) -> Rect {
        self.sidebar_filter_toggle_rect(5, "focus")
    }

    /// Shared geometry for the footer's filter toggle: centered between the
    /// chat and menu buttons, collapsing to an empty rect when the footer is
    /// too narrow to keep all three hit areas disjoint.
    fn sidebar_filter_toggle_rect(&self, cells: u16, label: &str) -> Rect {
        let footer = self.sidebar_footer_rect();
        if footer.width == 0 || footer.height == 0 {
            return Rect::default();
        }
        let chat = self.sidebar_new_button_rect();
        let menu = self.global_launcher_rect();
        let label_w = self.sidebar_chrome.control_width(cells, label);
        let left = chat.x + chat.width + 1;
        let right = menu.x.saturating_sub(1);
        if right <= left || right - left < label_w {
            return Rect::default();
        }
        let x = left + (right - left - label_w) / 2;
        Rect::new(x, footer.y, label_w, footer.height)
    }

    pub(crate) fn global_launcher_rect(&self) -> Rect {
        if self.view.layout == ViewLayout::Mobile {
            // The global menu lives at the foot of the spaces drawer, which the
            // left header button opens.
            return self.view.mobile_header_hits.spaces_menu;
        }

        let footer = self.sidebar_footer_rect();
        let label_cells = if self.global_menu_attention_badge_visible() {
            8
        } else {
            6
        };
        let width = self
            .sidebar_chrome
            .control_width(label_cells, "menu")
            .min(footer.width.max(1));
        let x = footer.x + footer.width.saturating_sub(width);
        Rect::new(x, footer.y, width, footer.height)
    }

    /// The Projects-tab row (if any) whose laid-out rect contains `(col, row)`.
    ///
    /// TP-PROJTAB-01: refuses the hit entirely when the session poll rewrote
    /// `projects_sessions` after these rects were laid out. The rects carry
    /// indices into the list they were computed against; resolving them
    /// against a newer list opens whichever chat *now* holds the index — a
    /// chat in the wrong project directory. A no-op click is honest; the next
    /// frame lays out fresh rects.
    pub(super) fn project_row_kind_at(
        &self,
        col: u16,
        row: u16,
    ) -> Option<crate::app::state::ProjectRowKind> {
        if self.view.project_rows_generation != self.projects_sessions_generation {
            return None;
        }
        self.view
            .project_row_areas
            .iter()
            .find(|area| {
                row == area.rect.y && col >= area.rect.x && col < area.rect.x + area.rect.width
            })
            .map(|area| area.kind)
    }

    /// The workspace whose identity cwd matches the project's directory —
    /// worktree actions launched from the Projects tab act on that workspace,
    /// mirroring the Spaces context menu.
    pub(crate) fn project_workspace_index(&self, proj_idx: usize) -> Option<usize> {
        let project = self.projects_sessions.get(proj_idx)?;
        self.workspaces
            .iter()
            .position(|ws| ws.identity_cwd == project.path)
    }

    /// Open the new-chat agent selector for `proj_idx` at `(x, y)`, with the
    /// current default agent highlighted. When the project is also open as a
    /// workspace the menu grows that workspace's worktree actions.
    /// Open a chat remembered under a workspace.
    ///
    /// Same contract the Projects tab uses, so one click means one thing on
    /// both surfaces: a chat already wired to a live tab is FOCUSED, never
    /// resumed a second time — resuming it again would spawn a duplicate
    /// process against the same transcript.
    pub(crate) fn open_workspace_chat(&mut self, ws_idx: usize, session_id: &str) {
        let Some(workspace) = self.workspaces.get(ws_idx) else {
            return;
        };
        let project_path = workspace.identity_cwd.clone();
        let key = crate::persist::workspace_chats::ledger_key(&project_path);
        // The row rects carry the chat's identity, never its position: the
        // ledger can gain a row between two frames, and a stale index used to
        // open whichever chat had shifted into the slot. A chat that left the
        // ledger since the frame was drawn answers with nothing.
        if !self
            .workspace_chat_rows
            .get(&key)
            .is_some_and(|rows| rows.iter().any(|row| row.session_id == session_id))
        {
            return;
        }
        let session_id = session_id.to_string();

        if let Some((live_ws, live_tab)) = self.find_resumed_chat_tab(&session_id) {
            self.switch_workspace_tab(live_ws, live_tab);
            self.mode = crate::app::Mode::Terminal;
            return;
        }
        self.request_project_chat_tab = Some(crate::app::state::ProjectChatTabRequest {
            project_path,
            session_id: Some(session_id),
        });
    }

    /// Open a chat from the daily section — the sibling of
    /// [`AppState::open_workspace_chat`], and deliberately the same contract:
    /// a session already running in a tab is switched to rather than resumed
    /// a second time, and only a session with nowhere to land is queued.
    ///
    /// TP-DAILY-07: the queued request is rooted at the daily directory, not
    /// at whatever workspace happens to be active — that substitution is
    /// exactly how #46 opened an agent in `$HOME` instead of the checkout,
    /// with the roles reversed.
    /// Take a press on a container's chat row where it can actually go.
    ///
    /// TP-CHAT-MOVE-07: a live chat is switched to, exactly as every other
    /// chat row does. A chat that is *not* running does nothing here, and that
    /// is deliberate rather than unfinished: a declared container has no
    /// directory, and the ledger records where a chat was moved *to*, never
    /// where it came from — so there is no honest cwd to resume it into.
    /// Guessing one would resume the conversation in whatever checkout
    /// happened to be active, which is #46 with the roles reversed. Giving a
    /// container a directory is what unlocks the rest, and that is its own
    /// piece of work.
    pub(crate) fn open_module_chat(&mut self, node_key: &str, session_id: &str) {
        let key = crate::persist::workspace_chats::module_ledger_key(node_key);
        // Identity, not position — the contract every chat row shares now. A
        // chat that left this container since the frame was drawn is a no-op.
        if !self
            .workspace_chat_rows
            .get(&key)
            .is_some_and(|rows| rows.iter().any(|row| row.session_id == session_id))
        {
            return;
        }
        let session_id = session_id.to_string();
        if let Some((live_ws, live_tab)) = self.find_resumed_chat_tab(&session_id) {
            self.switch_workspace_tab(live_ws, live_tab);
            self.mode = crate::app::Mode::Terminal;
            return;
        }
        // TP-CHAT-MOVE-10: a dead chat filed into a module reopens in that
        // module's own directory — the boundary TP-CHAT-MOVE-07 drew, opened
        // from the side it was always meant to open from.
        //
        // A container without a directory still refuses. That refusal is the
        // whole point: the ledger records which module a chat belongs to, not
        // where it came from, so resuming without a stated directory would
        // mean inventing one — and #46 measured exactly where invented
        // directories land ($HOME). The directory is now a fact the person
        // stated (TP-MOD-33), which is what makes reopening safe.
        // TP-MOD-36: resolved through the one definition, so a chat filed into
        // a BUCKET reopens too. Reading `space_nodes` directly answered "no
        // directory" for every bucket, and buckets are twenty of the
        // twenty-four modules on the machine this was reported from — the move
        // would have succeeded and the chat would then have been unreachable.
        let Some(dir) = self.module_directory_for_key(node_key) else {
            return;
        };
        // Checked again here, not just when it was written: a directory can be
        // removed after the fact (a worktree pruned, a disk unmounted), and
        // opening a pane the shell cannot enter reads as the chat being broken
        // rather than the target being gone.
        if !dir.is_dir() {
            tracing::warn!(dir = %dir.display(), "module directory is gone; refusing to resume");
            return;
        }
        self.request_project_chat_tab = Some(crate::app::state::ProjectChatTabRequest {
            project_path: dir,
            session_id: Some(session_id),
        });
    }

    pub(crate) fn open_daily_chat(&mut self, session_id: &str) {
        let Some(project_path) = self.daily_chat_cwd.clone() else {
            return;
        };
        let key = crate::persist::workspace_chats::ledger_key(&project_path);
        // Identity, not position: the list can refresh between the frame a
        // person clicked and the click arriving, and an index resolved
        // against the new list used to answer with the WRONG chat, not with
        // nothing. The drawn row's own session id cannot mis-resolve; a chat
        // that left the ledger answers with nothing.
        if !self
            .workspace_chat_rows
            .get(&key)
            .is_some_and(|rows| rows.iter().any(|row| row.session_id == session_id))
        {
            return;
        }
        let session_id = session_id.to_string();

        if let Some((live_ws, live_tab)) = self.find_resumed_chat_tab(&session_id) {
            self.switch_workspace_tab(live_ws, live_tab);
            self.mode = crate::app::Mode::Terminal;
            return;
        }
        self.request_project_chat_tab = Some(crate::app::state::ProjectChatTabRequest {
            project_path,
            session_id: Some(session_id),
        });
    }

    /// Start a fresh chat rooted at the daily directory.
    ///
    /// TP-DAILY-11: the sibling of [`AppState::request_workspace_chat`], and
    /// rooted the same deliberate way [`AppState::open_daily_chat`] is — at
    /// the daily directory, never at whatever workspace happens to be active.
    /// A client with no home directory asks for nothing rather than for `/`.
    pub(crate) fn request_daily_chat(&mut self) {
        let Some(project_path) = self.daily_chat_cwd.clone() else {
            return;
        };
        self.request_project_chat_tab = Some(crate::app::state::ProjectChatTabRequest {
            project_path,
            session_id: None,
        });
    }

    /// Open the daily area's header menu (TP-DAILY-12) — the one door both
    /// the "⋯" and the right-click walk, so they can never drift apart.
    pub(crate) fn open_daily_header_menu(&mut self, x: u16, y: u16) {
        // TP-DAILY-19: two or more interchangeable workspaces is the whole
        // condition for offering the merge. Computed here, at open time, from
        // the core set rather than from the drawn rows — the verb has to be
        // offered on the same grounds whether the section is folded or not.
        let has_mergeable = self.mergeable_daily_workspaces().len() >= 2;
        self.context_menu = Some(crate::app::state::ContextMenuState {
            kind: crate::app::state::ContextMenuKind::DailyHeader {
                collapsed: self.daily_section_collapsed,
                has_mergeable,
            },
            x,
            y,
            list: crate::app::state::MenuListState::new(0),
        });
        self.enter_overlay_mode(crate::app::Mode::ContextMenu);
    }

    /// Open the tree's empty-space menu (TP-MOD-31).
    pub(crate) fn open_sidebar_blank_menu(&mut self, x: u16, y: u16) {
        self.context_menu = Some(crate::app::state::ContextMenuState {
            kind: crate::app::state::ContextMenuKind::SidebarBlank,
            x,
            y,
            list: crate::app::state::MenuListState::new(0),
        });
        self.enter_overlay_mode(crate::app::Mode::ContextMenu);
    }

    /// Open the daily section's "+" menu: the agents, and nothing else.
    pub(crate) fn open_daily_new_chat_menu(&mut self, x: u16, y: u16) {
        let highlighted = crate::app::projects::CHAT_AGENTS
            .iter()
            .position(|agent| *agent == self.default_chat_agent)
            .unwrap_or(0);
        self.context_menu = Some(crate::app::state::ContextMenuState {
            kind: crate::app::state::ContextMenuKind::DailyNewChat,
            x,
            y,
            list: crate::app::state::MenuListState::new(highlighted),
        });
        self.enter_overlay_mode(crate::app::Mode::ContextMenu);
    }

    /// Fold or unfold the daily section on this display (TP-DAILY-03).
    pub(crate) fn toggle_daily_section(&mut self) {
        self.daily_section_collapsed = !self.daily_section_collapsed;
    }

    /// Ask the daily section for every chat it holds, or fold it back to the
    /// glance surface's five (TP-DAILY-04 — the row is both ways).
    pub(crate) fn toggle_full_daily_drawer(&mut self) {
        self.daily_section_expanded = !self.daily_section_expanded;
        if let Some(daily) = self.daily_chat_cwd.clone() {
            let key = crate::persist::workspace_chats::ledger_key(&daily);
            // The read budget follows the switch: an opened section that is
            // still parsed at the glance limit would promise older chats it
            // can never list (TP-DRAW-10's reason, in this section).
            if self.daily_section_expanded {
                self.fully_open_chat_drawers.insert(key);
            } else {
                self.fully_open_chat_drawers.remove(&key);
            }
        }
    }

    /// Start a fresh chat rooted at a workspace's directory.
    pub(crate) fn request_workspace_chat(&mut self, ws_idx: usize) {
        let Some(workspace) = self.workspaces.get(ws_idx) else {
            return;
        };
        // TP-WSID-02: the chat starts where the row says it will — the
        // checkout, never the directory the workspace was born in.
        self.request_project_chat_tab = Some(crate::app::state::ProjectChatTabRequest {
            project_path: workspace.effective_cwd().to_path_buf(),
            session_id: None,
        });
    }

    /// Open the Spaces "+" menu for a workspace.
    ///
    /// A repository root offers worktree actions alongside the chat agents,
    /// because "start something new here" genuinely means two things there: a
    /// chat, or a new branch checkout. A row that IS already a linked worktree
    /// only offers chats — nesting worktrees is not a thing, and a menu entry
    /// that cannot work is worse than no entry.
    pub(crate) fn open_workspace_new_chat_menu(&mut self, ws_idx: usize, x: u16, y: u16) {
        let Some(workspace) = self.workspaces.get(ws_idx) else {
            return;
        };
        let offers_worktree = workspace.worktree_space.is_none();
        let highlighted = crate::app::projects::CHAT_AGENTS
            .iter()
            .position(|agent| *agent == self.default_chat_agent)
            .unwrap_or(0);
        self.context_menu = Some(crate::app::state::ContextMenuState {
            kind: crate::app::state::ContextMenuKind::WorkspaceNewChat {
                ws_idx,
                offers_worktree,
            },
            x,
            y,
            list: crate::app::state::MenuListState::new(highlighted),
        });
        self.enter_overlay_mode(crate::app::Mode::ContextMenu);
    }

    pub(super) fn open_project_new_chat_menu(&mut self, proj_idx: usize, x: u16, y: u16) {
        let highlighted = crate::app::projects::CHAT_AGENTS
            .iter()
            .position(|agent| *agent == self.default_chat_agent)
            .unwrap_or(0);
        let has_workspace = self.project_workspace_index(proj_idx).is_some();
        self.context_menu = Some(crate::app::state::ContextMenuState {
            kind: crate::app::state::ContextMenuKind::ProjectNewChat {
                proj_idx,
                has_workspace,
            },
            x,
            y,
            list: crate::app::state::MenuListState::new(highlighted),
        });
        self.enter_overlay_mode(crate::app::Mode::ContextMenu);
    }

    /// Handle a left click on a Projects-tab row. A project header row toggles
    /// its collapse state; a chat row queues a `claude --resume` tab request;
    /// the "(no chats)" row queues a new-chat tab request; the header's " +"
    /// button opens a new chat with the default agent, or the agent selector
    /// when shift is held (both consumed by the event loop, which owns the
    /// runtime). Hit-tests the same `project_row_areas` the render drew.
    pub(super) fn toggle_projects_row_at(
        &mut self,
        col: u16,
        row: u16,
        modifiers: crossterm::event::KeyModifiers,
    ) {
        let hit = self.project_row_kind_at(col, row);

        match hit {
            Some(crate::app::state::ProjectRowKind::Project { proj_idx }) => {
                if let Some(project) = self.projects_sessions.get(proj_idx) {
                    let path = project.path.clone();
                    if !self.collapsed_project_paths.remove(&path) {
                        self.collapsed_project_paths.insert(path);
                    }
                }
            }
            Some(crate::app::state::ProjectRowKind::Chat { proj_idx, chat_idx }) => {
                if let Some((project, session)) = self
                    .projects_sessions
                    .get(proj_idx)
                    .and_then(|project| Some((project, project.sessions.get(chat_idx)?)))
                {
                    // Spam-click guard: a chat already wired to a live tab is
                    // focused, never resumed a second time.
                    if let Some((ws_idx, tab_idx)) = self.find_resumed_chat_tab(&session.id) {
                        self.switch_workspace_tab(ws_idx, tab_idx);
                        self.mode = crate::app::Mode::Terminal;
                    } else {
                        self.request_project_chat_tab =
                            Some(crate::app::state::ProjectChatTabRequest {
                                project_path: project.path.clone(),
                                session_id: Some(session.id.clone()),
                            });
                    }
                }
            }
            Some(crate::app::state::ProjectRowKind::Empty { proj_idx }) => {
                if let Some(project) = self.projects_sessions.get(proj_idx) {
                    self.request_project_chat_tab =
                        Some(crate::app::state::ProjectChatTabRequest {
                            project_path: project.path.clone(),
                            session_id: None,
                        });
                }
            }
            Some(crate::app::state::ProjectRowKind::More { .. }) => {}
            Some(crate::app::state::ProjectRowKind::NewChat { proj_idx }) => {
                if modifiers.contains(crossterm::event::KeyModifiers::SHIFT) {
                    self.open_project_new_chat_menu(proj_idx, col, row);
                } else if let Some(project) = self.projects_sessions.get(proj_idx) {
                    self.request_project_chat_tab =
                        Some(crate::app::state::ProjectChatTabRequest {
                            project_path: project.path.clone(),
                            session_id: None,
                        });
                }
            }
            None => {}
        }
    }

    pub(crate) fn global_menu_labels(&self) -> Vec<&'static str> {
        let mut labels = vec!["settings", "keybinds", "reload config"];
        if self.update_available.is_some() {
            labels.push("update ready");
        } else if self.latest_release_notes_available {
            labels.push("what's new");
        }
        labels.push("detach");
        labels
    }

    pub(crate) fn global_menu_rect(&self) -> Rect {
        let screen = self.screen_rect();
        let launcher = self.global_launcher_rect();
        let labels = self.global_menu_labels();
        let content_width = labels
            .iter()
            .map(|label| {
                let badge_width = if self.global_menu_item_has_badge(label) {
                    2
                } else {
                    0
                };
                label.chars().count() as u16 + badge_width
            })
            .max()
            .unwrap_or(8)
            .saturating_add(2);
        let menu_w = content_width.saturating_add(2).min(screen.width.max(1));
        let menu_h = (labels.len() as u16 + 2).min(screen.height.max(1));
        let max_x = screen.x + screen.width.saturating_sub(menu_w);
        let desired_x = launcher.x + launcher.width.saturating_sub(menu_w);
        let x = desired_x.min(max_x);
        let y = launcher.y.saturating_sub(menu_h);
        Rect::new(x, y, menu_w, menu_h)
    }

    pub(super) fn on_sidebar_divider(&self, col: u16, row: u16) -> bool {
        if self.sidebar_collapsed {
            return false;
        }
        let sidebar = self.view.sidebar_rect;
        let toggle = crate::ui::expanded_sidebar_toggle_rect(sidebar, self.sidebar_chrome);
        let on_toggle = toggle.width > 0
            && col >= toggle.x
            && col < toggle.x + toggle.width
            && row >= toggle.y
            && row < toggle.y + toggle.height;
        sidebar.width > 0
            && !on_toggle
            && col == sidebar.x + sidebar.width.saturating_sub(1)
            && row >= sidebar.y
            && row < sidebar.y + sidebar.height
    }

    pub(super) fn on_sidebar_toggle(&self, col: u16, row: u16) -> bool {
        let rect = if self.sidebar_collapsed {
            crate::ui::collapsed_sidebar_toggle_rect(self.view.sidebar_rect)
        } else {
            crate::ui::expanded_sidebar_toggle_rect(self.view.sidebar_rect, self.sidebar_chrome)
        };
        rect.width > 0
            && col >= rect.x
            && col < rect.x + rect.width
            && row >= rect.y
            && row < rect.y + rect.height
    }

    pub(super) fn on_sidebar_section_divider(&self, col: u16, row: u16) -> bool {
        if self.sidebar_collapsed {
            return false;
        }
        let rect = crate::ui::sidebar_section_divider_rect(
            self.view.sidebar_rect,
            self.sidebar_section_split,
        );
        rect.width > 0
            && col >= rect.x
            && col < rect.x + rect.width
            && row >= rect.y
            && row < rect.y + rect.height
    }

    pub(super) fn set_sidebar_section_split(&mut self, row: u16) {
        let sidebar = self.view.sidebar_rect;
        let content_height = sidebar.height;
        if content_height < 6 {
            return;
        }
        let relative_y = row.saturating_sub(sidebar.y);
        let ratio = (relative_y as f32) / (content_height as f32);
        self.sidebar_section_split = ratio.clamp(0.1, 0.9);
        self.mark_session_dirty();
    }

    pub(super) fn workspace_at_row(&self, row: u16) -> Option<usize> {
        let footer = self.sidebar_footer_rect();
        if footer == Rect::default() {
            return None;
        }

        let cards = if self.view.workspace_card_areas.is_empty() {
            crate::ui::compute_workspace_card_areas(self, self.view.sidebar_rect)
        } else {
            self.view.workspace_card_areas.clone()
        };

        cards.iter().find_map(|card| {
            (row >= card.rect.y && row < card.rect.y + card.rect.height).then_some(card.ws_idx)
        })
    }

    pub(super) fn collapsed_workspace_at_row(&self, row: u16) -> Option<usize> {
        if !self.sidebar_collapsed {
            return None;
        }

        let (ws_area, _, _) = crate::ui::collapsed_sidebar_sections(self.view.sidebar_rect);
        if ws_area == Rect::default() || row < ws_area.y || row >= ws_area.y + ws_area.height {
            return None;
        }

        let idx = (row - ws_area.y) as usize;
        (idx < self.workspaces.len()).then_some(idx)
    }

    pub(super) fn collapsed_agent_detail_target_at(
        &self,
        row: u16,
    ) -> Option<(usize, usize, crate::layout::PaneId)> {
        if !self.sidebar_collapsed {
            return None;
        }

        let (_, _, detail_area) = crate::ui::collapsed_sidebar_sections(self.view.sidebar_rect);
        let detail_content_area = Rect::new(
            detail_area.x,
            detail_area.y,
            detail_area.width,
            detail_area.height.saturating_sub(1),
        );
        if detail_content_area == Rect::default()
            || row < detail_content_area.y
            || row >= detail_content_area.y + detail_content_area.height
        {
            return None;
        }

        let detail_idx = (row - detail_content_area.y) as usize;
        let details = crate::ui::agent_panel_entries(self);
        let detail = details.get(detail_idx)?;
        Some((detail.ws_idx, detail.tab_idx, detail.pane_id))
    }

    pub(super) fn workspace_drop_target_at_row(
        &self,
        row: u16,
    ) -> Option<crate::app::state::WorkspaceDropTarget> {
        let area = self.workspace_list_rect();
        let footer = self.sidebar_footer_rect();
        if area == Rect::default() || row < area.y || row >= footer.y {
            return None;
        }

        let cards = if self.view.workspace_card_areas.is_empty() {
            crate::ui::compute_workspace_card_areas(self, self.view.sidebar_rect)
        } else {
            self.view.workspace_card_areas.clone()
        };
        crate::ui::workspace_drop_slots(self, &cards, area)
            .into_iter()
            .enumerate()
            .min_by_key(|(slot_idx, (_, slot_row))| (row.abs_diff(*slot_row), *slot_idx))
            .map(|(_, (target, _))| target)
    }

    pub(super) fn workspace_move_block_params(
        &self,
        source_ws_idx: usize,
        drop_target: crate::app::state::WorkspaceDropTarget,
    ) -> Option<crate::api::schema::WorkspaceMoveBlockParams> {
        let source = self.workspaces.get(source_ws_idx)?;
        // TP-TREE-19: a drag moves a block only when it starts on a block
        // root of the VISIBLE list. Upstream expressed the same rule as "not
        // a linked worktree" because its roots are always the parent rows;
        // on the fork's tree the folded group's one visible card can be a
        // linked checkout (TP-TREE-03) and it IS the block's handle, while a
        // group's second expanded member never is.
        let roots = crate::ui::workspace_block_roots(self);
        let source_pos = roots.iter().position(|ws_idx| *ws_idx == source_ws_idx)?;
        let remaining_roots = roots
            .iter()
            .copied()
            .filter(|ws_idx| *ws_idx != source_ws_idx)
            .collect::<Vec<_>>();
        let insert_pos = match drop_target {
            crate::app::state::WorkspaceDropTarget::Before(target_ws_idx) => remaining_roots
                .iter()
                .position(|ws_idx| *ws_idx == target_ws_idx)?,
            crate::app::state::WorkspaceDropTarget::End => remaining_roots.len(),
        };
        if insert_pos == source_pos {
            return None;
        }

        // The block travels main-checkout-first, the order the expanded tree
        // draws it in, regardless of which member card the drag started on.
        // Membership is decided by the EFFECTIVE space (the same grouping the
        // list renders), so a repo split into sibling buckets never drags a
        // sibling bucket's checkouts along (TP-SPLIT-GROUP-01).
        let workspace_ids = match crate::ui::effective_space(self, source_ws_idx) {
            Some(source_space) => {
                let members = (0..self.workspaces.len())
                    .filter(|ws_idx| {
                        crate::ui::effective_space(self, *ws_idx)
                            .is_some_and(|space| space.key == source_space.key)
                    })
                    .collect::<Vec<_>>();
                let parent_idx = members.iter().copied().find(|ws_idx| {
                    crate::ui::effective_space(self, *ws_idx)
                        .is_some_and(|space| space.is_parent_candidate)
                });
                let ordered = match parent_idx {
                    Some(parent_idx) => std::iter::once(parent_idx)
                        .chain(members.iter().copied().filter(|idx| *idx != parent_idx))
                        .collect::<Vec<_>>(),
                    None => members,
                };
                ordered
                    .into_iter()
                    .filter_map(|ws_idx| self.workspaces.get(ws_idx))
                    .map(|workspace| workspace.id.clone())
                    .collect()
            }
            None => vec![source.id.clone()],
        };
        let before_workspace_id = match drop_target {
            crate::app::state::WorkspaceDropTarget::Before(target_ws_idx) => {
                let target = self.workspaces.get(target_ws_idx)?;
                let anchor = match crate::ui::workspace_parent_group_state(self, target_ws_idx)
                    .and_then(|_| crate::ui::effective_space(self, target_ws_idx))
                {
                    Some(target_space) => (0..self.workspaces.len())
                        .find(|ws_idx| {
                            crate::ui::effective_space(self, *ws_idx)
                                .is_some_and(|space| space.key == target_space.key)
                        })
                        .and_then(|ws_idx| self.workspaces.get(ws_idx))
                        .unwrap_or(target),
                    None => target,
                };
                Some(anchor.id.clone())
            }
            crate::app::state::WorkspaceDropTarget::End => None,
        };

        Some(crate::api::schema::WorkspaceMoveBlockParams {
            workspace_ids,
            before_workspace_id,
        })
    }

    pub(super) fn on_agent_panel_sort_toggle(&self, col: u16, row: u16) -> bool {
        if self.sidebar_collapsed || self.agent_view_override.is_some() {
            return false;
        }

        let (_, detail_area) = crate::ui::expanded_sidebar_sections(
            self.view.sidebar_rect,
            self.sidebar_section_split,
            self.sidebar_chrome,
        );
        let rect = crate::ui::agent_panel_toggle_rect(
            detail_area,
            self.agent_panel_sort,
            self.sidebar_chrome,
        );
        rect.width > 0
            && col >= rect.x
            && col < rect.x + rect.width
            && row >= rect.y
            && row < rect.y + rect.height
    }

    /// The ghost under `row`, resolved through the painter's own layout
    /// function so a grey row's hit box can never sit beside its paint.
    pub(super) fn closed_agent_target_at(&self, row: u16) -> Option<String> {
        if self.sidebar_collapsed {
            return None;
        }
        // TP-AGPANEL-43: resolved from the same placement walk the painter
        // uses, and across the ghost's whole card rather than only its first
        // row — a headstone is a card now, and a press on its lower half must
        // not fall through.
        // TP-AGPANEL-46: and through the same FILTERED sequence the painter
        // draws. The walk indexes visible ghosts, so the lookup must skip
        // hidden records too — an unfiltered `entries()` here bound every
        // row below a hidden record to its neighbour's headstone.
        let hit = crate::ui::closed_agent_index_at(self, self.agent_panel_rect(), row)?;
        self.visible_closed_agents()
            .nth(hit)
            .map(|record| record.agent_id.clone())
    }

    pub(super) fn agent_detail_target_at(
        &self,
        row: u16,
    ) -> Option<(usize, usize, crate::layout::PaneId)> {
        if self.sidebar_collapsed {
            return None;
        }

        let detail_area = self.agent_panel_rect();
        let metrics = crate::ui::agent_panel_scroll_metrics(self, detail_area);
        let body = crate::ui::agent_panel_body_rect(
            detail_area,
            crate::ui::should_show_scrollbar(metrics),
        );
        if body.height == 0 || row < body.y || row >= body.y + body.height {
            return None;
        }

        let mut row_y = body.y;
        let body_bottom = body.y + body.height;
        let entries = crate::ui::agent_panel_entries(self);
        let scroll = self.agent_panel_scroll.min(metrics.max_offset_from_bottom);
        for (index, detail) in entries.iter().enumerate().skip(scroll) {
            let height = crate::ui::agent_entry_height_in_body(self, detail, body.height);
            if row_y.saturating_add(height) > body_bottom {
                break;
            }
            if row >= row_y && row < row_y.saturating_add(height) {
                return Some((detail.ws_idx, detail.tab_idx, detail.pane_id));
            }
            row_y = row_y
                .saturating_add(height)
                .saturating_add(crate::ui::agent_entry_gap(self, index, entries.len()))
                .min(body_bottom);
        }
        None
    }

    /// The header tab (Spaces/Projects/Files) whose hit area contains
    /// `(col, row)`, if any. Returns `None` when the sidebar is collapsed or the
    /// point falls off every tab.
    pub(super) fn sidebar_tab_at(&self, col: u16, row: u16) -> Option<SidebarTab> {
        if self.sidebar_collapsed {
            return None;
        }
        SidebarTab::ALL.iter().enumerate().find_map(|(i, tab)| {
            let rect = self.view.sidebar_tab_hit_areas.get(i)?;
            (rect.width > 0
                && col >= rect.x
                && col < rect.x + rect.width
                && row >= rect.y
                && row < rect.y + rect.height)
                .then_some(*tab)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crossterm::event::{MouseButton, MouseEventKind};
    use ratatui::layout::Rect;

    use super::super::{app_for_mouse_test, capture_snapshot, mouse, unique_temp_path};
    use crate::{
        app::state::{AgentPanelSort, DragTarget, Mode},
        config::SidebarCollapsedModeConfig,
        detect::{Agent, AgentState},
        workspace::Workspace,
    };

    #[test]
    fn clicking_launcher_opens_global_menu() {
        let mut app = app_for_mouse_test();
        let rect = app.state.global_launcher_rect();

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            rect.x + rect.width.saturating_sub(1),
            rect.y,
        ));

        assert_eq!(app.state.mode, Mode::GlobalMenu);
    }

    #[test]
    fn hovering_global_menu_updates_highlight() {
        let mut app = app_for_mouse_test();
        let launcher = app.state.global_launcher_rect();
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            launcher.x,
            launcher.y,
        ));

        let menu = app.state.global_menu_rect();
        app.handle_mouse(mouse(MouseEventKind::Moved, menu.x + 2, menu.y + 2));

        assert_eq!(app.state.global_menu.highlighted, 1);
    }

    #[test]
    fn clicking_keybinds_menu_item_opens_help() {
        let mut app = app_for_mouse_test();
        let launcher = app.state.global_launcher_rect();
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            launcher.x,
            launcher.y,
        ));

        let menu = app.state.global_menu_rect();
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            menu.x + 2,
            menu.y + 2,
        ));

        assert_eq!(app.state.mode, Mode::KeybindHelp);
    }

    #[test]
    fn clicking_settings_menu_item_opens_settings() {
        let mut app = app_for_mouse_test();
        let launcher = app.state.global_launcher_rect();
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            launcher.x,
            launcher.y,
        ));

        let menu = app.state.global_menu_rect();
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            menu.x + 2,
            menu.y + 1,
        ));

        assert_eq!(app.state.mode, Mode::Settings);
    }

    #[test]
    fn clicking_reload_config_menu_item_requests_reload() {
        let mut app = app_for_mouse_test();
        let launcher = app.state.global_launcher_rect();
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            launcher.x,
            launcher.y,
        ));

        let menu = app.state.global_menu_rect();
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            menu.x + 2,
            menu.y + 3,
        ));

        assert!(app.state.request_reload_config);
        assert_eq!(app.state.mode, Mode::Navigate);
    }

    #[test]
    fn update_pending_menu_surfaces_update_ready_entry() {
        let mut app = app_for_mouse_test();
        app.state.update_available = Some("0.3.2".into());
        app.state.latest_release_notes_available = true;

        let launcher = app.state.global_launcher_rect();
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            launcher.x,
            launcher.y,
        ));

        assert_eq!(
            app.state.global_menu_labels(),
            vec![
                "settings",
                "keybinds",
                "reload config",
                "update ready",
                "detach"
            ]
        );
        assert!(!app.state.should_quit);
    }

    #[test]
    fn persistence_mode_menu_surfaces_detach_action() {
        let mut app = app_for_mouse_test();
        app.state.detach_exits = false;

        let launcher = app.state.global_launcher_rect();
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            launcher.x,
            launcher.y,
        ));

        assert_eq!(
            app.state.global_menu_labels(),
            vec!["settings", "keybinds", "reload config", "detach"]
        );

        let menu = app.state.global_menu_rect();
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            menu.x + 2,
            menu.y + 4,
        ));

        assert!(app.state.detach_requested);
        assert!(!app.state.should_quit);
        assert_ne!(app.state.mode, Mode::GlobalMenu);
    }

    #[test]
    fn whats_new_remains_in_menu_for_latest_installed_release_notes() {
        let mut app = app_for_mouse_test();
        app.state.latest_release_notes_available = true;

        assert_eq!(
            app.state.global_menu_labels(),
            vec![
                "settings",
                "keybinds",
                "reload config",
                "what's new",
                "detach"
            ]
        );
    }

    #[test]
    fn clicking_agent_detail_row_switches_to_correct_tab_and_pane() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        ws.tabs[0].set_custom_name("main".into());
        let first_pane = ws.tabs[0].root_pane;
        let first_tab = ws.test_add_tab(Some("logs"));
        let second_pane = ws.tabs[first_tab].root_pane;
        app.state.workspaces = vec![ws];
        app.state.ensure_test_terminals();
        let first_terminal_id = app.state.workspaces[0].tabs[0].panes[&first_pane]
            .attached_terminal_id
            .clone();
        app.state
            .terminals
            .get_mut(&first_terminal_id)
            .unwrap()
            .detected_agent = Some(Agent::Pi);
        let second_terminal_id = app.state.workspaces[0].tabs[first_tab].panes[&second_pane]
            .attached_terminal_id
            .clone();
        app.state
            .terminals
            .get_mut(&second_terminal_id)
            .unwrap()
            .detected_agent = Some(Agent::Claude);
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 2, 16));

        assert_eq!(app.state.workspaces[0].active_tab_index(), 1);
        assert_eq!(
            app.state.workspaces[0].tabs[1].layout.focused(),
            second_pane
        );
        assert_eq!(app.state.mode, Mode::Terminal);
        let snapshot = capture_snapshot(&app.state);
        assert_eq!(snapshot.workspaces[0].active_tab, first_tab);
        assert_eq!(
            snapshot.workspaces[0].tabs[first_tab].focused,
            Some(second_pane.raw())
        );
    }

    // TP-AGPANEL-23: the ghost click resolves through the painter's own
    // layout function, so the hit box can never sit beside the paint — and a
    // click above or below the graveyard names nobody.
    #[test]
    fn a_click_on_a_ghost_row_names_that_ghost() {
        let mut app = app_for_mouse_test();
        let ws = Workspace::test_new("one");
        let pane = ws.tabs[0].root_pane;
        app.state.workspaces = vec![ws];
        app.state.ensure_test_terminals();
        let terminal_id = app.state.workspaces[0].tabs[0].panes[&pane]
            .attached_terminal_id
            .clone();
        app.state
            .terminals
            .get_mut(&terminal_id)
            .unwrap()
            .detected_agent = Some(Agent::Pi);
        app.state.sidebar_agents.rows = vec![vec![crate::config::AgentSidebarToken::Agent]];

        for (id, at) in [("elder", 1), ("newer", 2)] {
            app.state
                .closed_agents
                .record_closed(crate::app::closed_agents::ClosedAgentRecord {
                    agent_id: id.into(),
                    label: id.into(),
                    cwd: None,
                    workspace_key: None,
                    session: None,
                    closed_at: at,
                    revival: crate::app::closed_agents::RevivalState::Dormant,
                });
        }

        let detail_area = app.state.agent_panel_rect();
        let (separator_y, ghost_rows) = crate::ui::closed_agent_row_slots(&app.state, detail_area)
            .expect("two ghosts and a roomy panel paint a graveyard");
        assert_eq!(ghost_rows.len(), 2);

        assert_eq!(
            app.state.closed_agent_target_at(ghost_rows[0]).as_deref(),
            Some("newer"),
            "the newest ghost sits first under the separator"
        );
        assert_eq!(
            app.state.closed_agent_target_at(ghost_rows[1]).as_deref(),
            Some("elder")
        );
        assert_eq!(
            app.state.closed_agent_target_at(separator_y),
            None,
            "the separator is furniture, not a target"
        );
        // The live row above still resolves to the live road, not a ghost.
        assert!(app.state.closed_agent_target_at(separator_y - 1).is_none());
    }

    // TP-AGPANEL-46: the resolver reads the same filtered sequence the
    // painter draws. A ledger record whose chat is hidden takes no row, so
    // it must take no hit either — with an unfiltered walk every ghost
    // BELOW a hidden record resolved to its neighbour, and a click revived
    // a different conversation than the one under the pointer.
    #[test]
    fn a_hidden_record_does_not_shift_the_ghost_a_click_names() {
        let mut app = app_for_mouse_test();
        let ws = Workspace::test_new("one");
        let pane = ws.tabs[0].root_pane;
        app.state.workspaces = vec![ws];
        app.state.ensure_test_terminals();
        let terminal_id = app.state.workspaces[0].tabs[0].panes[&pane]
            .attached_terminal_id
            .clone();
        app.state
            .terminals
            .get_mut(&terminal_id)
            .unwrap()
            .detected_agent = Some(Agent::Pi);
        app.state.sidebar_agents.rows = vec![vec![crate::config::AgentSidebarToken::Agent]];

        // Newest first in the ledger: the hidden chore sits at the FRONT,
        // ahead of both visible ghosts — the shape that shifted every hit.
        for (id, at) in [("elder", 1), ("newer", 2), ("chore", 3)] {
            app.state
                .closed_agents
                .record_closed(crate::app::closed_agents::ClosedAgentRecord {
                    agent_id: id.into(),
                    label: id.into(),
                    cwd: None,
                    workspace_key: None,
                    session: None,
                    closed_at: at,
                    revival: crate::app::closed_agents::RevivalState::Dormant,
                });
        }
        app.state.hidden_chat_labels = vec![crate::chat_labels::ChatLabel::Routine];
        app.state
            .derived_chat_labels
            .insert("chore".to_string(), crate::chat_labels::ChatLabel::Routine);

        let detail_area = app.state.agent_panel_rect();
        let (_, ghost_rows) = crate::ui::closed_agent_row_slots(&app.state, detail_area)
            .expect("two visible ghosts paint a graveyard");
        assert_eq!(ghost_rows.len(), 2, "the hidden chore takes no row");

        assert_eq!(
            app.state.closed_agent_target_at(ghost_rows[0]).as_deref(),
            Some("newer"),
            "the first drawn ghost is the one the click names"
        );
        assert_eq!(
            app.state.closed_agent_target_at(ghost_rows[1]).as_deref(),
            Some("elder"),
            "and the shift does not cascade to the row below"
        );
    }

    // TP-AGPANEL-46: the boundary the fix must not move — a hidden record
    // BEHIND every visible ghost never influenced the hits, and still must
    // not.
    #[test]
    fn a_hidden_record_at_the_tail_changes_nothing() {
        let mut app = app_for_mouse_test();
        let ws = Workspace::test_new("one");
        let pane = ws.tabs[0].root_pane;
        app.state.workspaces = vec![ws];
        app.state.ensure_test_terminals();
        let terminal_id = app.state.workspaces[0].tabs[0].panes[&pane]
            .attached_terminal_id
            .clone();
        app.state
            .terminals
            .get_mut(&terminal_id)
            .unwrap()
            .detected_agent = Some(Agent::Pi);
        app.state.sidebar_agents.rows = vec![vec![crate::config::AgentSidebarToken::Agent]];

        // The chore closed FIRST, so it sits at the ledger's tail.
        for (id, at) in [("chore", 1), ("elder", 2), ("newer", 3)] {
            app.state
                .closed_agents
                .record_closed(crate::app::closed_agents::ClosedAgentRecord {
                    agent_id: id.into(),
                    label: id.into(),
                    cwd: None,
                    workspace_key: None,
                    session: None,
                    closed_at: at,
                    revival: crate::app::closed_agents::RevivalState::Dormant,
                });
        }
        app.state.hidden_chat_labels = vec![crate::chat_labels::ChatLabel::Routine];
        app.state
            .derived_chat_labels
            .insert("chore".to_string(), crate::chat_labels::ChatLabel::Routine);

        let detail_area = app.state.agent_panel_rect();
        let (_, ghost_rows) = crate::ui::closed_agent_row_slots(&app.state, detail_area)
            .expect("two visible ghosts paint a graveyard");
        assert_eq!(ghost_rows.len(), 2);
        assert_eq!(
            app.state.closed_agent_target_at(ghost_rows[0]).as_deref(),
            Some("newer")
        );
        assert_eq!(
            app.state.closed_agent_target_at(ghost_rows[1]).as_deref(),
            Some("elder")
        );
    }

    #[test]
    fn per_agent_row_heights_preserve_card_gaps_and_trailing_mouse_targets() {
        let mut app = app_for_mouse_test();
        let first = Workspace::test_new("one");
        let first_pane = first.tabs[0].root_pane;
        let second = Workspace::test_new("two");
        let second_pane = second.tabs[0].root_pane;
        app.state.workspaces = vec![first, second];
        app.state.ensure_test_terminals();
        for (ws_idx, pane_id, agent) in
            [(0, first_pane, Agent::Pi), (1, second_pane, Agent::Claude)]
        {
            let terminal_id = app.state.workspaces[ws_idx].tabs[0].panes[&pane_id]
                .attached_terminal_id
                .clone();
            app.state
                .terminals
                .get_mut(&terminal_id)
                .unwrap()
                .detected_agent = Some(agent);
        }
        app.state.sidebar_agents.rows = vec![vec![crate::config::AgentSidebarToken::Agent]];
        app.state.sidebar_agents.rows_by_agent.insert(
            "claude".into(),
            vec![
                vec![crate::config::AgentSidebarToken::Agent],
                vec![crate::config::AgentSidebarToken::Workspace],
            ],
        );
        app.state.sidebar_agents.row_gap = 1;
        let detail_area = app.state.agent_panel_rect();
        let metrics = crate::ui::agent_panel_scroll_metrics(&app.state, detail_area);
        let body = crate::ui::agent_panel_body_rect(
            detail_area,
            crate::ui::should_show_scrollbar(metrics),
        );

        assert_eq!(
            app.state.agent_detail_target_at(body.y),
            Some((0, 0, first_pane))
        );
        assert_eq!(app.state.agent_detail_target_at(body.y + 1), None);
        assert_eq!(
            app.state.agent_detail_target_at(body.y + 3),
            Some((1, 0, second_pane))
        );

        app.state.sidebar_agents.row_gap = 0;
        assert_eq!(
            app.state.agent_detail_target_at(body.y + 1),
            Some((1, 0, second_pane))
        );
    }

    #[test]
    fn agent_hit_testing_clamps_scroll_after_dynamic_filter_shrink() {
        let mut app = app_for_mouse_test();
        let first = Workspace::test_new("one");
        let first_pane = first.tabs[0].root_pane;
        let second = Workspace::test_new("two");
        let second_pane = second.tabs[0].root_pane;
        app.state.workspaces = vec![first, second];
        app.state.ensure_test_terminals();
        app.state.active = Some(0);
        app.state.selected = 0;
        for (ws_idx, pane_id) in [(0, first_pane), (1, second_pane)] {
            let terminal_id = app.state.workspaces[ws_idx].tabs[0].panes[&pane_id]
                .attached_terminal_id
                .clone();
            app.state
                .terminals
                .get_mut(&terminal_id)
                .unwrap()
                .detected_agent = Some(Agent::Claude);
        }
        app.state.agent_view_override = Some(crate::api::schema::AgentViewSetParams {
            source: "example.views".to_string(),
            label: None,
            filter: Some(crate::api::schema::AgentViewFilter::Eq {
                field: crate::api::schema::AgentViewField::Builtin(
                    crate::api::schema::AgentViewBuiltinField::WorkspaceId,
                ),
                value: crate::api::schema::AgentViewValue::Context {
                    context: crate::api::schema::AgentViewContext::CurrentWorkspaceId,
                },
            }),
            sort: Vec::new(),
        });
        app.state.agent_panel_scroll = 10;
        let detail_area = app.state.agent_panel_rect();
        let body = crate::ui::agent_panel_body_rect(detail_area, false);

        assert_eq!(
            app.state.agent_detail_target_at(body.y),
            Some((0, 0, first_pane))
        );
    }

    #[test]
    fn clicking_agent_panel_toggle_switches_sort() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("test")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        app.state.agent_panel_scroll = 3;

        let (_, detail_area) = crate::ui::expanded_sidebar_sections(
            app.state.view.sidebar_rect,
            app.state.sidebar_section_split,
            app.state.sidebar_chrome,
        );
        let toggle = crate::ui::agent_panel_toggle_rect(
            detail_area,
            app.state.agent_panel_sort,
            app.state.sidebar_chrome,
        );
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            toggle.x,
            toggle.y,
        ));

        assert_eq!(app.state.agent_panel_sort, AgentPanelSort::Priority);
        assert_eq!(app.state.agent_panel_scroll, 0);
    }

    #[test]
    fn clicking_all_workspaces_agent_row_switches_to_correct_workspace() {
        let mut app = app_for_mouse_test();
        let first = Workspace::test_new("one");
        let first_pane = first.tabs[0].root_pane;

        let second = Workspace::test_new("two");
        let second_pane = second.tabs[0].root_pane;

        app.state.workspaces = vec![first, second];
        app.state.ensure_test_terminals();
        let first_terminal_id = app.state.workspaces[0].tabs[0].panes[&first_pane]
            .attached_terminal_id
            .clone();
        app.state
            .terminals
            .get_mut(&first_terminal_id)
            .unwrap()
            .detected_agent = Some(Agent::Pi);
        let second_terminal_id = app.state.workspaces[1].tabs[0].panes[&second_pane]
            .attached_terminal_id
            .clone();
        app.state
            .terminals
            .get_mut(&second_terminal_id)
            .unwrap()
            .detected_agent = Some(Agent::Claude);
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;

        let (_, detail_area) = crate::ui::expanded_sidebar_sections(
            app.state.view.sidebar_rect,
            app.state.sidebar_section_split,
            app.state.sidebar_chrome,
        );
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            detail_area.x + 2,
            detail_area.y + 6,
        ));

        assert_eq!(app.state.active, Some(1));
        assert_eq!(app.state.selected, 1);
        assert_eq!(app.state.workspaces[1].active_tab_index(), 0);
        assert_eq!(
            app.state.workspaces[1].tabs[0].layout.focused(),
            second_pane
        );
    }

    #[test]
    fn scrolling_agent_panel_with_wheel_updates_agent_panel_scroll() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let first_pane = ws.tabs[0].root_pane;

        let mut tabs = Vec::new();
        for (tab_name, agent) in [
            ("logs", Agent::Claude),
            ("review", Agent::Codex),
            ("ops", Agent::Gemini),
        ] {
            let tab_idx = ws.test_add_tab(Some(tab_name));
            let pane_id = ws.tabs[tab_idx].root_pane;
            tabs.push((tab_idx, pane_id, agent));
        }

        app.state.workspaces = vec![ws];
        app.state.ensure_test_terminals();
        let first_terminal_id = app.state.workspaces[0].tabs[0].panes[&first_pane]
            .attached_terminal_id
            .clone();
        app.state
            .terminals
            .get_mut(&first_terminal_id)
            .unwrap()
            .detected_agent = Some(Agent::Pi);
        for (tab_idx, pane_id, agent) in tabs {
            let terminal_id = app.state.workspaces[0].tabs[tab_idx].panes[&pane_id]
                .attached_terminal_id
                .clone();
            app.state
                .terminals
                .get_mut(&terminal_id)
                .unwrap()
                .detected_agent = Some(agent);
        }
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;

        let detail_area = app.state.agent_panel_rect();
        assert!(crate::ui::should_show_scrollbar(
            crate::ui::agent_panel_scroll_metrics(&app.state, detail_area)
        ));

        app.handle_mouse(mouse(
            MouseEventKind::ScrollDown,
            detail_area.x + 1,
            detail_area.y + 4,
        ));

        assert_eq!(app.state.agent_panel_scroll, 1);
        assert_eq!(app.state.selected, 0);
    }

    #[test]
    fn scrolling_projects_tab_with_wheel_updates_projects_scroll() {
        let mut app = app_for_mouse_test();
        app.state.sidebar_tab = crate::app::state::SidebarTab::Projects;
        app.state.projects_actives_only = false;
        // 7-chat project + empty project = 9 logical lines
        // (Header, Chat×5, More, Header, Empty) — overflows the list body.
        let chats: Vec<_> = (0..7)
            .map(|i| crate::claude_sessions::ClaudeSession {
                id: format!("s{i}"),
                title: "t".to_string(),
                last_modified: std::time::SystemTime::UNIX_EPOCH,
                last_message_at: None,
                msg_count: 1,
                opening: None,
            })
            .collect();
        app.state.projects_sessions = vec![
            crate::app::state::ProjectSessions {
                path: std::path::PathBuf::from("/a"),
                total_count: chats.len(),
                sessions: chats,
            },
            crate::app::state::ProjectSessions {
                path: std::path::PathBuf::from("/b"),
                total_count: 0,
                sessions: Vec::new(),
            },
        ];
        app.state.mode = Mode::Terminal;

        let list_area = app.state.workspace_list_rect();
        assert!(crate::ui::should_show_scrollbar(
            crate::ui::projects_scroll_metrics(&app.state, list_area)
        ));

        app.handle_mouse(mouse(
            MouseEventKind::ScrollDown,
            list_area.x + 1,
            list_area.y + 3,
        ));
        assert_eq!(app.state.projects_scroll, 1);

        app.handle_mouse(mouse(
            MouseEventKind::ScrollUp,
            list_area.x + 1,
            list_area.y + 3,
        ));
        assert_eq!(app.state.projects_scroll, 0);
        // The hidden Spaces selection must not move while Projects is shown.
        assert_eq!(app.state.selected, 0);
    }

    #[test]
    fn clicking_scrolled_agent_detail_row_switches_to_correct_tab_and_pane() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let first_pane = ws.tabs[0].root_pane;
        let second_tab = ws.test_add_tab(Some("logs"));
        let second_pane = ws.tabs[second_tab].root_pane;
        let mut extra_tabs = Vec::new();
        for (tab_name, agent) in [("review", Agent::Codex), ("ops", Agent::Gemini)] {
            let tab_idx = ws.test_add_tab(Some(tab_name));
            let pane_id = ws.tabs[tab_idx].root_pane;
            extra_tabs.push((tab_idx, pane_id, agent));
        }

        app.state.workspaces = vec![ws];
        app.state.ensure_test_terminals();
        let first_terminal_id = app.state.workspaces[0].tabs[0].panes[&first_pane]
            .attached_terminal_id
            .clone();
        app.state
            .terminals
            .get_mut(&first_terminal_id)
            .unwrap()
            .detected_agent = Some(Agent::Pi);
        let second_terminal_id = app.state.workspaces[0].tabs[second_tab].panes[&second_pane]
            .attached_terminal_id
            .clone();
        app.state
            .terminals
            .get_mut(&second_terminal_id)
            .unwrap()
            .detected_agent = Some(Agent::Claude);
        for (tab_idx, pane_id, agent) in extra_tabs {
            let terminal_id = app.state.workspaces[0].tabs[tab_idx].panes[&pane_id]
                .attached_terminal_id
                .clone();
            app.state
                .terminals
                .get_mut(&terminal_id)
                .unwrap()
                .detected_agent = Some(agent);
        }
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        app.state.sidebar_agents.rows = vec![vec![crate::config::AgentSidebarToken::Agent]];
        app.state.sidebar_agents.rows_by_agent.insert(
            "claude".into(),
            vec![
                vec![crate::config::AgentSidebarToken::Agent],
                vec![crate::config::AgentSidebarToken::Workspace],
            ],
        );
        app.state.agent_panel_scroll = 1;

        let detail_area = app.state.agent_panel_rect();
        let body = crate::ui::agent_panel_body_rect(detail_area, true);
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            body.x + 1,
            body.y + 1,
        ));

        assert_eq!(app.state.workspaces[0].active_tab_index(), second_tab);
        assert_eq!(
            app.state.workspaces[0].tabs[second_tab].layout.focused(),
            second_pane
        );
        assert_eq!(app.state.mode, Mode::Terminal);
    }

    #[test]
    fn clicking_collapsed_agent_row_switches_to_correct_tab_and_pane() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        let first_pane = ws.tabs[0].root_pane;
        let second_tab = ws.test_add_tab(Some("logs"));
        let second_pane = ws.tabs[second_tab].root_pane;
        app.state.workspaces = vec![ws];
        app.state.ensure_test_terminals();
        let first_terminal_id = app.state.workspaces[0].tabs[0].panes[&first_pane]
            .attached_terminal_id
            .clone();
        app.state
            .terminals
            .get_mut(&first_terminal_id)
            .unwrap()
            .detected_agent = Some(Agent::Pi);
        let second_terminal_id = app.state.workspaces[0].tabs[second_tab].panes[&second_pane]
            .attached_terminal_id
            .clone();
        app.state
            .terminals
            .get_mut(&second_terminal_id)
            .unwrap()
            .detected_agent = Some(Agent::Claude);
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        app.state.sidebar_collapsed = true;
        app.state.view.sidebar_rect = Rect::new(0, 0, 4, 20);
        app.state.view.terminal_area = Rect::new(4, 0, 80, 20);

        let (_, _, detail_area) =
            crate::ui::collapsed_sidebar_sections(app.state.view.sidebar_rect);
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            detail_area.x,
            detail_area.y + 1,
        ));

        assert_eq!(app.state.workspaces[0].active_tab_index(), 1);
        assert_eq!(
            app.state.workspaces[0].tabs[1].layout.focused(),
            second_pane
        );
        assert_eq!(app.state.mode, Mode::Terminal);
    }

    #[test]
    fn clicking_collapsed_priority_agent_row_switches_to_matching_workspace() {
        let mut app = app_for_mouse_test();
        let first = Workspace::test_new("one");
        let first_pane = first.tabs[0].root_pane;
        let second = Workspace::test_new("two");
        let second_pane = second.tabs[0].root_pane;

        app.state.workspaces = vec![first, second];
        app.state.ensure_test_terminals();
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        app.state.sidebar_collapsed = true;
        app.state.agent_panel_sort = AgentPanelSort::Priority;
        app.state.view.sidebar_rect = Rect::new(0, 0, 4, 20);
        app.state.view.terminal_area = Rect::new(4, 0, 80, 20);

        let set_state = |app: &mut crate::app::App, ws_idx: usize, pane_id, state| {
            let terminal_id = app.state.workspaces[ws_idx].tabs[0].panes[&pane_id]
                .attached_terminal_id
                .clone();
            let terminal = app.state.terminals.get_mut(&terminal_id).unwrap();
            terminal.detected_agent = Some(Agent::Claude);
            terminal.state = state;
        };
        set_state(&mut app, 0, first_pane, AgentState::Working);
        set_state(&mut app, 1, second_pane, AgentState::Blocked);

        let (_, _, detail_area) =
            crate::ui::collapsed_sidebar_sections(app.state.view.sidebar_rect);
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            detail_area.x,
            detail_area.y,
        ));

        assert_eq!(app.state.active, Some(1));
        assert_eq!(app.state.selected, 1);
        assert_eq!(
            app.state.workspaces[1].tabs[0].layout.focused(),
            second_pane
        );
    }

    #[test]
    fn clicking_collapsed_sidebar_toggle_expands_sidebar() {
        let mut app = app_for_mouse_test();
        app.state.view.sidebar_rect = Rect::new(0, 0, 4, 20);
        app.state.view.terminal_area = Rect::new(4, 0, 80, 20);
        assert!(app.state.set_sidebar_collapsed(true));
        app.state.session_dirty = false;

        let toggle = crate::ui::collapsed_sidebar_toggle_rect(app.state.view.sidebar_rect);
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            toggle.x,
            toggle.y,
        ));

        assert!(!app.state.sidebar_collapsed);
    }

    #[test]
    fn hidden_collapsed_sidebar_has_no_mouse_expand_hotspot() {
        let mut app = app_for_mouse_test();
        app.state.sidebar_collapsed = true;
        app.state.sidebar_collapsed_mode = SidebarCollapsedModeConfig::Hidden;
        app.state.view.sidebar_rect = Rect::new(0, 0, 0, 20);
        app.state.view.terminal_area = Rect::new(0, 0, 80, 20);

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 0, 19));

        assert!(app.state.sidebar_collapsed);
    }

    #[test]
    fn clicking_expanded_sidebar_toggle_collapses_sidebar() {
        let mut app = app_for_mouse_test();
        app.state.sidebar_collapsed = false;
        app.state.session_dirty = false;
        app.state.view.sidebar_rect = Rect::new(0, 0, 26, 20);
        app.state.view.terminal_area = Rect::new(26, 0, 80, 20);

        let toggle = crate::ui::expanded_sidebar_toggle_rect(
            app.state.view.sidebar_rect,
            app.state.sidebar_chrome,
        );
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            toggle.x,
            toggle.y,
        ));

        assert!(app.state.sidebar_collapsed);
        assert!(app.state.session_dirty);
        assert!(app.state.drag.is_none());
    }

    // T59/T60/T61 · a chip is a frame and a frame needs rows of its own. Asking
    // for chips grows the footer from one row to CHIP_ROWS, and the list gives
    // up exactly those rows — no more, and never onto the buttons. The bare
    // path is asserted first because it is the one every user sees today.
    #[test]
    fn chips_grow_the_footer_and_the_list_gives_up_exactly_those_rows() {
        let mut app = app_for_mouse_test();
        app.state.sidebar_collapsed = false;
        app.state.view.sidebar_rect = Rect::new(0, 0, 26, 24);

        let ws = app.state.workspace_list_rect();
        let bare_footer = app.state.sidebar_footer_rect();
        assert_eq!(bare_footer.height, 1, "today's footer is one row");
        assert_eq!(
            bare_footer.y + bare_footer.height,
            ws.y + ws.height,
            "the footer ends where the list section ends"
        );

        app.state.sidebar_chrome = crate::ui::shell::SidebarChrome {
            spaces: None,
            agents: None,
            chips: Some(crate::ui::shell::BarTint::solid(
                ratatui::style::Color::Rgb(1, 2, 3),
            )),
        };

        let chip_footer = app.state.sidebar_footer_rect();
        let grown = crate::ui::widgets::CHIP_ROWS;

        assert_eq!(chip_footer.height, grown, "the footer holds a whole frame");
        assert_eq!(
            app.state.workspace_list_rect(),
            ws,
            "the section itself did not move; only the split inside it did"
        );
        assert_eq!(
            chip_footer.y,
            bare_footer.y - (grown - 1),
            "the footer grew upwards into the list, not downwards off the panel"
        );
        assert_eq!(
            chip_footer.y + chip_footer.height,
            ws.y + ws.height,
            "and it still ends where the list section ends"
        );

        // The three controls stay inside the taller footer and stay disjoint.
        let new = app.state.sidebar_new_button_rect();
        let menu = app.state.global_launcher_rect();
        for (name, rect) in [("new", new), ("menu", menu)] {
            assert!(
                rect.y >= chip_footer.y
                    && rect.y + rect.height <= chip_footer.y + chip_footer.height,
                "{name} left the footer: {rect:?} vs {chip_footer:?}"
            );
            assert_eq!(rect.height, grown, "{name} is as tall as its frame");
        }
        assert!(
            new.x + new.width <= menu.x,
            "the buttons do not overlap: {new:?} {menu:?}"
        );
    }

    // T58c · the agents header's sort control is the third place that recomputed
    // the panel geometry from a constant chrome. Its rectangle must come from
    // the live one, or the control is clickable where it used to be rather than
    // where it is.
    #[test]
    fn the_agents_sort_control_is_clicked_where_the_header_drew_it() {
        let mut app = app_for_mouse_test();
        app.state.sidebar_collapsed = false;
        app.state.view.sidebar_rect = Rect::new(0, 0, 34, 26);
        let tint = crate::ui::shell::BarTint::solid(ratatui::style::Color::Rgb(1, 2, 3));

        for chrome in [
            crate::ui::shell::SidebarChrome {
                spaces: None,
                agents: Some(tint),
                chips: None,
            },
            crate::ui::shell::SidebarChrome {
                spaces: None,
                agents: None,
                chips: Some(tint),
            },
        ] {
            app.state.sidebar_chrome = chrome;
            let (_, detail) = crate::ui::expanded_sidebar_sections(
                app.state.view.sidebar_rect,
                app.state.sidebar_section_split,
                chrome,
            );
            let drawn =
                crate::ui::agent_panel_toggle_rect(detail, app.state.agent_panel_sort, chrome);

            assert!(
                app.state
                    .on_agent_panel_sort_toggle(drawn.x, drawn.y + drawn.height - 1),
                "the control did not answer at its own bottom row: {drawn:?}"
            );
            assert!(
                !app.state
                    .on_agent_panel_sort_toggle(drawn.x.saturating_sub(1), drawn.y),
                "and it answered for a cell that is not its own: {drawn:?}"
            );
        }
    }

    // T58b · the two halves are hit-tested through the same inset they were
    // drawn through. A frame steals a row and a column from its half; if only
    // the drawing knows that, every row in the panel answers for its neighbour
    // and no assertion in the suite notices, because both rectangles are still
    // inside the sidebar and still non-empty.
    #[test]
    fn a_framed_half_is_clicked_through_the_inset_it_was_drawn_through() {
        let mut app = app_for_mouse_test();
        app.state.sidebar_collapsed = false;
        app.state.view.sidebar_rect = Rect::new(0, 0, 26, 20);
        let tint = crate::ui::shell::BarTint::solid(ratatui::style::Color::Rgb(1, 2, 3));

        for chrome in [
            crate::ui::shell::SidebarChrome {
                spaces: Some(tint),
                agents: None,
                chips: None,
            },
            crate::ui::shell::SidebarChrome {
                spaces: None,
                agents: Some(tint),
                chips: None,
            },
            crate::ui::shell::SidebarChrome {
                spaces: Some(tint),
                agents: Some(tint),
                chips: None,
            },
        ] {
            app.state.sidebar_chrome = chrome;
            let (drawn_spaces, drawn_agents) = crate::ui::expanded_sidebar_sections(
                app.state.view.sidebar_rect,
                app.state.sidebar_section_split,
                chrome,
            );

            assert_eq!(
                app.state.workspace_list_rect(),
                drawn_spaces,
                "the spaces half is clicked where it is drawn"
            );
            assert_eq!(
                app.state.agent_panel_rect(),
                drawn_agents,
                "the agents half is clicked where it is drawn"
            );
        }
    }

    // T58 · when the agents half wears a frame the collapse icon moves, and the
    // click has to move with it. Nothing else in the suite would notice a drift
    // here: the icon would still be painted, the old cell would still be inside
    // the sidebar, and the click would simply land on the frame and do nothing.
    #[test]
    fn a_framed_sidebar_takes_the_collapse_click_where_the_icon_was_drawn() {
        let mut app = app_for_mouse_test();
        app.state.sidebar_collapsed = false;
        app.state.view.sidebar_rect = Rect::new(0, 0, 26, 20);
        app.state.view.terminal_area = Rect::new(26, 0, 80, 20);
        app.state.sidebar_chrome = crate::ui::shell::SidebarChrome {
            spaces: None,
            agents: Some(crate::ui::shell::BarTint::solid(
                ratatui::style::Color::Rgb(1, 2, 3),
            )),
            chips: None,
        };

        let drawn = crate::ui::expanded_sidebar_toggle_rect(
            app.state.view.sidebar_rect,
            app.state.sidebar_chrome,
        );

        // The cell the icon used to occupy is now the frame's corner, and it
        // must no longer collapse anything.
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            drawn.x + 1,
            drawn.y + 1,
        ));
        assert!(
            !app.state.sidebar_collapsed,
            "the frame's corner is decoration, not a button"
        );

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            drawn.x,
            drawn.y,
        ));
        assert!(
            app.state.sidebar_collapsed,
            "the click follows the icon into the frame"
        );
    }

    #[test]
    fn clicking_workspace_switches_on_mouse_up() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("a"), Workspace::test_new("b")];
        app.state.active = Some(0);
        app.state.selected = 0;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let target_row = app.state.view.workspace_card_areas[1].rect.y;

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            2,
            target_row,
        ));
        assert_eq!(app.state.active, Some(0));
        assert_eq!(app.state.workspace_presses.len(), 1);

        app.handle_mouse(mouse(MouseEventKind::Up(MouseButton::Left), 2, target_row));
        assert_eq!(app.state.active, Some(1));
        assert_eq!(app.state.selected, 1);
        assert!(app.state.workspace_presses.is_empty());
        let snapshot = capture_snapshot(&app.state);
        assert_eq!(snapshot.active, Some(1));
        assert_eq!(snapshot.selected, 1);
    }

    #[test]
    fn clicking_worktree_parent_row_focuses_workspace_without_toggling() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("main"), Workspace::test_new("issue")];
        for (idx, checkout_path) in ["/repo/herdr", "/repo/herdr-issue"].into_iter().enumerate() {
            app.state.workspaces[idx].worktree_space =
                Some(crate::workspace::WorktreeSpaceMembership {
                    key: "repo-key".into(),
                    label: "herdr".into(),
                    repo_root: "/repo/herdr".into(),
                    checkout_path: checkout_path.into(),
                    is_linked_worktree: idx > 0,
                });
        }
        app.state.active = None;
        app.state.mode = Mode::Terminal;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let parent = app.state.view.workspace_card_areas[0].rect;

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            parent.x + 2,
            parent.y,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Up(MouseButton::Left),
            parent.x + 2,
            parent.y,
        ));

        assert_eq!(app.state.active, Some(0));
        assert!(!app.state.collapsed_space_keys.contains("repo-key"));
    }

    #[test]
    fn clicking_worktree_parent_chevron_toggles_group_only() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("main"), Workspace::test_new("issue")];
        for (idx, checkout_path) in ["/repo/herdr", "/repo/herdr-issue"].into_iter().enumerate() {
            app.state.workspaces[idx].worktree_space =
                Some(crate::workspace::WorktreeSpaceMembership {
                    key: "repo-key".into(),
                    label: "herdr".into(),
                    repo_root: "/repo/herdr".into(),
                    checkout_path: checkout_path.into(),
                    is_linked_worktree: idx > 0,
                });
        }
        app.state.active = None;
        app.state.mode = Mode::Terminal;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        // TP-TREE-14 moved this control off the parent checkout and onto the
        // repository's own row. The subject is unchanged: pressing it toggles
        // the group and does nothing else.
        let parent = app.state.view.workspace_group_header_areas[0].rect;

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            parent.x,
            parent.y,
        ));

        assert_eq!(app.state.active, None);
        assert!(app.state.workspace_presses.is_empty());
        assert!(app.state.collapsed_space_keys.contains("repo-key"));

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            parent.x,
            parent.y,
        ));

        assert!(!app.state.collapsed_space_keys.contains("repo-key"));
    }

    #[test]
    fn clicking_project_header_toggles_project_only() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("main"), Workspace::test_new("issue")];
        for (idx, checkout_path) in ["/repo/herdr", "/repo/herdr-issue"].into_iter().enumerate() {
            app.state.workspaces[idx].worktree_space =
                Some(crate::workspace::WorktreeSpaceMembership {
                    key: "repo-key".into(),
                    label: "herdr".into(),
                    repo_root: "/repo/herdr".into(),
                    checkout_path: checkout_path.into(),
                    is_linked_worktree: idx > 0,
                });
        }
        app.state.space_projects = vec![crate::spaces::SpaceProject {
            key: "project:herdr".into(),
            name: "herdr".into(),
            icon: None,
            repo_roots: vec!["/repo/herdr".into()],
            space_keys: Vec::new(),
        }];
        app.state.active = None;
        app.state.mode = Mode::Terminal;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let header = app.state.view.workspace_project_header_areas[0].rect;

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            header.x,
            header.y,
        ));

        assert_eq!(app.state.active, None);
        assert!(app.state.workspace_presses.is_empty());
        // The fold lands in the per-display set now (TP-NODE-06/07): the
        // session-wide project set is read for legacy folds, never written.
        assert!(app.state.node_folded("project:herdr"));
        assert!(!app.state.collapsed_project_keys.contains("project:herdr"));

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            header.x,
            header.y,
        ));

        assert!(!app.state.node_folded("project:herdr"));
    }

    #[test]
    fn wheel_workspace_selection_follows_grouped_visual_order_without_scrollbar() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![
            Workspace::test_new("main"),
            Workspace::test_new("normal"),
            Workspace::test_new("issue"),
        ];
        for (idx, checkout_path) in [(0, "/repo/herdr"), (2, "/repo/herdr-issue")] {
            app.state.workspaces[idx].worktree_space =
                Some(crate::workspace::WorktreeSpaceMembership {
                    key: "repo-key".into(),
                    label: "herdr".into(),
                    repo_root: "/repo/herdr".into(),
                    checkout_path: checkout_path.into(),
                    is_linked_worktree: idx != 0,
                });
        }
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Navigate;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 30));
        let list = app.state.workspace_list_rect();
        assert!(!crate::ui::should_show_scrollbar(
            crate::ui::workspace_list_scroll_metrics(&app.state, list)
        ));

        app.handle_mouse(mouse(MouseEventKind::ScrollDown, list.x + 1, list.y + 1));

        assert_eq!(app.state.selected, 2);
    }

    #[test]
    fn dragging_workspace_reorders_without_changing_identity() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![
            Workspace::test_new("a"),
            Workspace::test_new("b"),
            Workspace::test_new("c"),
        ];
        app.state.sidebar_spaces.rows = vec![vec![crate::config::SpaceSidebarToken::Workspace]];
        app.state.sidebar_spaces.row_gap = 0;
        let active_id = app.state.workspaces[1].id.clone();
        let selected_id = app.state.workspaces[2].id.clone();
        app.state.active = Some(1);
        app.state.selected = 2;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let packed_boundary_row = app.state.view.workspace_card_areas[1].rect.y;
        assert_eq!(
            app.state.workspace_drop_target_at_row(packed_boundary_row),
            Some(crate::app::state::WorkspaceDropTarget::Before(2))
        );

        let source_row = app.state.view.workspace_card_areas[1].rect.y;
        let target_row = crate::ui::workspace_drop_indicator_row(
            &app.state,
            &app.state.view.workspace_card_areas,
            app.state.workspace_list_rect(),
            crate::app::state::WorkspaceDropTarget::Before(0),
        )
        .unwrap();

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            2,
            source_row,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Drag(MouseButton::Left),
            2,
            target_row,
        ));
        assert!(matches!(
            app.state.drag.as_ref().map(|drag| &drag.target),
            Some(DragTarget::WorkspaceReorder {
                source_ws_idx: 1,
                drop_target: Some(crate::app::state::WorkspaceDropTarget::Before(0)),
                ..
            })
        ));
        app.handle_mouse(mouse(MouseEventKind::Up(MouseButton::Left), 2, target_row));

        let names: Vec<_> = app
            .state
            .workspaces
            .iter()
            .map(|ws| ws.display_name())
            .collect();
        assert_eq!(names, vec!["b", "a", "c"]);
        assert_eq!(app.state.active, Some(0));
        assert_eq!(app.state.selected, 2);
        assert_eq!(app.state.workspaces[0].id, active_id);
        assert_eq!(app.state.workspaces[2].id, selected_id);
        let events = app.event_hub.events_after(0);
        assert!(events.iter().any(|(_, event)| matches!(
            event.data,
            crate::api::schema::EventData::WorkspaceMoved { .. }
        )));
        assert!(!events.iter().any(|(_, event)| matches!(
            event.data,
            crate::api::schema::EventData::WorkspaceReordered { .. }
        )));
        let snapshot = capture_snapshot(&app.state);
        let captured_names: Vec<_> = snapshot
            .workspaces
            .iter()
            .map(|ws| ws.custom_name.clone().unwrap())
            .collect();
        assert_eq!(captured_names, vec!["b", "a", "c"]);
    }

    #[test]
    fn clicking_tab_scroll_button_reveals_hidden_tabs_without_renaming() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        ws.test_add_tab(Some("logs"));
        ws.test_add_tab(Some("review"));
        ws.test_add_tab(Some("ops"));
        ws.test_add_tab(Some("notes"));
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 65, 20));

        let right = app.state.view.tab_scroll_right_hit_area;
        assert!(right.width > 0);

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            right.x + 1,
            right.y,
        ));

        assert_eq!(app.state.tab_scroll, 1);
        assert!(!app.state.tab_scroll_follow_active);
        assert_eq!(app.state.workspaces[0].active_tab_index(), 0);
        assert_eq!(app.state.view.tab_hit_areas[0].width, 0);
        assert!(app.state.workspaces[0].tabs[0].custom_name.is_none());
        assert_eq!(
            app.state.workspaces[0].tabs[1].custom_name.as_deref(),
            Some("logs")
        );
    }

    #[test]
    fn clicking_last_visible_tab_at_right_edge_does_not_overscroll() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        for name in [
            "one", "two", "three", "four", "five", "six", "seven", "eight",
        ] {
            ws.test_add_tab(Some(name));
        }
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.tab_scroll = usize::MAX;
        app.state.tab_scroll_follow_active = false;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 65, 20));

        let last_idx = app.state.workspaces[0].tabs.len() - 1;
        let target = app.state.view.tab_hit_areas[last_idx];
        let clamped_scroll = app.state.tab_scroll;
        assert!(target.width > 0, "last tab should already be visible");

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            target.x + 1,
            target.y,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Up(MouseButton::Left),
            target.x + 1,
            target.y,
        ));

        assert_eq!(app.state.workspaces[0].active_tab_index(), last_idx);
        assert_eq!(app.state.tab_scroll, clamped_scroll);
        assert!(app.state.view.tab_hit_areas[last_idx].width > 0);
    }

    #[test]
    fn dragging_tab_reorders_auto_and_custom_names_without_materializing_numbers() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        ws.test_add_tab(Some("foo"));
        ws.test_add_tab(None);
        let moved_root = ws.tabs[0].root_pane;
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));

        let source = app.state.view.tab_hit_areas[0];
        let last = app.state.view.tab_hit_areas[2];
        let drop_col = last.x + last.width;

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            source.x + 1,
            source.y,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Drag(MouseButton::Left),
            drop_col,
            source.y,
        ));
        assert!(matches!(
            app.state.drag.as_ref().map(|drag| &drag.target),
            Some(DragTarget::TabReorder {
                ws_idx: 0,
                source_tab_idx: 0,
                insert_idx: Some(3),
                ..
            })
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Up(MouseButton::Left),
            drop_col,
            source.y,
        ));

        let labels: Vec<_> = app.state.workspaces[0]
            .tabs
            .iter()
            .enumerate()
            .map(|(tab_idx, _)| app.state.workspaces[0].tab_display_name(tab_idx).unwrap())
            .collect();
        assert_eq!(labels, vec!["foo", "2", "3"]);
        assert_eq!(
            app.state.workspaces[0].tabs[0].custom_name.as_deref(),
            Some("foo")
        );
        assert!(app.state.workspaces[0].tabs[1].custom_name.is_none());
        assert!(app.state.workspaces[0].tabs[2].custom_name.is_none());
        assert_eq!(app.state.workspaces[0].tabs[0].number, 2);
        assert_eq!(app.state.workspaces[0].tabs[1].number, 3);
        assert_eq!(app.state.workspaces[0].tabs[2].number, 1);
        assert_eq!(app.state.workspaces[0].tabs[2].root_pane, moved_root);
        assert_eq!(app.state.workspaces[0].active_tab_index(), 2);
    }

    fn temp_git_repo(branch: &str) -> std::path::PathBuf {
        let repo = unique_temp_path("sidebar-drop-slot-repo");
        fs::create_dir_all(repo.join(".git")).unwrap();
        fs::write(
            repo.join(".git/HEAD"),
            format!("ref: refs/heads/{branch}\n"),
        )
        .unwrap();
        repo
    }

    fn workspace_with_space(name: &str, key: &str) -> Workspace {
        let mut ws = Workspace::test_new(name);
        ws.worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: key.into(),
            label: "herdr".into(),
            repo_root: "/repo/herdr".into(),
            checkout_path: format!("/repo/{name}").into(),
            is_linked_worktree: name != "main",
        });
        ws
    }

    #[test]
    fn top_drop_slot_is_distinct_from_gap_below_first_workspace() {
        let mut app = app_for_mouse_test();
        let first_repo = temp_git_repo("main");
        let second_repo = temp_git_repo("main");

        let mut first = Workspace::test_new("a");
        let first_root = first.tabs[0].root_pane;
        first.identity_cwd = first_repo.clone();
        first.refresh_git_ahead_behind();

        let mut second = Workspace::test_new("b");
        let second_root = second.tabs[0].root_pane;
        second.identity_cwd = second_repo.clone();
        second.refresh_git_ahead_behind();

        app.state.workspaces = vec![first, second];
        app.state.ensure_test_terminals();
        let first_terminal_id = app.state.workspaces[0].tabs[0].panes[&first_root]
            .attached_terminal_id
            .clone();
        app.state.terminals.get_mut(&first_terminal_id).unwrap().cwd = first_repo.clone();
        let second_terminal_id = app.state.workspaces[1].tabs[0].panes[&second_root]
            .attached_terminal_id
            .clone();
        app.state
            .terminals
            .get_mut(&second_terminal_id)
            .unwrap()
            .cwd = second_repo.clone();
        app.state.sidebar_spaces.row_gap = 1;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));

        assert_eq!(
            app.state.workspace_drop_target_at_row(0),
            Some(crate::app::state::WorkspaceDropTarget::Before(0))
        );
        assert_eq!(
            app.state.workspace_drop_target_at_row(1),
            Some(crate::app::state::WorkspaceDropTarget::Before(0))
        );
        assert_eq!(
            app.state.workspace_drop_target_at_row(2),
            Some(crate::app::state::WorkspaceDropTarget::Before(0))
        );
        assert_eq!(
            app.state.workspace_drop_target_at_row(3),
            Some(crate::app::state::WorkspaceDropTarget::Before(1))
        );

        let _ = fs::remove_dir_all(first_repo);
        let _ = fs::remove_dir_all(second_repo);
    }

    #[test]
    fn bottom_drop_slot_stays_below_last_workspace_not_footer() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![
            Workspace::test_new("a"),
            Workspace::test_new("b"),
            Workspace::test_new("c"),
        ];
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 24));

        let cards = &app.state.view.workspace_card_areas;
        let bottom_slot = crate::ui::workspace_drop_indicator_row(
            &app.state,
            cards,
            app.state.workspace_list_rect(),
            crate::app::state::WorkspaceDropTarget::End,
        )
        .unwrap();

        let last = cards.last().unwrap().rect;
        assert_eq!(bottom_slot, last.y + last.height);
        assert!(bottom_slot < app.state.sidebar_footer_rect().y.saturating_sub(1));
    }

    #[test]
    fn grouped_sidebar_drop_slots_do_not_land_inside_compact_group() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![
            workspace_with_space("main", "repo-key"),
            Workspace::test_new("normal"),
            workspace_with_space("issue", "repo-key"),
        ];
        app.state.active = Some(1);
        app.state.selected = 1;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 40));

        let cards = &app.state.view.workspace_card_areas;
        let order = cards.iter().map(|card| card.ws_idx).collect::<Vec<_>>();
        assert_eq!(order, vec![0, 2, 1]);
        let issue = cards.iter().find(|card| card.ws_idx == 2).unwrap();
        let normal = cards.iter().find(|card| card.ws_idx == 1).unwrap();

        assert_eq!(
            app.state.workspace_drop_target_at_row(issue.rect.y),
            Some(crate::app::state::WorkspaceDropTarget::Before(1))
        );
        assert_eq!(
            crate::ui::workspace_drop_indicator_row(
                &app.state,
                cards,
                app.state.workspace_list_rect(),
                crate::app::state::WorkspaceDropTarget::End,
            ),
            Some(normal.rect.y + normal.rect.height)
        );
    }

    #[test]
    fn plain_drag_anchors_to_the_selected_parentless_linked_workspace() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![
            workspace_with_space("one", "repo-key"),
            workspace_with_space("two", "repo-key"),
            Workspace::test_new("normal"),
        ];
        let target_id = app.state.workspaces[1].id.clone();

        let params = app
            .state
            .workspace_move_block_params(2, crate::app::state::WorkspaceDropTarget::Before(1))
            .unwrap();

        assert_eq!(params.workspace_ids, [app.state.workspaces[2].id.clone()]);
        assert_eq!(
            params.before_workspace_id.as_deref(),
            Some(target_id.as_str())
        );
    }

    #[test]
    fn dragging_worktree_parent_reorders_the_complete_group() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![
            workspace_with_space("main", "repo-key"),
            Workspace::test_new("normal"),
            workspace_with_space("issue", "repo-key"),
        ];
        app.state.active = Some(2);
        app.state.selected = 1;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 40));

        let parent = app
            .state
            .view
            .workspace_card_areas
            .iter()
            .find(|card| card.ws_idx == 0)
            .unwrap()
            .rect;
        let target_row = crate::ui::workspace_drop_indicator_row(
            &app.state,
            &app.state.view.workspace_card_areas,
            app.state.workspace_list_rect(),
            crate::app::state::WorkspaceDropTarget::End,
        )
        .unwrap();
        let active_id = app.state.workspaces[2].id.clone();
        let selected_id = app.state.workspaces[1].id.clone();

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 2, parent.y));
        app.handle_mouse(mouse(
            MouseEventKind::Drag(MouseButton::Left),
            2,
            target_row,
        ));
        assert!(matches!(
            app.state.drag.as_ref().map(|drag| &drag.target),
            Some(DragTarget::WorkspaceReorder {
                source_ws_idx: 0,
                drop_target: Some(crate::app::state::WorkspaceDropTarget::End),
                ..
            })
        ));
        app.handle_mouse(mouse(MouseEventKind::Up(MouseButton::Left), 2, target_row));

        assert_eq!(
            app.state
                .workspaces
                .iter()
                .map(|workspace| workspace.display_name())
                .collect::<Vec<_>>(),
            ["normal", "main", "issue"]
        );
        assert_eq!(
            app.state.workspaces[app.state.active.unwrap()].id,
            active_id
        );
        assert_eq!(app.state.workspaces[app.state.selected].id, selected_id);
    }

    // TP-TREE-19
    #[test]
    fn dragging_collapsed_worktree_parent_still_moves_hidden_children() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![
            workspace_with_space("issue", "repo-key"),
            Workspace::test_new("normal"),
            workspace_with_space("main", "repo-key"),
            workspace_with_space("review", "repo-key"),
        ];
        app.state.active = Some(0);
        app.state.selected = 1;
        app.state.collapsed_space_keys.insert("repo-key".into());
        let active_id = app.state.workspaces[0].id.clone();
        let selected_id = app.state.workspaces[1].id.clone();
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 40));
        // TP-TREE-19, rebased from upstream: upstream keeps the group's
        // parent row visible while folded, so it saw three cards here. The
        // fork's folded group shows only the checkout the user stands in
        // (TP-TREE-03) next to the plain workspace — two cards — and that
        // single visible card is the folded block's drag handle.
        assert_eq!(app.state.view.workspace_card_areas.len(), 2);

        let parent = app.state.view.workspace_card_areas[0].rect;
        let target_row = crate::ui::workspace_drop_indicator_row(
            &app.state,
            &app.state.view.workspace_card_areas,
            app.state.workspace_list_rect(),
            crate::app::state::WorkspaceDropTarget::End,
        )
        .unwrap();
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 2, parent.y));
        app.handle_mouse(mouse(
            MouseEventKind::Drag(MouseButton::Left),
            2,
            target_row,
        ));
        app.handle_mouse(mouse(MouseEventKind::Up(MouseButton::Left), 2, target_row));

        assert_eq!(
            app.state
                .workspaces
                .iter()
                .map(|workspace| workspace.display_name())
                .collect::<Vec<_>>(),
            ["normal", "main", "issue", "review"]
        );
        assert_eq!(
            app.state.workspaces[app.state.active.unwrap()].id,
            active_id
        );
        assert_eq!(app.state.workspaces[app.state.selected].id, selected_id);
    }

    #[test]
    fn dragging_worktree_space_member_does_not_reorder_workspaces() {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![
            workspace_with_space("main", "repo-key"),
            Workspace::test_new("normal"),
            workspace_with_space("issue", "repo-key"),
        ];
        app.state.active = Some(0);
        app.state.selected = 0;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 40));

        let source = app
            .state
            .view
            .workspace_card_areas
            .iter()
            .find(|card| card.ws_idx == 2)
            .unwrap()
            .rect;
        let target_row = crate::ui::workspace_drop_indicator_row(
            &app.state,
            &app.state.view.workspace_card_areas,
            app.state.workspace_list_rect(),
            crate::app::state::WorkspaceDropTarget::Before(0),
        )
        .unwrap();

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 2, source.y));
        app.handle_mouse(mouse(
            MouseEventKind::Drag(MouseButton::Left),
            2,
            target_row,
        ));
        assert!(app.state.drag.is_none());
        app.handle_mouse(mouse(MouseEventKind::Up(MouseButton::Left), 2, target_row));

        let names = app
            .state
            .workspaces
            .iter()
            .map(|ws| ws.display_name())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["main", "normal", "issue"]);
    }

    #[test]
    fn sidebar_divider_down_captures_without_committing_or_dirtying() {
        let mut app = app_for_mouse_test();
        app.state.session_dirty = false;

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 25, 5));

        assert_eq!(
            (
                app.state.sidebar_width,
                app.state.session_dirty,
                shell_resize_capture_for_test(&app.state),
                shell_resize_preview_width_for_test(&app.state),
            ),
            (26, false, true, Some(26))
        );
    }

    #[test]
    fn sidebar_divider_drag_is_preview_only_until_mouse_up() {
        let mut app = app_for_mouse_test();
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 25, 5));
        app.state.session_dirty = false;

        app.handle_mouse(mouse(MouseEventKind::Drag(MouseButton::Left), 30, 5));

        assert_eq!(
            (
                app.state.sidebar_width,
                app.state.session_dirty,
                shell_resize_capture_for_test(&app.state),
                shell_resize_preview_width_for_test(&app.state),
                capture_snapshot(&app.state).sidebar_width,
            ),
            (26, false, true, Some(31), Some(26))
        );
    }

    #[test]
    fn sidebar_divider_mouse_up_is_the_commit_boundary() {
        let mut app = app_for_mouse_test();
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 25, 5));
        app.handle_mouse(mouse(MouseEventKind::Drag(MouseButton::Left), 30, 5));
        app.state.session_dirty = false;

        app.handle_mouse(mouse(MouseEventKind::Up(MouseButton::Left), 30, 5));

        assert_eq!(
            (
                app.state.sidebar_width,
                app.state.session_dirty,
                shell_resize_capture_for_test(&app.state),
                capture_snapshot(&app.state).sidebar_width,
            ),
            (31, true, false, Some(31))
        );
    }

    #[test]
    fn terminal_resize_cancels_sidebar_preview_without_dirtying() {
        let mut app = app_for_mouse_test();
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 25, 5));
        app.handle_mouse(mouse(MouseEventKind::Drag(MouseButton::Left), 30, 5));
        app.state.session_dirty = false;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 100, 40));

        assert_eq!(
            (
                app.state.sidebar_width,
                app.state.session_dirty,
                shell_resize_capture_for_test(&app.state),
                shell_resize_preview_width_for_test(&app.state),
            ),
            (26, false, false, None)
        );
    }

    #[test]
    fn sidebar_preview_geometry_rebases_generation_and_commits() {
        let mut app = app_for_mouse_test();
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 25, 5));
        app.handle_mouse(mouse(MouseEventKind::Drag(MouseButton::Left), 30, 5));
        app.state.session_dirty = false;

        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 40));
        assert_eq!(app.state.view.sidebar_rect.width, 31);
        assert_eq!(app.state.sidebar_width, 26);
        assert!(shell_resize_capture_for_test(&app.state));

        app.handle_mouse(mouse(MouseEventKind::Up(MouseButton::Left), 30, 5));

        assert_eq!(app.state.sidebar_width, 31);
        assert!(app.state.session_dirty);
        assert!(!shell_resize_capture_for_test(&app.state));
    }

    #[test]
    fn sidebar_divider_click_without_drag_is_clean() {
        let mut app = app_for_mouse_test();
        app.state.session_dirty = false;

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 25, 5));
        app.handle_mouse(mouse(MouseEventKind::Up(MouseButton::Left), 25, 5));

        assert_eq!(app.state.sidebar_width, 26);
        assert!(!app.state.session_dirty);
        assert!(!shell_resize_capture_for_test(&app.state));
    }

    #[test]
    fn dragging_sidebar_divider_sets_manual_width() {
        let mut app = app_for_mouse_test();

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 25, 5));
        app.handle_mouse(mouse(MouseEventKind::Drag(MouseButton::Left), 30, 5));
        app.handle_mouse(mouse(MouseEventKind::Up(MouseButton::Left), 30, 5));

        assert_eq!(app.state.sidebar_width, 31);
        let snapshot = capture_snapshot(&app.state);
        assert_eq!(snapshot.sidebar_width, Some(31));
    }

    fn shell_resize_capture_for_test(state: &crate::app::state::AppState) -> bool {
        state.shell_resize_active()
    }

    fn shell_resize_preview_width_for_test(state: &crate::app::state::AppState) -> Option<u16> {
        state.shell_resize_preview_width()
    }

    #[test]
    fn dragging_sidebar_bottom_divider_still_sets_manual_width() {
        let mut app = app_for_mouse_test();
        let divider_col = app.state.view.sidebar_rect.x + app.state.view.sidebar_rect.width - 1;
        let bottom_row = app.state.view.sidebar_rect.y + app.state.view.sidebar_rect.height - 1;

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            divider_col,
            bottom_row,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Drag(MouseButton::Left),
            divider_col + 5,
            bottom_row,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Up(MouseButton::Left),
            divider_col + 5,
            bottom_row,
        ));

        assert_eq!(app.state.sidebar_width, 31);
    }

    #[test]
    fn dragging_past_max_clamps_to_configured_max() {
        let mut app = app_for_mouse_test();
        app.state.sidebar_max_width = 30;

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 25, 5));
        app.handle_mouse(mouse(MouseEventKind::Drag(MouseButton::Left), 50, 5));
        app.handle_mouse(mouse(MouseEventKind::Up(MouseButton::Left), 50, 5));

        assert_eq!(app.state.sidebar_width, 30);
    }

    #[test]
    fn dragging_below_min_clamps_to_configured_min() {
        let mut app = app_for_mouse_test();
        app.state.sidebar_min_width = 22;

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 25, 5));
        app.handle_mouse(mouse(MouseEventKind::Drag(MouseButton::Left), 5, 5));
        app.handle_mouse(mouse(MouseEventKind::Up(MouseButton::Left), 5, 5));

        assert_eq!(app.state.sidebar_width, 22);
    }

    #[test]
    fn dragging_sidebar_section_divider_sets_split_ratio() {
        let mut app = app_for_mouse_test();
        let divider = crate::ui::sidebar_section_divider_rect(
            app.state.view.sidebar_rect,
            app.state.sidebar_section_split,
        );

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            divider.x + 1,
            divider.y,
        ));
        app.handle_mouse(mouse(
            MouseEventKind::Drag(MouseButton::Left),
            divider.x + 1,
            divider.y + 4,
        ));

        assert!(app.state.sidebar_section_split > 0.5);
        let snapshot = capture_snapshot(&app.state);
        assert_eq!(
            snapshot.sidebar_section_split,
            Some(app.state.sidebar_section_split)
        );
    }

    #[test]
    fn double_clicking_sidebar_divider_resets_default_width() {
        let mut app = app_for_mouse_test();
        app.state.default_sidebar_width = 26;
        app.state.sidebar_width = 30;

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 25, 5));
        app.handle_mouse(mouse(MouseEventKind::Up(MouseButton::Left), 25, 5));
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 25, 5));

        assert_eq!(app.state.sidebar_width, 26);
        assert!(app.state.drag.is_none());
        let snapshot = capture_snapshot(&app.state);
        assert_eq!(snapshot.sidebar_width, Some(26));
    }

    #[test]
    fn clicking_sidebar_tab_switches_sidebar_tab() {
        use crate::app::state::SidebarTab;
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("a")];
        app.state.active = Some(0);
        app.state.selected = 0;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        assert_eq!(app.state.sidebar_tab, SidebarTab::Spaces);

        let projects_rect = app.state.view.sidebar_tab_hit_areas[1];
        assert!(projects_rect.width > 0, "projects tab should have width");
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            projects_rect.x,
            projects_rect.y,
        ));
        assert_eq!(app.state.sidebar_tab, SidebarTab::Projects);

        let files_rect = app.state.view.sidebar_tab_hit_areas[2];
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            files_rect.x,
            files_rect.y,
        ));
        assert_eq!(
            app.state.sidebar_tab,
            SidebarTab::Projects,
            "Files opens the center without replacing the global body"
        );

        let spaces_rect = app.state.view.sidebar_tab_hit_areas[0];
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            spaces_rect.x,
            spaces_rect.y,
        ));
        assert_eq!(app.state.sidebar_tab, SidebarTab::Spaces);
    }

    // TP-FCL-SHELL-01: Files is a center-stage launcher. The global Spaces
    // projection, including its workspace/agent tracking body, stays owned by
    // Spaces after activation.
    #[test]
    fn fcl_shell_files_activation_preserves_spaces_sidebar_projection() {
        use crate::app::state::SidebarTab;
        use crate::ui::surface_host::StageSurfaceView;
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("tracked-space")];
        app.state.active = Some(0);
        app.state.selected = 0;
        let frame = Rect::new(0, 0, 106, 20);
        crate::ui::compute_view(&mut app.state, frame);
        assert_eq!(app.state.sidebar_tab, SidebarTab::Spaces);

        let files_rect = app.state.view.sidebar_tab_hit_areas[2];
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            files_rect.x,
            files_rect.y,
        ));
        crate::ui::compute_view(&mut app.state, frame);

        assert_eq!(
            app.state.sidebar_tab,
            SidebarTab::Spaces,
            "Files activation must not replace the global tracking body"
        );
        assert_eq!(
            app.state.stage.surface_view(),
            StageSurfaceView::NativeFiles
        );
        assert!(
            !app.state.view.workspace_card_areas.is_empty(),
            "the visible global panel keeps workspace/agent tracking geometry"
        );
    }

    // TP-FCL-SHELL-02: Projects and Files are independent presentation
    // owners. Opening Files cannot silently switch the global body away from
    // a user-selected Projects view.
    #[test]
    fn fcl_shell_files_activation_preserves_projects_sidebar_owner() {
        use crate::app::state::SidebarTab;
        use crate::ui::surface_host::StageSurfaceView;
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("tracked-project")];
        app.state.active = Some(0);
        app.state.selected = 0;
        let frame = Rect::new(0, 0, 106, 20);
        crate::ui::compute_view(&mut app.state, frame);

        let projects_rect = app.state.view.sidebar_tab_hit_areas[1];
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            projects_rect.x,
            projects_rect.y,
        ));
        assert_eq!(app.state.sidebar_tab, SidebarTab::Projects);

        let files_rect = app.state.view.sidebar_tab_hit_areas[2];
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            files_rect.x,
            files_rect.y,
        ));

        assert_eq!(
            app.state.sidebar_tab,
            SidebarTab::Projects,
            "Files activation must preserve the selected global body"
        );
        assert_eq!(
            app.state.stage.surface_view(),
            StageSurfaceView::NativeFiles
        );
    }

    // FCL shell contract: the Files launcher remains visible after FCL-5,
    // but it never becomes the global sidebar body owner.
    // TP-FIP-NAV-01: a primary click on the visible default-sidebar Files tab
    // must open the Native Files Stage, not only switch the visual tab.
    #[test]
    fn files_tab_primary_click_opens_native_files_stage() {
        use crate::app::state::SidebarTab;
        use crate::ui::surface_host::StageSurfaceView;
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("a")];
        app.state.active = Some(0);
        app.state.selected = 0;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        assert_eq!(
            app.state.stage.surface_view(),
            StageSurfaceView::TerminalWorkspace
        );
        let files_rect = app.state.view.sidebar_tab_hit_areas[2];
        assert!(files_rect.width > 0, "files tab should have width");

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            files_rect.x,
            files_rect.y,
        ));

        assert_eq!(app.state.sidebar_tab, SidebarTab::Spaces);
        assert_eq!(
            app.state.stage.surface_view(),
            StageSurfaceView::NativeFiles
        );
        assert!(app.state.file_manager.is_some());
    }

    // FCL shell contract: reactivation remains protected without transferring
    // global sidebar ownership.
    // TP-FIP-NAV-02: reactivating Files from the visible tab keeps the open
    // singleton surface without resetting file-manager state.
    #[test]
    fn files_tab_click_reuses_open_singleton_files_stage() {
        use crate::app::state::SidebarTab;
        use crate::ui::surface_host::StageSurfaceView;
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("a")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state
            .activate_dock_app(crate::ui::surface_host::BuiltInAppId::Files);
        {
            let fm = app.state.file_manager.as_mut().expect("open file manager");
            // Marker: a re-open would reset this client-local flag to default.
            fm.show_hidden = true;
        }
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let files_rect = app.state.view.sidebar_tab_hit_areas[2];
        assert!(files_rect.width > 0, "files tab should have width");

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            files_rect.x,
            files_rect.y,
        ));

        assert_eq!(app.state.sidebar_tab, SidebarTab::Spaces);
        assert_eq!(
            app.state.stage.surface_view(),
            StageSurfaceView::NativeFiles
        );
        let fm = app
            .state
            .file_manager
            .as_ref()
            .expect("singleton kept open");
        assert!(
            fm.show_hidden,
            "singleton must not be reset by reactivation"
        );
    }

    // TP-FIP-NAV-03: switching to Spaces or Projects while Files is open must
    // restore the terminal Stage client-locally with identical terminal
    // identities and no runtime mutation.
    #[test]
    fn spaces_tab_click_restores_terminal_stage_and_preserves_identity() {
        use crate::app::state::SidebarTab;
        use crate::ui::surface_host::StageSurfaceView;
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("a")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.ensure_test_terminals();
        let terminals_before: std::collections::HashSet<_> =
            app.state.terminals.keys().cloned().collect();
        app.state
            .activate_dock_app(crate::ui::surface_host::BuiltInAppId::Files);
        assert_eq!(
            app.state.stage.surface_view(),
            StageSurfaceView::NativeFiles
        );
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let spaces_rect = app.state.view.sidebar_tab_hit_areas[0];
        assert!(spaces_rect.width > 0, "spaces tab should have width");

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            spaces_rect.x,
            spaces_rect.y,
        ));

        assert_eq!(app.state.sidebar_tab, SidebarTab::Spaces);
        assert_eq!(
            app.state.stage.surface_view(),
            StageSurfaceView::TerminalWorkspace
        );
        // Re-baselined 2026-07-25 (TP-FTAB-DOCK-02): leaving Files through the
        // shell backgrounds the tab instead of closing it, so its state
        // survives. What this test guards — the terminal stage is restored and
        // no terminal identity moves — is unchanged.
        assert!(
            app.state.file_manager.is_some(),
            "the Files tab keeps its state while backgrounded"
        );
        assert_eq!(app.state.stage.app_tab_instances().count(), 1);
        let terminals_after: std::collections::HashSet<_> =
            app.state.terminals.keys().cloned().collect();
        assert_eq!(terminals_before, terminals_after);
    }

    // TP-FIP-NAV-04: modified, middle, release-only, and outside clicks must
    // not transition the Stage.
    #[test]
    fn modified_left_click_on_files_tab_does_not_activate_stage() {
        use crate::ui::surface_host::StageSurfaceView;
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("a")];
        app.state.active = Some(0);
        app.state.selected = 0;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let files_rect = app.state.view.sidebar_tab_hit_areas[2];
        app.handle_mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: files_rect.x,
            row: files_rect.y,
            modifiers: crossterm::event::KeyModifiers::CONTROL,
        });
        assert_eq!(
            app.state.stage.surface_view(),
            StageSurfaceView::TerminalWorkspace
        );
        assert!(app.state.file_manager.is_none());
    }

    #[test]
    fn middle_click_on_files_tab_does_not_activate_stage() {
        use crate::ui::surface_host::StageSurfaceView;
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("a")];
        app.state.active = Some(0);
        app.state.selected = 0;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let files_rect = app.state.view.sidebar_tab_hit_areas[2];
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Middle),
            files_rect.x,
            files_rect.y,
        ));
        assert_eq!(
            app.state.stage.surface_view(),
            StageSurfaceView::TerminalWorkspace
        );
        assert!(app.state.file_manager.is_none());
    }

    #[test]
    fn release_only_event_on_files_tab_does_not_activate_stage() {
        use crate::ui::surface_host::StageSurfaceView;
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("a")];
        app.state.active = Some(0);
        app.state.selected = 0;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let files_rect = app.state.view.sidebar_tab_hit_areas[2];
        app.handle_mouse(mouse(
            MouseEventKind::Up(MouseButton::Left),
            files_rect.x,
            files_rect.y,
        ));
        assert_eq!(
            app.state.stage.surface_view(),
            StageSurfaceView::TerminalWorkspace
        );
        assert!(app.state.file_manager.is_none());
    }

    #[test]
    fn outside_click_next_to_files_tab_does_not_activate_stage() {
        use crate::ui::surface_host::StageSurfaceView;
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("a")];
        app.state.active = Some(0);
        app.state.selected = 0;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let files_rect = app.state.view.sidebar_tab_hit_areas[2];
        // One row below the tab strip, same column: not a tab hit.
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            files_rect.x,
            files_rect.y + files_rect.height,
        ));
        assert_eq!(
            app.state.stage.surface_view(),
            StageSurfaceView::TerminalWorkspace
        );
        assert!(app.state.file_manager.is_none());
    }

    // TP-FIP-NAV-08: a collapsed sidebar exposes no Files tab target.
    #[test]
    fn collapsed_sidebar_files_tab_is_inert() {
        use crate::ui::surface_host::StageSurfaceView;
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("a")];
        app.state.active = Some(0);
        app.state.selected = 0;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let files_rect = app.state.view.sidebar_tab_hit_areas[2];
        app.state.set_sidebar_collapsed(true);
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            files_rect.x,
            files_rect.y,
        ));
        assert_eq!(
            app.state.stage.surface_view(),
            StageSurfaceView::TerminalWorkspace
        );
        assert!(app.state.file_manager.is_none());
    }

    // TP-FIP-NAV-03 (Projects variant): the symmetric exit path.
    #[test]
    fn projects_tab_click_restores_terminal_stage_and_preserves_identity() {
        use crate::app::state::SidebarTab;
        use crate::ui::surface_host::StageSurfaceView;
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("a")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.ensure_test_terminals();
        let terminals_before: std::collections::HashSet<_> =
            app.state.terminals.keys().cloned().collect();
        app.state
            .activate_dock_app(crate::ui::surface_host::BuiltInAppId::Files);
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let projects_rect = app.state.view.sidebar_tab_hit_areas[1];
        assert!(projects_rect.width > 0, "projects tab should have width");

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            projects_rect.x,
            projects_rect.y,
        ));

        assert_eq!(app.state.sidebar_tab, SidebarTab::Projects);
        assert_eq!(
            app.state.stage.surface_view(),
            StageSurfaceView::TerminalWorkspace
        );
        // Re-baselined 2026-07-25 (TP-FTAB-DOCK-02): leaving Files through the
        // shell backgrounds the tab instead of closing it, so its state
        // survives. What this test guards — the terminal stage is restored and
        // no terminal identity moves — is unchanged.
        assert!(
            app.state.file_manager.is_some(),
            "the Files tab keeps its state while backgrounded"
        );
        assert_eq!(app.state.stage.app_tab_instances().count(), 1);
        let terminals_after: std::collections::HashSet<_> =
            app.state.terminals.keys().cloned().collect();
        assert_eq!(terminals_before, terminals_after);
    }

    // TP-C6.1-NAV / TP-FCL-INPUT-01: the content rail row carries exact path identity. Mouse
    // input prepares one request only; it performs no directory read itself.
    #[test]
    fn clicking_file_locations_rail_item_prepares_exact_typed_navigation_request() {
        use crate::app::state::{
            FileManagerLocationIcon, FileManagerLocationItem, FileManagerLocationsModel, SidebarTab,
        };
        let mut app = app_for_mouse_test();
        app.state.sidebar_tab = SidebarTab::Files;
        app.state
            .activate_dock_app(crate::ui::surface_host::BuiltInAppId::Files);
        app.state.file_manager_locations_model = FileManagerLocationsModel::from_sources(
            vec![
                FileManagerLocationItem {
                    label: "Home".into(),
                    path: std::path::PathBuf::from("/home/a"),
                    icon: FileManagerLocationIcon::Home,
                    accessible: true,
                    ejectable: false,
                },
                FileManagerLocationItem {
                    label: "Downloads".into(),
                    path: std::path::PathBuf::from("/home/a/Downloads"),
                    icon: FileManagerLocationIcon::Downloads,
                    accessible: true,
                    ejectable: false,
                },
            ],
            Vec::new(),
            Vec::new(),
        );
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let row = app.state.view.file_manager_locations.rows[0].clone();
        let before_file_manager = app.state.file_manager.is_some();

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            row.rect.x,
            row.rect.y,
        ));

        assert_eq!(
            app.state.request_file_manager_location_navigation,
            Some(std::path::PathBuf::from("/home/a").into())
        );
        assert_eq!(app.state.file_manager.is_some(), before_file_manager);

        let replacement = app.state.view.file_manager_locations.rows[1].clone();
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            replacement.rect.x,
            replacement.rect.y,
        ));
        assert_eq!(
            app.state.request_file_manager_location_navigation,
            Some(std::path::PathBuf::from("/home/a/Downloads").into()),
            "latest exact click replaces the prior unconsumed intent"
        );
    }

    // FMR-2: close the seam left between request-only mouse coverage and the
    // manually invoked sidebar consumer. This drives the real scheduled-task
    // chain and asserts the final loaded Trail projection.
    #[test]
    fn locations_rail_mouse_click_consumes_to_loaded_trail() {
        use crate::app::state::{
            FileManagerLocationIcon, FileManagerLocationItem, FileManagerLocationsModel, SidebarTab,
        };

        let root = unique_temp_path("sidebar-shortcut-mouse-e2e");
        let initial = root.join("initial");
        let target = root.join("target");
        fs::create_dir_all(&initial).expect("create initial directory");
        fs::create_dir_all(&target).expect("create sidebar target");
        fs::write(target.join("visible.txt"), b"visible").expect("write target entry");

        let mut app = app_for_mouse_test();
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&initial)))
            .expect("open initial Files instance");
        let generation = app
            .state
            .stage
            .active_instance_generation()
            .expect("active Files generation");
        app.state.sidebar_tab = SidebarTab::Files;
        app.state.file_manager_locations_model = FileManagerLocationsModel::from_sources(
            vec![FileManagerLocationItem {
                label: "Home".into(),
                path: target.clone(),
                icon: FileManagerLocationIcon::Home,
                accessible: true,
                ejectable: false,
            }],
            Vec::new(),
            Vec::new(),
        );
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let row = app.state.view.file_manager_locations.rows[0].clone();

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            row.rect.x,
            row.rect.y,
        ));
        assert_eq!(
            app.state.request_file_manager_location_navigation,
            Some(target.clone().into()),
            "primary click prepares the exact current-model path"
        );

        assert!(
            app.handle_scheduled_tasks(std::time::Instant::now(), false),
            "scheduled production consumer observes the one-shot request"
        );
        app.wait_file_manager_io_for_test();
        assert!(
            app.handle_scheduled_tasks(std::time::Instant::now(), false),
            "the next scheduled tick applies the prepared root"
        );
        assert!(app.state.request_file_manager_location_navigation.is_none());
        assert_eq!(
            app.state.stage.active_instance_generation(),
            Some(generation),
            "navigation stays inside the existing Files instance"
        );
        let file_manager = app.state.file_manager.as_ref().expect("loaded Files state");
        assert_eq!(file_manager.cwd, target);
        assert_eq!(file_manager.trail.cols().len(), 1);
        assert_eq!(file_manager.trail.cols()[0].directory, target);
        assert_eq!(file_manager.trail_snapshots.cols().len(), 1);
        assert!(file_manager.trail_snapshots.cols()[0]
            .entries()
            .iter()
            .any(|entry| entry.name == "visible.txt"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn locations_rail_mouse_modified_click_is_inert() {
        use crate::app::state::{
            FileManagerLocationIcon, FileManagerLocationItem, FileManagerLocationsModel, SidebarTab,
        };

        let mut app = app_for_mouse_test();
        app.state.sidebar_tab = SidebarTab::Files;
        app.state
            .activate_dock_app(crate::ui::surface_host::BuiltInAppId::Files);
        app.state.file_manager_locations_model = FileManagerLocationsModel::from_sources(
            vec![FileManagerLocationItem {
                label: "Home".into(),
                path: std::path::PathBuf::from("/home/a"),
                icon: FileManagerLocationIcon::Home,
                accessible: true,
                ejectable: false,
            }],
            Vec::new(),
            Vec::new(),
        );
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let row = app.state.view.file_manager_locations.rows[0].clone();

        app.handle_mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: row.rect.x,
            row: row.rect.y,
            modifiers: crossterm::event::KeyModifiers::CONTROL,
        });

        assert!(
            app.state.request_file_manager_location_navigation.is_none(),
            "modified shortcut clicks cannot authorize directory navigation"
        );
    }

    #[test]
    fn locations_rail_mouse_non_primary_and_inaccessible_rows_are_inert() {
        use crate::app::state::{
            FileManagerLocationIcon, FileManagerLocationItem, FileManagerLocationsModel, SidebarTab,
        };

        let path = std::path::PathBuf::from("/home/a");
        let item = |accessible| FileManagerLocationItem {
            label: "Home".into(),
            path: path.clone(),
            icon: FileManagerLocationIcon::Home,
            accessible,
            ejectable: false,
        };
        let mut app = app_for_mouse_test();
        app.state.sidebar_tab = SidebarTab::Files;
        app.state
            .activate_dock_app(crate::ui::surface_host::BuiltInAppId::Files);
        app.state.file_manager_locations_model =
            FileManagerLocationsModel::from_sources(vec![item(true)], Vec::new(), Vec::new());
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let row = app.state.view.file_manager_locations.rows[0].clone();
        app.state.file_manager_locations_model =
            FileManagerLocationsModel::from_sources(vec![item(false)], Vec::new(), Vec::new());

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            row.rect.x,
            row.rect.y,
        ));
        assert!(
            app.state.request_file_manager_location_navigation.is_none(),
            "inaccessible current-model rows fail closed"
        );

        app.state.file_manager_locations_model =
            FileManagerLocationsModel::from_sources(vec![item(true)], Vec::new(), Vec::new());
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let row = app.state.view.file_manager_locations.rows[0].clone();
        for kind in [
            MouseEventKind::Down(MouseButton::Middle),
            MouseEventKind::Up(MouseButton::Left),
        ] {
            app.handle_mouse(mouse(kind, row.rect.x, row.rect.y));
            assert!(
                app.state.request_file_manager_location_navigation.is_none(),
                "{kind:?} cannot authorize shortcut navigation"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn locations_rail_mouse_symlink_directory_loads_exact_trail() {
        use crate::app::state::{
            FileManagerLocationIcon, FileManagerLocationItem, FileManagerLocationsModel, SidebarTab,
        };

        let root = unique_temp_path("sidebar-shortcut-symlink-e2e");
        let initial = root.join("initial");
        let target = root.join("target");
        let link = root.join("linked-target");
        fs::create_dir_all(&initial).expect("create initial directory");
        fs::create_dir_all(&target).expect("create target directory");
        fs::write(target.join("inside.txt"), b"inside").expect("write target entry");
        std::os::unix::fs::symlink(&target, &link).expect("create directory symlink");

        let mut app = app_for_mouse_test();
        app.state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&initial)))
            .expect("open initial Files instance");
        app.state.sidebar_tab = SidebarTab::Files;
        app.state.file_manager_locations_model = FileManagerLocationsModel::from_sources(
            Vec::new(),
            vec![FileManagerLocationItem {
                label: "Linked".into(),
                path: link.clone(),
                icon: FileManagerLocationIcon::Pin,
                accessible: true,
                ejectable: false,
            }],
            Vec::new(),
        );
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let row = app.state.view.file_manager_locations.rows[0].clone();

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            row.rect.x,
            row.rect.y,
        ));
        assert!(app.handle_scheduled_tasks(std::time::Instant::now(), false));
        app.wait_file_manager_io_for_test();
        assert!(app.handle_scheduled_tasks(std::time::Instant::now(), false));

        let file_manager = app.state.file_manager.as_ref().expect("loaded Files state");
        assert_eq!(file_manager.cwd, link);
        assert_eq!(file_manager.trail.cols()[0].directory, link);
        assert!(file_manager.trail_snapshots.cols()[0]
            .entries()
            .iter()
            .any(|entry| entry.name == "inside.txt"));

        let _ = fs::remove_dir_all(root);
    }

    // TP-C6.1-GEOMETRY/NAV: cached geometry cannot authorize a path after the
    // prepared model changes underneath it.
    #[test]
    fn stale_file_locations_rail_hit_area_is_inert_after_model_refresh() {
        use crate::app::state::{
            FileManagerLocationIcon, FileManagerLocationItem, FileManagerLocationsModel, SidebarTab,
        };
        let mut app = app_for_mouse_test();
        app.state.sidebar_tab = SidebarTab::Files;
        app.state
            .activate_dock_app(crate::ui::surface_host::BuiltInAppId::Files);
        app.state.file_manager_locations_model = FileManagerLocationsModel::from_sources(
            vec![FileManagerLocationItem {
                label: "Home".into(),
                path: std::path::PathBuf::from("/home/a"),
                icon: FileManagerLocationIcon::Home,
                accessible: true,
                ejectable: false,
            }],
            Vec::new(),
            Vec::new(),
        );
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let stale_row = app.state.view.file_manager_locations.rows[0].clone();
        app.state.file_manager_locations_model = FileManagerLocationsModel::default();

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            stale_row.rect.x,
            stale_row.rect.y,
        ));

        assert!(app.state.request_file_manager_location_navigation.is_none());
    }

    // TP-FCL-SHELL-01: a legacy Files tab value cannot hide or disable the
    // global Spaces tracker. Its wheel remains owned by the visible workspace
    // list while location scrolling stays inside CenterContent.
    #[test]
    fn legacy_files_tab_value_keeps_visible_spaces_wheel_interaction() {
        use crate::app::state::SidebarTab;
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![
            crate::workspace::Workspace::test_new("one"),
            crate::workspace::Workspace::test_new("two"),
        ];
        app.state.active = Some(0);
        app.state.sidebar_tab = SidebarTab::Files;
        app.state.workspace_scroll = 0;
        app.state.selected = 0;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let list = app.state.workspace_list_rect();

        app.handle_mouse(mouse(
            MouseEventKind::ScrollDown,
            list.x,
            list.y.saturating_add(2),
        ));

        assert_eq!(app.state.workspace_scroll, 0);
        assert_eq!(
            app.state.selected, 1,
            "the visible Spaces tracker keeps its normal wheel selection"
        );
    }

    #[test]
    fn clicking_sidebar_tab_does_not_start_a_workspace_press() {
        use crate::app::state::SidebarTab;
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("a"), Workspace::test_new("b")];
        app.state.active = Some(0);
        app.state.selected = 0;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));

        let projects_rect = app.state.view.sidebar_tab_hit_areas[1];
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            projects_rect.x,
            projects_rect.y,
        ));

        // Switching tabs must not begin a workspace drag/select gesture, and
        // must not change which workspace is active.
        assert_eq!(app.state.sidebar_tab, SidebarTab::Projects);
        assert!(app.state.workspace_presses.is_empty());
        assert_eq!(app.state.active, Some(0));
    }

    // ---- Projects tab row clicks (Task #5) --------------------------------

    fn test_chat(id: &str) -> crate::claude_sessions::ClaudeSession {
        crate::claude_sessions::ClaudeSession {
            id: id.to_string(),
            title: format!("chat {id}"),
            last_modified: std::time::SystemTime::UNIX_EPOCH,
            last_message_at: None,
            msg_count: 3,
            opening: None,
        }
    }

    /// An App on the Projects tab with one pinned project at `/home/x/proj`
    /// holding `sessions`, with `compute_view` already run so
    /// `view.project_row_areas` matches what the user sees.
    fn projects_tab_app(sessions: Vec<crate::claude_sessions::ClaudeSession>) -> crate::app::App {
        let mut app = app_for_mouse_test();
        app.state.workspaces = vec![Workspace::test_new("a")];
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.sidebar_tab = crate::app::state::SidebarTab::Projects;
        // Row-interaction tests exercise the full list; actives-toggle tests
        // opt back in explicitly.
        app.state.projects_actives_only = false;
        let total_count = sessions.len();
        app.state.projects_sessions = vec![crate::app::state::ProjectSessions {
            path: std::path::PathBuf::from("/home/x/proj"),
            sessions,
            total_count,
        }];
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        app
    }

    fn project_row_rect(
        app: &crate::app::App,
        want: impl Fn(&crate::app::state::ProjectRowKind) -> bool,
    ) -> Rect {
        app.state
            .view
            .project_row_areas
            .iter()
            .find(|area| want(&area.kind))
            .expect("expected project row missing from computed view")
            .rect
    }

    // T5a-3: clicking a chat row must queue a resume request carrying that
    // chat's project path (cwd) and session id — the core Task #5 trigger.
    #[test]
    fn clicking_project_chat_row_requests_resume_chat_tab() {
        let mut app = projects_tab_app(vec![test_chat("sess-1"), test_chat("sess-2")]);
        let rect = project_row_rect(&app, |kind| {
            matches!(
                kind,
                crate::app::state::ProjectRowKind::Chat {
                    proj_idx: 0,
                    chat_idx: 1
                }
            )
        });

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            rect.x + 2,
            rect.y,
        ));

        assert_eq!(
            app.state.request_project_chat_tab,
            Some(crate::app::state::ProjectChatTabRequest {
                project_path: std::path::PathBuf::from("/home/x/proj"),
                session_id: Some("sess-2".to_string()),
            })
        );
        // A chat click must not disturb the project's collapse state.
        assert!(app.state.collapsed_project_paths.is_empty());
    }

    // TP-PROJTAB-01: a click resolved against stale row rects must not act.
    // The session poll rewrites the chat list newest-first while the laid-out
    // rects still describe the old order; acting on the stale index is
    // exactly how a chat resumed in the wrong project directory. The guard
    // turns that click into a no-op instead — the next frame lays out fresh
    // rects and the next click means what the user sees.
    #[test]
    fn a_stale_projects_click_is_inert_after_the_list_shifts() {
        let mut app = projects_tab_app(vec![test_chat("sess-1"), test_chat("sess-2")]);
        let rect = project_row_rect(&app, |kind| {
            matches!(
                kind,
                crate::app::state::ProjectRowKind::Chat {
                    proj_idx: 0,
                    chat_idx: 1
                }
            )
        });

        // The poll lands between two computes: a fresh chat takes the top
        // slot and every row shifts one down under the unchanged rects.
        app.state.projects_sessions[0]
            .sessions
            .insert(0, test_chat("sess-0"));
        app.state.projects_sessions[0].total_count += 1;
        app.state.projects_sessions_generation =
            app.state.projects_sessions_generation.wrapping_add(1);

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            rect.x + 2,
            rect.y,
        ));

        assert_eq!(
            app.state.request_project_chat_tab, None,
            "a stale click must not resume whichever neighbour now holds the index"
        );
    }

    // TP-PROJTAB-01: the guard's other half — the poll writer actually bumps
    // the generation, so a stale projection is *detectable* at all.
    #[test]
    fn the_session_poll_bumps_the_projects_generation() {
        let mut app = projects_tab_app(Vec::new());
        let before = app.state.projects_sessions_generation;
        // `projects_pinned` is empty here, so the refresh never touches
        // the directory — any existing path serves as the projects root.
        app.state.refresh_project_sessions_in(&std::env::temp_dir());
        assert_eq!(
            app.state.projects_sessions_generation,
            before.wrapping_add(1),
            "every poll rewrite must move the generation"
        );
    }

    // T5a-4: clicking the "(no chats)" row starts a NEW chat in that project
    // (session_id None) — the per-project new-chat affordance.
    #[test]
    fn clicking_no_chats_row_requests_new_chat_tab() {
        let mut app = projects_tab_app(Vec::new());
        let rect = project_row_rect(&app, |kind| {
            matches!(
                kind,
                crate::app::state::ProjectRowKind::Empty { proj_idx: 0 }
            )
        });

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            rect.x + 2,
            rect.y,
        ));

        assert_eq!(
            app.state.request_project_chat_tab,
            Some(crate::app::state::ProjectChatTabRequest {
                project_path: std::path::PathBuf::from("/home/x/proj"),
                session_id: None,
            })
        );
    }

    // T5a-5: clicking empty space below the rows is inert — no request, no
    // collapse change. Guards against over-eager hit-testing.
    #[test]
    fn clicking_projects_body_outside_rows_is_inert() {
        let mut app = projects_tab_app(vec![test_chat("sess-1")]);
        let last_row_y = app
            .state
            .view
            .project_row_areas
            .iter()
            .map(|area| area.rect.y)
            .max()
            .expect("rows expected");

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            2,
            last_row_y + 2,
        ));

        assert_eq!(app.state.request_project_chat_tab, None);
        assert!(app.state.collapsed_project_paths.is_empty());
    }

    // T5b (spam-click): clicking a chat that is already wired to a live tab
    // focuses that tab instead of queueing another request — repeated clicks
    // must never spawn duplicates.
    #[test]
    fn clicking_wired_chat_row_focuses_existing_tab_without_request() {
        let mut app = projects_tab_app(vec![test_chat("sess-1")]);
        let mut ws = Workspace::test_new("proj");
        let tab_idx = ws.test_add_tab(Some("chat"));
        ws.tabs[tab_idx].resumed_session_id = Some("sess-1".to_string());
        app.state.workspaces.push(ws);
        let rect = project_row_rect(&app, |kind| {
            matches!(
                kind,
                crate::app::state::ProjectRowKind::Chat {
                    proj_idx: 0,
                    chat_idx: 0
                }
            )
        });

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            rect.x + 2,
            rect.y,
        ));

        assert_eq!(
            app.state.request_project_chat_tab, None,
            "wired chat must not queue a duplicate request"
        );
        assert_eq!(
            app.state.active,
            Some(1),
            "focus jumps to the wired tab's workspace"
        );
        assert_eq!(app.state.workspaces[1].active_tab_index(), tab_idx);
        assert_eq!(app.state.mode, Mode::Terminal);
    }

    // T12d (regression): with the Projects tab active, clicking an agent row
    // in the lower panel must still focus that agent's tab — the Projects
    // branch used to swallow every sidebar click, breaking "click an agent to
    // jump back to its chat".
    #[test]
    fn clicking_agent_row_focuses_chat_while_projects_tab_active() {
        let mut app = app_for_mouse_test();
        let mut ws = Workspace::test_new("test");
        ws.tabs[0].set_custom_name("main".into());
        let first_pane = ws.tabs[0].root_pane;
        let second_tab = ws.test_add_tab(Some("logs"));
        let second_pane = ws.tabs[second_tab].root_pane;
        app.state.workspaces = vec![ws];
        app.state.ensure_test_terminals();
        let first_terminal_id = app.state.workspaces[0].tabs[0].panes[&first_pane]
            .attached_terminal_id
            .clone();
        app.state
            .terminals
            .get_mut(&first_terminal_id)
            .unwrap()
            .detected_agent = Some(Agent::Pi);
        let second_terminal_id = app.state.workspaces[0].tabs[second_tab].panes[&second_pane]
            .attached_terminal_id
            .clone();
        app.state
            .terminals
            .get_mut(&second_terminal_id)
            .unwrap()
            .detected_agent = Some(Agent::Claude);
        app.state.active = Some(0);
        app.state.selected = 0;
        app.state.mode = Mode::Terminal;
        app.state.sidebar_tab = crate::app::state::SidebarTab::Projects;

        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 2, 16));

        assert_eq!(app.state.workspaces[0].active_tab_index(), 1);
        assert_eq!(
            app.state.workspaces[0].tabs[1].layout.focused(),
            second_pane
        );
    }

    // ---- Project "+" button + agent selector (Task #10) -------------------

    // C4: a plain left click on "+" queues a new chat in that project (the
    // event loop opens it with the default agent) — and must neither toggle
    // collapse nor open a menu.
    #[test]
    fn clicking_project_plus_button_requests_default_new_chat() {
        let mut app = projects_tab_app(vec![test_chat("sess-1")]);
        let rect = project_row_rect(&app, |kind| {
            matches!(
                kind,
                crate::app::state::ProjectRowKind::NewChat { proj_idx: 0 }
            )
        });

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            rect.x + 1,
            rect.y,
        ));

        assert_eq!(
            app.state.request_project_chat_tab,
            Some(crate::app::state::ProjectChatTabRequest {
                project_path: std::path::PathBuf::from("/home/x/proj"),
                session_id: None,
            })
        );
        assert!(app.state.collapsed_project_paths.is_empty());
        assert!(app.state.context_menu.is_none());
    }

    // C5: shift+left click on "+" opens the agent selector instead — no
    // request yet, and the CURRENT default is highlighted for orientation.
    #[test]
    fn shift_clicking_project_plus_button_opens_agent_selector() {
        let mut app = projects_tab_app(vec![test_chat("sess-1")]);
        app.state.default_chat_agent = "gemini".to_string();
        let rect = project_row_rect(&app, |kind| {
            matches!(
                kind,
                crate::app::state::ProjectRowKind::NewChat { proj_idx: 0 }
            )
        });

        app.handle_mouse(crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: rect.x + 1,
            row: rect.y,
            modifiers: crossterm::event::KeyModifiers::SHIFT,
        });

        assert_eq!(app.state.request_project_chat_tab, None);
        let menu = app.state.context_menu.as_ref().expect("selector open");
        assert!(matches!(
            menu.kind,
            crate::app::state::ContextMenuKind::ProjectNewChat { proj_idx: 0, .. }
        ));
        assert_eq!(menu.items(), crate::app::projects::CHAT_AGENTS);
        assert_eq!(
            menu.list.highlighted, 2,
            "current default (gemini) highlighted"
        );
        assert_eq!(app.state.mode, Mode::ContextMenu);
    }

    // C6a: right click on the project header opens the same selector — the
    // guaranteed trigger for terminals that swallow shift+click.
    #[test]
    fn right_clicking_project_header_opens_agent_selector() {
        let mut app = projects_tab_app(vec![test_chat("sess-1")]);
        let header = project_row_rect(&app, |kind| {
            matches!(
                kind,
                crate::app::state::ProjectRowKind::Project { proj_idx: 0 }
            )
        });

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Right),
            header.x + 2,
            header.y,
        ));

        assert!(matches!(
            app.state.context_menu.as_ref().map(|menu| &menu.kind),
            Some(crate::app::state::ContextMenuKind::ProjectNewChat { proj_idx: 0, .. })
        ));
        assert_eq!(app.state.mode, Mode::ContextMenu);
    }

    // FEAT-B: clicking the footer "actives" label flips the filter.
    #[test]
    fn clicking_footer_actives_toggle_flips_the_filter() {
        let mut app = projects_tab_app(vec![test_chat("sess-1")]);
        let toggle = app.state.sidebar_actives_toggle_rect();
        assert!(toggle.width > 0, "toggle must fit in the test footer");
        assert!(!app.state.projects_actives_only);

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            toggle.x + 1,
            toggle.y,
        ));

        assert!(app.state.projects_actives_only, "click turns the filter on");
    }

    // TP-FOCUS-SW-04: the footer slot belongs to whichever tab is showing.
    // On Spaces the click flips the tree's focus and leaves the Projects
    // filter exactly where it was — one rectangle, two owners, no crosstalk.
    #[test]
    fn clicking_the_footer_toggle_on_spaces_flips_focus_only() {
        let mut app = projects_tab_app(vec![test_chat("sess-1")]);
        app.state.sidebar_tab = crate::app::state::SidebarTab::Spaces;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let toggle = app.state.sidebar_focus_toggle_rect();
        assert!(toggle.width > 0, "toggle must fit in the test footer");
        assert!(!app.state.spaces_focus_only);
        assert!(!app.state.projects_actives_only);

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            toggle.x + 1,
            toggle.y,
        ));

        assert!(app.state.spaces_focus_only, "the click narrows this tree");
        assert!(
            !app.state.projects_actives_only,
            "the neighbouring tab's filter is not this button's business"
        );

        // And back: a switch that cannot be switched off is not a switch.
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            toggle.x + 1,
            toggle.y,
        ));
        assert!(!app.state.spaces_focus_only);
    }

    // TP-FOCUS-SW-04 (the other half): the same rectangle on the Projects tab
    // still means "actives", and never touches the tree's focus.
    #[test]
    fn clicking_the_footer_toggle_on_projects_leaves_focus_alone() {
        let mut app = projects_tab_app(vec![test_chat("sess-1")]);
        let toggle = app.state.sidebar_actives_toggle_rect();
        assert!(toggle.width > 0);

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            toggle.x + 1,
            toggle.y,
        ));

        assert!(app.state.projects_actives_only);
        assert!(
            !app.state.spaces_focus_only,
            "the Projects filter never narrows the Spaces tree"
        );
    }

    // FEAT-A: with the project also open as a workspace, the same menu grows
    // that workspace's worktree actions (mirroring the Spaces context menu).
    #[test]
    fn right_clicking_project_with_open_workspace_offers_worktree_actions() {
        let mut app = projects_tab_app(vec![test_chat("sess-1")]);
        app.state.workspaces[0].identity_cwd = app.state.projects_sessions[0].path.clone();
        let header = project_row_rect(&app, |kind| {
            matches!(
                kind,
                crate::app::state::ProjectRowKind::Project { proj_idx: 0 }
            )
        });

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Right),
            header.x + 2,
            header.y,
        ));

        let menu = app.state.context_menu.as_ref().expect("menu open");
        assert!(matches!(
            menu.kind,
            crate::app::state::ContextMenuKind::ProjectNewChat {
                proj_idx: 0,
                has_workspace: true
            }
        ));
        assert_eq!(
            menu.items(),
            crate::app::projects::PROJECT_CHAT_MENU_WITH_WORKTREES
        );
    }

    // C6b (no-happy-path): a right click on a chat row is inert AND must not
    // fall through to the invisible workspace-card menu underneath.
    #[test]
    fn right_clicking_chat_row_on_projects_tab_is_inert() {
        let mut app = projects_tab_app(vec![test_chat("sess-1")]);
        let chat = project_row_rect(&app, |kind| {
            matches!(
                kind,
                crate::app::state::ProjectRowKind::Chat {
                    proj_idx: 0,
                    chat_idx: 0
                }
            )
        });

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Right),
            chat.x + 2,
            chat.y,
        ));

        assert!(
            app.state.context_menu.is_none(),
            "no chat menu yet — and never the workspace menu"
        );
        assert_ne!(app.state.mode, Mode::ContextMenu);
    }

    // C7: picking an agent from the selector (API path, same as a mouse
    // click) makes it the default and queues the new chat in that project.
    #[test]
    fn selecting_agent_from_menu_sets_default_and_queues_chat() {
        let mut app = projects_tab_app(vec![test_chat("sess-1")]);
        app.state.open_project_new_chat_menu(0, 5, 5);
        let menu = app.state.context_menu.take().expect("selector open");
        let codex_idx = menu
            .items()
            .iter()
            .position(|item| *item == "codex")
            .expect("codex listed");

        app.apply_context_menu_action_via_api(menu, codex_idx);

        assert_eq!(app.state.default_chat_agent, "codex");
        assert_eq!(
            app.state.request_project_chat_tab,
            Some(crate::app::state::ProjectChatTabRequest {
                project_path: std::path::PathBuf::from("/home/x/proj"),
                session_id: None,
            })
        );
        assert_ne!(app.state.mode, Mode::ContextMenu, "selector closed");
        // Persisting goes through save_default_chat_agent → update_config_file,
        // which is a guarded no-op in tests without CONFIG_PATH_ENV_VAR.
    }

    // T5a-6 (regression): the Task #4 behavior — clicking the project header
    // row still toggles collapse and must NOT queue a chat request.
    #[test]
    fn clicking_project_header_still_toggles_collapse_only() {
        let mut app = projects_tab_app(vec![test_chat("sess-1")]);
        let rect = project_row_rect(&app, |kind| {
            matches!(
                kind,
                crate::app::state::ProjectRowKind::Project { proj_idx: 0 }
            )
        });

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            rect.x + 2,
            rect.y,
        ));

        assert!(app
            .state
            .collapsed_project_paths
            .contains(std::path::Path::new("/home/x/proj")));
        assert_eq!(app.state.request_project_chat_tab, None);
    }

    /// A state whose daily directory holds three chats and whose workspace
    /// lives somewhere else — the shape the section exists for.
    fn state_with_daily_chats() -> (crate::app::state::AppState, std::path::PathBuf) {
        let mut state = crate::app::state::AppState::test_new();
        let daily = std::path::PathBuf::from("/home/tester");
        let mut ws = Workspace::test_new("elsewhere");
        ws.identity_cwd = std::path::PathBuf::from("/repo/checkout");
        state.workspaces = vec![ws];
        state.active = Some(0);
        state.daily_chat_cwd = Some(daily.clone());
        let key = crate::persist::workspace_chats::ledger_key(&daily);
        state.workspace_chat_rows.insert(
            key,
            (0..3)
                .map(|idx| crate::app::state::WorkspaceChatRow {
                    session_id: format!("daily-{idx}"),
                    agent: "claude".to_string(),
                    title: Some(format!("daily chat {idx}")),
                    last_seen_ms: 10 + idx as u64,
                    last_modified: None,
                    last_message_at: None,
                })
                .collect(),
        );
        (state, daily)
    }

    // TP-DAILY-11: a fresh chat from the section starts in the daily
    // directory with no session to resume. Rooting it at the active workspace
    // would put the conversation in a checkout the person was not looking at —
    // the same substitution TP-DAILY-07 forbids for resumes.
    #[test]
    fn a_new_daily_chat_starts_in_the_daily_directory() {
        let (mut state, daily) = state_with_daily_chats();
        state.request_daily_chat();
        let request = state
            .request_project_chat_tab
            .as_ref()
            .expect("a fresh chat is queued");
        assert_eq!(request.project_path, daily);
        assert_eq!(
            request.session_id, None,
            "a new chat resumes nothing; a session id here would reopen an old one"
        );

        // A client with no home asks for nothing rather than for `/`.
        let (mut homeless, _) = state_with_daily_chats();
        homeless.daily_chat_cwd = None;
        homeless.request_daily_chat();
        assert!(homeless.request_project_chat_tab.is_none());
    }

    // TP-DAILY-11: the menu offers the agents and nothing else. The daily
    // directory is not a checkout, so a worktree verb would be an offer the
    // tree cannot keep — and the highlighted row is the persisted default, so
    // the common case is one press away.
    #[test]
    fn the_daily_plus_offers_agents_only_and_starts_on_the_default() {
        let (mut state, _) = state_with_daily_chats();
        state.default_chat_agent = crate::app::projects::CHAT_AGENTS
            .last()
            .expect("agents exist")
            .to_string();
        state.open_daily_new_chat_menu(4, 2);
        let menu = state.context_menu.as_ref().expect("the menu opens");
        assert!(matches!(
            menu.kind,
            crate::app::state::ContextMenuKind::DailyNewChat
        ));
        assert_eq!(
            menu.items(),
            crate::app::projects::CHAT_AGENTS.to_vec(),
            "no worktree verbs: the daily directory is not a checkout"
        );
        assert_eq!(
            menu.list.highlighted,
            crate::app::projects::CHAT_AGENTS.len() - 1,
            "the menu opens on the persisted default"
        );
    }

    // TP-DAILY-07: the request a daily row queues is rooted at the daily
    // directory. Substituting the active workspace's path is #46 with the
    // roles reversed — the chat would resume somewhere it never ran.
    #[test]
    fn a_daily_chat_resumes_in_the_daily_directory() {
        let (mut state, daily) = state_with_daily_chats();
        state.open_daily_chat("daily-1");
        let request = state
            .request_project_chat_tab
            .as_ref()
            .expect("a dormant chat is queued");
        assert_eq!(request.project_path, daily);
        assert_eq!(request.session_id.as_deref(), Some("daily-1"));
    }

    // TP-DAILY-07: the same contract `open_workspace_chat` keeps — a session
    // already running in a tab is switched to, not resumed a second time.
    // Two tabs on one conversation is #45's complaint in another surface.
    #[test]
    fn a_daily_chat_already_running_is_switched_to_rather_than_resumed() {
        let (mut state, _) = state_with_daily_chats();
        let second = state.workspaces[0].test_add_tab(Some("live"));
        state.workspaces[0].tabs[second].resumed_session_id = Some("daily-2".into());
        state.workspaces[0].set_active_tab(0);

        state.open_daily_chat("daily-2");

        assert_eq!(
            state.request_project_chat_tab, None,
            "nothing is queued when the chat already has a tab"
        );
        assert_eq!(state.workspaces[0].active_tab_index(), second);
        assert_eq!(state.mode, crate::app::Mode::Terminal);
    }

    /// A module holding one filed chat, with or without a directory.
    fn state_with_module_chat(dir: Option<std::path::PathBuf>) -> crate::app::state::AppState {
        let mut state = crate::app::state::AppState::test_new();
        state.space_nodes = vec![crate::spaces::SpaceNode {
            key: "docs".into(),
            name: "Docs".into(),
            icon: None,
            parent: None,
            dir,
        }];
        state.workspace_chat_rows.insert(
            crate::persist::workspace_chats::module_ledger_key("docs"),
            vec![crate::app::state::WorkspaceChatRow {
                session_id: "filed-session".into(),
                agent: "claude".into(),
                title: Some("a filed conversation".into()),
                last_seen_ms: 1,
                last_modified: None,
                last_message_at: None,
            }],
        );
        state
    }

    /// The same fixture, but the module is a BUCKET rather than a node.
    ///
    /// M1.11: buckets are twenty of the twenty-four modules on the machine this
    /// was reported from, so this is the shape the feature is actually used in.
    fn state_with_bucket_chat(repo_root: std::path::PathBuf) -> crate::app::state::AppState {
        let mut state = crate::app::state::AppState::test_new();
        state.space_nodes.clear();
        state.space_split_rules = vec![crate::spaces::SpaceSplitRule {
            repo_root,
            patterns: vec!["*".to_string()],
            key: "bucket".to_string(),
            label: "Bucket".to_string(),
            icon: None,
            parent: None,
            passive_color: None,
        }];
        state.workspace_chat_rows.insert(
            crate::persist::workspace_chats::module_ledger_key("bucket"),
            vec![crate::app::state::WorkspaceChatRow {
                session_id: "filed-session".into(),
                agent: "claude".into(),
                title: Some("a filed conversation".into()),
                last_seen_ms: 1,
                last_modified: None,
                last_message_at: None,
            }],
        );
        state
    }

    // M1.11 / TP-MOD-36: a chat filed into a BUCKET reopens too, in the
    // repository its rule names. Resolving through `space_nodes` alone answered
    // "no directory" for every bucket, so the move would have succeeded and the
    // chat would then have been unreachable — moved into a place that could
    // never open it.
    #[test]
    fn a_filed_chat_in_a_bucket_resumes_in_its_repository() {
        let dir = std::env::temp_dir();
        let mut state = state_with_bucket_chat(dir.clone());

        state.open_module_chat("bucket", "filed-session");

        let request = state
            .request_project_chat_tab
            .as_ref()
            .expect("the chat is queued to reopen");
        assert_eq!(request.project_path, dir);
        assert_eq!(request.session_id.as_deref(), Some("filed-session"));
    }

    // TP-CHAT-MOVE-10 (R1): a dead chat filed into a module reopens in that
    // module's directory. This is the boundary TP-CHAT-MOVE-07 drew, opened
    // from the side it was meant to open from — the directory is a fact the
    // person stated (TP-MOD-33), not one the machine guessed.
    #[test]
    fn a_filed_chat_resumes_in_the_modules_directory() {
        let dir = std::env::temp_dir();
        let mut state = state_with_module_chat(Some(dir.clone()));

        state.open_module_chat("docs", "filed-session");

        let request = state
            .request_project_chat_tab
            .as_ref()
            .expect("the chat is queued to reopen");
        assert_eq!(request.project_path, dir);
        assert_eq!(request.session_id.as_deref(), Some("filed-session"));
    }

    // TP-CHAT-MOVE-10 (R2): a module with no directory still refuses. The
    // ledger records which module a chat belongs to, never where it came
    // from, so resuming without a stated directory would mean inventing one —
    // and #46 measured where invented directories land ($HOME).
    #[test]
    fn a_filed_chat_in_a_module_without_a_directory_stays_put() {
        let mut state = state_with_module_chat(None);

        state.open_module_chat("docs", "filed-session");

        assert_eq!(
            state.request_project_chat_tab, None,
            "refusing beats guessing a working directory"
        );
    }

    // TP-CHAT-MOVE-10 (R3): the directory is checked again at open time. It
    // was validated when it was written, but a worktree can be pruned and a
    // disk unmounted afterwards — and a pane the shell cannot enter reads as
    // the chat being broken rather than the target being gone.
    #[test]
    fn a_filed_chat_refuses_a_directory_that_has_since_gone() {
        let gone = std::env::temp_dir().join("herdr-module-dir-that-never-existed");
        let mut state = state_with_module_chat(Some(gone));

        state.open_module_chat("docs", "filed-session");

        assert_eq!(state.request_project_chat_tab, None);
    }

    // TP-CHATROW-ID-01: the drawn row IS the chat — a press resolves by the
    // rect's own session id, so a ledger that gained a row between two
    // frames cannot make the press open whichever chat shifted into the old
    // position. The daily row is the surface the polls race hardest.
    #[test]
    fn a_stale_daily_row_press_opens_the_chat_the_user_saw() {
        let (mut state, daily) = state_with_daily_chats();
        // The frame drew "daily-1"; before the press arrives the ledger
        // gains a new head row, shifting every position by one.
        let key = crate::persist::workspace_chats::ledger_key(&daily);
        let rows = state.workspace_chat_rows.get_mut(&key).expect("daily rows");
        rows.insert(
            0,
            crate::app::state::WorkspaceChatRow {
                session_id: "daily-new".to_string(),
                agent: "claude".to_string(),
                title: Some("fresh head".to_string()),
                last_seen_ms: 99,
                last_modified: None,
                last_message_at: None,
            },
        );

        state.open_daily_chat("daily-1");

        let request = state
            .request_project_chat_tab
            .as_ref()
            .expect("the press still opens a chat");
        assert_eq!(
            request.session_id.as_deref(),
            Some("daily-1"),
            "the chat the user saw opens — never the neighbour that shifted in"
        );
    }

    // TP-DAILY-07: identity, not position — a chat that left the ledger
    // between the frame and the press answers with nothing rather than with
    // the wrong chat, and never panics.
    #[test]
    fn a_click_on_a_daily_row_that_no_longer_exists_does_nothing() {
        let (mut state, _) = state_with_daily_chats();
        state.open_daily_chat("no-such-session");
        assert_eq!(state.request_project_chat_tab, None);

        let mut homeless = crate::app::state::AppState::test_new();
        homeless.open_daily_chat("daily-0");
        assert_eq!(homeless.request_project_chat_tab, None);
    }

    // TP-DAILY-03/04: both switches are per-display registers, and the "open
    // the rest" one also moves the read budget — an opened section still
    // parsed at the glance limit promises older chats it can never list.
    #[test]
    fn the_daily_switches_toggle_both_ways_and_carry_the_read_budget() {
        let (mut state, daily) = state_with_daily_chats();
        let key = crate::persist::workspace_chats::ledger_key(&daily);

        state.toggle_daily_section();
        assert!(state.daily_section_collapsed);
        state.toggle_daily_section();
        assert!(!state.daily_section_collapsed, "a fold folds back");

        state.toggle_full_daily_drawer();
        assert!(state.daily_section_expanded);
        assert!(
            state.fully_open_chat_drawers.contains(&key),
            "the read budget follows the switch"
        );
        state.toggle_full_daily_drawer();
        assert!(!state.daily_section_expanded);
        assert!(!state.fully_open_chat_drawers.contains(&key));
    }

    // TP-WSID-02: the chat a "+" starts lives where the row says it will —
    // the checkout — never in the directory the workspace was born in.
    #[test]
    fn a_new_chat_request_carries_the_checkout_not_the_birthplace() {
        let mut state = crate::app::state::AppState::test_new();
        let mut ws = crate::workspace::Workspace::test_new("adopted");
        ws.identity_cwd = std::path::PathBuf::from("/home/user");
        ws.worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "repo-key".into(),
            label: "herdr".into(),
            repo_root: std::path::PathBuf::from("/repo/herdr"),
            checkout_path: std::path::PathBuf::from("/repo/herdr-branch"),
            is_linked_worktree: true,
        });
        state.workspaces = vec![ws];

        state.request_workspace_chat(0);

        assert_eq!(
            state
                .request_project_chat_tab
                .as_ref()
                .map(|req| req.project_path.clone()),
            Some(std::path::PathBuf::from("/repo/herdr-branch"))
        );
    }
}
