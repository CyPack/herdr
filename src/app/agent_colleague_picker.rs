//! Client-local "Work with other agent..." picker: see every OTHER live
//! agent, tagged by how near it lives — same branch, same space, or a
//! different space — and jump to the one you mean (TP-AGPANEL-48). Opening
//! and selecting perform no runtime work; the jump rides the same focus
//! roads every menu verb takes.

/// How near a colleague lives to the agent the menu was opened on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ColleagueRelation {
    SameBranch,
    SameSpace,
    Elsewhere,
}

impl ColleagueRelation {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::SameBranch => "same branch",
            Self::SameSpace => "same space",
            Self::Elsewhere => "different space",
        }
    }
}

/// One selectable colleague. `live` is false only for the placeholder row a
/// lonely agent sees — a dead-end action is explained, never silent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentColleagueRow {
    pub label: String,
    pub relation: Option<ColleagueRelation>,
    pub ws_idx: usize,
    pub tab_idx: usize,
    pub live: bool,
}

/// Blocking client-local picker state. Owns no watcher, worker, process,
/// pane, or server state; closing it discards only presentation data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentColleaguePickerState {
    pub rows: Vec<AgentColleagueRow>,
    pub selected: usize,
}

impl crate::app::state::AppState {
    /// Centered popup rect over the terminal area — the reference picker's
    /// geometry, byte for byte, because the two are one kind of surface.
    pub(crate) fn agent_colleague_picker_popup_rect(&self) -> Option<ratatui::layout::Rect> {
        let picker = self.agent_colleague_picker.as_ref()?;
        let area = self.view.terminal_area;
        let width = 44u16.min(area.width.saturating_sub(2)).max(4);
        let height = (picker.rows.len() as u16)
            .saturating_add(4)
            .min(area.height.saturating_sub(2))
            .max(4);
        if area.width < 8 || area.height < 6 {
            return None;
        }
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;
        Some(ratatui::layout::Rect::new(x, y, width, height))
    }

    pub(crate) fn agent_colleague_picker_row_hit_areas(&self) -> Vec<ratatui::layout::Rect> {
        let Some(picker) = self.agent_colleague_picker.as_ref() else {
            return Vec::new();
        };
        let Some(popup) = self.agent_colleague_picker_popup_rect() else {
            return Vec::new();
        };
        let inner = ratatui::layout::Rect::new(
            popup.x + 1,
            popup.y + 3,
            popup.width.saturating_sub(2),
            popup.height.saturating_sub(4),
        );
        picker
            .rows
            .iter()
            .enumerate()
            .take(inner.height as usize)
            .map(|(idx, _)| {
                ratatui::layout::Rect::new(inner.x, inner.y + idx as u16, inner.width, 1)
            })
            .collect()
    }

    /// Row index under one exact cell, or None outside every row.
    pub(crate) fn agent_colleague_picker_row_at(&self, column: u16, row: u16) -> Option<usize> {
        self.agent_colleague_picker_row_hit_areas()
            .iter()
            .position(|rect| {
                column >= rect.x && column < rect.right() && row >= rect.y && row < rect.bottom()
            })
    }

    /// Close the picker and restore the pre-overlay focus owner.
    pub(crate) fn close_agent_colleague_picker(&mut self) {
        if self.agent_colleague_picker.take().is_some() {
            crate::app::input::leave_modal(self);
        }
    }

    /// The rows the picker shows for a menu opened on `source_ws`/`source_tab`:
    /// every OTHER agent, nearest relation first. TP-AGPANEL-48: the relation
    /// reads the same answers the sidebar stands on — the workspace index for
    /// the branch, `effective_space` for the space — so the tag and the tree
    /// can never disagree.
    pub(crate) fn agent_colleague_rows(
        &self,
        source_ws: usize,
        source_tab: usize,
    ) -> Vec<AgentColleagueRow> {
        self.classify_colleagues(crate::ui::agent_panel_entries(self), source_ws, source_tab)
    }

    /// The classification core, split from the projection so a test can hand
    /// it synthetic entries without standing up agent terminals.
    pub(crate) fn classify_colleagues(
        &self,
        entries: Vec<crate::ui::AgentPanelEntry>,
        source_ws: usize,
        source_tab: usize,
    ) -> Vec<AgentColleagueRow> {
        let source_space = crate::ui::effective_space(self, source_ws).map(|space| space.key);
        let mut rows: Vec<AgentColleagueRow> = entries
            .into_iter()
            .filter(|entry| !(entry.ws_idx == source_ws && entry.tab_idx == source_tab))
            .map(|entry| {
                let relation = if entry.ws_idx == source_ws {
                    ColleagueRelation::SameBranch
                } else if source_space.is_some()
                    && crate::ui::effective_space(self, entry.ws_idx).map(|space| space.key)
                        == source_space
                {
                    ColleagueRelation::SameSpace
                } else {
                    ColleagueRelation::Elsewhere
                };
                AgentColleagueRow {
                    label: entry
                        .agent_label
                        .clone()
                        .unwrap_or_else(|| entry.primary_label.clone()),
                    relation: Some(relation),
                    ws_idx: entry.ws_idx,
                    tab_idx: entry.tab_idx,
                    live: true,
                }
            })
            .collect();
        rows.sort_by_key(|row| row.relation);
        rows
    }
}

impl crate::app::App {
    /// Open the picker for the agent the menu was opened on. A lonely agent
    /// gets the honest placeholder rather than a verb that silently does
    /// nothing.
    pub(crate) fn open_agent_colleague_picker(&mut self, source_ws: usize, source_tab: usize) {
        let mut rows = self.state.agent_colleague_rows(source_ws, source_tab);
        if rows.is_empty() {
            rows.push(AgentColleagueRow {
                label: "(no other agents running)".to_string(),
                relation: None,
                ws_idx: source_ws,
                tab_idx: source_tab,
                live: false,
            });
        }
        self.state.agent_colleague_picker = Some(AgentColleaguePickerState { rows, selected: 0 });
        self.state
            .enter_overlay_mode(crate::app::state::Mode::AgentColleaguePicker);
    }

    /// Jump to the selected colleague on the same focus roads every menu
    /// verb takes, then close.
    pub(crate) fn activate_agent_colleague_picker_selection(&mut self) -> bool {
        let Some((ws_idx, tab_idx)) =
            self.state
                .agent_colleague_picker
                .as_ref()
                .and_then(|picker| {
                    picker
                        .rows
                        .get(picker.selected)
                        .filter(|row| row.live)
                        .map(|row| (row.ws_idx, row.tab_idx))
                })
        else {
            return false;
        };
        self.state.close_agent_colleague_picker();
        self.focus_workspace_idx_via_api(ws_idx);
        self.focus_tab_idx_via_api(tab_idx);
        true
    }

    pub(crate) fn handle_agent_colleague_picker_key(&mut self, key: crossterm::event::KeyEvent) {
        match key.code {
            crossterm::event::KeyCode::Esc => self.state.close_agent_colleague_picker(),
            crossterm::event::KeyCode::Enter => {
                let _ = self.activate_agent_colleague_picker_selection();
            }
            crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => {
                if let Some(picker) = self.state.agent_colleague_picker.as_mut() {
                    picker.selected = picker.selected.saturating_sub(1);
                }
            }
            crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') => {
                if let Some(picker) = self.state.agent_colleague_picker.as_mut() {
                    if picker.selected.saturating_add(1) < picker.rows.len() {
                        picker.selected += 1;
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::{AppState, Mode};
    use crate::workspace::Workspace;

    fn entry(ws_idx: usize, tab_idx: usize, label: &str) -> crate::ui::AgentPanelEntry {
        crate::ui::AgentPanelEntry {
            ws_idx,
            tab_idx,
            pane_id: crate::layout::PaneId::alloc(),
            primary_label: label.to_string(),
            primary_tab_label: None,
            pane_label: None,
            terminal_title: None,
            terminal_title_stripped: None,
            agent_label: None,
            agent_kind_label: None,
            agent: None,
            state: crate::detect::AgentState::Working,
            seen: true,
            last_agent_state_change_seq: None,
            state_labels: Default::default(),
            tokens: Default::default(),
        }
    }

    // TP-AGPANEL-48: the source agent is excluded, a same-workspace
    // colleague is tagged "same branch", every other workspace without a
    // shared space is "different space", and the nearest relation sorts
    // first.
    #[test]
    fn colleagues_are_classified_and_sorted_by_nearness() {
        let mut state = AppState::test_new();
        state.workspaces = vec![Workspace::test_new("one"), Workspace::test_new("two")];

        let rows = state.classify_colleagues(
            vec![entry(1, 0, "far"), entry(0, 1, "near"), entry(0, 0, "me")],
            0,
            0,
        );

        assert_eq!(
            rows.iter()
                .map(|row| row.label.as_str())
                .collect::<Vec<_>>(),
            vec!["near", "far"],
            "the source row is excluded and the same-branch colleague leads"
        );
        assert_eq!(rows[0].relation, Some(ColleagueRelation::SameBranch));
        assert_eq!(rows[1].relation, Some(ColleagueRelation::Elsewhere));
    }

    // TP-AGPANEL-48: a lonely agent gets the honest placeholder — a
    // dead-end action is explained, never silent — and Enter on it goes
    // nowhere.
    #[test]
    fn a_lonely_agent_sees_the_placeholder_and_enter_does_nothing() {
        let mut app = crate::app::App::new(
            &crate::config::Config::default(),
            true,
            None,
            tokio::sync::mpsc::unbounded_channel().1,
            crate::api::EventHub::default(),
        );
        app.state.workspaces = vec![Workspace::test_new("one")];
        app.state.active = Some(0);

        app.open_agent_colleague_picker(0, 0);

        let picker = app
            .state
            .agent_colleague_picker
            .as_ref()
            .expect("the picker opened");
        assert_eq!(picker.rows.len(), 1);
        assert!(!picker.rows[0].live);
        assert_eq!(app.state.mode, Mode::AgentColleaguePicker);

        assert!(!app.activate_agent_colleague_picker_selection());
        assert!(
            app.state.agent_colleague_picker.is_some(),
            "a dead row does not close the popup out from under the reader"
        );
    }

    // TP-AGPANEL-48: activating a live row jumps to that colleague's tab on
    // the same focus roads every menu verb takes, and closes the picker.
    #[test]
    fn activating_a_colleague_jumps_to_its_tab() {
        let mut app = crate::app::App::new(
            &crate::config::Config::default(),
            true,
            None,
            tokio::sync::mpsc::unbounded_channel().1,
            crate::api::EventHub::default(),
        );
        let mut two = Workspace::test_new("two");
        two.test_add_tab(None);
        app.state.workspaces = vec![Workspace::test_new("one"), two];
        app.state.active = Some(0);
        app.state.agent_colleague_picker = Some(AgentColleaguePickerState {
            rows: vec![AgentColleagueRow {
                label: "peer".to_string(),
                relation: Some(ColleagueRelation::Elsewhere),
                ws_idx: 1,
                tab_idx: 1,
                live: true,
            }],
            selected: 0,
        });
        app.state.mode = Mode::AgentColleaguePicker;

        assert!(app.activate_agent_colleague_picker_selection());
        assert_eq!(app.state.active, Some(1));
        assert_eq!(app.state.workspaces[1].active_tab_index(), 1);
        assert!(app.state.agent_colleague_picker.is_none());
    }
}
