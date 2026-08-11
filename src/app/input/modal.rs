use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
#[cfg(test)]
use ratatui::layout::Direction;
use ratatui::layout::Rect;

use crate::{
    app::{
        state::{
            AppState, ContextMenuKind, ContextMenuState, FileManagerContextActionIntent,
            FileManagerContextMenuModel, MenuListState, Mode, NavigatorStateFilter,
        },
        App,
    },
    input::TerminalKey,
    layout::NavDirection,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ModalAction {
    Continue,
    Save,
    Clear,
    Cancel,
    Confirm,
    Apply,
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ModalKeyBinding {
    Enter,
    Esc,
    CtrlC,
}

impl ModalKeyBinding {
    fn matches(self, key: &KeyEvent) -> bool {
        match self {
            Self::Enter => key.code == KeyCode::Enter,
            Self::Esc => key.code == KeyCode::Esc,
            Self::CtrlC => {
                key.code == KeyCode::Char('c')
                    && key.modifiers == crossterm::event::KeyModifiers::CONTROL
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ModalActionSpec<A> {
    pub action: A,
    pub bindings: &'static [ModalKeyBinding],
}

pub(super) fn modal_action_from_key<A: Copy>(
    key: &KeyEvent,
    specs: &[ModalActionSpec<A>],
) -> Option<A> {
    specs
        .iter()
        .find(|spec| spec.bindings.iter().any(|binding| binding.matches(key)))
        .map(|spec| spec.action)
}

pub(super) fn modal_action_from_buttons<A: Copy>(
    col: u16,
    row: u16,
    buttons: &[(Rect, A)],
) -> Option<A> {
    buttons.iter().find_map(|(rect, action)| {
        (col >= rect.x && col < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height)
            .then_some(*action)
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GlobalMenuAction {
    Detach,
    WhatsNew,
    Keybinds,
    ReloadConfig,
    Settings,
}

pub(crate) fn global_menu_actions(state: &AppState) -> Vec<GlobalMenuAction> {
    let mut actions = vec![
        GlobalMenuAction::Settings,
        GlobalMenuAction::Keybinds,
        GlobalMenuAction::ReloadConfig,
    ];
    if state.update_available.is_some() || state.latest_release_notes_available {
        actions.push(GlobalMenuAction::WhatsNew);
    }
    actions.push(GlobalMenuAction::Detach);
    actions
}

pub(super) fn open_global_menu(state: &mut AppState) {
    state.global_menu = MenuListState::new(0);
    state.enter_overlay_mode(Mode::GlobalMenu);
}

pub(super) fn open_keybind_help(state: &mut AppState) {
    state.keybind_help.scroll = 0;
    state.keybind_help.query.clear();
    state.keybind_help.search_focused = false;
    state.enter_overlay_mode(Mode::KeybindHelp);
}

fn open_update_release_notes(state: &mut AppState) {
    let Some(notes) = crate::release_notes::load_latest() else {
        return;
    };

    state.release_notes = Some(crate::app::state::ReleaseNotesState {
        version: notes.version,
        body: notes.body,
        scroll: 0,
        preview: notes.preview,
    });
    state.enter_overlay_mode(Mode::ReleaseNotes);
}

pub(super) fn request_detach(state: &mut AppState) {
    if state.detach_exits {
        state.should_quit = true;
    } else {
        state.detach_requested = true;
    }
}

pub(crate) fn apply_global_menu_action(state: &mut AppState, action: GlobalMenuAction) {
    match action {
        GlobalMenuAction::Detach => {
            leave_modal(state);
            request_detach(state);
        }
        GlobalMenuAction::WhatsNew => open_update_release_notes(state),
        GlobalMenuAction::Keybinds => open_keybind_help(state),
        GlobalMenuAction::ReloadConfig => {
            state.request_reload_config = true;
            leave_modal(state);
        }
        GlobalMenuAction::Settings => super::settings::open_settings(state),
    }
}

pub(crate) fn handle_global_menu_key(state: &mut AppState, key: KeyEvent) {
    let actions = global_menu_actions(state);
    match key.code {
        KeyCode::Esc => leave_modal(state),
        KeyCode::Up | KeyCode::Char('k') => state.global_menu.move_prev(),
        KeyCode::Down | KeyCode::Char('j') => state.global_menu.move_next(actions.len()),
        KeyCode::Enter => {
            if let Some(action) = actions.get(state.global_menu.highlighted).copied() {
                apply_global_menu_action(state, action);
            }
        }
        _ => {}
    }
}

pub(crate) fn handle_navigator_key(
    state: &mut AppState,
    terminal_runtimes: &crate::terminal::TerminalRuntimeRegistry,
    key: KeyEvent,
) {
    if state.navigator.search_focused {
        match key.code {
            KeyCode::Esc => {
                state.navigator.search_focused = false;
            }
            KeyCode::Enter => {
                state.accept_navigator_selection_from(terminal_runtimes);
            }
            KeyCode::Backspace => {
                state.navigator.state_filter = None;
                state.navigator.query.pop();
                state.select_first_navigator_match_from(terminal_runtimes);
            }
            KeyCode::Up => state.move_navigator_selection_from(terminal_runtimes, -1),
            KeyCode::Down => state.move_navigator_selection_from(terminal_runtimes, 1),
            KeyCode::Char('n') if key.modifiers == KeyModifiers::CONTROL => {
                state.move_navigator_selection_from(terminal_runtimes, 1)
            }
            KeyCode::Char('p') if key.modifiers == KeyModifiers::CONTROL => {
                state.move_navigator_selection_from(terminal_runtimes, -1)
            }
            KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => {
                state.navigator.query.clear();
                state.navigator.state_filter = None;
                state.clamp_navigator_selection_from(terminal_runtimes);
            }
            KeyCode::Char(c)
                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
            {
                insert_navigator_search_text(state, terminal_runtimes, &c.to_string());
            }
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Esc => {
            leave_modal(state);
        }
        KeyCode::Enter => {
            state.accept_navigator_selection_from(terminal_runtimes);
        }
        KeyCode::Char('/') => {
            state.navigator.state_filter = None;
            state.navigator.search_focused = true;
            state.clamp_navigator_selection_from(terminal_runtimes);
        }
        KeyCode::Backspace if state.navigator.state_filter.is_some() => {
            state.navigator.state_filter = None;
            state.clamp_navigator_selection_from(terminal_runtimes);
        }
        KeyCode::Char('a') if key.modifiers.is_empty() => {
            state.navigator.query.clear();
            state.navigator.state_filter = None;
            state.clamp_navigator_selection_from(terminal_runtimes);
        }
        KeyCode::Char('b') if key.modifiers.is_empty() => {
            state.navigator.query.clear();
            state.navigator.state_filter = Some(NavigatorStateFilter::Blocked);
            state.select_first_navigator_match_from(terminal_runtimes);
        }
        KeyCode::Char('w') if key.modifiers.is_empty() => {
            state.navigator.query.clear();
            state.navigator.state_filter = Some(NavigatorStateFilter::Working);
            state.select_first_navigator_match_from(terminal_runtimes);
        }
        KeyCode::Char('i') if key.modifiers.is_empty() => {
            state.navigator.query.clear();
            state.navigator.state_filter = Some(NavigatorStateFilter::Idle);
            state.select_first_navigator_match_from(terminal_runtimes);
        }
        KeyCode::Char('d') if key.modifiers.is_empty() => {
            state.navigator.query.clear();
            state.navigator.state_filter = Some(NavigatorStateFilter::Done);
            state.select_first_navigator_match_from(terminal_runtimes);
        }
        KeyCode::Char('j') | KeyCode::Down => {
            state.move_navigator_selection_from(terminal_runtimes, 1)
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.move_navigator_selection_from(terminal_runtimes, -1)
        }
        KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => state
            .move_navigator_selection_by_lines_from(
                terminal_runtimes,
                (state.navigator_body_rect().height / 2).max(1) as isize,
            ),
        KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => state
            .move_navigator_selection_by_lines_from(
                terminal_runtimes,
                -((state.navigator_body_rect().height / 2).max(1) as isize),
            ),
        KeyCode::Char(' ') => state.toggle_selected_navigator_workspace_from(terminal_runtimes),
        KeyCode::Home => {
            state.navigator.selected = 0;
            state.ensure_navigator_selection_visible_from(terminal_runtimes);
        }
        KeyCode::End | KeyCode::Char('G') => {
            state.navigator.selected = state
                .navigator_rows_from(terminal_runtimes)
                .len()
                .saturating_sub(1);
            state.ensure_navigator_selection_visible_from(terminal_runtimes);
        }
        _ => {}
    }
}

pub(crate) fn insert_navigator_search_text(
    state: &mut AppState,
    terminal_runtimes: &crate::terminal::TerminalRuntimeRegistry,
    text: &str,
) {
    if !state.navigator.search_focused {
        return;
    }
    state.navigator.state_filter = None;
    state.navigator.query.push_str(text);
    state.select_first_navigator_match_from(terminal_runtimes);
}

pub(crate) fn insert_keybind_help_query_text(state: &mut AppState, text: &str) {
    if !state.keybind_help.search_focused {
        return;
    }
    state
        .keybind_help
        .query
        .extend(text.chars().filter(|ch| !ch.is_control()));
    state.keybind_help.scroll = 0;
}

pub(super) fn keybind_help_back(state: &mut AppState) {
    if state.keybind_help.search_focused {
        state.keybind_help.query.clear();
        state.keybind_help.search_focused = false;
        state.keybind_help.scroll = 0;
    } else {
        leave_modal(state);
    }
}

pub(crate) fn handle_keybind_help_key(state: &mut AppState, key: TerminalKey) {
    if state.keybind_help.search_focused {
        let text_char = keybind_help_text_char(key);
        match key.code {
            KeyCode::Up => state.scroll_keybind_help(-1),
            KeyCode::Down => state.scroll_keybind_help(1),
            KeyCode::PageUp => state.scroll_keybind_help(-8),
            KeyCode::PageDown => state.scroll_keybind_help(8),
            KeyCode::Home => state.keybind_help.scroll = 0,
            KeyCode::End => state.keybind_help.scroll = state.keybind_help_max_scroll(),
            KeyCode::Backspace => {
                state.keybind_help.query.pop();
                state.keybind_help.scroll = 0;
            }
            KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => {
                state.keybind_help.query.clear();
                state.keybind_help.scroll = 0;
            }
            KeyCode::Esc => keybind_help_back(state),
            KeyCode::Enter => leave_modal(state),
            _ => {
                if let Some(character) = text_char {
                    insert_keybind_help_query_text(state, &character.to_string());
                }
            }
        }
        return;
    }

    match key.code {
        KeyCode::Up | KeyCode::Char('k') => state.scroll_keybind_help(-1),
        KeyCode::Down | KeyCode::Char('j') => state.scroll_keybind_help(1),
        KeyCode::PageUp => state.scroll_keybind_help(-8),
        KeyCode::PageDown => state.scroll_keybind_help(8),
        KeyCode::Home => state.keybind_help.scroll = 0,
        KeyCode::End => state.keybind_help.scroll = state.keybind_help_max_scroll(),
        _ if keybind_help_text_char(key) == Some('/') => {
            state.keybind_help.search_focused = true;
            state.keybind_help.scroll = 0;
        }
        KeyCode::Esc => keybind_help_back(state),
        KeyCode::Enter => leave_modal(state),
        _ if keybind_help_text_char(key) == Some('?') => leave_modal(state),
        _ => {}
    }
}

fn keybind_help_text_char(key: TerminalKey) -> Option<char> {
    if !key.modifiers.difference(KeyModifiers::SHIFT).is_empty() {
        return None;
    }
    if let Some(character) = key.shifted_codepoint.and_then(char::from_u32) {
        return Some(character);
    }
    let KeyCode::Char(character) = key.code else {
        return None;
    };
    Some(character)
}

pub(super) fn open_rename_workspace(
    state: &mut AppState,
    terminal_runtimes: &crate::terminal::TerminalRuntimeRegistry,
    ws_idx: usize,
) {
    state.pending_workspace_create_cwd = None;
    state.selected = ws_idx;
    state.rename_pane_target = None;
    state.name_input =
        state.workspaces[ws_idx].display_name_from(&state.terminals, terminal_runtimes);
    state.name_input_replace_on_type = false;
    state.enter_overlay_mode(Mode::RenameWorkspace);
}

/// Arm the rename input to collect a new group's name for a move
/// (TP-RANK-13). The pending workspace rides in its own slot so a plain
/// rename can never turn into a group creation.
fn open_move_new_group_input(state: &mut AppState, ws_idx: usize) {
    state.pending_move_new_group = Some(ws_idx);
    state.pending_new_module = None;
    state.pending_workspace_create_cwd = None;
    state.rename_pane_target = None;
    state.name_input = String::new();
    state.name_input_replace_on_type = false;
    state.context_menu = None;
    state.enter_overlay_mode(Mode::RenameWorkspace);
}

/// Arm the rename input to collect a new module's name (TP-DOTS-05). The
/// parent rides in its own slot so a plain rename can never write a node —
/// the same isolation the move-naming road relies on.
fn open_new_module_input(state: &mut AppState, parent: Option<String>) {
    state.pending_new_module = Some(crate::app::state::PendingNewModule { parent });
    state.pending_move_new_group = None;
    state.pending_workspace_create_cwd = None;
    state.rename_pane_target = None;
    state.name_input = String::new();
    state.name_input_replace_on_type = false;
    state.context_menu = None;
    state.enter_overlay_mode(Mode::RenameWorkspace);
}

pub(crate) fn open_new_workspace_dialog(state: &mut AppState, cwd: std::path::PathBuf) {
    let suggested_name = crate::workspace::derive_label_from_cwd(&cwd);
    state.creating_new_tab = false;
    state.requested_new_tab_name = None;
    state.pending_workspace_create_cwd = Some(cwd);
    state.rename_pane_target = None;
    state.name_input = suggested_name;
    state.name_input_replace_on_type = true;
    state.mode = Mode::RenameWorkspace;
}

pub(super) fn open_rename_active_tab(state: &mut AppState, replace_on_type: bool) {
    state.creating_new_tab = false;
    state.requested_new_tab_name = None;
    state.pending_workspace_create_cwd = None;
    state.rename_pane_target = None;
    if let Some(ws) = state.active.and_then(|i| state.workspaces.get(i)) {
        if let Some(name) = ws.active_tab_display_name() {
            state.name_input = name;
            state.name_input_replace_on_type = replace_on_type;
            state.enter_overlay_mode(Mode::RenameTab);
        }
    }
}

pub(super) fn open_rename_pane(state: &mut AppState, pane_id: crate::layout::PaneId) {
    let Some(ws) = state.active.and_then(|i| state.workspaces.get(i)) else {
        return;
    };
    let Some(pane) = ws.pane_state(pane_id) else {
        return;
    };
    let terminal = state.terminals.get(&pane.attached_terminal_id);
    state.creating_new_tab = false;
    state.requested_new_tab_name = None;
    state.pending_workspace_create_cwd = None;
    state.rename_pane_target = Some(pane_id);
    state.name_input = terminal
        .and_then(|t| t.manual_label.clone())
        .unwrap_or_default();
    state.name_input_replace_on_type = terminal.and_then(|t| t.manual_label.as_ref()).is_none();
    state.enter_overlay_mode(Mode::RenamePane);
}

fn workspace_create_label(input: &str, suggested_name: &str) -> Option<String> {
    let name = input.trim();
    (!name.is_empty() && name != suggested_name).then(|| name.to_string())
}

fn next_new_tab_default_name(state: &AppState) -> String {
    state
        .active
        .and_then(|i| state.workspaces.get(i))
        .map(|ws| (ws.tabs.len() + 1).to_string())
        .unwrap_or_else(|| "1".to_string())
}

pub(super) fn open_new_tab_dialog(state: &mut AppState) {
    state.creating_new_tab = true;
    state.requested_new_tab_name = None;
    state.pending_workspace_create_cwd = None;
    state.rename_pane_target = None;
    state.name_input = next_new_tab_default_name(state);
    state.name_input_replace_on_type = true;
    state.enter_overlay_mode(Mode::RenameTab);
}

pub(crate) fn leave_modal(state: &mut AppState) {
    // Restore the remembered pre-overlay focus owner while it is still
    // valid; otherwise fall back to the template default. The value is
    // consumed either way so it can never restore a long-dead owner.
    let restored = state
        .overlay_return_mode
        .take()
        .filter(|owner| match owner {
            Mode::Resize => state.active.is_some(),
            Mode::Copy => state.copy_mode.is_some(),
            _ => false,
        });
    state.mode = restored.unwrap_or(if state.active.is_some() {
        Mode::Terminal
    } else {
        Mode::Navigate
    });
}

pub(super) const ONBOARDING_WELCOME_ACTIONS: &[ModalActionSpec<ModalAction>] = &[ModalActionSpec {
    action: ModalAction::Continue,
    bindings: &[ModalKeyBinding::Enter],
}];

pub(super) const RELEASE_NOTES_ACTIONS: &[ModalActionSpec<ModalAction>] = &[ModalActionSpec {
    action: ModalAction::Close,
    bindings: &[ModalKeyBinding::Enter, ModalKeyBinding::Esc],
}];

pub(super) const RENAME_ACTIONS: &[ModalActionSpec<ModalAction>] = &[
    ModalActionSpec {
        action: ModalAction::Save,
        bindings: &[ModalKeyBinding::Enter],
    },
    ModalActionSpec {
        action: ModalAction::Clear,
        bindings: &[ModalKeyBinding::CtrlC],
    },
    ModalActionSpec {
        action: ModalAction::Cancel,
        bindings: &[ModalKeyBinding::Esc],
    },
];

pub(super) const CONFIRM_CLOSE_ACTIONS: &[ModalActionSpec<ModalAction>] = &[
    ModalActionSpec {
        action: ModalAction::Confirm,
        bindings: &[ModalKeyBinding::Enter],
    },
    ModalActionSpec {
        action: ModalAction::Cancel,
        bindings: &[ModalKeyBinding::Esc],
    },
];

pub(super) const SETTINGS_ACTIONS: &[ModalActionSpec<ModalAction>] = &[
    ModalActionSpec {
        action: ModalAction::Apply,
        bindings: &[ModalKeyBinding::Enter],
    },
    ModalActionSpec {
        action: ModalAction::Close,
        bindings: &[ModalKeyBinding::Esc],
    },
];

#[cfg(test)]
pub(super) fn apply_rename_action(state: &mut AppState, action: ModalAction) {
    match action {
        ModalAction::Save => {
            let new_name = if state.name_input.trim().is_empty() {
                state.name_input.clone()
            } else {
                state.name_input.trim().to_string()
            };
            match state.mode {
                // The move-naming road, mirrored from the live save path.
                Mode::RenameWorkspace if state.pending_move_new_group.is_some() => {
                    if let Some(ws_idx) = state.pending_move_new_group.take() {
                        state.submit_move_to_new_group(ws_idx, &new_name);
                    }
                }
                // The module-naming road (TP-DOTS-05), mirrored the same way.
                Mode::RenameWorkspace if state.pending_new_module.is_some() => {
                    if let Some(pending) = state.pending_new_module.take() {
                        state.submit_new_module(pending.parent, &new_name);
                    }
                }
                Mode::RenameWorkspace
                    if state.pending_workspace_create_cwd.is_none()
                        && !state.workspaces.is_empty()
                        && !new_name.is_empty() =>
                {
                    let workspace_id = state.workspaces[state.selected].id.clone();
                    state.workspaces[state.selected].set_custom_name(new_name);
                    crate::logging::workspace_renamed(&workspace_id);
                    state.mark_session_dirty();
                }
                Mode::RenameTab if state.creating_new_tab => {
                    state.request_new_tab = true;
                    let default_name = next_new_tab_default_name(state);
                    state.requested_new_tab_name =
                        if new_name.is_empty() || new_name == default_name {
                            None
                        } else {
                            Some(new_name)
                        };
                }
                Mode::RenameTab => {
                    if let Some(ws_idx) = state.active {
                        if let Some(ws) = state.workspaces.get_mut(ws_idx) {
                            let workspace_id = ws.id.clone();
                            let active_tab = ws.active_tab_index();
                            let keep_auto_name = ws
                                .tabs
                                .get(active_tab)
                                .is_some_and(|tab| tab.is_auto_named())
                                && ws
                                    .tab_display_name(active_tab)
                                    .is_some_and(|name| new_name == name);
                            if let Some(tab) = ws.active_tab_mut() {
                                if !new_name.is_empty() && !keep_auto_name {
                                    tab.set_custom_name(new_name);
                                    let tab_id = ws
                                        .public_tab_number(active_tab)
                                        .map(|number| {
                                            crate::workspace::public_tab_id_for_number(
                                                &workspace_id,
                                                number,
                                            )
                                        })
                                        .unwrap_or_else(|| workspace_id.clone());
                                    crate::logging::tab_renamed(&workspace_id, &tab_id);
                                    state.mark_session_dirty();
                                }
                            }
                        }
                    }
                }
                Mode::RenamePane => {
                    if let (Some(ws_idx), Some(pane_id)) = (state.active, state.rename_pane_target)
                    {
                        if let Some(ws) = state.workspaces.get(ws_idx) {
                            if let Some(pane) = ws.pane_state(pane_id) {
                                let terminal_id = pane.attached_terminal_id.clone();
                                if let Some(terminal) = state.terminals.get_mut(&terminal_id) {
                                    terminal.set_manual_label(new_name);
                                    state.mark_session_dirty();
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
            state.creating_new_tab = false;
            state.pending_workspace_create_cwd = None;
            state.rename_pane_target = None;
            state.file_manager_rename = None;
            state.name_input.clear();
            state.name_input_replace_on_type = false;
            leave_modal(state);
        }
        ModalAction::Clear => {
            state.name_input.clear();
            state.name_input_replace_on_type = false;
        }
        ModalAction::Cancel => {
            state.creating_new_tab = false;
            state.requested_new_tab_name = None;
            state.pending_workspace_create_cwd = None;
            state.rename_pane_target = None;
            state.file_manager_rename = None;
            state.name_input.clear();
            state.name_input_replace_on_type = false;
            leave_modal(state);
        }
        _ => {}
    }
}

fn clear_rename_input(state: &mut AppState) {
    state.name_input.clear();
    state.name_input_replace_on_type = false;
    if let Some(rename) = state.file_manager_rename.as_mut() {
        rename.validation_error = None;
    }
}

pub(crate) fn insert_rename_input_text(state: &mut AppState, text: &str) {
    if state.name_input_replace_on_type {
        clear_rename_input(state);
    }
    state.name_input.push_str(text);
    if let Some(rename) = state.file_manager_rename.as_mut() {
        rename.validation_error = None;
    }
}

fn delete_rename_input_char(state: &mut AppState) {
    if state.name_input_replace_on_type {
        clear_rename_input(state);
    } else {
        state.name_input.pop();
    }
    if let Some(rename) = state.file_manager_rename.as_mut() {
        rename.validation_error = None;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RenameWordDeleteClass {
    Word,
    Separator,
}

fn rename_word_delete_class(ch: char) -> RenameWordDeleteClass {
    if ch.is_alphanumeric() || ch == '_' {
        RenameWordDeleteClass::Word
    } else {
        RenameWordDeleteClass::Separator
    }
}

fn delete_rename_input_word(state: &mut AppState) {
    if state.name_input_replace_on_type {
        clear_rename_input(state);
        return;
    }

    while state
        .name_input
        .chars()
        .last()
        .is_some_and(char::is_whitespace)
    {
        state.name_input.pop();
    }
    if let Some(rename) = state.file_manager_rename.as_mut() {
        rename.validation_error = None;
    }

    let Some(class) = state
        .name_input
        .chars()
        .last()
        .map(rename_word_delete_class)
    else {
        return;
    };

    while state
        .name_input
        .chars()
        .last()
        .is_some_and(|ch| !ch.is_whitespace() && rename_word_delete_class(ch) == class)
    {
        state.name_input.pop();
    }
}

fn handle_rename_edit_key(state: &mut AppState, key: KeyEvent) {
    match key.code {
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            clear_rename_input(state);
        }
        KeyCode::Backspace if key.modifiers.contains(KeyModifiers::SUPER) => {
            clear_rename_input(state);
        }
        KeyCode::Backspace
            if key.modifiers.contains(KeyModifiers::CONTROL)
                || key.modifiers.contains(KeyModifiers::ALT) =>
        {
            delete_rename_input_word(state);
        }
        KeyCode::Char('h' | 'w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            delete_rename_input_word(state);
        }
        KeyCode::Backspace => delete_rename_input_char(state),
        KeyCode::Char(c) if key.modifiers.difference(KeyModifiers::SHIFT).is_empty() => {
            insert_rename_input_text(state, &c.to_string());
        }
        _ => {}
    }
}

#[cfg(test)]
pub(crate) fn handle_rename_key(state: &mut AppState, key: KeyEvent) {
    if let Some(action) = modal_action_from_key(&key, RENAME_ACTIONS) {
        apply_rename_action(state, action);
        return;
    }

    handle_rename_edit_key(state, key);
}

#[cfg(test)]
pub(crate) fn handle_resize_key(state: &mut AppState, raw_key: TerminalKey) {
    let key = raw_key.as_key_event();
    if key.code == KeyCode::Esc
        || key.code == KeyCode::Enter
        || state.keybinds.resize_mode.matches_prefix_key(raw_key)
        || state.keybinds.resize_mode.matches_direct_key(raw_key)
    {
        if state.active.is_some() {
            state.mode = Mode::Terminal;
        } else {
            state.mode = Mode::Navigate;
        }
        return;
    }

    match key.code {
        KeyCode::Char('h') | KeyCode::Left => state.resize_pane(NavDirection::Left),
        KeyCode::Char('l') | KeyCode::Right => state.resize_pane(NavDirection::Right),
        KeyCode::Char('j') | KeyCode::Down => state.resize_pane(NavDirection::Down),
        KeyCode::Char('k') | KeyCode::Up => state.resize_pane(NavDirection::Up),
        _ => {}
    }
}

pub(super) fn open_confirm_close(state: &mut AppState) {
    state.enter_overlay_mode(Mode::ConfirmClose);
}

#[cfg(test)]
pub(super) fn confirm_close_accept(state: &mut AppState) {
    state.close_selected_workspace();
    if state.workspaces.is_empty() {
        state.mode = Mode::Navigate;
    } else {
        state.mode = Mode::Terminal;
    }
}

pub(super) fn confirm_close_cancel(state: &mut AppState) {
    state.mode = Mode::Navigate;
}

fn validated_file_context_action(
    state: &AppState,
    snapshot: &FileManagerContextMenuModel,
    idx: usize,
) -> Option<FileManagerContextActionIntent> {
    let snapshot_item = snapshot.items.get(idx)?;
    if !snapshot_item.enabled {
        return None;
    }
    let file_manager = state.file_manager.as_ref()?;
    let action_bar = crate::ui::compute_file_manager_action_bar_model(
        file_manager,
        &state.file_manager_clipboard,
        state
            .file_manager_operation
            .as_ref()
            .is_some_and(|operation| operation.is_running()),
        state.file_manager_locations.focus,
    );
    let plugin_actions = crate::app::api::plugins::file_manifest_actions(&state.installed_plugins);
    let current =
        FileManagerContextMenuModel::from_action_bar_with_plugins(&action_bar, &plugin_actions)?;
    let current_item = current.items.get(idx)?;
    if current.paths != snapshot.paths
        || current.target_kind != snapshot.target_kind
        || current_item.action != snapshot_item.action
        || !current_item.enabled
    {
        return None;
    }
    let intent = FileManagerContextActionIntent {
        action: snapshot_item.action.clone(),
        paths: snapshot.paths.clone(),
    };
    if matches!(
        &intent.action,
        crate::app::state::FileManagerContextMenuAction::Plugin { .. }
    ) && intent.plugin_invocation_params().is_none()
    {
        return None;
    }
    Some(intent)
}

#[cfg(test)]
pub(crate) fn handle_confirm_close_key(state: &mut AppState, key: KeyEvent) {
    match modal_action_from_key(&key, CONFIRM_CLOSE_ACTIONS) {
        Some(ModalAction::Confirm) => confirm_close_accept(state),
        Some(ModalAction::Cancel) => confirm_close_cancel(state),
        _ => {}
    }
}

#[cfg(test)]
pub(super) fn apply_context_menu_action(
    state: &mut AppState,
    terminal_runtimes: &mut crate::terminal::TerminalRuntimeRegistry,
    menu: ContextMenuState,
    idx: usize,
) {
    let item = menu.items().get(idx).map(|item| (*item).to_string());
    let (menu_x, menu_y) = (menu.x, menu.y);
    let file_intent = match &menu.kind {
        ContextMenuKind::File { model } => validated_file_context_action(state, model, idx),
        _ => None,
    };
    match (menu.kind, item.as_deref()) {
        (ContextMenuKind::File { .. }, _) => {
            state.request_file_manager_context_action = file_intent;
            leave_modal(state);
        }
        (ContextMenuKind::AppDock { app }, _) => {
            state.activate_dock_app(app);
            leave_modal(state);
        }
        // Worktree rows must match BEFORE the agent catch-all below, which
        // would otherwise persist "New worktree" as the default chat agent.
        (ContextMenuKind::ProjectNewChat { proj_idx, .. }, Some("New worktree")) => {
            if let Some(ws_idx) = state.project_workspace_index(proj_idx) {
                state.request_new_linked_worktree = Some(ws_idx);
            }
            leave_modal(state);
        }
        (ContextMenuKind::ProjectNewChat { proj_idx, .. }, Some("Open worktree...")) => {
            if let Some(ws_idx) = state.project_workspace_index(proj_idx) {
                state.request_open_existing_worktree = Some(ws_idx);
            }
            leave_modal(state);
        }
        (ContextMenuKind::ProjectNewChat { proj_idx, .. }, Some(agent)) => {
            state.default_chat_agent = agent.to_string();
            if let Some(project) = state.projects_sessions.get(proj_idx) {
                state.request_project_chat_tab = Some(crate::app::state::ProjectChatTabRequest {
                    project_path: project.path.clone(),
                    session_id: None,
                });
            }
            leave_modal(state);
        }
        // Worktree rows before the agent catch-all, same ordering reason as
        // the Projects menu above.
        (ContextMenuKind::WorkspaceNewChat { ws_idx, .. }, Some("New worktree")) => {
            state.request_new_linked_worktree = Some(ws_idx);
            leave_modal(state);
        }
        (ContextMenuKind::WorkspaceNewChat { ws_idx, .. }, Some("Open worktree...")) => {
            state.request_open_existing_worktree = Some(ws_idx);
            leave_modal(state);
        }
        (ContextMenuKind::WorkspaceNewChat { ws_idx, .. }, Some(agent)) => {
            state.default_chat_agent = agent.to_string();
            state.request_workspace_chat(ws_idx);
            leave_modal(state);
        }
        (ContextMenuKind::GitWorkspace { ws_idx, .. }, Some("New worktree")) => {
            state.request_new_linked_worktree = Some(ws_idx);
            leave_modal(state);
        }
        (ContextMenuKind::GitWorkspace { ws_idx, .. }, Some("Delete worktree checkout...")) => {
            state.request_remove_linked_worktree = Some(ws_idx);
            leave_modal(state);
        }
        // TP-RANK-07: the mouse road to `herdr space promote` — write the
        // managed rule and regroup in place, exactly like the CLI.
        (ContextMenuKind::GitWorkspace { ws_idx, .. }, Some("Promote to module")) => {
            state.promote_workspace_space(ws_idx, false);
            leave_modal(state);
        }
        (ContextMenuKind::GitWorkspace { ws_idx, .. }, Some("Promote to project")) => {
            state.promote_workspace_space(ws_idx, true);
            leave_modal(state);
        }
        (ContextMenuKind::GitWorkspace { ws_idx, .. }, Some("Demote from module")) => {
            state.demote_workspace_space(ws_idx);
            leave_modal(state);
        }
        // TP-RANK-13: the move chain — "Move..." opens the verb submenu in
        // place, a verb opens the target picker, and the picker resolves by
        // index so display names never have to be unique.
        (ContextMenuKind::GitWorkspace { ws_idx, .. }, Some("Move...")) => {
            let targets = state.move_target_entries();
            state.context_menu = Some(crate::app::state::ContextMenuState {
                kind: ContextMenuKind::MoveWorkspace {
                    ws_idx,
                    has_targets: !targets.is_empty(),
                },
                x: menu_x,
                y: menu_y,
                list: crate::app::state::MenuListState::new(0),
            });
        }
        (
            ContextMenuKind::MoveWorkspace { ws_idx, .. },
            Some(verb @ ("Under a group..." | "Beside a group..." | "Above a group...")),
        ) => {
            let op = match verb {
                "Under a group..." => crate::spaces::MoveOp::Under,
                "Beside a group..." => crate::spaces::MoveOp::Beside,
                _ => crate::spaces::MoveOp::Above,
            };
            state.context_menu = Some(crate::app::state::ContextMenuState {
                kind: ContextMenuKind::MoveTarget {
                    ws_idx,
                    op,
                    targets: state.move_target_entries(),
                },
                x: menu_x,
                y: menu_y,
                list: crate::app::state::MenuListState::new(0),
            });
        }
        (ContextMenuKind::MoveWorkspace { ws_idx, .. }, Some("Under a new group...")) => {
            open_move_new_group_input(state, ws_idx);
        }
        (ContextMenuKind::MoveWorkspace { ws_idx, .. }, Some("To top level")) => {
            state.move_workspace_space(ws_idx, None, None);
            leave_modal(state);
        }
        // TP-DOTS-05/07: the header's creation road — sub hangs under the
        // header itself, parallel beside it (the header's own parent) — and
        // the fold verbs mirror what a left press does on the row.
        (ContextMenuKind::NodeHeader { node_key, .. }, Some("New sub-module...")) => {
            open_new_module_input(state, Some(node_key));
        }
        (ContextMenuKind::NodeHeader { node_key, .. }, Some("New parallel module...")) => {
            let parent = state
                .space_nodes
                .iter()
                .find(|node| node.key == node_key)
                .and_then(|node| node.parent.clone());
            open_new_module_input(state, parent);
        }
        (ContextMenuKind::NodeHeader { node_key, .. }, Some("Collapse")) => {
            state.fold_node(node_key);
            leave_modal(state);
        }
        (ContextMenuKind::NodeHeader { node_key, .. }, Some("Expand")) => {
            state.unfold_node(&node_key);
            leave_modal(state);
        }
        (ContextMenuKind::SpaceHeader { space_key, .. }, Some("Collapse")) => {
            state.collapsed_space_keys.insert(space_key);
            state.mark_session_dirty();
            leave_modal(state);
        }
        (ContextMenuKind::SpaceHeader { space_key, .. }, Some("Expand")) => {
            state.collapsed_space_keys.remove(&space_key);
            state.mark_session_dirty();
            leave_modal(state);
        }
        (
            ContextMenuKind::MoveTarget {
                ws_idx,
                op,
                targets,
            },
            Some(_),
        ) => {
            if let Some((key, _)) = targets.get(idx) {
                match crate::spaces::move_parent_for(&state.space_nodes, key, op) {
                    Ok(parent) => state.move_workspace_space(ws_idx, parent, None),
                    Err(err) => tracing::warn!(error = %err, "move target vanished"),
                }
            }
            leave_modal(state);
        }
        // TP-CHAT-MOVE-04: the chat menu parks its decision for the App
        // loop, which owns the ledger the decision is written into.
        (
            ContextMenuKind::WorkspaceChat {
                ws_idx, session_id, ..
            },
            Some("Move to branch..."),
        ) => {
            let targets = state.chat_move_target_entries(ws_idx);
            state.context_menu = Some(crate::app::state::ContextMenuState {
                kind: ContextMenuKind::ChatMoveTarget {
                    session_id,
                    targets,
                },
                x: menu_x,
                y: menu_y,
                list: crate::app::state::MenuListState::new(0),
            });
        }
        (ContextMenuKind::WorkspaceChat { session_id, .. }, Some("Move back")) => {
            state.request_chat_move = Some((session_id, None));
            leave_modal(state);
        }
        (
            ContextMenuKind::ChatMoveTarget {
                session_id,
                targets,
            },
            Some(_),
        ) => {
            if let Some((key, _)) = targets.get(idx) {
                state.request_chat_move = Some((session_id, Some(key.clone())));
            }
            leave_modal(state);
        }
        (ContextMenuKind::GitWorkspace { ws_idx, .. }, Some("Open worktree...")) => {
            state.request_open_existing_worktree = Some(ws_idx);
            leave_modal(state);
        }
        (
            ContextMenuKind::GitWorkspace {
                ws_idx, collapsed, ..
            },
            Some("Collapse" | "Expand"),
        ) => {
            if let Some(key) = state
                .workspaces
                .get(ws_idx)
                .and_then(|ws| ws.worktree_space())
                .map(|space| space.key.clone())
            {
                if collapsed {
                    state.collapsed_space_keys.remove(&key);
                } else {
                    state.collapsed_space_keys.insert(key);
                }
                state.mark_session_dirty();
            }
            leave_modal(state);
        }
        (
            ContextMenuKind::Workspace { ws_idx } | ContextMenuKind::GitWorkspace { ws_idx, .. },
            Some("Rename"),
        ) => {
            open_rename_workspace(state, terminal_runtimes, ws_idx);
        }
        (
            ContextMenuKind::Workspace { ws_idx } | ContextMenuKind::GitWorkspace { ws_idx, .. },
            Some("Close" | "Close group"),
        ) => {
            state.selected = ws_idx;
            if state.confirm_close {
                open_confirm_close(state);
            } else {
                state.close_selected_workspace();
                state.mode = Mode::Navigate;
            }
        }
        (ContextMenuKind::Tab { ws_idx, tab_idx }, Some("New tab")) => {
            state.selected = ws_idx;
            state.active = Some(ws_idx);
            state.switch_tab(tab_idx);
            open_new_tab_dialog(state);
        }
        (ContextMenuKind::Tab { ws_idx, tab_idx }, Some("Rename")) => {
            state.selected = ws_idx;
            state.active = Some(ws_idx);
            state.switch_tab(tab_idx);
            open_rename_active_tab(state, false);
        }
        (ContextMenuKind::Tab { ws_idx, tab_idx }, Some("Close")) => {
            state.selected = ws_idx;
            state.active = Some(ws_idx);
            state.switch_tab(tab_idx);
            if !state.close_tab() {
                state.mode = if state.active.is_some() {
                    Mode::Terminal
                } else {
                    Mode::Navigate
                };
            }
        }
        (ContextMenuKind::Pane { pane_id, .. }, Some("Rename pane")) => {
            open_rename_pane(state, pane_id);
        }
        (
            ContextMenuKind::Pane {
                ws_idx, pane_id, ..
            },
            Some("Clear pane name"),
        ) => {
            if let Some(ws) = state.workspaces.get(ws_idx) {
                if let Some(pane) = ws.pane_state(pane_id) {
                    let terminal_id = pane.attached_terminal_id.clone();
                    if let Some(terminal) = state.terminals.get_mut(&terminal_id) {
                        terminal.clear_manual_label();
                        state.mark_session_dirty();
                    }
                }
            }
            state.mode = Mode::Terminal;
        }
        (
            ContextMenuKind::Pane {
                ws_idx,
                tab_idx,
                pane_id,
                source_pane_id,
                ..
            },
            Some("Swap with focused pane"),
        ) => {
            if let Some(source_pane_id) = source_pane_id {
                state.selected = ws_idx;
                state.active = Some(ws_idx);
                state.switch_tab(tab_idx);
                if let Some(tab) = state
                    .workspaces
                    .get_mut(ws_idx)
                    .and_then(|ws| ws.tabs.get_mut(tab_idx))
                {
                    if tab.layout.swap_panes(source_pane_id, pane_id) {
                        tab.layout.focus_pane(source_pane_id);
                        state.mark_session_dirty();
                    }
                }
            }
            state.mode = Mode::Terminal;
        }
        (
            ContextMenuKind::Pane {
                ws_idx,
                tab_idx,
                pane_id,
                ..
            },
            Some("Split right"),
        ) => {
            state.selected = ws_idx;
            state.active = Some(ws_idx);
            state.switch_tab(tab_idx);
            state.focus_pane_in_workspace(ws_idx, pane_id);
            state.split_pane(terminal_runtimes, Direction::Horizontal);
            state.mode = Mode::Terminal;
        }
        (
            ContextMenuKind::Pane {
                ws_idx,
                tab_idx,
                pane_id,
                ..
            },
            Some("Split down"),
        ) => {
            state.selected = ws_idx;
            state.active = Some(ws_idx);
            state.switch_tab(tab_idx);
            state.focus_pane_in_workspace(ws_idx, pane_id);
            state.split_pane(terminal_runtimes, Direction::Vertical);
            state.mode = Mode::Terminal;
        }
        (
            ContextMenuKind::Pane {
                ws_idx,
                tab_idx,
                pane_id,
                ..
            },
            Some("Zoom"),
        ) => {
            state.selected = ws_idx;
            state.active = Some(ws_idx);
            state.switch_tab(tab_idx);
            state.focus_pane_in_workspace(ws_idx, pane_id);
            state.toggle_zoom();
            state.mode = Mode::Terminal;
        }
        (
            ContextMenuKind::Pane {
                ws_idx,
                tab_idx,
                pane_id,
                ..
            },
            Some("Close pane"),
        ) => {
            state.selected = ws_idx;
            state.active = Some(ws_idx);
            state.switch_tab(tab_idx);
            state.focus_pane_in_workspace(ws_idx, pane_id);
            if !state.close_pane() {
                state.mode = if state.active.is_some() {
                    Mode::Terminal
                } else {
                    Mode::Navigate
                };
            }
        }
        _ => leave_modal(state),
    }
}

#[cfg(test)]
pub(crate) fn handle_context_menu_key(
    state: &mut AppState,
    terminal_runtimes: &mut crate::terminal::TerminalRuntimeRegistry,
    key: KeyEvent,
) {
    match key.code {
        KeyCode::Esc => {
            state.context_menu = None;
            leave_modal(state);
        }
        KeyCode::Up => {
            if let Some(menu) = &mut state.context_menu {
                menu.list.move_prev();
            }
        }
        KeyCode::Down => {
            if let Some(menu) = &mut state.context_menu {
                menu.list.move_next(menu.items().len());
            }
        }
        KeyCode::Enter => {
            let idx = state
                .context_menu
                .as_ref()
                .map(|menu| menu.list.highlighted);
            let enabled_idx = idx.filter(|idx| {
                state
                    .context_menu
                    .as_ref()
                    .is_some_and(|menu| menu.item_enabled(*idx))
            });
            if let Some(idx) = enabled_idx {
                if let Some(menu) = state.context_menu.take() {
                    apply_context_menu_action(state, terminal_runtimes, menu, idx);
                }
            }
        }
        _ => {}
    }
}

impl App {
    pub(crate) fn handle_rename_key_via_api(&mut self, key: KeyEvent) {
        if let Some(action) = modal_action_from_key(&key, RENAME_ACTIONS) {
            self.apply_rename_mouse_action_via_api(action);
            return;
        }

        handle_rename_edit_key(&mut self.state, key);
    }

    fn save_rename_modal_via_api(&mut self) {
        let new_name = if self.state.name_input.trim().is_empty() {
            self.state.name_input.clone()
        } else {
            self.state.name_input.trim().to_string()
        };

        match self.state.mode {
            // TP-RANK-13: a name collected for a move becomes the group and
            // the re-hang in one write; the pending slot is consumed here so
            // the shared cancel below stays a no-op for it.
            Mode::RenameWorkspace if self.state.pending_move_new_group.is_some() => {
                if let Some(ws_idx) = self.state.pending_move_new_group.take() {
                    self.state.submit_move_to_new_group(ws_idx, &new_name);
                }
            }
            // TP-DOTS-05: a name collected for a new module becomes one
            // managed node entry; consumed here for the same reason.
            Mode::RenameWorkspace if self.state.pending_new_module.is_some() => {
                if let Some(pending) = self.state.pending_new_module.take() {
                    self.state.submit_new_module(pending.parent, &new_name);
                }
            }
            Mode::RenameWorkspace => {
                if let Some(cwd) = self.state.pending_workspace_create_cwd.take() {
                    let suggested_name = crate::workspace::derive_label_from_cwd(&cwd);
                    let label = workspace_create_label(&new_name, &suggested_name);
                    self.runtime_workspace_create(
                        "tui.workspace.create_named",
                        crate::api::schema::WorkspaceCreateParams {
                            cwd: Some(cwd.display().to_string()),
                            focus: true,
                            label,
                            env: Default::default(),
                        },
                    );
                } else if !self.state.workspaces.is_empty() && !new_name.is_empty() {
                    let workspace_id = self.public_workspace_id(self.state.selected);
                    self.runtime_workspace_rename(
                        "tui.workspace.rename",
                        crate::api::schema::WorkspaceRenameParams {
                            workspace_id,
                            label: new_name,
                        },
                    );
                }
            }
            Mode::RenameTab if self.state.creating_new_tab => {
                let default_name = next_new_tab_default_name(&self.state);
                let label = if new_name.is_empty() || new_name == default_name {
                    None
                } else {
                    Some(new_name)
                };
                self.runtime_tab_create(
                    "tui.tab.create_named",
                    crate::api::schema::TabCreateParams {
                        workspace_id: None,
                        cwd: None,
                        focus: true,
                        label,
                        env: Default::default(),
                    },
                );
            }
            Mode::RenameTab if !new_name.is_empty() => {
                let Some(ws_idx) = self.state.active else {
                    cancel_rename_modal(&mut self.state);
                    return;
                };
                let tab_idx = self.state.workspaces[ws_idx].active_tab_index();
                let keep_auto_name = self.state.workspaces[ws_idx]
                    .tabs
                    .get(tab_idx)
                    .is_some_and(|tab| tab.is_auto_named())
                    && self.state.workspaces[ws_idx]
                        .tab_display_name(tab_idx)
                        .is_some_and(|name| new_name == name);
                if !keep_auto_name {
                    if let Some(tab_id) = self.public_tab_id(ws_idx, tab_idx) {
                        self.runtime_tab_rename(
                            "tui.tab.rename",
                            crate::api::schema::TabRenameParams {
                                tab_id,
                                label: new_name,
                            },
                        );
                    }
                }
            }
            Mode::RenamePane => {
                if let (Some(ws_idx), Some(pane_id)) =
                    (self.state.active, self.state.rename_pane_target)
                {
                    if let Some(pane_id) = self.public_pane_id(ws_idx, pane_id) {
                        self.runtime_pane_rename(
                            "tui.pane.rename",
                            crate::api::schema::PaneRenameParams {
                                pane_id,
                                label: Some(new_name),
                            },
                        );
                    }
                }
            }
            _ => {}
        }

        cancel_rename_modal(&mut self.state);
    }

    pub(super) fn apply_rename_mouse_action_via_api(&mut self, action: ModalAction) {
        match action {
            ModalAction::Save if self.state.mode == Mode::RenameFile => {
                if self.submit_file_manager_rename() {
                    cancel_rename_modal(&mut self.state);
                }
            }
            ModalAction::Save => self.save_rename_modal_via_api(),
            ModalAction::Clear => {
                clear_rename_input(&mut self.state);
            }
            ModalAction::Cancel => cancel_rename_modal(&mut self.state),
            _ => {}
        }
    }

    pub(super) fn confirm_close_accept_via_api(&mut self) {
        let ws_idx = self.state.selected;
        if ws_idx < self.state.workspaces.len() {
            self.close_workspace_idx_via_api(ws_idx);
        }
        self.state.mode = if self.state.active.is_some() {
            Mode::Terminal
        } else {
            Mode::Navigate
        };
    }

    pub(crate) fn handle_resize_key_via_api(&mut self, raw_key: TerminalKey) {
        let key = raw_key.as_key_event();
        if key.code == KeyCode::Esc
            || key.code == KeyCode::Enter
            || self.state.keybinds.resize_mode.matches_prefix_key(raw_key)
            || self.state.keybinds.resize_mode.matches_direct_key(raw_key)
        {
            self.state.mode = if self.state.active.is_some() {
                Mode::Terminal
            } else {
                Mode::Navigate
            };
            return;
        }

        let direction = match key.code {
            KeyCode::Char('h') | KeyCode::Left => Some(NavDirection::Left),
            KeyCode::Char('l') | KeyCode::Right => Some(NavDirection::Right),
            KeyCode::Char('j') | KeyCode::Down => Some(NavDirection::Down),
            KeyCode::Char('k') | KeyCode::Up => Some(NavDirection::Up),
            _ => None,
        };
        if let Some(direction) = direction {
            self.runtime_pane_resize(
                "tui.pane.resize",
                crate::api::schema::PaneResizeParams {
                    pane_id: None,
                    direction: super::navigate::api_pane_direction(direction),
                    amount: None,
                },
            );
        }
    }

    pub(crate) fn handle_confirm_close_key_via_api(&mut self, key: KeyEvent) {
        match modal_action_from_key(&key, CONFIRM_CLOSE_ACTIONS) {
            Some(ModalAction::Confirm) => {
                self.confirm_close_accept_via_api();
            }
            Some(ModalAction::Cancel) => confirm_close_cancel(&mut self.state),
            _ => {}
        }
    }

    pub(crate) fn handle_context_menu_key_via_api(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.state.context_menu = None;
                leave_modal(&mut self.state);
            }
            KeyCode::Up => {
                if let Some(menu) = &mut self.state.context_menu {
                    menu.list.move_prev();
                }
            }
            KeyCode::Down => {
                if let Some(menu) = &mut self.state.context_menu {
                    menu.list.move_next(menu.items().len());
                }
            }
            KeyCode::Enter => {
                let idx = self
                    .state
                    .context_menu
                    .as_ref()
                    .map(|menu| menu.list.highlighted);
                let enabled_idx = idx.filter(|idx| {
                    self.state
                        .context_menu
                        .as_ref()
                        .is_some_and(|menu| menu.item_enabled(*idx))
                });
                if let Some(idx) = enabled_idx {
                    if let Some(menu) = self.state.context_menu.take() {
                        self.apply_context_menu_action_via_api(menu, idx);
                    }
                }
            }
            _ => {}
        }
    }

    pub(crate) fn apply_context_menu_action_via_api(&mut self, menu: ContextMenuState, idx: usize) {
        let item = menu.items().get(idx).map(|item| (*item).to_string());
        let (menu_x, menu_y) = (menu.x, menu.y);
        let file_intent = match &menu.kind {
            ContextMenuKind::File { model } => {
                validated_file_context_action(&self.state, model, idx)
            }
            _ => None,
        };
        match (menu.kind, item.as_deref()) {
            (ContextMenuKind::File { .. }, _) => {
                self.state.request_file_manager_context_action = file_intent;
                leave_modal(&mut self.state);
            }
            // Worktree rows must match BEFORE the agent catch-all below, which
            // would otherwise persist "New worktree" as the default chat agent.
            (ContextMenuKind::ProjectNewChat { proj_idx, .. }, Some("New worktree")) => {
                if let Some(ws_idx) = self.state.project_workspace_index(proj_idx) {
                    self.state.request_new_linked_worktree = Some(ws_idx);
                }
                leave_modal(&mut self.state);
            }
            (ContextMenuKind::ProjectNewChat { proj_idx, .. }, Some("Open worktree...")) => {
                if let Some(ws_idx) = self.state.project_workspace_index(proj_idx) {
                    self.state.request_open_existing_worktree = Some(ws_idx);
                }
                leave_modal(&mut self.state);
            }
            // Picking an agent makes it the persisted default AND opens the
            // new chat in that project with it.
            (ContextMenuKind::ProjectNewChat { proj_idx, .. }, Some(agent)) => {
                self.state.default_chat_agent = agent.to_string();
                if let Some(project) = self.state.projects_sessions.get(proj_idx) {
                    self.state.request_project_chat_tab =
                        Some(crate::app::state::ProjectChatTabRequest {
                            project_path: project.path.clone(),
                            session_id: None,
                        });
                }
                self.save_default_chat_agent(agent);
                leave_modal(&mut self.state);
            }
            (ContextMenuKind::GitWorkspace { ws_idx, .. }, Some("New worktree")) => {
                self.state.request_new_linked_worktree = Some(ws_idx);
                leave_modal(&mut self.state);
            }
            (ContextMenuKind::GitWorkspace { ws_idx, .. }, Some("Delete worktree checkout...")) => {
                self.state.request_remove_linked_worktree = Some(ws_idx);
                leave_modal(&mut self.state);
            }
            (ContextMenuKind::GitWorkspace { ws_idx, .. }, Some("Promote to module")) => {
                self.state.promote_workspace_space(ws_idx, false);
                leave_modal(&mut self.state);
            }
            (ContextMenuKind::GitWorkspace { ws_idx, .. }, Some("Promote to project")) => {
                self.state.promote_workspace_space(ws_idx, true);
                leave_modal(&mut self.state);
            }
            (ContextMenuKind::GitWorkspace { ws_idx, .. }, Some("Demote from module")) => {
                self.state.demote_workspace_space(ws_idx);
                leave_modal(&mut self.state);
            }
            // TP-RANK-13: the move chain, on the mouse road — the same three
            // steps the keyboard dispatch walks.
            (ContextMenuKind::GitWorkspace { ws_idx, .. }, Some("Move...")) => {
                let targets = self.state.move_target_entries();
                self.state.context_menu = Some(crate::app::state::ContextMenuState {
                    kind: ContextMenuKind::MoveWorkspace {
                        ws_idx,
                        has_targets: !targets.is_empty(),
                    },
                    x: menu_x,
                    y: menu_y,
                    list: crate::app::state::MenuListState::new(0),
                });
            }
            (
                ContextMenuKind::MoveWorkspace { ws_idx, .. },
                Some(verb @ ("Under a group..." | "Beside a group..." | "Above a group...")),
            ) => {
                let op = match verb {
                    "Under a group..." => crate::spaces::MoveOp::Under,
                    "Beside a group..." => crate::spaces::MoveOp::Beside,
                    _ => crate::spaces::MoveOp::Above,
                };
                self.state.context_menu = Some(crate::app::state::ContextMenuState {
                    kind: ContextMenuKind::MoveTarget {
                        ws_idx,
                        op,
                        targets: self.state.move_target_entries(),
                    },
                    x: menu_x,
                    y: menu_y,
                    list: crate::app::state::MenuListState::new(0),
                });
            }
            (ContextMenuKind::MoveWorkspace { ws_idx, .. }, Some("Under a new group...")) => {
                open_move_new_group_input(&mut self.state, ws_idx);
            }
            (ContextMenuKind::MoveWorkspace { ws_idx, .. }, Some("To top level")) => {
                self.state.move_workspace_space(ws_idx, None, None);
                leave_modal(&mut self.state);
            }
            // TP-DOTS-05/07: the header's creation road on the mouse dispatch
            // — the same arms the keyboard road walks.
            (ContextMenuKind::NodeHeader { node_key, .. }, Some("New sub-module...")) => {
                open_new_module_input(&mut self.state, Some(node_key));
            }
            (ContextMenuKind::NodeHeader { node_key, .. }, Some("New parallel module...")) => {
                let parent = self
                    .state
                    .space_nodes
                    .iter()
                    .find(|node| node.key == node_key)
                    .and_then(|node| node.parent.clone());
                open_new_module_input(&mut self.state, parent);
            }
            (ContextMenuKind::NodeHeader { node_key, .. }, Some("Collapse")) => {
                self.state.fold_node(node_key);
                leave_modal(&mut self.state);
            }
            (ContextMenuKind::NodeHeader { node_key, .. }, Some("Expand")) => {
                self.state.unfold_node(&node_key);
                leave_modal(&mut self.state);
            }
            (ContextMenuKind::SpaceHeader { space_key, .. }, Some("Collapse")) => {
                self.state.collapsed_space_keys.insert(space_key);
                self.state.mark_session_dirty();
                leave_modal(&mut self.state);
            }
            (ContextMenuKind::SpaceHeader { space_key, .. }, Some("Expand")) => {
                self.state.collapsed_space_keys.remove(&space_key);
                self.state.mark_session_dirty();
                leave_modal(&mut self.state);
            }
            (
                ContextMenuKind::MoveTarget {
                    ws_idx,
                    op,
                    targets,
                },
                Some(_),
            ) => {
                if let Some((key, _)) = targets.get(idx) {
                    match crate::spaces::move_parent_for(&self.state.space_nodes, key, op) {
                        Ok(parent) => self.state.move_workspace_space(ws_idx, parent, None),
                        Err(err) => tracing::warn!(error = %err, "move target vanished"),
                    }
                }
                leave_modal(&mut self.state);
            }
            // TP-CHAT-MOVE-04: the chat menu on the mouse road — the same
            // request-parking the keyboard dispatch does.
            (
                ContextMenuKind::WorkspaceChat {
                    ws_idx, session_id, ..
                },
                Some("Move to branch..."),
            ) => {
                let targets = self.state.chat_move_target_entries(ws_idx);
                self.state.context_menu = Some(crate::app::state::ContextMenuState {
                    kind: ContextMenuKind::ChatMoveTarget {
                        session_id,
                        targets,
                    },
                    x: menu_x,
                    y: menu_y,
                    list: crate::app::state::MenuListState::new(0),
                });
            }
            (ContextMenuKind::WorkspaceChat { session_id, .. }, Some("Move back")) => {
                self.state.request_chat_move = Some((session_id, None));
                leave_modal(&mut self.state);
            }
            (
                ContextMenuKind::ChatMoveTarget {
                    session_id,
                    targets,
                },
                Some(_),
            ) => {
                if let Some((key, _)) = targets.get(idx) {
                    self.state.request_chat_move = Some((session_id, Some(key.clone())));
                }
                leave_modal(&mut self.state);
            }
            (ContextMenuKind::GitWorkspace { ws_idx, .. }, Some("Open worktree...")) => {
                self.state.request_open_existing_worktree = Some(ws_idx);
                leave_modal(&mut self.state);
            }
            (
                ContextMenuKind::GitWorkspace {
                    ws_idx, collapsed, ..
                },
                Some("Collapse" | "Expand"),
            ) => {
                if let Some(key) = self
                    .state
                    .workspaces
                    .get(ws_idx)
                    .and_then(|ws| ws.worktree_space())
                    .map(|space| space.key.clone())
                {
                    if collapsed {
                        self.state.collapsed_space_keys.remove(&key);
                    } else {
                        self.state.collapsed_space_keys.insert(key);
                    }
                    self.state.mark_session_dirty();
                }
                leave_modal(&mut self.state);
            }
            (
                ContextMenuKind::Workspace { ws_idx }
                | ContextMenuKind::GitWorkspace { ws_idx, .. },
                Some("Rename"),
            ) => open_rename_workspace(&mut self.state, &self.terminal_runtimes, ws_idx),
            (
                ContextMenuKind::Workspace { ws_idx }
                | ContextMenuKind::GitWorkspace { ws_idx, .. },
                Some("Close" | "Close group"),
            ) => {
                self.state.selected = ws_idx;
                if self.state.confirm_close {
                    open_confirm_close(&mut self.state);
                } else {
                    self.close_workspace_idx_via_api(ws_idx);
                    self.state.mode = Mode::Navigate;
                }
            }
            (ContextMenuKind::Tab { ws_idx, tab_idx }, Some("New tab")) => {
                self.focus_workspace_idx_via_api(ws_idx);
                self.focus_tab_idx_via_api(tab_idx);
                open_new_tab_dialog(&mut self.state);
            }
            (ContextMenuKind::Tab { ws_idx, tab_idx }, Some("Rename")) => {
                self.focus_workspace_idx_via_api(ws_idx);
                self.focus_tab_idx_via_api(tab_idx);
                open_rename_active_tab(&mut self.state, false);
            }
            (ContextMenuKind::Tab { ws_idx, tab_idx }, Some("Close")) => {
                self.focus_workspace_idx_via_api(ws_idx);
                self.focus_tab_idx_via_api(tab_idx);
                if !self.close_active_tab_via_api_requires_confirmation() {
                    leave_modal(&mut self.state);
                }
            }
            (ContextMenuKind::Pane { pane_id, .. }, Some("Rename pane")) => {
                open_rename_pane(&mut self.state, pane_id);
            }
            (
                ContextMenuKind::Pane {
                    ws_idx, pane_id, ..
                },
                Some("Clear pane name"),
            ) => {
                if let Some(pane_id) = self.public_pane_id(ws_idx, pane_id) {
                    self.runtime_pane_rename(
                        "tui.pane.clear_name",
                        crate::api::schema::PaneRenameParams {
                            pane_id,
                            label: None,
                        },
                    );
                }
                self.state.mode = Mode::Terminal;
            }
            (
                ContextMenuKind::Pane {
                    ws_idx,
                    pane_id,
                    source_pane_id: Some(source_pane_id),
                    ..
                },
                Some("Swap with focused pane"),
            ) => {
                let source_public_id = self.public_pane_id(ws_idx, source_pane_id);
                let target_public_id = self.public_pane_id(ws_idx, pane_id);
                if let (Some(source_public_id), Some(target_public_id)) =
                    (source_public_id, target_public_id)
                {
                    self.runtime_pane_swap(
                        "tui.pane.swap_exact",
                        crate::api::schema::PaneSwapParams {
                            pane_id: None,
                            direction: None,
                            source_pane_id: Some(source_public_id),
                            target_pane_id: Some(target_public_id),
                        },
                    );
                    self.focus_pane_internal_via_api(ws_idx, source_pane_id);
                }
                self.state.mode = Mode::Terminal;
            }
            (
                ContextMenuKind::Pane {
                    ws_idx, pane_id, ..
                },
                Some("Split right"),
            ) => {
                self.focus_pane_internal_via_api(ws_idx, pane_id);
                self.split_focused_pane_via_api(crate::api::schema::SplitDirection::Right);
                self.state.mode = Mode::Terminal;
            }
            (
                ContextMenuKind::Pane {
                    ws_idx, pane_id, ..
                },
                Some("Split down"),
            ) => {
                self.focus_pane_internal_via_api(ws_idx, pane_id);
                self.split_focused_pane_via_api(crate::api::schema::SplitDirection::Down);
                self.state.mode = Mode::Terminal;
            }
            (
                ContextMenuKind::Pane {
                    ws_idx, pane_id, ..
                },
                Some("Zoom"),
            ) => {
                self.focus_pane_internal_via_api(ws_idx, pane_id);
                self.zoom_focused_pane_via_api();
                self.state.mode = Mode::Terminal;
            }
            (
                ContextMenuKind::Pane {
                    ws_idx, pane_id, ..
                },
                Some("Close pane"),
            ) => {
                self.focus_pane_internal_via_api(ws_idx, pane_id);
                if !self.close_focused_pane_via_api_requires_confirmation() {
                    self.state.mode = if self.state.active.is_some() {
                        Mode::Terminal
                    } else {
                        Mode::Navigate
                    };
                }
            }
            _ => leave_modal(&mut self.state),
        }
    }
}

fn cancel_rename_modal(state: &mut AppState) {
    state.creating_new_tab = false;
    state.requested_new_tab_name = None;
    state.pending_workspace_create_cwd = None;
    // Disarm a pending move: an escaped name input must never let the next
    // plain rename create a group (TP-RANK-13).
    state.pending_move_new_group = None;
    // Disarm a pending module the same way (TP-DOTS-06).
    state.pending_new_module = None;
    state.rename_pane_target = None;
    state.file_manager_rename = None;
    state.name_input.clear();
    state.name_input_replace_on_type = false;
    leave_modal(state);
}

impl AppState {
    pub(super) fn global_menu_item_at(&self, col: u16, row: u16) -> Option<GlobalMenuAction> {
        let rect = self.global_menu_rect();
        if col <= rect.x
            || col >= rect.x + rect.width.saturating_sub(1)
            || row <= rect.y
            || row >= rect.y + rect.height.saturating_sub(1)
        {
            return None;
        }
        let idx = (row - rect.y - 1) as usize;
        global_menu_actions(self).get(idx).copied()
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::layout::Rect;

    use super::super::{capture_snapshot, state_with_workspaces};
    use super::*;
    use crate::workspace::Workspace;

    fn config_env_lock() -> &'static std::sync::Mutex<()> {
        crate::config::test_config_env_lock()
    }

    fn temp_config_path(name: &str) -> std::path::PathBuf {
        let unique = format!(
            "herdr-modal-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        std::env::temp_dir().join(unique).join("config.toml")
    }

    fn app_with_test_workspaces(names: &[&str]) -> App {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &crate::config::Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        app.state.workspaces = names.iter().map(|name| Workspace::test_new(name)).collect();
        app.state.ensure_test_terminals();
        app.state.active = (!app.state.workspaces.is_empty()).then_some(0);
        app.state.selected = 0;
        app
    }

    #[test]
    fn workspace_create_label_preserves_auto_name_for_suggestion_or_blank() {
        assert_eq!(workspace_create_label("project", "project"), None);
        assert_eq!(workspace_create_label("", "project"), None);
        assert_eq!(workspace_create_label("   ", "project"), None);
        assert_eq!(
            workspace_create_label("  logs  ", "project").as_deref(),
            Some("logs")
        );
    }

    fn mark_worktree_space_member(state: &mut AppState, ws_idx: usize, key: &str) {
        state.workspaces[ws_idx].worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: key.into(),
            label: "herdr".into(),
            repo_root: "/repo/herdr".into(),
            checkout_path: format!("/repo/worktree-{ws_idx}").into(),
            is_linked_worktree: ws_idx != 0,
        });
    }

    #[test]
    fn custom_resize_key_exits_resize_mode() {
        let mut state = state_with_workspaces(&["test"]);
        state.mode = Mode::Resize;
        state.keybinds.resize_mode = crate::config::ActionKeybinds::prefix("g");

        handle_resize_key(
            &mut state,
            TerminalKey::new(KeyCode::Char('g'), KeyModifiers::empty()),
        );

        assert_eq!(state.mode, Mode::Terminal);
    }

    #[test]
    fn direct_resize_key_exits_resize_mode() {
        let mut state = state_with_workspaces(&["test"]);
        state.mode = Mode::Resize;
        state.keybinds.resize_mode = crate::config::ActionKeybinds::direct("ctrl+alt+r");

        handle_resize_key(
            &mut state,
            TerminalKey::new(
                KeyCode::Char('r'),
                KeyModifiers::CONTROL | KeyModifiers::ALT,
            ),
        );

        assert_eq!(state.mode, Mode::Terminal);
    }

    #[test]
    fn resize_key_exit_matches_enhanced_shifted_punctuation() {
        let mut state = state_with_workspaces(&["test"]);
        state.mode = Mode::Resize;
        state.keybinds.resize_mode = crate::config::ActionKeybinds::prefix("?");

        handle_resize_key(
            &mut state,
            TerminalKey::new(KeyCode::Char('/'), KeyModifiers::SHIFT)
                .with_shifted_codepoint('?' as u32),
        );

        assert_eq!(state.mode, Mode::Terminal);
    }

    #[test]
    fn detach_requests_client_detach_in_persistence_mode() {
        let mut state = state_with_workspaces(&["test"]);
        state.detach_exits = false;

        request_detach(&mut state);

        assert!(state.detach_requested);
        assert!(!state.should_quit);
    }

    #[test]
    fn detach_exits_in_no_session_mode() {
        let mut state = state_with_workspaces(&["test"]);
        state.detach_exits = true;

        request_detach(&mut state);

        assert!(state.should_quit);
        assert!(!state.detach_requested);
    }

    #[test]
    fn global_menu_whats_new_opens_saved_release_notes() {
        let _guard = config_env_lock().lock().unwrap();
        let path = temp_config_path("whats-new-saved-release-notes");
        std::env::set_var(crate::config::CONFIG_PATH_ENV_VAR, &path);
        crate::release_notes::save_pending(env!("CARGO_PKG_VERSION"), "### Changed\n- Menu")
            .unwrap();

        let mut state = state_with_workspaces(&["test"]);
        state.latest_release_notes_available = true;

        assert!(global_menu_actions(&state).contains(&GlobalMenuAction::WhatsNew));

        apply_global_menu_action(&mut state, GlobalMenuAction::WhatsNew);

        assert_eq!(state.mode, Mode::ReleaseNotes);
        assert_eq!(
            state
                .release_notes
                .as_ref()
                .map(|notes| notes.body.as_str()),
            Some("### Changed\n- Menu")
        );

        std::env::remove_var(crate::config::CONFIG_PATH_ENV_VAR);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn rename_modal_keyboard_and_mouse_share_actions() {
        let mut state = state_with_workspaces(&["test"]);
        state.mode = Mode::RenameWorkspace;
        state.name_input = "hello".into();

        handle_rename_key(
            &mut state,
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        );
        assert!(state.name_input.is_empty());

        state.name_input = "renamed".into();
        handle_rename_key(
            &mut state,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        );
        assert_eq!(state.mode, Mode::Terminal);
        assert_eq!(state.workspaces[0].display_name(), "renamed");
        let snapshot = capture_snapshot(&state);
        assert_eq!(
            snapshot.workspaces[0].custom_name.as_deref(),
            Some("renamed")
        );

        state.view.sidebar_rect = Rect::new(0, 0, 26, 20);
        state.view.terminal_area = Rect::new(26, 0, 80, 20);
        state.mode = Mode::RenameWorkspace;
        state.name_input = "mouse".into();
        let inner = state.rename_modal_inner().unwrap();
        let (save, _, _) = crate::ui::rename_button_rects(inner);
        let action = modal_action_from_buttons(save.x, save.y, &[(save, ModalAction::Save)]);
        assert_eq!(action, Some(ModalAction::Save));
    }

    #[test]
    fn tab_rename_updates_captured_snapshot() {
        let mut state = state_with_workspaces(&["test"]);
        state.mode = Mode::RenameTab;
        state.name_input = "logs".into();

        handle_rename_key(
            &mut state,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        );

        let snapshot = capture_snapshot(&state);
        assert_eq!(
            snapshot.workspaces[0].tabs[0].custom_name.as_deref(),
            Some("logs")
        );
    }

    #[test]
    fn rename_cancel_returns_to_terminal_when_workspace_is_active() {
        let mut state = state_with_workspaces(&["test"]);
        state.mode = Mode::RenameTab;
        state.name_input = "test".into();

        handle_rename_key(
            &mut state,
            KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()),
        );

        assert_eq!(state.mode, Mode::Terminal);
        assert!(state.name_input.is_empty());
    }

    #[test]
    fn rename_modal_replaces_prefilled_text_on_first_type() {
        let mut state = state_with_workspaces(&["test"]);
        state.mode = Mode::RenameTab;
        state.name_input = "2".into();
        state.name_input_replace_on_type = true;

        handle_rename_key(
            &mut state,
            KeyEvent::new(KeyCode::Char('n'), KeyModifiers::empty()),
        );
        assert_eq!(state.name_input, "n");
        assert!(!state.name_input_replace_on_type);

        handle_rename_key(
            &mut state,
            KeyEvent::new(KeyCode::Char('e'), KeyModifiers::empty()),
        );
        assert_eq!(state.name_input, "ne");
    }

    #[test]
    fn rename_modal_replaces_prefilled_text_on_paste() {
        let mut state = state_with_workspaces(&["test"]);
        state.mode = Mode::RenameTab;
        state.name_input = "2".into();
        state.name_input_replace_on_type = true;

        insert_rename_input_text(&mut state, "feature/logs");

        assert_eq!(state.name_input, "feature/logs");
        assert!(!state.name_input_replace_on_type);

        insert_rename_input_text(&mut state, "-copy");

        assert_eq!(state.name_input, "feature/logs-copy");
    }

    #[test]
    fn rename_modal_handles_line_editing_shortcuts() {
        let mut state = state_with_workspaces(&["test"]);
        state.mode = Mode::RenameWorkspace;
        state.name_input = "website zero".into();

        handle_rename_key(
            &mut state,
            KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty()),
        );
        assert_eq!(state.name_input, "website zer");

        handle_rename_key(
            &mut state,
            KeyEvent::new(KeyCode::Backspace, KeyModifiers::CONTROL),
        );
        assert_eq!(state.name_input, "website ");

        state.name_input = "website-zero".into();
        handle_rename_key(
            &mut state,
            KeyEvent::new(KeyCode::Backspace, KeyModifiers::ALT),
        );
        assert_eq!(state.name_input, "website-");

        state.name_input = "website-zero".into();
        handle_rename_key(
            &mut state,
            KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL),
        );
        assert_eq!(state.name_input, "website-");

        state.name_input = "website-zero".into();
        handle_rename_key(
            &mut state,
            KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL),
        );
        assert_eq!(state.name_input, "website-");

        handle_rename_key(
            &mut state,
            KeyEvent::new(KeyCode::Backspace, KeyModifiers::SUPER),
        );
        assert!(state.name_input.is_empty());

        state.name_input = "website zero".into();
        handle_rename_key(
            &mut state,
            KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
        );
        assert!(state.name_input.is_empty());
    }

    #[test]
    fn rename_modal_does_not_insert_modified_shortcut_chars() {
        let mut state = state_with_workspaces(&["test"]);
        state.mode = Mode::RenameWorkspace;
        state.name_input = "website".into();

        handle_rename_key(
            &mut state,
            KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL),
        );
        assert_eq!(state.name_input, "website");

        handle_rename_key(
            &mut state,
            KeyEvent::new(KeyCode::Char('Z'), KeyModifiers::SHIFT),
        );
        assert_eq!(state.name_input, "websiteZ");
    }

    #[test]
    fn keybind_help_slash_focuses_filter_and_preserves_vim_scroll() {
        let mut state = state_with_workspaces(&["test"]);
        state.keybind_help.query = "stale".into();
        state.keybind_help.search_focused = true;
        state.view.terminal_area = Rect::new(0, 0, 100, 30);

        open_keybind_help(&mut state);
        handle_keybind_help_key(
            &mut state,
            TerminalKey::new(KeyCode::Char('j'), KeyModifiers::empty()),
        );
        assert_eq!(state.keybind_help.scroll, 1);
        handle_keybind_help_key(
            &mut state,
            TerminalKey::new(KeyCode::Char('k'), KeyModifiers::empty()),
        );
        assert_eq!(state.keybind_help.scroll, 0);

        handle_keybind_help_key(
            &mut state,
            TerminalKey::new(KeyCode::Char('w'), KeyModifiers::empty()),
        );
        assert!(state.keybind_help.query.is_empty());

        handle_keybind_help_key(
            &mut state,
            TerminalKey::new(KeyCode::Char('/'), KeyModifiers::empty()),
        );
        for character in "work".chars() {
            state.keybind_help.scroll = 2;
            handle_keybind_help_key(
                &mut state,
                TerminalKey::new(KeyCode::Char(character), KeyModifiers::empty()),
            );
        }

        assert!(state.keybind_help.search_focused);
        assert_eq!(state.keybind_help.query, "work");
        assert_eq!(state.keybind_help.scroll, 0);
    }

    #[test]
    fn keybind_help_query_supports_backspace_clear_and_sanitized_paste() {
        let mut state = state_with_workspaces(&["test"]);
        open_keybind_help(&mut state);
        handle_keybind_help_key(
            &mut state,
            TerminalKey::new(KeyCode::Char('/'), KeyModifiers::empty()),
        );

        insert_keybind_help_query_text(&mut state, "work\nspace");
        assert_eq!(state.keybind_help.query, "workspace");

        handle_keybind_help_key(
            &mut state,
            TerminalKey::new(KeyCode::Backspace, KeyModifiers::empty()),
        );
        assert_eq!(state.keybind_help.query, "workspac");

        handle_keybind_help_key(
            &mut state,
            TerminalKey::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
        );
        assert!(state.keybind_help.query.is_empty());
    }

    #[test]
    fn keybind_help_escape_leaves_search_before_closing() {
        let mut state = state_with_workspaces(&["test"]);
        open_keybind_help(&mut state);
        state.keybind_help.search_focused = true;
        state.keybind_help.query = "work".into();

        handle_keybind_help_key(
            &mut state,
            TerminalKey::new(KeyCode::Esc, KeyModifiers::empty()),
        );
        assert_eq!(state.mode, Mode::KeybindHelp);
        assert!(!state.keybind_help.search_focused);
        assert!(state.keybind_help.query.is_empty());

        handle_keybind_help_key(
            &mut state,
            TerminalKey::new(KeyCode::Esc, KeyModifiers::empty()),
        );
        assert_eq!(state.mode, Mode::Terminal);
    }

    #[test]
    fn enhanced_shifted_slash_focuses_keybind_help_filter() {
        let mut state = state_with_workspaces(&["test"]);
        open_keybind_help(&mut state);

        handle_keybind_help_key(
            &mut state,
            TerminalKey::new(KeyCode::Char('7'), KeyModifiers::SHIFT)
                .with_shifted_codepoint('/' as u32),
        );

        assert!(state.keybind_help.search_focused);
    }

    #[test]
    fn enhanced_shifted_question_mark_closes_keybind_help_when_not_searching() {
        let mut state = state_with_workspaces(&["test"]);
        open_keybind_help(&mut state);

        handle_keybind_help_key(
            &mut state,
            TerminalKey::new(KeyCode::Char('/'), KeyModifiers::SHIFT)
                .with_shifted_codepoint('?' as u32),
        );

        assert_eq!(state.mode, Mode::Terminal);

        open_keybind_help(&mut state);
        handle_keybind_help_key(
            &mut state,
            TerminalKey::new(KeyCode::Char('/'), KeyModifiers::empty()),
        );
        handle_keybind_help_key(
            &mut state,
            TerminalKey::new(KeyCode::Char('/'), KeyModifiers::SHIFT)
                .with_shifted_codepoint('?' as u32),
        );

        assert_eq!(state.keybind_help.query, "?");
    }

    #[test]
    fn navigator_search_accepts_pasted_text_when_focused() {
        let mut state = state_with_workspaces(&["alpha", "beta"]);
        let terminal_runtimes = crate::terminal::TerminalRuntimeRegistry::new();
        state.mode = Mode::Navigator;
        state.navigator.search_focused = true;
        state.navigator.state_filter = Some(NavigatorStateFilter::Working);

        insert_navigator_search_text(&mut state, &terminal_runtimes, "beta");

        assert_eq!(state.navigator.query, "beta");
        assert_eq!(state.navigator.state_filter, None);
    }

    #[test]
    fn navigator_search_ignores_paste_when_search_is_not_focused() {
        let mut state = state_with_workspaces(&["alpha", "beta"]);
        let terminal_runtimes = crate::terminal::TerminalRuntimeRegistry::new();
        state.mode = Mode::Navigator;
        state.navigator.search_focused = false;

        insert_navigator_search_text(&mut state, &terminal_runtimes, "beta");

        assert!(state.navigator.query.is_empty());
    }

    #[test]
    fn navigator_empty_search_escape_returns_to_commands() {
        let mut state = state_with_workspaces(&["alpha", "beta"]);
        let terminal_runtimes = crate::terminal::TerminalRuntimeRegistry::new();
        state.mode = Mode::Navigator;
        state.navigator.search_focused = true;

        handle_navigator_key(
            &mut state,
            &terminal_runtimes,
            KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()),
        );

        assert_eq!(state.mode, Mode::Navigator);
        assert!(!state.navigator.search_focused);
        assert!(state.navigator.query.is_empty());

        handle_navigator_key(
            &mut state,
            &terminal_runtimes,
            KeyEvent::new(KeyCode::Char('w'), KeyModifiers::empty()),
        );

        assert_eq!(
            state.navigator.state_filter,
            Some(NavigatorStateFilter::Working)
        );
        assert!(state.navigator.query.is_empty());

        handle_navigator_key(
            &mut state,
            &terminal_runtimes,
            KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()),
        );

        assert_eq!(state.mode, Mode::Terminal);
    }

    #[test]
    fn navigator_search_escape_blurs_then_next_escape_closes() {
        let mut state = state_with_workspaces(&["alpha", "beta"]);
        let terminal_runtimes = crate::terminal::TerminalRuntimeRegistry::new();
        state.mode = Mode::Navigator;
        state.navigator.search_focused = true;
        state.navigator.query = "a".into();

        handle_navigator_key(
            &mut state,
            &terminal_runtimes,
            KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()),
        );

        assert_eq!(state.mode, Mode::Navigator);
        assert!(!state.navigator.search_focused);
        assert_eq!(state.navigator.query, "a");

        handle_navigator_key(
            &mut state,
            &terminal_runtimes,
            KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty()),
        );

        assert_eq!(state.navigator.selected, 1);
        assert_eq!(state.navigator.query, "a");

        handle_navigator_key(
            &mut state,
            &terminal_runtimes,
            KeyEvent::new(KeyCode::Char('/'), KeyModifiers::empty()),
        );

        assert_eq!(state.mode, Mode::Navigator);
        assert!(state.navigator.search_focused);
        assert_eq!(state.navigator.query, "a");

        handle_navigator_key(
            &mut state,
            &terminal_runtimes,
            KeyEvent::new(KeyCode::Char('l'), KeyModifiers::empty()),
        );

        assert_eq!(state.navigator.query, "al");

        handle_navigator_key(
            &mut state,
            &terminal_runtimes,
            KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()),
        );

        assert_eq!(state.mode, Mode::Navigator);
        assert!(!state.navigator.search_focused);

        handle_navigator_key(
            &mut state,
            &terminal_runtimes,
            KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()),
        );

        assert_eq!(state.mode, Mode::Terminal);
    }

    #[test]
    fn open_rename_active_tab_can_prefill_default_new_tab_name() {
        let mut state = state_with_workspaces(&["test"]);
        state.workspaces[0].test_add_tab(None);
        state.workspaces[0].switch_tab(1);

        open_rename_active_tab(&mut state, true);

        assert_eq!(state.mode, Mode::RenameTab);
        assert_eq!(state.name_input, "2");
        assert!(state.name_input_replace_on_type);
    }

    #[test]
    fn cancel_new_tab_dialog_leaves_workspace_unchanged() {
        let mut state = state_with_workspaces(&["test"]);
        open_new_tab_dialog(&mut state);

        handle_rename_key(
            &mut state,
            KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()),
        );

        assert_eq!(state.mode, Mode::Terminal);
        assert!(!state.creating_new_tab);
        assert!(!state.request_new_tab);
        assert!(state.requested_new_tab_name.is_none());
        assert_eq!(state.workspaces[0].tabs.len(), 1);
    }

    #[test]
    fn saving_new_tab_dialog_requests_creation_with_name() {
        let mut state = state_with_workspaces(&["test"]);
        open_new_tab_dialog(&mut state);
        state.name_input = "logs".into();
        state.name_input_replace_on_type = false;

        handle_rename_key(
            &mut state,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        );

        assert_eq!(state.mode, Mode::Terminal);
        assert!(!state.creating_new_tab);
        assert!(state.request_new_tab);
        assert_eq!(state.requested_new_tab_name.as_deref(), Some("logs"));
    }

    #[test]
    fn saving_new_tab_dialog_with_default_name_keeps_tab_auto_named() {
        let mut state = state_with_workspaces(&["test"]);
        open_new_tab_dialog(&mut state);

        handle_rename_key(
            &mut state,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        );

        assert_eq!(state.mode, Mode::Terminal);
        assert!(!state.creating_new_tab);
        assert!(state.request_new_tab);
        assert!(state.requested_new_tab_name.is_none());
    }

    #[test]
    fn closing_first_auto_tab_compacts_remaining_auto_tab_label_and_next_prompt() {
        let mut state = state_with_workspaces(&["test"]);
        open_new_tab_dialog(&mut state);
        handle_rename_key(
            &mut state,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        );

        state.workspaces[0].test_add_tab(state.requested_new_tab_name.as_deref());
        state.request_new_tab = false;
        state.requested_new_tab_name = None;

        state.workspaces[0].close_tab(0);
        state.workspaces[0].switch_tab(0);

        assert_eq!(
            state.workspaces[0].tab_display_name(0).as_deref(),
            Some("1")
        );
        assert!(state.workspaces[0].tabs[0].custom_name.is_none());

        open_new_tab_dialog(&mut state);
        assert_eq!(state.name_input, "2");
    }

    #[test]
    fn renaming_auto_tab_to_its_default_number_keeps_it_auto_named() {
        let mut state = state_with_workspaces(&["test"]);
        state.workspaces[0].test_add_tab(None);
        state.workspaces[0].switch_tab(1);

        open_rename_active_tab(&mut state, false);
        handle_rename_key(
            &mut state,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        );

        assert_eq!(state.mode, Mode::Terminal);
        assert!(state.workspaces[0].tabs[1].custom_name.is_none());
        assert_eq!(
            state.workspaces[0].tab_display_name(1).as_deref(),
            Some("2")
        );
    }

    #[test]
    fn confirm_close_keyboard_actions_are_direct_not_focused() {
        let mut state = state_with_workspaces(&["a", "b"]);
        state.mode = Mode::ConfirmClose;
        state.selected = 1;

        handle_confirm_close_key(
            &mut state,
            KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()),
        );
        assert_eq!(state.mode, Mode::Navigate);
        assert_eq!(state.workspaces.len(), 2);

        state.mode = Mode::ConfirmClose;
        handle_confirm_close_key(
            &mut state,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        );
        assert_eq!(state.workspaces.len(), 1);
    }

    #[test]
    fn confirm_close_for_linked_worktree_closes_workspace_only() {
        let mut state = state_with_workspaces(&["main", "issue"]);
        state.mode = Mode::ConfirmClose;
        state.selected = 1;
        state.workspaces[1].worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "repo-key".into(),
            label: "herdr".into(),
            repo_root: "/repo/herdr".into(),
            checkout_path: "/repo/herdr-issue".into(),
            is_linked_worktree: true,
        });

        handle_confirm_close_key(
            &mut state,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        );

        assert_eq!(state.request_remove_linked_worktree, None);
        assert_eq!(state.workspaces.len(), 1);
        assert_eq!(state.workspaces[0].display_name(), "main");
        assert_eq!(state.mode, Mode::Terminal);
    }

    #[test]
    fn context_menu_close_group_opens_group_close_confirmation() {
        let mut state = state_with_workspaces(&["main", "issue"]);
        state.active = Some(0);
        state.selected = 1;
        state.workspaces[0].worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "repo-key".into(),
            label: "herdr".into(),
            repo_root: "/repo/herdr".into(),
            checkout_path: "/repo/herdr".into(),
            is_linked_worktree: false,
        });
        state.workspaces[1].worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "repo-key".into(),
            label: "herdr".into(),
            repo_root: "/repo/herdr".into(),
            checkout_path: "/repo/herdr-issue".into(),
            is_linked_worktree: true,
        });
        let menu = ContextMenuState {
            kind: ContextMenuKind::GitWorkspace {
                ws_idx: 0,
                is_linked_worktree: false,
                has_worktree_children: true,
                collapsed: false,
                space_is_custom: false,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };
        let mut terminal_runtimes = crate::terminal::TerminalRuntimeRegistry::new();

        apply_context_menu_action(&mut state, &mut terminal_runtimes, menu, 1);

        assert_eq!(state.selected, 0);
        assert_eq!(state.mode, Mode::ConfirmClose);

        confirm_close_accept(&mut state);

        assert!(state.workspaces.is_empty());
        assert_eq!(state.mode, Mode::Navigate);
    }

    #[test]
    fn context_menu_close_pane_last_parent_group_pane_keeps_confirmation_mode() {
        let mut state = state_with_workspaces(&["main", "issue"]);
        state.active = Some(0);
        state.selected = 1;
        state.workspaces[0].worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "repo-key".into(),
            label: "herdr".into(),
            repo_root: "/repo/herdr".into(),
            checkout_path: "/repo/herdr".into(),
            is_linked_worktree: false,
        });
        state.workspaces[1].worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "repo-key".into(),
            label: "herdr".into(),
            repo_root: "/repo/herdr".into(),
            checkout_path: "/repo/herdr-issue".into(),
            is_linked_worktree: true,
        });
        let pane_id = state.workspaces[0].tabs[0].root_pane;
        let menu = ContextMenuState {
            kind: ContextMenuKind::Pane {
                ws_idx: 0,
                tab_idx: 0,
                pane_id,
                source_pane_id: None,
                has_manual_label: false,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };
        let idx = menu
            .items()
            .iter()
            .position(|item| *item == "Close pane")
            .expect("close pane item");
        let mut terminal_runtimes = crate::terminal::TerminalRuntimeRegistry::new();

        apply_context_menu_action(&mut state, &mut terminal_runtimes, menu, idx);

        assert_eq!(state.selected, 0);
        assert_eq!(state.mode, Mode::ConfirmClose);
        assert_eq!(state.workspaces.len(), 2);
    }

    #[test]
    fn api_context_menu_close_tab_last_parent_group_workspace_keeps_confirmation_mode() {
        let mut app = app_with_test_workspaces(&["main", "issue"]);
        mark_worktree_space_member(&mut app.state, 0, "repo-key");
        mark_worktree_space_member(&mut app.state, 1, "repo-key");
        app.state.active = Some(0);
        app.state.selected = 1;
        app.state.mode = Mode::ContextMenu;
        let menu = ContextMenuState {
            kind: ContextMenuKind::Tab {
                ws_idx: 0,
                tab_idx: 0,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };
        let idx = menu
            .items()
            .iter()
            .position(|item| *item == "Close")
            .expect("close tab item");

        app.apply_context_menu_action_via_api(menu, idx);

        assert_eq!(app.state.selected, 0);
        assert_eq!(app.state.mode, Mode::ConfirmClose);
        assert_eq!(app.state.workspaces.len(), 2);
    }

    // ---- Projects-tab worktree menu entries (FEAT-A) ----

    fn app_with_pinned_project(project_path: &str) -> crate::app::App {
        let mut app = app_with_test_workspaces(&["main"]);
        app.state.workspaces[0].identity_cwd = std::path::PathBuf::from(project_path);
        app.state.projects_sessions = vec![crate::app::state::ProjectSessions {
            path: std::path::PathBuf::from(project_path),
            sessions: Vec::new(),
            total_count: 0,
        }];
        app
    }

    fn project_menu(has_workspace: bool) -> ContextMenuState {
        ContextMenuState {
            kind: ContextMenuKind::ProjectNewChat {
                proj_idx: 0,
                has_workspace,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        }
    }

    fn item_index(menu: &ContextMenuState, label: &str) -> usize {
        menu.items()
            .iter()
            .position(|item| *item == label)
            .unwrap_or_else(|| panic!("menu should offer {label:?}"))
    }

    fn app_with_movable_branch() -> crate::app::App {
        let mut app = app_with_test_workspaces(&["tiling"]);
        app.state.workspaces[0].cached_git_branch = Some("worktree/Tiling".into());
        app.state.workspaces[0].worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "repo-key".into(),
            label: "herdr".into(),
            repo_root: std::path::PathBuf::from("/repo/herdr"),
            checkout_path: std::path::PathBuf::from("/repo/herdr-tiling"),
            is_linked_worktree: true,
        });
        app.state.space_nodes = vec![
            crate::spaces::SpaceNode {
                key: "group:ui".into(),
                name: "UI".into(),
                icon: None,
                parent: None,
            },
            crate::spaces::SpaceNode {
                key: "group:ops".into(),
                name: "Ops".into(),
                icon: None,
                parent: Some("group:ui".into()),
            },
        ];
        app
    }

    fn linked_branch_menu() -> ContextMenuState {
        ContextMenuState {
            kind: ContextMenuKind::GitWorkspace {
                ws_idx: 0,
                is_linked_worktree: true,
                has_worktree_children: false,
                collapsed: false,
                space_is_custom: false,
            },
            x: 3,
            y: 7,
            list: MenuListState::new(0),
        }
    }

    // TP-RANK-13: "Move..." opens the verb submenu in place, and a verb
    // opens the target picker loaded with the forest — names shown, keys
    // carried.
    #[test]
    fn move_walks_the_submenu_then_the_picker() {
        let mut app = app_with_movable_branch();
        // A menu selection always happens from inside the menu overlay.
        app.state.mode = Mode::ContextMenu;
        let menu = linked_branch_menu();
        let idx = item_index(&menu, "Move...");

        app.apply_context_menu_action_via_api(menu, idx);

        assert_eq!(app.state.mode, Mode::ContextMenu, "the chain stays modal");
        let submenu = app.state.context_menu.clone().expect("submenu is open");
        assert!(
            matches!(
                submenu.kind,
                ContextMenuKind::MoveWorkspace {
                    ws_idx: 0,
                    has_targets: true,
                }
            ),
            "got {:?}",
            submenu.kind
        );
        assert_eq!((submenu.x, submenu.y), (3, 7), "the chain stays anchored");

        let idx = item_index(&submenu, "Under a group...");
        app.apply_context_menu_action_via_api(submenu, idx);

        let picker = app.state.context_menu.clone().expect("picker is open");
        assert_eq!(picker.items(), &["UI", "Ops"], "names, never keys");
        match &picker.kind {
            ContextMenuKind::MoveTarget {
                ws_idx: 0,
                op: crate::spaces::MoveOp::Under,
                targets,
            } => {
                assert_eq!(targets[0].0, "group:ui");
                assert_eq!(targets[1].0, "group:ops");
            }
            other => panic!("expected the under-picker, got {other:?}"),
        }
    }

    // TP-RANK-13: naming road — the new-group entry closes the menu chain
    // and opens the rename input armed for the move.
    #[test]
    fn a_new_group_pick_opens_the_name_input() {
        let mut app = app_with_movable_branch();
        let submenu = ContextMenuState {
            kind: ContextMenuKind::MoveWorkspace {
                ws_idx: 0,
                has_targets: true,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };
        let idx = item_index(&submenu, "Under a new group...");

        app.apply_context_menu_action_via_api(submenu, idx);

        assert_eq!(app.state.mode, Mode::RenameWorkspace);
        assert_eq!(app.state.pending_move_new_group, Some(0));
        assert_eq!(app.state.name_input, "", "the name starts empty");
        assert!(app.state.context_menu.is_none(), "the menu chain is done");
    }

    // TP-CHAT-MOVE-04: the chat menu's move road — the picker lists the
    // other open drawers by ledger key, and the selection parks a request
    // for the App loop, which owns the ledger.
    #[test]
    fn chat_move_walks_the_picker_and_requests_the_move() {
        let mut app = app_with_test_workspaces(&["tiling", "other"]);
        app.state.workspaces[0].identity_cwd = std::path::PathBuf::from("/repo/a");
        app.state.workspaces[1].identity_cwd = std::path::PathBuf::from("/repo/b");
        app.state.mode = Mode::ContextMenu;
        let menu = ContextMenuState {
            kind: ContextMenuKind::WorkspaceChat {
                ws_idx: 0,
                session_id: "s1".into(),
                has_move: false,
            },
            x: 2,
            y: 5,
            list: MenuListState::new(0),
        };
        let idx = item_index(&menu, "Move to branch...");

        app.apply_context_menu_action_via_api(menu, idx);

        // The real dispatcher takes the menu out of the state before acting
        // on a selection; the test mirrors that hand-off.
        let picker = app.state.context_menu.take().expect("picker is open");
        match &picker.kind {
            ContextMenuKind::ChatMoveTarget {
                session_id,
                targets,
            } => {
                assert_eq!(session_id, "s1");
                assert_eq!(
                    targets.len(),
                    1,
                    "the chat's own drawer is not a destination"
                );
                assert_eq!(targets[0].0, "/repo/b");
            }
            other => panic!("expected the chat target picker, got {other:?}"),
        }

        app.apply_context_menu_action_via_api(picker, 0);

        assert_eq!(
            app.state.request_chat_move,
            Some(("s1".to_string(), Some("/repo/b".to_string())))
        );
        assert!(app.state.context_menu.is_none(), "the chain is done");
    }

    // TP-CHAT-MOVE-04: "Move back" parks a withdrawal for the App loop.
    #[test]
    fn move_back_requests_a_clear() {
        let mut app = app_with_test_workspaces(&["tiling"]);
        app.state.mode = Mode::ContextMenu;
        let menu = ContextMenuState {
            kind: ContextMenuKind::WorkspaceChat {
                ws_idx: 0,
                session_id: "s1".into(),
                has_move: true,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };
        let idx = item_index(&menu, "Move back");

        app.apply_context_menu_action_via_api(menu, idx);

        assert_eq!(app.state.request_chat_move, Some(("s1".to_string(), None)));
        assert!(app.state.context_menu.is_none());
    }

    // TP-RANK-13: an escaped name input disarms the move — the next plain
    // rename must never turn into a group creation.
    #[test]
    fn an_escaped_group_name_never_leaks_into_the_next_rename() {
        let mut app = app_with_movable_branch();
        let submenu = ContextMenuState {
            kind: ContextMenuKind::MoveWorkspace {
                ws_idx: 0,
                has_targets: true,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };
        let idx = item_index(&submenu, "Under a new group...");
        app.apply_context_menu_action_via_api(submenu, idx);
        assert_eq!(app.state.pending_move_new_group, Some(0));

        app.apply_rename_mouse_action_via_api(ModalAction::Cancel);

        assert_eq!(
            app.state.pending_move_new_group, None,
            "cancel disarms the pending move"
        );
    }

    // TP-DOTS-01: the node header's menu — creation plus the one fold verb
    // the current state calls for. The bucket header only folds: a split
    // rule cannot parent a node, so offering creation there would be a
    // promise the tree cannot keep.
    #[test]
    fn header_menus_offer_creation_and_the_right_fold_verb() {
        let node_menu = |collapsed| ContextMenuState {
            kind: ContextMenuKind::NodeHeader {
                node_key: "group:docs".into(),
                collapsed,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };
        assert_eq!(
            node_menu(false).items(),
            vec!["New sub-module...", "New parallel module...", "Collapse"]
        );
        assert_eq!(
            node_menu(true).items(),
            vec!["New sub-module...", "New parallel module...", "Expand"]
        );

        let space_menu = ContextMenuState {
            kind: ContextMenuKind::SpaceHeader {
                space_key: "repo-key".into(),
                collapsed: false,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };
        assert_eq!(space_menu.items(), vec!["Collapse"]);
    }

    // TP-DOTS-05: "New sub-module..." closes the menu chain and arms the
    // rename input with the header itself as the parent.
    #[test]
    fn a_new_sub_module_pick_opens_the_name_input_with_the_header_as_parent() {
        let mut app = app_with_movable_branch();
        let menu = ContextMenuState {
            kind: ContextMenuKind::NodeHeader {
                node_key: "group:ui".into(),
                collapsed: false,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };
        let idx = item_index(&menu, "New sub-module...");

        app.apply_context_menu_action_via_api(menu, idx);

        assert_eq!(app.state.mode, Mode::RenameWorkspace);
        assert_eq!(
            app.state.pending_new_module,
            Some(crate::app::state::PendingNewModule {
                parent: Some("group:ui".into()),
            })
        );
        assert_eq!(app.state.name_input, "", "the name starts empty");
        assert!(app.state.context_menu.is_none(), "the menu chain is done");
    }

    // TP-DOTS-07: "parallel" hangs the new module beside the header — the
    // parent is the header's OWN parent, and a top-level header makes a
    // top-level sibling.
    #[test]
    fn a_parallel_module_pick_arms_with_the_headers_own_parent() {
        let mut app = app_with_movable_branch();
        app.state.space_nodes = vec![
            crate::spaces::SpaceNode {
                key: "group:ops".into(),
                name: "Ops".into(),
                icon: None,
                parent: None,
            },
            crate::spaces::SpaceNode {
                key: "group:ui".into(),
                name: "UI".into(),
                icon: None,
                parent: Some("group:ops".into()),
            },
        ];
        let menu_for = |key: &str| ContextMenuState {
            kind: ContextMenuKind::NodeHeader {
                node_key: key.into(),
                collapsed: false,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };

        let menu = menu_for("group:ui");
        let idx = item_index(&menu, "New parallel module...");
        app.apply_context_menu_action_via_api(menu, idx);
        assert_eq!(
            app.state.pending_new_module,
            Some(crate::app::state::PendingNewModule {
                parent: Some("group:ops".into()),
            }),
            "a nested header's sibling shares its parent"
        );

        app.state.pending_new_module = None;
        let menu = menu_for("group:ops");
        let idx = item_index(&menu, "New parallel module...");
        app.apply_context_menu_action_via_api(menu, idx);
        assert_eq!(
            app.state.pending_new_module,
            Some(crate::app::state::PendingNewModule { parent: None }),
            "a top-level header's sibling is top level"
        );
    }

    // TP-DOTS-06: an escaped name input disarms the pending module — the
    // next plain rename must never write a node.
    #[test]
    fn an_escaped_module_name_never_creates_on_the_next_rename() {
        let mut app = app_with_movable_branch();
        app.state.pending_new_module = Some(crate::app::state::PendingNewModule {
            parent: Some("group:ui".into()),
        });
        app.state.enter_overlay_mode(Mode::RenameWorkspace);

        app.apply_rename_mouse_action_via_api(ModalAction::Cancel);

        assert_eq!(
            app.state.pending_new_module, None,
            "cancel disarms the pending module"
        );
    }

    // TP-DOTS-02 companion: the fold verbs do from the menu exactly what a
    // left press does on the row, for both header kinds.
    #[test]
    fn header_menu_fold_verbs_fold_and_unfold() {
        let mut app = app_with_movable_branch();

        let node_menu = |collapsed| ContextMenuState {
            kind: ContextMenuKind::NodeHeader {
                node_key: "group:ui".into(),
                collapsed,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };
        let menu = node_menu(false);
        let idx = item_index(&menu, "Collapse");
        app.apply_context_menu_action_via_api(menu, idx);
        assert!(app.state.node_folded("group:ui"), "the menu folds the node");

        let menu = node_menu(true);
        let idx = item_index(&menu, "Expand");
        app.apply_context_menu_action_via_api(menu, idx);
        assert!(
            !app.state.node_folded("group:ui"),
            "the menu unfolds the node"
        );

        let space_menu = ContextMenuState {
            kind: ContextMenuKind::SpaceHeader {
                space_key: "repo-key".into(),
                collapsed: false,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };
        let idx = item_index(&space_menu, "Collapse");
        app.apply_context_menu_action_via_api(space_menu, idx);
        assert!(
            app.state.collapsed_space_keys.contains("repo-key"),
            "the menu folds the bucket the way a left press does"
        );
    }

    #[test]
    fn project_menu_new_worktree_targets_the_matching_workspace() {
        let mut app = app_with_pinned_project("/proj/herdr");
        let menu = project_menu(true);
        let idx = item_index(&menu, "New worktree");
        let default_agent = app.state.default_chat_agent.clone();

        app.apply_context_menu_action_via_api(menu, idx);

        assert_eq!(app.state.request_new_linked_worktree, Some(0));
        assert!(
            app.state.request_project_chat_tab.is_none(),
            "worktree rows must not fall through to the agent arm"
        );
        assert_eq!(app.state.default_chat_agent, default_agent);
        assert_ne!(app.state.mode, Mode::ContextMenu);
    }

    #[test]
    fn project_menu_open_worktree_targets_the_matching_workspace() {
        let mut app = app_with_pinned_project("/proj/herdr");
        let menu = project_menu(true);
        let idx = item_index(&menu, "Open worktree...");

        app.apply_context_menu_action_via_api(menu, idx);

        assert_eq!(app.state.request_open_existing_worktree, Some(0));
        assert!(app.state.request_project_chat_tab.is_none());
    }

    #[test]
    fn project_menu_worktree_actions_are_inert_without_a_matching_workspace() {
        let mut app = app_with_pinned_project("/proj/herdr");
        // The workspace moved away between menu open and selection.
        app.state.workspaces[0].identity_cwd = std::path::PathBuf::from("/elsewhere");
        let menu = project_menu(true);
        let idx = item_index(&menu, "New worktree");
        let default_agent = app.state.default_chat_agent.clone();

        app.apply_context_menu_action_via_api(menu, idx);

        assert_eq!(app.state.request_new_linked_worktree, None);
        assert_eq!(app.state.default_chat_agent, default_agent);
        assert_ne!(app.state.mode, Mode::ContextMenu);
    }

    #[test]
    fn project_menu_agent_selection_still_opens_the_chat() {
        let mut app = app_with_pinned_project("/proj/herdr");
        let menu = project_menu(true);
        let idx = item_index(&menu, "claude");

        app.apply_context_menu_action_via_api(menu, idx);

        assert_eq!(app.state.default_chat_agent, "claude");
        assert_eq!(
            app.state
                .request_project_chat_tab
                .as_ref()
                .map(|req| req.project_path.clone()),
            Some(std::path::PathBuf::from("/proj/herdr"))
        );
        assert_eq!(app.state.request_new_linked_worktree, None);
    }

    #[test]
    fn api_context_menu_enter_close_pane_last_parent_group_pane_keeps_confirmation_mode() {
        let mut app = app_with_test_workspaces(&["main", "issue"]);
        mark_worktree_space_member(&mut app.state, 0, "repo-key");
        mark_worktree_space_member(&mut app.state, 1, "repo-key");
        app.state.active = Some(0);
        app.state.selected = 1;
        app.state.mode = Mode::ContextMenu;
        let pane_id = app.state.workspaces[0].tabs[0].root_pane;
        let mut menu = ContextMenuState {
            kind: ContextMenuKind::Pane {
                ws_idx: 0,
                tab_idx: 0,
                pane_id,
                source_pane_id: None,
                has_manual_label: false,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };
        let close_idx = menu
            .items()
            .iter()
            .position(|item| *item == "Close pane")
            .expect("close pane item");
        menu.list.highlighted = close_idx;
        app.state.context_menu = Some(menu);

        app.handle_context_menu_key_via_api(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));

        assert_eq!(app.state.selected, 0);
        assert_eq!(app.state.mode, Mode::ConfirmClose);
        assert_eq!(app.state.workspaces.len(), 2);
        assert!(app.state.context_menu.is_none());
    }
}
