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
    open_module_name_input(state, parent, None);
}

/// TP-MOD-32: the rename prompt. Deliberately the same body as the
/// creation prompt — one modal, one set of cleared fields — differing only
/// in the key it carries forward, because a rename that derived a new key
/// from the new name would re-key the container and orphan its children.
fn open_rename_module_input(state: &mut AppState, node_key: String, parent: Option<String>) {
    open_module_name_input(state, parent, Some(node_key));
}

/// Open the text box on a chat's name, prefilled with what the row reads now.
///
/// TP-CHAT-NAME-01: prefilled and selected, like the directory road — the
/// common edit is a correction, and a chat whose row already says something
/// useful should not have to be retyped from nothing to be adjusted.
fn open_chat_rename_input(state: &mut AppState, session_id: String, current: Option<String>) {
    state.pending_chat_rename = Some(session_id);
    state.pending_module_dir = None;
    state.pending_new_module = None;
    state.pending_move_new_group = None;
    state.pending_workspace_create_cwd = None;
    state.rename_pane_target = None;
    state.name_input = current.unwrap_or_default();
    state.name_input_replace_on_type = !state.name_input.is_empty();
    state.context_menu = None;
    state.enter_overlay_mode(Mode::RenameWorkspace);
}

fn open_module_dir_input(state: &mut AppState, node_key: String, current: Option<String>) {
    state.pending_module_dir = Some(node_key);
    state.pending_chat_rename = None;
    state.pending_new_module = None;
    state.pending_move_new_group = None;
    state.pending_workspace_create_cwd = None;
    state.rename_pane_target = None;
    // Prefilled with what the module already points at, and selected: the
    // common edit is a correction, and retyping a path by hand is where typos
    // come from.
    state.name_input = current.unwrap_or_default();
    state.name_input_replace_on_type = !state.name_input.is_empty();
    state.context_menu = None;
    state.enter_overlay_mode(Mode::RenameWorkspace);
}

fn open_module_name_input(
    state: &mut AppState,
    parent: Option<String>,
    rename_key: Option<String>,
) {
    state.pending_new_module = Some(crate::app::state::PendingNewModule { parent, rename_key });
    state.pending_module_dir = None;
    state.pending_chat_rename = None;
    state.pending_move_new_group = None;
    state.pending_workspace_create_cwd = None;
    state.rename_pane_target = None;
    state.name_input = String::new();
    state.name_input_replace_on_type = false;
    state.context_menu = None;
    state.enter_overlay_mode(Mode::RenameWorkspace);
}

/// TP-DOTS-17: the one branch-road every door walks. The header menu's
/// "New branch..." (keyboard and mouse dispatch) and the header's trailing
/// "+" all share this body, so the doors can never drift apart: resolve a
/// source workspace under the module (ancestors included, TP-DOTS-14),
/// arm the module and request the proven worktree dialog — or report the
/// missing repository instead of silently doing nothing.
pub(super) fn start_branch_from_module(state: &mut AppState, module_key: String) {
    use crate::ui::ModuleBranchSource;

    match crate::ui::module_branch_source(state, &module_key) {
        ModuleBranchSource::Workspace(ws_idx) => {
            state.pending_branch_module = Some(module_key);
            state.request_new_linked_worktree = Some(ws_idx);
            state.context_menu = None;
        }
        // TP-MOD-37: the module stands in a repository of its own but nothing
        // has it open yet. Opening it is the missing step, not an error to
        // report — the person stated this directory so the module would have
        // somewhere to branch from.
        ModuleBranchSource::Repository(_) => {
            state.request_module_branch_workspace = Some(module_key);
            state.context_menu = None;
        }
        // TP-MOD-38: a real directory that is not a repository yet. Saying so
        // and naming the verb that fixes it is the difference between an
        // explanation and a dead end.
        //
        // TP-MOD-40: broken across lines because the panel truncates and does
        // not wrap. Written as one sentence the verb sat at column 105 of a
        // 158-character line and was cut off the screen — the dead end this
        // message exists to prevent, dressed as an explanation. The path goes
        // last: it is the one part that can be any length, so it is the only
        // part that can afford to lose its tail.
        ModuleBranchSource::UninitializedDirectory(dir) => {
            state.config_diagnostic = Some(format!(
                "module {module_key:?} is not a git repository yet\n\
                 use \"Initialize git repository\" on the module to set one up\n\
                 {}",
                dir.display()
            ));
            state.context_menu = None;
        }
        ModuleBranchSource::NoDirectory => {
            state.config_diagnostic = Some(format!(
                "module {module_key:?} has no directory yet\n\
                 use \"Set directory...\" to point it at one, or move a branch under it"
            ));
            state.context_menu = None;
        }
    }
}

/// TP-AGPANEL-45: the graveyard verbs, in one body both context-menu bodies
/// call. `Forget` is pure state so it happens here; `Revive` parks a request
/// because the API call lives on `App`, which the `#[cfg(test)]` body does not
/// have. Writing either one out twice is how a verb comes to mean two different
/// things depending on which door was used.
pub(super) fn apply_closed_agent_action(state: &mut AppState, agent_id: String, item: &str) {
    match item {
        "Revive" => {
            state.request_revive_closed_agent = Some(agent_id);
        }
        "Forget" => {
            // A row that a refresh already took away is not an error: the menu
            // can outlive the list it was opened from — so the removal is
            // measured, and only a real one dirties the session.
            //
            // Kept as a binding rather than folded into the match arm: clippy
            // would rather see `"Forget" if state.closed_agents.forget(..)`,
            // which hides a mutation inside a pattern guard, where a reader
            // scanning the arms would never look for one.
            let removed = state.closed_agents.forget(&agent_id);
            if removed {
                state.mark_session_dirty();
            }
        }
        _ => {}
    }
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
                // TP-CHAT-NAME-01: first, because its pending is only ever
                // set by an opener that clears every other one — so if it is
                // set, this is unambiguously the road being walked.
                Mode::RenameWorkspace if state.pending_chat_rename.is_some() => {
                    if let Some(session_id) = state.pending_chat_rename.take() {
                        state.request_chat_rename = Some((session_id, new_name));
                    }
                }
                Mode::RenameWorkspace if state.pending_module_dir.is_some() => {
                    if let Some(node_key) = state.pending_module_dir.take() {
                        state.submit_module_dir(node_key, &new_name);
                    }
                }
                Mode::RenameWorkspace if state.pending_new_module.is_some() => {
                    if let Some(pending) = state.pending_new_module.take() {
                        state.submit_module_name(pending.rename_key, pending.parent, &new_name);
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
        (ContextMenuKind::SidebarBlank, Some("New module...")) => {
            open_new_module_input(state, None);
        }
        (ContextMenuKind::ClosedAgent { agent_id }, Some(item @ ("Revive" | "Forget"))) => {
            apply_closed_agent_action(state, agent_id, item);
            leave_modal(state);
        }
        (ContextMenuKind::DailyHeader { .. }, Some("New chat...")) => {
            state.open_daily_new_chat_menu(menu_x, menu_y);
        }
        // TP-DAILY-19: one line, and the sibling body below writes the same
        // one. The work happens in the App loop, which is the only place that
        // can dispatch the pane moves — writing it out here instead would give
        // the verb two implementations and one of them would rot.
        (ContextMenuKind::DailyHeader { .. }, Some("Merge workspaces here")) => {
            state.request_merge_daily_workspaces = true;
            leave_modal(state);
        }
        (ContextMenuKind::DailyHeader { .. }, Some("Collapse" | "Expand")) => {
            state.toggle_daily_section();
            leave_modal(state);
        }
        // TP-DAILY-11: the same road, rooted at the daily directory instead of
        // at a workspace — the section's answer to "start something new here".
        (ContextMenuKind::DailyNewChat, Some(agent)) => {
            state.default_chat_agent = agent.to_string();
            state.request_daily_chat();
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
        // TP-DOTS-13/14: the branch road — resolve a source workspace under
        // the module (ancestors included) and reuse the proven worktree
        // dialog; the module rides in its own pending slot to the submit.
        (
            ContextMenuKind::NodeHeader { node_key: key, .. }
            | ContextMenuKind::SpaceHeader { space_key: key, .. },
            Some("New branch..."),
        ) => {
            start_branch_from_module(state, key);
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
        // TP-MOD-35: the file picker, not a text box. Falls back to the text
        // box only when the picker cannot open at all — a verb that does
        // nothing is worse than a verb that asks you to type.
        (ContextMenuKind::NodeHeader { node_key, .. }, Some("Set directory...")) => {
            if !state.open_module_dir_picker(node_key.clone()) {
                let current = state
                    .space_nodes
                    .iter()
                    .find(|node| node.key == node_key)
                    .and_then(|node| node.dir.as_ref())
                    .map(|dir| dir.display().to_string());
                open_module_dir_input(state, node_key, current);
            }
        }
        // TP-MOD-38: the step that was missing between "Set directory..." and
        // "New branch...". The key travels, not the path — the App loop
        // re-resolves and re-measures before it writes anything, because a
        // directory can become a repository between the menu opening and this
        // item being picked.
        (ContextMenuKind::NodeHeader { node_key, .. }, Some("Initialize git repository")) => {
            state.request_module_git_init = Some(node_key);
            leave_modal(state);
        }
        (ContextMenuKind::NodeHeader { node_key, .. }, Some("Rename module...")) => {
            let parent = state
                .space_nodes
                .iter()
                .find(|node| node.key == node_key)
                .and_then(|node| node.parent.clone());
            open_rename_module_input(state, node_key, parent);
        }
        // TP-MOD-08: the keyboard road to taking a module back.
        (ContextMenuKind::NodeHeader { node_key, .. }, Some("Delete module")) => {
            state.delete_managed_node(&node_key);
            leave_modal(state);
        }
        // TP-DOTS-10/11/12: the bucket header creates too — sub hangs under
        // the bucket itself (TP-NODE-08), parallel beside it, under the
        // bucket's own owner (none resolvable = top level).
        (ContextMenuKind::SpaceHeader { space_key, .. }, Some("New sub-module...")) => {
            open_new_module_input(state, Some(space_key));
        }
        (ContextMenuKind::SpaceHeader { space_key, .. }, Some("New parallel module...")) => {
            let parent = crate::ui::space_owner_for_key(state, &space_key);
            open_new_module_input(state, parent);
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
        // TP-MOD-34: the bucket's rename walks the same road the module's
        // does, because it writes the same kind of thing — a name, keyed by
        // the rule's key, taking no part in resolution.
        (ContextMenuKind::SpaceHeader { space_key, .. }, Some("Rename module...")) => {
            open_rename_module_input(state, space_key, None);
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
        // TP-AGPANEL-28: the same road from the agents panel. The panel row
        // knows which chat it is running, so "send this agent somewhere" is
        // the same decision the chat row already parks — one ledger, two ways
        // in.
        (
            ContextMenuKind::AgentEntry {
                ws_idx,
                session_id: Some(session_id),
                ..
            },
            Some("Move to..."),
        ) => {
            let targets = state.chat_move_target_entries(Some(ws_idx));
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
        (ContextMenuKind::WorkspaceChat { session_id, .. }, Some("Rename chat...")) => {
            let current = state.chat_row_title(&session_id);
            open_chat_rename_input(state, session_id, current);
        }
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
        // TP-CHAT-MOVE-11: the module road. It opens the very same target menu
        // the branch road opens — one picker, two source lists — so a chat
        // filed into a module travels exactly the path a chat filed into a
        // branch does, down to the ledger write.
        (ContextMenuKind::WorkspaceChat { session_id, .. }, Some("Move to module...")) => {
            let targets = state.module_move_target_entries();
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
                state.remember_move_target(key);
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
        // TP-AGPANEL-04: the keyboard road to the same close, aimed at the
        // agent row's own pane.
        (
            ContextMenuKind::AgentEntry {
                ws_idx,
                tab_idx,
                pane_id,
                ..
            },
            Some("Close agent"),
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
        // TP-AGPANEL-06: the chat road on the keyboard dispatch — the session
        // is resolved now, so a menu that outlived its agent closes nothing.
        (ContextMenuKind::WorkspaceChat { session_id, .. }, Some("Close agent")) => {
            match state.find_resumed_chat_tab(&session_id) {
                Some((ws_idx, tab_idx)) => {
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
                None => leave_modal(state),
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
            Mode::RenameWorkspace if self.state.pending_chat_rename.is_some() => {
                if let Some(session_id) = self.state.pending_chat_rename.take() {
                    self.state.request_chat_rename = Some((session_id, new_name));
                }
            }
            Mode::RenameWorkspace if self.state.pending_module_dir.is_some() => {
                if let Some(node_key) = self.state.pending_module_dir.take() {
                    self.state.submit_module_dir(node_key, &new_name);
                }
            }
            Mode::RenameWorkspace if self.state.pending_new_module.is_some() => {
                if let Some(pending) = self.state.pending_new_module.take() {
                    self.state
                        .submit_module_name(pending.rename_key, pending.parent, &new_name);
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
            // TP-DOTS-13/14: the branch road on the mouse dispatch — the
            // same arm the keyboard road walks.
            (
                ContextMenuKind::NodeHeader { node_key: key, .. }
                | ContextMenuKind::SpaceHeader { space_key: key, .. },
                Some("New branch..."),
            ) => {
                start_branch_from_module(&mut self.state, key);
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
            // TP-MOD-32: the rename prompt keeps the module's own key and its
            // own parent — only the name is up for change. Resolved from the
            // live node list rather than from the menu, because the menu
            // carries no parent and guessing one would re-seat the module.
            // TP-MOD-33: on the production body too. #91 was a menu item whose
            // arm lived only in the `#[cfg(test)]` sibling — green tests, dead
            // affordance.
            // TP-MOD-35: the production twin. A verb wired into only one of the
            // two bodies works in tests and does nothing on screen.
            (ContextMenuKind::NodeHeader { node_key, .. }, Some("Set directory...")) => {
                if !self.state.open_module_dir_picker(node_key.clone()) {
                    let current = self
                        .state
                        .space_nodes
                        .iter()
                        .find(|node| node.key == node_key)
                        .and_then(|node| node.dir.as_ref())
                        .map(|dir| dir.display().to_string());
                    open_module_dir_input(&mut self.state, node_key, current);
                }
            }
            // TP-MOD-38: the production twin, for the reason the comment above
            // "Set directory..." already gives.
            (ContextMenuKind::NodeHeader { node_key, .. }, Some("Initialize git repository")) => {
                self.state.request_module_git_init = Some(node_key);
                leave_modal(&mut self.state);
            }
            (ContextMenuKind::NodeHeader { node_key, .. }, Some("Rename module...")) => {
                let parent = self
                    .state
                    .space_nodes
                    .iter()
                    .find(|node| node.key == node_key)
                    .and_then(|node| node.parent.clone());
                open_rename_module_input(&mut self.state, node_key, parent);
            }
            // TP-MOD-08: the same verb on the mouse dispatch.
            (ContextMenuKind::NodeHeader { node_key, .. }, Some("Delete module")) => {
                self.state.delete_managed_node(&node_key);
                leave_modal(&mut self.state);
            }
            // TP-DOTS-10/11/12: the bucket's creation road on the mouse
            // dispatch — the same arms the keyboard road walks.
            (ContextMenuKind::SpaceHeader { space_key, .. }, Some("New sub-module...")) => {
                open_new_module_input(&mut self.state, Some(space_key));
            }
            (ContextMenuKind::SpaceHeader { space_key, .. }, Some("New parallel module...")) => {
                let parent = crate::ui::space_owner_for_key(&self.state, &space_key);
                open_new_module_input(&mut self.state, parent);
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
            // TP-MOD-34: the production twin of the bucket rename. Menu verbs
            // have two bodies and a verb wired into only one of them is a menu
            // item that works in tests and does nothing on screen.
            (ContextMenuKind::SpaceHeader { space_key, .. }, Some("Rename module...")) => {
                open_rename_module_input(&mut self.state, space_key, None);
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
            // TP-AGPANEL-28: and on the road the mouse actually takes. #91
            // was exactly this arm missing from exactly this body; adding a
            // menu item without adding it here ships a dead affordance.
            (
                ContextMenuKind::AgentEntry {
                    ws_idx,
                    session_id: Some(session_id),
                    ..
                },
                Some("Move to..."),
            ) => {
                let targets = self.state.chat_move_target_entries(Some(ws_idx));
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
            (ContextMenuKind::WorkspaceChat { session_id, .. }, Some("Rename chat...")) => {
                let current = self.state.chat_row_title(&session_id);
                open_chat_rename_input(&mut self.state, session_id, current);
            }
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
            // TP-CHAT-MOVE-11: the same verb on the road the mouse actually
            // takes. The sibling body above is `#[cfg(test)]`; a menu answered
            // only there is a menu that works in tests and does nothing in the
            // product.
            (ContextMenuKind::WorkspaceChat { session_id, .. }, Some("Move to module...")) => {
                let targets = self.state.module_move_target_entries();
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
                    self.state.remember_move_target(key);
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
            // TP-AGPANEL-04: the agents panel closes through the pane road
            // the pane menu already owns — a graceful close with its
            // confirmation gate intact, never a kill — aimed at the row's
            // own pane rather than whatever is focused.
            (
                ContextMenuKind::AgentEntry {
                    ws_idx, pane_id, ..
                },
                Some("Close agent"),
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
            // TP-MOD-31: the blank's one verb, on the production road for the
            // same reason the daily menu below is — `parent: None`, so the
            // container is born at the top where the person can see it.
            (ContextMenuKind::SidebarBlank, Some("New module...")) => {
                open_new_module_input(&mut self.state, None);
            }
            // TP-DAILY-12: the area's own verbs. "New chat..." walks the very
            // road the "+" walks, so the two doors cannot drift apart.
            // TP-AGPANEL-45: the production twin, through the same body.
            (ContextMenuKind::ClosedAgent { agent_id }, Some(item @ ("Revive" | "Forget"))) => {
                apply_closed_agent_action(&mut self.state, agent_id, item);
                leave_modal(&mut self.state);
            }
            (ContextMenuKind::DailyHeader { .. }, Some("New chat...")) => {
                self.state.open_daily_new_chat_menu(menu_x, menu_y);
            }
            // TP-DAILY-19: the same one line as the `#[cfg(test)]` body above.
            (ContextMenuKind::DailyHeader { .. }, Some("Merge workspaces here")) => {
                self.state.request_merge_daily_workspaces = true;
                leave_modal(&mut self.state);
            }
            (ContextMenuKind::DailyHeader { .. }, Some("Collapse" | "Expand")) => {
                self.state.toggle_daily_section();
                leave_modal(&mut self.state);
            }
            // TP-DAILY-11: the daily "+" lands HERE, on the road both the
            // mouse and the keyboard actually take — the sibling body above
            // is `#[cfg(test)]`, so a menu answered only there is a menu that
            // works in tests and does nothing in the product.
            (ContextMenuKind::DailyNewChat, Some(agent)) => {
                self.state.default_chat_agent = agent.to_string();
                self.state.request_daily_chat();
                self.save_default_chat_agent(agent);
                leave_modal(&mut self.state);
            }
            // TP-AGPANEL-27: the workspace card's "+" answers on this road too.
            // It answered only in the `#[cfg(test)]` sibling body until now, so
            // the menu opened, offered its agents, and started nothing — green
            // tests over a dead affordance. Found by grepping both bodies for
            // every `ContextMenuKind`, which is the only way this class of
            // defect surfaces without a user report.
            (ContextMenuKind::WorkspaceNewChat { ws_idx, .. }, Some("New worktree")) => {
                self.state.request_new_linked_worktree = Some(ws_idx);
                leave_modal(&mut self.state);
            }
            (ContextMenuKind::WorkspaceNewChat { ws_idx, .. }, Some("Open worktree...")) => {
                self.state.request_open_existing_worktree = Some(ws_idx);
                leave_modal(&mut self.state);
            }
            (ContextMenuKind::WorkspaceNewChat { ws_idx, .. }, Some(agent)) => {
                self.state.default_chat_agent = agent.to_string();
                self.state.request_workspace_chat(ws_idx);
                self.save_default_chat_agent(agent);
                leave_modal(&mut self.state);
            }
            // TP-AGPANEL-06: the chat road resolves the session's tab NOW,
            // not at open time — a menu can outlive the agent it was opened
            // for, and a stale index would close a bystander.
            (ContextMenuKind::WorkspaceChat { session_id, .. }, Some("Close agent")) => {
                match self.state.find_resumed_chat_tab(&session_id) {
                    Some((ws_idx, tab_idx)) => {
                        self.focus_workspace_idx_via_api(ws_idx);
                        self.focus_tab_idx_via_api(tab_idx);
                        if !self.close_active_tab_via_api_requires_confirmation() {
                            leave_modal(&mut self.state);
                        }
                    }
                    None => leave_modal(&mut self.state),
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

    // TP-DAILY-11: the daily "+" is answered on the PRODUCTION road, the one
    // both the mouse (`MouseAction::ContextMenu`) and the keyboard take. The
    // sibling body in this file is `#[cfg(test)]`, so a menu wired only there
    // passes its unit tests while doing nothing at all in the product — this
    // test exists because that is exactly what happened while writing it.
    // TP-AGPANEL-27 (#91): the workspace card's "+" lands on the road the
    // mouse and keyboard actually take. This arm lived only in the
    // `#[cfg(test)]` sibling body, so picking an agent there passed every test
    // and started nothing in the product — the same shape as TP-DAILY-11's
    // note, found by measuring rather than by a report.
    #[test]
    fn the_workspace_plus_menu_starts_a_chat_on_the_production_road() {
        let mut app = app_with_test_workspaces(&["main"]);
        let checkout = std::path::PathBuf::from("/repo/checkout");
        app.state.workspaces[0].identity_cwd = checkout.clone();
        let menu = ContextMenuState {
            kind: ContextMenuKind::WorkspaceNewChat {
                ws_idx: 0,
                offers_worktree: false,
            },
            x: 0,
            y: 0,
            list: crate::app::state::MenuListState::new(0),
        };
        let agent = *crate::app::projects::CHAT_AGENTS.first().expect("an agent");
        let idx = menu
            .items()
            .iter()
            .position(|item| *item == agent)
            .expect("the agent is on the menu");

        app.apply_context_menu_action_via_api(menu, idx);

        let request = app
            .state
            .request_project_chat_tab
            .as_ref()
            .expect("the product road queues the chat");
        assert_eq!(
            request.project_path, checkout,
            "the chat starts in the workspace's own checkout (TP-WSID-02)"
        );
        assert_eq!(request.session_id, None, "a new chat resumes nothing");
        assert_eq!(
            app.state.default_chat_agent, agent,
            "the chosen agent becomes the default the next press starts on"
        );
    }

    #[test]
    fn the_daily_menu_starts_a_chat_on_the_production_road() {
        let mut app = app_with_test_workspaces(&["main"]);
        let daily = std::path::PathBuf::from("/home/tester");
        app.state.workspaces[0].identity_cwd = std::path::PathBuf::from("/repo/checkout");
        app.state.daily_chat_cwd = Some(daily.clone());
        let menu = ContextMenuState {
            kind: ContextMenuKind::DailyNewChat,
            x: 0,
            y: 0,
            list: crate::app::state::MenuListState::new(0),
        };
        let agent = *crate::app::projects::CHAT_AGENTS.first().expect("an agent");
        let idx = menu
            .items()
            .iter()
            .position(|item| *item == agent)
            .expect("the agent is on the menu");

        app.apply_context_menu_action_via_api(menu, idx);

        let request = app
            .state
            .request_project_chat_tab
            .as_ref()
            .expect("the product road queues the chat");
        assert_eq!(request.project_path, daily);
        assert_eq!(request.session_id, None);
        assert_eq!(app.state.default_chat_agent, agent);
    }

    // TP-MOD-32: a module can be renamed from its own menu, and the rename
    // keeps the module's key. Deriving a key from the new name — the way a
    // creation does — would write a second module and leave this one's
    // children and members pointing at a key nothing declares any more.
    //
    // TP-MOD-34 removed the ownership restriction this test used to assert.
    // The key rule survives it unchanged, and matters more than before: the
    // display entry IS keyed by the module's own key, so a derived one would
    // rename a module nobody is looking at.
    #[test]
    fn renaming_a_module_keeps_its_key_and_its_parent() {
        let mut app = app_with_test_workspaces(&["main"]);
        app.state.space_nodes = vec![crate::spaces::SpaceNode {
            key: "group:docs".into(),
            name: "Docs".into(),
            icon: None,
            parent: Some("project:herdr".into()),
            dir: None,
        }];

        let managed = ContextMenuState {
            kind: ContextMenuKind::NodeHeader {
                needs_git_init: false,
                node_key: "group:docs".into(),
                collapsed: false,
                deletable: true,
            },
            x: 0,
            y: 0,
            list: crate::app::state::MenuListState::new(0),
        };
        let items = managed.items();
        assert!(
            items.contains(&"Rename module..."),
            "a module the machine owns can be renamed: {items:?}"
        );
        let idx = items
            .iter()
            .position(|item| *item == "Rename module...")
            .expect("the verb is on the menu");

        app.apply_context_menu_action_via_api(managed, idx);

        let pending = app
            .state
            .pending_new_module
            .as_ref()
            .expect("the rename prompt opens");
        assert_eq!(
            pending.rename_key.as_deref(),
            Some("group:docs"),
            "the rename carries the EXISTING key; a derived one orphans the children"
        );
        assert_eq!(
            pending.parent.as_deref(),
            Some("project:herdr"),
            "and the module keeps its seat in the tree"
        );
        assert_eq!(app.state.mode, Mode::RenameWorkspace);

        // TP-MOD-34: a hand-written module walks the same road and carries the
        // same key. This half used to assert the opposite — that the verb was
        // withheld — which was true of the implementation and never of the
        // module.
        let hand_written = ContextMenuState {
            kind: ContextMenuKind::NodeHeader {
                needs_git_init: false,
                node_key: "group:docs".into(),
                collapsed: false,
                deletable: false,
            },
            x: 0,
            y: 0,
            list: crate::app::state::MenuListState::new(0),
        };
        let items = hand_written.items();
        let idx = items
            .iter()
            .position(|item| *item == "Rename module...")
            .expect("a hand-written module can be renamed too");

        app.state.pending_new_module = None;
        app.apply_context_menu_action_via_api(hand_written, idx);

        assert_eq!(
            app.state
                .pending_new_module
                .as_ref()
                .and_then(|pending| pending.rename_key.as_deref()),
            Some("group:docs"),
            "the key rule holds wherever the module was authored"
        );
    }

    // TP-DAILY-12: the area's menu folds it and starts chats — on the
    // production road, and "New chat..." hands off to the very menu the "+"
    // opens rather than duplicating its body, so the two doors cannot drift.
    #[test]
    fn the_daily_header_menu_folds_and_starts_chats() {
        let mut app = app_with_test_workspaces(&["main"]);
        app.state.daily_chat_cwd = Some(std::path::PathBuf::from("/home/tester"));

        let fold = ContextMenuState {
            kind: ContextMenuKind::DailyHeader {
                collapsed: false,
                // TP-DAILY-19: nothing to fold here, so the exact-list
                // assertions below also pin that the merge verb stays off
                // the menu when it would have no work.
                has_mergeable: false,
            },
            x: 0,
            y: 0,
            list: crate::app::state::MenuListState::new(0),
        };
        assert_eq!(fold.items(), vec!["New chat...", "Collapse"]);
        let idx = fold
            .items()
            .iter()
            .position(|item| *item == "Collapse")
            .expect("the fold verb is on the menu");
        app.apply_context_menu_action_via_api(fold, idx);
        assert!(app.state.daily_section_collapsed, "the menu folds the area");

        // Folded, the verb reads the other way round.
        let unfold = ContextMenuState {
            kind: ContextMenuKind::DailyHeader {
                collapsed: true,
                has_mergeable: false,
            },
            x: 0,
            y: 0,
            list: crate::app::state::MenuListState::new(0),
        };
        assert_eq!(unfold.items(), vec!["New chat...", "Expand"]);

        // "New chat..." opens the agent menu — the same one the "+" opens.
        let start = ContextMenuState {
            kind: ContextMenuKind::DailyHeader {
                collapsed: true,
                has_mergeable: false,
            },
            x: 4,
            y: 2,
            list: crate::app::state::MenuListState::new(0),
        };
        app.apply_context_menu_action_via_api(start, 0);
        assert!(
            matches!(
                app.state.context_menu.as_ref().map(|menu| &menu.kind),
                Some(ContextMenuKind::DailyNewChat)
            ),
            "the verb hands off to the agent menu; got {:?}",
            app.state.context_menu.as_ref().map(|menu| &menu.kind)
        );
    }

    // M1.6 / TP-CHAT-MOVE-11: the module verb appears when the tree declares
    // any module at all. Its absence when there is none is M1.5 below.
    #[test]
    fn a_chat_menu_offers_the_module_verb_when_modules_exist() {
        let menu = ContextMenuState {
            kind: ContextMenuKind::WorkspaceChat {
                ws_idx: Some(0),
                session_id: "s".to_string(),
                has_move: false,
                has_live: false,
                has_modules: true,
            },
            x: 0,
            y: 0,
            list: crate::app::state::MenuListState::new(0),
        };

        assert_eq!(
            menu.items(),
            vec!["Rename chat...", "Move to branch...", "Move to module..."],
            "the module road sits beside the branch road, named for what it does"
        );
    }

    // M1.5 / TP-CHAT-MOVE-11: with no module declared the verb would open an
    // empty picker — a button that does nothing.
    #[test]
    fn a_chat_menu_without_modules_offers_no_module_verb() {
        let menu = ContextMenuState {
            kind: ContextMenuKind::WorkspaceChat {
                ws_idx: Some(0),
                session_id: "s".to_string(),
                has_move: false,
                has_live: false,
                has_modules: false,
            },
            x: 0,
            y: 0,
            list: crate::app::state::MenuListState::new(0),
        };

        assert_eq!(menu.items(), vec!["Rename chat...", "Move to branch..."]);
    }

    // M1.7 / TP-CHAT-MOVE-11 / constraint 31: BOTH bodies answer the module
    // verb, and both open the same target picker. #91 shipped a menu entry
    // wired into only the `#[cfg(test)]` body: green tests, dead affordance.
    #[test]
    fn both_context_menu_bodies_open_the_module_picker() {
        let mut app = app_with_test_workspaces(&["main"]);
        app.state.space_nodes = vec![crate::spaces::SpaceNode {
            key: "docs".to_string(),
            name: "Docs".to_string(),
            icon: None,
            parent: None,
            dir: None,
        }];
        let menu = ContextMenuState {
            kind: ContextMenuKind::WorkspaceChat {
                ws_idx: Some(0),
                session_id: "s".to_string(),
                has_move: false,
                has_live: false,
                has_modules: true,
            },
            x: 0,
            y: 0,
            list: crate::app::state::MenuListState::new(0),
        };
        let idx = menu
            .items()
            .iter()
            .position(|item| *item == "Move to module...")
            .expect("the verb is on the menu");

        app.apply_context_menu_action_via_api(menu.clone(), idx);
        assert!(
            matches!(
                app.state.context_menu.as_ref().map(|m| &m.kind),
                Some(ContextMenuKind::ChatMoveTarget { .. })
            ),
            "the production body must open the target picker; got {:?}",
            app.state.context_menu.as_ref().map(|m| &m.kind)
        );

        app.state.context_menu = None;
        let mut terminal_runtimes = crate::terminal::TerminalRuntimeRegistry::new();
        apply_context_menu_action(&mut app.state, &mut terminal_runtimes, menu, idx);
        assert!(
            matches!(
                app.state.context_menu.as_ref().map(|m| &m.kind),
                Some(ContextMenuKind::ChatMoveTarget { .. })
            ),
            "and so must the test body"
        );
    }

    // M2.7 / TP-MOD-37: a module with no directory is told which verb fixes
    // that. The old answer named a road ("move a branch under it first") that
    // does not exist for a module the person gave a directory to instead.
    #[test]
    fn branching_a_module_with_no_directory_points_at_set_directory() {
        let mut app = app_with_test_workspaces(&["main"]);
        app.state.space_nodes = vec![crate::spaces::SpaceNode {
            key: "docs".to_string(),
            name: "Docs".to_string(),
            icon: None,
            parent: None,
            dir: None,
        }];

        start_branch_from_module(&mut app.state, "docs".to_string());

        let diagnostic = app
            .state
            .config_diagnostic
            .as_deref()
            .expect("the refusal says something");
        assert!(
            diagnostic.contains("Set directory"),
            "a refusal must name the next step; got {diagnostic:?}"
        );
    }

    // M2.8-pre / TP-MOD-38: a module whose directory is not a repository yet
    // is told about the verb that makes it one, rather than about a road that
    // does not apply.
    #[test]
    fn branching_an_uninitialised_module_points_at_the_init_verb() {
        let dir =
            std::env::temp_dir().join(format!("herdr-modgap-init-hint-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch directory");
        let mut app = app_with_test_workspaces(&["main"]);
        app.state.space_nodes = vec![crate::spaces::SpaceNode {
            key: "docs".to_string(),
            name: "Docs".to_string(),
            icon: None,
            parent: None,
            dir: Some(dir.clone()),
        }];

        start_branch_from_module(&mut app.state, "docs".to_string());

        let diagnostic = app
            .state
            .config_diagnostic
            .as_deref()
            .expect("the refusal says something");
        assert!(
            diagnostic.contains("Initialize git repository"),
            "the refusal must name the verb that fixes it; got {diagnostic:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    // TP-MOD-40: a refusal the screen cannot show is a refusal that did not
    // happen. `render_config_diagnostic` draws one line per newline and cuts
    // each to the panel width — it does not wrap — so everything past the cut
    // is gone, and the verb was written at the end of a single long line.
    #[test]
    fn a_module_branch_refusal_shows_its_verb_on_a_line_that_fits() {
        // The product proof drives an 130-column terminal and the notice is
        // drawn inside a frame narrower than that, so the actionable line has
        // to be comfortably shorter than the screen.
        const LINE_BUDGET: usize = 100;

        let dir =
            std::env::temp_dir().join(format!("herdr-modgap-diag-fit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch directory");

        let refusal = |directory: Option<std::path::PathBuf>| {
            let mut app = app_with_test_workspaces(&["main"]);
            app.state.space_nodes = vec![crate::spaces::SpaceNode {
                key: "docs".to_string(),
                name: "Docs".to_string(),
                icon: None,
                parent: None,
                dir: directory,
            }];
            start_branch_from_module(&mut app.state, "docs".to_string());
            app.state
                .config_diagnostic
                .clone()
                .expect("the refusal says something")
        };

        for (label, verb, diagnostic) in [
            (
                "a directory that is not a repository",
                "Initialize git repository",
                refusal(Some(dir.clone())),
            ),
            ("no directory at all", "Set directory...", refusal(None)),
        ] {
            let carrying = diagnostic
                .lines()
                .find(|line| line.contains(verb))
                .unwrap_or_else(|| {
                    panic!("{label}: the refusal must still name {verb:?}; got {diagnostic:?}")
                });
            assert!(
                carrying.chars().count() <= LINE_BUDGET,
                "{label}: the line carrying {verb:?} is {} characters, past the {LINE_BUDGET} \
                 the panel can draw — render_config_diagnostic truncates it and the reader \
                 never sees the verb; got {carrying:?}",
                carrying.chars().count()
            );
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    // TP-MOD-38: the menu offers the init verb only in the state it can fix.
    #[test]
    fn the_init_verb_is_offered_only_when_it_has_work() {
        let offered = ContextMenuState {
            kind: ContextMenuKind::NodeHeader {
                node_key: "docs".to_string(),
                collapsed: false,
                deletable: false,
                needs_git_init: true,
            },
            x: 0,
            y: 0,
            list: crate::app::state::MenuListState::new(0),
        };
        assert!(offered.items().contains(&"Initialize git repository"));

        let withheld = ContextMenuState {
            kind: ContextMenuKind::NodeHeader {
                node_key: "docs".to_string(),
                collapsed: false,
                deletable: false,
                needs_git_init: false,
            },
            x: 0,
            y: 0,
            list: crate::app::state::MenuListState::new(0),
        };
        assert!(!withheld.items().contains(&"Initialize git repository"));
    }

    // TP-MOD-38 / constraint 31: both bodies arm the init request.
    #[test]
    fn both_context_menu_bodies_request_the_git_init() {
        let mut app = app_with_test_workspaces(&["main"]);
        let menu = ContextMenuState {
            kind: ContextMenuKind::NodeHeader {
                node_key: "docs".to_string(),
                collapsed: false,
                deletable: false,
                needs_git_init: true,
            },
            x: 0,
            y: 0,
            list: crate::app::state::MenuListState::new(0),
        };
        let idx = menu
            .items()
            .iter()
            .position(|item| *item == "Initialize git repository")
            .expect("the verb is on the menu");

        app.apply_context_menu_action_via_api(menu.clone(), idx);
        assert_eq!(
            app.state.request_module_git_init.as_deref(),
            Some("docs"),
            "the production body must arm the request"
        );

        app.state.request_module_git_init = None;
        let mut terminal_runtimes = crate::terminal::TerminalRuntimeRegistry::new();
        apply_context_menu_action(&mut app.state, &mut terminal_runtimes, menu, idx);
        assert_eq!(
            app.state.request_module_git_init.as_deref(),
            Some("docs"),
            "and so must the test body"
        );
    }

    // P2.1 / TP-DAILY-19: the verb appears exactly when there is something to
    // fold. Its absence is pinned by the exact-list assertions in
    // `the_daily_header_menu_folds_and_starts_chats`, which build the same menu
    // with `has_mergeable: false`.
    #[test]
    fn the_daily_header_offers_the_merge_only_when_there_is_something_to_fold() {
        let offered = ContextMenuState {
            kind: ContextMenuKind::DailyHeader {
                collapsed: false,
                has_mergeable: true,
            },
            x: 0,
            y: 0,
            list: crate::app::state::MenuListState::new(0),
        };

        assert_eq!(
            offered.items(),
            vec!["New chat...", "Merge workspaces here", "Collapse"],
            "the verb sits between starting something new and folding the area"
        );
    }

    // P2.7 / TP-DAILY-19 / constraint 31: BOTH bodies answer the verb, and they
    // answer it with the same single line. #91 shipped a menu entry that worked
    // in every test and did nothing in the product because only the
    // `#[cfg(test)]` body had been taught it; this test is the guard against
    // that class, and the reason the verb sets a flag instead of doing the work
    // in place — one line cannot drift from itself.
    #[test]
    fn both_context_menu_bodies_request_the_daily_merge() {
        let mut app = app_with_test_workspaces(&["main"]);
        let menu = ContextMenuState {
            kind: ContextMenuKind::DailyHeader {
                collapsed: false,
                has_mergeable: true,
            },
            x: 0,
            y: 0,
            list: crate::app::state::MenuListState::new(0),
        };
        let idx = menu
            .items()
            .iter()
            .position(|item| *item == "Merge workspaces here")
            .expect("the verb is on the menu when there is something to fold");

        // The production road.
        app.apply_context_menu_action_via_api(menu.clone(), idx);
        assert!(
            app.state.request_merge_daily_workspaces,
            "the production body must request the merge"
        );

        // The `#[cfg(test)]` sibling, from the same menu.
        app.state.request_merge_daily_workspaces = false;
        let mut terminal_runtimes = crate::terminal::TerminalRuntimeRegistry::new();
        apply_context_menu_action(&mut app.state, &mut terminal_runtimes, menu, idx);
        assert!(
            app.state.request_merge_daily_workspaces,
            "the test body must request the very same thing"
        );
    }

    // G2 / TP-AGPANEL-45: two verbs, and the one that takes something away
    // comes last — the order TP-MOD-08/26 already keeps for "Delete module".
    #[test]
    fn a_graveyard_row_offers_revive_then_forget() {
        let menu = ContextMenuState {
            kind: ContextMenuKind::ClosedAgent {
                agent_id: "ghost-1".to_string(),
            },
            x: 0,
            y: 0,
            list: crate::app::state::MenuListState::new(0),
        };

        assert_eq!(menu.items(), vec!["Revive", "Forget"]);
    }

    // G3 + G6 / TP-AGPANEL-45 / constraint 31: both bodies answer both verbs,
    // and "Revive" parks the very request the left-click road raises. A verb
    // wired into one body works in tests and does nothing on screen.
    #[test]
    fn both_context_menu_bodies_answer_the_graveyard_verbs() {
        let mut app = app_with_test_workspaces(&["main"]);
        let menu = ContextMenuState {
            kind: ContextMenuKind::ClosedAgent {
                agent_id: "ghost-1".to_string(),
            },
            x: 0,
            y: 0,
            list: crate::app::state::MenuListState::new(0),
        };
        let revive = menu
            .items()
            .iter()
            .position(|item| *item == "Revive")
            .expect("revive is on the menu");

        app.apply_context_menu_action_via_api(menu.clone(), revive);
        assert_eq!(
            app.state.request_revive_closed_agent.as_deref(),
            Some("ghost-1"),
            "the production body must raise the revival request"
        );

        app.state.request_revive_closed_agent = None;
        let mut terminal_runtimes = crate::terminal::TerminalRuntimeRegistry::new();
        apply_context_menu_action(&mut app.state, &mut terminal_runtimes, menu, revive);
        assert_eq!(
            app.state.request_revive_closed_agent.as_deref(),
            Some("ghost-1"),
            "and so must the test body"
        );
    }

    // G4 / TP-AGPANEL-45: "Forget" reaches the ledger through the menu, on the
    // production road.
    #[test]
    fn forgetting_from_the_menu_removes_the_record() {
        let mut app = app_with_test_workspaces(&["main"]);
        app.state
            .closed_agents
            .record_closed(crate::app::closed_agents::ClosedAgentRecord {
                agent_id: "ghost-1".to_string(),
                label: "a ghost".to_string(),
                cwd: Some(std::path::PathBuf::from("/tmp")),
                workspace_key: None,
                session: None,
                closed_at: 1,
                revival: crate::app::closed_agents::RevivalState::Dormant,
            });
        assert_eq!(app.state.closed_agents.entries().count(), 1);

        let menu = ContextMenuState {
            kind: ContextMenuKind::ClosedAgent {
                agent_id: "ghost-1".to_string(),
            },
            x: 0,
            y: 0,
            list: crate::app::state::MenuListState::new(0),
        };
        let forget = menu
            .items()
            .iter()
            .position(|item| *item == "Forget")
            .expect("forget is on the menu");

        app.apply_context_menu_action_via_api(menu, forget);

        assert_eq!(
            app.state.closed_agents.entries().count(),
            0,
            "the headstone is gone from the graveyard"
        );
    }

    // TP-MOD-31: the blank's verb opens the name prompt with NO parent, so
    // the container is born at the top level where the person who asked for
    // it can see it — a parent leaking in here would bury it inside whatever
    // happened to be nearby. Walked on the production road for the reason
    // TP-DAILY-11 records. The write itself (managed overlay only, never the
    // hand-written config) is `upsert_managed_node`'s own contract, and is
    // deliberately not re-exercised here: calling it would write to this
    // machine's real `spaces.managed.toml`.
    #[test]
    fn the_blank_menu_opens_a_top_level_module_prompt() {
        let mut app = app_with_test_workspaces(&["main"]);
        let menu = ContextMenuState {
            kind: ContextMenuKind::SidebarBlank,
            x: 3,
            y: 9,
            list: crate::app::state::MenuListState::new(0),
        };
        assert_eq!(
            menu.items(),
            vec!["New module..."],
            "no 'New project...': the blank has no repository to claim, and an \
             entry that cannot be honoured does not belong on a menu"
        );
        let idx = menu
            .items()
            .iter()
            .position(|item| *item == "New module...")
            .expect("the verb is on the menu");

        app.apply_context_menu_action_via_api(menu, idx);

        let pending = app
            .state
            .pending_new_module
            .as_ref()
            .expect("the production road opens the name prompt");
        assert_eq!(
            pending.parent, None,
            "the blank makes a TOP-LEVEL container; a parent here hides it"
        );
        assert_eq!(app.state.mode, Mode::RenameWorkspace);
        assert!(
            app.state.context_menu.is_none(),
            "the menu closes behind the prompt"
        );
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
                dir: None,
            },
            crate::spaces::SpaceNode {
                key: "group:ops".into(),
                name: "Ops".into(),
                icon: None,
                parent: Some("group:ui".into()),
                dir: None,
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
                has_modules: false,
                ws_idx: Some(0),
                session_id: "s1".into(),
                has_move: false,
                has_live: false,
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
                has_modules: false,
                ws_idx: Some(0),
                session_id: "s1".into(),
                has_move: true,
                has_live: false,
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
                needs_git_init: false,
                node_key: "group:docs".into(),
                collapsed,
                deletable: false,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };
        assert_eq!(
            node_menu(false).items(),
            vec![
                "New branch...",
                "New sub-module...",
                "New parallel module...",
                "Collapse",
                "Rename module...",
                "Set directory...",
            ]
        );
        assert_eq!(
            node_menu(true).items(),
            vec![
                "New branch...",
                "New sub-module...",
                "New parallel module...",
                "Expand",
                "Rename module...",
                "Set directory...",
            ]
        );

        // TP-DOTS-10: the bucket header is a module to the person using it —
        // it creates exactly like the node header does.
        let space_menu = ContextMenuState {
            kind: ContextMenuKind::SpaceHeader {
                space_key: "repo-key".into(),
                collapsed: false,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };
        assert_eq!(
            space_menu.items(),
            vec![
                "New branch...",
                "New sub-module...",
                "New parallel module...",
                "Collapse",
                "Rename module...",
            ]
        );
    }

    // TP-DOTS-13: every module header leads with the branch road — the
    // point of a module is the branches inside it.
    fn a_node_menu(key: &str, deletable: bool) -> ContextMenuState {
        ContextMenuState {
            kind: ContextMenuKind::NodeHeader {
                needs_git_init: false,
                node_key: key.into(),
                collapsed: false,
                deletable,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        }
    }

    // TP-MOD-08: creating a module is two clicks; before this, taking one back
    // was a hand-edit of a file the person never opens. The verb comes last —
    // it is the one item here that removes something.
    #[test]
    fn a_managed_module_offers_to_be_deleted() {
        assert_eq!(
            a_node_menu("group:docs", true).items(),
            vec![
                "New branch...",
                "New sub-module...",
                "New parallel module...",
                "Collapse",
                // TP-MOD-34: renaming used to ride with the delete verb,
                // because both rewrote the file the machine owns. It no longer
                // rewrites anything, so it no longer asks who owns what — but
                // its position is unchanged, and this list is what says so.
                "Rename module...",
                "Set directory...",
                "Delete module",
            ]
        );
    }

    // TP-MOD-26: a module written by hand into config.toml looks the same on
    // screen and cannot be taken back by machine. Offering the verb there
    // would be a button that quietly does nothing — the shape of the promise
    // #64 was opened for.
    #[test]
    fn a_hand_written_module_does_not_offer_to_be_deleted() {
        assert!(!a_node_menu("group:docs", false)
            .items()
            .contains(&"Delete module"));
    }

    // TP-MOD-28: splitting the shared arm must not change what a bucket
    // header offers. This is the anchor that says the bucket kept its menu
    // while the module grew one.
    #[test]
    fn a_bucket_header_menu_is_unchanged_by_the_module_delete_verb() {
        let space_menu = ContextMenuState {
            kind: ContextMenuKind::SpaceHeader {
                space_key: "repo-key".into(),
                collapsed: true,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };
        assert_eq!(
            space_menu.items(),
            vec![
                "New branch...",
                "New sub-module...",
                "New parallel module...",
                "Expand",
                "Rename module...",
            ],
            "a bucket is taken back by its own verb, not this one"
        );
    }

    #[test]
    fn module_menus_lead_with_new_branch() {
        let node_menu = ContextMenuState {
            kind: ContextMenuKind::NodeHeader {
                needs_git_init: false,
                node_key: "group:docs".into(),
                collapsed: false,
                deletable: false,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };
        // "Set directory..." trails the shape verbs and precedes nothing:
        // it changes where a module stands, not what the tree looks like, so
        // it sits after the verbs that build the tree (TP-MOD-33).
        assert_eq!(
            node_menu.items(),
            vec![
                "New branch...",
                "New sub-module...",
                "New parallel module...",
                "Collapse",
                "Rename module...",
                "Set directory...",
            ]
        );
        let space_menu = ContextMenuState {
            kind: ContextMenuKind::SpaceHeader {
                space_key: "repo-key".into(),
                collapsed: true,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };
        assert_eq!(
            space_menu.items(),
            vec![
                "New branch...",
                "New sub-module...",
                "New parallel module...",
                "Expand",
                "Rename module...",
            ]
        );
    }

    // T4.1 / TP-MOD-35: the verb opens the file picker, seeded at the
    // directory the module already points at. The text box it replaces
    // prefilled the same value for the same reason: the common edit is a
    // correction, and retyping a path by hand is where typos come from.
    #[test]
    fn setting_a_module_directory_opens_the_picker_where_the_module_stands() {
        let mut app = app_with_test_workspaces(&["main"]);
        let dir = std::env::temp_dir();
        app.state.space_nodes = vec![crate::spaces::SpaceNode {
            key: "group:docs".into(),
            name: "Docs".into(),
            icon: None,
            parent: None,
            dir: Some(dir.clone()),
        }];

        let menu = ContextMenuState {
            kind: ContextMenuKind::NodeHeader {
                needs_git_init: false,
                node_key: "group:docs".into(),
                collapsed: false,
                deletable: true,
            },
            x: 0,
            y: 0,
            list: crate::app::state::MenuListState::new(0),
        };
        let idx = menu
            .items()
            .iter()
            .position(|item| *item == "Set directory...")
            .expect("the verb is on the menu");

        app.apply_context_menu_action_via_api(menu, idx);

        let picker = app
            .state
            .agent_attachment_picker
            .as_ref()
            .expect("the picker opens");
        assert_eq!(
            picker.module_key(),
            Some("group:docs"),
            "the picker knows which module it is choosing for"
        );
        assert_eq!(
            picker.file_manager.cwd,
            crate::worktree::canonical_or_original(&dir),
            "and it starts where the module already stands"
        );
        assert_eq!(app.state.mode, Mode::AttachFile);
    }

    // T4.6 / TP-MOD-35: cancel is cancel. The module keeps the directory it
    // had and nothing is written — an overlay write on the way out would make
    // the escape hatch a decision.
    #[test]
    fn cancelling_the_directory_picker_writes_nothing() {
        let mut app = app_with_test_workspaces(&["main"]);
        app.state.space_nodes = vec![crate::spaces::SpaceNode {
            key: "group:docs".into(),
            name: "Docs".into(),
            icon: None,
            parent: None,
            dir: Some(std::env::temp_dir()),
        }];
        assert!(app.state.open_module_dir_picker("group:docs".into()));

        app.state.close_agent_attachment_picker();

        assert!(app.state.agent_attachment_picker.is_none());
        assert_ne!(app.state.mode, Mode::AttachFile);
        assert_eq!(
            app.state.space_nodes[0].dir.as_deref(),
            Some(std::env::temp_dir().as_path()),
            "the module keeps the place it stood in"
        );
    }

    // T4.9 / TP-MOD-35: the target is a directory. A file under the cursor
    // does not disqualify the choice — the person is standing in a directory
    // and pressing Set means "this one". Refusing because the cursor happens
    // to rest on a README would be a button that looks pressable and is not.
    #[test]
    fn the_picker_chooses_a_directory_even_when_the_cursor_rests_on_a_file() {
        let root = std::env::temp_dir().join("herdr-dir-picker-fixture");
        let _ = std::fs::create_dir_all(root.join("inner"));
        let _ = std::fs::write(root.join("a-file.txt"), b"x");

        let mut app = app_with_test_workspaces(&["main"]);
        app.state.space_nodes = vec![crate::spaces::SpaceNode {
            key: "group:docs".into(),
            name: "Docs".into(),
            icon: None,
            parent: None,
            dir: Some(root.clone()),
        }];
        assert!(app.state.open_module_dir_picker("group:docs".into()));

        let picker = app
            .state
            .agent_attachment_picker
            .as_mut()
            .expect("the picker opens");
        let file_idx = picker
            .file_manager
            .entries
            .iter()
            .position(|entry| !entry.is_dir())
            .expect("the fixture has a file");
        picker.file_manager.select(file_idx);
        let chosen = app
            .state
            .agent_attachment_picker
            .as_ref()
            .map(|picker| picker.chosen_directory())
            .expect("a choice");

        assert!(
            chosen.is_dir(),
            "a directory is what a module can stand in: {chosen:?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    // T3.3 / TP-CHAT-NAME-01: a chat row had no rename at all. The name it
    // wore came from the transcript, and for a chat started outside a
    // workspace's directory there is no title to derive — which is how a row
    // came back reading `user` with nothing to say what it was spawned for.
    #[test]
    fn a_chat_row_offers_to_be_renamed() {
        let chat_menu = |ws_idx: Option<usize>| ContextMenuState {
            kind: ContextMenuKind::WorkspaceChat {
                has_modules: false,
                ws_idx,
                session_id: "s1".into(),
                has_move: false,
                has_live: false,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };

        assert_eq!(
            chat_menu(Some(0)).items().first(),
            Some(&"Rename chat..."),
            "naming comes first — it is what makes the row addressable at all"
        );
        assert!(
            chat_menu(None).items().contains(&"Rename chat..."),
            "a daily chat belongs to no workspace and needs the name most"
        );
    }

    // T3.1 / TP-MOD-34: the bucket header had no rename at all. Every module
    // on the reported machine was a hand-written bucket, so this one absence
    // was the whole of "modüllerde rename göremiyorum".
    #[test]
    fn a_bucket_header_offers_to_be_renamed() {
        let space_menu = ContextMenuState {
            kind: ContextMenuKind::SpaceHeader {
                space_key: "herdr-web:tabs".into(),
                collapsed: false,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };
        assert!(
            space_menu.items().contains(&"Rename module..."),
            "to the person using the tree this header IS a module"
        );
    }

    // T3.2 / TP-MOD-34: a hand-written module offers the rename too. The old
    // restriction was a property of the implementation, not of the module.
    #[test]
    fn a_hand_written_module_offers_to_be_renamed() {
        assert!(
            a_node_menu("group:docs", false)
                .items()
                .contains(&"Rename module..."),
            "a display entry is not a rule, so there is nothing to lose at first-match"
        );
    }

    // TP-MOD-34: the delete verb keeps its old restriction. Renaming stopped
    // rewriting the machine's file; deleting never did anything else, so the
    // two verbs part company here and this is the test that says so.
    #[test]
    fn a_hand_written_module_still_does_not_offer_to_be_deleted() {
        let menu = a_node_menu("group:docs", false);
        let items = menu.items();
        assert!(items.contains(&"Rename module..."));
        assert!(!items.contains(&"Delete module"));
    }

    // TP-DOTS-14: "New branch..." resolves a source workspace under the
    // module and opens the proven worktree dialog with the module pending.
    #[test]
    fn a_new_branch_pick_opens_the_worktree_dialog_with_the_module_pending() {
        let mut app = app_with_movable_branch();
        let key = crate::ui::effective_space(&app.state, 0)
            .expect("the fixture branch has a space")
            .key;
        let menu = ContextMenuState {
            kind: ContextMenuKind::SpaceHeader {
                space_key: key.clone(),
                collapsed: false,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };
        let idx = item_index(&menu, "New branch...");

        app.apply_context_menu_action_via_api(menu, idx);

        assert_eq!(
            app.state.pending_branch_module,
            Some(key),
            "the module rides along to the submit"
        );
        assert_eq!(
            app.state.request_new_linked_worktree,
            Some(0),
            "the proven dialog road is reused, not reinvented"
        );
        assert!(app.state.context_menu.is_none(), "the menu chain is done");
    }

    // TP-DOTS-11: a bucket's "New sub-module..." arms with the bucket itself
    // as the parent — modules hang under buckets like they hang under nodes.
    #[test]
    fn a_bucket_sub_module_arms_with_the_bucket_itself() {
        let mut app = app_with_movable_branch();
        let menu = ContextMenuState {
            kind: ContextMenuKind::SpaceHeader {
                space_key: "herdr:tiling".into(),
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
                parent: Some("herdr:tiling".into()),
                rename_key: None,
            })
        );
    }

    // TP-DOTS-12: a bucket's "New parallel module..." arms with the bucket's
    // own owner — and with no resolvable owner the sibling is top level.
    #[test]
    fn a_bucket_parallel_module_arms_with_the_buckets_owner() {
        let mut app = app_with_movable_branch();
        let menu = ContextMenuState {
            kind: ContextMenuKind::SpaceHeader {
                space_key: "herdr:tiling".into(),
                collapsed: false,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };
        let idx = item_index(&menu, "New parallel module...");

        app.apply_context_menu_action_via_api(menu, idx);

        assert_eq!(
            app.state.pending_new_module,
            Some(crate::app::state::PendingNewModule {
                parent: None,
                rename_key: None,
            }),
            "no resolvable owner makes a top-level sibling"
        );
    }

    // TP-DOTS-05: "New sub-module..." closes the menu chain and arms the
    // rename input with the header itself as the parent.
    #[test]
    fn a_new_sub_module_pick_opens_the_name_input_with_the_header_as_parent() {
        let mut app = app_with_movable_branch();
        let menu = ContextMenuState {
            kind: ContextMenuKind::NodeHeader {
                needs_git_init: false,
                node_key: "group:ui".into(),
                collapsed: false,
                deletable: false,
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
                rename_key: None,
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
                dir: None,
            },
            crate::spaces::SpaceNode {
                key: "group:ui".into(),
                name: "UI".into(),
                icon: None,
                parent: Some("group:ops".into()),
                dir: None,
            },
        ];
        let menu_for = |key: &str| ContextMenuState {
            kind: ContextMenuKind::NodeHeader {
                needs_git_init: false,
                node_key: key.into(),
                collapsed: false,
                deletable: false,
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
                rename_key: None,
            }),
            "a nested header's sibling shares its parent"
        );

        app.state.pending_new_module = None;
        let menu = menu_for("group:ops");
        let idx = item_index(&menu, "New parallel module...");
        app.apply_context_menu_action_via_api(menu, idx);
        assert_eq!(
            app.state.pending_new_module,
            Some(crate::app::state::PendingNewModule {
                parent: None,
                rename_key: None,
            }),
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
            rename_key: None,
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
                needs_git_init: false,
                node_key: "group:ui".into(),
                collapsed,
                deletable: false,
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

    // TP-AGPANEL-03/28: an agents-panel row whose chat the ledger cannot name
    // owns exactly one verb. The panel is a list of running agents, so the
    // question it answers that no other surface answers is "close this one",
    // and a longer menu here would duplicate the pane menu the row's own pane
    // already has. The move verb is the one exception, and it appears only
    // when there is an identity to move (see the sibling test below) — an
    // offer that cannot be honoured is worse than no offer.
    fn node_menu(node_key: &str, deletable: bool) -> ContextMenuState {
        ContextMenuState {
            kind: ContextMenuKind::NodeHeader {
                needs_git_init: false,
                node_key: node_key.into(),
                collapsed: false,
                deletable,
            },
            x: 0,
            y: 0,
            list: crate::app::state::MenuListState::new(0),
        }
    }

    fn app_with_module(key: &str, dir: Option<&str>) -> crate::app::App {
        let mut app = app_with_test_workspaces(&["main"]);
        app.state.space_nodes = vec![crate::spaces::SpaceNode {
            key: key.into(),
            name: "Docs Render".into(),
            icon: None,
            parent: None,
            dir: dir.map(std::path::PathBuf::from),
        }];
        app
    }

    // TP-MOD-33 (D2): the verb the user went looking for and did not find.
    // Offered on every module, hand-written or machine-owned: unlike a rename,
    // a directory is a new field on the same key rather than a value that
    // loses to a hand-written rule at first-match.
    #[test]
    fn a_module_menu_offers_to_set_its_directory() {
        for deletable in [true, false] {
            let menu = node_menu("docs", deletable);
            let items = menu.items();
            assert!(
                items.contains(&"Set directory..."),
                "deletable={deletable}: {items:?}"
            );
        }
    }

    // TP-MOD-33 (D3): on the PRODUCTION body. #91 was a menu item whose arm
    // lived only in the `#[cfg(test)]` sibling — every test green, the
    // affordance dead in the product. This asserts the road the mouse and the
    // keyboard actually take.
    #[test]
    fn setting_a_module_directory_opens_the_input_on_the_production_road() {
        let mut app = app_with_module("docs", None);
        let menu = node_menu("docs", true);
        let idx = menu
            .items()
            .iter()
            .position(|item| *item == "Set directory...")
            .expect("the verb is on the menu");

        app.apply_context_menu_action_via_api(menu, idx);

        // TP-MOD-35 changed what the verb opens — a file picker rather than a
        // text box — and left the thing this test is about untouched: that the
        // PRODUCTION arm exists at all. Either road proves it.
        let opened_for = app
            .state
            .agent_attachment_picker
            .as_ref()
            .and_then(|picker| picker.module_key())
            .map(str::to_string)
            .or_else(|| app.state.pending_module_dir.clone());
        assert_eq!(
            opened_for.as_deref(),
            Some("docs"),
            "the production road opens the directory chooser for this module"
        );
        assert!(
            app.state.pending_new_module.is_none(),
            "and not the name input — the two verbs share an overlay, not a meaning"
        );
    }

    // TP-MOD-33: an existing directory is prefilled, because the common edit
    // is a correction and retyping a path by hand is where typos come from.
    #[test]
    fn the_directory_input_starts_from_what_the_module_already_points_at() {
        let mut app = app_with_module("docs", Some("/tmp"));
        let menu = node_menu("docs", true);
        let idx = menu
            .items()
            .iter()
            .position(|item| *item == "Set directory...")
            .expect("the verb is on the menu");

        app.apply_context_menu_action_via_api(menu, idx);

        // TP-MOD-35: the prefill moved from a text box to the picker's own
        // starting directory. Same contract — the chooser opens where the
        // answer already is — expressed in whichever surface is on screen.
        match app.state.agent_attachment_picker.as_ref() {
            Some(picker) => assert_eq!(
                picker.file_manager.cwd,
                crate::worktree::canonical_or_original(std::path::Path::new("/tmp"))
            ),
            None => {
                assert_eq!(app.state.name_input, "/tmp");
                assert!(app.state.name_input_replace_on_type);
            }
        }
    }

    // TP-MOD-33 (D7): a directory that does not exist is refused rather than
    // written. A module pointing at a missing path is worse than one pointing
    // nowhere — the chat filed into it would open a pane the shell cannot
    // enter, and the person would read that as the move having failed.
    #[test]
    fn a_directory_that_does_not_exist_is_refused() {
        let mut app = app_with_module("docs", None);
        app.state
            .submit_module_dir("docs".into(), "/definitely/not/a/real/path");

        assert!(
            app.state.space_nodes.iter().all(|node| node.dir.is_none()),
            "nothing was written for a path that cannot be opened"
        );
    }

    #[test]
    fn the_agent_row_menu_offers_exactly_the_close_verb() {
        let workspace = Workspace::test_new("one");
        let pane_id = workspace.tabs[0].root_pane;
        let menu = ContextMenuState {
            kind: ContextMenuKind::AgentEntry {
                ws_idx: 0,
                tab_idx: 0,
                pane_id,
                session_id: None,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };

        assert_eq!(menu.items(), vec!["Close agent"]);
    }

    // TP-CHAT-MOVE-08 (P4): filing a daily chat offers EVERY workspace. The
    // picker excludes the drawer a chat is shown in, and a daily chat is shown
    // in none — excluding an arbitrary one would quietly drop a legitimate
    // destination. Asserted on the production body, the road the press takes.
    #[test]
    fn filing_a_daily_chat_offers_every_workspace_on_the_production_road() {
        let mut app = app_with_test_workspaces(&["main", "other"]);
        let all = app.state.chat_move_target_entries(None).len();
        let from_a_drawer = app.state.chat_move_target_entries(Some(0)).len();
        assert!(
            all > from_a_drawer,
            "a chat with no drawer excludes nothing; got {all} vs {from_a_drawer}"
        );

        let menu = ContextMenuState {
            kind: ContextMenuKind::WorkspaceChat {
                has_modules: false,
                ws_idx: None,
                session_id: "daily-a".to_string(),
                has_move: false,
                has_live: false,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };
        let idx = menu
            .items()
            .iter()
            .position(|item| *item == "Move to branch...")
            .expect("the move verb is on a daily chat's menu");

        app.apply_context_menu_action_via_api(menu, idx);

        match app.state.context_menu.as_ref().map(|m| &m.kind) {
            Some(ContextMenuKind::ChatMoveTarget {
                session_id,
                targets,
            }) => {
                assert_eq!(session_id, "daily-a");
                assert_eq!(
                    targets.len(),
                    all,
                    "the picker offers every destination, none excluded"
                );
            }
            other => panic!("the production road opens the picker; got {other:?}"),
        }
    }

    // TP-AGPANEL-28 (N1): a row whose chat the ledger knows can send it
    // somewhere. This is the user's own sentence — "I should be able to send
    // the agent I want to the module area I want" — answered on the surface
    // where the running agents actually are.
    #[test]
    fn an_agent_row_with_a_known_chat_offers_to_move_it() {
        let workspace = Workspace::test_new("one");
        let pane_id = workspace.tabs[0].root_pane;
        let menu = ContextMenuState {
            kind: ContextMenuKind::AgentEntry {
                ws_idx: 0,
                tab_idx: 0,
                pane_id,
                session_id: Some("sess-7".to_string()),
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };

        assert_eq!(menu.items(), vec!["Move to...", "Close agent"]);
    }

    // TP-AGPANEL-28 (N3 + N4): the verb opens the target picker carrying THIS
    // row's session — and it does so on the production road, the one the mouse
    // and keyboard take. #91 was this exact arm missing from this exact body.
    #[test]
    fn moving_from_an_agent_row_opens_the_picker_on_the_production_road() {
        let mut app = app_with_test_workspaces(&["main", "other"]);
        let pane_id = app.state.workspaces[0].tabs[0].root_pane;
        let menu = ContextMenuState {
            kind: ContextMenuKind::AgentEntry {
                ws_idx: 0,
                tab_idx: 0,
                pane_id,
                session_id: Some("sess-7".to_string()),
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };
        let idx = menu
            .items()
            .iter()
            .position(|item| *item == "Move to...")
            .expect("the move verb is on the menu");

        app.apply_context_menu_action_via_api(menu, idx);

        match app.state.context_menu.as_ref().map(|m| &m.kind) {
            Some(ContextMenuKind::ChatMoveTarget {
                session_id,
                targets,
            }) => {
                assert_eq!(
                    session_id, "sess-7",
                    "the picker carries the row's own chat, resolved when the menu opened"
                );
                assert!(
                    !targets.is_empty(),
                    "the picker offers somewhere to go; got {targets:?}"
                );
            }
            other => panic!("the production road opens the target picker; got {other:?}"),
        }
    }

    // TP-AGPANEL-04: closing from an agent row closes THAT row's pane, not
    // whichever pane happens to be focused — the row is the target, and it
    // travels in the menu. It closes through the same proper close road the
    // pane menu uses (a graceful close, never a kill).
    #[test]
    fn closing_an_agent_row_closes_that_row_pane_not_the_focused_one() {
        let mut app = app_with_test_workspaces(&["one"]);
        let root = app.state.workspaces[0].tabs[0].root_pane;
        let second = app.state.workspaces[0].test_split(ratatui::layout::Direction::Horizontal);
        app.state.ensure_test_terminals();
        app.state.workspaces[0].tabs[0].layout.focus_pane(root);
        app.state.mode = Mode::ContextMenu;

        let menu = ContextMenuState {
            kind: ContextMenuKind::AgentEntry {
                ws_idx: 0,
                tab_idx: 0,
                pane_id: second,
                session_id: None,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };
        let idx = item_index(&menu, "Close agent");

        app.apply_context_menu_action_via_api(menu, idx);

        assert_eq!(
            app.state.workspaces[0].tabs[0].layout.pane_count(),
            1,
            "the row's own pane is the one that closed"
        );
        assert_eq!(
            app.state.workspaces[0].focused_pane_id(),
            Some(root),
            "the pane that was focused survives — it was not the target"
        );
    }

    // TP-AGPANEL-05: a chat row offers the close verb only while the chat has
    // a running tab behind it. A drawer lists finished chats too, and a
    // "Close agent" on a transcript with nothing running would be a button
    // that does nothing — the same rule "Move back" already follows.
    #[test]
    fn a_chat_row_offers_close_only_while_something_is_running() {
        let live = ContextMenuState {
            kind: ContextMenuKind::WorkspaceChat {
                has_modules: false,
                ws_idx: Some(0),
                session_id: "s1".into(),
                has_move: false,
                has_live: true,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };
        assert_eq!(
            live.items(),
            // TP-CHAT-NAME-01 put naming at the head of this list; the close
            // verb's own rule — last, and only while something is running —
            // is what this test is about and is unchanged.
            vec!["Rename chat...", "Move to branch...", "Close agent"]
        );

        let finished = ContextMenuState {
            kind: ContextMenuKind::WorkspaceChat {
                has_modules: false,
                ws_idx: Some(0),
                session_id: "s1".into(),
                has_move: true,
                has_live: false,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };
        assert_eq!(
            finished.items(),
            vec!["Rename chat...", "Move to branch...", "Move back"],
            "a finished chat is never offered a close it cannot perform"
        );
    }

    // TP-AGPANEL-06: the chat road re-resolves the session's tab when the
    // item is picked. A menu can stay open while the agent exits, and firing
    // a close at the tab index captured at open time would close whatever
    // moved into that slot; a session that is gone closes nothing at all.
    #[test]
    fn closing_a_chat_agent_targets_the_session_tab_and_a_stale_menu_is_inert() {
        let mut app = app_with_test_workspaces(&["one"]);
        let logs = app.state.workspaces[0].test_add_tab(Some("logs"));
        app.state.workspaces[0].tabs[logs].resumed_session_id = Some("s1".into());
        app.state.ensure_test_terminals();
        app.state.workspaces[0].switch_tab(0);
        app.state.mode = Mode::ContextMenu;

        let menu = |session: &str| ContextMenuState {
            kind: ContextMenuKind::WorkspaceChat {
                has_modules: false,
                ws_idx: Some(0),
                session_id: session.into(),
                has_move: false,
                has_live: true,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };
        let idx = item_index(&menu("s1"), "Close agent");

        app.apply_context_menu_action_via_api(menu("s1"), idx);

        assert_eq!(
            app.state.workspaces[0].tabs.len(),
            1,
            "the tab wired to the session closed"
        );
        assert!(
            app.state.workspaces[0]
                .tabs
                .iter()
                .all(|tab| tab.resumed_session_id.as_deref() != Some("s1")),
            "no tab still claims the closed session"
        );

        // A menu left open for a session that no longer runs closes nothing.
        app.state.mode = Mode::ContextMenu;
        app.apply_context_menu_action_via_api(menu("s1"), idx);
        assert_eq!(
            app.state.workspaces[0].tabs.len(),
            1,
            "a stale close is inert rather than closing a bystander"
        );
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
