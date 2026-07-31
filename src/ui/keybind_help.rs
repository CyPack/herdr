use std::borrow::Cow;

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::release_notes::release_notes_close_button_rect;
use super::scrollbar::{release_notes_scrollbar_rect, render_scrollbar};
use super::text::truncate_end;
use super::widgets::{
    modal_stack_areas, panel_contrast_fg, render_action_button, render_modal_header,
    render_modal_shell_or_notice,
};
use crate::app::AppState;

pub(super) type HelpEntry = (String, Cow<'static, str>);
pub(super) type HelpGroup = (&'static str, Vec<HelpEntry>);

fn help_entry(key: impl Into<String>, label: &'static str) -> HelpEntry {
    (key.into(), Cow::Borrowed(label))
}

fn keybind_label(bindings: &crate::config::ActionKeybinds) -> String {
    bindings.label().unwrap_or_else(|| "unset".to_string())
}

fn indexed_label(bindings: &[crate::config::IndexedKeybind]) -> String {
    if bindings.is_empty() {
        return "unset".to_string();
    }

    let mut parts = Vec::new();
    let mut index = 0;
    while index < bindings.len() {
        if let Some(prefix) = indexed_range_prefix(&bindings[index..]) {
            parts.push(format!("{prefix}1..9"));
            index += 9;
        } else {
            parts.push(bindings[index].label.clone());
            index += 1;
        }
    }

    parts.join(" / ")
}

fn indexed_range_prefix(bindings: &[crate::config::IndexedKeybind]) -> Option<&str> {
    let run = bindings.get(..9)?;
    let prefix = run[0].label.strip_suffix('1')?;
    for (offset, binding) in run.iter().enumerate() {
        let digit = char::from(b'1' + offset as u8);
        if binding.label.strip_suffix(digit) != Some(prefix) {
            return None;
        }
    }
    Some(prefix)
}

pub(super) fn keybind_help_groups(app: &AppState) -> Vec<HelpGroup> {
    let kb = &app.keybinds;
    let mut groups = Vec::new();

    groups.push((
        "global",
        vec![
            help_entry(
                crate::config::format_key_combo((app.prefix_code, app.prefix_mods)),
                "prefix mode",
            ),
            help_entry(keybind_label(&kb.help), "keybinds"),
            help_entry(keybind_label(&kb.settings), "settings"),
            help_entry(keybind_label(&kb.detach), "detach"),
            help_entry(keybind_label(&kb.reload_config), "reload config"),
            help_entry(
                keybind_label(&kb.open_notification_target),
                "open notification target",
            ),
        ],
    ));

    groups.push((
        "navigation",
        vec![
            help_entry("esc", "back"),
            help_entry(
                format!(
                    "{} / {}",
                    keybind_label(&kb.navigate.workspace_up),
                    keybind_label(&kb.navigate.workspace_down)
                ),
                "workspace list",
            ),
            help_entry(
                format!(
                    "{} / {} / {} / {} / left / right",
                    keybind_label(&kb.navigate.pane_left),
                    keybind_label(&kb.navigate.pane_down),
                    keybind_label(&kb.navigate.pane_up),
                    keybind_label(&kb.navigate.pane_right)
                ),
                "move focus",
            ),
            help_entry("tab / shift+tab", "cycle pane"),
            help_entry("enter", "open workspace"),
            help_entry("1..9", "switch workspace"),
        ],
    ));

    let workspace_tab = vec![
        help_entry(keybind_label(&kb.workspace_picker), "workspace navigation"),
        help_entry(keybind_label(&kb.goto), "session navigator"),
        help_entry(keybind_label(&kb.new_workspace), "new workspace"),
        help_entry(keybind_label(&kb.new_worktree), "new worktree"),
        help_entry(keybind_label(&kb.open_worktree), "open worktree"),
        help_entry(
            keybind_label(&kb.remove_worktree),
            "delete worktree checkout",
        ),
        help_entry(keybind_label(&kb.rename_workspace), "rename workspace"),
        help_entry(keybind_label(&kb.close_workspace), "close workspace"),
        help_entry(keybind_label(&kb.previous_workspace), "previous workspace"),
        help_entry(keybind_label(&kb.next_workspace), "next workspace"),
        help_entry(indexed_label(&kb.switch_workspace), "switch workspace 1-9"),
        help_entry(keybind_label(&kb.previous_agent), "previous agent"),
        help_entry(keybind_label(&kb.next_agent), "next agent"),
        help_entry(indexed_label(&kb.focus_agent), "focus agent 1-9"),
        help_entry(keybind_label(&kb.new_tab), "new tab"),
        help_entry(keybind_label(&kb.new_chat_tab), "new chat tab"),
        help_entry(keybind_label(&kb.rename_tab), "rename tab"),
        help_entry(keybind_label(&kb.previous_tab), "previous tab"),
        help_entry(keybind_label(&kb.next_tab), "next tab"),
        help_entry(indexed_label(&kb.switch_tab), "switch tab 1-9"),
        help_entry(keybind_label(&kb.close_tab), "close tab"),
    ];
    groups.push(("workspaces / tabs", workspace_tab));

    let panes = vec![
        help_entry(keybind_label(&kb.split_vertical), "split vertical"),
        help_entry(keybind_label(&kb.split_horizontal), "split horizontal"),
        help_entry(keybind_label(&kb.close_pane), "close pane"),
        help_entry(keybind_label(&kb.rename_pane), "rename pane"),
        help_entry(keybind_label(&kb.edit_scrollback), "edit scrollback"),
        help_entry(keybind_label(&kb.copy_mode), "copy mode"),
        help_entry(keybind_label(&kb.zoom), "zoom pane"),
        help_entry(keybind_label(&kb.resize_mode), "resize mode"),
        help_entry(keybind_label(&kb.toggle_sidebar), "toggle sidebar"),
        help_entry(
            keybind_label(&kb.toggle_file_manager),
            "toggle file manager",
        ),
        help_entry(
            keybind_label(&kb.agent_attachment_picker),
            "attach file to focused agent",
        ),
        help_entry(keybind_label(&kb.focus_pane_left), "focus pane left"),
        help_entry(keybind_label(&kb.focus_pane_down), "focus pane down"),
        help_entry(keybind_label(&kb.focus_pane_up), "focus pane up"),
        help_entry(keybind_label(&kb.focus_pane_right), "focus pane right"),
        help_entry(keybind_label(&kb.cycle_pane_next), "cycle pane next"),
        help_entry(
            keybind_label(&kb.cycle_pane_previous),
            "cycle pane previous",
        ),
        help_entry(keybind_label(&kb.last_pane), "last pane"),
    ];
    groups.push(("panes", panes));

    if !kb.custom_commands.is_empty() {
        groups.push((
            "custom",
            kb.custom_commands
                .iter()
                .map(|binding| {
                    (
                        binding.label.clone(),
                        binding
                            .description
                            .clone()
                            .map(Cow::Owned)
                            .unwrap_or(Cow::Borrowed("custom command")),
                    )
                })
                .collect(),
        ));
    }

    groups
}

fn filter_keybind_help_groups(groups: Vec<HelpGroup>, query: &str) -> Vec<HelpGroup> {
    if query.is_empty() {
        return groups;
    }

    let query = query.to_lowercase();
    groups
        .into_iter()
        .filter_map(|(group, entries)| {
            let entries = entries
                .into_iter()
                .filter(|(key, label)| {
                    key.to_lowercase().contains(&query) || label.to_lowercase().contains(&query)
                })
                .collect::<Vec<_>>();
            (!entries.is_empty()).then_some((group, entries))
        })
        .collect()
}

/// Columns the stacked layout indents a label by, so it reads as belonging to
/// the shortcut above it rather than as a row of its own.
const STACKED_LABEL_INDENT: usize = 3;

/// Build the help body for a body `content_width` columns wide.
///
/// The lines are laid out to fit that width exactly — the caller must not wrap
/// them. Wrapping is what this replaced: a wrapped label continued at the left
/// edge of the next line, landing under the key column and breaking the very
/// alignment the two-column layout exists for.
///
/// When the widest `key + label` pair does not fit, every row switches to the
/// stacked form together. Deciding per row would align some rows and not
/// others; deciding for the whole list keeps one reading for the whole screen,
/// and the choice follows the content rather than a guessed breakpoint.
pub(crate) fn keybind_help_lines(app: &AppState, content_width: u16) -> Vec<Line<'static>> {
    let heading_style = Style::default()
        .fg(app.palette.accent)
        .add_modifier(Modifier::BOLD);
    let key_style = Style::default()
        .fg(app.palette.mauve)
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(app.palette.text);

    let groups = filter_keybind_help_groups(keybind_help_groups(app), &app.keybind_help.query);
    let key_width = groups
        .iter()
        .flat_map(|(_, entries)| entries.iter().map(|(key, _)| key.chars().count()))
        .max()
        .unwrap_or(8);

    if groups.is_empty() {
        return vec![Line::from(Span::styled(
            " no matching keybinds",
            Style::default().fg(app.palette.overlay1),
        ))];
    }

    let budget = content_width as usize;
    let widest_row = groups
        .iter()
        .flat_map(|(_, entries)| {
            entries
                .iter()
                .map(|(_, label)| key_width + 2 + label.chars().count())
        })
        .max()
        .unwrap_or(0);
    let stacked = widest_row > budget;

    let mut lines = Vec::new();
    for (group, entries) in groups {
        lines.push(Line::from(vec![Span::styled(
            truncate_end(&format!(" {group}"), budget),
            heading_style,
        )]));
        for (key, label) in entries {
            if stacked {
                // A key narrower than its own row is still worth showing: the
                // stacked form exists precisely for widths where the paired
                // layout cannot hold, and clipping here only bites below the
                // width at which the overlay refuses to draw a body at all.
                lines.push(Line::from(vec![Span::styled(
                    truncate_end(&format!(" {key}"), budget),
                    key_style,
                )]));
                lines.push(Line::from(vec![Span::styled(
                    format!(
                        "{}{}",
                        " ".repeat(STACKED_LABEL_INDENT.min(budget)),
                        truncate_end(&label, budget.saturating_sub(STACKED_LABEL_INDENT)),
                    ),
                    label_style,
                )]));
            } else {
                let padded_key = format!(" {:<width$} ", key, width = key_width);
                let label_budget = budget.saturating_sub(padded_key.chars().count());
                lines.push(Line::from(vec![
                    Span::styled(truncate_end(&padded_key, budget), key_style),
                    Span::styled(truncate_end(&label, label_budget), label_style),
                ]));
            }
        }
        lines.push(Line::raw(""));
    }

    lines
}

/// A body width wide enough that the help keeps its single-line layout.
#[cfg(test)]
pub(crate) const WIDE_HELP_BODY_WIDTH: u16 = 74;

/// The width the help body lays its lines out for.
///
/// Always one column narrower than the body, whether or not the scrollbar is
/// currently drawn. The scrollbar appears exactly when the content overflows,
/// and with the stacked layout the content's height now depends on the width —
/// so asking "is there a scrollbar?" before laying out would be circular.
/// Reserving the column unconditionally keeps the layout, the scroll metrics
/// and the render reading the same number.
pub(crate) fn keybind_help_layout_width(body: Rect) -> u16 {
    body.width.saturating_sub(1)
}

/// The widest search hint that fits `width`.
///
/// Shortening beats truncating: a hint cut mid-word ("…by command or short")
/// spends the same columns saying less, and the shorter phrasings still name
/// the key that opens the filter, which is the only thing the hint is for.
fn search_hint_for_width(width: u16) -> &'static str {
    const HINTS: [&str; 3] = [
        " press / to filter by command or shortcut",
        " press / to filter",
        " / filter",
    ];
    HINTS
        .into_iter()
        .find(|hint| hint.chars().count() <= width as usize)
        .unwrap_or(HINTS[HINTS.len() - 1])
}

pub(super) fn render_keybind_help_overlay(app: &AppState, frame: &mut Frame) {
    super::dim_background(frame, frame.area());

    let Some(inner) = render_modal_shell_or_notice(
        frame,
        frame.area(),
        76,
        22,
        "keybinds",
        (20, 6),
        &app.palette,
    ) else {
        return;
    };

    let stack = modal_stack_areas(inner, 2, 1, 0, 1);
    let header_rows =
        Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas::<2>(stack.header);

    render_modal_header(frame, header_rows[0], "keybinds", &app.palette);
    render_action_button(
        frame,
        release_notes_close_button_rect(header_rows[0]),
        Some("esc"),
        if app.keybind_help.search_focused {
            "back"
        } else {
            "close"
        },
        Style::default()
            .fg(panel_contrast_fg(&app.palette))
            .bg(app.palette.accent)
            .add_modifier(Modifier::BOLD),
    );
    let search_line = if app.keybind_help.search_focused {
        Line::from(vec![
            Span::styled(
                " / ",
                Style::default()
                    .fg(app.palette.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                app.keybind_help.query.as_str(),
                Style::default()
                    .fg(app.palette.text)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    } else {
        Line::from(Span::styled(
            search_hint_for_width(header_rows[1].width),
            Style::default().fg(app.palette.overlay0),
        ))
    };
    frame.render_widget(Paragraph::new(search_line), header_rows[1]);

    let body_area = stack.content;
    let metrics = crate::pane::ScrollMetrics {
        offset_from_bottom: app
            .keybind_help_max_scroll()
            .saturating_sub(app.keybind_help.scroll) as usize,
        max_offset_from_bottom: app.keybind_help_max_scroll() as usize,
        viewport_rows: body_area.height.max(1) as usize,
    };
    let track = release_notes_scrollbar_rect(body_area, metrics);
    let text_area = track
        .map(|_| {
            Rect::new(
                body_area.x,
                body_area.y,
                body_area.width.saturating_sub(1),
                body_area.height,
            )
        })
        .unwrap_or(body_area);

    let body = Paragraph::new(keybind_help_lines(
        app,
        keybind_help_layout_width(body_area),
    ))
    .scroll((app.keybind_help.scroll, 0));
    frame.render_widget(body, text_area);
    if let Some(track) = track {
        render_scrollbar(
            frame,
            metrics,
            track,
            app.palette.overlay0,
            app.palette.overlay1,
            "▐",
        );
    }

    let footer_area = stack.footer.unwrap_or_default();
    frame.render_widget(
        Paragraph::new(footer_line(app, footer_area.width)),
        footer_area,
    );
}

/// One footer hint: a description, the keys that do it, and how hard it fights
/// to stay when the footer does not fit.
struct FooterHint {
    label: &'static str,
    keys: &'static str,
    /// Lower drops first. The way out of the overlay holds the highest rank,
    /// because a hint list that fits by hiding the exit has optimised away the
    /// one thing a stuck reader needs.
    rank: u8,
}

fn footer_hints(search_focused: bool) -> Vec<FooterHint> {
    if search_focused {
        vec![
            FooterHint {
                label: "filter ",
                keys: "type/backspace",
                rank: 2,
            },
            FooterHint {
                label: "clear ",
                keys: "ctrl+u",
                rank: 0,
            },
            FooterHint {
                label: "scroll ",
                keys: "↑↓/pgup/pgdn",
                rank: 1,
            },
            FooterHint {
                label: "back ",
                keys: "esc",
                rank: 3,
            },
        ]
    } else {
        vec![
            FooterHint {
                label: "search ",
                keys: "/",
                rank: 2,
            },
            FooterHint {
                label: "scroll ",
                keys: "j/k/↑↓/pgup/pgdn",
                rank: 1,
            },
            FooterHint {
                label: "close ",
                keys: "esc/enter",
                rank: 3,
            },
        ]
    }
}

fn footer_width(hints: &[FooterHint]) -> usize {
    let separators = 3 * hints.len().saturating_sub(1);
    let content: usize = hints
        .iter()
        .map(|hint| hint.label.chars().count() + hint.keys.chars().count())
        .sum();
    1 + content + separators
}

/// Build the footer for a `width`-column strip.
///
/// Hints are dropped by rank when they do not all fit, but the survivors keep
/// their original order: a footer that reshuffles as the terminal is resized
/// makes the reader re-find every hint. The last hint is never dropped — if it
/// still does not fit, its description goes and the keys stay, because the
/// keys are the part that gets the reader out.
fn footer_line(app: &AppState, width: u16) -> Line<'static> {
    let mut hints = footer_hints(app.keybind_help.search_focused);
    while hints.len() > 1 && footer_width(&hints) > width as usize {
        let Some(weakest) = hints
            .iter()
            .enumerate()
            .min_by_key(|(_, hint)| hint.rank)
            .map(|(idx, _)| idx)
        else {
            break;
        };
        hints.remove(weakest);
    }

    let label_style = Style::default().fg(app.palette.overlay0);
    let keys_style = Style::default().fg(app.palette.text);

    if let [only] = hints.as_slice() {
        if footer_width(&hints) > width as usize {
            return Line::from(Span::styled(
                truncate_end(&format!(" {}", only.keys), width as usize),
                keys_style,
            ));
        }
    }

    let mut spans = Vec::with_capacity(hints.len() * 3);
    for (idx, hint) in hints.iter().enumerate() {
        spans.push(Span::styled(
            if idx == 0 {
                format!(" {}", hint.label)
            } else {
                hint.label.to_string()
            },
            label_style,
        ));
        spans.push(Span::styled(hint.keys.to_string(), keys_style));
        if idx + 1 < hints.len() {
            spans.push(Span::styled(" · ", label_style));
        }
    }
    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn groups() -> Vec<HelpGroup> {
        vec![
            (
                "workspaces / tabs",
                vec![
                    help_entry("w", "workspace navigation"),
                    help_entry("c", "new tab"),
                ],
            ),
            (
                "panes",
                vec![
                    help_entry("v", "split vertical"),
                    help_entry("x", "close pane"),
                ],
            ),
        ]
    }

    #[test]
    fn keybind_help_filter_matches_labels_case_insensitively() {
        let filtered = filter_keybind_help_groups(groups(), "WoRk");

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, "workspaces / tabs");
        assert_eq!(filtered[0].1.len(), 1);
        assert_eq!(filtered[0].1[0].1, "workspace navigation");
    }

    #[test]
    fn keybind_help_filter_matches_shortcuts_without_matching_group_headings() {
        let filtered = filter_keybind_help_groups(groups(), "x");

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, "panes");
        assert_eq!(filtered[0].1.len(), 1);
        assert_eq!(filtered[0].1[0].1, "close pane");

        assert!(filter_keybind_help_groups(groups(), "panes").is_empty());
    }

    #[test]
    fn help_lists_the_new_chat_tab_action() {
        let app = AppState::test_new();

        let lines = keybind_help_lines(&app, WIDE_HELP_BODY_WIDTH);

        assert!(
            lines.iter().any(|line| {
                line.spans
                    .iter()
                    .any(|span| span.content.contains("new chat tab"))
            }),
            "keybind help should list the new chat tab action"
        );
    }

    // TP-ACT-5: the file manager action is discoverable in the keybind help.
    #[test]
    fn help_lists_the_file_manager_action() {
        let app = AppState::test_new();

        let lines = keybind_help_lines(&app, WIDE_HELP_BODY_WIDTH);

        assert!(
            lines.iter().any(|line| {
                line.spans
                    .iter()
                    .any(|span| span.content.contains("file manager"))
            }),
            "keybind help should list the file manager action"
        );
    }

    /// Render the overlay into a buffer and return it row by row.
    fn help_rows(width: u16, height: u16) -> Vec<String> {
        use ratatui::{backend::TestBackend, Terminal};

        let mut app = AppState::test_new();
        app.workspaces = vec![crate::workspace::Workspace::test_new("one")];
        app.active = Some(0);
        app.selected = 0;
        app.mode = crate::app::Mode::KeybindHelp;
        crate::ui::compute_view(&mut app, Rect::new(0, 0, width, height));

        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("terminal");
        terminal
            .draw(|frame| crate::ui::render(&app, frame))
            .expect("draw");
        let buffer = terminal.backend().buffer();
        (0..height)
            .map(|y| (0..width).map(|x| buffer[(x, y)].symbol()).collect())
            .collect()
    }

    /// The keys the help lists, longest first, as they appear in a rendered row.
    fn rendered_keys(app: &AppState) -> Vec<String> {
        keybind_help_groups(app)
            .into_iter()
            .flat_map(|(_, entries)| entries.into_iter().map(|(key, _)| key))
            .collect()
    }

    // TP-MOB-11: a narrow help body stacks each label under its shortcut
    // instead of letting it wrap. The wrapped form put the tail of a label at
    // the left edge of the next line, under the key column.
    #[test]
    fn narrow_keybind_help_stacks_labels_under_their_keys() {
        let app = AppState::test_new();
        let lines = keybind_help_lines(&app, 38);
        let rendered: Vec<String> = lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect();

        let prefix_row = rendered
            .iter()
            .position(|row| row.trim() == "ctrl+b")
            .expect("the prefix shortcut gets a row of its own when stacked");
        assert_eq!(
            rendered[prefix_row + 1].trim(),
            "prefix mode",
            "the label follows on the next line, indented under its key"
        );
        assert!(
            rendered[prefix_row + 1].starts_with(&" ".repeat(STACKED_LABEL_INDENT)),
            "stacked labels are indented so they read as belonging to the key above"
        );
    }

    // TP-MOB-12: no rendered row is wider than the body it was laid out for,
    // at any width the overlay can be drawn at. This closes the whole class of
    // overflow rather than the one instance that was observed.
    #[test]
    fn keybind_help_rows_never_exceed_the_body_width() {
        let app = AppState::test_new();
        for content_width in 8..=100u16 {
            for line in keybind_help_lines(&app, content_width) {
                let rendered: String = line
                    .spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect();
                assert!(
                    rendered.chars().count() <= content_width as usize,
                    "row {rendered:?} is wider than the {content_width}-column body it was \
                     laid out for"
                );
            }
        }
    }

    // TP-MOB-13: in a rendered narrow overlay, no body row starts with the
    // tail of a label — the exact shape the wrapped layout produced.
    #[test]
    fn narrow_keybind_help_never_starts_a_row_with_a_label_tail() {
        let app = AppState::test_new();
        let keys = rendered_keys(&app);
        let rows = help_rows(44, 22);

        for row in &rows {
            let Some(body) = row.strip_prefix('|') else {
                continue;
            };
            let text = body.trim_end_matches('|');
            let trimmed = text.trim();
            if trimmed.is_empty() {
                continue;
            }
            // A body row is legitimate if it is blank, a group heading, a key,
            // an indented label, or chrome. What it must never be is a bare
            // label starting hard against the border, which is what a wrapped
            // continuation looked like.
            let starts_flush = text.starts_with(|c: char| c != ' ');
            if starts_flush {
                assert!(
                    keys.iter().any(|key| trimmed.starts_with(key.as_str()))
                        || trimmed.starts_with('─')
                        || trimmed.starts_with('│'),
                    "row {row:?} starts flush against the border with neither a key nor \
                     chrome, which is what a wrapped label tail looked like"
                );
            }
        }
    }

    // TP-MOB-14: the search hint shortens to a phrasing that still names the
    // key, instead of being cut mid-word.
    #[test]
    fn keybind_help_search_hint_shortens_instead_of_truncating() {
        assert_eq!(
            search_hint_for_width(80),
            " press / to filter by command or shortcut"
        );
        assert_eq!(search_hint_for_width(20), " press / to filter");
        assert_eq!(search_hint_for_width(12), " / filter");
        assert_eq!(search_hint_for_width(2), " / filter");
        for width in 0..=100u16 {
            let hint = search_hint_for_width(width);
            assert!(
                hint.contains('/'),
                "every hint names the key that opens the filter"
            );
        }
    }

    // TP-MOB-15: a wide help body keeps the single-line, key-aligned layout it
    // has always had.
    #[test]
    fn wide_keybind_help_keeps_the_single_line_layout() {
        let app = AppState::test_new();
        let lines = keybind_help_lines(&app, WIDE_HELP_BODY_WIDTH);
        let rendered: Vec<String> = lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect();
        assert!(
            rendered
                .iter()
                .any(|row| row.contains("ctrl+b") && row.contains("prefix mode")),
            "a wide body keeps key and label on one row"
        );
    }

    // TP-MOB-16: the footer drops its least important hint rather than being
    // cut mid-word, and never drops the way out of the overlay.
    #[test]
    fn narrow_keybind_help_footer_drops_hints_by_rank() {
        let mut app = AppState::test_new();

        let wide: String = footer_line(&app, 80)
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        assert_eq!(
            wide,
            " search / · scroll j/k/↑↓/pgup/pgdn · close esc/enter"
        );

        let narrow: String = footer_line(&app, 40)
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        assert_eq!(narrow, " search / · close esc/enter");

        let tiny: String = footer_line(&app, 8)
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        assert!(
            tiny.contains("esc"),
            "the way out of the overlay survives every width, got {tiny:?}"
        );

        app.keybind_help.search_focused = true;
        let focused: String = footer_line(&app, 8)
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        assert!(
            focused.contains("esc"),
            "the search footer also keeps its way out, got {focused:?}"
        );
    }

    // TP-MOB-17: the footer never renders wider than the strip it is given,
    // for either of its two states, at any width.
    #[test]
    fn keybind_help_footer_never_exceeds_its_strip() {
        let mut app = AppState::test_new();
        for search_focused in [false, true] {
            app.keybind_help.search_focused = search_focused;
            for width in 12..=100u16 {
                let rendered: String = footer_line(&app, width)
                    .spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect();
                assert!(
                    rendered.chars().count() <= width as usize,
                    "footer {rendered:?} exceeds its {width}-column strip \
                     (search_focused={search_focused})"
                );
            }
        }
    }
}
