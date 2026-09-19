use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use crate::ui::theme::ThemePalette;

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
