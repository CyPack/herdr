//! The work-log sheet — a chat's confirmed commits, grouped by kind.
//!
//! One conversation touches ten areas and its commits drown in the
//! transcript; this modal is the answer the user asked for by name:
//! "feature and bugfix based, not numeric". Groups come in a fixed
//! order (features first, fixes second), entries stay chronological
//! inside their group, and only CONFIRMED commits appear — an attempt
//! that left no sha behind is not work done (TP-WORKLOG-01).
//!
//! Skeleton mirrors the keybind sheet: same shell, same header, same
//! scroll contract, so the app keeps one modal language.

use ratatui::layout::{Constraint, Layout};
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::app::state::AppState;

use super::release_notes::release_notes_close_button_rect;
use super::scrollbar::{release_notes_scrollbar_rect, render_scrollbar};
use super::widgets::{
    modal_stack_areas, panel_contrast_fg, render_action_button, render_modal_header,
    render_modal_shell_or_notice,
};

/// Fixed group order: what was BUILT leads, what was FIXED follows, the
/// housekeeping kinds trail. "other" (non-conventional subjects) goes last —
/// it is the least classified, not the least real.
const KIND_ORDER: [&str; 11] = [
    "feat", "fix", "perf", "refactor", "docs", "test", "chore", "ci", "style", "revert", "other",
];

fn kind_title(kind: &str) -> &'static str {
    match kind {
        "feat" => "features",
        "fix" => "fixes",
        "perf" => "performance",
        "refactor" => "refactors",
        "docs" => "docs",
        "test" => "tests",
        "chore" => "chores",
        "ci" => "ci",
        "style" => "style",
        "revert" => "reverts",
        _ => "other",
    }
}

/// The body lines the render draws and the scroll math counts — one
/// function so the two can never disagree (the keybind sheet's contract).
pub(crate) fn chat_worklog_lines(app: &AppState) -> Vec<Line<'static>> {
    let p = &app.palette;
    let entries = app
        .chat_worklog
        .confirmed_for(&app.chat_worklog_modal.session_id);
    if entries.is_empty() {
        return vec![Line::from(Span::styled(
            "(no confirmed commits)",
            Style::default().fg(p.overlay0),
        ))];
    }
    let mut lines = Vec::new();
    for kind in KIND_ORDER {
        let mut group: Vec<_> = entries.iter().filter(|e| e.kind == kind).collect();
        if group.is_empty() {
            continue;
        }
        group.sort_by(|a, b| a.ts.cmp(&b.ts));
        if !lines.is_empty() {
            lines.push(Line::default());
        }
        lines.push(Line::from(Span::styled(
            format!("{} — {}", kind_title(kind), group.len()),
            Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
        )));
        for entry in group {
            let mut spans = vec![
                Span::raw("  "),
                Span::styled(entry.subject.clone(), Style::default().fg(p.text)),
            ];
            let mut meta = Vec::new();
            if let Some(scope) = entry.scope.as_deref() {
                if !scope.is_empty() {
                    meta.push(scope.to_string());
                }
            }
            if let Some(branch) = entry.branch.as_deref() {
                meta.push(branch.to_string());
            }
            if let Some(sha) = entry.sha.as_deref() {
                meta.push(sha.chars().take(7).collect());
            }
            if !meta.is_empty() {
                spans.push(Span::styled(
                    format!("  {}", meta.join(" · ")),
                    Style::default().fg(p.overlay0),
                ));
            }
            if entry.pushed {
                spans.push(Span::styled(" ↑pushed", Style::default().fg(p.green)));
            }
            lines.push(Line::from(spans));
        }
    }
    lines
}

pub(super) fn render_chat_worklog_overlay(app: &AppState, frame: &mut Frame) {
    super::dim_background(frame, frame.area());

    let Some(inner) = render_modal_shell_or_notice(
        frame,
        frame.area(),
        76,
        22,
        "work log",
        (20, 6),
        &app.palette,
    ) else {
        return;
    };

    let stack = modal_stack_areas(inner, 2, 1, 0, 1);
    let header_rows =
        Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas::<2>(stack.header);

    render_modal_header(frame, header_rows[0], "work log", &app.palette);
    render_action_button(
        frame,
        release_notes_close_button_rect(header_rows[0]),
        Some("esc"),
        "close",
        Style::default()
            .fg(panel_contrast_fg(&app.palette))
            .bg(app.palette.accent)
            .add_modifier(Modifier::BOLD),
    );
    // The chat's own name under the title, so the sheet says whose work it is.
    let chat_label = app
        .workspace_chat_rows
        .values()
        .flatten()
        .find(|row| row.session_id == app.chat_worklog_modal.session_id)
        .map(|row| row.display_label())
        .unwrap_or_else(|| app.chat_worklog_modal.session_id.clone());
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            chat_label,
            Style::default().fg(app.palette.overlay0),
        ))),
        header_rows[1],
    );

    let body_area = stack.content;
    let metrics = crate::pane::ScrollMetrics {
        offset_from_bottom: app
            .chat_worklog_max_scroll()
            .saturating_sub(app.chat_worklog_modal.scroll) as usize,
        max_offset_from_bottom: app.chat_worklog_max_scroll() as usize,
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

    let body = Paragraph::new(chat_worklog_lines(app)).scroll((app.chat_worklog_modal.scroll, 0));
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::Mode;
    use ratatui::backend::TestBackend;

    fn entry(kind: &str, subject: &str, ts: &str) -> crate::persist::chat_worklog::WorkLogEntry {
        crate::persist::chat_worklog::WorkLogEntry {
            repo: "/repo".into(),
            branch: Some("feat/x".into()),
            kind: kind.into(),
            scope: Some("scan".into()),
            subject: subject.into(),
            sha: Some("abc1234def".into()),
            ts: ts.into(),
            pushed: kind == "feat",
            status: "committed".into(),
        }
    }

    fn worklog_rows(app: &mut AppState, width: u16, height: u16) -> Vec<String> {
        let area = Rect::new(0, 0, width, height);
        crate::ui::compute_view(app, area);
        let backend = TestBackend::new(width, height);
        let mut terminal = ratatui::Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|frame| render_chat_worklog_overlay(app, frame))
            .expect("draw");
        let buffer = terminal.backend().buffer();
        (0..height)
            .map(|y| (0..width).map(|x| buffer[(x, y)].symbol()).collect())
            .collect()
    }

    // TP-WORKLOG-01
    #[test]
    fn the_sheet_groups_commits_by_kind_with_features_first() {
        let mut app = AppState::test_new();
        app.chat_worklog.chats.insert(
            "sid-1".into(),
            vec![
                entry("fix", "repair the seam", "2026-08-20T11:00:00Z"),
                entry("feat", "build the scanner", "2026-08-20T10:00:00Z"),
            ],
        );
        app.chat_worklog_modal.session_id = "sid-1".into();
        app.mode = Mode::ChatWorkLog;
        let rows = worklog_rows(&mut app, 90, 26);
        let all = rows.join("\n");
        let features = all.find("features — 1").expect("features group drawn");
        let fixes = all.find("fixes — 1").expect("fixes group drawn");
        assert!(
            features < fixes,
            "what was built leads, what was fixed follows"
        );
        assert!(all.contains("build the scanner"));
        assert!(all.contains("↑pushed"), "the pushed marker is drawn");
        assert!(all.contains("abc1234"), "the short sha is drawn");
    }

    // TP-WORKLOG-01 — attempts without a sha are not work done
    #[test]
    fn an_unconfirmed_attempt_is_not_listed() {
        let mut app = AppState::test_new();
        let mut attempted = entry("fix", "never landed", "2026-08-20T11:00:00Z");
        attempted.sha = None;
        app.chat_worklog.chats.insert(
            "sid-1".into(),
            vec![attempted, entry("feat", "landed", "2026-08-20T10:00:00Z")],
        );
        app.chat_worklog_modal.session_id = "sid-1".into();
        app.mode = Mode::ChatWorkLog;
        let all = worklog_rows(&mut app, 90, 26).join("\n");
        assert!(all.contains("landed"));
        assert!(!all.contains("never landed"));
    }
}
