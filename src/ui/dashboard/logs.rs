use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::{App, FocusedPanel, HitAction, Hitbox, LogFilter};
use crate::ui::theme::ThemePalette;

/// Renders the Live Logs streaming panel with interactive filter tabs and auto-scroll control.
pub fn render_logs_panel(
    f: &mut Frame,
    app: &App,
    theme: &ThemePalette,
    area: Rect,
    hitboxes: &mut Vec<Hitbox>,
) {
    // Clear area to eliminate any ghost characters from previous renders or modals
    f.render_widget(ratatui::widgets::Clear, area);

    // Register logs area for mouse scroll
    hitboxes.push(Hitbox {
        rect: area,
        action: HitAction::LogsArea,
    });

    let is_focused = app.focused_panel == FocusedPanel::Logs;
    let border_color = if is_focused { theme.border_focus } else { theme.border_logs };

    let max_text_width = (area.width.saturating_sub(3) as usize).max(20);
    let mut all_wrapped: Vec<Line> = Vec::new();

    for line in &app.live.log_lines {
        if app.log_filter.matches(line) {
            all_wrapped.extend(wrap_and_colorize_log_line(line, max_text_width, theme));
        }
    }

    if all_wrapped.is_empty() {
        let msg = match app.log_filter {
            LogFilter::All => " No logs available at the moment.",
            LogFilter::Files => " No file transfers in recent logs.",
            LogFilter::Problems => " No issues (errors/warnings) detected.",
        };
        all_wrapped.push(Line::from(vec![Span::styled(msg, Style::default().fg(theme.text_muted))]));
    }

    let total_lines = all_wrapped.len();

    let visible_height = area.height.saturating_sub(2) as usize;
    let max_scroll = total_lines.saturating_sub(visible_height);
    let effective_scroll = app.logs_scroll.min(max_scroll);

    let cur_line = if app.auto_scroll {
        total_lines
    } else {
        total_lines.saturating_sub(effective_scroll)
    };

    let (up_col, down_col) = if total_lines <= visible_height {
        (theme.text_muted, theme.text_muted)
    } else if effective_scroll == 0 {
        (theme.red, theme.text_muted)
    } else if effective_scroll >= max_scroll {
        (theme.text_muted, theme.red)
    } else {
        (theme.red, theme.red)
    };

    let bg = app.border_glyphs();
    let left_bottom = Line::from(vec![
        Span::styled(bg.bot_left, Style::default().fg(border_color)),
        Span::styled("↑", Style::default().fg(up_col).add_modifier(Modifier::BOLD)),
        Span::styled(" scroll ", Style::default().fg(Color::White)),
        Span::styled("↓", Style::default().fg(down_col).add_modifier(Modifier::BOLD)),
        Span::styled(bg.bot_right, Style::default().fg(border_color)),
    ]);
    let right_bottom = Line::from(vec![
        Span::styled(format!("{} {}/{} {}", bg.horizontal, cur_line, total_lines, bg.horizontal), Style::default().fg(border_color).add_modifier(Modifier::BOLD)),
    ]);

    let mut title_spans: Vec<Span> = Vec::new();
    title_spans.push(Span::styled(bg.top_left, Style::default().fg(border_color)));
    title_spans.push(Span::styled("⁴", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)));
    title_spans.push(Span::styled("logs", Style::default().fg(border_color).add_modifier(Modifier::BOLD)));
    title_spans.push(Span::styled(format!("{}{}", bg.top_right, bg.top_left), Style::default().fg(border_color)));

    hitboxes.push(Hitbox {
        rect: Rect {
            x: area.x,
            y: area.y,
            width: 8,
            height: 1,
        },
        action: HitAction::ToggleBox(4),
    });

    // area.x + 1 is start of block title inside border.
    // "┐" (1) + "⁴" (1) + "logs" (4) + "┌┐" (2) = 8 chars, so selector starts at area.x + 1 + 8 = area.x + 9
    let mut cur_hit_x = area.x + 9;

    // Left arrow button: ←
    hitboxes.push(Hitbox {
        rect: Rect {
            x: cur_hit_x,
            y: area.y,
            width: 1,
            height: 1,
        },
        action: HitAction::LogFilterPrev,
    });
    title_spans.push(Span::styled("←", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)));
    cur_hit_x += 1;

    // Filter label: e.g. " All " / " Files " / " Problems "
    let filter_text = format!(" {} ", app.log_filter.label());
    let label_len = filter_text.chars().count() as u16;
    hitboxes.push(Hitbox {
        rect: Rect {
            x: cur_hit_x,
            y: area.y,
            width: label_len,
            height: 1,
        },
        action: HitAction::LogFilterCycle,
    });
    title_spans.push(Span::styled(
        filter_text,
        Style::default().fg(if is_focused { theme.green } else { Color::White }).add_modifier(Modifier::BOLD),
    ));
    cur_hit_x += label_len;

    // Right arrow button: →
    hitboxes.push(Hitbox {
        rect: Rect {
            x: cur_hit_x,
            y: area.y,
            width: 1,
            height: 1,
        },
        action: HitAction::LogFilterNext,
    });
    title_spans.push(Span::styled("→", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)));
    cur_hit_x += 1;

    title_spans.push(Span::styled(format!("{}{}", bg.top_right, bg.top_left), Style::default().fg(border_color)));
    cur_hit_x += 2;

    let auto_text = if app.auto_scroll { "pause " } else { "auto " };
    let space_glyph = "␣";
    let status_text = if app.auto_scroll { " [ON]" } else { " [OFF]" };
    let auto_width = (auto_text.chars().count() + 1 + status_text.chars().count()) as u16;

    title_spans.push(Span::styled(auto_text, Style::default().fg(theme.text_bright)));
    title_spans.push(Span::styled(space_glyph, Style::default().fg(theme.red).add_modifier(Modifier::BOLD)));
    title_spans.push(Span::styled(
        status_text,
        Style::default().fg(if app.auto_scroll { theme.green } else { theme.yellow }).add_modifier(Modifier::BOLD),
    ));
    title_spans.push(Span::styled(bg.top_right, Style::default().fg(border_color)));

    hitboxes.push(Hitbox {
        rect: Rect {
            x: cur_hit_x,
            y: area.y,
            width: auto_width,
            height: 1,
        },
        action: HitAction::ToggleLogsAuto,
    });

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(app.border_type())
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.card_bg))
        .title(Line::from(title_spans))
        .title_bottom(left_bottom.alignment(Alignment::Left))
        .title_bottom(right_bottom.alignment(Alignment::Right));

    let skip_count = if app.auto_scroll {
        max_scroll
    } else {
        max_scroll.saturating_sub(effective_scroll)
    };

    let lines: Vec<Line> = all_wrapped
        .into_iter()
        .skip(skip_count)
        .take(visible_height)
        .collect();

    let p = Paragraph::new(lines).block(outer_block);
    f.render_widget(p, area);

    // Integrated scrollbar
    let current_pos = if app.auto_scroll {
        max_scroll
    } else {
        max_scroll.saturating_sub(effective_scroll)
    };
    crate::ui::render_scrollbar(
        f,
        area,
        total_lines,
        current_pos,
        visible_height,
        theme,
        hitboxes,
        crate::app::ScrollbarTarget::Logs,
    );
}

/// Wraps text cleanly to prevent horizontal cropping in terminal windows.
/// Preserves words when possible and breaks long words cleanly without UTF-8 panics.
pub fn wrap_text(text: &str, first_max: usize, cont_max: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }

    let first_limit = first_max.max(10);
    let cont_limit = cont_max.max(10);

    let mut lines = Vec::new();
    let mut cur_line = String::new();
    let mut cur_limit = first_limit;

    for word in text.split(' ') {
        if word.is_empty() {
            if !cur_line.is_empty() && cur_line.chars().count() < cur_limit {
                cur_line.push(' ');
            }
            continue;
        }

        let cur_len = cur_line.chars().count();
        let word_len = word.chars().count();
        let needed = if cur_len == 0 { word_len } else { cur_len + 1 + word_len };

        if needed <= cur_limit {
            if cur_len > 0 {
                cur_line.push(' ');
            }
            cur_line.push_str(word);
        } else if word_len > cur_limit {
            if cur_len > 0 {
                lines.push(cur_line);
                cur_line = String::new();
                cur_limit = cont_limit;
            }
            let mut rem = word;
            while !rem.is_empty() {
                let count = rem.chars().count();
                if count <= cur_limit {
                    cur_line.push_str(rem);
                    break;
                } else {
                    let split_idx = rem.char_indices().nth(cur_limit).map(|(i, _)| i).unwrap_or(rem.len());
                    lines.push(rem[..split_idx].to_string());
                    rem = &rem[split_idx..];
                    cur_limit = cont_limit;
                }
            }
        } else {
            if !cur_line.is_empty() {
                lines.push(cur_line);
            }
            cur_line = word.to_string();
            cur_limit = cont_limit;
        }
    }

    if !cur_line.is_empty() || lines.is_empty() {
        lines.push(cur_line);
    }

    lines
}

/// Counts total wrapped lines for a single log line.
pub fn count_wrapped_line(line: &str, max_width: usize) -> usize {
    let sanitized = line.replace('\t', "    ");
    let line = &sanitized;
    if line.len() > 25 && line.chars().nth(4) == Some('-') && line.chars().nth(7) == Some('-') {
        let rest = &line[25..];
        let rest_first = max_width.saturating_sub(25);
        let cont = max_width.saturating_sub(4);
        wrap_text(rest, rest_first, cont).len()
    } else {
        let cont = max_width.saturating_sub(4);
        wrap_text(line, max_width, cont).len()
    }
}

/// Counts total wrapped lines across a collection of log lines with filtering.
pub fn count_wrapped_log_lines(lines: &std::collections::VecDeque<String>, filter: LogFilter, max_width: usize) -> usize {
    let mut total = 0;
    for l in lines {
        if filter.matches(l) {
            total += count_wrapped_line(l, max_width);
        }
    }
    total
}

/// Wraps and colorizes a single log line according to log level and message content.
pub fn wrap_and_colorize_log_line(line: &str, max_width: usize, theme: &ThemePalette) -> Vec<Line<'static>> {
    let sanitized = line.replace('\t', "    ");
    let line = &sanitized;
    let ll = line.to_lowercase();

    let (prefix_color, is_bold) = if ll.contains("error") || ll.contains("failed") || ll.contains("critical") {
        (theme.red, true)
    } else if ll.contains("notice") || ll.contains("warning") || ll.contains("warn") {
        (theme.yellow, false)
    } else if ll.contains("bisync successful") || ll.contains("copied (new)") {
        (theme.green, true)
    } else if ll.contains("transferred:") || ll.contains("checks:") {
        (theme.cyan, false)
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
        let rest_first_limit = max_width.saturating_sub(25);
        let cont_limit = max_width.saturating_sub(4);

        let wrapped = wrap_text(rest, rest_first_limit, cont_limit);
        let mut lines = Vec::with_capacity(wrapped.len());

        for (i, part) in wrapped.into_iter().enumerate() {
            if i == 0 {
                lines.push(Line::from(vec![
                    Span::styled(ts.to_string(), Style::default().fg(theme.text_muted)),
                    Span::styled(part, style),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("  ↳ ".to_string(), Style::default().fg(theme.text_muted)),
                    Span::styled(part, style),
                ]));
            }
        }
        lines
    } else {
        let cont_limit = max_width.saturating_sub(4);
        let wrapped = wrap_text(line, max_width, cont_limit);
        let mut lines = Vec::with_capacity(wrapped.len());

        for (i, part) in wrapped.into_iter().enumerate() {
            if i == 0 {
                lines.push(Line::from(vec![Span::styled(part, style)]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("  ↳ ".to_string(), Style::default().fg(theme.text_muted)),
                    Span::styled(part, style),
                ]));
            }
        }
        lines
    }
}
