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
}
