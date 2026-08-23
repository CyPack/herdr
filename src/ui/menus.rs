use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::widgets::{panel_contrast_fg, render_panel_shell};
use crate::app::AppState;

fn prefix_rhs_label(bindings: &crate::config::ActionKeybinds) -> String {
    bindings
        .prefix_rhs_label()
        .unwrap_or_else(|| "unset".to_string())
}

fn keybind_label(bindings: &crate::config::ActionKeybinds) -> String {
    bindings.label().unwrap_or_else(|| "unset".to_string())
}

fn render_bottom_bar(frame: &mut Frame, area: Rect, line: Line<'_>, bg: ratatui::style::Color) {
    frame.render_widget(Clear, area);
    let buf = frame.buffer_mut();
    for x in area.x..area.x + area.width {
        buf[(x, area.y)].set_style(Style::default().bg(bg));
    }
    frame.render_widget(Paragraph::new(line), area);
}

pub(super) fn render_prefix_overlay(app: &AppState, frame: &mut Frame, area: Rect) {
    let key = Style::default()
        .fg(app.palette.accent)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(app.palette.overlay0);
    let mode_style = Style::default()
        .fg(panel_contrast_fg(&app.palette))
        .bg(app.palette.accent)
        .add_modifier(Modifier::BOLD);

    let workspace_picker = prefix_rhs_label(&app.keybinds.workspace_picker);
    let help = prefix_rhs_label(&app.keybinds.help);
    let prefix = crate::config::format_key_combo((app.prefix_code, app.prefix_mods));

    let line = Line::from(vec![
        Span::styled(" PREFIX ", mode_style),
        Span::raw(" "),
        Span::styled("esc", key),
        Span::styled(" cancel  ", dim),
        Span::styled(prefix, key),
        Span::styled(" send prefix  ", dim),
        Span::styled(workspace_picker, key),
        Span::styled(" workspace nav  ", dim),
        Span::styled(help, key),
        Span::styled(" keybinds", dim),
    ]);

    let overlay_y = area.y + area.height.saturating_sub(1);
    let overlay_area = Rect::new(area.x, overlay_y, area.width, 1);
    render_bottom_bar(frame, overlay_area, line, app.palette.panel_bg);
}

pub(super) fn render_copy_mode_overlay(app: &AppState, frame: &mut Frame, area: Rect) {
    let key = Style::default()
        .fg(app.palette.accent)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(app.palette.overlay0);
    let mode_style = Style::default()
        .fg(panel_contrast_fg(&app.palette))
        .bg(app.palette.accent)
        .add_modifier(Modifier::BOLD);

    let Some(copy_mode) = app.copy_mode.as_ref() else {
        return;
    };
    let line = if let Some(prompt) = copy_mode.search.prompt.as_ref() {
        let marker = match prompt.direction {
            crate::app::state::CopyModeSearchDirection::Forward => "/",
            crate::app::state::CopyModeSearchDirection::Backward => "?",
        };
        Line::from(vec![
            Span::styled(" COPY ", mode_style),
            Span::raw(" "),
            Span::styled(marker, key),
            Span::styled(prompt.query.clone(), Style::default().fg(app.palette.text)),
            Span::styled("█", key),
            Span::styled("  enter search  esc cancel", dim),
        ])
    } else {
        let select = if copy_mode.selection.is_some() {
            "selecting"
        } else {
            "select"
        };
        let match_status = copy_mode
            .search
            .current
            .map(|current| format!(" {}/{}", current + 1, copy_mode.search.matches.len()))
            .or_else(|| (!copy_mode.search.query.is_empty()).then(|| " 0/0".to_string()))
            .unwrap_or_default();
        let (exit_keys, exit_label) =
            if copy_mode.search.query.is_empty() && copy_mode.selection.is_none() {
                ("q/esc", " exit")
            } else {
                ("esc", " clear  q exit")
            };
        Line::from(vec![
            Span::styled(" COPY ", mode_style),
            Span::raw(" "),
            Span::styled("h/j/k/l w/b/e { }", key),
            Span::styled(" move  ", dim),
            Span::styled("/ ?", key),
            Span::styled(" search  ", dim),
            Span::styled("n/N", key),
            Span::styled(format!(" repeat{match_status}  "), dim),
            Span::styled("v/space", key),
            Span::styled(format!(" {select}  "), dim),
            Span::styled("y/enter", key),
            Span::styled(" copy  ", dim),
            Span::styled(exit_keys, key),
            Span::styled(exit_label, dim),
        ])
    };

    let overlay_y = area.y + area.height.saturating_sub(1);
    let overlay_area = Rect::new(area.x, overlay_y, area.width, 1);
    render_bottom_bar(frame, overlay_area, line, app.palette.panel_bg);
}

pub(super) fn render_navigate_overlay(app: &AppState, frame: &mut Frame, area: Rect) {
    let key = Style::default()
        .fg(app.palette.accent)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(app.palette.overlay0);

    let mode_style = Style::default()
        .fg(panel_contrast_fg(&app.palette))
        .bg(app.palette.accent)
        .add_modifier(Modifier::BOLD);

    let kb = &app.keybinds;
    let new_tab = prefix_rhs_label(&kb.new_tab);
    let split_vertical = prefix_rhs_label(&kb.split_vertical);
    let split_horizontal = prefix_rhs_label(&kb.split_horizontal);
    let close_pane = prefix_rhs_label(&kb.close_pane);
    let zoom = prefix_rhs_label(&kb.zoom);
    let resize = prefix_rhs_label(&kb.resize_mode);
    let help = prefix_rhs_label(&kb.help);
    let settings = prefix_rhs_label(&kb.settings);
    let goto = prefix_rhs_label(&kb.goto);
    let detach = prefix_rhs_label(&kb.detach);
    let workspace_nav = format!(
        "{} / {}",
        keybind_label(&kb.navigate.workspace_up),
        keybind_label(&kb.navigate.workspace_down)
    );
    let line = Line::from(vec![
        Span::styled(" NAVIGATE ", mode_style),
        Span::raw(" "),
        Span::styled("esc", key),
        Span::styled(" back  ", dim),
        Span::styled(workspace_nav, key),
        Span::styled(" ws  ", dim),
        Span::styled("⇥", key),
        Span::styled(" pane  ", dim),
        Span::styled(goto, key),
        Span::styled(" navigator  ", dim),
        Span::styled(new_tab, key),
        Span::styled(" new tab  ", dim),
        Span::styled(split_vertical, key),
        Span::styled(" split│  ", dim),
        Span::styled(split_horizontal, key),
        Span::styled(" split─  ", dim),
        Span::styled(close_pane, key),
        Span::styled(" close  ", dim),
        Span::styled(zoom, key),
        Span::styled(" zoom  ", dim),
        Span::styled(resize, key),
        Span::styled(" resize  ", dim),
        Span::styled(help, key),
        Span::styled(" keybinds  ", dim),
        Span::styled(settings, key),
        Span::styled(" settings  ", dim),
        Span::styled(detach, key),
        Span::styled(" detach", dim),
    ]);

    let overlay_y = area.y + area.height.saturating_sub(1);
    let overlay_area = Rect::new(area.x, overlay_y, area.width, 1);
    render_bottom_bar(frame, overlay_area, line, app.palette.panel_bg);

    if app.update_available.is_some() {
        let status = Line::from(vec![Span::styled(
            " update ready",
            Style::default()
                .fg(app.palette.accent)
                .add_modifier(Modifier::BOLD),
        )]);
        let width = 13u16.min(overlay_area.width);
        let status_area = Rect::new(
            overlay_area.x + overlay_area.width.saturating_sub(width),
            overlay_area.y,
            width,
            overlay_area.height,
        );
        frame.render_widget(Clear, status_area);
        frame.render_widget(
            Paragraph::new(status).alignment(Alignment::Right),
            status_area,
        );
    }
}

pub(super) fn render_global_launcher_menu(app: &AppState, frame: &mut Frame) {
    let rect = app.global_menu_rect();
    let Some(inner) = render_panel_shell(frame, rect, app.palette.accent, app.palette.panel_bg)
    else {
        return;
    };

    let items = app.global_menu_labels();
    for (idx, item) in items.iter().enumerate() {
        let y = inner.y + idx as u16;
        if y >= inner.y + inner.height {
            break;
        }
        let selected = idx == app.global_menu.highlighted;
        let rect = Rect::new(inner.x, y, inner.width, 1);

        let selected_style = Style::default()
            .fg(panel_contrast_fg(&app.palette))
            .bg(app.palette.accent)
            .add_modifier(Modifier::BOLD);
        let item_style = if selected {
            selected_style
        } else {
            Style::default().fg(app.palette.text)
        };
        let badge_style = if selected {
            selected_style
        } else {
            Style::default()
                .fg(app.palette.accent)
                .add_modifier(Modifier::BOLD)
        };

        let line = if app.global_menu_item_has_badge(item) {
            Line::from(vec![
                Span::styled(" ●", badge_style),
                Span::styled(format!(" {item} "), item_style),
            ])
        } else {
            Line::from(Span::styled(format!(" {item} "), item_style))
        };
        frame.render_widget(Paragraph::new(line).alignment(Alignment::Left), rect);
    }
}

pub(super) fn render_resize_overlay(app: &AppState, frame: &mut Frame, area: Rect) {
    let key = Style::default()
        .fg(app.palette.accent)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(app.palette.overlay0);

    let mode_style = Style::default()
        .fg(panel_contrast_fg(&app.palette))
        .bg(app.palette.mauve)
        .add_modifier(Modifier::BOLD);

    let line = Line::from(vec![
        Span::styled(" RESIZE ", mode_style),
        Span::raw("  "),
        Span::styled("h/l", key),
        Span::styled(" width  ", dim),
        Span::styled("j/k", key),
        Span::styled(" height  ", dim),
        Span::styled("esc", key),
        Span::styled(" done", dim),
    ]);

    let overlay_y = area.y + area.height.saturating_sub(1);
    let overlay_area = Rect::new(area.x, overlay_y, area.width, 1);
    render_bottom_bar(frame, overlay_area, line, app.palette.panel_bg);
}

pub(super) fn render_context_menu(app: &AppState, frame: &mut Frame) {
    let Some(menu) = &app.context_menu else {
        return;
    };

    let p = &app.palette;
    let Some(menu_rect) = app.context_menu_rect() else {
        return;
    };
    let Some(inner) = render_panel_shell(frame, menu_rect, p.accent, p.panel_bg) else {
        return;
    };

    let (items, highlight_style): (Vec<ListItem>, Style) = match &menu.kind {
        crate::app::state::ContextMenuKind::File { model } => {
            let items = model
                .items
                .iter()
                .enumerate()
                .map(|(idx, item)| {
                    let fg = if !item.enabled {
                        p.overlay0
                    } else if idx == menu.list.highlighted {
                        panel_contrast_fg(p)
                    } else {
                        p.text
                    };
                    ListItem::new(Line::from(item.label.as_str())).style(Style::default().fg(fg))
                })
                .collect();
            (
                items,
                Style::default().bg(p.accent).add_modifier(Modifier::BOLD),
            )
        }
        _ => (
            menu.items()
                .iter()
                .map(|item| ListItem::new(Line::from(*item)))
                .collect(),
            Style::default()
                .bg(p.accent)
                .fg(panel_contrast_fg(p))
                .add_modifier(Modifier::BOLD),
        ),
    };
    let list = List::new(items)
        .style(Style::default().fg(p.text))
        .highlight_style(highlight_style)
        .highlight_symbol(" ");
    let mut state = ListState::default().with_selected(Some(menu.list.highlighted));
    frame.render_stateful_widget(list, inner, &mut state);
}

/// Render the blocking "Add Reference to Agent..." picker. Geometry comes
/// from the same pure AppState helpers the mouse hit-testing uses, so the
/// painted rows and the clickable rows can never drift apart.
pub(super) fn render_agent_colleague_picker(app: &AppState, frame: &mut Frame) {
    let Some(picker) = app.agent_colleague_picker.as_ref() else {
        return;
    };
    let Some(popup) = app.agent_colleague_picker_popup_rect() else {
        return;
    };
    let p = &app.palette;
    if render_panel_shell(frame, popup, p.accent, p.panel_bg).is_none() {
        return;
    }
    let header = Rect::new(popup.x + 1, popup.y + 1, popup.width.saturating_sub(2), 1);
    frame.render_widget(
        Paragraph::new("Work with other agent...")
            .style(Style::default().fg(p.text).add_modifier(Modifier::BOLD)),
        header,
    );
    for (idx, (row, rect)) in picker
        .rows
        .iter()
        .zip(app.agent_colleague_picker_row_hit_areas())
        .enumerate()
    {
        let marker = if idx == picker.selected { ">" } else { " " };
        // TP-AGPANEL-48: the relation tag rides at the row's end — same
        // branch, same space, or a different space — so the reader knows how
        // far the jump goes before taking it.
        let tag = row
            .relation
            .map(|relation| format!("  [{}]", relation.label()))
            .unwrap_or_default();
        let label = crate::ui::text::truncate_end(
            &format!("{marker} {}{tag}", row.label),
            rect.width as usize,
        );
        let style = if !row.live {
            Style::default().fg(p.overlay0)
        } else if idx == picker.selected {
            Style::default().bg(p.surface1).fg(p.text)
        } else {
            Style::default().fg(p.subtext0)
        };
        frame.render_widget(Paragraph::new(label).style(style), rect);
    }
}

/// TP-CHROME-150: the bar config panel — the pickers' popup shell, rows of
/// knob-and-value with the selected one highlighted, the current draft's
/// words coming from the same `row_value_label` the tests read.
pub(super) fn render_bar_config_panel(app: &AppState, frame: &mut Frame) {
    let Some(panel) = app.bar_config_panel.as_ref() else {
        return;
    };
    let Some(popup) = app.bar_config_panel_popup_rect() else {
        return;
    };
    let p = &app.palette;
    if render_panel_shell(frame, popup, p.accent, p.panel_bg).is_none() {
        return;
    }
    let header = Rect::new(popup.x + 1, popup.y + 1, popup.width.saturating_sub(2), 1);
    frame.render_widget(
        Paragraph::new(format!(
            "Configure {} bar",
            crate::app::bar_config_panel::bar_edge_name(panel.edge)
        ))
        .style(Style::default().fg(p.text).add_modifier(Modifier::BOLD)),
        header,
    );
    // TP-CHROME-153: the two faces' labels ride the strip line; the forward
    // one wears the selection tone so the eye knows which face answers.
    for (tab, rect) in crate::app::bar_config_panel::BarPanelTab::ALL
        .iter()
        .zip(app.bar_config_panel_tab_hit_areas())
    {
        let style = if *tab == panel.tab {
            Style::default()
                .bg(p.surface1)
                .fg(p.text)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(p.subtext0)
        };
        frame.render_widget(
            Paragraph::new(format!(" {} ", tab.label())).style(style),
            rect,
        );
    }
    match panel.tab {
        crate::app::bar_config_panel::BarPanelTab::Configure => {
            for (idx, (row, rect)) in panel
                .rows()
                .iter()
                .zip(app.bar_config_panel_row_hit_areas())
                .enumerate()
            {
                let marker = if idx == panel.selected { ">" } else { " " };
                let value = crate::app::bar_config_panel::row_value_label(panel, *row);
                let label = crate::ui::text::truncate_end(
                    &format!("{marker} {value}"),
                    rect.width as usize,
                );
                let style = if idx == panel.selected {
                    Style::default().bg(p.surface1).fg(p.text)
                } else {
                    Style::default().fg(p.subtext0)
                };
                frame.render_widget(Paragraph::new(label).style(style), rect);
            }
        }
        crate::app::bar_config_panel::BarPanelTab::Apps => {
            if let Some(form) = panel.edit.as_ref() {
                // TP-CHROME-166: the editor wears the Configure face's
                // shape — label rows, one selected, two verbs at the end.
                for (idx, (field, rect)) in crate::app::bar_config_panel::BarAppEditField::ALL
                    .iter()
                    .zip(app.bar_config_panel_row_hit_areas())
                    .enumerate()
                {
                    let marker = if idx == panel.selected { ">" } else { " " };
                    let value = crate::app::bar_config_panel::edit_row_label(form, *field);
                    let label = crate::ui::text::truncate_end(
                        &format!("{marker} {value}"),
                        rect.width as usize,
                    );
                    let focused = *field == form.field;
                    let style = if focused || idx == panel.selected {
                        Style::default().bg(p.surface1).fg(p.text)
                    } else {
                        Style::default().fg(p.subtext0)
                    };
                    frame.render_widget(Paragraph::new(label).style(style), rect);
                }
            } else {
                for (idx, (row, rect)) in panel
                    .app_rows()
                    .iter()
                    .zip(app.bar_config_panel_row_hit_areas())
                    .enumerate()
                {
                    let marker = if idx == panel.selected { ">" } else { " " };
                    // A row the editor cannot reach says so where the eye
                    // already is (TP-CHROME-166).
                    let badge = if row.section_id.is_none() {
                        "  \u{b7}no id"
                    } else {
                        ""
                    };
                    let label = crate::ui::text::truncate_end(
                        &format!("{marker} {}  {}{badge}", row.shows, row.does),
                        rect.width as usize,
                    );
                    let style = if !row.live {
                        Style::default().fg(p.overlay0)
                    } else if idx == panel.selected {
                        Style::default().bg(p.surface1).fg(p.text)
                    } else {
                        Style::default().fg(p.subtext0)
                    };
                    frame.render_widget(Paragraph::new(label).style(style), rect);
                }
            }
        }
    }

    // TP-CHROME-161: the last inner line teaches the face's real keys. Dim,
    // unselectable, and different per face — Apps has no \u{2190}\u{2192} adjustment.
    if popup.height >= 5 {
        let hint_rect = Rect::new(
            popup.x + 1,
            popup.y + popup.height - 2,
            popup.width.saturating_sub(2),
            1,
        );
        let (hint, hint_style) = match panel.edit.as_ref() {
            Some(form) => match form.validation_error.as_deref() {
                // A refused Save says why, where the hint was — the reader
                // is already looking there (TP-CHROME-166).
                Some(err) => (err.to_string(), Style::default().fg(p.red)),
                None => (
                    "Enter save \u{b7} Esc back \u{b7} Tab next field".to_string(),
                    Style::default().fg(p.overlay0),
                ),
            },
            None => (
                crate::app::bar_config_panel::panel_hint(panel.tab).to_string(),
                Style::default().fg(p.overlay0),
            ),
        };
        frame.render_widget(
            Paragraph::new(crate::ui::text::truncate_end(
                &hint,
                hint_rect.width as usize,
            ))
            .style(hint_style),
            hint_rect,
        );
    }
}

pub(super) fn render_agent_reference_picker(app: &AppState, frame: &mut Frame) {
    let Some(picker) = app.agent_reference_picker.as_ref() else {
        return;
    };
    let Some(popup) = app.agent_reference_picker_popup_rect() else {
        return;
    };
    let p = &app.palette;
    if render_panel_shell(frame, popup, p.accent, p.panel_bg).is_none() {
        return;
    }
    let header = Rect::new(popup.x + 1, popup.y + 1, popup.width.saturating_sub(2), 1);
    frame.render_widget(
        Paragraph::new("Add Reference to Agent...")
            .style(Style::default().fg(p.text).add_modifier(Modifier::BOLD)),
        header,
    );
    for (idx, (row, rect)) in picker
        .rows
        .iter()
        .zip(app.agent_reference_picker_row_hit_areas())
        .enumerate()
    {
        let marker = if idx == picker.selected { ">" } else { " " };
        let current = if row.is_current { " (current)" } else { "" };
        let label = crate::ui::text::truncate_end(
            &format!("{marker} {}{current}", row.label),
            rect.width as usize,
        );
        let style = if !row.live {
            Style::default().fg(p.overlay0)
        } else if idx == picker.selected {
            Style::default().bg(p.surface1).fg(p.text)
        } else {
            Style::default().fg(p.subtext0)
        };
        frame.render_widget(Paragraph::new(label).style(style), rect);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::{
        ContextMenuKind, ContextMenuState, FileManagerActionDisabledReason,
        FileManagerContextMenuAction, FileManagerContextMenuItem, FileManagerContextMenuModel,
        FileManagerContextMenuTargetKind, MenuListState,
    };
    use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};

    // TP-CHROME-160/161: one popup width for both faces (no jump on Tab),
    // one extra line for the hint, and the hint is neither a row nor a click
    // target. The Configure face must show every style choice on screen —
    // the exact discoverability the user reported missing — and each face
    // must teach its own real keys.
    #[test]
    fn the_panel_enumerates_choices_and_teaches_its_keys_on_screen() {
        let mut app = crate::app::state::AppState::test_new();
        app.view.terminal_area = Rect::new(0, 0, 80, 30);
        app.bar_config_panel = Some(crate::app::bar_config_panel::BarConfigPanelState::open(
            crate::ui::shell::BarEdge::Top,
            &Default::default(),
        ));

        let configure = app.bar_config_panel_popup_rect().expect("popup");
        assert_eq!(configure.width, 56, "both faces share one width");
        let rows = app.bar_config_panel.as_ref().unwrap().forward_row_count() as u16;
        assert_eq!(configure.height, rows + 5, "one line reserved for the hint");
        let hit = app.bar_config_panel_row_hit_areas();
        assert_eq!(hit.len() as u16, rows);
        let last_row_y = hit.last().unwrap().y;
        assert_eq!(
            last_row_y,
            configure.y + configure.height - 3,
            "rows stop above the hint line"
        );

        let backend = TestBackend::new(80, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_bar_config_panel(&app, frame))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let text: String = (0..30)
            .map(|y| {
                (0..80)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
                    + "\n"
            })
            .collect();
        assert!(text.contains("islands"), "every style is on screen: {text}");
        assert!(
            text.contains("[framed]"),
            "the selection wears brackets: {text}"
        );
        assert!(
            text.contains("\u{2190}\u{2192} change"),
            "the Configure face teaches its arrow verb: {text}"
        );

        // The Apps face keeps the width and swaps the hint for its own keys.
        app.bar_config_panel.as_mut().unwrap().tab =
            crate::app::bar_config_panel::BarPanelTab::Apps;
        let apps = app.bar_config_panel_popup_rect().expect("popup");
        assert_eq!(apps.width, 56);
        let backend = TestBackend::new(80, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_bar_config_panel(&app, frame))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let text: String = (0..30)
            .map(|y| {
                (0..80)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
                    + "\n"
            })
            .collect();
        assert!(text.contains("Tab configure"), "{text}");
        assert!(
            !text.contains("\u{2190}\u{2192} change"),
            "Apps must not claim an arrow verb it does not have: {text}"
        );
    }

    use std::path::PathBuf;

    fn multiple_file_menu(highlighted: usize) -> AppState {
        let mut app = AppState::test_new();
        app.view.sidebar_rect = Rect::new(0, 0, 1, 12);
        app.view.terminal_area = Rect::new(1, 0, 39, 12);
        let items = FileManagerContextMenuAction::ALL.map(|action| {
            let disabled = matches!(
                &action,
                FileManagerContextMenuAction::Open
                    | FileManagerContextMenuAction::Rename
                    | FileManagerContextMenuAction::Compress
                    | FileManagerContextMenuAction::SendAgent
            );
            let label = action.label().to_string();
            let disabled_reason = match action {
                FileManagerContextMenuAction::Compress => {
                    Some(FileManagerActionDisabledReason::UnsupportedAction)
                }
                _ => disabled.then_some(FileManagerActionDisabledReason::MultipleSelection),
            };
            FileManagerContextMenuItem {
                action,
                label,
                enabled: !disabled,
                disabled_reason,
            }
        });
        app.context_menu = Some(ContextMenuState {
            kind: ContextMenuKind::File {
                model: FileManagerContextMenuModel {
                    target_kind: FileManagerContextMenuTargetKind::Multiple,
                    paths: vec![PathBuf::from("a.txt"), PathBuf::from("b.txt")],
                    items: items.to_vec(),
                },
            },
            x: 2,
            y: 1,
            list: MenuListState::new(highlighted),
        });
        app
    }

    fn render_menu(app: &AppState) -> Buffer {
        let backend = TestBackend::new(40, 12);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|frame| render_context_menu(app, frame))
            .expect("render context menu");
        terminal.backend().buffer().clone()
    }

    fn glyph_fg(buffer: &Buffer, row: u16, glyph: &str) -> ratatui::style::Color {
        (0..buffer.area.width)
            .find_map(|column| {
                let cell = &buffer[(column, row)];
                (cell.symbol() == glyph).then_some(cell.fg)
            })
            .unwrap_or_else(|| panic!("missing glyph {glyph:?} on row {row}"))
    }

    // TP-C3.2-POPUP-LIFECYCLE: disabled file actions remain visibly dim even
    // when highlighted, while enabled rows retain normal and selected menu
    // contrast. The model is read-only during rendering.
    #[test]
    fn disabled_file_context_items_have_distinct_highlight_safe_style() {
        let disabled_highlight = multiple_file_menu(0);
        let menu_rect = disabled_highlight.context_menu_rect().expect("menu rect");
        let buffer = render_menu(&disabled_highlight);
        assert_eq!(
            glyph_fg(&buffer, menu_rect.y + 1, "O"),
            disabled_highlight.palette.overlay0,
            "highlighted disabled Open remains dim"
        );
        // Row 2 is Enlarge, which a multiple selection also disables; Copy is
        // the first enabled row at 3.
        assert_eq!(
            glyph_fg(&buffer, menu_rect.y + 3, "C"),
            disabled_highlight.palette.text,
            "enabled Copy uses normal text while not highlighted"
        );

        let enabled_highlight = multiple_file_menu(2);
        let buffer = render_menu(&enabled_highlight);
        assert_eq!(
            glyph_fg(&buffer, menu_rect.y + 1, "O"),
            enabled_highlight.palette.overlay0,
            "non-highlighted disabled Open remains dim"
        );
        assert_eq!(
            glyph_fg(&buffer, menu_rect.y + 3, "C"),
            panel_contrast_fg(&enabled_highlight.palette),
            "highlighted enabled Copy keeps selected contrast"
        );
    }
}
