use ratatui::layout::Rect;

use crate::app::state::{AppState, SidebarTab, ViewLayout};

use super::ScrollbarClickTarget;

impl AppState {
    pub(super) fn workspace_list_rect(&self) -> Rect {
        let sidebar = self.view.sidebar_rect;
        if self.sidebar_collapsed || sidebar.width <= 1 || sidebar.height == 0 {
            return Rect::default();
        }
        crate::ui::workspace_list_rect(sidebar, self.sidebar_section_split)
    }

    pub(super) fn agent_panel_rect(&self) -> Rect {
        let sidebar = self.view.sidebar_rect;
        if self.sidebar_collapsed || sidebar.width <= 1 || sidebar.height == 0 {
            return Rect::default();
        }
        let (_, detail_area) =
            crate::ui::expanded_sidebar_sections(sidebar, self.sidebar_section_split);
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
        let y = ws_area.y + ws_area.height.saturating_sub(1);
        Rect::new(ws_area.x, y, ws_area.width, 1)
    }

    pub(crate) fn sidebar_new_button_rect(&self) -> Rect {
        let footer = self.sidebar_footer_rect();
        let width = 5u16.min(footer.width.max(1));
        Rect::new(footer.x, footer.y, width, footer.height)
    }

    /// The footer "actives" toggle, centered between the chat and menu
    /// buttons; collapses to an empty rect when the footer is too narrow to
    /// keep all three hit areas disjoint.
    pub(crate) fn sidebar_actives_toggle_rect(&self) -> Rect {
        let footer = self.sidebar_footer_rect();
        if footer.width == 0 || footer.height == 0 {
            return Rect::default();
        }
        let chat = self.sidebar_new_button_rect();
        let menu = self.global_launcher_rect();
        let label_w: u16 = 7; // "actives"
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
            return self.view.mobile_menu_hit_area;
        }

        let footer = self.sidebar_footer_rect();
        let width = if self.global_menu_attention_badge_visible() {
            8
        } else {
            6
        }
        .min(footer.width.max(1));
        let x = footer.x + footer.width.saturating_sub(width);
        Rect::new(x, footer.y, width, footer.height)
    }

    /// The Projects-tab row (if any) whose laid-out rect contains `(col, row)`.
    pub(super) fn project_row_kind_at(
        &self,
        col: u16,
        row: u16,
    ) -> Option<crate::app::state::ProjectRowKind> {
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
        let toggle = crate::ui::expanded_sidebar_toggle_rect(sidebar);
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
            crate::ui::expanded_sidebar_toggle_rect(self.view.sidebar_rect)
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

    pub(super) fn workspace_drop_index_at_row(&self, row: u16) -> Option<usize> {
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
        if cards.is_empty() {
            return Some(0);
        }

        let mut insert_indices = Vec::with_capacity(cards.len() + 1);
        for (idx, card) in cards.iter().enumerate() {
            let card_group = self
                .workspaces
                .get(card.ws_idx)
                .and_then(|ws| ws.worktree_space())
                .map(|space| space.key.as_str());
            let previous_group = idx.checked_sub(1).and_then(|prev_idx| {
                self.workspaces
                    .get(cards[prev_idx].ws_idx)
                    .and_then(|ws| ws.worktree_space())
                    .map(|space| space.key.as_str())
            });
            let inside_group_gap = card_group.is_some() && card_group == previous_group;
            if !inside_group_gap {
                insert_indices.push(card.ws_idx);
            }
        }
        insert_indices.push(cards.last().map(|card| card.ws_idx + 1).unwrap_or(0));

        let mut best: Option<(usize, u16)> = None;
        for insert_idx in insert_indices {
            let Some(slot_row) = crate::ui::workspace_drop_indicator_row(&cards, area, insert_idx)
            else {
                continue;
            };
            let distance = row.abs_diff(slot_row);
            match best {
                Some((best_idx, best_distance))
                    if distance > best_distance
                        || (distance == best_distance && insert_idx < best_idx) => {}
                _ => best = Some((insert_idx, distance)),
            }
        }

        best.map(|(insert_idx, _)| insert_idx)
    }

    pub(super) fn on_agent_panel_sort_toggle(&self, col: u16, row: u16) -> bool {
        if self.sidebar_collapsed {
            return false;
        }

        let (_, detail_area) = crate::ui::expanded_sidebar_sections(
            self.view.sidebar_rect,
            self.sidebar_section_split,
        );
        let rect = crate::ui::agent_panel_toggle_rect(detail_area, self.agent_panel_sort);
        rect.width > 0
            && col >= rect.x
            && col < rect.x + rect.width
            && row >= rect.y
            && row < rect.y + rect.height
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
        if body.height < 2 || row < body.y || row >= body.y + body.height {
            return None;
        }

        let mut row_y = body.y;
        for detail in crate::ui::agent_panel_entries(self)
            .into_iter()
            .skip(self.agent_panel_scroll)
        {
            if row_y.saturating_add(1) >= body.y + body.height {
                break;
            }
            if row == row_y || row == row_y + 1 {
                return Some((detail.ws_idx, detail.tab_idx, detail.pane_id));
            }
            row_y = row_y.saturating_add(2);
            if row_y < body.y + body.height {
                row_y = row_y.saturating_add(1);
            }
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

        assert_eq!(app.state.workspaces[0].active_tab, 1);
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
        );
        let toggle = crate::ui::agent_panel_toggle_rect(detail_area, app.state.agent_panel_sort);
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
        );
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            detail_area.x + 2,
            detail_area.y + 6,
        ));

        assert_eq!(app.state.active, Some(1));
        assert_eq!(app.state.selected, 1);
        assert_eq!(app.state.workspaces[1].active_tab, 0);
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
                msg_count: 1,
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
        app.state.agent_panel_scroll = 1;

        let detail_area = app.state.agent_panel_rect();
        let body = crate::ui::agent_panel_body_rect(detail_area, true);
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            body.x + 1,
            body.y,
        ));

        assert_eq!(app.state.workspaces[0].active_tab, second_tab);
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

        assert_eq!(app.state.workspaces[0].active_tab, 1);
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

        let toggle = crate::ui::expanded_sidebar_toggle_rect(app.state.view.sidebar_rect);
        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            toggle.x,
            toggle.y,
        ));

        assert!(app.state.sidebar_collapsed);
        assert!(app.state.session_dirty);
        assert!(app.state.drag.is_none());
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
        assert!(app.state.workspace_press.is_some());

        app.handle_mouse(mouse(MouseEventKind::Up(MouseButton::Left), 2, target_row));
        assert_eq!(app.state.active, Some(1));
        assert_eq!(app.state.selected, 1);
        assert!(app.state.workspace_press.is_none());
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
        let parent = app.state.view.workspace_card_areas[0].rect;

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            parent.x,
            parent.y,
        ));

        assert_eq!(app.state.active, None);
        assert!(app.state.workspace_press.is_none());
        assert!(app.state.collapsed_space_keys.contains("repo-key"));

        app.handle_mouse(mouse(
            MouseEventKind::Down(MouseButton::Left),
            parent.x,
            parent.y,
        ));

        assert!(!app.state.collapsed_space_keys.contains("repo-key"));
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
        let active_id = app.state.workspaces[1].id.clone();
        let selected_id = app.state.workspaces[2].id.clone();
        app.state.active = Some(1);
        app.state.selected = 2;
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));
        let source_row = app.state.view.workspace_card_areas[1].rect.y;
        let target_row = crate::ui::workspace_drop_indicator_row(
            &app.state.view.workspace_card_areas,
            app.state.workspace_list_rect(),
            0,
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
                insert_idx: Some(0),
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
        assert_eq!(app.state.workspaces[0].active_tab, 0);
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

        assert_eq!(app.state.workspaces[0].active_tab, last_idx);
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
        assert_eq!(app.state.workspaces[0].active_tab, 2);
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
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));

        assert_eq!(app.state.workspace_drop_index_at_row(0), Some(0));
        assert_eq!(app.state.workspace_drop_index_at_row(1), Some(0));
        assert_eq!(app.state.workspace_drop_index_at_row(2), Some(0));
        assert_eq!(app.state.workspace_drop_index_at_row(3), Some(1));

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
        crate::ui::compute_view(&mut app.state, Rect::new(0, 0, 106, 20));

        let cards = &app.state.view.workspace_card_areas;
        let bottom_slot = crate::ui::workspace_drop_indicator_row(
            cards,
            app.state.workspace_list_rect(),
            cards.len(),
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

        assert_eq!(app.state.workspace_drop_index_at_row(issue.rect.y), Some(1));
        assert_eq!(
            crate::ui::workspace_drop_indicator_row(cards, app.state.workspace_list_rect(), 2),
            Some(normal.rect.y + normal.rect.height)
        );
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
            &app.state.view.workspace_card_areas,
            app.state.workspace_list_rect(),
            0,
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
        assert!(app.state.file_manager.is_none());
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
        assert!(app.state.file_manager.is_none());
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
            Some(std::path::PathBuf::from("/home/a"))
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
            Some(std::path::PathBuf::from("/home/a/Downloads")),
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
            Some(target.clone()),
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
        assert!(app.state.workspace_press.is_none());
        assert_eq!(app.state.active, Some(0));
    }

    // ---- Projects tab row clicks (Task #5) --------------------------------

    fn test_chat(id: &str) -> crate::claude_sessions::ClaudeSession {
        crate::claude_sessions::ClaudeSession {
            id: id.to_string(),
            title: format!("chat {id}"),
            last_modified: std::time::SystemTime::UNIX_EPOCH,
            msg_count: 3,
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
        assert_eq!(app.state.workspaces[1].active_tab, tab_idx);
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

        assert_eq!(app.state.workspaces[0].active_tab, 1);
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
}
