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
            Constraint::Min(60),    // Raccourcis btop++
            Constraint::Length(28), // Panel actif + Tick rate
        ])
        .split(area);

    // 1. Barre de raccourcis btop++
    let mut spans: Vec<Span> = Vec::new();

    let items = [
        ("Esc", "Menu"),
        ("q", "Quitter"),
        ("s", "Sync"),
        ("f", "Fichiers"),
        ("e", "Filtres"),
        ("t", "Thème"),
        ("Tab", "Panel"),
        ("+/-", "Vitesse"),
        ("?", "Aide"),
    ];

    for (i, (key, label)) in items.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" ", Style::default().bg(theme.footer_bg)));
        }
        // Clé en surbrillance avec badge discret
        spans.push(Span::styled(
            format!(" {} ", key),
            Style::default()
                .fg(theme.highlight)
                .bg(theme.border_focus)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!("{} ", label),
            Style::default().fg(theme.text_bright).bg(theme.footer_bg),
        ));
    }

    let p_left = Paragraph::new(Line::from(spans))
        .alignment(Alignment::Left)
        .block(Block::default().style(Style::default().bg(theme.footer_bg)));
    f.render_widget(p_left, chunks[0]);

    // 2. Info panel actif et rafraîchissement
    let right_spans = vec![
        Span::styled("Focus: ", Style::default().fg(theme.text_muted).bg(theme.footer_bg)),
        Span::styled(
            format!("{} ", app.focused_panel.label()),
            Style::default()
                .fg(theme.accent)
                .bg(theme.footer_bg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│ ", Style::default().fg(theme.border).bg(theme.footer_bg)),
        Span::styled("⚡ ", Style::default().fg(theme.yellow).bg(theme.footer_bg)),
        Span::styled(
            format!("{}ms ", app.tick_rate_ms_live),
            Style::default().fg(theme.text_bright).bg(theme.footer_bg),
        ),
    ];

    let p_right = Paragraph::new(Line::from(right_spans))
        .alignment(Alignment::Right)
        .block(Block::default().style(Style::default().bg(theme.footer_bg)));
    f.render_widget(p_right, chunks[1]);
}
