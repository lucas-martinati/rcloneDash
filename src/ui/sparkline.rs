use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::monitor::history::{PastRun, RunStatus};
use crate::ui::theme::ThemePalette;

pub const BLOCKS: [char; 8] = [' ', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Render speed sparkline with btop++ gradient colors
#[allow(dead_code)]
pub fn render_speed_sparkline(
    f: &mut Frame,
    speed_history: &[u64],
    current_speed: &str,
    theme: &ThemePalette,
    area: Rect,
) {
    let max_val = speed_history.iter().copied().max().unwrap_or(0).max(1);

    let mut spans = Vec::new();
    let display_len = (area.width.saturating_sub(4) as usize).min(speed_history.len());
    let start_idx = speed_history.len().saturating_sub(display_len);

    for &speed_kib in &speed_history[start_idx..] {
        let level = if speed_kib > 0 {
            (((speed_kib as f64 / max_val as f64) * 6.0).ceil() as usize).clamp(1, 7)
        } else {
            0
        };
        let block = if level == 0 { '·' } else { BLOCKS[level] };
        let color = theme.speed_gradient_color(speed_kib);
        spans.push(Span::styled(block.to_string(), Style::default().fg(color)));
    }

    let title = if current_speed.is_empty() || current_speed == "--" {
        " ⚡ Débit Réseau ".to_string()
    } else {
        format!(" ⚡ Débit Réseau : {} ", current_speed)
    };

    let p = Paragraph::new(Line::from(spans))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.card_bg))
                .title(Span::styled(title, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))),
        );

    f.render_widget(p, area);
}

/// Render duration sparkline of past runs with status and gradient colors
#[allow(dead_code)]
pub fn render_history_sparkline(
    past_runs: &[PastRun],
    theme: &ThemePalette,
    width: usize,
    selected_idx: Option<usize>,
) -> Line<'static> {
    if past_runs.is_empty() {
        return Line::from(vec![Span::styled("[Aucun run]", Style::default().fg(theme.text_muted))]);
    }

    let count = past_runs.len().min(width).min(24);
    let runs_slice: Vec<&PastRun> = past_runs.iter().take(count).rev().collect();

    let durations: Vec<f64> = runs_slice.iter().map(|r| parse_duration_seconds(&r.duration)).collect();
    let max_dur = durations.iter().copied().fold(0.0f64, f64::max).max(1.0);

    let mut spans = Vec::new();

    for (i, run) in runs_slice.iter().enumerate() {
        let original_idx = count.saturating_sub(1 + i);
        let is_sel = selected_idx == Some(original_idx);

        let dur = durations[i];
        let level = if dur > 0.0 {
            (((dur / max_dur) * 6.0).ceil() as usize).clamp(1, 7)
        } else {
            1
        };
        let block = BLOCKS[level];

        let color = match run.status {
            RunStatus::Success => {
                if level <= 2 {
                    theme.green
                } else if level <= 4 {
                    theme.cyan
                } else if level <= 6 {
                    theme.yellow
                } else {
                    theme.orange
                }
            }
            RunStatus::Failed => theme.red,
            RunStatus::Skipped => theme.text_muted,
            RunStatus::Running => theme.highlight,
        };

        let mut style = Style::default().fg(color).add_modifier(Modifier::BOLD);
        if is_sel {
            style = style.bg(theme.border_focus).add_modifier(Modifier::UNDERLINED);
        }

        spans.push(Span::styled(block.to_string(), style));
        spans.push(Span::styled(" ", Style::default()));
    }

    Line::from(spans)
}

/// Renders a multi-line 2D duration bar chart (btop++ style) for past runs.
/// Returns the rendered lines and the column layout `(col_start_x, col_width, original_idx)` for hitboxes.
pub fn render_history_graph_multiline(
    past_runs: &[PastRun],
    theme: &ThemePalette,
    width: usize,
    height: usize,
    selected_idx: Option<usize>,
) -> (Vec<Line<'static>>, Vec<(usize, usize, usize)>) {
    if past_runs.is_empty() || width < 10 || height < 2 {
        let empty_line = Line::from(vec![Span::styled(" [Aucun run dans l'historique]", Style::default().fg(theme.text_muted))]);
        return (vec![empty_line], vec![]);
    }

    let chart_height = height.saturating_sub(1).max(1);
    let col_width = if width >= 50 { 2 } else { 1 };
    let gap = 1;
    let col_total_width = col_width + gap;

    let margin_left = 1;
    let usable_width = width.saturating_sub(margin_left + 1);
    let max_cols = usable_width / col_total_width;
    let count = past_runs.len().min(max_cols).min(32);

    if count == 0 {
        return (vec![Line::from(vec![Span::styled(" [Espace insuffisant pour le graphe]", Style::default().fg(theme.text_muted))])], vec![]);
    }

    // Chronological order: oldest on left, newest on right
    let runs_slice: Vec<(usize, &PastRun)> = (0..count).rev().map(|orig_idx| (orig_idx, &past_runs[orig_idx])).collect();

    let durations: Vec<f64> = runs_slice.iter().map(|(_, r)| parse_duration_seconds(&r.duration)).collect();
    let max_dur = durations.iter().copied().fold(0.0f64, f64::max).max(1.0);
    let total_levels = chart_height * 8;

    let mut hitboxes_coords = Vec::new();
    for (i, (orig_idx, _)) in runs_slice.iter().enumerate() {
        let col_x = margin_left + i * col_total_width;
        hitboxes_coords.push((col_x, col_width, *orig_idx));
    }

    // --- Line 0: Header with stats and selection info ---
    let mut header_spans = Vec::new();
    header_spans.push(Span::styled("⏱ DURATION ", Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD)));

    if let Some(sel) = selected_idx {
        if let Some(r) = past_runs.get(sel) {
            let status_span = match r.status {
                RunStatus::Success => Span::styled("● OK", Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
                RunStatus::Failed => Span::styled("● FAILED", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
                RunStatus::Skipped => Span::styled("● SKIPPED", Style::default().fg(theme.text_muted)),
                RunStatus::Running => Span::styled("● RUNNING", Style::default().fg(theme.highlight)),
            };
            header_spans.push(Span::styled(format!("Run #{} : {} (", sel + 1, r.duration), Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)));
            header_spans.push(status_span);
            header_spans.push(Span::styled(format!(") · {} ", r.time), Style::default().fg(theme.text_muted)));
        }
    } else {
        let max_str = format_duration_clean(max_dur);
        header_spans.push(Span::styled(format!("(max: {}) · ", max_str), Style::default().fg(theme.text_muted)));
        header_spans.push(Span::styled("●", Style::default().fg(theme.green)));
        header_spans.push(Span::styled(" ok  ", Style::default().fg(theme.text_muted)));
        header_spans.push(Span::styled("●", Style::default().fg(theme.red)));
        header_spans.push(Span::styled(" err  ", Style::default().fg(theme.text_muted)));
        header_spans.push(Span::styled("●", Style::default().fg(theme.text_muted)));
        header_spans.push(Span::styled(" skip", Style::default().fg(theme.text_muted)));
    }

    let mut lines = Vec::with_capacity(height);
    lines.push(Line::from(header_spans));

    // --- Rows 1..=chart_height : 2D Bar Columns ---
    for row in 0..chart_height {
        let row_from_bottom = chart_height - 1 - row;
        let low_thresh = row_from_bottom * 8;
        let high_thresh = (row_from_bottom + 1) * 8;

        let mut row_spans = Vec::new();
        row_spans.push(Span::styled(" ".repeat(margin_left), Style::default()));

        for (i, (orig_idx, run)) in runs_slice.iter().enumerate() {
            let dur = durations[i];
            let level = if dur > 0.0 {
                (((dur / max_dur) * (total_levels as f64)).ceil() as usize).clamp(1, total_levels)
            } else {
                1
            };

            let block_char = if level >= high_thresh {
                '█'
            } else if level <= low_thresh {
                ' '
            } else {
                let frac = level - low_thresh;
                BLOCKS[frac.clamp(0, 7)]
            };

            let is_sel = selected_idx == Some(*orig_idx);
            let color = match run.status {
                RunStatus::Success => {
                    if row_from_bottom == 0 {
                        theme.green
                    } else if row_from_bottom == 1 {
                        theme.cyan
                    } else if row_from_bottom == 2 {
                        theme.yellow
                    } else {
                        theme.orange
                    }
                }
                RunStatus::Failed => theme.red,
                RunStatus::Skipped => theme.text_muted,
                RunStatus::Running => theme.highlight,
            };

            let mut style = Style::default().fg(color);
            if is_sel {
                style = style.add_modifier(Modifier::BOLD);
                if block_char == ' ' && row_from_bottom == 0 {
                    row_spans.push(Span::styled("·".repeat(col_width), Style::default().fg(theme.highlight)));
                    row_spans.push(Span::styled(" ".repeat(gap), Style::default()));
                    continue;
                }
            }

            let s = block_char.to_string().repeat(col_width);
            row_spans.push(Span::styled(s, style));
            row_spans.push(Span::styled(" ".repeat(gap), Style::default()));
        }

        lines.push(Line::from(row_spans));
    }

    (lines, hitboxes_coords)
}

fn format_duration_clean(sec: f64) -> String {
    if sec < 1.0 {
        "<1s".to_string()
    } else if sec < 60.0 {
        format!("{:.0}s", sec)
    } else {
        let mins = (sec / 60.0).floor() as u64;
        let rem_sec = (sec % 60.0).round() as u64;
        if rem_sec > 0 {
            format!("{}m{:02}s", mins, rem_sec)
        } else {
            format!("{}m", mins)
        }
    }
}

/// Render btop++ style horizontal gradient progress bar: [████████░░░░░░]
pub fn render_gradient_bar(
    pct: f64,
    width: usize,
    theme: &ThemePalette,
) -> Vec<Span<'static>> {
    if width == 0 {
        return vec![];
    }

    let clamped_pct = pct.clamp(0.0, 100.0);
    let filled_slots = ((clamped_pct / 100.0) * width as f64).round() as usize;

    let mut spans = Vec::new();
    spans.push(Span::styled("[", Style::default().fg(theme.border)));

    for i in 0..width {
        if i < filled_slots {
            // Calcul du dégradé selon position relative
            let ratio = i as f64 / width as f64;
            let color = if ratio < 0.5 {
                theme.cyan
            } else if ratio < 0.75 {
                theme.yellow
            } else if ratio < 0.90 {
                theme.orange
            } else {
                theme.red
            };
            spans.push(Span::styled("■", Style::default().fg(color).add_modifier(Modifier::BOLD)));
        } else {
            spans.push(Span::styled("·", Style::default().fg(theme.separator)));
        }
    }

    spans.push(Span::styled("]", Style::default().fg(theme.border)));
    spans
}

/// Parse duration string (handles "4m39.9s", "10m59s", "45s", "1h20m10s", "1.5s", "<1s")
pub fn parse_duration_seconds(dur: &str) -> f64 {
    let d = dur.trim();
    if d.is_empty() || d == "--" {
        return 0.0;
    }
    if d.starts_with('<') {
        return 0.5;
    }

    let mut total = 0.0;
    let mut num_buf = String::new();

    for c in d.chars() {
        if c.is_ascii_digit() || c == '.' {
            num_buf.push(c);
        } else if c == 'h' || c == 'H' {
            if let Ok(val) = num_buf.parse::<f64>() {
                total += val * 3600.0;
            }
            num_buf.clear();
        } else if c == 'm' || c == 'M' {
            if let Ok(val) = num_buf.parse::<f64>() {
                total += val * 60.0;
            }
            num_buf.clear();
        } else if c == 's' || c == 'S' {
            if let Ok(val) = num_buf.parse::<f64>() {
                total += val;
            }
            num_buf.clear();
        }
    }
    if !num_buf.is_empty() {
        if let Ok(val) = num_buf.parse::<f64>() {
            total += val;
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration_seconds() {
        assert_eq!(parse_duration_seconds("4m39.9s"), 279.9);
        assert_eq!(parse_duration_seconds("10m59.9s"), 659.9);
        assert_eq!(parse_duration_seconds("45s"), 45.0);
        assert_eq!(parse_duration_seconds("<1s"), 0.5);
        assert_eq!(parse_duration_seconds("1h 20m 10s"), 4810.0);
        assert_eq!(parse_duration_seconds("--"), 0.0);
        assert_eq!(parse_duration_seconds(""), 0.0);
    }

    #[test]
    fn test_render_history_graph_multiline() {
        use crate::monitor::history::{PastRun, RunStatus};
        use crate::ui::theme::ThemeChoice;
        let theme = ThemeChoice::TokyoNight.palette();
        let runs = vec![
            PastRun {
                id: 1,
                date: "2026-09-18".into(),
                time: "08:00".into(),
                duration: "10s".into(),
                status: RunStatus::Success,
                summary: "".into(),
                files_copied: vec!["a.txt".into()],
                files_modified: vec![],
                files_deleted: vec![],
                synced_files: vec![],
                errors: vec![],
            },
            PastRun {
                id: 2,
                date: "2026-09-18".into(),
                time: "08:15".into(),
                duration: "20s".into(),
                status: RunStatus::Failed,
                summary: "".into(),
                files_copied: vec![],
                files_modified: vec![],
                files_deleted: vec![],
                synced_files: vec![],
                errors: vec!["error".into()],
            },
        ];

        let (lines, hitboxes) = render_history_graph_multiline(&runs, &theme, 60, 5, Some(0));
        assert_eq!(lines.len(), 5); // 1 header line + 4 chart rows
        assert_eq!(hitboxes.len(), 2);
    }
}
