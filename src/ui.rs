use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::Span,
    Frame,
};

pub(crate) mod app_dock;
mod compose;
mod dialogs;
mod file_manager;
mod keybind_help;
mod menus;
mod mobile;
mod navigator;
mod onboarding;
mod panes;
mod release_notes;
mod scrollbar;
mod settings;
pub(crate) mod shell;
mod sidebar;
mod status;
pub(crate) mod surface_host;
mod tabs;
mod text;
mod widgets;

use self::dialogs::{
    render_confirm_close_overlay, render_file_delete_confirmation_overlay,
    render_new_linked_worktree_overlay, render_open_existing_worktree_overlay,
    render_remove_worktree_overlay, render_rename_overlay,
};
pub(crate) use self::file_manager::compute_file_manager_action_bar_model;
#[cfg(test)]
pub(crate) use self::file_manager::file_manager_preview_content_area_with;
#[cfg(test)]
pub(crate) use self::file_manager::miller::project_miller_view;
pub(crate) use self::file_manager::miller::{
    miller_resize_column_is_live, MillerColumnKind, MillerColumnView, MillerDirectorySource,
    MillerRowColumnKind, MillerViewSnapshot,
};
use self::file_manager::{
    agent_attachment_picker_visible_rows, compute_agent_attachment_picker_row_areas,
    compute_file_manager_header_action_areas, render_agent_attachment_picker, render_file_manager,
    FileManagerRowGeometry,
};
use self::keybind_help::render_keybind_help_overlay;
use self::menus::{
    render_context_menu, render_copy_mode_overlay, render_global_launcher_menu,
    render_navigate_overlay, render_prefix_overlay, render_resize_overlay,
};
use self::mobile::{
    compute_mobile_header_hit_areas, is_mobile_width, mobile_switcher_max_scroll_for_height,
    mobile_toast_banner_rect, render_mobile_header, render_mobile_panel,
    render_mobile_toast_banner,
};
use self::navigator::render_navigator_overlay;
pub(crate) use self::onboarding::onboarding_welcome_continue_rect;
use self::onboarding::render_onboarding_overlay;
use self::panes::{compute_pane_infos, render_panes, resize_tab_panes};
pub(crate) use self::release_notes::{
    product_announcement_display_lines, release_notes_close_button_rect,
    release_notes_display_lines, release_notes_wrapped_line_count, PRODUCT_ANNOUNCEMENT_MODAL_SIZE,
    RELEASE_NOTES_MODAL_SIZE,
};
use self::release_notes::{render_product_announcement_overlay, render_release_notes_overlay};
pub(crate) use self::scrollbar::{
    pane_scrollbar_rect, release_notes_scrollbar_rect, scrollbar_offset_from_drag_row,
    scrollbar_offset_from_row, scrollbar_thumb_grab_offset, should_show_scrollbar,
};
use self::settings::render_settings_overlay;
use self::shell::{RegionId, ShellGeometryKey, ShellLayout};
use self::sidebar::{render_sidebar, render_sidebar_collapsed};
use self::status::{
    copy_feedback_rect, render_config_diagnostic, render_copy_feedback, render_toast_notification,
    toast_notification_rect,
};
use self::tabs::render_tab_bar;
pub(crate) use self::text::display_width_u16;
pub(crate) use self::{
    dialogs::{
        confirm_close_button_rects, confirm_close_popup_rect, new_linked_worktree_button_rects,
        new_linked_worktree_inner_rect, open_existing_worktree_button_rects,
        open_existing_worktree_inner_rect, open_existing_worktree_max_visible_rows,
        open_existing_worktree_visible_start, remove_worktree_button_rects,
        remove_worktree_popup_rect, rename_button_rects,
    },
    dialogs::{
        file_delete_choose_button_rects, file_delete_confirmation_inner_rect,
        file_delete_permanent_button_rects,
    },
    settings::{
        settings_button_rects, settings_popup_height, settings_show_primary_action,
        SETTINGS_POPUP_WIDTH,
    },
    sidebar::{
        agent_panel_body_rect, agent_panel_entries, agent_panel_scroll_metrics,
        agent_panel_scrollbar_rect, agent_panel_toggle_rect, collapsed_sidebar_sections,
        collapsed_sidebar_toggle_rect, compute_workspace_card_areas, expanded_sidebar_sections,
        expanded_sidebar_toggle_rect, normalized_workspace_scroll, projects_scroll_metrics,
        projects_scrollbar_rect, sidebar_section_divider_rect, workspace_drop_indicator_row,
        workspace_list_entries, workspace_list_entries_expanded, workspace_list_rect,
        workspace_list_scroll_metrics, workspace_list_scrollbar_rect, workspace_parent_group_state,
        WorkspaceListEntry,
    },
};
pub(crate) use self::{
    keybind_help::keybind_help_lines,
    mobile::{
        mobile_switcher_areas, mobile_switcher_max_scroll, mobile_switcher_target_at,
        mobile_switcher_workspace_doc_range, MobileSwitcherTarget,
    },
    panes::{apply_pane_chrome, pane_inner_rect, pane_is_scrolled_back},
    tabs::compute_tab_bar_view,
    widgets::{centered_popup_rect, modal_stack_areas},
};
use crate::app::state::ViewLayout;
use crate::app::{AppState, Mode};
use crate::terminal::TerminalRuntimeRegistry;

const COLLAPSED_WIDTH: u16 = 4; // num + space + dot + separator
const LEGACY_DESKTOP_SHELL_LAYOUT_REVISION: u64 = 1;
const MOBILE_EMPTY_SHELL_LAYOUT_REVISION: u64 = 2;

// Braille spinner frames — smooth rotation
const SPINNERS: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Map spinner_tick (incremented every frame at ~60fps) to a spinner frame.
/// We want ~8 updates/sec so divide by 8.
pub(super) fn spinner_frame(tick: u32) -> &'static str {
    SPINNERS[(tick as usize / 8) % SPINNERS.len()]
}

/// Compute view geometry and reconcile pane sizes.
/// Called before render to separate mutation from drawing.
#[cfg_attr(not(test), allow(dead_code))]
pub fn compute_view(app: &mut AppState, area: Rect) {
    let terminal_runtimes = TerminalRuntimeRegistry::new();
    compute_view_with_runtime_registry(app, &terminal_runtimes, area);
}

pub fn compute_view_with_runtime_registry(
    app: &mut AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    area: Rect,
) {
    compute_view_internal(
        app,
        terminal_runtimes,
        area,
        true,
        crate::kitty_graphics::HostCellSize::default(),
    );
}

pub fn compute_view_with_cell_size(
    app: &mut AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    area: Rect,
    cell_size: crate::kitty_graphics::HostCellSize,
) {
    compute_view_internal(app, terminal_runtimes, area, true, cell_size);
}

/// Compute view geometry for a client-sized render without resizing pane runtimes.
///
/// This is used by the headless server when a non-foreground client needs its
/// own frame size while the shared pane runtimes stay pinned to the foreground
/// client.
pub(crate) fn compute_view_without_resizing_panes(
    app: &mut AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    area: Rect,
) {
    compute_view_internal(
        app,
        terminal_runtimes,
        area,
        false,
        crate::kitty_graphics::HostCellSize::default(),
    );
}

fn resize_background_tab_panes_to_area(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    terminal_area: Rect,
    cell_size: crate::kitty_graphics::HostCellSize,
) {
    for (ws_idx, ws) in app.workspaces.iter().enumerate() {
        for (tab_idx, tab) in ws.tabs.iter().enumerate() {
            if app.active == Some(ws_idx) && tab_idx == ws.active_tab_index() {
                continue;
            }
            resize_tab_panes(app, terminal_runtimes, tab, terminal_area, cell_size);
        }
    }
}

fn resize_background_tab_panes_for_desktop(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    main_area: Rect,
    cell_size: crate::kitty_graphics::HostCellSize,
) {
    for (ws_idx, ws) in app.workspaces.iter().enumerate() {
        let (_, terminal_area) = desktop_tab_bar_and_terminal_area(app, ws, main_area);
        for (tab_idx, tab) in ws.tabs.iter().enumerate() {
            if app.active == Some(ws_idx) && tab_idx == ws.active_tab_index() {
                continue;
            }
            resize_tab_panes(app, terminal_runtimes, tab, terminal_area, cell_size);
        }
    }
}

fn desktop_tab_bar_and_terminal_area(
    app: &AppState,
    ws: &crate::workspace::Workspace,
    main_area: Rect,
) -> (Rect, Rect) {
    let hide_single_tab_bar = app.hide_tab_bar_when_single_tab && ws.tabs.len() == 1;
    if !hide_single_tab_bar && main_area.height > 1 {
        let [tab_bar_rect, terminal_area] =
            Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).areas(main_area);
        (tab_bar_rect, terminal_area)
    } else {
        (Rect::default(), main_area)
    }
}

fn compute_view_internal(
    app: &mut AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    area: Rect,
    resize_panes: bool,
    cell_size: crate::kitty_graphics::HostCellSize,
) {
    app.cancel_miller_resize_for_terminal_area(area.width);
    if app
        .shell_resize_original_total()
        .is_some_and(|original_total| original_total != area.width)
    {
        app.cancel_sidebar_resize_for_terminal_area(area.width);
    }

    if is_mobile_width(area, app.mobile_width_threshold) {
        compute_mobile_view(app, terminal_runtimes, area, resize_panes, cell_size);
        return;
    }

    let committed_sidebar_w = if app.sidebar_collapsed {
        match app.sidebar_collapsed_mode {
            crate::config::SidebarCollapsedModeConfig::Compact => COLLAPSED_WIDTH,
            crate::config::SidebarCollapsedModeConfig::Hidden => 0,
        }
    } else {
        app.sidebar_width
            .clamp(app.sidebar_min_width, app.sidebar_max_width)
    };
    let shell_preview_active = app.shell_resize_active();
    let sidebar_w = app
        .shell_resize_preview_width()
        .unwrap_or(committed_sidebar_w);
    let resize_panes = resize_panes_during_shell_preview(resize_panes, shell_preview_active);

    // Derive the outer split from the named-region shell tree. `default()`
    // encodes exactly today's `sidebar | main` layout, so this stays
    // behavior-identical to `Layout::horizontal([Length(sidebar_w), Min(1)])`
    // while making the regions individually addressable for future composition.
    let shell_layout = ShellLayout::default();
    let shell_key = ShellGeometryKey::new(
        area,
        LEGACY_DESKTOP_SHELL_LAYOUT_REVISION,
        u64::from(sidebar_w),
        app.shell_presentation.left_panel_collapse_revision(),
    );
    let previous_shell_view = std::mem::take(&mut app.view.shell);
    let shell_view =
        shell::compute_shell_view(&shell_layout, shell_key, previous_shell_view, &|region| {
            if region == RegionId::LeftPanel {
                sidebar_w
            } else {
                0
            }
        });
    if shell_preview_active {
        app.rebase_sidebar_resize_generation(shell_view.generation);
    }
    let sidebar_area = shell_view.regions.get(RegionId::LeftPanel);
    let main_area = shell_view.regions.get(RegionId::CenterContent);

    // Exactly one stage surface owns the center. The tab bar is
    // terminal-app chrome: while NativeFiles is active the Files surface
    // owns the COMPLETE WorkspaceStage and no tab-bar row is carved out.
    let terminal_surface_active =
        app.stage.surface_view() == surface_host::StageSurfaceView::TerminalWorkspace;
    let (tab_bar_rect, terminal_area) = if terminal_surface_active {
        app.active
            .and_then(|i| app.workspaces.get(i))
            .map(|ws| desktop_tab_bar_and_terminal_area(app, ws, main_area))
            .unwrap_or((Rect::default(), main_area))
    } else {
        (Rect::default(), main_area)
    };
    let file_manager_miller = sync_miller_view(app, terminal_area);
    let FileManagerRowGeometry {
        rows: file_manager_row_areas,
        actions: file_manager_row_action_areas,
    } = sync_file_manager_view(app, &file_manager_miller);
    let file_manager_action_bar = app.file_manager.as_ref().map(|file_manager| {
        compute_file_manager_action_bar_model(
            file_manager,
            &app.file_manager_clipboard,
            app.file_manager_operation
                .as_ref()
                .is_some_and(crate::app::state::FileManagerOperationState::is_running),
        )
    });
    let file_manager_header_action_areas = if app.file_manager.is_some() {
        compute_file_manager_header_action_areas(terminal_area)
    } else {
        Vec::new()
    };

    if !app.sidebar_collapsed {
        app.workspace_scroll = normalized_workspace_scroll(app, sidebar_area, app.workspace_scroll);
        let (_, detail_area) = expanded_sidebar_sections(sidebar_area, app.sidebar_section_split);
        let max_agent_scroll = agent_panel_scroll_metrics(app, detail_area).max_offset_from_bottom;
        app.agent_panel_scroll = app.agent_panel_scroll.min(max_agent_scroll);
    } else {
        app.workspace_scroll = app
            .workspace_scroll
            .min(app.workspaces.len().saturating_sub(1));
        app.agent_panel_scroll = 0;
    }

    // The workspace list belongs to the Spaces tab. Projects/Files render their
    // own content, so no workspace cards are laid out for them.
    let show_spaces_content = app.sidebar_tab == crate::app::state::SidebarTab::Spaces;
    let workspace_card_areas = if app.sidebar_collapsed || !show_spaces_content {
        Vec::new()
    } else {
        compute_workspace_card_areas(app, sidebar_area)
    };
    let sidebar_tab_hit_areas = if app.sidebar_collapsed {
        Vec::new()
    } else {
        sidebar::compute_sidebar_tab_areas(sidebar::workspace_list_rect(
            sidebar_area,
            app.sidebar_section_split,
        ))
    };
    // The Projects tab owns its own row layout. Lay it out here (geometry only)
    // so render stays pure and the mouse handler hit-tests the same rects.
    let project_row_areas =
        if app.sidebar_collapsed || app.sidebar_tab != crate::app::state::SidebarTab::Projects {
            Vec::new()
        } else {
            let list_rect = sidebar::workspace_list_rect(sidebar_area, app.sidebar_section_split);
            // The projects list length changes underneath the scroll offset
            // via the session polls; re-normalize before laying out so the
            // viewport can never point past the end of the list.
            app.projects_scroll =
                sidebar::normalized_projects_scroll(app, list_rect, app.projects_scroll);
            sidebar::compute_project_row_areas(app, list_rect)
        };
    let file_manager_sidebar_row_areas =
        if app.sidebar_collapsed || app.sidebar_tab != crate::app::state::SidebarTab::Files {
            Vec::new()
        } else {
            sidebar::compute_file_manager_sidebar_row_areas(
                app,
                sidebar::workspace_list_rect(sidebar_area, app.sidebar_section_split),
            )
        };

    let tab_bar_view = app
        .active
        .and_then(|ws_idx| app.workspaces.get(ws_idx))
        .map(|ws| {
            compute_tab_bar_view(
                ws,
                tab_bar_rect,
                app.tab_scroll,
                app.tab_scroll_follow_active,
                app.mouse_capture,
            )
        })
        .unwrap_or_default();
    app.tab_scroll = tab_bar_view.scroll;

    // The same surface exclusivity governs projected hit geometry: the
    // hidden terminal projects no pane/split rectangles (and receives no
    // resize side effects) while the NativeFiles surface is active.
    let split_borders = if terminal_surface_active {
        app.active
            .and_then(|i| app.workspaces.get(i))
            .map(|ws| {
                if ws.zoomed {
                    Vec::new()
                } else {
                    ws.layout.splits(terminal_area)
                }
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let pane_infos = if terminal_surface_active {
        compute_pane_infos(
            app,
            terminal_runtimes,
            terminal_area,
            resize_panes,
            cell_size,
        )
    } else {
        Vec::new()
    };
    let agent_attachment_action_area =
        panes::compute_agent_attachment_action_area(app, &pane_infos);
    let agent_worktree_action_area = panes::compute_agent_worktree_action_area(app, &pane_infos);
    let agent_attachment_picker_row_areas = sync_agent_attachment_picker_view(app, terminal_area);
    if resize_panes {
        resize_background_tab_panes_for_desktop(app, terminal_runtimes, main_area, cell_size);
    }

    // Complete dock targets for this frame; the legacy default template
    // projects no dock region, so this stays empty until one is live.
    let app_dock_entry_areas = app_dock::app_dock_entry_areas(
        &app_dock::AppDockModel::for_state(app),
        shell_view.regions.get(RegionId::AppDock),
    );

    let toast_hit_area = app
        .toast
        .as_ref()
        .map(|toast| {
            toast_notification_rect(
                area,
                toast,
                app.config_diagnostic.is_some(),
                toast.position.unwrap_or(app.toast_config.herdr.position),
            )
        })
        .unwrap_or_default();

    app.view = crate::app::ViewState {
        layout: ViewLayout::Desktop,
        shell: shell_view,
        sidebar_rect: sidebar_area,
        workspace_card_areas,
        sidebar_tab_hit_areas,
        project_row_areas,
        file_manager_sidebar_row_areas,
        app_dock_entry_areas,
        file_manager_miller,
        file_manager_row_areas,
        file_manager_row_action_areas,
        file_manager_header_action_areas,
        file_manager_action_bar,
        agent_attachment_action_area,
        agent_worktree_action_area,
        agent_attachment_picker_row_areas,
        tab_bar_rect,
        tab_hit_areas: tab_bar_view.tab_hit_areas,
        tab_scroll_left_hit_area: tab_bar_view.scroll_left_hit_area,
        tab_scroll_right_hit_area: tab_bar_view.scroll_right_hit_area,
        new_tab_hit_area: tab_bar_view.new_tab_hit_area,
        terminal_area,
        mobile_header_rect: Rect::default(),
        mobile_menu_hit_area: Rect::default(),
        toast_hit_area,
        pane_infos,
        split_borders,
    };
    app.sync_copy_mode_search_geometry();
}

fn resize_panes_during_shell_preview(resize_panes: bool, shell_preview_active: bool) -> bool {
    resize_panes && !shell_preview_active
}

fn compute_mobile_view(
    app: &mut AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    area: Rect,
    resize_panes: bool,
    cell_size: crate::kitty_graphics::HostCellSize,
) {
    let header_h = area.height.min(2);
    let (header_rect, terminal_area) = if area.height > header_h {
        let [header_rect, terminal_area] =
            Layout::vertical([Constraint::Length(header_h), Constraint::Min(1)]).areas(area);
        (header_rect, terminal_area)
    } else {
        (area, Rect::default())
    };
    let file_manager_miller = sync_miller_view(app, terminal_area);
    let FileManagerRowGeometry {
        rows: file_manager_row_areas,
        actions: file_manager_row_action_areas,
    } = sync_file_manager_view(app, &file_manager_miller);
    let file_manager_action_bar = app.file_manager.as_ref().map(|file_manager| {
        compute_file_manager_action_bar_model(
            file_manager,
            &app.file_manager_clipboard,
            app.file_manager_operation
                .as_ref()
                .is_some_and(crate::app::state::FileManagerOperationState::is_running),
        )
    });
    let file_manager_header_action_areas = if app.file_manager.is_some() {
        compute_file_manager_header_action_areas(terminal_area)
    } else {
        Vec::new()
    };

    if app.mode == Mode::Navigate {
        let switcher_viewport_h = area.height.saturating_sub(header_h + 1);
        let max_scroll = mobile_switcher_max_scroll_for_height(app, switcher_viewport_h);
        app.mobile_switcher_scroll = app.mobile_switcher_scroll.min(max_scroll);
    }

    // The same surface-exclusivity contract as the desktop projection: a
    // hidden terminal projects no pane/split hit geometry under NativeFiles.
    let terminal_surface_active =
        app.stage.surface_view() == surface_host::StageSurfaceView::TerminalWorkspace;
    let split_borders = if terminal_surface_active {
        app.active
            .and_then(|i| app.workspaces.get(i))
            .map(|ws| {
                if ws.zoomed {
                    Vec::new()
                } else {
                    ws.layout.splits(terminal_area)
                }
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let pane_infos = if terminal_surface_active {
        compute_pane_infos(
            app,
            terminal_runtimes,
            terminal_area,
            resize_panes,
            cell_size,
        )
    } else {
        Vec::new()
    };
    let agent_attachment_picker_row_areas = sync_agent_attachment_picker_view(app, terminal_area);
    if resize_panes {
        resize_background_tab_panes_to_area(app, terminal_runtimes, terminal_area, cell_size);
    }
    let header_hits = compute_mobile_header_hit_areas(app, header_rect);

    let toast_hit_area = app
        .toast
        .as_ref()
        .map(|_| mobile_toast_banner_rect(area, app.config_diagnostic.is_some()))
        .unwrap_or_default();
    let shell_view = shell::compute_empty_shell_view(
        ShellGeometryKey::new(area, MOBILE_EMPTY_SHELL_LAYOUT_REVISION, 0, 0),
        std::mem::take(&mut app.view.shell),
    );

    app.view = crate::app::ViewState {
        layout: ViewLayout::Mobile,
        // Mobile keeps its own header/terminal split; named shell regions are a
        // desktop concept for now, so leave the region map empty.
        shell: shell_view,
        sidebar_rect: Rect::default(),
        workspace_card_areas: Vec::new(),
        sidebar_tab_hit_areas: Vec::new(),
        project_row_areas: Vec::new(),
        file_manager_sidebar_row_areas: Vec::new(),
        app_dock_entry_areas: Vec::new(),
        file_manager_miller,
        file_manager_row_areas,
        file_manager_row_action_areas,
        file_manager_header_action_areas,
        file_manager_action_bar,
        agent_attachment_action_area: None,
        agent_worktree_action_area: None,
        agent_attachment_picker_row_areas,
        tab_bar_rect: Rect::default(),
        tab_hit_areas: Vec::new(),
        tab_scroll_left_hit_area: Rect::default(),
        tab_scroll_right_hit_area: Rect::default(),
        new_tab_hit_area: Rect::default(),
        terminal_area,
        mobile_header_rect: header_rect,
        mobile_menu_hit_area: header_hits.menu,
        toast_hit_area,
        pane_infos,
        split_borders,
    };
    app.sync_copy_mode_search_geometry();
}

fn sync_file_manager_view(app: &AppState, snapshot: &MillerViewSnapshot) -> FileManagerRowGeometry {
    let Some(file_manager) = app.file_manager.as_ref() else {
        return FileManagerRowGeometry::default();
    };
    let Some(current) = snapshot
        .columns
        .iter()
        .find(|column| column.kind.is_current())
    else {
        return FileManagerRowGeometry::default();
    };
    file_manager::compute_file_manager_row_geometry_in_content(
        current.content_rect,
        &file_manager.entries,
        file_manager.viewport_start,
    )
}

fn sync_miller_view(app: &mut AppState, area: Rect) -> MillerViewSnapshot {
    if app.stage.surface_view() != surface_host::StageSurfaceView::NativeFiles {
        return MillerViewSnapshot::default();
    }
    let Some(files_generation) = app.stage.active_instance_generation() else {
        return MillerViewSnapshot::default();
    };
    let viewport_area = file_manager::file_manager_miller_viewport_area(area);
    let resize_preview = app.shell_interaction.miller_resize_preview();
    let Some(file_manager) = app.file_manager.as_mut() else {
        return MillerViewSnapshot::default();
    };
    let mut snapshot = file_manager::miller::project_miller_view_with_resize_preview(
        viewport_area,
        file_manager,
        files_generation,
        resize_preview,
    );
    file_manager.miller.horizontal.first_visible = snapshot.first_visible;
    let current_visible_rows = snapshot
        .columns
        .iter()
        .find(|column| column.kind.is_current())
        .map_or(0, |column| column.content_rect.height as usize);
    let previous_viewport_start = file_manager.viewport_start;
    file_manager.sync_viewport(current_visible_rows);
    if file_manager.viewport_start != previous_viewport_start {
        snapshot = file_manager::miller::project_miller_view_with_resize_preview(
            viewport_area,
            file_manager,
            files_generation,
            resize_preview,
        );
    }
    snapshot
}

fn sync_agent_attachment_picker_view(
    app: &mut AppState,
    area: Rect,
) -> Vec<crate::app::state::FileManagerRowArea> {
    let visible_rows = agent_attachment_picker_visible_rows(area);
    let Some(picker) = app.agent_attachment_picker.as_mut() else {
        return Vec::new();
    };
    picker.file_manager.sync_viewport(visible_rows);
    compute_agent_attachment_picker_row_areas(
        area,
        &picker.file_manager.entries,
        picker.file_manager.viewport_start,
    )
}

/// Render the UI — reads AppState but does not mutate it.
#[cfg_attr(not(test), allow(dead_code))]
pub fn render(app: &AppState, frame: &mut Frame) {
    let terminal_runtimes = TerminalRuntimeRegistry::new();
    render_with_runtime_registry(app, &terminal_runtimes, frame);
}

pub fn render_with_runtime_registry(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    frame: &mut Frame,
) {
    // The whole UI is composed as a back-to-front layer stack (helix's
    // Compositor): the base chrome first, then the single active overlay on
    // top. This is the additive seam for future composition (regions/pages/
    // popups); today it is behavior-identical to painting the two phases inline.
    let ctx = compose::RenderCtx {
        app,
        terminals: terminal_runtimes,
    };
    let compositor = compose::Compositor::new(vec![
        Box::new(BaseLayer) as Box<dyn compose::Component>,
        Box::new(OverlayLayer),
    ]);
    compositor.render(frame, frame.area(), &ctx);
}

/// Layer 0: the persistent base UI — sidebar (or mobile header), tab bar,
/// panes, and ambient notifications. Reads the geometry that `compute_view`
/// stored in `app.view.*`.
struct BaseLayer;

impl compose::Component for BaseLayer {
    fn render(&self, frame: &mut Frame, _area: Rect, ctx: &compose::RenderCtx) {
        let app = ctx.app;
        let terminal_runtimes = ctx.terminals;
        let sidebar_area = app.view.sidebar_rect;
        let tab_bar_area = app.view.tab_bar_rect;
        let terminal_area = app.view.terminal_area;

        if app.view.layout == ViewLayout::Mobile {
            render_mobile_header(app, terminal_runtimes, frame, app.view.mobile_header_rect);
        } else if sidebar_area.width > 0 {
            if app.sidebar_collapsed {
                render_sidebar_collapsed(app, frame, sidebar_area);
            } else {
                render_sidebar(app, terminal_runtimes, frame, sidebar_area);
            }
        }
        if app.view.layout != ViewLayout::Mobile {
            render_tab_bar(app, frame, tab_bar_area);
        }
        // The AppDock renders only when the current shell projects it a
        // non-empty region (the legacy default template projects none, so
        // this stays a no-op until a dock-bearing template is live).
        let dock_area = app.view.shell.regions.get(RegionId::AppDock);
        if !dock_area.is_empty() {
            app_dock::render_app_dock(
                app,
                &app_dock::AppDockModel::for_state(app),
                frame,
                dock_area,
            );
        }
        // CenterContent hosts exactly one typed stage surface. The TYPED
        // Stage projection chooses the renderer so a divergent legacy
        // boolean can never paint a surface the stage does not own.
        match app.stage.surface_view() {
            surface_host::StageSurfaceView::NativeFiles => {
                render_file_manager(app, frame, terminal_area)
            }
            surface_host::StageSurfaceView::TerminalWorkspace => {
                render_panes(app, terminal_runtimes, frame, terminal_area)
            }
        }

        // Ambient notifications sit above the center content, but below
        // interactive overlays.
        render_notifications(app, frame, terminal_area);
    }
}

/// Layer 1: the single active interactive overlay selected by `app.mode`,
/// painted on top of the base. `Mode::Terminal` renders no overlay.
struct OverlayLayer;

impl compose::Component for OverlayLayer {
    fn render(&self, frame: &mut Frame, _area: Rect, ctx: &compose::RenderCtx) {
        let app = ctx.app;
        let terminal_runtimes = ctx.terminals;
        let terminal_area = app.view.terminal_area;

        match app.mode {
            Mode::Onboarding => render_onboarding_overlay(app, frame, frame.area()),
            Mode::ReleaseNotes => render_release_notes_overlay(app, frame, frame.area()),
            Mode::ProductAnnouncement => {
                render_product_announcement_overlay(app, frame, frame.area())
            }
            Mode::Navigate if app.view.layout == ViewLayout::Mobile => {
                render_mobile_panel(app, terminal_runtimes, frame, frame.area())
            }
            Mode::Navigate => render_navigate_overlay(app, frame, terminal_area),
            Mode::Prefix => render_prefix_overlay(app, frame, terminal_area),
            Mode::Copy => render_copy_mode_overlay(app, frame, terminal_area),
            Mode::Resize => render_resize_overlay(app, frame, terminal_area),
            Mode::ConfirmClose => render_confirm_close_overlay(app, frame, terminal_area),
            Mode::ConfirmFileDelete => {
                render_file_delete_confirmation_overlay(app, frame, terminal_area)
            }
            Mode::ContextMenu => {
                render_context_menu(app, frame);
            }
            Mode::Settings => render_settings_overlay(app, frame, frame.area()),
            Mode::RenameWorkspace | Mode::RenameTab | Mode::RenamePane | Mode::RenameFile => {
                render_rename_overlay(app, frame, frame.area())
            }
            Mode::NewLinkedWorktree => render_new_linked_worktree_overlay(app, frame, frame.area()),
            Mode::OpenExistingWorktree => {
                render_open_existing_worktree_overlay(app, frame, frame.area())
            }
            Mode::ConfirmRemoveWorktree => render_remove_worktree_overlay(app, frame, frame.area()),
            Mode::GlobalMenu => render_global_launcher_menu(app, frame),
            Mode::KeybindHelp => render_keybind_help_overlay(app, frame),
            Mode::Navigator => render_navigator_overlay(app, terminal_runtimes, frame),
            Mode::AttachFile => render_agent_attachment_picker(app, frame, terminal_area),
            Mode::Terminal => {}
        }
    }
}

fn render_notifications(app: &AppState, frame: &mut Frame, terminal_area: Rect) {
    let has_config_diagnostic = app.config_diagnostic.is_some();
    if let Some(message) = &app.config_diagnostic {
        let diagnostic_area = if app.view.layout == ViewLayout::Mobile {
            terminal_area
        } else {
            frame.area()
        };
        render_config_diagnostic(frame, diagnostic_area, message, &app.palette);
    }
    let mut copy_feedback_offset = u16::from(has_config_diagnostic);
    let mut toast_rect = None;
    if let Some(toast) = &app.toast {
        if app.view.layout == ViewLayout::Mobile {
            render_mobile_toast_banner(
                frame,
                frame.area(),
                toast,
                has_config_diagnostic,
                &app.palette,
            );
        } else {
            render_toast_notification(
                frame,
                frame.area(),
                toast,
                has_config_diagnostic,
                toast.position.unwrap_or(app.toast_config.herdr.position),
                &app.palette,
            );
            toast_rect = Some(toast_notification_rect(
                frame.area(),
                toast,
                has_config_diagnostic,
                toast.position.unwrap_or(app.toast_config.herdr.position),
            ));
        }
        if app.view.layout == ViewLayout::Mobile {
            toast_rect = Some(mobile_toast_banner_rect(
                frame.area(),
                has_config_diagnostic,
            ));
        }
    }
    if let Some(feedback) = &app.copy_feedback {
        let area = if app.view.layout == ViewLayout::Mobile {
            frame.area()
        } else {
            terminal_area
        };
        if let Some(toast_rect) = toast_rect {
            copy_feedback_offset = copy_feedback_offset_for_toast(
                area,
                feedback,
                copy_feedback_offset,
                app.toast_config.clipboard.position,
                toast_rect,
            );
        }
        render_copy_feedback(
            frame,
            area,
            feedback,
            copy_feedback_offset,
            app.toast_config.clipboard.position,
            &app.palette,
        );
    }
}

fn copy_feedback_offset_for_toast(
    area: Rect,
    feedback: &crate::app::state::CopyFeedback,
    base_offset: u16,
    position: crate::config::ToastClipboardPosition,
    toast_rect: Rect,
) -> u16 {
    let feedback_rect = copy_feedback_rect(area, feedback, base_offset, position);
    if rects_overlap(feedback_rect, toast_rect) {
        base_offset.saturating_add(toast_rect.height)
    } else {
        base_offset
    }
}

fn rects_overlap(a: Rect, b: Rect) -> bool {
    a.x < b.x.saturating_add(b.width)
        && b.x < a.x.saturating_add(a.width)
        && a.y < b.y.saturating_add(b.height)
        && b.y < a.y.saturating_add(a.height)
}

fn dim_background(frame: &mut Frame, area: Rect) {
    let buf = frame.buffer_mut();
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let cell = &mut buf[(x, y)];
            cell.set_style(cell.style().add_modifier(Modifier::DIM));
        }
    }
}

/// Floating overlay for navigate mode — appears at bottom of terminal area.
fn _build_hints(items: &[(&str, &str)], key_style: Style, dim_style: Style) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    spans.push(Span::raw(" "));
    for (i, (k, desc)) in items.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", dim_style));
        }
        spans.push(Span::styled(k.to_string(), key_style));
        spans.push(Span::styled(format!(" {desc}"), dim_style));
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::keybind_help::keybind_help_groups;
    use super::scrollbar::scrollbar_thumb;
    use super::*;
    use crate::{app::state::ViewLayout, layout::PaneInfo, workspace::Workspace};
    use ratatui::style::Color;
    use ratatui::{backend::TestBackend, Terminal};

    fn prepared_miller_projection_app(
        chain_len: usize,
        focused_index: usize,
    ) -> (crate::app::state::AppState, Vec<std::path::PathBuf>) {
        assert!(chain_len > 0);
        assert!(focused_index < chain_len);

        let mut file_manager =
            crate::fm::FmState::new(std::env::current_dir().expect("current directory"));
        let directories = (0..chain_len)
            .map(|index| {
                std::path::PathBuf::from(format!(
                    "/definitely-missing-herdr-miller/segment-{index}"
                ))
            })
            .collect::<Vec<_>>();
        file_manager.miller.chain = directories
            .iter()
            .cloned()
            .map(crate::fm::miller::MillerPathSegment::new)
            .collect();
        file_manager.miller.focused_directory = directories[focused_index].clone();
        file_manager.cwd = directories[focused_index].clone();
        file_manager.parent = None;

        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.mobile_width_threshold = 0;
        app.sidebar_collapsed = true;
        app.sidebar_collapsed_mode = crate::config::SidebarCollapsedModeConfig::Hidden;
        app.try_open_file_manager_with(|_| Some(file_manager))
            .expect("Files activation");
        (app, directories)
    }

    // S2 integration: `compute_view` populates `view.shell.regions` from the shell tree
    // consistently with the established `sidebar_rect`/main-area geometry — the
    // named-region map is the same outer split, just addressable by `RegionId`.
    #[test]
    fn desktop_shell_regions_match_computed_geometry() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;

        let frame = Rect::new(0, 0, 100, 30);
        compute_view(&mut app, frame);

        let left = app.view.shell.regions.get(RegionId::LeftPanel);
        let center = app.view.shell.regions.get(RegionId::CenterContent);

        // LeftPanel is exactly the sidebar; CenterContent is the rest of the frame.
        assert_eq!(left, app.view.sidebar_rect);
        assert!(left.width > 0, "expanded sidebar should have width");
        assert_eq!(center.x, left.x + left.width);
        assert_eq!(center.y, frame.y);
        assert_eq!(center.height, frame.height);
        assert_eq!(center.width, frame.width - left.width);
        // The tab bar + terminal partition the CenterContent region vertically.
        assert_eq!(app.view.tab_bar_rect.x, center.x);
        assert_eq!(app.view.terminal_area.x, center.x);
        assert_eq!(
            app.view.terminal_area.y + app.view.terminal_area.height,
            center.y + center.height
        );
        // Reserved regions are not laid out yet.
        assert_eq!(
            app.view.shell.regions.get(RegionId::RightPanel),
            Rect::default()
        );
        assert_eq!(
            app.view.shell.regions.get(RegionId::TopBar),
            Rect::default()
        );
    }

    #[test]
    fn shell_resize_preview_suppresses_runtime_resize_policy() {
        assert!(!resize_panes_during_shell_preview_for_test(true, true));
        assert!(resize_panes_during_shell_preview_for_test(true, false));
        assert!(!resize_panes_during_shell_preview_for_test(false, true));
    }

    fn resize_panes_during_shell_preview_for_test(
        resize_panes: bool,
        shell_preview_active: bool,
    ) -> bool {
        resize_panes_during_shell_preview(resize_panes, shell_preview_active)
    }

    // Mobile keeps its own header/terminal split; the named shell regions stay
    // empty there for now (desktop-only concept).
    #[test]
    fn mobile_view_leaves_shell_regions_empty() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;

        // Width <= mobile threshold (64) selects the mobile layout path.
        compute_view(&mut app, Rect::new(0, 0, 30, 20));

        assert_eq!(app.view.layout, ViewLayout::Mobile);
        assert_eq!(
            app.view.shell.regions.get(RegionId::LeftPanel),
            Rect::default()
        );
        assert_eq!(
            app.view.shell.regions.get(RegionId::CenterContent),
            Rect::default()
        );
    }

    // TP-A3.2-VIEWPORT: compute_view owns viewport normalization for both
    // responsive layouts. Shrinking and expanding the available height keeps
    // the cursor visible and clamps stale offsets to the new maximum.
    #[test]
    fn compute_view_normalizes_file_manager_viewport_after_resize() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "herdr-ui-fm-viewport-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).expect("create temp root");
        for index in 0..10 {
            std::fs::write(root.join(format!("{index:02}.txt")), b"x")
                .expect("write viewport fixture");
        }

        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&root)))
            .expect("Files activation");
        app.file_manager.as_mut().expect("open fm").cursor = 8;

        // Desktop: Files owns the full stage (no tab-bar carve-out since
        // SF6.1) -> height 7 = FM header 1, panel title 1, status 1, list 4.
        compute_view(&mut app, Rect::new(0, 0, 100, 7));
        assert_eq!(
            app.file_manager.as_ref().expect("open fm").viewport_start,
            5
        );

        // Expanding to nine list rows clamps the old start to max_start=1.
        compute_view(&mut app, Rect::new(0, 0, 100, 12));
        assert_eq!(
            app.file_manager.as_ref().expect("open fm").viewport_start,
            1
        );

        // Mobile: height 7 -> mobile header 2, FM header 1, panel title 1,
        // status 1, leaving two visible rows and requiring start=7 for cursor 8.
        compute_view(&mut app, Rect::new(0, 0, 30, 7));
        assert_eq!(
            app.file_manager.as_ref().expect("open fm").viewport_start,
            7
        );

        std::fs::remove_dir_all(root).expect("remove temp root");
    }

    // TP-C2.1-VIEWSTATE: desktop compute_view snapshots CURRENT name and action
    // rects from one geometry source, then clears both when FM closes so stale
    // terminal coordinates can never remain clickable.
    #[test]
    fn compute_view_snapshots_and_clears_file_manager_row_areas() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "herdr-ui-fm-hit-geometry-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).expect("create temp root");
        for index in 0..6 {
            std::fs::write(root.join(format!("{index:02}.txt")), b"x")
                .expect("write hit geometry fixture");
        }

        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&root)))
            .expect("Files activation");
        app.file_manager.as_mut().expect("open fm").cursor = 4;

        compute_view(&mut app, Rect::new(0, 0, 100, 6));
        assert_eq!(
            app.view
                .file_manager_row_areas
                .iter()
                .map(|row| row.entry_idx)
                .collect::<Vec<_>>(),
            vec![2, 3, 4],
            "Files owns the full stage since SF6.1, so height 6 shows three rows"
        );
        assert!(app
            .view
            .file_manager_row_areas
            .iter()
            .all(|row| row.rect.width > 0 && row.rect.height == 1));
        assert_eq!(
            app.view
                .file_manager_row_action_areas
                .iter()
                .map(|area| (area.entry_idx, area.action))
                .collect::<Vec<_>>(),
            [2, 3, 4]
                .into_iter()
                .flat_map(|entry_idx| {
                    crate::app::state::FileManagerRowAction::ALL.map(|action| (entry_idx, action))
                })
                .collect::<Vec<_>>()
        );
        assert_eq!(
            app.view
                .file_manager_header_action_areas
                .iter()
                .map(|area| area.action)
                .collect::<Vec<_>>(),
            vec![
                crate::app::state::FileManagerHeaderAction::Copy,
                crate::app::state::FileManagerHeaderAction::Paste,
                crate::app::state::FileManagerHeaderAction::NewFolder,
                crate::app::state::FileManagerHeaderAction::Delete,
            ]
        );
        assert!(
            !app.view.file_manager_miller.columns.is_empty(),
            "open Files projects at least one bounded Miller column"
        );

        app.close_file_manager();
        assert!(
            app.view.file_manager_miller.columns.is_empty()
                && app.view.file_manager_miller.dividers.is_empty()
                && app.view.file_manager_miller.files_generation.is_none(),
            "close must retire the Miller projection in the same transaction"
        );
        compute_view(&mut app, Rect::new(0, 0, 100, 6));
        assert!(app.view.file_manager_row_areas.is_empty());
        assert!(app.view.file_manager_row_action_areas.is_empty());
        assert!(app.view.file_manager_header_action_areas.is_empty());

        std::fs::remove_dir_all(root).expect("remove temp root");
    }

    // P1.4: the inline preview is existing published Files behavior. A
    // snapshot cutover must type and place it instead of silently dropping it.
    #[test]
    fn windowed_projection_preserves_inline_preview_column() {
        let cwd = std::env::current_dir().expect("current directory");
        let mut file_manager = crate::fm::FmState::new(cwd.clone());
        file_manager.miller.chain =
            std::iter::once(crate::fm::miller::MillerPathSegment::new(cwd.clone())).collect();
        file_manager.miller.focused_directory = cwd;

        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.mobile_width_threshold = 0;
        app.sidebar_collapsed = true;
        app.sidebar_collapsed_mode = crate::config::SidebarCollapsedModeConfig::Hidden;
        app.try_open_file_manager_with(|_| Some(file_manager))
            .expect("Files activation");

        compute_view(&mut app, Rect::new(0, 0, 16, 16));
        assert_eq!(
            app.view.file_manager_miller.columns.len(),
            1,
            "one-column width must keep the operational current directory visible"
        );

        compute_view(&mut app, Rect::new(0, 0, 33, 16));
        assert_eq!(
            app.view.file_manager_miller.columns.len(),
            2,
            "two-column width must preserve current plus the inline preview column"
        );
        assert_eq!(
            app.view.file_manager_miller.dividers.len(),
            1,
            "current and inline preview are separated by one projected divider"
        );
    }

    #[test]
    fn current_legacy_row_adapter_derives_from_snapshot_column() {
        let cwd = std::env::current_dir().expect("current directory");
        let mut file_manager = crate::fm::FmState::new(cwd.clone());
        let mut directories = (0..4)
            .map(|index| std::path::PathBuf::from(format!("/virtual/ancestor-{index}")))
            .collect::<Vec<_>>();
        directories.push(cwd.clone());
        file_manager.miller.chain = directories
            .iter()
            .cloned()
            .map(crate::fm::miller::MillerPathSegment::new)
            .collect();
        file_manager.miller.focused_directory = cwd.clone();

        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.mobile_width_threshold = 0;
        app.sidebar_collapsed = true;
        app.sidebar_collapsed_mode = crate::config::SidebarCollapsedModeConfig::Hidden;
        app.try_open_file_manager_with(|_| Some(file_manager))
            .expect("Files activation");

        compute_view(&mut app, Rect::new(0, 0, 115, 16));

        let current_column = app
            .view
            .file_manager_miller
            .columns
            .iter()
            .find(|column| column.kind.directory() == Some(&cwd))
            .expect("current column projected");
        let current_content = current_column.content_rect;
        assert!(
            !app.view.file_manager_row_areas.is_empty(),
            "the repository cwd fixture must expose visible current rows"
        );
        assert!(
            app.view
                .file_manager_row_areas
                .iter()
                .all(|row| row.rect.intersection(current_content) == row.rect),
            "current row rects must derive from the snapshot current content rect: \
             current={current_content:?}, rows={:?}",
            app.view.file_manager_row_areas
        );
        assert!(
            app.view
                .file_manager_row_action_areas
                .iter()
                .all(|action| action.rect.intersection(current_content) == action.rect),
            "current action rects must stay inside the same snapshot current content rect"
        );
    }

    // P1 RED: the pure FM1.3 geometry exists, but production `compute_view`
    // must consume it and persist the clamped horizontal origin. This uses a
    // prepared in-memory logical chain; compute is forbidden from loading any
    // of these synthetic paths.
    #[test]
    fn compute_view_projects_one_to_five_miller_columns() {
        let mut file_manager =
            crate::fm::FmState::new(std::env::current_dir().expect("current directory"));
        let directories = (0..7)
            .map(|index| std::path::PathBuf::from(format!("/virtual/segment-{index}")))
            .collect::<Vec<_>>();
        file_manager.miller.chain = directories
            .iter()
            .cloned()
            .map(crate::fm::miller::MillerPathSegment::new)
            .collect();
        file_manager.miller.focused_directory =
            directories.last().expect("focused segment").clone();

        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.mobile_width_threshold = 0;
        app.sidebar_collapsed = true;
        app.sidebar_collapsed_mode = crate::config::SidebarCollapsedModeConfig::Hidden;
        app.try_open_file_manager_with(|_| Some(file_manager))
            .expect("Files activation");

        let mut observed_column_counts = Vec::new();
        for width in [16, 57, 86, 115, 144, 400] {
            let file_manager = app.file_manager.as_mut().expect("open FM");
            file_manager.miller.horizontal.first_visible = 0;
            let focused_index = file_manager.miller.chain.len() - 1;
            let mut preferred_widths = file_manager
                .miller
                .chain
                .iter()
                .take(focused_index + 1)
                .map(|segment| segment.preferred_width)
                .collect::<Vec<_>>();
            let frame = Rect::new(0, 0, width, 16);
            let expected_body = Rect::new(
                frame.x,
                frame.y + 1,
                frame.width,
                frame.height.saturating_sub(2),
            );
            let expected_focused_index = if expected_body.width
                >= crate::fm::miller::MILLER_COLUMN_MIN_WIDTH * 2
                    + file_manager::miller::MILLER_DIVIDER_WIDTH
            {
                let pair_budget = expected_body
                    .width
                    .saturating_sub(file_manager::miller::MILLER_DIVIDER_WIDTH);
                let current_max = pair_budget
                    .saturating_sub(crate::fm::miller::MILLER_COLUMN_MIN_WIDTH)
                    .min(crate::fm::miller::MILLER_COLUMN_MAX_WIDTH);
                let current_width = preferred_widths[focused_index]
                    .clamp(crate::fm::miller::MILLER_COLUMN_MIN_WIDTH, current_max);
                let preview_width = pair_budget.saturating_sub(current_width).clamp(
                    crate::fm::miller::MILLER_COLUMN_MIN_WIDTH,
                    crate::fm::miller::MILLER_COLUMN_PREFERRED_WIDTH,
                );
                preferred_widths[focused_index] = current_width;
                preferred_widths.push(preview_width);
                focused_index + 1
            } else {
                focused_index
            };
            let expected = file_manager::miller::miller_viewport_geometry(
                expected_body,
                &preferred_widths,
                expected_focused_index,
                0,
            );

            compute_view(&mut app, frame);

            let snapshot = &app.view.file_manager_miller;
            assert_eq!(
                snapshot.files_generation,
                app.stage.active_instance_generation(),
                "the frame snapshot carries the active Files singleton generation"
            );
            assert_eq!(
                snapshot.model_revision,
                app.file_manager.as_ref().expect("open FM").miller.revision
            );
            assert_eq!(
                snapshot.first_visible, expected.first_visible,
                "the frame snapshot owns the clamped window at width {width}"
            );
            assert_eq!(
                snapshot.focused_chain_index,
                Some(focused_index),
                "the exact focused logical segment is projected"
            );
            assert_eq!(
                snapshot.columns.len(),
                expected.columns.len(),
                "production projection publishes every complete visible column"
            );
            observed_column_counts.push(snapshot.columns.len());
            assert_eq!(
                snapshot.dividers.len(),
                expected.dividers.len(),
                "production projection publishes one divider per adjacent pair"
            );
            for (projected, geometry) in snapshot.columns.iter().zip(&expected.columns) {
                assert_eq!(projected.projection_index, geometry.chain_index);
                assert_eq!(projected.rect, geometry.rect);
                if geometry.chain_index == focused_index + 1 {
                    assert!(
                        projected.kind.is_preview(),
                        "the synthetic final projection is the typed inline preview"
                    );
                } else {
                    assert_eq!(
                        projected.kind.directory(),
                        Some(&directories[geometry.chain_index]),
                        "column identity comes from the exact logical segment"
                    );
                }
            }
            for (projected, geometry) in snapshot.dividers.iter().zip(&expected.dividers) {
                assert_eq!(projected.rect, geometry.rect);
                assert_eq!(
                    snapshot.columns[projected.left_column].projection_index,
                    geometry.left_chain_index
                );
                assert_eq!(
                    snapshot.columns[projected.right_column].projection_index,
                    geometry.right_chain_index
                );
            }
            assert_eq!(
                app.file_manager
                    .as_ref()
                    .expect("open FM")
                    .miller
                    .horizontal
                    .first_visible,
                expected.first_visible,
                "production compute must persist the clamped Miller window at width {width}"
            );
            assert!(
                (1..=crate::fm::miller::MAX_RESIDENT_MILLER_COLUMNS)
                    .contains(&expected.columns.len()),
                "the width {width} fixture must exercise one to five complete columns"
            );
            assert_eq!(
                expected.dividers.len(),
                expected.columns.len().saturating_sub(1),
                "every adjacent visible pair has one divider at width {width}"
            );
        }
        assert_eq!(
            observed_column_counts,
            vec![1, 2, 3, 4, 5, 5],
            "production projection exercises every supported visible-column count"
        );

        compute_view(&mut app, Rect::new(0, 0, 0, 16));
        assert!(
            app.view.file_manager_miller.columns.is_empty()
                && app.view.file_manager_miller.dividers.is_empty(),
            "zero-width Files projection publishes no stale Miller targets"
        );
    }

    #[test]
    fn windowed_projection_uses_model_first_visible() {
        let (mut app, directories) = prepared_miller_projection_app(7, 4);
        let original_revision = app.file_manager.as_ref().expect("open FM").miller.revision;
        app.file_manager
            .as_mut()
            .expect("open FM")
            .miller
            .horizontal
            .first_visible = 2;

        compute_view(&mut app, Rect::new(0, 0, 86, 16));

        let snapshot = &app.view.file_manager_miller;
        assert_eq!(
            snapshot.first_visible, 3,
            "current plus preview stay visible, so the stale ancestor origin clamps forward"
        );
        assert_eq!(
            snapshot
                .columns
                .iter()
                .map(|column| column.kind.chain_index())
                .collect::<Vec<_>>(),
            vec![Some(3), Some(4), None],
            "the bounded window carries the nearest ancestor, current, and typed preview"
        );
        let file_manager = app.file_manager.as_ref().expect("open FM");
        assert_eq!(file_manager.miller.horizontal.first_visible, 3);
        assert_eq!(file_manager.miller.focused_directory, directories[4]);
        assert_eq!(file_manager.miller.revision, original_revision);
        assert_eq!(
            file_manager
                .miller
                .chain
                .iter()
                .map(|segment| segment.directory.clone())
                .collect::<Vec<_>>(),
            directories,
            "projection must not rewrite the logical chain"
        );
    }

    #[test]
    fn zero_files_area_retires_windowed_miller_targets() {
        let (mut app, _) = prepared_miller_projection_app(7, 6);

        compute_view(&mut app, Rect::new(0, 0, 100, 16));
        assert!(!app.view.file_manager_miller.columns.is_empty());

        compute_view(&mut app, Rect::new(0, 0, 0, 16));
        assert!(app.view.file_manager_miller.columns.is_empty());
        assert!(app.view.file_manager_miller.dividers.is_empty());
        assert!(app.view.file_manager_row_areas.is_empty());
        assert!(app.view.file_manager_row_action_areas.is_empty());
        assert!(app.view.file_manager_header_action_areas.is_empty());

        app.mobile_width_threshold = u16::MAX;
        compute_view(&mut app, Rect::new(0, 0, 80, 16));
        assert!(
            !app.view.file_manager_miller.columns.is_empty(),
            "precondition: the mobile Files body owns live targets"
        );
        compute_view(&mut app, Rect::new(0, 0, 80, 2));
        assert!(app.view.file_manager_miller.columns.is_empty());
        assert!(app.view.file_manager_miller.dividers.is_empty());
        assert!(app.view.file_manager_row_areas.is_empty());
        assert!(app.view.file_manager_row_action_areas.is_empty());
        assert!(
            app.view.file_manager_header_action_areas.is_empty(),
            "a header-only mobile frame must expose no Files body targets"
        );
    }

    #[test]
    fn windowed_projection_does_not_read_filesystem() {
        let (mut app, directories) = prepared_miller_projection_app(7, 6);
        assert!(
            directories.iter().all(|directory| !directory.exists()),
            "precondition: every logical segment is intentionally absent on disk"
        );
        let file_manager = app.file_manager.as_ref().expect("open FM");
        let original_cwd = file_manager.cwd.clone();
        let original_entries = file_manager.entries.clone();
        let original_revision = file_manager.miller.revision;

        compute_view(&mut app, Rect::new(0, 0, 144, 16));

        let file_manager = app.file_manager.as_ref().expect("open FM");
        assert_eq!(file_manager.cwd, original_cwd);
        assert_eq!(file_manager.entries, original_entries);
        assert_eq!(file_manager.miller.revision, original_revision);
        assert_eq!(
            file_manager
                .miller
                .chain
                .iter()
                .map(|segment| segment.directory.clone())
                .collect::<Vec<_>>(),
            directories,
            "compute consumes prepared state without loading synthetic directories"
        );
        assert_eq!(app.view.file_manager_miller.columns.len(), 5);
    }

    #[test]
    fn focused_column_remains_visible_after_projection_shrink() {
        let (mut app, directories) = prepared_miller_projection_app(7, 6);

        compute_view(&mut app, Rect::new(0, 0, 144, 16));
        assert_eq!(app.view.file_manager_miller.columns.len(), 5);
        let wide_first_visible = app.view.file_manager_miller.first_visible;

        compute_view(&mut app, Rect::new(0, 0, 57, 16));

        let snapshot = &app.view.file_manager_miller;
        assert_eq!(snapshot.columns.len(), 2);
        assert!(snapshot.first_visible > wide_first_visible);
        assert_eq!(snapshot.focused_chain_index, Some(6));
        assert!(
            snapshot.columns.iter().any(|column| {
                column.kind.chain_index() == Some(6)
                    && column.kind.directory() == Some(&directories[6])
            }),
            "the focused logical column remains visible after shrink"
        );
        let body = file_manager::file_manager_miller_viewport_area(Rect::new(0, 0, 57, 16));
        assert!(snapshot.columns.iter().all(|column| {
            column.rect.x >= body.x
                && column.rect.y >= body.y
                && column.rect.right() <= body.right()
                && column.rect.bottom() <= body.bottom()
        }));
        assert_eq!(
            app.file_manager
                .as_ref()
                .expect("open FM")
                .miller
                .chain
                .len(),
            7,
            "responsive shrink changes only the viewport window"
        );
    }

    #[test]
    fn reopened_files_projection_uses_fresh_instance_generation() {
        let (mut app, _) = prepared_miller_projection_app(3, 2);
        compute_view(&mut app, Rect::new(0, 0, 100, 16));
        let first_generation = app
            .view
            .file_manager_miller
            .files_generation
            .expect("first Files generation");

        app.close_file_manager();
        assert!(app.view.file_manager_miller.files_generation.is_none());
        app.try_open_file_manager_with(|_| {
            Some(crate::fm::FmState::new(
                std::env::current_dir().expect("current directory"),
            ))
        })
        .expect("reopen Files");
        compute_view(&mut app, Rect::new(0, 0, 100, 16));

        let reopened_generation = app
            .view
            .file_manager_miller
            .files_generation
            .expect("reopened Files generation");
        assert!(
            reopened_generation > first_generation,
            "close/reopen must not alias prior Files projection identity"
        );
    }

    #[test]
    fn windowed_projection_requires_typed_files_surface() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.mobile_width_threshold = 0;
        app.sidebar_collapsed = true;
        app.sidebar_collapsed_mode = crate::config::SidebarCollapsedModeConfig::Hidden;
        app.try_open_file_manager_with(|_| {
            Some(crate::fm::FmState::new(
                std::env::current_dir().expect("current directory"),
            ))
        })
        .expect("Files activation");

        compute_view(&mut app, Rect::new(0, 0, 100, 16));
        assert!(
            !app.view.file_manager_miller.columns.is_empty(),
            "precondition: typed Files owns a live Miller projection"
        );

        // Adversarial split-brain fixture: retain the domain model but remove
        // typed Files ownership. Projection must follow Stage authority.
        app.stage = crate::ui::surface_host::StageState::default();
        compute_view(&mut app, Rect::new(0, 0, 100, 16));

        assert!(
            app.view.file_manager_miller.columns.is_empty()
                && app.view.file_manager_miller.dividers.is_empty()
                && app.view.file_manager_miller.files_generation.is_none(),
            "a foreign typed Stage surface must project no Miller geometry"
        );
        assert!(
            app.file_manager.is_some(),
            "the authority test must not pass by deleting the Files model"
        );
    }

    // TP-N3.1-LIFECYCLE: compute_view rebuilds persistent action-bar content
    // after navigation/reload, clears it on close, and restores current empty
    // selection plus the client-local clipboard summary on reopen.
    #[test]
    fn compute_view_refreshes_and_clears_file_manager_action_bar_content() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "herdr-ui-fm-action-bar-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).expect("create temp root");
        std::fs::write(root.join("a.txt"), b"a").expect("write a fixture");
        std::fs::write(root.join("b.txt"), b"b").expect("write b fixture");

        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&root)))
            .expect("Files activation");
        app.file_manager_clipboard = vec![root.join("clipboard.txt")];

        compute_view(&mut app, Rect::new(0, 0, 100, 6));
        let initial = app
            .view
            .file_manager_action_bar
            .as_ref()
            .expect("open FM action bar");
        assert!(initial.selection.is_none());
        assert_eq!(initial.clipboard_count, 1);

        assert!(app
            .file_manager
            .as_mut()
            .expect("open FM")
            .replace_selection(0));
        compute_view(&mut app, Rect::new(0, 0, 100, 6));
        assert_eq!(
            app.view
                .file_manager_action_bar
                .as_ref()
                .and_then(|model| model.selection.as_ref())
                .map(|selection| selection.label.as_str()),
            Some("a.txt")
        );

        app.file_manager.as_mut().expect("open FM").move_down();
        compute_view(&mut app, Rect::new(0, 0, 100, 6));
        assert_eq!(
            app.view
                .file_manager_action_bar
                .as_ref()
                .and_then(|model| model.selection.as_ref())
                .map(|selection| selection.label.as_str()),
            Some("a.txt")
        );

        assert!(app
            .file_manager
            .as_mut()
            .expect("open FM")
            .replace_selection(1));
        compute_view(&mut app, Rect::new(0, 0, 100, 6));
        assert_eq!(
            app.view
                .file_manager_action_bar
                .as_ref()
                .and_then(|model| model.selection.as_ref())
                .map(|selection| selection.label.as_str()),
            Some("b.txt")
        );

        std::fs::remove_file(root.join("b.txt")).expect("remove selected fixture");
        app.file_manager.as_mut().expect("open FM").reload();
        compute_view(&mut app, Rect::new(0, 0, 100, 6));
        assert_eq!(
            app.view
                .file_manager_action_bar
                .as_ref()
                .and_then(|model| model.selection.as_ref())
                .map(|selection| selection.label.as_str()),
            None
        );

        std::fs::remove_file(root.join("a.txt")).expect("remove final fixture");
        app.file_manager.as_mut().expect("open FM").reload();
        compute_view(&mut app, Rect::new(0, 0, 100, 6));
        let empty = app
            .view
            .file_manager_action_bar
            .as_ref()
            .expect("empty open FM action bar");
        assert!(empty.selection.is_none());
        assert_eq!(empty.clipboard_count, 1);

        app.file_manager = None;
        compute_view(&mut app, Rect::new(0, 0, 100, 6));
        assert!(app.view.file_manager_action_bar.is_none());

        app.try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&root)))
            .expect("Files activation");
        compute_view(&mut app, Rect::new(0, 0, 100, 6));
        let reopened = app
            .view
            .file_manager_action_bar
            .as_ref()
            .expect("reopened action bar");
        assert!(reopened.selection.is_none());
        assert_eq!(reopened.clipboard_count, 1);

        std::fs::remove_dir_all(root).expect("remove temp root");
    }

    // SF4.3-03 characterization: valid RED refuted by source sweep — the
    // stage surface render path (`render_panes` / `render_file_manager`)
    // reads no clock and no randomness, so identical state must produce
    // byte-identical buffers. (The sidebar Projects tab DOES read
    // `SystemTime::now()` for relative timestamps; that sits outside the
    // stage surface and is recorded in the SF4.3 evidence.) This freezes
    // determinism for BOTH stage surfaces through the real Compositor.
    #[test]
    fn surface_render_is_deterministic_for_identical_state() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "herdr-render-determinism-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).expect("create temp root");
        std::fs::write(root.join("00.txt"), b"x").expect("write fixture entry");

        let draw = |app: &crate::app::state::AppState| {
            let mut terminal = Terminal::new(TestBackend::new(100, 30)).expect("test terminal");
            terminal
                .draw(|frame| render(app, frame))
                .expect("render frame");
            terminal.backend().buffer().clone()
        };

        let mut app = crate::app::state::AppState::test_new();
        let mut workspace = Workspace::test_new("render-determinism");
        workspace.test_split(ratatui::layout::Direction::Horizontal);
        app.workspaces = vec![workspace];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.mobile_width_threshold = 0;

        compute_view(&mut app, Rect::new(0, 0, 100, 30));
        assert_eq!(
            draw(&app),
            draw(&app),
            "the terminal surface must render byte-identically for identical state"
        );

        app.try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&root)))
            .expect("Files activation");
        compute_view(&mut app, Rect::new(0, 0, 100, 30));
        assert_eq!(
            draw(&app),
            draw(&app),
            "the Files surface must render byte-identically for identical state"
        );

        std::fs::remove_dir_all(&root).expect("remove temp root");
    }

    // SF4.3-04 characterization: `render` takes `&AppState`, so direct
    // mutation is compile-impossible; the remaining hazard is interior
    // mutability reached through the runtime registry. Freeze the observable
    // stage state across a render of both surfaces.
    #[test]
    fn surface_render_does_not_mutate_app_state() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "herdr-render-no-mutation-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).expect("create temp root");
        std::fs::write(root.join("00.txt"), b"x").expect("write fixture entry");

        let mut app = crate::app::state::AppState::test_new();
        let mut workspace = Workspace::test_new("render-no-mutation");
        workspace.test_split(ratatui::layout::Direction::Horizontal);
        app.workspaces = vec![workspace];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.mobile_width_threshold = 0;

        let render_once = |app: &crate::app::state::AppState| {
            let mut terminal = Terminal::new(TestBackend::new(100, 30)).expect("test terminal");
            terminal
                .draw(|frame| render(app, frame))
                .expect("render frame");
        };
        let snapshot = |app: &crate::app::state::AppState| {
            (
                app.stage,
                app.mode,
                app.view.terminal_area,
                app.view.sidebar_rect,
                app.view.pane_infos.len(),
                app.view.split_borders.len(),
                app.view.file_manager_row_areas.len(),
                app.file_manager
                    .as_ref()
                    .map(|fm| (fm.cursor, fm.entries.len())),
                app.workspace_scroll,
                app.tab_scroll,
            )
        };

        compute_view(&mut app, Rect::new(0, 0, 100, 30));
        let before = snapshot(&app);
        render_once(&app);
        assert_eq!(
            snapshot(&app),
            before,
            "rendering the terminal surface must not change observable state"
        );

        app.try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&root)))
            .expect("Files activation");
        compute_view(&mut app, Rect::new(0, 0, 100, 30));
        let before = snapshot(&app);
        render_once(&app);
        assert_eq!(
            snapshot(&app),
            before,
            "rendering the Files surface must not change observable state"
        );

        std::fs::remove_dir_all(&root).expect("remove temp root");
    }

    // SF4.3-05 characterization: valid RED refuted by source — the cached
    // `ShellView` already returns the previous projection when the geometry
    // key is unchanged (SF2.4). This freezes the retained path END-TO-END
    // through `compute_view`: a dirty terminal row triggers a recompute with
    // identical geometry, and that recompute must keep the exact cached
    // shell generation instead of re-solving the shell per PTY row. The
    // control phase proves the generation DOES advance when geometry truly
    // changes, so the pin cannot pass vacuously.
    #[test]
    fn terminal_dirty_row_keeps_retained_path_with_static_shell() {
        let mut app = crate::app::state::AppState::test_new();
        let mut workspace = Workspace::test_new("retained-shell");
        workspace.test_split(ratatui::layout::Direction::Horizontal);
        app.workspaces = vec![workspace];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.mobile_width_threshold = 0;
        let area = Rect::new(0, 0, 100, 30);

        compute_view(&mut app, area);
        let retained_generation = app.view.shell.generation;
        let retained_regions = app.view.shell.regions.clone();

        // A dirty-row frame recomputes with identical geometry: the shell
        // stays on the retained path.
        for _ in 0..3 {
            compute_view(&mut app, area);
            assert_eq!(
                app.view.shell.generation, retained_generation,
                "a static shell must keep its cached generation across dirty-row recomputes"
            );
            assert_eq!(app.view.shell.regions, retained_regions);
        }

        // Control: a real geometry change leaves the retained path exactly
        // once.
        compute_view(&mut app, Rect::new(0, 0, 101, 30));
        assert_eq!(
            app.view.shell.generation,
            retained_generation + 1,
            "control: changed geometry must advance the shell generation"
        );
    }

    // SF4.3-06: the stage renderer is chosen by the TYPED Stage authority
    // (`stage.surface_view()`), not by the legacy `file_manager.is_some()`
    // boolean. The adversarial divergent state (Files domain state present
    // while the typed stage says TerminalWorkspace) pins which source wins:
    // exactly one typed surface may be rendered and actionable.
    #[test]
    fn stage_renderer_follows_typed_surface_authority() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "herdr-typed-renderer-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).expect("create temp root");
        std::fs::write(root.join("MARKER_ENTRY.txt"), b"x").expect("write marker");

        let text_of = |app: &crate::app::state::AppState| {
            let mut terminal = Terminal::new(TestBackend::new(100, 30)).expect("test terminal");
            terminal
                .draw(|frame| render(app, frame))
                .expect("render frame");
            let buffer = terminal.backend().buffer().clone();
            let mut text = String::new();
            for y in 0..30 {
                for x in 0..100 {
                    text.push(buffer[(x, y)].symbol().chars().next().unwrap_or(' '));
                }
            }
            text
        };

        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("typed-renderer")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.mobile_width_threshold = 0;

        // Control: the aligned NativeFiles state renders the Files surface.
        app.try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&root)))
            .expect("Files activation");
        compute_view(&mut app, Rect::new(0, 0, 100, 30));
        assert!(
            text_of(&app).contains("MARKER_ENTRY.txt"),
            "control: the aligned Files state must render the Files surface"
        );

        // Adversarial divergence: Files domain state present while the typed
        // stage says TerminalWorkspace. The TYPED authority must win — the
        // hidden Files surface may not be rendered.
        app.stage.close_files();
        compute_view(&mut app, Rect::new(0, 0, 100, 30));
        assert!(
            !text_of(&app).contains("MARKER_ENTRY.txt"),
            "the typed stage authority must choose the renderer, not the legacy boolean"
        );

        std::fs::remove_dir_all(&root).expect("remove temp root");
    }

    // A2 integration: when the file manager is open, the base layer renders the
    // directory list in the center (CenterContent) instead of the terminal panes.
    #[test]
    fn open_file_manager_renders_directory_list_in_center() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "herdr-ui-fm-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).expect("create temp root");
        std::fs::write(root.join("MARKER_FILE.txt"), b"x").expect("write marker");

        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&root)))
            .expect("Files activation");

        compute_view(&mut app, Rect::new(0, 0, 100, 30));
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        terminal.draw(|frame| render(&app, frame)).unwrap();
        let buffer = terminal.backend().buffer().clone();

        let mut text = String::new();
        for y in 0..30 {
            for x in 0..100 {
                text.push(buffer[(x, y)].symbol().chars().next().unwrap_or(' '));
            }
        }
        assert!(
            text.contains("MARKER_FILE.txt"),
            "open file manager shows its entries in the center"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    // SF6.1 target (replaces the SF1 curtain characterization): an active
    // NativeFiles surface owns EXACTLY the WorkspaceStage — Files content is
    // clipped to the stage, the terminal-app chrome (tab bar) is absent, the
    // sidebar stays separately rendered, no terminal pane text leaks, and the
    // server-owned terminal runtime survives untouched.
    #[tokio::test]
    async fn files_renders_as_native_workspace_stage_surface() {
        use std::sync::atomic::{AtomicU64, Ordering};

        struct FixtureRoot(std::path::PathBuf);

        impl Drop for FixtureRoot {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }

        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "herdr-ui-fm-curtain-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _fixture_root = FixtureRoot(root.clone());
        std::fs::create_dir_all(&root).expect("create curtain fixture root");
        std::fs::write(root.join("FM_VISIBLE"), b"x").expect("write file manager curtain marker");

        let mut app = crate::app::state::AppState::test_new();
        let workspace = Workspace::test_new("one");
        let pane_id = workspace.tabs[0].root_pane;
        let terminal_id = workspace
            .terminal_id(pane_id)
            .expect("root pane terminal identity")
            .clone();
        app.workspaces = vec![workspace];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&root)))
            .expect("Files activation");

        let mut terminal_runtimes = crate::terminal::TerminalRuntimeRegistry::new();
        assert!(
            terminal_runtimes
                .insert(
                    terminal_id.clone(),
                    crate::terminal::TerminalRuntime::test_with_screen_bytes(
                        100,
                        30,
                        b"TERMINAL_SURFACE_SHOULD_BE_HIDDEN",
                    ),
                )
                .is_none(),
            "fixture inserts exactly one runtime"
        );
        let runtime_count = terminal_runtimes.len();

        let area = Rect::new(0, 0, 100, 30);

        // Control: with the terminal surface active the tab-bar chrome is a
        // real, non-empty region the Files surface must later reclaim.
        app.close_file_manager();
        compute_view_with_runtime_registry(&mut app, &terminal_runtimes, area);
        assert!(
            !app.view.tab_bar_rect.is_empty(),
            "control: the terminal surface owns a visible tab bar"
        );

        app.try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&root)))
            .expect("Files reactivation");
        compute_view_with_runtime_registry(&mut app, &terminal_runtimes, area);
        let stage = app
            .view
            .shell
            .regions
            .get(crate::ui::shell::RegionId::WorkspaceStage);
        assert_eq!(
            app.view.terminal_area, stage,
            "active NativeFiles must own exactly the WorkspaceStage"
        );
        assert_eq!(
            app.view.tab_bar_rect,
            Rect::default(),
            "the terminal-app tab bar is not part of the Files surface"
        );
        assert!(
            !app.view.sidebar_rect.is_empty(),
            "the sidebar remains a separately rendered shell region"
        );

        let mut terminal =
            Terminal::new(TestBackend::new(area.width, area.height)).expect("files stage terminal");
        terminal
            .draw(|frame| render_with_runtime_registry(&app, &terminal_runtimes, frame))
            .expect("files stage render");
        let stage_text = buffer_rect_text(terminal.backend().buffer(), stage);
        assert!(
            stage_text.contains("FM_VISIBLE"),
            "Files content must occupy the stage; rendered stage: {stage_text:?}"
        );
        assert!(
            !stage_text.contains("TERMINAL_SURFACE_SHOULD_BE_HIDDEN"),
            "terminal pane content must be absent under the Files surface"
        );
        assert_eq!(terminal_runtimes.len(), runtime_count);
        assert!(
            terminal_runtimes.get(&terminal_id).is_some(),
            "Files rendering must preserve the exact terminal runtime"
        );

        // Collapsed sidebar: the wider stage stays fully owned by Files.
        app.sidebar_collapsed = true;
        compute_view_with_runtime_registry(&mut app, &terminal_runtimes, area);
        let collapsed_stage = app
            .view
            .shell
            .regions
            .get(crate::ui::shell::RegionId::WorkspaceStage);
        assert!(collapsed_stage.width > stage.width);
        assert_eq!(app.view.terminal_area, collapsed_stage);
        app.sidebar_collapsed = false;

        // Tiny terminal: degenerate geometry stays bounded and panic-free.
        let tiny = Rect::new(0, 0, 12, 4);
        compute_view_with_runtime_registry(&mut app, &terminal_runtimes, tiny);
        let mut tiny_terminal =
            Terminal::new(TestBackend::new(tiny.width, tiny.height)).expect("tiny terminal");
        tiny_terminal
            .draw(|frame| render_with_runtime_registry(&app, &terminal_runtimes, frame))
            .expect("tiny files stage render");

        // Mobile keeps its explicit dedicated full-width contract: the shell
        // projects no desktop regions and Files fills the mobile content
        // area below the header.
        app.mobile_width_threshold = 500;
        compute_view_with_runtime_registry(&mut app, &terminal_runtimes, area);
        assert_eq!(app.view.layout, crate::app::state::ViewLayout::Mobile);
        assert!(app
            .view
            .shell
            .regions
            .get(crate::ui::shell::RegionId::WorkspaceStage)
            .is_empty());
        assert!(!app.view.terminal_area.is_empty());
    }

    fn native_fm_visual_composition_app(root: &std::path::Path) -> AppState {
        use crate::app::state::{
            FileManagerOperationKind, FileManagerOperationState, FileManagerOperationStatus,
            FileManagerSidebarIcon, FileManagerSidebarItem, FileManagerSidebarModel, SidebarTab,
        };

        let mut file_manager = crate::fm::FmState::new(root);
        assert!(file_manager.replace_selection(0));
        let mut app = AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.palette = crate::app::state::Palette::catppuccin_latte();
        app.sidebar_tab = SidebarTab::Files;
        app.file_manager_sidebar = FileManagerSidebarModel::from_sources(
            vec![FileManagerSidebarItem {
                label: "Visual fixture".into(),
                path: root.to_path_buf(),
                icon: FileManagerSidebarIcon::Home,
                accessible: true,
                ejectable: false,
            }],
            Vec::new(),
            Vec::new(),
        );
        app.try_open_file_manager_with(|_| Some(file_manager))
            .expect("Files activation");
        app.file_manager_operation = Some(FileManagerOperationState {
            generation: 1,
            kind: FileManagerOperationKind::Copy,
            destination_directory: root.to_path_buf(),
            total_items: 1,
            completed_items: 0,
            failed_items: 0,
            status: FileManagerOperationStatus::Running,
            items: Vec::new(),
        });
        app
    }

    fn render_full_frame_for_test(app: &AppState, area: Rect) -> ratatui::buffer::Buffer {
        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height))
            .expect("visual composition test terminal");
        terminal
            .draw(|frame| render(app, frame))
            .expect("visual composition should render");
        terminal.backend().buffer().clone()
    }

    fn buffer_rect_text(buffer: &ratatui::buffer::Buffer, area: Rect) -> String {
        (area.y..area.bottom())
            .flat_map(|y| (area.x..area.right()).map(move |x| (x, y)))
            .map(|(x, y)| buffer[(x, y)].symbol())
            .collect()
    }

    // TP-C6.4-VISUAL: expanded/collapsed desktop and responsive mobile layouts
    // compose the same exact FM state without stale sidebar or row authority.
    #[test]
    fn native_fm_composes_sidebar_breakpoints_and_status_across_full_frames() {
        use crate::app::state::ViewLayout;
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "herdr-ui-fm-visual-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(root.join("child")).expect("create visual child");
        std::fs::write(root.join("child").join("preview.txt"), b"preview")
            .expect("write visual preview");
        std::fs::write(root.join("peer.txt"), b"peer").expect("write visual peer");
        let mut app = native_fm_visual_composition_app(&root);

        let desktop = Rect::new(0, 0, 120, 24);
        compute_view(&mut app, desktop);
        assert_eq!(app.view.layout, ViewLayout::Desktop);
        let expanded_sidebar_width = app.view.sidebar_rect.width;
        let expanded = render_full_frame_for_test(&app, desktop);
        let center = buffer_rect_text(&expanded, app.view.terminal_area);
        for label in ["PARENT", "CURRENT", "PREVIEW", "copy 0/1", "Esc cancel"] {
            assert!(center.contains(label), "expanded center missing {label:?}");
        }
        let current_row = app
            .view
            .file_manager_sidebar_row_areas
            .first()
            .expect("expanded Files sidebar row")
            .rect;
        assert!((current_row.x..current_row.right())
            .any(|x| { expanded[(x, current_row.y)].bg == app.palette.accent }));
        let status_y = app.view.terminal_area.bottom() - 1;
        let status_x = (app.view.terminal_area.x..app.view.terminal_area.right())
            .find(|&x| expanded[(x, status_y)].symbol() == "c")
            .expect("running copy status");
        assert_eq!(expanded[(status_x, status_y)].fg, app.palette.yellow);

        app.sidebar_collapsed = true;
        compute_view(&mut app, desktop);
        assert_eq!(app.view.layout, ViewLayout::Desktop);
        assert!(app.view.sidebar_rect.width < expanded_sidebar_width);
        assert!(app.view.file_manager_sidebar_row_areas.is_empty());
        let collapsed = render_full_frame_for_test(&app, desktop);
        assert!(buffer_rect_text(&collapsed, app.view.terminal_area).contains("copy 0/1"));

        app.sidebar_collapsed = false;
        let mobile_two = Rect::new(0, 0, 33, 15);
        compute_view(&mut app, mobile_two);
        assert_eq!(app.view.layout, ViewLayout::Mobile);
        assert!(app.view.file_manager_sidebar_row_areas.is_empty());
        let two = buffer_rect_text(
            &render_full_frame_for_test(&app, mobile_two),
            app.view.terminal_area,
        );
        assert!(!two.contains("PARENT"));
        assert!(two.contains("CURRENT"));
        assert!(two.contains("PREVIEW"));
        assert!(two.contains("copy 0/1"));

        let mobile_one = Rect::new(0, 0, 20, 15);
        compute_view(&mut app, mobile_one);
        assert_eq!(app.view.layout, ViewLayout::Mobile);
        let one = buffer_rect_text(
            &render_full_frame_for_test(&app, mobile_one),
            app.view.terminal_area,
        );
        assert!(one.contains("CURRENT"));
        assert!(!one.contains("PARENT"));
        assert!(!one.contains("PREVIEW"));
        assert!(one.contains("copy 0/1"));

        std::fs::remove_dir_all(root).expect("remove visual composition fixture");
    }

    // TP-C6.4-VISUAL: context and destructive-modal overlays remain bounded and
    // paint above the composed FM without changing its prepared operation state.
    #[test]
    fn native_fm_context_and_delete_modal_compose_above_status_surface() {
        use crate::app::state::{
            ContextMenuKind, ContextMenuState, FileManagerContextMenuModel,
            FileManagerDeleteConfirmation, FileManagerDeleteConfirmationStage, MenuListState,
        };
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "herdr-ui-fm-overlay-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).expect("create overlay fixture");
        std::fs::write(root.join("selected.txt"), b"selected").expect("write overlay fixture");
        let mut app = native_fm_visual_composition_app(&root);
        let area = Rect::new(0, 0, 100, 24);
        compute_view(&mut app, area);
        let model = FileManagerContextMenuModel::from_action_bar(
            app.view
                .file_manager_action_bar
                .as_ref()
                .expect("prepared action bar"),
        )
        .expect("single-selection context model");
        app.context_menu = Some(ContextMenuState {
            kind: ContextMenuKind::File { model },
            x: app.view.terminal_area.x + 2,
            y: app.view.terminal_area.y + 2,
            list: MenuListState::new(0),
        });
        app.mode = Mode::ContextMenu;
        let context_rect = app.context_menu_rect().expect("bounded context rect");
        assert!(app.view.terminal_area.contains(context_rect.as_position()));
        let context = render_full_frame_for_test(&app, area);
        let context_text = buffer_rect_text(&context, context_rect);
        for label in ["Open", "Copy", "Rename", "Delete", "Send to Agent"] {
            assert!(context_text.contains(label), "context missing {label:?}");
        }
        assert_eq!(
            app.file_manager_operation
                .as_ref()
                .expect("running operation preserved")
                .status,
            crate::app::state::FileManagerOperationStatus::Running
        );

        app.mode = Mode::ConfirmFileDelete;
        app.file_manager_delete_confirmation = Some(FileManagerDeleteConfirmation {
            paths: vec![root.join("selected.txt")],
            stage: FileManagerDeleteConfirmationStage::ChooseAction,
        });
        let modal = render_full_frame_for_test(&app, area);
        let modal_text = buffer_rect_text(&modal, app.view.terminal_area);
        for label in [
            "Delete 1 selected item?",
            "move to trash",
            "delete permanently",
            "cancel",
        ] {
            assert!(modal_text.contains(label), "delete modal missing {label:?}");
        }

        std::fs::remove_dir_all(root).expect("remove overlay composition fixture");
    }

    #[test]
    fn copy_feedback_offset_only_increases_when_toast_rect_overlaps() {
        let area = Rect::new(0, 0, 80, 24);
        let feedback = crate::app::state::CopyFeedback {
            message: "copied to clipboard".into(),
        };
        let toast = crate::app::state::ToastNotification {
            kind: crate::app::state::ToastKind::Finished,
            title: "pi finished".into(),
            context: "workspace · 1".into(),
            position: None,
            target: None,
        };

        let bottom_right_toast = toast_notification_rect(
            area,
            &toast,
            false,
            crate::config::ToastHerdrPosition::BottomRight,
        );
        assert_eq!(
            copy_feedback_offset_for_toast(
                area,
                &feedback,
                0,
                crate::config::ToastClipboardPosition::TopCenter,
                bottom_right_toast,
            ),
            0
        );

        let bottom_center_toast = Rect::new(28, 21, 24, 3);
        assert_eq!(
            copy_feedback_offset_for_toast(
                area,
                &feedback,
                0,
                crate::config::ToastClipboardPosition::BottomCenter,
                bottom_center_toast,
            ),
            bottom_center_toast.height
        );
    }

    #[tokio::test]
    async fn focused_pane_cursor_wins_during_terminal_render() {
        let mut app = crate::app::state::AppState::test_new();
        let mut ws = Workspace::test_new("test");
        let first_pane = ws.tabs[0].root_pane;
        let second_pane = ws.test_split(ratatui::layout::Direction::Horizontal);

        ws.insert_test_runtime(
            first_pane,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(20, 5, b"left"),
        );
        ws.insert_test_runtime(
            second_pane,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(20, 5, b"r\r\nb"),
        );
        ws.tabs[0].layout.focus_pane(first_pane);

        app.workspaces = vec![ws];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;

        compute_view(&mut app, Rect::new(0, 0, 80, 20));
        let focused = app
            .view
            .pane_infos
            .iter()
            .find(|info| info.id == first_pane)
            .expect("focused pane info");

        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(&app, frame)).unwrap();

        terminal
            .backend_mut()
            .assert_cursor_position((focused.inner_rect.x + 4, focused.inner_rect.y));
    }

    #[test]
    fn mobile_width_uses_header_and_full_width_terminal() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;

        compute_view(&mut app, Rect::new(0, 0, 44, 20));

        assert_eq!(app.view.layout, ViewLayout::Mobile);
        assert_eq!(app.view.sidebar_rect, Rect::default());
        assert_eq!(app.view.tab_bar_rect, Rect::default());
        assert_eq!(app.view.mobile_header_rect, Rect::new(0, 0, 44, 2));
        assert_eq!(app.view.terminal_area, Rect::new(0, 2, 44, 18));
        assert_eq!(app.view.mobile_menu_hit_area.height, 2);
        assert_eq!(
            app.view.mobile_menu_hit_area.x + app.view.mobile_menu_hit_area.width,
            44
        );
    }

    #[test]
    fn mobile_config_diagnostic_keeps_command_visible() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.config_diagnostic = Some("config.toml:100:10; herdr config check".into());

        let area = Rect::new(0, 0, 44, 20);
        compute_view(&mut app, area);
        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
        terminal.draw(|frame| render(&app, frame)).unwrap();
        let row = buffer_row_text(terminal.backend().buffer(), area, app.view.terminal_area.y);

        assert!(row.contains("config.toml:100:10"), "{row}");
        assert!(row.contains("herdr config check"), "{row}");
    }

    #[test]
    fn desktop_toast_hit_area_uses_full_frame_not_terminal_area() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.toast_config.herdr.position = crate::config::ToastHerdrPosition::TopLeft;
        app.toast = Some(crate::app::state::ToastNotification {
            kind: crate::app::state::ToastKind::Finished,
            title: "pi finished".into(),
            context: "one".into(),
            position: None,
            target: None,
        });

        compute_view(&mut app, Rect::new(0, 0, 100, 20));

        assert_eq!(app.view.layout, ViewLayout::Desktop);
        assert!(app.view.terminal_area.x > 0);
        assert_eq!(app.view.toast_hit_area.x, 0);
        assert_eq!(app.view.toast_hit_area.y, 0);
    }

    #[test]
    fn desktop_toast_hit_area_still_offsets_for_config_diagnostic() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.config_diagnostic = Some("config warning".into());
        app.toast_config.herdr.position = crate::config::ToastHerdrPosition::TopLeft;
        app.toast = Some(crate::app::state::ToastNotification {
            kind: crate::app::state::ToastKind::Finished,
            title: "pi finished".into(),
            context: "one".into(),
            position: None,
            target: None,
        });

        compute_view(&mut app, Rect::new(0, 0, 100, 20));

        assert_eq!(app.view.toast_hit_area.x, 0);
        assert_eq!(app.view.toast_hit_area.y, 1);
    }

    #[test]
    fn configured_mobile_width_threshold_controls_layout_switch() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;

        compute_view(&mut app, Rect::new(0, 0, 80, 20));
        assert_eq!(app.view.layout, ViewLayout::Desktop);

        app.mobile_width_threshold = 90;
        compute_view(&mut app, Rect::new(0, 0, 80, 20));
        assert_eq!(app.view.layout, ViewLayout::Mobile);
        assert_eq!(app.view.mobile_header_rect, Rect::new(0, 0, 80, 2));
        assert_eq!(app.view.terminal_area, Rect::new(0, 2, 80, 18));
    }

    #[test]
    fn hide_tab_bar_when_single_tab_toggles_geometry_with_tab_count() {
        let mut app = crate::app::state::AppState::test_new();
        app.hide_tab_bar_when_single_tab = true;
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;

        compute_view(&mut app, Rect::new(0, 0, 80, 20));
        let single_tab_terminal_area = app.view.terminal_area;
        assert_eq!(app.view.tab_bar_rect, Rect::default());
        assert_eq!(single_tab_terminal_area, Rect::new(26, 0, 54, 20));
        assert!(app.view.tab_hit_areas.is_empty());
        assert_eq!(app.view.new_tab_hit_area, Rect::default());

        app.workspaces[0].test_add_tab(Some("logs"));
        compute_view(&mut app, Rect::new(0, 0, 80, 20));

        assert_eq!(app.view.tab_bar_rect, Rect::new(26, 0, 54, 1));
        assert_eq!(app.view.terminal_area, Rect::new(26, 1, 54, 19));
        assert_eq!(app.view.tab_hit_areas.len(), 2);
        assert!(app.view.tab_hit_areas.iter().all(|rect| rect.width > 0));
        assert!(app.view.new_tab_hit_area.width > 0);

        assert!(app.workspaces[0].close_tab(1));
        compute_view(&mut app, Rect::new(0, 0, 80, 20));

        assert_eq!(app.view.terminal_area, single_tab_terminal_area);
        assert_eq!(app.view.tab_bar_rect, Rect::default());
        assert!(app.view.tab_hit_areas.is_empty());
        assert_eq!(app.view.new_tab_hit_area, Rect::default());
    }

    #[tokio::test]
    async fn hide_tab_bar_when_single_tab_resizes_background_tabs_per_workspace() {
        let mut app = crate::app::state::AppState::test_new();
        app.hide_tab_bar_when_single_tab = true;

        let mut one_tab_workspace = Workspace::test_new("one");
        let one_tab_pane = one_tab_workspace.tabs[0].root_pane;
        let one_tab_runtime = crate::terminal::TerminalRuntime::test_with_screen_bytes(10, 5, b"");
        one_tab_workspace.tabs[0]
            .runtimes
            .insert(one_tab_pane, one_tab_runtime);

        let mut two_tab_workspace = Workspace::test_new("two");
        let background_tab = two_tab_workspace.test_add_tab(Some("logs"));
        let two_tab_pane = two_tab_workspace.tabs[background_tab].root_pane;
        let two_tab_runtime = crate::terminal::TerminalRuntime::test_with_screen_bytes(10, 5, b"");
        two_tab_workspace.tabs[background_tab]
            .runtimes
            .insert(two_tab_pane, two_tab_runtime);

        app.workspaces = vec![one_tab_workspace, two_tab_workspace];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;

        compute_view(&mut app, Rect::new(0, 0, 80, 20));

        let one_tab_size = app.workspaces[0].tabs[0].runtimes[&one_tab_pane].current_size();
        let two_tab_size =
            app.workspaces[1].tabs[background_tab].runtimes[&two_tab_pane].current_size();
        assert_eq!(one_tab_size, (20, 53));
        assert_eq!(two_tab_size, (19, 53));
    }

    #[tokio::test]
    async fn mobile_background_tabs_use_mobile_terminal_area() {
        let mut app = crate::app::state::AppState::test_new();

        let mut workspace = Workspace::test_new("mobile");
        let background_tab = workspace.test_add_tab(Some("logs"));
        let background_pane = workspace.tabs[background_tab].root_pane;
        let runtime = crate::terminal::TerminalRuntime::test_with_screen_bytes(10, 5, b"");
        workspace.tabs[background_tab]
            .runtimes
            .insert(background_pane, runtime);

        app.workspaces = vec![workspace];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;

        compute_view(&mut app, Rect::new(0, 0, 44, 20));

        assert_eq!(app.view.layout, ViewLayout::Mobile);
        assert_eq!(app.view.terminal_area, Rect::new(0, 2, 44, 18));
        assert_eq!(
            app.workspaces[0].tabs[background_tab].runtimes[&background_pane].current_size(),
            (18, 43)
        );
    }

    #[test]
    fn product_announcement_renders_above_config_diagnostic() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::ProductAnnouncement;
        app.product_announcement = Some(crate::app::state::ProductAnnouncementState {
            version: "0.6.0".into(),
            id: "keybinding-v2".into(),
            title: "Keybinding syntax changed".into(),
            body: "### Update\n- Body".into(),
            scroll: 0,
            preview: false,
        });
        app.config_diagnostic = Some(
            "unsafe direct keybinding: keys.new_workspace = \"n\"\nunsafe direct keybinding: keys.new_tab = \"c\""
                .into(),
        );

        let area = Rect::new(0, 0, 44, 20);
        compute_view(&mut app, area);

        let backend = TestBackend::new(area.width, area.height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(&app, frame)).unwrap();
        let buffer = terminal.backend().buffer();

        let popup = centered_popup_rect(
            area,
            PRODUCT_ANNOUNCEMENT_MODAL_SIZE.0,
            PRODUCT_ANNOUNCEMENT_MODAL_SIZE.1,
        )
        .expect("announcement popup");
        let title_row = popup.y + 1;
        let row = buffer_row_text(buffer, Rect::new(0, title_row, area.width, 1), title_row);

        assert!(row.contains("Keybinding syntax changed"));
        assert!(!row.contains("config warning"));
    }

    #[test]
    fn compute_view_clamps_sidebar_width_to_configured_max() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.sidebar_max_width = 30;
        app.sidebar_width = 999;

        compute_view(&mut app, Rect::new(0, 0, 100, 20));

        assert_eq!(app.view.sidebar_rect.width, 30);
    }

    #[test]
    fn compute_view_clamps_sidebar_width_to_configured_min() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.sidebar_min_width = 22;
        app.sidebar_width = 5;

        compute_view(&mut app, Rect::new(0, 0, 100, 20));

        assert_eq!(app.view.sidebar_rect.width, 22);
    }

    #[test]
    fn hidden_collapsed_sidebar_uses_full_width_terminal_area() {
        let mut app = crate::app::state::AppState::test_new();
        app.sidebar_collapsed = true;
        app.sidebar_collapsed_mode = crate::config::SidebarCollapsedModeConfig::Hidden;
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;

        compute_view(&mut app, Rect::new(0, 0, 80, 20));

        assert_eq!(app.view.sidebar_rect, Rect::new(0, 0, 0, 20));
        assert_eq!(app.view.tab_bar_rect, Rect::new(0, 0, 80, 1));
        assert_eq!(app.view.terminal_area, Rect::new(0, 1, 80, 19));
        assert!(app.view.workspace_card_areas.is_empty());

        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(&app, frame)).unwrap();
    }

    #[test]
    fn collapsed_sidebar_keeps_active_workspace_highlight_in_terminal_mode() {
        let mut app = crate::app::state::AppState::test_new();
        app.sidebar_collapsed = true;
        app.workspaces = vec![Workspace::test_new("one"), Workspace::test_new("two")];
        app.active = Some(1);
        app.selected = 0;
        app.mode = Mode::Terminal;

        compute_view(&mut app, Rect::new(0, 0, 80, 20));

        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(&app, frame)).unwrap();
        let buffer = terminal.backend().buffer();

        let (ws_area, _, _) = collapsed_sidebar_sections(app.view.sidebar_rect);
        let active_row = ws_area.y + 1;
        let active_style = buffer[(ws_area.x, active_row)].style();

        assert_eq!(active_style.bg, Some(app.palette.surface_dim));
    }

    #[test]
    fn expanded_sidebar_workspace_rows_show_state_before_name_without_numbers() {
        let mut app = crate::app::state::AppState::test_new();
        let mut ws = Workspace::test_new("one");
        let repo = temp_git_repo("main");
        ws.identity_cwd = repo.clone();
        let root_pane = ws.tabs[0].root_pane;
        ws.refresh_git_ahead_behind();

        app.workspaces = vec![ws];
        app.ensure_test_terminals();
        let root_terminal_id = app.workspaces[0].tabs[0].panes[&root_pane]
            .attached_terminal_id
            .clone();
        app.terminals.get_mut(&root_terminal_id).unwrap().cwd = repo.clone();
        app.selected = 0;
        app.mode = Mode::Navigate;

        compute_view(&mut app, Rect::new(0, 0, 80, 20));

        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(&app, frame)).unwrap();
        let buffer = terminal.backend().buffer();

        let card = app.view.workspace_card_areas[0].rect;
        let line1 = buffer_row_text(buffer, card, card.y);
        let line2 = buffer_row_text(buffer, card, card.y + 1);

        assert!(line1.starts_with(" · one"));
        assert!(!line1.contains("1 one"));
        assert_eq!(line2, "   main");

        std::fs::remove_dir_all(repo).ok();
    }

    #[test]
    fn tab_bar_dims_auto_named_tabs_and_emphasizes_custom_tabs() {
        let mut app = crate::app::state::AppState::test_new();
        let mut ws = Workspace::test_new("test");
        let custom_tab = ws.test_add_tab(Some("logs"));
        ws.switch_tab(custom_tab);

        app.workspaces = vec![ws];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;

        compute_view(&mut app, Rect::new(0, 0, 80, 20));

        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(&app, frame)).unwrap();
        let buffer = terminal.backend().buffer();

        let auto_rect = app.view.tab_hit_areas[0];
        let custom_rect = app.view.tab_hit_areas[1];
        let auto_style = buffer[(auto_rect.x + 1, auto_rect.y)].style();
        let custom_style = buffer[(custom_rect.x + 1, custom_rect.y)].style();

        assert_eq!(auto_style.fg, Some(app.palette.overlay0));
        assert!(auto_style.add_modifier.contains(Modifier::DIM));
        assert_eq!(custom_style.fg, Some(app.palette.panel_bg));
        assert!(custom_style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn tab_bar_uses_surface_dim_when_panel_background_resets() {
        let mut app = crate::app::state::AppState::test_new();
        let mut ws = Workspace::test_new("test");
        let custom_tab = ws.test_add_tab(Some("logs"));
        ws.switch_tab(custom_tab);

        app.palette.panel_bg = Color::Reset;
        app.workspaces = vec![ws];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;

        compute_view(&mut app, Rect::new(0, 0, 80, 20));

        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(&app, frame)).unwrap();
        let buffer = terminal.backend().buffer();

        let custom_rect = app.view.tab_hit_areas[1];
        let custom_style = buffer[(custom_rect.x + 1, custom_rect.y)].style();

        assert_eq!(custom_style.bg, Some(app.palette.accent));
        assert_eq!(custom_style.fg, Some(app.palette.surface_dim));
        assert!(custom_style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn new_tab_button_tracks_rightmost_tab_when_tabs_fit() {
        let mut app = crate::app::state::AppState::test_new();
        let mut ws = Workspace::test_new("test");
        ws.test_add_tab(Some("logs"));

        app.workspaces = vec![ws];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;

        compute_view(&mut app, Rect::new(0, 0, 80, 20));

        let last_visible = app
            .view
            .tab_hit_areas
            .iter()
            .rev()
            .find(|rect| rect.width > 0)
            .copied()
            .expect("last visible tab");

        assert_eq!(
            app.view.new_tab_hit_area.x,
            last_visible.x + last_visible.width
        );
    }

    #[test]
    fn tab_bar_shows_scroll_controls_when_tabs_overflow() {
        let mut app = crate::app::state::AppState::test_new();
        let mut ws = Workspace::test_new("test");
        for name in ["alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta"] {
            ws.test_add_tab(Some(name));
        }

        app.workspaces = vec![ws];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.tab_scroll_follow_active = false;
        app.tab_scroll = 2;

        compute_view(&mut app, Rect::new(0, 0, 65, 20));

        assert!(app.view.tab_scroll_left_hit_area.width > 0);
        assert!(app.view.tab_scroll_right_hit_area.width > 0);
        assert_eq!(app.view.tab_hit_areas[0].width, 0);
        assert_eq!(app.view.tab_hit_areas[1].width, 0);
        assert!(app.view.tab_hit_areas[2].width > 0);
        assert!(app.view.new_tab_hit_area.width > 0);

        let last_visible = app
            .view
            .tab_hit_areas
            .iter()
            .rev()
            .find(|rect| rect.width > 0)
            .copied()
            .expect("last visible tab");

        assert_eq!(
            app.view.tab_scroll_right_hit_area.x,
            last_visible.x + last_visible.width
        );
        assert_eq!(
            app.view.new_tab_hit_area.x,
            app.view.tab_scroll_right_hit_area.x + app.view.tab_scroll_right_hit_area.width
        );
    }

    #[test]
    fn tab_bar_clamps_manual_scroll_at_last_visible_tab() {
        let mut app = crate::app::state::AppState::test_new();
        let mut ws = Workspace::test_new("test");
        for name in [
            "one", "two", "three", "four", "five", "six", "seven", "eight",
        ] {
            ws.test_add_tab(Some(name));
        }

        app.workspaces = vec![ws];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.tab_scroll_follow_active = false;
        app.tab_scroll = usize::MAX;

        compute_view(&mut app, Rect::new(0, 0, 65, 20));

        let last_idx = app.workspaces[0].tabs.len() - 1;
        assert!(app.view.tab_hit_areas[last_idx].width > 0);
        let clamped_scroll = app.tab_scroll;

        app.scroll_tabs_right();

        assert_eq!(app.tab_scroll, clamped_scroll);
        assert!(app.view.tab_hit_areas[last_idx].width > 0);
    }

    #[test]
    fn pane_scrollbar_rect_uses_reserved_rightmost_column() {
        let info = PaneInfo {
            id: crate::layout::PaneId::from_raw(1),
            rect: Rect::new(0, 0, 12, 8),
            inner_rect: Rect::new(1, 1, 9, 6),
            scrollbar_rect: Some(Rect::new(10, 1, 1, 6)),
            borders: ratatui::widgets::Borders::ALL,
            is_focused: true,
        };

        assert_eq!(pane_scrollbar_rect(&info), Some(Rect::new(10, 1, 1, 6)));
    }

    #[tokio::test]
    async fn compute_view_reserves_terminal_column_when_pane_scrollbar_is_visible() {
        let mut app = crate::app::state::AppState::test_new();
        let mut ws = Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        ws.insert_test_runtime(
            pane_id,
            crate::terminal::TerminalRuntime::test_with_scrollback_bytes(
                12,
                4,
                4096,
                b"000000000000\r\n111111111111\r\n222222222222\r\n333333333333\r\n444444444444\r\n",
            ),
        );

        app.workspaces = vec![ws];
        app.active = Some(0);
        app.selected = 0;

        compute_view(&mut app, Rect::new(0, 0, 40, 12));

        let info = app.view.pane_infos.first().expect("pane info");
        assert_eq!(info.inner_rect.width + 1, app.view.terminal_area.width);
        assert_eq!(
            info.scrollbar_rect,
            Some(Rect::new(
                info.inner_rect.x + info.inner_rect.width,
                info.inner_rect.y,
                1,
                info.inner_rect.height,
            ))
        );
    }

    #[test]
    fn scrollbar_stays_hidden_without_scrollback() {
        let metrics = crate::pane::ScrollMetrics {
            offset_from_bottom: 0,
            max_offset_from_bottom: 0,
            viewport_rows: 5,
        };

        assert!(!should_show_scrollbar(metrics));
    }

    #[test]
    fn scrollbar_shows_with_scrollback() {
        let metrics = crate::pane::ScrollMetrics {
            offset_from_bottom: 0,
            max_offset_from_bottom: 20,
            viewport_rows: 5,
        };

        assert!(should_show_scrollbar(metrics));
    }

    #[test]
    fn scrollbar_thumb_reaches_bottom_when_scrolled_to_bottom() {
        let metrics = crate::pane::ScrollMetrics {
            offset_from_bottom: 0,
            max_offset_from_bottom: 20,
            viewport_rows: 5,
        };
        let track = Rect::new(9, 4, 1, 5);

        let thumb = scrollbar_thumb(metrics, track).expect("thumb");
        assert_eq!(thumb.top + thumb.len, track.y + track.height);
    }

    #[test]
    fn scrollbar_offset_mapping_hits_top_middle_and_bottom() {
        let metrics = crate::pane::ScrollMetrics {
            offset_from_bottom: 0,
            max_offset_from_bottom: 20,
            viewport_rows: 5,
        };
        let track = Rect::new(9, 4, 1, 5);

        assert_eq!(scrollbar_offset_from_row(metrics, track, 4), 20);
        assert_eq!(scrollbar_offset_from_row(metrics, track, 6), 10);
        assert_eq!(scrollbar_offset_from_row(metrics, track, 8), 0);
    }

    #[test]
    fn dragging_from_current_thumb_row_preserves_offset() {
        let metrics = crate::pane::ScrollMetrics {
            offset_from_bottom: 7,
            max_offset_from_bottom: 20,
            viewport_rows: 5,
        };
        let track = Rect::new(9, 4, 1, 8);
        let thumb = scrollbar_thumb(metrics, track).expect("thumb");
        let row = thumb.top + thumb.len / 2;
        let grab = scrollbar_thumb_grab_offset(metrics, track, row).expect("grab");

        assert_eq!(scrollbar_offset_from_drag_row(metrics, track, row, grab), 7);
    }

    fn buffer_row_text(buffer: &ratatui::buffer::Buffer, area: Rect, row: u16) -> String {
        (area.x..area.x + area.width)
            .map(|x| buffer[(x, row)].symbol())
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    fn temp_git_repo(branch: &str) -> std::path::PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("unix time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("herdr-ui-test-{unique}"));
        std::fs::create_dir_all(root.join(".git")).expect("create .git dir");
        std::fs::write(
            root.join(".git/HEAD"),
            format!("ref: refs/heads/{branch}\n"),
        )
        .expect("write HEAD");
        root
    }

    #[test]
    fn prefix_mode_renders_prefix_indicator() {
        let mut app = crate::app::state::AppState::test_new();
        app.mode = Mode::Prefix;
        app.view.terminal_area = ratatui::layout::Rect::new(0, 0, 60, 4);
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(60, 4))
            .expect("test terminal");

        terminal
            .draw(|frame| render_prefix_overlay(&app, frame, app.view.terminal_area))
            .expect("draw prefix overlay");

        let rendered = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(rendered.contains("PREFIX"));
    }

    #[test]
    fn keybind_help_shows_unset_for_optional_actions() {
        let app = crate::app::state::AppState::test_new();
        let groups = keybind_help_groups(&app);

        let workspace_tab = groups
            .iter()
            .find(|(name, _)| *name == "workspaces / tabs")
            .expect("workspace tab group")
            .1
            .clone();
        let panes = groups
            .iter()
            .find(|(name, _)| *name == "panes")
            .expect("panes group")
            .1
            .clone();

        assert!(workspace_tab
            .iter()
            .any(|(key, label)| key == "unset" && label.as_ref() == "previous workspace"));
        assert!(workspace_tab
            .iter()
            .any(|(key, label)| key == "unset" && label.as_ref() == "next workspace"));
        assert!(workspace_tab
            .iter()
            .any(|(key, label)| key == "unset" && label.as_ref() == "previous agent"));
        assert!(workspace_tab
            .iter()
            .any(|(key, label)| key == "unset" && label.as_ref() == "next agent"));
        assert!(workspace_tab
            .iter()
            .any(|(key, label)| key == "unset" && label.as_ref() == "focus agent 1-9"));
        assert!(workspace_tab
            .iter()
            .any(|(key, label)| key == "unset" && label.as_ref() == "switch workspace 1-9"));
        assert!(panes
            .iter()
            .any(|(key, label)| key == "prefix+h" && label.as_ref() == "focus pane left"));
        assert!(panes
            .iter()
            .any(|(key, label)| key == "prefix+j" && label.as_ref() == "focus pane down"));
        assert!(panes
            .iter()
            .any(|(key, label)| key == "prefix+k" && label.as_ref() == "focus pane up"));
        assert!(panes
            .iter()
            .any(|(key, label)| key == "prefix+l" && label.as_ref() == "focus pane right"));
    }

    #[test]
    fn keybind_help_shows_custom_command_descriptions() {
        let mut app = crate::app::state::AppState::test_new();
        app.keybinds.custom_commands = vec![
            crate::config::CustomCommandKeybind {
                bindings: crate::config::ActionKeybinds::prefix("alt+g"),
                label: "prefix+alt+g".to_string(),
                command: "lazygit".to_string(),
                action: crate::config::CustomCommandAction::Pane,
                description: Some("open lazygit".to_string()),
            },
            crate::config::CustomCommandKeybind {
                bindings: crate::config::ActionKeybinds::prefix("alt+h"),
                label: "prefix+alt+h".to_string(),
                command: "echo hello".to_string(),
                action: crate::config::CustomCommandAction::Shell,
                description: None,
            },
        ];

        let groups = keybind_help_groups(&app);
        let custom = groups
            .iter()
            .find(|(name, _)| *name == "custom")
            .expect("custom group")
            .1
            .clone();
        assert!(custom
            .iter()
            .any(|(key, label)| key == "prefix+alt+g" && label.as_ref() == "open lazygit"));
        assert!(custom
            .iter()
            .any(|(key, label)| key == "prefix+alt+h" && label.as_ref() == "custom command"));

        let rendered_help = keybind_help_lines(&app)
            .into_iter()
            .flat_map(|(_, line)| line.spans)
            .map(|span| span.content.into_owned())
            .collect::<Vec<_>>()
            .join("");
        assert!(rendered_help.contains("open lazygit"));
        assert!(rendered_help.contains("custom command"));
    }

    #[test]
    fn keybind_help_compacts_multiple_indexed_ranges() {
        let config: crate::config::Config = toml::from_str(
            r#"
[keys]
switch_tab = ["prefix+1..9", "alt+1..9"]
switch_workspace = "ctrl+1..9"
"#,
        )
        .expect("config parses");

        let mut app = crate::app::state::AppState::test_new();
        app.keybinds = config.keybinds();

        let workspace_tab = keybind_help_groups(&app)
            .into_iter()
            .find(|(name, _)| *name == "workspaces / tabs")
            .expect("workspace tab group")
            .1;

        let switch_tab_key = workspace_tab
            .iter()
            .find(|(_, label)| label.as_ref() == "switch tab 1-9")
            .map(|(key, _)| key.as_str())
            .expect("switch tab help entry");
        let switch_workspace_key = workspace_tab
            .iter()
            .find(|(_, label)| label.as_ref() == "switch workspace 1-9")
            .map(|(key, _)| key.as_str())
            .expect("switch workspace help entry");

        assert_eq!(switch_tab_key, "prefix+1..9 / alt+1..9");
        assert_eq!(switch_workspace_key, "ctrl+1..9");
    }
}
