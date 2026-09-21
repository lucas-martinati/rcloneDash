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

/// Extrait un horodatage en début de ligne de log s'il est présent.
/// Supporte les formats ISO (YYYY-MM-DD...), date standard (YYYY/MM/DD...) et heure seule (HH:MM:SS...).
pub fn split_log_timestamp(line: &str) -> Option<(&str, &str)> {
    let bytes = line.as_bytes();
    let len = bytes.len();

    // 1. Format ISO ou date standard: "YYYY-MM-DD HH:MM:SS" ou "YYYY/MM/DD HH:MM:SS" (au moins 19 chars)
    if len >= 19 {
        let is_year = bytes[0..4].iter().all(|b| b.is_ascii_digit());
        let sep1 = bytes[4];
        let sep2 = bytes[7];
        let is_month = bytes[5].is_ascii_digit() && bytes[6].is_ascii_digit();
        let is_day = bytes[8].is_ascii_digit() && bytes[9].is_ascii_digit();
        let sep3 = bytes[10];
        let is_hour = bytes[11].is_ascii_digit() && bytes[12].is_ascii_digit();
        let is_min = bytes[14].is_ascii_digit() && bytes[15].is_ascii_digit();
        let is_sec = bytes[17].is_ascii_digit() && bytes[18].is_ascii_digit();

        if is_year
            && (sep1 == b'-' || sep1 == b'/')
            && (sep2 == b'-' || sep2 == b'/')
            && is_month
            && is_day
            && (sep3 == b' ' || sep3 == b'T')
            && is_hour
            && bytes[13] == b':'
            && is_min
            && bytes[16] == b':'
            && is_sec
        {
            let mut end_ts = 19;
            while end_ts < len && bytes[end_ts] != b' ' && bytes[end_ts] != b'\t' {
                end_ts += 1;
            }
            let mut start_rest = end_ts;
            while start_rest < len && (bytes[start_rest] == b' ' || bytes[start_rest] == b'\t') {
                start_rest += 1;
            }
            return Some((&line[..end_ts], &line[start_rest..]));
        }
    }

    // 2. Format court: "HH:MM:SS" (au moins 8 chars)
    if len >= 8
        && bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && bytes[2] == b':'
        && bytes[3].is_ascii_digit()
        && bytes[4].is_ascii_digit()
        && bytes[5] == b':'
        && bytes[6].is_ascii_digit()
        && bytes[7].is_ascii_digit()
    {
        let mut end_ts = 8;
        while end_ts < len && (bytes[end_ts].is_ascii_digit() || bytes[end_ts] == b'.' || bytes[end_ts] == b'+' || bytes[end_ts] == b'-') {
            end_ts += 1;
        }
        if end_ts == len || bytes[end_ts] == b' ' || bytes[end_ts] == b'\t' {
            let mut start_rest = end_ts;
            while start_rest < len && (bytes[start_rest] == b' ' || bytes[start_rest] == b'\t') {
                start_rest += 1;
            }
            return Some((&line[..end_ts], &line[start_rest..]));
        }
    }

    None
}

/// Counts total wrapped lines for a single log line.
pub fn count_wrapped_line(line: &str, max_width: usize) -> usize {
    let sanitized = line.replace('\t', "    ");
    if let Some((ts, rest)) = split_log_timestamp(&sanitized) {
        let ts_len = ts.chars().count() + 2;
        let rest_first = max_width.saturating_sub(ts_len);
        let cont = max_width.saturating_sub(4);
        wrap_text(rest, rest_first, cont).len()
    } else {
        let cont = max_width.saturating_sub(4);
        wrap_text(&sanitized, max_width, cont).len()
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

/// Colorise intelligemment une portion de log (hors timestamp) selon son contenu sémantique.
fn colorize_log_part(part: &str, theme: &ThemePalette) -> Vec<Span<'static>> {
    let ll = part.to_lowercase();

    // 1. Erreur critique
    if ll.contains("error") || ll.contains("failed") || ll.contains("fatal") || ll.contains("critical") {
        return vec![Span::styled(
            part.to_string(),
            Style::default().fg(theme.red).add_modifier(Modifier::BOLD),
        )];
    }

    // 2. Succès bisync
    if ll.contains("bisync successful") {
        return vec![Span::styled(
            part.to_string(),
            Style::default().fg(theme.green).add_modifier(Modifier::BOLD),
        )];
    }

    // 3. Fichier synchronisé avec action à la fin (ex: "path/file.txt: Copied (new)")
    if let Some(colon_pos) = part.rfind(':') {
        let after_colon = part[colon_pos + 1..].trim();
        let after_lower = after_colon.to_lowercase();
        let action_style = if after_lower.contains("copied (new)") {
            Some(Style::default().fg(theme.green).add_modifier(Modifier::BOLD))
        } else if after_lower.contains("copied (replaced") || after_lower.contains("updated file") || after_lower.contains("updated mod") {
            Some(Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD))
        } else if after_lower.contains("deleted") {
            Some(Style::default().fg(theme.red).add_modifier(Modifier::BOLD))
        } else {
            None
        };

        if let Some(act_style) = action_style {
            let before = &part[..colon_pos + 1];
            return vec![
                Span::styled(before.to_string(), Style::default().fg(theme.text_bright)),
                Span::styled(format!(" {}", after_colon), act_style),
            ];
        }
    }

    // 4. Ligne avec préfixe de niveau rclone (ex: "INFO  : message")
    for (prefix, col, bold) in [
        ("INFO  :", theme.cyan, false),
        ("INFO:", theme.cyan, false),
        ("NOTICE:", theme.yellow, false),
        ("WARN  :", theme.yellow, true),
        ("WARNING:", theme.yellow, true),
        ("DEBUG :", theme.text_muted, false),
        ("DEBUG:", theme.text_muted, false),
    ] {
        if let Some(after) = part.strip_prefix(prefix) {
            let mut p_style = Style::default().fg(col);
            if bold {
                p_style = p_style.add_modifier(Modifier::BOLD);
            }
            return vec![
                Span::styled(prefix.to_string(), p_style),
                Span::styled(after.to_string(), Style::default().fg(theme.text_bright)),
            ];
        }
    }

    // 5. Différentiels bisync (ex: "- Path1 File is new - path")
    if (part.starts_with("- Path1") || part.starts_with("- Path2") || part.starts_with("Path1:") || part.starts_with("Path2:"))
        && (ll.contains("file is new") || ll.contains("file changed") || ll.contains("file was deleted") || ll.contains("file deleted") || ll.contains("queue copy"))
    {
        let (act_str, act_style) = if ll.contains("file is new") {
            ("File is new", Style::default().fg(theme.green).add_modifier(Modifier::BOLD))
        } else if ll.contains("file changed") {
            ("File changed", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD))
        } else if ll.contains("file was deleted") || ll.contains("file deleted") {
            ("File deleted", Style::default().fg(theme.red).add_modifier(Modifier::BOLD))
        } else {
            ("Queue copy", Style::default().fg(theme.green))
        };

        if let Some(pos) = part.to_lowercase().find(&act_str.to_lowercase()) {
            let before = &part[..pos];
            let after = &part[pos + act_str.len()..];
            return vec![
                Span::styled(before.to_string(), Style::default().fg(theme.cyan)),
                Span::styled(part[pos..pos + act_str.len()].to_string(), act_style),
                Span::styled(after.to_string(), Style::default().fg(theme.text_bright)),
            ];
        }
    }

    // 6. Stats de transfert (Transferred:, Checks:, Elapsed time:)
    if ll.starts_with("transferred:") || ll.starts_with("checks:") || ll.starts_with("elapsed time:") {
        if let Some(colon_pos) = part.find(':') {
            let label = &part[..colon_pos + 1];
            let value = &part[colon_pos + 1..];
            return vec![
                Span::styled(label.to_string(), Style::default().fg(theme.cyan)),
                Span::styled(value.to_string(), Style::default().fg(theme.text_bright)),
            ];
        }
        return vec![Span::styled(part.to_string(), Style::default().fg(theme.cyan))];
    }

    // 7. Avertissement générique
    if ll.contains("warning") || ll.contains("warn") {
        return vec![Span::styled(part.to_string(), Style::default().fg(theme.yellow))];
    }

    // 8. Défaut : texte clair standard
    vec![Span::styled(part.to_string(), Style::default().fg(theme.text_bright))]
}

/// Wraps and colorizes a single log line according to log level and message content.
pub fn wrap_and_colorize_log_line(line: &str, max_width: usize, theme: &ThemePalette) -> Vec<Line<'static>> {
    let sanitized = line.replace('\t', "    ");

    if let Some((ts, rest)) = split_log_timestamp(&sanitized) {
        let ts_len = ts.chars().count() + 2;
        let rest_first_limit = max_width.saturating_sub(ts_len);
        let cont_limit = max_width.saturating_sub(4);

        let wrapped = wrap_text(rest, rest_first_limit, cont_limit);
        let mut lines = Vec::with_capacity(wrapped.len());

        for (i, part) in wrapped.into_iter().enumerate() {
            let mut spans = Vec::new();
            if i == 0 {
                spans.push(Span::styled(format!("{}  ", ts), Style::default().fg(theme.text_muted)));
            } else {
                spans.push(Span::styled("  ↳ ".to_string(), Style::default().fg(theme.text_muted)));
            }
            spans.extend(colorize_log_part(&part, theme));
            lines.push(Line::from(spans));
        }
        lines
    } else {
        let cont_limit = max_width.saturating_sub(4);
        let wrapped = wrap_text(&sanitized, max_width, cont_limit);
        let mut lines = Vec::with_capacity(wrapped.len());

        for (i, part) in wrapped.into_iter().enumerate() {
            let mut spans = Vec::new();
            if i > 0 {
                spans.push(Span::styled("  ↳ ".to_string(), Style::default().fg(theme.text_muted)));
            }
            spans.extend(colorize_log_part(&part, theme));
            lines.push(Line::from(spans));
        }
        lines
    }
}
