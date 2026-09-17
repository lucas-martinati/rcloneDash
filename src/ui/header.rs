use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::app::{App, HitAction, Hitbox};
use crate::systemd::ServiceState;
use crate::ui::theme::ThemePalette;

pub fn render_header(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect, hitboxes: &mut Vec<Hitbox>) {
    // Fond global du header
    f.render_widget(Block::default().style(Style::default().bg(theme.header_bg)), area);

    let is_wide = area.width >= 120;
    let right_width = if is_wide { 66 } else { 58 };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(23), // [m]enu + ⚡ RcloneDash
            Constraint::Min(25),    // Statuts, Pulse, Countdown
            Constraint::Length(right_width), // 5 Boutons d'action + Widget Tick Rate btop++
        ])
        .split(area);

    // 1. Gauche : Bouton [m]enu btop++ et Logo RcloneDash
    let left_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(8),  // [m]enu
            Constraint::Length(15), // ⚡ RcloneDash
        ])
        .split(chunks[0]);

    hitboxes.push(Hitbox {
        rect: left_chunks[0],
        action: HitAction::ButtonMenu,
    });

    let menu_btn = Paragraph::new(Line::from(vec![
        Span::styled("[", Style::default().fg(theme.border)),
        Span::styled("m", Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD)),
        Span::styled("]enu", Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.accent))
            .style(Style::default().bg(theme.card_bg)),
    );
    f.render_widget(menu_btn, left_chunks[0]);

    let title_line = Line::from(vec![
        Span::styled(" ⚡ ", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)),
        Span::styled("Rclone", Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)),
        Span::styled("Dash", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
    ]);
    let title_p = Paragraph::new(title_line)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.header_bg)),
        );
    f.render_widget(title_p, left_chunks[1]);

    // 2. Statut & pulse btop++
    let is_active = app.service_info.state == ServiceState::Active || app.live.is_syncing;
    let pulse_cycle = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() / 500) % 2;
    let (status_dot, status_text, status_color) = if is_active {
        let dot = if pulse_cycle == 0 { "●" } else { "◉" };
        (dot, "SYNC ACTIVE", theme.green)
    } else {
        match app.service_info.state {
            ServiceState::Idle => ("●", "EN ATTENTE", theme.cyan),
            ServiceState::Failed => ("●", "ÉCHEC", theme.red),
            _ => ("●", "INCONNU", theme.text_muted),
        }
    };

    let last_sync_str = app.past_runs.first()
        .map(|r| format!("{} ({})", r.time, r.duration))
        .unwrap_or_else(|| "--".to_string());

    let mut status_spans = vec![
        Span::styled(format!(" {} {} ", status_dot, status_text), Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
        Span::styled("│ ", Style::default().fg(theme.border)),
    ];

    if is_active && !app.live.phase.is_empty() {
        status_spans.push(Span::styled("Phase: ", Style::default().fg(theme.text_muted)));
        status_spans.push(Span::styled(format!("{} ", app.live.phase), Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)));
        status_spans.push(Span::styled("│ ", Style::default().fg(theme.border)));
    }

    status_spans.extend(vec![
        Span::styled("Dernière: ", Style::default().fg(theme.text_muted)),
        Span::styled(format!("{} ", last_sync_str), Style::default().fg(theme.text_bright)),
        Span::styled("│ ", Style::default().fg(theme.border)),
        Span::styled("Prochaine: ", Style::default().fg(theme.text_muted)),
        Span::styled(
            format!("{} ", if app.service_info.timer_left.is_empty() { "--" } else { &app.service_info.timer_left }),
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        ),
    ]);

    let status_p = Paragraph::new(Line::from(status_spans))
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.header_bg)),
        );
    f.render_widget(status_p, chunks[1]);

    // 3. Boutons d'action web + Widget Tick Rate btop++
    let right_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(45),   // Boutons d'action
            Constraint::Length(12), // Widget - 250ms +
        ])
        .split(chunks[2]);

    let btn_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 5), // Sync / Arrêter
            Constraint::Ratio(1, 5), // Simuler
            Constraint::Ratio(1, 5), // Fichiers
            Constraint::Ratio(1, 5), // Filtres
            Constraint::Ratio(1, 5), // Paramètres
        ])
        .split(right_chunks[0]);

    // Bouton 1 : Synchroniser OU Arrêter (quand sync active)
    if is_active {
        hitboxes.push(Hitbox { rect: btn_chunks[0], action: HitAction::ButtonCancel });
        render_modern_button(f, btn_chunks[0], "⏹ Arrêter", theme.red, theme);
    } else {
        hitboxes.push(Hitbox { rect: btn_chunks[0], action: HitAction::ButtonSync });
        render_modern_button(f, btn_chunks[0], "⟳ Sync", theme.accent, theme);
    }

    // Bouton 2 : Simuler (Dry Run)
    hitboxes.push(Hitbox { rect: btn_chunks[1], action: HitAction::ButtonDryRun });
    render_modern_button(f, btn_chunks[1], "🛡 Simuler", theme.cyan, theme);

    // Bouton 3 : Fichiers
    hitboxes.push(Hitbox { rect: btn_chunks[2], action: HitAction::ButtonFiles });
    render_modern_button(f, btn_chunks[2], "📁 Fichiers", theme.blue, theme);

    // Bouton 4 : Exclusions / Filtres
    hitboxes.push(Hitbox { rect: btn_chunks[3], action: HitAction::ButtonFilters });
    render_modern_button(f, btn_chunks[3], "⊘ Filtres", theme.purple, theme);

    // Bouton 5 : Paramètres
    hitboxes.push(Hitbox { rect: btn_chunks[4], action: HitAction::ButtonSettings });
    render_modern_button(f, btn_chunks[4], "⚙ Options", theme.yellow, theme);

    // Widget Tick Rate btop++ : [-] 250ms [+]
    render_tick_rate_widget(f, right_chunks[1], app.tick_rate_ms_live, theme, hitboxes);
}

fn render_tick_rate_widget(
    f: &mut Frame,
    area: Rect,
    tick_ms: u64,
    theme: &ThemePalette,
    hitboxes: &mut Vec<Hitbox>,
) {
    let sub = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(3), // [-]
            Constraint::Min(5),    // 250ms
            Constraint::Length(3), // [+]
        ])
        .split(area);

    hitboxes.push(Hitbox { rect: sub[0], action: HitAction::TickRateDec });
    hitboxes.push(Hitbox { rect: sub[2], action: HitAction::TickRateInc });

    let dec_p = Paragraph::new("[-]")
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.card_bg)),
        );
    f.render_widget(dec_p, sub[0]);

    let ms_str = format!("{}ms", tick_ms);
    let ms_p = Paragraph::new(Line::from(vec![
        Span::styled(ms_str, Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::TOP | Borders::BOTTOM)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.card_bg)),
    );
    f.render_widget(ms_p, sub[1]);

    let inc_p = Paragraph::new("[+]")
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.card_bg)),
        );
    f.render_widget(inc_p, sub[2]);
}

fn render_modern_button(f: &mut Frame, area: Rect, text: &str, accent_color: Color, theme: &ThemePalette) {
    let p = Paragraph::new(Line::from(vec![
        Span::styled(format!(" {} ", text), Style::default().fg(accent_color).add_modifier(Modifier::BOLD)),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.card_bg)),
    );
    f.render_widget(p, area);
}
