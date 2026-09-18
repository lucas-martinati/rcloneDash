use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::{App, HitAction, Hitbox};
use crate::ui::theme::ThemePalette;

pub const SETTINGS_ITEMS_COUNT: usize = 7;

pub fn render_settings_modal(f: &mut Frame, app: &App, theme: &ThemePalette, hitboxes: &mut Vec<Hitbox>) {
    let area = centered_rect(70, 68, f.area());
    f.render_widget(Clear, area);

    let cur_opt = app.settings_selected_idx + 1;

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(theme.border_sys))
        .style(Style::default().bg(theme.card_bg))
        .title(Line::from(vec![
            Span::styled("┌⚙ options", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)),
            Span::styled("┐", Style::default().fg(theme.border)),
            Span::styled("┌sauvegarder: s / ↵", Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
            Span::styled("┐", Style::default().fg(theme.border)),
            Span::styled("┌fermer: Esc", Style::default().fg(theme.text_muted)),
            Span::styled("┐", Style::default().fg(theme.border)),
        ]))
        .title_bottom(
            Line::from(vec![
                Span::styled("↑/↓", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
                Span::styled(" naviguer  ", Style::default().fg(theme.text_muted)),
                Span::styled("←/→", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
                Span::styled(" modifier  ", Style::default().fg(theme.text_muted)),
                Span::styled("↵", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
                Span::styled(" enregistrer  ", Style::default().fg(theme.text_muted)),
                Span::styled("Esc", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
                Span::styled(" fermer ", Style::default().fg(theme.text_muted)),
                Span::styled(format!("─ {}/{} ", cur_opt, SETTINGS_ITEMS_COUNT), Style::default().fg(theme.border_sys).add_modifier(Modifier::BOLD)),
            ])
            .alignment(Alignment::Right),
        );
    f.render_widget(outer_block, area);

    let inner = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(2),
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Marge haute
            Constraint::Length(8), // Options (7 lignes)
            Constraint::Length(2), // Boutons action (Sauvegarder / Fermer)
            Constraint::Min(3),    // Aide contextuelle
        ])
        .split(inner);

    // 1. Liste des options avec hitboxes
    let mut option_lines = Vec::new();

    let full_sync_display = match app.config.full_sync_interval.as_str() {
        "60" => "1h (Recommandé)".to_string(),
        "120" => "2h".to_string(),
        "240" => "4h".to_string(),
        "360" => "6h".to_string(),
        "720" => "12h".to_string(),
        "1440" => "24h (1 jour)".to_string(),
        "never" => "Jamais (Local)".to_string(),
        other => other.to_string(),
    };

    let settings = [
        ("Thème de l'interface", app.current_theme.name()),
        ("Intervalle timer bisync", &app.config.timer_interval),
        ("Filet de sécurité Cloud", &full_sync_display),
        ("Limite bande passante", app.config.bwlimit.as_deref().unwrap_or("Désactivé")),
        ("Fréquence UI", &format!("{} ms", app.config.tick_rate_ms.unwrap_or(250))),
        ("Dossier local synchronisé", &app.config.local_dir),
        ("Remote rclone distant", &app.config.remote),
    ];

    for (i, (label, val)) in settings.iter().enumerate() {
        let is_selected = i == app.settings_selected_idx;

        let cursor = if is_selected {
            Span::styled(" ▶ ", Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD))
        } else {
            Span::styled("   ", Style::default())
        };

        let label_style = if is_selected {
            Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text_muted)
        };

        let val_style = if is_selected {
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.green)
        };

        option_lines.push(Line::from(vec![
            cursor,
            Span::styled(format!("{:<28}", label), label_style),
            Span::styled(" :  [◀] ", Style::default().fg(if is_selected { theme.highlight } else { theme.border })),
            Span::styled(format!("{:<22}", val), val_style),
            Span::styled(" [▶]", Style::default().fg(if is_selected { theme.highlight } else { theme.border })),
        ]));

        // Enregistrer la hitbox de la ligne
        let row_y = chunks[1].y + i as u16;
        hitboxes.push(Hitbox {
            rect: Rect {
                x: chunks[1].x,
                y: row_y,
                width: chunks[1].width,
                height: 1,
            },
            action: HitAction::SettingOption(i),
        });

        // Hitboxes pour les boutons fléchés spécifiques [◀] et [▶]
        let left_arrow_x = chunks[1].x + 3 + 28 + 4;
        hitboxes.push(Hitbox {
            rect: Rect { x: left_arrow_x, y: row_y, width: 3, height: 1 },
            action: HitAction::SettingCycle(i, false),
        });

        let right_arrow_x = left_arrow_x + 3 + 1 + 22 + 1;
        hitboxes.push(Hitbox {
            rect: Rect { x: right_arrow_x, y: row_y, width: 3, height: 1 },
            action: HitAction::SettingCycle(i, true),
        });
    }

    let p_options = Paragraph::new(option_lines);
    f.render_widget(p_options, chunks[1]);

    // 2. Boutons d'action en bas de modal
    let btn_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(chunks[2]);

    hitboxes.push(Hitbox { rect: btn_chunks[0], action: HitAction::SaveSettings });
    hitboxes.push(Hitbox { rect: btn_chunks[1], action: HitAction::CloseModal });

    let save_p = Paragraph::new(Line::from(vec![
        Span::styled(" [ Enregistrer (s) ] ", Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
    ]))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL).border_type(BorderType::Plain).border_style(Style::default().fg(theme.border_sys)));
    f.render_widget(save_p, btn_chunks[0]);

    let close_p = Paragraph::new(Line::from(vec![
        Span::styled(" [ Fermer (Échap) ] ", Style::default().fg(theme.text_muted)),
    ]))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL).border_type(BorderType::Plain).border_style(Style::default().fg(theme.border)));
    f.render_widget(close_p, btn_chunks[1]);

    // 3. Aide contextuelle
    let help_text = match app.settings_selected_idx {
        0 => "Basculez entre les 6 thèmes modernes (Tokyo Night, Catppuccin Mocha, Nord Frost, Gruvbox Dark, Dracula, Monokai Pro).",
        1 => "Fréquence de vérification du timer rclone-bisync (10min, 15min, 30min, 1h).",
        2 => "Délai max avant une synchronisation complète avec le cloud même sans modifs locales.",
        3 => "Limite de bande passante rclone (bwlimit.env).",
        4 => "Fréquence de boucle TUI en millisecondes (défilement et réactivité).",
        _ => "Cliquez sur [◀] / [▶] ou appuyez sur ← / → pour modifier, Entrée pour enregistrer.",
    };

    let p_help = Paragraph::new(vec![
        Line::from(Span::styled("┌aide contextuelle┐", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(help_text, Style::default().fg(theme.text_bright))),
    ])
    .block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(theme.border)));
    f.render_widget(p_help, chunks[3]);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
