use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Clear},
    Frame,
};

pub(crate) mod app_dock;
#[cfg(test)]
pub(crate) use sidebar::closed_agent_row_slots;

mod compose;
mod dialogs;
mod file_manager;
mod keybind_help;
mod menus;
mod mobile;
mod navigator;
mod onboarding;
mod panes;
mod preview_viewer;
mod release_notes;
mod scrollbar;
mod settings;
pub(crate) mod shell;
mod sidebar;
mod size_class;
mod status;
pub(crate) mod surface_host;
mod tab_surface;
mod tabs;
mod tailscale_send;
mod text;
#[cfg(test)]
pub(crate) mod visual_fixture;
pub(crate) mod widgets;

use self::dialogs::{
    render_confirm_close_overlay, render_file_delete_confirmation_overlay,
    render_new_linked_worktree_overlay, render_open_existing_worktree_overlay,
    render_remove_worktree_overlay, render_rename_overlay,
};
pub(crate) use self::file_manager::compute_file_manager_action_bar_model;
#[cfg(test)]
pub(crate) use self::file_manager::file_manager_preview_content_area;
pub(crate) use self::file_manager::locations::locations_drawer_content_area;
pub(crate) use self::file_manager::locations::FileManagerLocationsView;
use self::file_manager::locations::{
    project_file_manager_locations_view, FileManagerLocationsMode,
};
#[cfg(test)]
pub(crate) use self::file_manager::miller::project_miller_view;
pub(crate) use self::file_manager::miller::{
    miller_resize_column_is_live, MillerColumnKind, MillerColumnView, MillerViewSnapshot,
};
pub(crate) use self::file_manager::pdf_page_indicator_for;
pub(crate) use self::file_manager::send_header_target_paths;
#[cfg(test)]
pub(crate) use self::file_manager::trail_view::project_trail_view;
pub(crate) use self::file_manager::trail_view::{
    trail_row_at, trail_section_header_at, TrailRowView, TrailViewSnapshot,
};
pub(crate) use self::file_manager::zip_header_target_paths;
#[cfg(test)]
pub(crate) use self::file_manager::PdfPageIndicator;
use self::file_manager::{
    agent_attachment_picker_visible_rows, compute_agent_attachment_picker_row_areas,
    compute_file_manager_header_action_areas, render_agent_attachment_picker, render_file_manager,
    FileManagerRowGeometry,
};
use self::keybind_help::render_keybind_help_overlay;
use self::menus::{
    render_agent_colleague_picker, render_agent_reference_picker, render_bar_config_panel,
    render_context_menu, render_copy_mode_overlay, render_global_launcher_menu,
    render_navigate_overlay, render_prefix_overlay, render_resize_overlay,
};
use self::mobile::{
    compute_mobile_header_hit_areas, mobile_drawer_max_scroll_for_height, mobile_toast_banner_rect,
    render_mobile_drawer, render_mobile_header, render_mobile_toast_banner,
};
use self::navigator::render_navigator_overlay;
pub(crate) use self::onboarding::onboarding_welcome_continue_rect;
use self::onboarding::render_onboarding_overlay;
pub(crate) use self::panes::popup_pane_rects;
use self::panes::{render_empty, render_popup_pane, resize_popup_pane};
pub(crate) use self::preview_viewer::{preview_viewer_content_area, render_preview_viewer};
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
use self::shell::{RegionId, ShellGeometryKey};
use self::sidebar::{render_sidebar, render_sidebar_collapsed};
use self::status::{
    copy_feedback_rect, render_config_diagnostic, render_copy_feedback, render_toast_notification,
    toast_notification_rect,
};
pub(crate) use self::tab_surface::{
    compute_tab_surface, render_tab_surface, resize_tab_surface, TabSurfaceLayout,
};
use self::tabs::render_tab_bar;
pub(crate) use self::tailscale_send::{
    device_row_at, render_tailscale_send, tailscale_send_popup_rect,
};
pub(crate) use self::text::display_width_u16;
pub(crate) use self::{
    dialogs::{
        confirm_close_button_rects, confirm_close_popup_rect, delete_module_button_rects,
        delete_module_popup_rect, new_linked_worktree_button_rects, new_linked_worktree_inner_rect,
        open_existing_worktree_button_rects, open_existing_worktree_inner_rect,
        open_existing_worktree_max_visible_rows, open_existing_worktree_visible_start,
        remove_worktree_button_rects, remove_worktree_popup_rect, rename_button_rects,
    },
    dialogs::{
        file_delete_choose_button_rects, file_delete_confirmation_inner_rect,
        file_delete_permanent_button_rects,
    },
    file_manager::module_dir_button_rects,
    settings::{settings_button_rects, settings_popup_rect_in, settings_show_primary_action},
    sidebar::{
        agent_entry_gap, agent_entry_height_in_body, agent_panel_body_rect, agent_panel_entries,
        agent_panel_scroll_for_target, agent_panel_scroll_metrics, agent_panel_scrollbar_rect,
        agent_panel_toggle_rect, all_agent_panel_entries, closed_agent_index_at,
        collapsed_sidebar_sections, collapsed_sidebar_toggle_rect, compute_workspace_card_areas,
        daily_new_chat_cell, effective_space, expanded_sidebar_sections,
        expanded_sidebar_toggle_rect, header_menu_cell, header_new_branch_cell,
        is_git_repository_root, module_branch_source, normalized_workspace_scroll,
        projects_scroll_metrics, projects_scrollbar_rect, sidebar_section_divider_rect,
        space_owner_for_key, workspace_chat_toggle_cell, workspace_drop_indicator_row,
        workspace_list_entries, workspace_list_entries_expanded, workspace_list_rect,
        workspace_list_scroll_metrics, workspace_list_scrollbar_rect, workspace_menu_cell,
        workspace_new_chat_cell, workspace_parent_group_state, AgentPanelEntry, ModuleBranchSource,
        WorkspaceListEntry,
    },
};
pub(crate) use self::{
    keybind_help::{keybind_help_layout_width, keybind_help_lines},
    mobile::{
        clamp_to_mobile_screen, mobile_drawer_areas, mobile_drawer_cursor_doc_range,
        mobile_drawer_cursor_stops, mobile_drawer_cursor_target, mobile_drawer_default_cursor,
        mobile_drawer_footer_band_height, mobile_drawer_max_scroll, mobile_drawer_pinned_start,
        mobile_drawer_rows, mobile_drawer_target_at, mobile_drawer_workspace_doc_range,
        mobile_screen_rect, DrawerRowContent, MobileHeaderHitAreas, MobileSwitcherTarget,
    },
    panes::{apply_pane_chrome, pane_inner_rect, pane_is_scrolled_back},
    tab_surface::{tab_surface_cursor, tab_surface_hyperlinks, TabSurfaceView},
    tabs::compute_tab_bar_view,
    widgets::{centered_popup_rect, modal_stack_areas},
};
use crate::app::state::ViewLayout;
use crate::app::{AppState, Mode};
use crate::terminal::TerminalRuntimeRegistry;

const COLLAPSED_WIDTH: u16 = 4; // num + space + dot + separator
const MOBILE_EMPTY_SHELL_LAYOUT_REVISION: u64 = 2;

// Braille spinner frames — smooth rotation
const SPINNERS: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Map spinner_tick (incremented every frame at ~60fps) to a spinner frame.
/// We want ~8 updates/sec so divide by 8.
pub(super) fn spinner_frame(tick: u32) -> &'static str {
    SPINNERS[(tick as usize / 8) % SPINNERS.len()]
}

/// Whether a view computation also reconciles the size of background tabs —
/// tabs no display is looking at. That sweep belongs to size-change events
/// (client connect, disconnect, resize) and to the single-client monolithic
/// loop. A per-client render pass must skip it: with two displays of
/// different shapes attached, each frame would rewrite every unwatched tab
/// to whichever display happens to be drawing, and every one of those
/// rewrites reflows that pane's whole scrollback. TP-MCF-SIZE-03
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum BackgroundTabSweep {
    Reconcile,
    Skip,
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
        BackgroundTabSweep::Reconcile,
        crate::kitty_graphics::HostCellSize::default(),
    );
}

pub fn compute_view_with_cell_size(
    app: &mut AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    area: Rect,
    cell_size: crate::kitty_graphics::HostCellSize,
) {
    compute_view_internal(
        app,
        terminal_runtimes,
        area,
        true,
        BackgroundTabSweep::Reconcile,
        cell_size,
    );
}

/// Compute view geometry for one display, resizing the panes that display is
/// looking at while leaving background tabs to the size-change event path.
///
/// Used by the per-client render passes and by the pre-input reconcile: both
/// serve a single display, and neither is a change in session geometry, so
/// neither has any business rewriting tabs nobody is watching.
/// See [`BackgroundTabSweep`].
pub(crate) fn compute_view_skipping_background_tabs(
    app: &mut AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    area: Rect,
    cell_size: crate::kitty_graphics::HostCellSize,
) {
    compute_view_internal(
        app,
        terminal_runtimes,
        area,
        true,
        BackgroundTabSweep::Skip,
        cell_size,
    );
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
        BackgroundTabSweep::Skip,
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
            // TP-STAGE-SBS-01: the side-by-side right half's active tab is
            // on screen at the split rect — the full-size sweep would fight
            // the split resize every frame.
            if app.side_by_side.map(|sbs| sbs.right)
                == Some(crate::app::state::SideBySideRight::Workspace(ws_idx))
                && tab_idx == ws.active_tab_index()
            {
                continue;
            }
            // Another display is watching this tab, so its own render pass
            // owns the size. Sweeping it here would make the last render in
            // the frame resize every tab to its own display. TP-MCF-SIZE-01
            if ws.tab_is_watched(tab_idx) {
                continue;
            }
            resize_tab_surface(app, terminal_runtimes, tab, terminal_area, cell_size);
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
            // Another display is watching this tab, so its own render pass
            // owns the size. Sweeping it here would make the last render in
            // the frame resize every tab to its own display. TP-MCF-SIZE-01
            if ws.tab_is_watched(tab_idx) {
                continue;
            }
            resize_tab_surface(app, terminal_runtimes, tab, terminal_area, cell_size);
        }
    }
}

fn desktop_tab_bar_and_terminal_area(
    app: &AppState,
    ws: &crate::workspace::Workspace,
    main_area: Rect,
) -> (Rect, Rect) {
    // TP-FTAB-ENTRY-04: the rule hides chrome that shows a single entry. An
    // open stage app is a second entry, so the strip must come back or the
    // Files tab would be unreachable by mouse.
    let strip_entries = ws.tabs.len() + app.stage.app_tab_instances().count();
    // A phone held sideways is wide but only fourteen rows tall. A strip
    // showing one entry costs a row and says nothing there, so the short
    // viewport applies the same rule the setting applies — and gives the row
    // back the moment the viewport grows, because this reads the size rather
    // than changing the setting.
    let short_viewport = size_class::SizeClass::of(main_area, app.mobile_width_threshold).height
        == size_class::HeightClass::Short;
    let hide_single_tab_bar =
        (app.hide_tab_bar_when_single_tab || short_viewport) && strip_entries == 1;
    if !hide_single_tab_bar && main_area.height > 1 {
        let [tab_bar_rect, terminal_area] =
            Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).areas(main_area);
        (tab_bar_rect, terminal_area)
    } else {
        (Rect::default(), main_area)
    }
}

fn sync_file_manager_locations_view(app: &mut AppState, area: Rect) -> FileManagerLocationsView {
    let mut view = project_file_manager_locations_view(app, area);
    let drawer_is_invalid = app.file_manager_locations.drawer_is_open()
        && (view.layout.mode != FileManagerLocationsMode::Compact
            || view.locations_action_area.is_none()
            || view.drawer_area.is_none());
    if drawer_is_invalid {
        app.request_file_manager_location_navigation = None;
        app.file_manager_locations.pending = None;
        let _ = app.file_manager_locations.close_drawer();
        view = project_file_manager_locations_view(app, area);
    }
    view
}

fn compute_view_internal(
    app: &mut AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    area: Rect,
    resize_panes: bool,
    sweep: BackgroundTabSweep,
    cell_size: crate::kitty_graphics::HostCellSize,
) {
    let _profile = crate::render_prof::duration_guard("shell.compute_view");
    let _ = app
        .file_manager_locations
        .reconcile_model(&app.file_manager_locations_model);
    app.cancel_miller_resize_for_terminal_area(area.width);
    if app
        .shell_resize_original_total()
        .is_some_and(|original_total| original_total != area.width)
    {
        app.cancel_sidebar_resize_for_terminal_area(area.width);
    }

    if size_class::SizeClass::of(area, app.mobile_width_threshold).is_mobile_shell() {
        compute_mobile_view(app, terminal_runtimes, area, resize_panes, sweep, cell_size);
        return;
    }

    // A short viewport spends its rows on the terminal, so the sidebar falls
    // back to its status rail unless the person expanded it themselves. This
    // reads the viewport rather than writing `sidebar_collapsed`: the setting
    // is a preference, the size is a fact, and conflating them would leave a
    // collapsed sidebar behind after the terminal grew back.
    let auto_collapsed = size_class::SizeClass::of(area, app.mobile_width_threshold).height
        == size_class::HeightClass::Short
        && !app.sidebar_expanded_explicitly;
    let committed_sidebar_w = if app.sidebar_collapsed || auto_collapsed {
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
    // One home for "which tree is this". Asking for nothing still answers with
    // the legacy desktop tree, so this path is unchanged; what changed is that
    // the revision and the identity now come FROM the derivation instead of
    // being asserted next to it, which is what lets a configured tree key the
    // cache correctly the day one exists.
    // The request comes from the presentation, which the session file fills in
    // on restore. It is `None` today for everyone, so the answer is still the
    // legacy tree and nothing on screen moves; what ended is the era where the
    // file recorded a composition the draw path never asked for.
    // Focus quiets the edges that opted in, and it does so here rather than in
    // the stored value: the geometry key below compares what is drawn, so an
    // edge going quiet has to move the key or the frame would keep the old
    // composition and nothing on screen would change.
    let visible_bars = app
        .shell_presentation
        .bars()
        .visible(app.spaces_focus_only, app.shell_presentation.toggled_off());
    let derived =
        shell::derive_desktop_shell_layout(app.shell_presentation.shell_template(), visible_bars);
    let shell_layout = derived.layout;
    let shell_key = ShellGeometryKey::new(
        area,
        derived.revision,
        u64::from(sidebar_w),
        app.shell_presentation.left_panel_collapse_revision(),
        derived.template,
        visible_bars,
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

    // Exactly one stage surface owns the center's CONTENT. The tab strip is
    // shell chrome above it, not terminal-app chrome: both surfaces carve out
    // the identical strip and receive the identical content rect, so opening
    // Files switches tabs inside the workspace instead of replacing it.
    // TP-FTAB-CHROME-01/02.
    let terminal_surface_active =
        app.stage.surface_view() == surface_host::StageSurfaceView::TerminalWorkspace;
    let (tab_bar_rect, terminal_area) = app
        .active
        .and_then(|i| app.workspaces.get(i))
        .map(|ws| desktop_tab_bar_and_terminal_area(app, ws, main_area))
        .unwrap_or((Rect::default(), main_area));
    // TP-STAGE-SBS-01: with a side-by-side pairing on, the stage splits in
    // two — the ACTIVE workspace keeps the left of both the strip and the
    // content, the named right half gets its own strip row and content rect.
    // The pairing self-heals here rather than at every consumer: a right
    // index that vanished, equals the active, or a stage too narrow to
    // split, and the mode is dropped whole.
    let mut sbs_divider_rect = None;
    let side_by_side = match app.side_by_side {
        Some(sbs) if terminal_surface_active && terminal_area.width >= 40 => match sbs.right {
            crate::app::state::SideBySideRight::Workspace(right)
                if Some(right) != app.active && app.workspaces.get(right).is_some() =>
            {
                Some(sbs)
            }
            // TP-SBS-FILES-01: Files needs no partner workspace — only a
            // resident file manager to draw.
            crate::app::state::SideBySideRight::Files if app.file_manager.is_some() => Some(sbs),
            _ => {
                app.side_by_side = None;
                None
            }
        },
        Some(_) => {
            app.side_by_side = None;
            None
        }
        None => None,
    };
    let (tab_bar_rect, terminal_area, right_halves) = match side_by_side {
        Some(sbs) => {
            let ratio = u16::from(sbs.ratio_percent.clamp(20, 80));
            let left_w = (terminal_area.width.saturating_sub(1)) * ratio / 100;
            let right_x = terminal_area.x + left_w + 1;
            // The split already leaves this one column empty between the
            // halves — name it so the mouse can grab it (the ratio drag).
            sbs_divider_rect = Some(Rect::new(
                terminal_area.x + left_w,
                terminal_area.y,
                1,
                terminal_area.height,
            ));
            let right_w = terminal_area.width.saturating_sub(left_w).saturating_sub(1);
            let left_strip = Rect::new(tab_bar_rect.x, tab_bar_rect.y, left_w, tab_bar_rect.height);
            let right_strip = Rect::new(right_x, tab_bar_rect.y, right_w, tab_bar_rect.height);
            let left_area = Rect::new(
                terminal_area.x,
                terminal_area.y,
                left_w,
                terminal_area.height,
            );
            let right_area = Rect::new(right_x, terminal_area.y, right_w, terminal_area.height);
            (left_strip, left_area, Some((sbs, right_strip, right_area)))
        }
        None => (tab_bar_rect, terminal_area, None),
    };
    // TP-SBS-FILES-01: when Files rides the right half, every Files
    // projection lives in that rectangle — geometry and hits alike.
    let files_viewport = right_halves
        .as_ref()
        .filter(|(sbs, _, _)| matches!(sbs.right, crate::app::state::SideBySideRight::Files))
        .map(|(_, _, area)| *area)
        .unwrap_or(terminal_area);
    let file_manager_locations = sync_file_manager_locations_view(app, files_viewport);
    let file_manager_miller = sync_miller_view(app, file_manager_locations.layout.trail);
    let file_manager_trail = sync_trail_view(app, file_manager_locations.layout.trail);
    let FileManagerRowGeometry {
        rows: file_manager_row_areas,
        actions: file_manager_row_action_areas,
    } = sync_file_manager_view(app, &file_manager_trail);
    let file_manager_action_bar = app.staged_file_manager().map(|file_manager| {
        compute_file_manager_action_bar_model(
            file_manager,
            &app.file_manager_clipboard,
            app.file_manager_operation
                .as_ref()
                .is_some_and(crate::app::state::FileManagerOperationState::is_running),
            app.file_manager_locations.focus,
        )
    });
    let file_manager_header_action_areas = if app.staged_file_manager().is_some() {
        compute_file_manager_header_action_areas(files_viewport)
    } else {
        Vec::new()
    };
    let preview_viewer_content_area = app
        .preview_viewer
        .is_some()
        .then(|| preview_viewer_content_area(area))
        .flatten();

    if !app.sidebar_collapsed {
        app.workspace_scroll = normalized_workspace_scroll(app, sidebar_area, app.workspace_scroll);
        let (_, detail_area) =
            expanded_sidebar_sections(sidebar_area, app.sidebar_section_split, app.sidebar_chrome);
        let max_agent_scroll = agent_panel_scroll_metrics(app, detail_area).max_offset_from_bottom;
        app.agent_panel_scroll = app.agent_panel_scroll.min(max_agent_scroll);
    } else {
        app.workspace_scroll = app
            .workspace_scroll
            .min(app.workspaces.len().saturating_sub(1));
        app.agent_panel_scroll = 0;
    }

    // Files is a center-stage app, not a global-sidebar content owner. A
    // legacy Files tab value therefore falls back to the Spaces projection;
    // current Files activation preserves whichever Spaces/Projects owner was
    // already selected.
    let show_spaces_content = app.sidebar_tab != crate::app::state::SidebarTab::Projects;
    let (
        workspace_card_areas,
        workspace_chat_row_areas,
        workspace_group_header_areas,
        workspace_project_header_areas,
        workspace_more_chats_areas,
        workspace_empty_module_areas,
        daily_areas,
        module_chat_row_areas,
    ) = if app.sidebar_collapsed || !show_spaces_content {
        (
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            sidebar::DailySectionAreas::default(),
            Vec::new(),
        )
    } else {
        sidebar::compute_workspace_list_areas(app, sidebar_area)
    };
    let sidebar_tab_hit_areas = if app.sidebar_collapsed {
        Vec::new()
    } else {
        sidebar::compute_sidebar_tab_areas(sidebar::workspace_list_rect(
            sidebar_area,
            app.sidebar_section_split,
            app.sidebar_chrome,
        ))
    };
    // The Projects tab owns its own row layout. Lay it out here (geometry only)
    // so render stays pure and the mouse handler hit-tests the same rects.
    let project_row_areas =
        if app.sidebar_collapsed || app.sidebar_tab != crate::app::state::SidebarTab::Projects {
            Vec::new()
        } else {
            let list_rect = sidebar::workspace_list_rect(
                sidebar_area,
                app.sidebar_section_split,
                app.sidebar_chrome,
            );
            // The projects list length changes underneath the scroll offset
            // via the session polls; re-normalize before laying out so the
            // viewport can never point past the end of the list.
            app.projects_scroll =
                sidebar::normalized_projects_scroll(app, list_rect, app.projects_scroll);
            sidebar::compute_project_row_areas(app, list_rect)
        };

    let stage_tabs: Vec<_> = app.stage.app_tab_instances().collect();
    let tab_bar_view = app
        .active
        .and_then(|ws_idx| app.workspaces.get(ws_idx))
        .map(|ws| {
            compute_tab_bar_view(
                ws,
                &stage_tabs,
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
    let TabSurfaceLayout {
        pane_infos,
        split_borders,
    } = if terminal_surface_active {
        compute_tab_surface(
            app,
            terminal_runtimes,
            terminal_area,
            resize_panes,
            cell_size,
        )
    } else {
        TabSurfaceLayout {
            pane_infos: Vec::new(),
            split_borders: Vec::new(),
        }
    };
    let right_surface = right_halves.map(|(sbs, strip_rect, area)| {
        let layout = match sbs.right {
            crate::app::state::SideBySideRight::Workspace(right_ws) => {
                tab_surface::compute_tab_surface_for(
                    app,
                    terminal_runtimes,
                    right_ws,
                    area,
                    resize_panes,
                    cell_size,
                )
            }
            // TP-SBS-FILES-01: Files hosts no panes; its own geometry is the
            // file-manager projection computed above.
            crate::app::state::SideBySideRight::Files => TabSurfaceLayout {
                pane_infos: Vec::new(),
                split_borders: Vec::new(),
            },
        };
        crate::app::state::RightSurfaceView {
            right: sbs.right,
            area,
            strip_rect,
            pane_infos: layout.pane_infos,
            split_borders: layout.split_borders,
        }
    });
    let agent_attachment_action_area =
        panes::compute_agent_attachment_action_area(app, &pane_infos);
    let agent_worktree_action_area = panes::compute_agent_worktree_action_area(app, &pane_infos);
    let agent_attachment_picker_row_areas = sync_agent_attachment_picker_view(app, terminal_area);
    if resize_panes {
        if sweep == BackgroundTabSweep::Reconcile {
            resize_background_tab_panes_for_desktop(app, terminal_runtimes, main_area, cell_size);
        }
        resize_popup_pane(app, terminal_runtimes, terminal_area, cell_size);
    }

    // Complete dock targets for this frame; the legacy default template
    // projects no dock region, so this stays empty until one is live.
    let app_dock_entry_areas = app_dock::app_dock_entry_areas(
        &app_dock::AppDockModel::for_state(app),
        app.shell_presentation
            .bars()
            .visible(app.spaces_focus_only, app.shell_presentation.toggled_off())
            .left
            .inner(shell_view.regions.get(RegionId::AppDock)),
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
        workspace_chat_row_areas,
        workspace_more_chats_areas,
        workspace_group_header_areas,
        workspace_project_header_areas,
        workspace_empty_module_areas,
        daily_header_area: daily_areas.header,
        daily_chat_row_areas: daily_areas.chats,
        module_chat_row_areas,
        daily_more_area: daily_areas.more,
        daily_more_workspaces_area: daily_areas.more_workspaces,
        sidebar_tab_hit_areas,
        project_row_areas,
        project_rows_generation: app.projects_sessions_generation,
        sbs_divider_rect,
        app_dock_entry_areas,
        file_manager_locations,
        file_manager_miller,
        file_manager_trail,
        file_manager_row_areas,
        file_manager_row_action_areas,
        file_manager_header_action_areas,
        preview_viewer_content_area,
        file_manager_action_bar,
        agent_attachment_action_area,
        agent_worktree_action_area,
        agent_attachment_picker_row_areas,
        tab_bar_rect,
        tab_hit_areas: tab_bar_view.tab_hit_areas,
        stage_tab_hit_areas: tab_bar_view.stage_tab_hit_areas,
        tab_scroll_left_hit_area: tab_bar_view.scroll_left_hit_area,
        tab_scroll_right_hit_area: tab_bar_view.scroll_right_hit_area,
        new_tab_hit_area: tab_bar_view.new_tab_hit_area,
        split_right_hit_area: tab_bar_view.split_right_hit_area,
        split_down_hit_area: tab_bar_view.split_down_hit_area,
        terminal_area,
        mobile_header_rect: Rect::default(),
        mobile_header_hits: crate::ui::MobileHeaderHitAreas::default(),
        toast_hit_area,
        pane_infos,
        right_surface,
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
    sweep: BackgroundTabSweep,
    cell_size: crate::kitty_graphics::HostCellSize,
) {
    // Four rows make the header's buttons a 44pt touch square on a phone
    // cell; a short viewport folds back to two — rows are what it is short
    // of, and the one-row reach below the buttons (TP-MOB-66) still tops the
    // targets up (TP-MOB-89).
    let header_h =
        match crate::ui::size_class::SizeClass::of(area, app.mobile_width_threshold).height {
            crate::ui::size_class::HeightClass::Short => area.height.min(2),
            crate::ui::size_class::HeightClass::Regular => area.height.min(4),
        };
    let (header_rect, terminal_area) = if area.height > header_h {
        let [header_rect, terminal_area] =
            Layout::vertical([Constraint::Length(header_h), Constraint::Min(1)]).areas(area);
        (header_rect, terminal_area)
    } else {
        (area, Rect::default())
    };
    let file_manager_locations = sync_file_manager_locations_view(app, terminal_area);
    let file_manager_miller = sync_miller_view(app, file_manager_locations.layout.trail);
    let file_manager_trail = sync_trail_view(app, file_manager_locations.layout.trail);
    let FileManagerRowGeometry {
        rows: file_manager_row_areas,
        actions: file_manager_row_action_areas,
    } = sync_file_manager_view(app, &file_manager_trail);
    let file_manager_action_bar = app.staged_file_manager().map(|file_manager| {
        compute_file_manager_action_bar_model(
            file_manager,
            &app.file_manager_clipboard,
            app.file_manager_operation
                .as_ref()
                .is_some_and(crate::app::state::FileManagerOperationState::is_running),
            app.file_manager_locations.focus,
        )
    });
    let file_manager_header_action_areas = if app.staged_file_manager().is_some() {
        compute_file_manager_header_action_areas(terminal_area)
    } else {
        Vec::new()
    };
    let preview_viewer_content_area = app
        .preview_viewer
        .is_some()
        .then(|| preview_viewer_content_area(area))
        .flatten();

    if app.mode == Mode::Navigate {
        // On mobile, navigate mode *is* an open drawer. Every other way into
        // the mode — the prefix key, a keybind, a restored session — predates
        // the drawers and sets only the mode, which would leave a mode with no
        // surface: keystrokes going somewhere the reader cannot see. Spaces is
        // the drawer the old single panel showed, so that is what those paths
        // keep getting.
        if !app.mobile_drawer.is_open() {
            app.mobile_drawer = crate::app::state::MobileDrawer::Spaces;
            // A drawer opened by one of those paths still gets its cursor
            // placed in context, so the first arrow key means the same thing
            // however the drawer was reached.
            app.mobile_drawer_cursor = mobile_drawer_default_cursor(app);
        }
        let drawer_viewport_h = area
            .height
            .saturating_sub(header_h + 1)
            .saturating_sub(crate::ui::mobile_drawer_footer_band_height(app) as u16);
        let max_scroll = mobile_drawer_max_scroll_for_height(app, drawer_viewport_h);
        app.mobile_switcher_scroll = app.mobile_switcher_scroll.min(max_scroll);
    } else if app.mobile_drawer.is_open() {
        // And the converse: leaving navigate mode by any route closes the
        // drawer, so the two can never describe different things.
        app.mobile_drawer = crate::app::state::MobileDrawer::None;
    }

    // The same surface-exclusivity contract as the desktop projection: a
    // hidden terminal projects no pane/split hit geometry under NativeFiles.
    let terminal_surface_active =
        app.stage.surface_view() == surface_host::StageSurfaceView::TerminalWorkspace;
    let TabSurfaceLayout {
        pane_infos,
        split_borders,
    } = if terminal_surface_active {
        compute_tab_surface(
            app,
            terminal_runtimes,
            terminal_area,
            resize_panes,
            cell_size,
        )
    } else {
        TabSurfaceLayout {
            pane_infos: Vec::new(),
            split_borders: Vec::new(),
        }
    };
    let agent_attachment_picker_row_areas = sync_agent_attachment_picker_view(app, terminal_area);
    if resize_panes {
        if sweep == BackgroundTabSweep::Reconcile {
            resize_background_tab_panes_to_area(app, terminal_runtimes, terminal_area, cell_size);
        }
        resize_popup_pane(app, terminal_runtimes, terminal_area, cell_size);
    }
    let header_hits = compute_mobile_header_hit_areas(app, header_rect);

    let toast_hit_area = app
        .toast
        .as_ref()
        .map(|_| mobile_toast_banner_rect(area, app.config_diagnostic.is_some()))
        .unwrap_or_default();
    let shell_view = shell::compute_empty_shell_view(
        ShellGeometryKey::new(
            area,
            MOBILE_EMPTY_SHELL_LAYOUT_REVISION,
            0,
            0,
            None,
            shell::ShellBars::NONE,
        ),
        std::mem::take(&mut app.view.shell),
    );

    app.view = crate::app::ViewState {
        layout: ViewLayout::Mobile,
        // Mobile keeps its own header/terminal split; named shell regions are a
        // desktop concept for now, so leave the region map empty.
        shell: shell_view,
        sidebar_rect: Rect::default(),
        workspace_card_areas: Vec::new(),
        workspace_chat_row_areas: Vec::new(),
        workspace_more_chats_areas: Vec::new(),
        daily_header_area: None,
        daily_chat_row_areas: Vec::new(),
        module_chat_row_areas: Vec::new(),
        daily_more_area: None,
        daily_more_workspaces_area: None,
        workspace_group_header_areas: Vec::new(),
        workspace_project_header_areas: Vec::new(),
        workspace_empty_module_areas: Vec::new(),
        sidebar_tab_hit_areas: Vec::new(),
        stage_tab_hit_areas: Vec::new(),
        project_row_areas: Vec::new(),
        project_rows_generation: app.projects_sessions_generation,
        sbs_divider_rect: None,
        app_dock_entry_areas: Vec::new(),
        file_manager_locations,
        file_manager_miller,
        file_manager_trail,
        file_manager_row_areas,
        file_manager_row_action_areas,
        file_manager_header_action_areas,
        preview_viewer_content_area,
        file_manager_action_bar,
        agent_attachment_action_area: None,
        agent_worktree_action_area: None,
        agent_attachment_picker_row_areas,
        tab_bar_rect: Rect::default(),
        tab_hit_areas: Vec::new(),
        tab_scroll_left_hit_area: Rect::default(),
        tab_scroll_right_hit_area: Rect::default(),
        new_tab_hit_area: Rect::default(),
        split_right_hit_area: Rect::default(),
        split_down_hit_area: Rect::default(),
        terminal_area,
        mobile_header_rect: header_rect,
        mobile_header_hits: header_hits,
        toast_hit_area,
        pane_infos,
        right_surface: None,
        split_borders,
    };
    app.sync_copy_mode_search_geometry();
}

fn sync_file_manager_view(app: &AppState, snapshot: &TrailViewSnapshot) -> FileManagerRowGeometry {
    let Some(file_manager) = app.staged_file_manager() else {
        return FileManagerRowGeometry::default();
    };
    let Some(column) = snapshot
        .columns
        .iter()
        .find(|column| column.directory == file_manager.cwd)
    else {
        return FileManagerRowGeometry::default();
    };
    FileManagerRowGeometry {
        rows: column
            .rows
            .iter()
            .map(|row| crate::app::state::FileManagerRowArea {
                rect: row.name_rect,
                entry_idx: row.entry_index,
                entry_path: row.entry_path.clone(),
            })
            .collect(),
        actions: column
            .rows
            .iter()
            .flat_map(|row| row.actions.iter().cloned())
            .collect(),
    }
}

/// The Files surface this frame projects: the active stage instance when
/// Files owns the stage, the resident (backgrounded) one when Files rides
/// the right half (TP-SBS-FILES-01). `None` hides every Files projection.
fn files_projection_generation(app: &AppState) -> Option<u32> {
    if app.stage.surface_view() == surface_host::StageSurfaceView::NativeFiles {
        return app.stage.active_instance_generation();
    }
    if app.files_beside_active() {
        return app.resident_files_generation();
    }
    None
}

fn sync_miller_view(app: &mut AppState, viewport_area: Rect) -> MillerViewSnapshot {
    let Some(files_generation) = files_projection_generation(app) else {
        return MillerViewSnapshot::default();
    };
    // Surface ownership is read first so the file-manager borrow below stays a
    // disjoint field borrow (see `staged_file_manager_mut` for the rule).
    let files_surface_active =
        app.stage.surface_view() == surface_host::StageSurfaceView::NativeFiles;
    let resize_preview = app.shell_interaction.miller_resize_preview();
    let Some(file_manager) = app.file_manager.as_mut().filter(|_| files_surface_active) else {
        return MillerViewSnapshot::default();
    };
    let trail_directories = file_manager
        .trail
        .cols()
        .iter()
        .map(|column| column.directory.clone())
        .collect::<Vec<_>>();
    file_manager
        .miller
        .sync_trail_directories(&trail_directories);
    let mut snapshot = file_manager::miller::project_miller_view_with_resize_preview(
        viewport_area,
        file_manager,
        files_generation,
        resize_preview,
    );
    if !snapshot.columns.is_empty() {
        file_manager.miller.horizontal.offset_cells = snapshot.horizontal_offset_cells;
    }
    let current_visible_rows = snapshot
        .columns
        .iter()
        .find(|column| column.kind.is_directory(&file_manager.cwd))
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

fn sync_trail_view(app: &mut AppState, viewport_area: Rect) -> TrailViewSnapshot {
    let Some(files_generation) = files_projection_generation(app) else {
        return TrailViewSnapshot::default();
    };
    let show_row_actions = app.files_show_row_actions;
    let Some(file_manager) = app.file_manager.as_mut() else {
        return TrailViewSnapshot::default();
    };
    let preferred_widths = file_manager.miller.preferred_widths_for(
        file_manager
            .trail
            .cols()
            .iter()
            .map(|column| column.directory.clone()),
    );
    let detail_preferred_width = file_manager.miller.preview_preferred_width;
    let horizontal = file_manager.miller.horizontal;
    let vertical = file_manager.miller.vertical.clone();
    let mut snapshot = file_manager::trail_view::project_trail_view_both_axes(
        viewport_area,
        &file_manager.trail,
        &file_manager.trail_snapshots,
        &preferred_widths,
        detail_preferred_width,
        (!horizontal.follow_active).then_some(horizontal.offset_cells),
        &vertical,
        show_row_actions,
    );
    if !snapshot.columns.is_empty() {
        file_manager.miller.horizontal.offset_cells = snapshot.offset_cells;
    }
    snapshot.files_generation = Some(files_generation);
    snapshot.model_revision = file_manager.miller.revision;
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
        // Each configured edge wears its own shell first, then whatever lives
        // inside it draws into what the border left. The dock reads the same
        // inner rectangle the hit areas were built from, so a click lands where
        // the icon is rather than one cell off it.
        let bars = app
            .shell_presentation
            .bars()
            .visible(app.spaces_focus_only, app.shell_presentation.toggled_off());
        let colors = app.shell_presentation.bar_colors();
        for region in [
            RegionId::TopBar,
            RegionId::BottomBar,
            RegionId::AppDock,
            RegionId::RightPanel,
        ] {
            let outer = app.view.shell.regions.get(region);
            if outer.is_empty() {
                continue;
            }
            // TP-CHROME-147: the strip is painted on the theme's own general
            // background unless this bar says otherwise — including when it
            // wears no frame, which is what gives a plain or islands bar a
            // surface to sit on instead of whatever the terminal shows.
            let background = colors.background_for(region);
            if bars.track_for(region).has_border() {
                widgets::render_bar_shell(frame, outer, colors.for_region(region), background);
            } else {
                frame.render_widget(Clear, outer);
                frame.render_widget(
                    Block::default().style(Style::default().bg(background)),
                    outer,
                );
            }
        }

        // Then whatever each section shows, into the rectangle that section was
        // given. Read from the same track the hit areas were built from, so a
        // label and the click that lands on it can never belong to different
        // sections. Undivided bars produce no rectangles here and cost nothing.
        // Ordinary text colour, because the surface underneath is the panel
        // background this bar was just painted with. The neighbouring
        // `panel_contrast_fg` is for the opposite case — dark text on an accent
        // fill, the way the workspace chips read — and using it here returns
        // the panel background itself, which paints every label the exact
        // colour of the surface it lands on. That shipped once: the border was
        // visible, the glyphs were in the buffer, and the bar looked empty.
        let section_style = Style::default().fg(app.palette.text);
        for region in [
            RegionId::TopBar,
            RegionId::BottomBar,
            RegionId::AppDock,
            RegionId::RightPanel,
        ] {
            let outer = app.view.shell.regions.get(region);
            if outer.is_empty() {
                continue;
            }
            // Two passes, because an island can now span several sections:
            // every frame is painted before any widget is, or the frame that
            // closes at a run's last member would paint over the widgets its
            // earlier members already drew.
            let rects = bars.track_for(region).section_rects(region, outer);
            let mut inners: Vec<(usize, usize, Rect)> = Vec::new();
            let mut open: Option<(usize, Rect, crate::ui::shell::IslandSlot)> = None;
            for (index, rect) in rects.occupied() {
                let slot_index = u8::try_from(index).unwrap_or(u8::MAX);
                let Some(slot) = app.shell_bar_chrome.island_for(region, slot_index) else {
                    continue;
                };
                if slot.first {
                    open = Some((index, rect, slot));
                }
                let Some((start, start_rect, opening)) = open else {
                    continue;
                };
                if slot.last {
                    let island = start_rect.union(rect);
                    match opening.chrome {
                        // TP-CHROME-148: a pill is a filled band, no stroke —
                        // the whole run's rectangle is its own inner, so a
                        // pill spends no cells on chrome.
                        crate::ui::shell::SlotChrome::Pill { bg, .. } => {
                            frame.render_widget(Clear, island);
                            frame.render_widget(
                                Block::default().style(Style::default().bg(bg)),
                                island,
                            );
                            inners.push((start, index, island));
                        }
                        crate::ui::shell::SlotChrome::Frame { backdrop, .. } => {
                            // The inner rectangle is read back from the call
                            // that painted it rather than recomputed, for the
                            // reason `BarTrack::inner` exists: two places
                            // doing the same arithmetic are two places that
                            // can drift, and the drift shows up as a label
                            // one cell away from its box.
                            if let Some(inner) = widgets::render_bar_shell(
                                frame,
                                island,
                                opening.tint,
                                colors.background_for(region),
                            ) {
                                // TP-CHROME-148: a written backdrop fills the
                                // frame's inside — the island reading of the
                                // same `background` key the pill band reads.
                                if let Some(fill) = backdrop {
                                    frame.render_widget(
                                        Block::default().style(Style::default().bg(fill)),
                                        inner,
                                    );
                                }
                                inners.push((start, index, inner));
                            }
                        }
                    }
                    open = None;
                }
            }
            for (index, rect) in rects.occupied() {
                let slot_index = u8::try_from(index).unwrap_or(u8::MAX);
                let content = match app.shell_bar_chrome.island_for(region, slot_index) {
                    None => rect,
                    Some(_) => {
                        let Some(inner) = inners
                            .iter()
                            .find(|(start, end, _)| *start <= index && index <= *end)
                            .map(|(_, _, inner)| rect.intersection(*inner))
                            .filter(|content| !content.is_empty())
                        else {
                            // Too small for the frame that was asked for.
                            // Drawing the widget anyway would put content where
                            // the person is expecting a box and leave nothing
                            // to explain the missing one.
                            continue;
                        };
                        inner
                    }
                };
                if let Some(widget) = app.shell_bar_chrome.widget_for(region, slot_index) {
                    // TP-CHROME-148: text on a pill is the run's vivid tone —
                    // the same family as the band it sits on, resolved beside
                    // the band so the two cannot drift apart.
                    let widget_style = match app
                        .shell_bar_chrome
                        .island_for(region, slot_index)
                        .map(|slot| slot.chrome)
                    {
                        Some(crate::ui::shell::SlotChrome::Pill { fg, .. }) => {
                            Style::default().fg(fg)
                        }
                        // The island reading of the same `color` key: a
                        // written family reaches the words inside the frame.
                        Some(crate::ui::shell::SlotChrome::Frame { fg: Some(fg), .. }) => {
                            Style::default().fg(fg)
                        }
                        _ => section_style,
                    };
                    widgets::render_section_widget(
                        frame,
                        widget,
                        &app.resources,
                        &app.resource_history,
                        app.clock_now,
                        &app.palette,
                        content,
                        widget_style,
                    );
                }
            }
        }

        let dock_area = bars
            .left
            .inner(app.view.shell.regions.get(RegionId::AppDock));
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
                // No active workspace paints an explicit empty state rather
                // than an unexplained blank center (upstream behavior).
                if app
                    .active
                    .and_then(|ws_idx| app.workspaces.get(ws_idx))
                    .is_some()
                {
                    render_tab_surface(app, terminal_runtimes, app.view.tab_surface(), frame);
                    // TP-STAGE-SBS-01: the other half — its own strip, its
                    // own panes, a one-column divider between the worlds.
                    if let Some(right) = app.view.right_surface.as_ref() {
                        render_side_by_side_right(app, terminal_runtimes, frame, right);
                    }
                } else {
                    render_empty(app, frame, terminal_area)
                }
            }
        }

        // Ambient notifications sit above the center content, but below
        // interactive overlays.
        render_notifications(app, frame, terminal_area);
        render_popup_pane(app, terminal_runtimes, frame, terminal_area);
    }
}

/// TP-STAGE-SBS-01: paint the side-by-side right half — divider column, a
/// minimal strip naming the workspace and its tabs (presentation only in the
/// POC: the strip takes no clicks yet), then the panes through the same
/// named-workspace road the left half uses.
fn render_side_by_side_right(
    app: &AppState,
    terminal_runtimes: &TerminalRuntimeRegistry,
    frame: &mut Frame,
    right: &crate::app::state::RightSurfaceView,
) {
    let p = &app.palette;
    let divider_x = right.area.x.saturating_sub(1);
    let full_height = right.strip_rect.height.saturating_add(right.area.height);
    for row in 0..full_height {
        let y = right.strip_rect.y.saturating_add(row);
        let buf = frame.buffer_mut();
        if divider_x < buf.area.right() && y < buf.area.bottom() {
            let cell = &mut buf[(divider_x, y)];
            cell.set_symbol("\u{2502}");
            cell.set_style(ratatui::style::Style::default().fg(p.surface1));
        }
    }
    match right.right {
        crate::app::state::SideBySideRight::Workspace(right_ws) => {
            if let Some(ws) = app.workspaces.get(right_ws) {
                let mut strip = format!(" {} ", ws.display_name());
                for (idx, _tab) in ws.tabs.iter().enumerate() {
                    let marker = if idx == ws.active_tab_index() {
                        "*"
                    } else {
                        " "
                    };
                    strip.push_str(&format!("[{}{marker}]", idx + 1));
                }
                let text = self::text::truncate_end(&strip, right.strip_rect.width as usize);
                frame.render_widget(
                    ratatui::widgets::Paragraph::new(text).style(
                        ratatui::style::Style::default()
                            .bg(p.panel_bg)
                            .fg(p.subtext0),
                    ),
                    right.strip_rect,
                );
            }
            panes::render_panes_for(
                app,
                terminal_runtimes,
                frame,
                right_ws,
                &right.pane_infos,
                &right.split_borders,
            );
        }
        // TP-SBS-FILES-01: the right half hosts the resident Files surface —
        // strip names it, the body is the same renderer the full stage uses.
        crate::app::state::SideBySideRight::Files => {
            let text = self::text::truncate_end(" Files ", right.strip_rect.width as usize);
            frame.render_widget(
                ratatui::widgets::Paragraph::new(text).style(
                    ratatui::style::Style::default()
                        .bg(p.panel_bg)
                        .fg(p.subtext0),
                ),
                right.strip_rect,
            );
            file_manager::render_file_manager(app, frame, right.area);
        }
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
                render_mobile_drawer(app, terminal_runtimes, frame)
            }
            Mode::Navigate => render_navigate_overlay(app, frame, terminal_area),
            Mode::Prefix => render_prefix_overlay(app, frame, terminal_area),
            Mode::Copy => render_copy_mode_overlay(app, frame, terminal_area),
            Mode::Resize => render_resize_overlay(app, frame, terminal_area),
            Mode::ConfirmClose => {
                render_confirm_close_overlay(app, terminal_runtimes, frame, terminal_area)
            }
            Mode::ConfirmFileDelete => {
                render_file_delete_confirmation_overlay(app, frame, terminal_area)
            }
            Mode::ContextMenu => {
                render_context_menu(app, frame);
            }
            Mode::Settings => render_settings_overlay(app, frame, frame.area()),
            Mode::AgentReferencePicker => render_agent_reference_picker(app, frame),
            Mode::AgentColleaguePicker => render_agent_colleague_picker(app, frame),
            Mode::BarConfigPanel => render_bar_config_panel(app, frame),
            Mode::PreviewViewer => render_preview_viewer(app, frame, frame.area()),
            Mode::TailscaleSend => render_tailscale_send(app, frame, terminal_area),
            Mode::RenameWorkspace | Mode::RenameTab | Mode::RenamePane | Mode::RenameFile => {
                render_rename_overlay(app, frame, frame.area())
            }
            Mode::NewLinkedWorktree => render_new_linked_worktree_overlay(app, frame, frame.area()),
            Mode::OpenExistingWorktree => {
                render_open_existing_worktree_overlay(app, frame, frame.area())
            }
            Mode::ConfirmRemoveWorktree => render_remove_worktree_overlay(app, frame, frame.area()),
            Mode::ConfirmDeleteModule => {
                dialogs::render_delete_module_overlay(app, frame, frame.area())
            }
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

    // The question a person asks about this feature is "what do I see?", and
    // the answer differs per edge today: the dock is a finished component that
    // was only ever missing a rectangle, while the other three edges have no
    // renderer yet. Pinning both halves keeps the next layer honest about which
    // of those it is closing, and stops "the bar works" from being said about
    // an edge that draws nothing.
    // TC-C1/TC-C2/TC-C7 · a label is EMITTED, addressed, and PAINTED. The last
    // of those is the one nothing else can see: a widget that is derived, given
    // a rectangle and never drawn passes every state test and the compiler
    // counts it as used, because it IS used — just not by anything that paints.
    // Only a buffer dump can tell those apart, which is how this fork lost a
    // sidebar row for a whole release once.
    //
    // The second section's label is deliberately wider than its cells and
    // written with a double-width glyph: clipping by character count would fit
    // "四四四" into three cells and paint six, over the neighbour.
    #[test]
    fn a_section_label_is_painted_inside_its_own_section_and_clipped_by_display_width() {
        use ratatui::{backend::TestBackend, Terminal};

        let mut first = crate::config::ShellBarSectionConfig {
            kind: "fixed".to_string(),
            cells: 6,
            ..Default::default()
        };
        first.widget.kind = "label".to_string();
        first.widget.text = "CPU".to_string();

        let mut second = crate::config::ShellBarSectionConfig {
            kind: "fixed".to_string(),
            cells: 4,
            ..Default::default()
        };
        second.widget.kind = "label".to_string();
        second.widget.text = "四四四四".to_string();

        let config = crate::config::ShellBarsConfig {
            top: crate::config::ShellBarConfig {
                enabled: true,
                size: 1,
                border: Some(false),
                color: String::new(),
                gradient: Vec::new(),
                sections: vec![first, second],
                ..Default::default()
            },
            ..Default::default()
        };

        let frame = Rect::new(0, 0, 100, 30);
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.shell_presentation = crate::ui::shell::ShellPresentationState::from_restored(
            app.sidebar_width,
            false,
            None,
            crate::ui::shell::ShellBars::from_config(&config),
        );
        app.shell_bar_chrome =
            crate::ui::shell::ShellBarChrome::from_config(&config, true, &app.palette);
        compute_view(&mut app, frame);

        let mut terminal =
            Terminal::new(TestBackend::new(frame.width, frame.height)).expect("test backend");
        terminal
            .draw(|f| render(&app, f))
            .expect("a bar with labelled sections draws");
        let buffer = terminal.backend().buffer().clone();

        let row: String = (0..10)
            .map(|x| {
                buffer
                    .cell((x, 0))
                    .map(|cell| cell.symbol().to_string())
                    .unwrap_or_default()
            })
            .collect();

        assert!(
            row.starts_with("CPU"),
            "the first section's label must be painted where that section is: {row:?}"
        );
        assert!(
            row[..].contains('四'),
            "the second section's label must be painted too: {row:?}"
        );
        // Four cells hold two double-width glyphs at most, and the truncation
        // spends one on the ellipsis, so exactly one fits beside it.
        assert_eq!(
            row.matches('四').count(),
            1,
            "clipping by display width must not paint six cells into four: {row:?}"
        );
    }

    // TP-CHROME-147: a borderless bar still paints its band — every cell of
    // the strip, sections or not, sits on the theme's own general background
    // instead of whatever the terminal happens to show underneath. This is
    // the reported complaint: the bar's backdrop matched the tab strip's
    // empty area rather than following the theme.
    #[test]
    fn a_borderless_bar_paints_its_band_on_the_theme_background() {
        use ratatui::{backend::TestBackend, Terminal};

        let mut section = crate::config::ShellBarSectionConfig {
            kind: "fixed".to_string(),
            cells: 6,
            ..Default::default()
        };
        section.widget.kind = "label".to_string();
        section.widget.text = "CPU".to_string();

        let config = crate::config::ShellBarsConfig {
            top: crate::config::ShellBarConfig {
                enabled: true,
                size: 1,
                border: Some(false),
                sections: vec![section],
                ..Default::default()
            },
            ..Default::default()
        };

        let frame = Rect::new(0, 0, 100, 30);
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.shell_presentation = crate::ui::shell::ShellPresentationState::from_restored(
            app.sidebar_width,
            false,
            None,
            crate::ui::shell::ShellBars::from_config(&config),
        )
        .with_bar_colors(crate::ui::shell::BarColors::from_config(
            &config,
            &app.palette,
        ));
        app.shell_bar_chrome =
            crate::ui::shell::ShellBarChrome::from_config(&config, true, &app.palette);
        compute_view(&mut app, frame);

        let mut terminal =
            Terminal::new(TestBackend::new(frame.width, frame.height)).expect("test backend");
        terminal
            .draw(|f| render(&app, f))
            .expect("a borderless bar draws");
        let buffer = terminal.backend().buffer().clone();

        assert_ne!(
            app.palette.bg, app.palette.panel_bg,
            "precondition: this theme distinguishes the two"
        );
        let far_cell = buffer.cell((60, 0)).expect("cell");
        assert_eq!(
            far_cell.bg, app.palette.bg,
            "the band beyond the sections wears the theme's general background"
        );
    }

    // TP-CHROME-147: a written `background` reaches the painted band — the
    // resolver's explicit-wins rule is worth nothing if the paint keeps
    // reading the theme.
    #[test]
    fn a_written_background_reaches_the_painted_band() {
        use ratatui::{backend::TestBackend, Terminal};

        let config = crate::config::ShellBarsConfig {
            top: crate::config::ShellBarConfig {
                enabled: true,
                size: 1,
                border: Some(false),
                background: "red".to_string(),
                sections: Vec::new(),
                ..Default::default()
            },
            ..Default::default()
        };

        let frame = Rect::new(0, 0, 100, 30);
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.shell_presentation = crate::ui::shell::ShellPresentationState::from_restored(
            app.sidebar_width,
            false,
            None,
            crate::ui::shell::ShellBars::from_config(&config),
        )
        .with_bar_colors(crate::ui::shell::BarColors::from_config(
            &config,
            &app.palette,
        ));
        app.shell_bar_chrome =
            crate::ui::shell::ShellBarChrome::from_config(&config, true, &app.palette);
        compute_view(&mut app, frame);

        let mut terminal =
            Terminal::new(TestBackend::new(frame.width, frame.height)).expect("test backend");
        terminal
            .draw(|f| render(&app, f))
            .expect("a borderless bar with a written background draws");
        let buffer = terminal.backend().buffer().clone();

        let far_cell = buffer.cell((60, 0)).expect("cell");
        assert_eq!(
            far_cell.bg, app.palette.red,
            "the written background wins on the painted band too"
        );
    }

    // TP-CHROME-147: an island's frame sits on the bar's background — the
    // island shell is passed the same resolved backdrop the band wears, not
    // the floating-panel tone it used to borrow.
    #[test]
    fn an_island_frame_sits_on_the_bars_background() {
        use ratatui::{backend::TestBackend, Terminal};

        let mut section = crate::config::ShellBarSectionConfig {
            kind: "fixed".to_string(),
            cells: 6,
            border: Some(true),
            color: "teal".to_string(),
            ..Default::default()
        };
        section.widget.kind = "label".to_string();
        section.widget.text = "CPU".to_string();

        let config = crate::config::ShellBarsConfig {
            top: crate::config::ShellBarConfig {
                enabled: true,
                size: 3,
                border: Some(false),
                sections: vec![section],
                ..Default::default()
            },
            ..Default::default()
        };

        let frame = Rect::new(0, 0, 100, 30);
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.shell_presentation = crate::ui::shell::ShellPresentationState::from_restored(
            app.sidebar_width,
            false,
            None,
            crate::ui::shell::ShellBars::from_config(&config),
        )
        .with_bar_colors(crate::ui::shell::BarColors::from_config(
            &config,
            &app.palette,
        ));
        app.shell_bar_chrome =
            crate::ui::shell::ShellBarChrome::from_config(&config, true, &app.palette);
        compute_view(&mut app, frame);

        let mut terminal =
            Terminal::new(TestBackend::new(frame.width, frame.height)).expect("test backend");
        terminal
            .draw(|f| render(&app, f))
            .expect("an islands bar draws");
        let buffer = terminal.backend().buffer().clone();

        let corner = (0..6u16)
            .filter_map(|x| buffer.cell((x, 0)).map(|cell| (x, cell.clone())))
            .find(|(_, cell)| cell.symbol() == "\u{256d}" || cell.symbol() == "\u{250c}");
        let (_, frame_cell) = corner.expect("an island frame corner is painted on the top row");
        assert_eq!(
            frame_cell.bg, app.palette.bg,
            "the island frame sits on the bar's resolved background"
        );
    }

    // TP-CHROME-163: a written section `color` reaches the words inside an
    // island, the way it already reaches the words on a pill — the frame
    // stroke and the text speak the same family.
    #[test]
    fn an_islands_written_color_paints_its_widget_text() {
        use ratatui::{backend::TestBackend, Terminal};

        let mut section = crate::config::ShellBarSectionConfig {
            kind: "fixed".to_string(),
            cells: 6,
            border: Some(true),
            color: "teal".to_string(),
            ..Default::default()
        };
        section.widget.kind = "label".to_string();
        section.widget.text = "CPU".to_string();

        let config = crate::config::ShellBarsConfig {
            top: crate::config::ShellBarConfig {
                enabled: true,
                size: 3,
                border: Some(false),
                sections: vec![section],
                ..Default::default()
            },
            ..Default::default()
        };

        let frame = Rect::new(0, 0, 100, 30);
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.shell_presentation = crate::ui::shell::ShellPresentationState::from_restored(
            app.sidebar_width,
            false,
            None,
            crate::ui::shell::ShellBars::from_config(&config),
        )
        .with_bar_colors(crate::ui::shell::BarColors::from_config(
            &config,
            &app.palette,
        ));
        app.shell_bar_chrome =
            crate::ui::shell::ShellBarChrome::from_config(&config, true, &app.palette);
        compute_view(&mut app, frame);

        let mut terminal =
            Terminal::new(TestBackend::new(frame.width, frame.height)).expect("test backend");
        terminal
            .draw(|f| render(&app, f))
            .expect("an islands bar draws");
        let buffer = terminal.backend().buffer().clone();

        let label_cell = (0..3u16)
            .flat_map(|y| (0..8u16).map(move |x| (x, y)))
            .filter_map(|pos| buffer.cell(pos))
            .find(|cell| cell.symbol() == "C")
            .expect("the island label is painted");
        assert_eq!(
            label_cell.fg, app.palette.teal,
            "the written colour reaches the island's text"
        );
    }

    // TP-CHROME-163: a section that writes no colour keeps today's plain
    // text tone — the family stroke is opt-in for the words, never imposed.
    #[test]
    fn an_islands_unwritten_section_keeps_the_plain_text_tone() {
        use ratatui::{backend::TestBackend, Terminal};

        let mut section = crate::config::ShellBarSectionConfig {
            kind: "fixed".to_string(),
            cells: 6,
            border: Some(true),
            ..Default::default()
        };
        section.widget.kind = "label".to_string();
        section.widget.text = "CPU".to_string();

        let config = crate::config::ShellBarsConfig {
            top: crate::config::ShellBarConfig {
                enabled: true,
                size: 3,
                border: Some(false),
                sections: vec![section],
                ..Default::default()
            },
            ..Default::default()
        };

        let frame = Rect::new(0, 0, 100, 30);
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.shell_presentation = crate::ui::shell::ShellPresentationState::from_restored(
            app.sidebar_width,
            false,
            None,
            crate::ui::shell::ShellBars::from_config(&config),
        )
        .with_bar_colors(crate::ui::shell::BarColors::from_config(
            &config,
            &app.palette,
        ));
        app.shell_bar_chrome =
            crate::ui::shell::ShellBarChrome::from_config(&config, true, &app.palette);
        compute_view(&mut app, frame);

        let mut terminal =
            Terminal::new(TestBackend::new(frame.width, frame.height)).expect("test backend");
        terminal
            .draw(|f| render(&app, f))
            .expect("an islands bar draws");
        let buffer = terminal.backend().buffer().clone();

        let label_cell = (0..3u16)
            .flat_map(|y| (0..8u16).map(move |x| (x, y)))
            .filter_map(|pos| buffer.cell(pos))
            .find(|cell| cell.symbol() == "C")
            .expect("the island label is painted");
        assert_eq!(
            label_cell.fg, app.palette.text,
            "an unwritten section keeps the plain tone"
        );
    }

    // TP-CHROME-148: a pills bar paints each section as a filled band in its
    // own family — pastel behind, vivid text on it — the spacer between them
    // stays the bar's backdrop, and a grouped run is ONE continuous band.
    #[test]
    fn a_pills_bar_paints_family_bands_with_vivid_text() {
        use ratatui::{backend::TestBackend, Terminal};

        let mut cpu = crate::config::ShellBarSectionConfig {
            kind: "fixed".to_string(),
            cells: 6,
            color: "yellow".to_string(),
            ..Default::default()
        };
        cpu.widget.kind = "label".to_string();
        cpu.widget.text = "CPU".to_string();

        let spacer = crate::config::ShellBarSectionConfig {
            kind: "fill".to_string(),
            ..Default::default()
        };

        let mut a = crate::config::ShellBarSectionConfig {
            kind: "fixed".to_string(),
            cells: 4,
            group: "sys".to_string(),
            color: "red".to_string(),
            ..Default::default()
        };
        a.widget.kind = "label".to_string();
        a.widget.text = "A".to_string();
        let mut b = crate::config::ShellBarSectionConfig {
            kind: "fixed".to_string(),
            cells: 4,
            group: "sys".to_string(),
            ..Default::default()
        };
        b.widget.kind = "label".to_string();
        b.widget.text = "B".to_string();

        let config = crate::config::ShellBarsConfig {
            top: crate::config::ShellBarConfig {
                enabled: true,
                size: 1,
                style: "pills".to_string(),
                sections: vec![cpu, spacer, a, b],
                ..Default::default()
            },
            ..Default::default()
        };

        let frame = Rect::new(0, 0, 100, 30);
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.shell_presentation = crate::ui::shell::ShellPresentationState::from_restored(
            app.sidebar_width,
            false,
            None,
            crate::ui::shell::ShellBars::from_config(&config),
        )
        .with_bar_colors(crate::ui::shell::BarColors::from_config(
            &config,
            &app.palette,
        ));
        app.shell_bar_chrome =
            crate::ui::shell::ShellBarChrome::from_config(&config, true, &app.palette);
        compute_view(&mut app, frame);

        let mut terminal =
            Terminal::new(TestBackend::new(frame.width, frame.height)).expect("test backend");
        terminal
            .draw(|f| render(&app, f))
            .expect("a pills bar draws");
        let buffer = terminal.backend().buffer().clone();

        let (yellow_band, yellow_text) =
            crate::ui::shell::pill_tones(app.palette.yellow, app.palette.bg);
        let (red_band, _) = crate::ui::shell::pill_tones(app.palette.red, app.palette.bg);

        // The first pill: band behind, vivid text on it.
        let first_cell = buffer.cell((0, 0)).expect("cell");
        assert_eq!(first_cell.bg, yellow_band, "the first pill wears its band");
        let label_cell = (0..6u16)
            .filter_map(|x| buffer.cell((x, 0)))
            .find(|cell| cell.symbol() == "C")
            .expect("the label is painted");
        assert_eq!(label_cell.fg, yellow_text, "the text is the vivid tone");

        // The spacer between the pills is the bar's own backdrop.
        let gap_cell = buffer.cell((40, 0)).expect("cell");
        assert_eq!(gap_cell.bg, app.palette.bg, "the fill is not a pill");

        // The grouped run is one continuous red band, both members' cells.
        let grouped: Vec<_> = (0..100u16)
            .filter_map(|x| buffer.cell((x, 0)).map(|cell| (x, cell.clone())))
            .filter(|(_, cell)| cell.bg == red_band)
            .map(|(x, _)| x)
            .collect();
        assert!(
            grouped.len() >= 8,
            "both members sit on one band: {grouped:?}"
        );
        let contiguous = grouped.windows(2).all(|pair| pair[1] == pair[0] + 1);
        assert!(contiguous, "one band, no seam: {grouped:?}");
    }

    /// G6 · a group is ONE frame on the screen: three sections, one box, the
    /// members' widgets inside it. Counted in the buffer, because a chrome
    /// that carries the slots and a geometry that carries the rects are both
    /// present and correct in a build that paints three frames — or none.
    #[test]
    fn a_grouped_run_is_painted_as_one_frame_with_its_members_inside() {
        use ratatui::{backend::TestBackend, Terminal};

        let section = |text: &str| {
            let mut section = crate::config::ShellBarSectionConfig {
                kind: "content".to_string(),
                min: 9,
                max: 9,
                group: "sys".to_string(),
                ..Default::default()
            };
            section.widget.kind = "label".to_string();
            section.widget.text = text.to_string();
            section
        };
        let config = crate::config::ShellBarsConfig {
            top: crate::config::ShellBarConfig {
                enabled: true,
                size: 3,
                border: Some(false),
                sections: vec![section("CPUX"), section("MEMX"), section("SWPX")],
                ..Default::default()
            },
            ..Default::default()
        };

        let frame_area = Rect::new(0, 0, 100, 30);
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.shell_presentation = crate::ui::shell::ShellPresentationState::from_restored(
            app.sidebar_width,
            false,
            None,
            crate::ui::shell::ShellBars::from_config(&config),
        );
        app.shell_bar_chrome =
            crate::ui::shell::ShellBarChrome::from_config(&config, true, &app.palette);
        compute_view(&mut app, frame_area);

        let mut terminal = Terminal::new(TestBackend::new(frame_area.width, frame_area.height))
            .expect("test backend");
        terminal
            .draw(|f| render(&app, f))
            .expect("a grouped bar draws");
        let buffer = terminal.backend().buffer().clone();
        let bar = app.view.shell.regions.get(RegionId::TopBar);

        let mut corners = Vec::new();
        for y in bar.y..bar.bottom() {
            for x in bar.x..bar.right() {
                if buffer.cell((x, y)).is_some_and(|cell| cell.symbol() == "╭") {
                    corners.push((x, y));
                }
            }
        }
        assert_eq!(
            corners.len(),
            1,
            "three grouped sections share ONE frame — one top-left corner, not \
             three and not none: {corners:?}"
        );

        let row: String = (bar.x..bar.right())
            .map(|x| {
                buffer
                    .cell((x, bar.y + 1))
                    .map(|cell| cell.symbol().to_string())
                    .unwrap_or_default()
            })
            .collect();
        for label in ["CPUX", "MEMX", "SWPX"] {
            assert!(
                row.contains(label),
                "every member's widget belongs on the frame's inner row: {row:?}"
            );
        }
        assert!(
            !row.trim_start().starts_with("CPUX"),
            "the first member sits inside the frame, not on the frame column: {row:?}"
        );
    }

    /// I1/I4 · an island is a frame on the screen, drawn inside its bar's own.
    ///
    /// The claim nothing else can make. A tone in the chrome, a rectangle from
    /// the geometry and a widget that reads both are all present and correct in
    /// a build that never paints the frame — the compiler counts every one of
    /// them as used, because they are, just not by anything that draws. This
    /// fork lost a sidebar row for a whole release to exactly that shape, and
    /// only a buffer dump tells the two apart.
    ///
    /// The bar keeps its own frame here on purpose. Two frames at once is a
    /// composition somebody may legitimately want — an outer strip holding
    /// separate islands — so the question is not whether to refuse it but
    /// whether the arithmetic survives it: the island has to land strictly
    /// inside the bar's border rather than on top of it.
    // TP-CHROME-138: an island's frame is painted, inside the bar's own rather
    // than over it, and its widget goes into what that frame leaves.
    #[test]
    fn an_island_draws_its_own_frame_inside_the_bars_and_holds_its_label() {
        use ratatui::{backend::TestBackend, Terminal};

        // `size = 5` because both frames are being drawn: the bar spends two
        // rows on its own, which leaves exactly the three an island needs.
        let bars = |island: bool| {
            let mut first = crate::config::ShellBarSectionConfig {
                kind: "fixed".to_string(),
                cells: 6,
                border: Some(island),
                color: "teal".to_string(),
                ..Default::default()
            };
            first.widget.kind = "label".to_string();
            first.widget.text = "CPU".to_string();

            let spacer = crate::config::ShellBarSectionConfig {
                kind: "fill".to_string(),
                ..Default::default()
            };

            let mut last = crate::config::ShellBarSectionConfig {
                kind: "fixed".to_string(),
                cells: 5,
                border: Some(island),
                ..Default::default()
            };
            last.widget.kind = "label".to_string();
            last.widget.text = "12:00".to_string();

            crate::config::ShellBarsConfig {
                top: crate::config::ShellBarConfig {
                    enabled: true,
                    size: 5,
                    border: Some(true),
                    color: "mauve".to_string(),
                    gradient: Vec::new(),
                    sections: vec![first, spacer, last],
                    ..Default::default()
                },
                ..Default::default()
            }
        };

        let draw = |config: &crate::config::ShellBarsConfig| {
            let frame = Rect::new(0, 0, 100, 30);
            let mut app = crate::app::state::AppState::test_new();
            app.workspaces = vec![Workspace::test_new("one")];
            app.active = Some(0);
            app.selected = 0;
            app.mode = Mode::Terminal;
            app.shell_presentation = crate::ui::shell::ShellPresentationState::from_restored(
                app.sidebar_width,
                false,
                None,
                crate::ui::shell::ShellBars::from_config(config),
            )
            .with_bar_colors(crate::ui::shell::BarColors::from_config(
                config,
                &app.palette,
            ));
            app.shell_bar_chrome =
                crate::ui::shell::ShellBarChrome::from_config(config, true, &app.palette);
            compute_view(&mut app, frame);

            let mut terminal =
                Terminal::new(TestBackend::new(frame.width, frame.height)).expect("test backend");
            terminal
                .draw(|f| render(&app, f))
                .expect("a bar with framed sections draws");
            let buffer = terminal.backend().buffer().clone();
            let bar = app.view.shell.regions.get(RegionId::TopBar);
            let rects = app
                .shell_presentation
                .bars()
                .track_for(RegionId::TopBar)
                .section_rects(RegionId::TopBar, bar);
            (buffer, bar, rects)
        };

        // Only the bar's own rectangle is read: the sidebar and the tab strip
        // draw rounded corners of their own, and counting those would make this
        // pass on a screen with no island anywhere in the bar.
        let corners = |buffer: &ratatui::buffer::Buffer, bar: Rect| {
            let mut found = Vec::new();
            for y in bar.y..bar.bottom() {
                for x in bar.x..bar.right() {
                    if buffer.cell((x, y)).is_some_and(|cell| cell.symbol() == "╭") {
                        found.push((x, y));
                    }
                }
            }
            found
        };

        let (buffer, bar, rects) = draw(&bars(true));

        // I2 at the screen layer: the six cells asked for became eight on the
        // bar, so six survive inside the frame.
        let first = rects.get(0).expect("the first section has a rectangle");
        assert_eq!(
            first.width, 8,
            "a six-cell island must occupy eight so six are left inside it"
        );

        let found = corners(&buffer, bar);
        assert_eq!(
            found.len(),
            3,
            "the bar's own frame and two islands make three top-left corners in \
             the bar's rectangle; found {found:?}"
        );
        assert!(
            found.contains(&(bar.x, bar.y)),
            "the bar keeps its own frame: {found:?}"
        );
        assert!(
            found.contains(&(first.x, first.y)),
            "the first island's frame must start where its rectangle does: \
             {first:?} in {found:?}"
        );
        // I4 · strictly inside, not on top of, the bar's own border.
        assert!(
            first.x > bar.x && first.y > bar.y,
            "an island must land inside the bar's border, not over it: \
             {first:?} in {bar:?}"
        );

        // The label moves down and in with the frame rather than being drawn
        // under it: a frame that overwrote its own content would satisfy every
        // corner assertion above.
        let row = |buffer: &ratatui::buffer::Buffer, rect: Rect, y: u16| -> String {
            (rect.x..rect.right())
                .map(|x| {
                    buffer
                        .cell((x, y))
                        .map(|cell| cell.symbol().to_string())
                        .unwrap_or_default()
                })
                .collect()
        };
        assert!(
            row(&buffer, first, first.y + 1).contains("CPU"),
            "the label belongs on the island's inner row: {:?}",
            row(&buffer, first, first.y + 1)
        );
        assert!(
            !row(&buffer, first, first.y).contains("CPU"),
            "the frame row is the frame's, not the label's: {:?}",
            row(&buffer, first, first.y)
        );

        // The negative control. Without it every assertion above would also
        // pass on a screen whose bar simply draws a lot of corners.
        let (bare_buffer, bare_bar, bare_rects) = draw(&bars(false));
        assert_eq!(
            bare_rects.get(0).expect("a rectangle").width,
            6,
            "a section that asked for no frame is not widened by one"
        );
        assert_eq!(
            corners(&bare_buffer, bare_bar),
            vec![(bare_bar.x, bare_bar.y)],
            "with no island asked for, the bar's own corner is the only one"
        );
    }

    // TC-C8 · a label has to be legible, not merely present.
    //
    // The test above reads symbols, and symbols are in the buffer whether or
    // not a person can see them. This fork shipped a label whose foreground was
    // the colour of the surface under it: every state test passed, the glyph
    // dump passed, and the user saw an empty bar with a visible border. A
    // character-only assertion cannot tell "painted" from "painted invisibly",
    // so the colour needs its own claim.
    //
    // The claim is deliberately the weakest one that still catches it —
    // foreground must differ from the background of the very cell it lands in.
    // Anything stronger (a named colour, a contrast ratio) would pin a theme
    // decision into a test that is about legibility, and would break the next
    // time the palette moves.
    // TP-CHROME-55: a section label is drawn in a colour you can see against
    // its own surface.
    #[test]
    fn a_section_label_is_painted_in_a_colour_you_can_see_against_its_own_surface() {
        use ratatui::{backend::TestBackend, Terminal};

        let mut section = crate::config::ShellBarSectionConfig {
            kind: "fixed".to_string(),
            cells: 6,
            ..Default::default()
        };
        section.widget.kind = "label".to_string();
        section.widget.text = "CPU".to_string();

        let config = crate::config::ShellBarsConfig {
            top: crate::config::ShellBarConfig {
                enabled: true,
                size: 3,
                // The border is the point: it makes the shell paint the bar's
                // interior, which is the surface the label has to survive.
                border: Some(true),
                color: "mauve".to_string(),
                gradient: Vec::new(),
                sections: vec![section],
                ..Default::default()
            },
            ..Default::default()
        };

        let frame = Rect::new(0, 0, 100, 30);
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.shell_presentation = crate::ui::shell::ShellPresentationState::from_restored(
            app.sidebar_width,
            false,
            None,
            crate::ui::shell::ShellBars::from_config(&config),
        );
        app.shell_bar_chrome =
            crate::ui::shell::ShellBarChrome::from_config(&config, true, &app.palette);
        compute_view(&mut app, frame);

        let mut terminal =
            Terminal::new(TestBackend::new(frame.width, frame.height)).expect("test backend");
        terminal
            .draw(|f| render(&app, f))
            .expect("a bordered bar with a labelled section draws");
        let buffer = terminal.backend().buffer().clone();

        // Row 0 is the top border, so the content row is row 1.
        let painted: Vec<_> = (0..10)
            .filter_map(|x| buffer.cell((x, 1)))
            .filter(|cell| {
                let symbol = cell.symbol();
                !symbol.trim().is_empty() && symbol != "│"
            })
            .collect();

        assert!(
            !painted.is_empty(),
            "the label has to reach the buffer before its colour can be judged"
        );
        for cell in painted {
            assert_ne!(
                cell.fg,
                cell.bg,
                "a label drawn in the colour of its own surface is invisible: \
                 symbol {:?} fg {:?} bg {:?}",
                cell.symbol(),
                cell.fg,
                cell.bg
            );
        }
    }

    // TC-D5 · the last link in the chain, which needed its own test because
    // dropping it is invisible everywhere else: the numbers on screen have to
    // come from the sample in state. A render that ignored state and formatted
    // a default would paint `MEM  --`, and `--` is exactly what an unreadable
    // machine paints — so every other test, and the screen itself, would go on
    // looking correct. A mutation that swapped the sample for `Default` left
    // this file's whole suite green until this test existed.
    // TP-RES-10: what a live section paints comes from the sampled state.
    #[test]
    fn a_live_section_paints_the_sample_that_is_in_state_rather_than_a_default() {
        use ratatui::{backend::TestBackend, Terminal};

        let mut section = crate::config::ShellBarSectionConfig {
            kind: "fixed".to_string(),
            cells: 16,
            ..Default::default()
        };
        section.widget.kind = "resource".to_string();
        section.widget.metric = "mem".to_string();

        let config = crate::config::ShellBarsConfig {
            top: crate::config::ShellBarConfig {
                enabled: true,
                size: 1,
                border: Some(false),
                color: String::new(),
                gradient: Vec::new(),
                sections: vec![section],
                ..Default::default()
            },
            ..Default::default()
        };

        let frame = Rect::new(0, 0, 100, 30);
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.shell_presentation = crate::ui::shell::ShellPresentationState::from_restored(
            app.sidebar_width,
            false,
            None,
            crate::ui::shell::ShellBars::from_config(&config),
        );
        app.shell_bar_chrome =
            crate::ui::shell::ShellBarChrome::from_config(&config, true, &app.palette);
        // A reading nothing else could have produced.
        app.resources = crate::resource::ResourceSample {
            mem: Some(crate::resource::Usage {
                used: 7 * 1024 * 1024 * 1024,
                total: 16 * 1024 * 1024 * 1024,
            }),
            ..crate::resource::ResourceSample::default()
        };
        compute_view(&mut app, frame);

        let mut terminal =
            Terminal::new(TestBackend::new(frame.width, frame.height)).expect("test backend");
        terminal
            .draw(|f| render(&app, f))
            .expect("a bar with a live section draws");
        let buffer = terminal.backend().buffer().clone();

        let row: String = (0..16)
            .map(|x| {
                buffer
                    .cell((x, 0))
                    .map(|cell| cell.symbol().to_string())
                    .unwrap_or_default()
            })
            .collect();

        assert!(
            row.contains("7.0G/16G"),
            "the painted numbers must be the sampled ones: {row:?}"
        );
        assert!(
            !row.contains("--"),
            "a section with a reading must not paint the unreadable form: {row:?}"
        );
    }

    /// A bar whose only section is the bundled `herd` mark: ten cells wide,
    /// three cell rows tall, no border so every row is content.
    fn art_bar_config() -> crate::config::ShellBarsConfig {
        let mut section = crate::config::ShellBarSectionConfig {
            kind: "fixed".to_string(),
            cells: 10,
            ..Default::default()
        };
        section.widget.kind = "icon".to_string();
        section.widget.art = "herd".to_string();
        crate::config::ShellBarsConfig {
            top: crate::config::ShellBarConfig {
                enabled: true,
                size: 3,
                border: Some(false),
                color: String::new(),
                gradient: Vec::new(),
                sections: vec![section],
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn app_with_art_bar() -> crate::app::state::AppState {
        let config = art_bar_config();
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.shell_presentation = crate::ui::shell::ShellPresentationState::from_restored(
            app.sidebar_width,
            false,
            None,
            crate::ui::shell::ShellBars::from_config(&config),
        );
        app.shell_bar_chrome =
            crate::ui::shell::ShellBarChrome::from_config(&config, true, &app.palette);
        app
    }

    // TC-I3/TC-I4 · a picture occupies every row it was given, and the upper
    // pixel is the foreground.
    //
    // Both halves need saying. The rectangle was always three rows tall —
    // `section_rects` hands each section the bar's whole inner height — and the
    // widget renderer clipped it to one, so a picture was impossible for a
    // reason no geometry test could see. And the half-block mapping is
    // invisible when wrong: swapping foreground and background draws the mark
    // upside down, which looks like a design choice rather than a bug.
    // TP-ART-01: a picture paints its whole rectangle, upper pixel first.
    #[test]
    fn a_picture_paints_every_row_it_was_given_with_the_upper_pixel_in_the_foreground() {
        use ratatui::{backend::TestBackend, Terminal};

        let frame = Rect::new(0, 0, 100, 30);
        let mut app = app_with_art_bar();
        compute_view(&mut app, frame);

        let mut terminal =
            Terminal::new(TestBackend::new(frame.width, frame.height)).expect("test backend");
        terminal
            .draw(|f| render(&app, f))
            .expect("a bar with a picture draws");
        let buffer = terminal.backend().buffer().clone();

        // The mark is three cell rows, and every one of them must carry paint.
        for row in 0..3 {
            let painted = (0..10)
                .filter_map(|x| buffer.cell((x, row)))
                .filter(|cell| cell.symbol() == "▀" || cell.symbol() == "▄")
                .count();
            assert!(
                painted > 0,
                "row {row} of the picture is empty; the renderer is still clipping to one line"
            );
        }

        // Row 0 of `herd` is `..a....a..` over `...a..a...`: column 2 has a
        // pixel above and none below, so it is an upper half in the mark's
        // first colour with no background of its own.
        let cell = buffer.cell((2, 0)).expect("column 2 exists");
        assert_eq!(
            cell.symbol(),
            "▀",
            "a pixel with nothing under it is an upper half"
        );
        assert_eq!(
            cell.fg, app.palette.mauve,
            "the upper pixel is the foreground colour"
        );

        // Column 3 is the other way round: transparent above, a pixel below.
        let cell = buffer.cell((3, 0)).expect("column 3 exists");
        assert_eq!(
            cell.symbol(),
            "▄",
            "a pixel with nothing above it is a lower half, so transparency \
             never needs an invented background"
        );

        // The cell that carries BOTH pixels is the one that pins the mapping.
        // Cell row 1 is pixel rows 2 and 3 — `....aa....` over `....bb....` —
        // so column 4 is the first colour above the second. Asserting only on
        // a cell whose lower half is transparent leaves the two-colour branch
        // untested, and swapping foreground for background there draws the
        // whole mark upside down while every other assertion still passes. A
        // mutation proved exactly that before this block existed.
        let both = buffer
            .cell((4, 1))
            .expect("column 4 of the second row exists");
        assert_eq!(both.symbol(), "▀");
        assert_eq!(
            both.fg, app.palette.mauve,
            "the upper pixel must be the foreground"
        );
        assert_eq!(
            both.bg, app.palette.teal,
            "the lower pixel must be the background"
        );

        // Nothing outside the ten cells the picture declared.
        let beyond = buffer.cell((10, 0)).expect("column 10 exists");
        assert!(
            beyond.symbol() != "▀" && beyond.symbol() != "▄",
            "the picture painted past its own rectangle: {:?}",
            beyond.symbol()
        );
    }

    // TC-I7 · the user's own rule, as a test: "diff yoksa trafik yok".
    //
    // herdr's server sends a cell diff, so an unchanged picture costs nothing —
    // but only if drawing it twice really does produce the same cells. A
    // renderer that reached for a clock, a counter, or an allocation-ordered
    // map would still look right on screen and would make the diff resend the
    // whole mark on every frame, which is the failure this whole design exists
    // to avoid and the one nothing else would catch.
    // TP-ART-02: a picture that has not changed produces an identical buffer.
    #[test]
    fn drawing_the_same_picture_twice_produces_an_identical_buffer_so_the_diff_sends_nothing() {
        use ratatui::{backend::TestBackend, Terminal};

        let frame = Rect::new(0, 0, 100, 30);
        let mut app = app_with_art_bar();
        compute_view(&mut app, frame);

        let mut terminal =
            Terminal::new(TestBackend::new(frame.width, frame.height)).expect("test backend");
        terminal.draw(|f| render(&app, f)).expect("first frame");
        let first = terminal.backend().buffer().clone();
        terminal.draw(|f| render(&app, f)).expect("second frame");
        let second = terminal.backend().buffer().clone();

        assert_eq!(
            first, second,
            "two draws of the same state differ, so the cell diff would resend the bar \
             on every frame"
        );
    }

    fn app_with_meter(
        metric: &str,
        cells: u16,
        sample: crate::resource::ResourceSample,
    ) -> crate::app::state::AppState {
        let mut section = crate::config::ShellBarSectionConfig {
            kind: "fixed".to_string(),
            cells,
            ..Default::default()
        };
        section.widget.kind = "meter".to_string();
        section.widget.metric = metric.to_string();
        let config = crate::config::ShellBarsConfig {
            top: crate::config::ShellBarConfig {
                enabled: true,
                size: 2,
                border: Some(false),
                color: String::new(),
                gradient: Vec::new(),
                sections: vec![section],
                ..Default::default()
            },
            ..Default::default()
        };
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.shell_presentation = crate::ui::shell::ShellPresentationState::from_restored(
            app.sidebar_width,
            false,
            None,
            crate::ui::shell::ShellBars::from_config(&config),
        );
        app.shell_bar_chrome =
            crate::ui::shell::ShellBarChrome::from_config(&config, true, &app.palette);
        app.resources = sample;
        app
    }

    // TC-M4 · the bar fills every row it was given, stops where the value says,
    // and stays inside its own rectangle.
    //
    // All three matter and none is implied by the others. One row would read as
    // a line rather than a block; a bar that stopped at the wrong cell would
    // still look like a plausible reading; and one cell of overrun paints the
    // section next door, which is the defect this fork has already paid for
    // twice in other surfaces.
    // TP-METER-01: a meter fills its rows, honours the value, stays inside.
    #[test]
    fn a_meter_fills_every_row_to_the_level_it_was_given_and_no_further() {
        use ratatui::{backend::TestBackend, Terminal};

        let mut app = app_with_meter(
            "mem",
            10,
            crate::resource::ResourceSample {
                mem: Some(crate::resource::Usage { used: 3, total: 10 }),
                ..crate::resource::ResourceSample::default()
            },
        );
        let frame = Rect::new(0, 0, 100, 30);
        compute_view(&mut app, frame);

        let mut terminal =
            Terminal::new(TestBackend::new(frame.width, frame.height)).expect("test backend");
        terminal.draw(|f| render(&app, f)).expect("a meter draws");
        let buffer = terminal.backend().buffer().clone();

        for row in 0..2 {
            let filled = (0..10)
                .filter_map(|x| buffer.cell((x, row)))
                .filter(|cell| cell.symbol() == "\u{2588}")
                .count();
            assert_eq!(
                filled, 3,
                "row {row}: three tenths of ten cells is three full cells"
            );
        }

        // Green at 30%, and the colour is the whole point of a glanceable bar.
        let cell = buffer.cell((0, 0)).expect("first cell");
        assert_eq!(cell.fg, app.palette.green, "30% is not a problem yet");

        // Nothing past the section.
        let beyond = buffer.cell((10, 0)).expect("column 10");
        assert_ne!(
            beyond.symbol(),
            "\u{2588}",
            "the meter painted into its neighbour"
        );
    }

    // TC-M5 · a level that is a problem looks like one, and a metric with no
    // ratio draws NOTHING rather than an empty bar.
    //
    // The second half is the honest one: an empty bar says "plenty free", which
    // is a claim. About an unreadable counter or a machine with no swap, it is
    // a false one — the same lie a fabricated 0% would be.
    // TP-METER-02: level changes colour; no ratio draws nothing.
    #[test]
    fn a_full_meter_reads_red_and_a_metric_with_no_ratio_draws_nothing() {
        use ratatui::{backend::TestBackend, Terminal};

        let mut app = app_with_meter(
            "mem",
            8,
            crate::resource::ResourceSample {
                mem: Some(crate::resource::Usage {
                    used: 19,
                    total: 20,
                }),
                ..crate::resource::ResourceSample::default()
            },
        );
        // 100x30: below roughly this the shell projects no top bar at all, which
        // is a property of the layout rather than of the meter.
        let frame = Rect::new(0, 0, 100, 30);
        compute_view(&mut app, frame);
        let mut terminal =
            Terminal::new(TestBackend::new(frame.width, frame.height)).expect("test backend");
        terminal
            .draw(|f| render(&app, f))
            .expect("a full meter draws");
        let cell = terminal
            .backend()
            .buffer()
            .cell((0, 0))
            .expect("first cell")
            .clone();
        assert_eq!(cell.fg, app.palette.red, "95% has to read as a problem");

        // Same section, a metric this machine cannot answer for.
        let mut app = app_with_meter("swap", 8, crate::resource::ResourceSample::default());
        compute_view(&mut app, frame);
        let mut terminal =
            Terminal::new(TestBackend::new(frame.width, frame.height)).expect("test backend");
        terminal
            .draw(|f| render(&app, f))
            .expect("an unknown meter draws");
        let buffer = terminal.backend().buffer().clone();
        let painted = (0..8)
            .filter_map(|x| buffer.cell((x, 0)))
            .filter(|cell| cell.symbol() != " ")
            .count();
        assert_eq!(
            painted, 0,
            "an empty bar would claim the pool is free; it is unknown"
        );
    }

    #[test]
    fn a_configured_left_bar_puts_the_finished_dock_on_screen() {
        use ratatui::{backend::TestBackend, Terminal};

        let frame = Rect::new(0, 0, 100, 30);
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        // Built from a config table rather than from the internal type, so
        // this walks the same path a person's `[shell.bars]` walks.
        let bars = crate::ui::shell::ShellBars::from_config(&crate::config::ShellBarsConfig {
            left: crate::config::ShellBarConfig {
                enabled: true,
                size: 5,
                border: Some(false),
                color: String::new(),
                gradient: Vec::new(),
                sections: Vec::new(),
                ..Default::default()
            },
            top: crate::config::ShellBarConfig {
                enabled: true,
                size: 1,
                border: Some(false),
                color: String::new(),
                gradient: Vec::new(),
                sections: Vec::new(),
                ..Default::default()
            },
            ..Default::default()
        });
        app.shell_presentation = crate::ui::shell::ShellPresentationState::from_restored(
            app.sidebar_width,
            false,
            None,
            bars,
        );

        compute_view(&mut app, frame);
        let dock = app.view.shell.regions.get(RegionId::AppDock);
        let top = app.view.shell.regions.get(RegionId::TopBar);
        assert_eq!(dock.width, 5, "the left bar owns the columns it asked for");
        assert_eq!(top.height, 1, "and the top bar the rows it asked for");

        let mut terminal =
            Terminal::new(TestBackend::new(frame.width, frame.height)).expect("test backend");
        terminal
            .draw(|f| render(&app, f))
            .expect("the shell draws with bars configured");
        let buffer = terminal.backend().buffer().clone();

        let painted = |rect: Rect| {
            (rect.y..rect.y + rect.height).any(|y| {
                (rect.x..rect.x + rect.width)
                    .any(|x| buffer.cell((x, y)).is_some_and(|cell| cell.symbol() != " "))
            })
        };

        assert!(
            painted(dock),
            "the dock is a finished component; a rectangle is all it was missing"
        );
        assert!(
            !painted(top),
            "the top bar has no renderer yet — when one lands, this line is the \
             reminder to change it deliberately rather than discover it"
        );
    }

    // T36 · the reader asked for rounded corners in a warm tone, and asked to
    // be sure they were there. Unicode has no thick *rounded* corner, so the
    // weight comes from BOLD on the rounded set; squaring the corners to get a
    // heavy glyph would trade away the thing that was actually requested.
    #[test]
    fn a_bordered_bar_draws_rounded_corners_in_its_configured_tone() {
        use ratatui::{backend::TestBackend, style::Modifier, Terminal};

        let frame = Rect::new(0, 0, 100, 30);
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;

        let table = crate::config::ShellBarsConfig {
            top: crate::config::ShellBarConfig {
                enabled: true,
                size: 3,
                border: Some(true),
                color: "orange".to_string(),
                gradient: Vec::new(),
                sections: Vec::new(),
                ..Default::default()
            },
            ..Default::default()
        };
        app.shell_presentation = crate::ui::shell::ShellPresentationState::from_restored(
            app.sidebar_width,
            false,
            None,
            crate::ui::shell::ShellBars::from_config(&table),
        )
        .with_bar_colors(crate::ui::shell::BarColors::from_config(
            &table,
            &app.palette,
        ));

        compute_view(&mut app, frame);
        let bar = app.view.shell.regions.get(RegionId::TopBar);
        assert_eq!(bar.height, 3, "three rows: border, content, border");

        let mut terminal =
            Terminal::new(TestBackend::new(frame.width, frame.height)).expect("test backend");
        terminal
            .draw(|f| render(&app, f))
            .expect("a bordered bar draws");
        let buffer = terminal.backend().buffer().clone();

        let cell = |x: u16, y: u16| buffer.cell((x, y)).expect("inside the frame").clone();
        let top_left = cell(bar.x, bar.y);
        assert_eq!(top_left.symbol(), "╭", "the corner is round, not square");
        assert_eq!(cell(bar.x + bar.width - 1, bar.y).symbol(), "╮");
        assert_eq!(cell(bar.x, bar.y + bar.height - 1).symbol(), "╰");
        assert_eq!(
            cell(bar.x + bar.width - 1, bar.y + bar.height - 1).symbol(),
            "╯"
        );

        assert_eq!(
            top_left.fg, app.palette.peach,
            "`orange` resolves through the palette, so the tone follows the theme"
        );
        assert!(
            top_left.modifier.contains(Modifier::BOLD),
            "weight comes from BOLD, since a thick rounded corner does not exist"
        );
    }

    // T10b · the restored identity reaches geometry, not just the file.
    //
    // The read half of a round trip is the half that can be quietly skipped:
    // the writer keeps working, the file keeps looking right, and nothing ever
    // goes red. This asserts the draw path asks the presentation which tree to
    // derive — the default answer is still the legacy tree, which is why the
    // composition baselines are untouched.
    #[test]
    fn the_draw_path_derives_the_tree_the_presentation_restored() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        let frame = Rect::new(0, 0, 100, 30);

        compute_view(&mut app, frame);
        assert_eq!(
            app.view.shell.regions.get(RegionId::AppDock),
            Rect::default(),
            "the default presentation is the legacy tree, which has no dock"
        );

        app.shell_presentation = crate::ui::shell::ShellPresentationState::from_restored(
            app.sidebar_width,
            false,
            Some(crate::ui::shell::ShellTemplateId::DockSidebarStage),
            crate::ui::shell::ShellBars::NONE,
        );
        compute_view(&mut app, frame);

        let dock = app.view.shell.regions.get(RegionId::AppDock);
        assert!(
            dock.width > 0 && dock.height > 0,
            "a restored template must reach the projection, not stop at the file"
        );
        assert_eq!(dock.x, frame.x, "the dock owns the leading edge");
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

        // Desktop: Files is a peer tab, so the shell tab strip takes one row
        // (TP-FTAB-CHROME-01; before 2026-07-25 Files reclaimed it) -> height 8
        // = tab strip 1, FM header 1, panel title 1, status 1, list 4. The
        // heights below grew by that one row so the list sizes under test are
        // unchanged.
        compute_view(&mut app, Rect::new(0, 0, 100, 8));
        assert_eq!(
            app.file_manager.as_ref().expect("open fm").viewport_start,
            5
        );

        // Expanding to nine list rows clamps the old start to max_start=1.
        compute_view(&mut app, Rect::new(0, 0, 100, 13));
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

    // TP-FMN-NAV-08 RED: the composed Files projection must preserve the
    // canonical Trail focus. The legacy Miller projection runs first during
    // `compute_view`, but its viewport may not drag an inactive preview child
    // into focus before the Trail projection is published.
    #[test]
    fn compute_view_auto_follow_tracks_active_trail_owner() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "herdr-ui-active-trail-owner-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let child = root.join("00-child");
        std::fs::create_dir_all(&child).expect("create active-owner fixture");
        std::fs::write(child.join("inside.txt"), b"x").expect("write active-owner fixture");

        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.mobile_width_threshold = 0;
        app.try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&root)))
            .expect("Files activation");
        assert_eq!(
            app.file_manager
                .as_ref()
                .expect("open file manager")
                .trail
                .active_col(),
            0
        );

        compute_view(&mut app, Rect::new(0, 0, 120, 40));

        assert_eq!(
            app.view.file_manager_trail.offset_cells, 0,
            "composed auto-follow preserves the owner column"
        );
        assert_eq!(
            app.view.file_manager_miller.focused_chain_index,
            Some(0),
            "legacy geometry reports the same canonical focus"
        );
        std::fs::remove_dir_all(root).expect("remove active-owner fixture");
    }

    // TP-C2.1-VIEWSTATE: desktop compute_view snapshots CURRENT name and action
    // rects from one geometry source, then clears both when FM closes so stale
    // terminal coordinates can never remain clickable.

    // TP-SBS-FILES-01: with Files riding the right half, the terminal keeps
    // the left, the file manager is drawn — and hit-projected — strictly
    // inside the right rectangle, and the stage stays on the terminal.
    #[test]
    fn files_beside_projects_the_file_manager_into_the_right_half() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "herdr-files-beside-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).expect("temp root");
        std::fs::write(root.join("a.txt"), b"x").expect("file");

        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("left")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&root)))
            .expect("Files activation");
        // Back to the terminal: Files is resident but backgrounded…
        app.show_terminal_workspace();
        assert_eq!(
            app.stage.surface_view(),
            crate::ui::surface_host::StageSurfaceView::TerminalWorkspace
        );
        // …and then summoned beside it.
        app.enter_files_beside();
        assert!(app.files_beside_active());

        let frame = Rect::new(0, 0, 120, 30);
        compute_view(&mut app, frame);

        let right = app
            .view
            .right_surface
            .as_ref()
            .expect("the right surface was computed");
        assert_eq!(right.right, crate::app::state::SideBySideRight::Files);
        assert!(right.pane_infos.is_empty(), "Files hosts no panes");
        // The staged exception keeps the action bar alive while Files rides
        // the right half — New/Copy and friends do not vanish beside a
        // terminal stage.
        assert!(
            app.view.file_manager_action_bar.is_some(),
            "the Files action bar is computed while riding the right half"
        );
        // The stage stays on the terminal; the pairing survives compute.
        assert!(app.files_beside_active());
        // Every Files hit lives inside the right rectangle.
        let rows = &app.view.file_manager_row_areas;
        assert!(!rows.is_empty(), "the file rows were projected");
        for row in rows {
            assert!(
                row.rect.x >= right.area.x,
                "a Files row leaked left of the divider: {:?} vs x={}",
                row.rect,
                right.area.x
            );
        }
        std::fs::remove_dir_all(root).expect("remove fixture");
    }

    // TP-SBS-FILES-01: the pairing self-heals in compute — Files on the
    // right with no resident file manager drops the mode whole instead of
    // drawing a vacant half.
    #[test]
    fn a_missing_file_manager_heals_the_files_pairing() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("left")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        assert!(
            app.file_manager.is_none(),
            "precondition: no resident Files"
        );
        app.side_by_side = Some(crate::app::state::SideBySideView {
            right: crate::app::state::SideBySideRight::Files,
            ratio_percent: 50,
            focus: Default::default(),
        });

        compute_view(&mut app, Rect::new(0, 0, 120, 30));

        assert!(
            app.side_by_side.is_none(),
            "the vacant Files pairing was dropped whole"
        );
        assert!(
            app.view.right_surface.is_none(),
            "no right surface is computed for the healed pairing"
        );
    }

    // TP-STAGE-SBS-01: with a pairing on, compute carves the stage in two —
    // the active workspace's panes stay strictly left of the divider, the
    // right half's panes strictly right of it, and the divider column
    // belongs to neither.
    #[test]
    fn the_side_by_side_split_computes_two_disjoint_surfaces() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("solak"), Workspace::test_new("sagdic")];
        app.active = Some(0);
        app.mode = Mode::Terminal;
        app.ensure_test_terminals();
        assert!(app.enter_side_by_side(1));

        compute_view(&mut app, Rect::new(0, 0, 120, 30));

        let right = app
            .view
            .right_surface
            .as_ref()
            .expect("the right surface was computed");
        assert_eq!(
            right.right,
            crate::app::state::SideBySideRight::Workspace(1)
        );
        let divider_x = right.area.x - 1;
        assert!(
            !app.view.pane_infos.is_empty() && !right.pane_infos.is_empty(),
            "both halves carry panes"
        );
        for info in &app.view.pane_infos {
            assert!(
                info.rect.right() <= divider_x,
                "a left pane crossed the divider: {:?} vs x={divider_x}",
                info.rect
            );
        }
        for info in &right.pane_infos {
            assert!(
                info.rect.x >= right.area.x,
                "a right pane crossed back: {:?}",
                info.rect
            );
        }
        assert_eq!(
            right.strip_rect.y,
            app.view.terminal_area.y - 1,
            "the right strip rides directly above its content"
        );
    }

    // TP-STAGE-SBS-01: the paint is real — the divider column and the right
    // half's own strip land in the buffer, so the split is a thing the eye
    // gets, not only a geometry the tests read.
    #[test]
    fn the_stage_paints_the_divider_and_the_right_strip() {
        use ratatui::{backend::TestBackend, Terminal};

        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("solak"), Workspace::test_new("sagdic")];
        app.active = Some(0);
        app.mode = Mode::Terminal;
        app.ensure_test_terminals();
        assert!(app.enter_side_by_side(1));
        compute_view(&mut app, Rect::new(0, 0, 120, 30));
        let (divider_x, strip) = {
            let right = app.view.right_surface.as_ref().expect("right computed");
            (right.area.x - 1, right.strip_rect)
        };

        let mut terminal = Terminal::new(TestBackend::new(120, 30)).expect("test backend");
        terminal
            .draw(|frame| render(&app, frame))
            .expect("the split stage draws");
        let buffer = terminal.backend().buffer();
        assert_eq!(
            buffer[(divider_x, strip.y + 2)].symbol(),
            "\u{2502}",
            "the divider column is painted"
        );
        let strip_text: String = (strip.x..strip.right())
            .map(|x| buffer[(x, strip.y)].symbol().to_string())
            .collect();
        assert!(
            strip_text.contains("sagdic"),
            "the right strip names its workspace: {strip_text:?}"
        );
    }

    // TP-STAGE-SBS-01: a pairing that stopped making sense heals on compute
    // — right vanished, right == active, or a stage too narrow — rather
    // than every consumer re-checking it.
    // TP-SBS-DRAG-01: the ratio the drag commits is the geometry the next
    // frame draws — the divider's own x moves with it, on the cell layer.
    #[test]
    fn the_sbs_divider_rect_follows_the_ratio() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("l"), Workspace::test_new("r")];
        app.active = Some(0);
        app.mode = Mode::Terminal;
        app.ensure_test_terminals();
        app.side_by_side = Some(crate::app::state::SideBySideView {
            right: crate::app::state::SideBySideRight::Workspace(1),
            ratio_percent: 20,
            focus: Default::default(),
        });
        compute_view(&mut app, Rect::new(0, 0, 120, 30));
        let narrow = app
            .view
            .sbs_divider_rect
            .expect("a split names its divider");

        app.side_by_side = Some(crate::app::state::SideBySideView {
            right: crate::app::state::SideBySideRight::Workspace(1),
            ratio_percent: 80,
            focus: Default::default(),
        });
        compute_view(&mut app, Rect::new(0, 0, 120, 30));
        let wide = app.view.sbs_divider_rect.expect("still split");

        assert!(
            narrow.x < wide.x,
            "a bigger ratio moves the divider right: {} vs {}",
            narrow.x,
            wide.x
        );
        assert_eq!(wide.width, 1, "the handle is the one empty column");

        app.side_by_side = None;
        compute_view(&mut app, Rect::new(0, 0, 120, 30));
        assert!(
            app.view.sbs_divider_rect.is_none(),
            "no split, no handle to grab"
        );
    }

    #[test]
    fn an_invalid_pairing_heals_on_compute() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("solo")];
        app.active = Some(0);
        app.mode = Mode::Terminal;
        app.ensure_test_terminals();
        assert!(
            !app.enter_side_by_side(0),
            "the same world twice is refused at the door"
        );
        app.side_by_side = Some(crate::app::state::SideBySideView {
            right: crate::app::state::SideBySideRight::Workspace(7),
            ratio_percent: 50,
            focus: Default::default(),
        });
        compute_view(&mut app, Rect::new(0, 0, 120, 30));
        assert!(
            app.side_by_side.is_none(),
            "a vanished right half healed away"
        );
        assert!(app.view.right_surface.is_none());
    }

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
        // This test pins row-ACTION hit snapshots, so the opt-in buttons
        // must be on (TP-FM-ACTIONS-01 ships them hidden).
        app.files_show_row_actions = true;

        // Preserve the characterized Trail viewport after FCL-3 adds the
        // 24-cell content rail and one-cell separator inside CenterContent,
        // and after TP-FTAB-CHROME-01 gives the shell tab strip one row of the
        // stage: the height grew from 6 to 7 so the four visible Trail lines
        // under test are unchanged.
        compute_view(&mut app, Rect::new(0, 0, 125, 7));
        assert_eq!(
            app.view
                .file_manager_row_areas
                .iter()
                .map(|row| row.entry_idx)
                .collect::<Vec<_>>(),
            vec![0, 1, 2],
            "one date header plus three entries fill the four visible Trail lines"
        );
        assert_eq!(
            app.view.file_manager_trail.columns[0].section_headers.len(),
            1,
            "the grouped projection publishes the date header as non-row geometry"
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
            [0, 1, 2]
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
                crate::app::state::FileManagerHeaderAction::SendTailscale,
                crate::app::state::FileManagerHeaderAction::Compress,
                crate::app::state::FileManagerHeaderAction::Search,
                crate::app::state::FileManagerHeaderAction::CopyPath,
                crate::app::state::FileManagerHeaderAction::More,
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
        compute_view(&mut app, Rect::new(0, 0, 125, 6));
        assert!(app.view.file_manager_row_areas.is_empty());
        assert!(app.view.file_manager_row_action_areas.is_empty());
        assert!(app.view.file_manager_header_action_areas.is_empty());

        std::fs::remove_dir_all(root).expect("remove temp root");
    }

    // P1.4: the inline preview is existing published Files behavior. A
    // snapshot cutover must type and place it instead of silently dropping it.

    // P1 RED: the pure FM1.3 geometry exists, but production `compute_view`
    // must consume it and persist the clamped horizontal origin. This uses a
    // prepared in-memory logical chain; compute is forbidden from loading any
    // of these synthetic paths.

    // TP-TRAIL-T7-RENDER-01: compute_view publishes the exact Trail geometry
    // for the live Native Files frame and retires it with the surface.
    #[test]
    fn compute_view_publishes_live_trail_snapshot_and_clears_it_outside_files() {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("trail-render")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app.mobile_width_threshold = 0;
        app.sidebar_collapsed = true;
        app.sidebar_collapsed_mode = crate::config::SidebarCollapsedModeConfig::Hidden;
        let root = std::env::current_dir().expect("current directory");
        app.try_open_file_manager_with(|_| Some(crate::fm::FmState::new(root)))
            .expect("Files activation");

        compute_view(&mut app, Rect::new(0, 0, 86, 12));

        let fm = app.file_manager.as_ref().expect("open FM");
        let files_generation = app
            .stage
            .active_instance_generation()
            .expect("active Files generation");
        assert_eq!(
            app.view.file_manager_locations.files_generation,
            Some(files_generation)
        );
        assert_ne!(
            app.view.file_manager_locations.layout.mode,
            file_manager::locations::FileManagerLocationsMode::Compact
        );
        assert_eq!(
            app.view.file_manager_trail.columns.len(),
            fm.trail.cols().len()
        );
        assert_eq!(
            app.view.file_manager_trail.columns[0].directory,
            fm.trail.cols()[0].directory
        );
        assert!(app.view.file_manager_trail.columns.iter().all(|column| {
            column.rect.x >= app.view.file_manager_locations.layout.trail.x
                && column.rect.right() <= app.view.file_manager_locations.layout.trail.right()
        }));

        app.close_file_manager();
        compute_view(&mut app, Rect::new(0, 0, 86, 12));

        assert_eq!(
            app.view.file_manager_trail,
            file_manager::trail_view::TrailViewSnapshot::default()
        );
        assert_eq!(
            app.view.file_manager_locations,
            file_manager::locations::FileManagerLocationsView::default()
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
    fn compute_view_profile_records_desktop_and_mobile_invocations() {
        let mut app = AppState::test_new();
        app.mobile_width_threshold = 60;

        let (_, profile) = crate::render_prof::observe_for_test(|| {
            compute_view(&mut app, Rect::new(0, 0, 120, 40));
            compute_view(&mut app, Rect::new(0, 0, 40, 20));
        });

        assert_eq!(
            profile.duration_count("shell.compute_view"),
            2,
            "the shared compute seam records desktop and mobile early-return paths"
        );
    }

    #[test]
    fn performance_workloads_meet_frozen_budgets() {
        const WARM_UP_SAMPLES: usize = if cfg!(debug_assertions) { 2 } else { 16 };
        const MEASURED_SAMPLES: usize = if cfg!(debug_assertions) { 10 } else { 100 };
        const COMPUTE_BUDGET: std::time::Duration = std::time::Duration::from_micros(500);
        const FRAME_120_BUDGET: std::time::Duration = std::time::Duration::from_millis(8);
        const FRAME_240_BUDGET: std::time::Duration = std::time::Duration::from_millis(16);

        fn p95(
            warm_up_samples: usize,
            measured_samples: usize,
            mut workload: impl FnMut(),
        ) -> std::time::Duration {
            for _ in 0..warm_up_samples {
                workload();
            }
            let mut samples = Vec::with_capacity(measured_samples);
            for _ in 0..measured_samples {
                let started = std::time::Instant::now();
                workload();
                samples.push(started.elapsed());
            }
            samples.sort_unstable();
            let rank = measured_samples.saturating_mul(95).div_ceil(100);
            samples[rank.saturating_sub(1)]
        }

        let (mut app, _) = prepared_miller_projection_app(32, 31);
        let medium = Rect::new(0, 0, 120, 40);
        let large = Rect::new(0, 0, 240, 80);

        let compute_medium = p95(WARM_UP_SAMPLES, MEASURED_SAMPLES, || {
            compute_view(&mut app, medium);
        });
        let compute_large = p95(WARM_UP_SAMPLES, MEASURED_SAMPLES, || {
            compute_view(&mut app, large);
        });
        compute_view(&mut app, medium);
        let frame_medium = p95(WARM_UP_SAMPLES, MEASURED_SAMPLES, || {
            std::hint::black_box(render_full_frame_for_test(&app, medium));
        });
        compute_view(&mut app, large);
        let frame_large = p95(WARM_UP_SAMPLES, MEASURED_SAMPLES, || {
            std::hint::black_box(render_full_frame_for_test(&app, large));
        });

        let file_manager = app.file_manager.as_ref().expect("open Files");
        assert_eq!(
            file_manager.miller.chain.len(),
            file_manager.trail.cols().len(),
            "layout preferences mirror the canonical Trail topology"
        );
        assert!(file_manager.miller.chain.len() <= crate::fm::miller::MAX_MILLER_HISTORY_DEPTH);
        assert!((1..=5).contains(&app.view.file_manager_miller.columns.len()));
        if cfg!(debug_assertions) {
            return;
        }

        eprintln!(
            "Miller release p95: compute_120={}us compute_240={}us frame_120={}us frame_240={}us",
            compute_medium.as_micros(),
            compute_large.as_micros(),
            frame_medium.as_micros(),
            frame_large.as_micros()
        );

        assert!(
            compute_medium <= COMPUTE_BUDGET,
            "120x40 Miller compute p95={}us exceeds {}us",
            compute_medium.as_micros(),
            COMPUTE_BUDGET.as_micros()
        );
        assert!(
            compute_large <= COMPUTE_BUDGET,
            "240x80 Miller compute p95={}us exceeds {}us",
            compute_large.as_micros(),
            COMPUTE_BUDGET.as_micros()
        );
        assert!(
            frame_medium <= FRAME_120_BUDGET,
            "120x40 Miller full-frame p95={}us exceeds {}us",
            frame_medium.as_micros(),
            FRAME_120_BUDGET.as_micros()
        );
        assert!(
            frame_large <= FRAME_240_BUDGET,
            "240x80 Miller full-frame p95={}us exceeds {}us",
            frame_large.as_micros(),
            FRAME_240_BUDGET.as_micros()
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
        crate::fm::pin_equal_fixture_mtimes(&[&root.join("a.txt"), &root.join("b.txt")]);

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

        // Control: with the terminal surface active the tab strip is a real,
        // non-empty region, and the Files surface must keep it rather than
        // reclaim it.
        app.close_file_manager();
        compute_view_with_runtime_registry(&mut app, &terminal_runtimes, area);
        assert!(
            !app.view.tab_bar_rect.is_empty(),
            "control: the terminal surface owns a visible tab bar"
        );
        let terminal_surface_strip = app.view.tab_bar_rect;

        app.try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&root)))
            .expect("Files reactivation");
        compute_view_with_runtime_registry(&mut app, &terminal_runtimes, area);
        let stage = app
            .view
            .shell
            .regions
            .get(crate::ui::shell::RegionId::WorkspaceStage);
        // Re-baselined 2026-07-25 (TP-FTAB-CHROME-01/02). Until then the tab
        // strip was terminal-app chrome and Files reclaimed its row, owning the
        // COMPLETE WorkspaceStage. Files is now a peer tab in that same strip,
        // so it owns the stage BELOW the strip and the strip itself is
        // byte-identical to the terminal surface's. The surface-exclusivity
        // contract is unchanged: only the content below the strip switches.
        assert_eq!(
            app.view.tab_bar_rect, terminal_surface_strip,
            "the Files surface must keep the identical tab strip, not reclaim it"
        );
        assert_eq!(
            app.view.terminal_area,
            Rect::new(
                stage.x,
                stage.y.saturating_add(1),
                stage.width,
                stage.height.saturating_sub(1)
            ),
            "active NativeFiles must own exactly the WorkspaceStage below the tab strip"
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

        // Collapsed sidebar: the wider stage stays owned by Files below the
        // strip. Collapsing changes the stage's width, never the strip's row.
        app.sidebar_collapsed = true;
        compute_view_with_runtime_registry(&mut app, &terminal_runtimes, area);
        let collapsed_stage = app
            .view
            .shell
            .regions
            .get(crate::ui::shell::RegionId::WorkspaceStage);
        assert!(collapsed_stage.width > stage.width);
        assert_eq!(
            app.view.terminal_area,
            Rect::new(
                collapsed_stage.x,
                collapsed_stage.y.saturating_add(1),
                collapsed_stage.width,
                collapsed_stage.height.saturating_sub(1)
            )
        );
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
            FileManagerLocationIcon, FileManagerLocationItem, FileManagerLocationsModel,
            FileManagerOperationKind, FileManagerOperationState, FileManagerOperationStatus,
            SidebarTab,
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
        app.file_manager_locations_model = FileManagerLocationsModel::from_sources(
            vec![FileManagerLocationItem {
                label: "Visual fixture".into(),
                path: root.to_path_buf(),
                icon: FileManagerLocationIcon::Home,
                accessible: true,
                ejectable: false,
            }],
            Vec::new(),
            Vec::new(),
        );
        app.try_open_file_manager_with(|_| Some(file_manager))
            .expect("Files activation");
        assert!(app
            .file_manager_locations
            .activate_location(root, &app.file_manager_locations_model));
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
        let equal_modified = std::time::UNIX_EPOCH + std::time::Duration::from_secs(10);
        for path in [root.join("child"), root.join("peer.txt")] {
            std::fs::File::open(path)
                .expect("open visual composition mtime fixture")
                .set_times(std::fs::FileTimes::new().set_modified(equal_modified))
                .expect("set visual composition fixture mtime");
        }
        let mut app = native_fm_visual_composition_app(&root);

        let desktop = Rect::new(0, 0, 120, 24);
        compute_view(&mut app, desktop);
        assert_eq!(app.view.layout, ViewLayout::Desktop);
        assert!(
            app.file_manager
                .as_ref()
                .is_some_and(|file_manager| file_manager.miller.horizontal.follow_active),
            "rendering alone must preserve active-column auto-follow"
        );
        let expanded_sidebar_width = app.view.sidebar_rect.width;
        let expanded = render_full_frame_for_test(&app, desktop);
        let center = buffer_rect_text(&expanded, app.view.terminal_area);
        for label in [
            "child/",
            "peer.txt",
            "preview.txt",
            "copy 0/1",
            "Esc cancel",
        ] {
            assert!(center.contains(label), "expanded center missing {label:?}");
        }
        for legacy in ["PARENT", "CURRENT", "PREVIEW"] {
            assert!(
                !center.contains(legacy),
                "live Trail must not render legacy marker {legacy:?}"
            );
        }
        let current_row = app
            .view
            .file_manager_locations
            .rows
            .first()
            .expect("expanded Files content-rail row")
            .rect;
        assert!((current_row.x..current_row.right()).any(|x| {
            let cell = &expanded[(x, current_row.y)];
            cell.fg == app.palette.accent
                && cell.modifier.contains(Modifier::BOLD | Modifier::REVERSED)
        }));
        let status_y = app.view.terminal_area.bottom() - 1;
        let status_x = (app.view.terminal_area.x..app.view.terminal_area.right())
            .find(|&x| expanded[(x, status_y)].symbol() == "c")
            .expect("running copy status");
        assert_eq!(expanded[(status_x, status_y)].fg, app.palette.yellow);

        app.sidebar_collapsed = true;
        compute_view(&mut app, desktop);
        assert_eq!(app.view.layout, ViewLayout::Desktop);
        assert!(app.view.sidebar_rect.width < expanded_sidebar_width);
        assert!(!app.view.file_manager_locations.rows.is_empty());
        let collapsed = render_full_frame_for_test(&app, desktop);
        assert!(buffer_rect_text(&collapsed, app.view.terminal_area).contains("copy 0/1"));

        app.sidebar_collapsed = false;
        let mobile_two = Rect::new(0, 0, 33, 15);
        {
            let file_manager = app.file_manager.as_ref().expect("open Files state");
            assert_eq!(file_manager.trail.active_col(), 0);
            assert_eq!(
                file_manager.trail.deepest(),
                1,
                "the child snapshot remains resident without owning focus"
            );
        }
        compute_view(&mut app, mobile_two);
        assert_eq!(app.view.layout, ViewLayout::Mobile);
        assert!(!app.view.file_manager_locations.rows.is_empty());
        let two = buffer_rect_text(
            &render_full_frame_for_test(&app, mobile_two),
            app.view.terminal_area,
        );
        assert!(two.contains("child/"));
        assert!(two.contains("peer.txt"));
        assert!(
            !two.contains("preview.txt"),
            "a resident child must not displace the active root column"
        );
        assert!(!two.contains("PARENT"));
        assert!(!two.contains("CURRENT"));
        assert!(!two.contains("PREVIEW"));
        assert!(two.contains("copy 0/1"));

        assert!(
            app.file_manager
                .as_mut()
                .expect("open Files state")
                .trail
                .move_active_right(),
            "explicit Right focuses the resident child"
        );
        compute_view(&mut app, mobile_two);
        let child = buffer_rect_text(
            &render_full_frame_for_test(&app, mobile_two),
            app.view.terminal_area,
        );
        assert!(
            child.contains("preview.txt"),
            "the child column becomes visible only after explicit Right"
        );

        assert!(
            app.file_manager
                .as_mut()
                .expect("open Files state")
                .trail
                .move_active_left(),
            "explicit Left restores the root owner"
        );

        let mobile_one = Rect::new(0, 0, 20, 15);
        compute_view(&mut app, mobile_one);
        assert_eq!(app.view.layout, ViewLayout::Mobile);
        let one = buffer_rect_text(
            &render_full_frame_for_test(&app, mobile_one),
            app.view.terminal_area,
        );
        assert!(one.contains("child/"));
        assert!(one.contains("peer.txt"));
        assert!(!one.contains("preview.txt"));
        assert!(!one.contains("CURRENT"));
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
        for label in [
            "Open",
            "Copy",
            "Rename",
            "Delete",
            "Add Reference to Agent...",
        ] {
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

    #[test]
    fn workspace_creation_dialog_renders_new_workspace_title() {
        let mut app = crate::app::state::AppState::test_new();
        app.mode = Mode::RenameWorkspace;
        app.pending_workspace_create_cwd = Some("/tmp/project".into());
        app.name_input = "project".into();

        let area = Rect::new(0, 0, 80, 20);
        compute_view(&mut app, area);
        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
        terminal.draw(|frame| render(&app, frame)).unwrap();
        let screen = (0..area.height)
            .map(|row| buffer_row_text(terminal.backend().buffer(), area, row))
            .collect::<Vec<_>>()
            .join("\n");

        assert!(screen.contains("new workspace"), "{screen}");
        assert!(screen.contains("project"), "{screen}");
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

    /// A desktop-shaped app with one workspace and one tab.
    fn desktop_app() -> crate::app::state::AppState {
        let mut app = crate::app::state::AppState::test_new();
        app.workspaces = vec![Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = Mode::Terminal;
        app
    }

    // TP-MOB-26: the shell choice for every viewport with room to spare is
    // exactly what it was. Height became an input to the layout; it must not
    // have become an input to which shell is drawn.
    #[test]
    fn the_shell_choice_is_unchanged_for_every_viewport() {
        for width in [20u16, 36, 44, 64, 65, 80, 120] {
            for height in [8u16, 14, 16, 17, 24, 40] {
                let mut app = desktop_app();
                compute_view(&mut app, Rect::new(0, 0, width, height));
                let expected = if width > 0 && width <= app.mobile_width_threshold {
                    ViewLayout::Mobile
                } else {
                    ViewLayout::Desktop
                };
                assert_eq!(
                    app.view.layout, expected,
                    "{width}x{height} must keep the shell its width chose"
                );
            }
        }
    }

    // TP-MOB-27: a wide but very short viewport — a phone held sideways —
    // stays on the desktop shell. Ninety columns fit a sidebar; the problem
    // there is rows, and swapping shells would not have returned any.
    #[test]
    fn a_wide_short_viewport_keeps_the_desktop_shell() {
        let mut app = desktop_app();
        compute_view(&mut app, Rect::new(0, 0, 90, 14));
        assert_eq!(app.view.layout, ViewLayout::Desktop);
    }

    // TP-MOB-28: on a short viewport the sidebar falls back to its status
    // rail, giving the terminal the columns back.
    #[test]
    fn a_short_viewport_folds_the_sidebar_to_its_rail() {
        let mut app = desktop_app();
        compute_view(&mut app, Rect::new(0, 0, 90, 14));
        assert_eq!(app.view.sidebar_rect.width, COLLAPSED_WIDTH);

        // And gives it back once the rows return, because this reads the
        // viewport rather than writing the preference.
        compute_view(&mut app, Rect::new(0, 0, 90, 40));
        assert!(
            app.view.sidebar_rect.width > COLLAPSED_WIDTH,
            "a taller viewport restores the sidebar it folded"
        );
        assert!(
            !app.sidebar_collapsed,
            "the fold must not have written the collapse preference"
        );
    }

    // TP-MOB-29: someone who expanded the sidebar themselves has answered the
    // question the heuristic was guessing at, so the guess stops arguing.
    #[test]
    fn an_explicit_expansion_outranks_the_short_viewport_fold() {
        let mut app = desktop_app();
        app.sidebar_expanded_explicitly = true;
        compute_view(&mut app, Rect::new(0, 0, 90, 14));
        assert!(
            app.view.sidebar_rect.width > COLLAPSED_WIDTH,
            "an explicit expansion survives a short viewport"
        );
    }

    // TP-MOB-30: a short viewport hides a tab strip that shows a single entry,
    // and brings it back when the rows return.
    #[test]
    fn a_short_viewport_hides_a_single_entry_tab_strip() {
        let mut app = desktop_app();
        compute_view(&mut app, Rect::new(0, 0, 90, 14));
        assert_eq!(app.view.tab_bar_rect, Rect::default());

        compute_view(&mut app, Rect::new(0, 0, 90, 40));
        assert_ne!(
            app.view.tab_bar_rect,
            Rect::default(),
            "a taller viewport restores the strip"
        );
    }

    // TP-MOB-31: a short viewport keeps a tab strip that shows more than one
    // entry — hiding it there would make the other tabs unreachable by mouse.
    #[test]
    fn a_short_viewport_keeps_a_multi_entry_tab_strip() {
        let mut app = desktop_app();
        app.workspaces[0].test_add_tab(Some("logs"));
        compute_view(&mut app, Rect::new(0, 0, 90, 14));
        assert_ne!(app.view.tab_bar_rect, Rect::default());
    }

    // TP-MOB-25: every overlay reached by entering a mode paints something on
    // a viewport too small to hold it. A mode with no visible overlay is a
    // terminal that has stopped responding as far as the reader can tell:
    // keystrokes go to a surface that is not on screen, and nothing says which
    // key gets them back.
    #[test]
    fn no_overlay_mode_leaves_a_blank_screen_on_a_tiny_viewport() {
        use ratatui::{backend::TestBackend, Terminal};

        let modes = [
            Mode::Onboarding,
            Mode::ReleaseNotes,
            Mode::ProductAnnouncement,
            Mode::Settings,
            Mode::KeybindHelp,
            Mode::RenameWorkspace,
            Mode::RenameTab,
            Mode::NewLinkedWorktree,
            Mode::OpenExistingWorktree,
        ];

        for mode in modes {
            for (w, h) in [(16u16, 8u16), (20, 10), (28, 12)] {
                let mut app = crate::app::state::AppState::test_new();
                app.workspaces = vec![Workspace::test_new("one")];
                app.active = Some(0);
                app.selected = 0;
                // These two modes are only reachable when there is something to
                // announce, so give them their content: an overlay that draws
                // nothing because it has nothing to say is a different
                // question from one that draws nothing because it does not fit.
                app.release_notes = Some(crate::app::state::ReleaseNotesState {
                    version: "0.7.5".to_string(),
                    body: "a line of notes".to_string(),
                    scroll: 0,
                    preview: false,
                });
                app.product_announcement = Some(crate::app::state::ProductAnnouncementState {
                    version: "0.7.5".to_string(),
                    id: "test".to_string(),
                    title: "something changed".to_string(),
                    body: "a line of announcement".to_string(),
                    scroll: 0,
                    preview: false,
                });
                app.worktree_create = Some(crate::app::state::WorktreeCreateState {
                    source_workspace_id: "one".to_string(),
                    source_checkout_path: std::path::PathBuf::from("/tmp/herdr-test-repo"),
                    source_existing_membership: None,
                    source_repo_root: std::path::PathBuf::from("/tmp/herdr-test-repo"),
                    repo_key: "herdr-test-repo".to_string(),
                    repo_name: "herdr-test-repo".to_string(),
                    branch: "feature".to_string(),
                    checkout_path: std::path::PathBuf::from("/tmp/herdr-test-repo-feature"),
                    error: None,
                    creating: false,
                });
                app.worktree_open = Some(crate::app::state::WorktreeOpenState {
                    source_workspace_id: "one".to_string(),
                    source_existing_membership: None,
                    source_checkout_path: std::path::PathBuf::from("/tmp/herdr-test-repo"),
                    source_repo_root: std::path::PathBuf::from("/tmp/herdr-test-repo"),
                    repo_key: "herdr-test-repo".to_string(),
                    repo_name: "herdr-test-repo".to_string(),
                    entries: Vec::new(),
                    selected: 0,
                    query: String::new(),
                    search_focused: false,
                    error: None,
                });
                app.mode = mode;
                let area = Rect::new(0, 0, w, h);
                compute_view(&mut app, area);

                let mut terminal = Terminal::new(TestBackend::new(w, h)).expect("test terminal");
                terminal
                    .draw(|frame| render(&app, frame))
                    .expect("overlay renders");
                let rendered: String = terminal
                    .backend()
                    .buffer()
                    .content()
                    .iter()
                    .map(|cell| cell.symbol())
                    .collect();

                assert!(
                    rendered.contains("esc") || rendered.contains('┌') || rendered.contains('╭'),
                    "{mode:?} at {w}x{h} drew neither a modal frame nor a way out: \
                     {rendered:?}"
                );
            }
        }
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
        assert_eq!(app.view.mobile_header_rect, Rect::new(0, 0, 44, 4));
        assert_eq!(app.view.terminal_area, Rect::new(0, 4, 44, 16));
        let hits = app.view.mobile_header_hits;
        // Nine columns by four rows each since TP-MOB-89 made the buttons a
        // 44pt touch square, plus the one row of reach TP-MOB-66 gave them
        // for a thumb that lands just under.
        assert_eq!(hits.spaces_menu, Rect::new(0, 0, 9, 5));
        assert_eq!(hits.tabs_menu, Rect::new(35, 0, 9, 5));
        assert_eq!(hits.tab_strip, Rect::new(9, 0, 26, 4));
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
        assert_eq!(app.view.mobile_header_rect, Rect::new(0, 0, 80, 4));
        assert_eq!(app.view.terminal_area, Rect::new(0, 4, 80, 16));
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
        assert_eq!(app.view.terminal_area, Rect::new(0, 4, 44, 16));
        assert_eq!(
            app.workspaces[0].tabs[background_tab].runtimes[&background_pane].current_size(),
            (16, 43)
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

        // TP-TREE-10 reserves the disclosure column on every row so sibling
        // names line up; the subject here — the state dot leads the name and
        // no ordinal does — is unchanged. TP-ART-01 rides the branch glyph
        // on the label itself, after the dot, so the order the test pins
        // still reads state first.
        assert!(line1.starts_with("  ·  one"));
        assert!(!line1.contains("1 one"));
        assert_eq!(line2, "    main");

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
                width: None,
                height: None,
            },
            crate::config::CustomCommandKeybind {
                bindings: crate::config::ActionKeybinds::prefix("alt+h"),
                label: "prefix+alt+h".to_string(),
                command: "echo hello".to_string(),
                action: crate::config::CustomCommandAction::Shell,
                description: None,
                width: None,
                height: None,
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

        let rendered_help = keybind_help_lines(&app, keybind_help::WIDE_HELP_BODY_WIDTH)
            .into_iter()
            .flat_map(|line| line.spans)
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
