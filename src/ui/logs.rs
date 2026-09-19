#![allow(dead_code)]

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use crate::app::App;
use crate::ui::theme::ThemePalette;

pub fn render_logs(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect) {
    let title_badge = if app.auto_scroll {
        Span::styled(" [AUTO-SCROLL: ON] ", Style::default().fg(theme.green).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(" [PAUSED - SPACE TO RESUME] ", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD))
    };

    let total_lines = app.live.log_lines.len();
    let visible_height = area.height.saturating_sub(2) as usize;

    let skip_count = if app.auto_scroll {
        total_lines.saturating_sub(visible_height)
    } else {
        let max_scroll = total_lines.saturating_sub(visible_height);
        max_scroll.saturating_sub(app.logs_scroll)
    };

    let lines: Vec<Line> = app
        .live
        .log_lines
        .iter()
        .skip(skip_count)
        .take(visible_height)
        .map(|line| colorize_log_line(line, theme))
        .collect();

    let p = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(app.border_type())
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.card_bg))
            .title(Line::from(vec![
                Span::styled(" Live Logs (journalctl -f) ", Style::default().fg(theme.accent)),
                title_badge,
            ])),
    );

    f.render_widget(p, area);

    // Scrollbar latérale
    if total_lines > visible_height {
        let current_pos = if app.auto_scroll {
            total_lines
        } else {
            total_lines.saturating_sub(app.logs_scroll)
        };

        let mut scrollbar_state = ScrollbarState::new(total_lines)
            .position(current_pos);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_style(Style::default().fg(theme.border))
            .thumb_style(Style::default().fg(theme.accent));
        f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}

pub fn colorize_log_line<'a>(line: &'a str, theme: &ThemePalette) -> Line<'a> {
    let ll = line.to_lowercase();

    let (prefix_color, is_bold) = if ll.contains("error") || ll.contains("failed") || ll.contains("critical") {
        (theme.red, true)
    } else if ll.contains("notice") || ll.contains("warning") || ll.contains("warn") {
        (theme.yellow, false)
    } else if ll.contains("bisync successful") || ll.contains("copied (new)") {
        (theme.green, true)
    } else if ll.contains("transferred:") || ll.contains("checks:") {
        (theme.accent, false)
    } else {
        (theme.text_bright, false)
    };

    let mut style = Style::default().fg(prefix_color);
    if is_bold {
        style = style.add_modifier(Modifier::BOLD);
    }

    if line.len() > 25 && line.chars().nth(4) == Some('-') && line.chars().nth(7) == Some('-') {
        let ts = &line[..25];
        let rest = &line[25..];
        Line::from(vec![
            Span::styled(ts, Style::default().fg(theme.text_muted)),
            Span::styled(rest, style),
        ])
    } else {
        Line::from(Span::styled(line, style))
    }
}
