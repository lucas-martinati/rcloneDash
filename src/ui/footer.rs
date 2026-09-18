use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

use crate::app::App;
use crate::ui::theme::ThemePalette;

pub fn render_footer(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(65),    // Raccourcis btop++
            Constraint::Length(30), // Panel actif + Tick rate
        ])
        .split(area);

    // 1. Barre de raccourcis btop++ authentique (clé colorée + action en texte clair)
    let mut spans: Vec<Span> = Vec::new();

    let items = [
        ("Esc", "menu", theme.highlight),
        ("q", "quit", theme.red),
        ("s", "sync", theme.green),
        ("d", "dry-run", theme.cyan),
        ("f", "files", theme.blue),
        ("e", "filtres", theme.purple),
        ("o", "options", theme.yellow),
        ("t", "theme", theme.accent),
        ("Tab", "panel", theme.highlight),
        ("+/-", "speed", theme.highlight),
        ("?", "aide", theme.text_muted),
    ];

    for (i, (key, label, color)) in items.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", Style::default()));
        }
        spans.push(Span::styled(*key, Style::default().fg(*color).add_modifier(Modifier::BOLD)));
        spans.push(Span::styled(format!(" {}", label), Style::default().fg(theme.text_bright)));
    }

    let p_left = Paragraph::new(Line::from(spans))
        .alignment(Alignment::Left)
        .block(Block::default().style(Style::default().bg(theme.footer_bg)));
    f.render_widget(p_left, chunks[0]);

    // 2. Info panel actif et rafraîchissement
    let right_spans = vec![
        Span::styled("Focus: ", Style::default().fg(theme.text_muted)),
        Span::styled(
            format!("{} ", app.focused_panel.label()),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│ ", Style::default().fg(theme.border)),
        Span::styled("⚡ ", Style::default().fg(theme.yellow)),
        Span::styled(
            format!("{}ms ", app.tick_rate_ms_live),
            Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD),
        ),
    ];

    let p_right = Paragraph::new(Line::from(right_spans))
        .alignment(Alignment::Right)
        .block(Block::default().style(Style::default().bg(theme.footer_bg)));
    f.render_widget(p_right, chunks[1]);
}
