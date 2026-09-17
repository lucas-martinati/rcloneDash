use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::monitor::history::{PastRun, RunStatus};
use crate::ui::theme::ThemePalette;

const BLOCKS: [char; 8] = [' ', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

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
        let level = if max_val > 0 {
            ((speed_kib as f64 / max_val as f64) * 7.0).round() as usize
        } else {
            0
        };
        let block = BLOCKS[level.min(7)];
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

/// Render duration sparkline of past runs with status colors (like web sparkline.js)
pub fn render_history_sparkline(
    past_runs: &[PastRun],
    theme: &ThemePalette,
    width: usize,
) -> Line<'static> {
    if past_runs.is_empty() {
        return Line::from(vec![Span::styled(" Aucun historique ", Style::default().fg(theme.text_muted))]);
    }

    // Prendre les derniers runs dans l'ordre chronologique
    let runs_slice: Vec<&PastRun> = past_runs.iter().take(width).rev().collect();

    let durations: Vec<f64> = runs_slice.iter().map(|r| parse_duration_seconds(&r.duration)).collect();
    let max_dur = durations.iter().copied().fold(0.0f64, f64::max).max(1.0);

    let mut spans = Vec::new();
    spans.push(Span::styled(" Graphe durées : ", Style::default().fg(theme.text_muted)));

    for (i, run) in runs_slice.iter().enumerate() {
        let dur = durations[i];
        let level = ((dur / max_dur) * 7.0).round() as usize;
        let block = BLOCKS[level.min(7)];

        let color = match run.status {
            RunStatus::Success => theme.green,
            RunStatus::Failed => theme.red,
            RunStatus::Skipped => theme.text_muted,
            RunStatus::Running => theme.cyan,
        };

        spans.push(Span::styled(block.to_string(), Style::default().fg(color).add_modifier(Modifier::BOLD)));
        spans.push(Span::styled(" ", Style::default()));
    }

    Line::from(spans)
}

fn parse_duration_seconds(dur: &str) -> f64 {
    let d = dur.trim();
    if d.is_empty() || d == "--" {
        return 0.0;
    }
    if d.starts_with('<') {
        return 0.5;
    }

    let mut total = 0.0;
    let parts: Vec<&str> = d.split_whitespace().collect();
    for part in parts {
        if part.ends_with('m') {
            if let Ok(m) = part.trim_end_matches('m').parse::<f64>() {
                total += m * 60.0;
            }
        } else if part.ends_with('s') {
            if let Ok(s) = part.trim_end_matches('s').parse::<f64>() {
                total += s;
            }
        } else if let Ok(s) = part.parse::<f64>() {
            total += s;
        }
    }
    total
}
