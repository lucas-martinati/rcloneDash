use ratatui::{
    style::Style,
    text::{Line, Span},
};

use crate::ui::theme::ThemePalette;

pub fn colorize_log_line<'a>(line: &'a str, theme: &ThemePalette) -> Line<'a> {
    if let Some((ts, rest)) = crate::ui::dashboard::logs::split_log_timestamp(line) {
        let mut spans = vec![
            Span::styled(format!("{}  ", ts), Style::default().fg(theme.text_muted)),
        ];
        spans.extend(crate::ui::dashboard::logs::colorize_log_part(rest, theme));
        Line::from(spans)
    } else {
        Line::from(crate::ui::dashboard::logs::colorize_log_part(line, theme))
    }
}
