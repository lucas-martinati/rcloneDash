use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::{App, HitAction, Hitbox, Modal};
use crate::ui::files::render_files_modal;
use crate::ui::filters::render_filters_modal;
use crate::ui::history::render_history_details_modal;
use crate::ui::menu::render_menu_modal;
use crate::ui::settings::render_settings_modal;
use crate::ui::theme::ThemePalette;

pub fn render_popups(f: &mut Frame, app: &App, theme: &ThemePalette, hitboxes: &mut Vec<Hitbox>) {
    // 1. Toast notification flottant
    if let Some((msg, _)) = &app.toast {
        let area = f.area();
        let toast_width = (msg.len() as u16 + 6).min(area.width.saturating_sub(4));
        let toast_area = Rect {
            x: area.width.saturating_sub(toast_width + 2),
            y: 1,
            width: toast_width,
            height: 3,
        };

        f.render_widget(Clear, toast_area);
        let toast_p = Paragraph::new(Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled(msg, Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.accent))
                .style(Style::default().bg(theme.card_bg)),
        );
        f.render_widget(toast_p, toast_area);
    }

    // 2. Modales
    match &app.modal {
        Modal::Menu => {
            render_menu_modal(f, app, theme, hitboxes);
        }
        Modal::Settings => {
            render_settings_modal(f, app, theme, hitboxes);
        }
        Modal::Files => {
            render_files_modal(f, app, theme, hitboxes);
        }
        Modal::Filters => {
            render_filters_modal(f, app, theme, hitboxes);
        }
        Modal::HistoryDetails(idx) => {
            render_history_details_modal(f, app, *idx, theme, hitboxes);
        }
        Modal::DryRun => {
            render_dry_run_modal(f, app, theme, hitboxes);
        }
        Modal::ConfirmResync => {
            let area = centered_rect(60, 30, f.area());
            f.render_widget(Clear, area);

            let text = vec![
                Line::from(Span::styled("ATTENTION : RESYNCHRONISATION COMPLÈTE", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD))),
                Line::from(""),
                Line::from("Cette opération va reconstruire les index locaux et distants"),
                Line::from("avec l'option --resync pour résoudre un conflit ou une incohérence."),
                Line::from(""),
                Line::from(Span::styled("Voulez-vous lancer la commande maintenant ?", Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD))),
                Line::from(""),
                Line::from(vec![
                    Span::styled(" [O] Oui / Confirmer ", Style::default().fg(theme.card_bg).bg(theme.red).add_modifier(Modifier::BOLD)),
                    Span::styled("    ", Style::default()),
                    Span::styled(" [N] Non / Annuler (Échap) ", Style::default().fg(theme.text_bright).bg(theme.border)),
                ]),
            ];

            let p = Paragraph::new(text)
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(theme.yellow))
                        .style(Style::default().bg(theme.card_bg))
                        .title(Span::styled(" Confirmation requise ", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD))),
                );
            f.render_widget(p, area);
        }
        Modal::ConfirmCancel => {
            let area = centered_rect(55, 25, f.area());
            f.render_widget(Clear, area);

            let text = vec![
                Line::from(Span::styled("INTERROMPRE LA SYNCHRONISATION ?", Style::default().fg(theme.red).add_modifier(Modifier::BOLD))),
                Line::from(""),
                Line::from("Le processus rclone-bisync en cours sera immédiatement arrêté."),
                Line::from(""),
                Line::from(vec![
                    Span::styled(" [O] Oui, arrêter ", Style::default().fg(theme.card_bg).bg(theme.red).add_modifier(Modifier::BOLD)),
                    Span::styled("    ", Style::default()),
                    Span::styled(" [N] Continuer (Échap) ", Style::default().fg(theme.text_bright).bg(theme.border)),
                ]),
            ];

            let p = Paragraph::new(text)
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(theme.red))
                        .style(Style::default().bg(theme.card_bg))
                        .title(Span::styled(" Arrêt forcé ", Style::default().fg(theme.red).add_modifier(Modifier::BOLD))),
                );
            f.render_widget(p, area);
        }
        Modal::ConfirmDelete(rel_path) => {
            let area = centered_rect(55, 25, f.area());
            f.render_widget(Clear, area);

            let text = vec![
                Line::from(Span::styled("SUPPRIMER CE FICHIER LOCALEMENT ?", Style::default().fg(theme.red).add_modifier(Modifier::BOLD))),
                Line::from(""),
                Line::from(Span::styled(rel_path.as_str(), Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD))),
                Line::from(""),
                Line::from(vec![
                    Span::styled(" [O] Supprimer ", Style::default().fg(theme.card_bg).bg(theme.red).add_modifier(Modifier::BOLD)),
                    Span::styled("    ", Style::default()),
                    Span::styled(" [N] Annuler (Échap) ", Style::default().fg(theme.text_bright).bg(theme.border)),
                ]),
            ];

            let p = Paragraph::new(text)
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(theme.red))
                        .style(Style::default().bg(theme.card_bg))
                        .title(Span::styled(" Suppression ", Style::default().fg(theme.red).add_modifier(Modifier::BOLD))),
                );
            f.render_widget(p, area);
        }
        Modal::Help => {
            let area = centered_rect(72, 80, f.area());
            f.render_widget(Clear, area);

            let help_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(16),
                    Constraint::Length(3),
                ])
                .split(area);

            let close_btn_area = Rect {
                x: help_chunks[1].x + 4,
                y: help_chunks[1].y,
                width: help_chunks[1].width.saturating_sub(8),
                height: 2,
            };

            hitboxes.push(Hitbox {
                rect: close_btn_area,
                action: HitAction::CloseModal,
            });

            let text = vec![
                Line::from(Span::styled("Raccourcis clavier & Souris de RcloneDash (btop++ style)", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))),
                Line::from(""),
                shortcut_line("Clic gauche", "Cliquer sur un bouton, un run ou un fichier", theme),
                shortcut_line("Molette", "Défilement fluide des logs et des listes", theme),
                shortcut_line("Esc", "Menu principal btop++ / Fermer la modale active", theme),
                shortcut_line("Tab / Shift+Tab", "Changer de panel actif (Historique / Logs / Récents)", theme),
                shortcut_line("Entrée", "Inspecter les détails du run sélectionné dans l'historique", theme),
                shortcut_line("m", "Ouvrir les Paramètres & Options", theme),
                shortcut_line("f", "Ouvrir l'Explorateur de fichiers local", theme),
                shortcut_line("e", "Ouvrir l'Éditeur de règles gdrive-filters.txt", theme),
                shortcut_line("t", "Changer de thème visuel (6 thèmes btop++)", theme),
                shortcut_line("+ / -", "Ajuster la vitesse de rafraîchissement (100ms - 2000ms)", theme),
                shortcut_line("s", "Déclencher une synchronisation forcée", theme),
                shortcut_line("r", "Lancer une resynchronisation complète (--resync)", theme),
                shortcut_line("c", "Arrêter la synchronisation en cours", theme),
                shortcut_line("Espace", "Pause / reprise du défilement automatique des logs", theme),
                shortcut_line("? ou h", "Afficher ou fermer cette aide", theme),
                shortcut_line("q / Ctrl+C", "Quitter l'application silencieusement", theme),
                Line::from(""),
            ];

            let p = Paragraph::new(text)
                .alignment(Alignment::Left)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(theme.accent))
                        .style(Style::default().bg(theme.card_bg))
                        .title(Span::styled(" ❓ Aide & Raccourcis ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))),
                );
            f.render_widget(p, area);

            let close_p = Paragraph::new(Line::from(vec![
                Span::styled(" [ Fermer l'aide (Échap) ] ", Style::default().fg(theme.text_bright).bg(theme.border)),
            ])).alignment(Alignment::Center);
            f.render_widget(close_p, close_btn_area);
        }
        Modal::None => {}
    }
}

fn shortcut_line<'a>(keys: &'a str, desc: &'a str, theme: &ThemePalette) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("  {:14}", keys), Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)),
        Span::styled(" : ", Style::default().fg(theme.border)),
        Span::styled(desc, Style::default().fg(theme.text_bright)),
    ])
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

fn render_dry_run_modal(f: &mut Frame, app: &App, theme: &ThemePalette, hitboxes: &mut Vec<Hitbox>) {
    let area = centered_rect(82, 80, f.area());
    f.render_widget(Clear, area);

    let status_str = if app.dry_run_running {
        "⏳ Simulation en cours (rclone bisync --dry-run)..."
    } else {
        "✔ Simulation terminée"
    };

    let total_lines = app.dry_run_logs.len();
    let visible_height = area.height.saturating_sub(5) as usize;
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll = app.dry_run_scroll.min(max_scroll);

    let lines: Vec<Line> = if app.dry_run_logs.is_empty() {
        vec![Line::from(Span::styled("Initialisation de la simulation dry-run...", Style::default().fg(theme.text_muted)))]
    } else {
        app.dry_run_logs
            .iter()
            .skip(scroll)
            .take(visible_height)
            .map(|l| crate::ui::logs::colorize_log_line(l, theme))
            .collect()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.cyan))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled(format!(" 🛡 Simulation Dry-Run │ {} ", status_str), Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD)));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let p = Paragraph::new(lines);
    f.render_widget(p, chunks[0]);

    let close_btn_area = Rect {
        x: chunks[1].x + chunks[1].width.saturating_sub(15),
        y: chunks[1].y,
        width: 14,
        height: 1,
    };
    hitboxes.push(Hitbox {
        rect: close_btn_area,
        action: HitAction::CloseModal,
    });

    let footer_line = Line::from(vec![
        Span::styled("[r] ", Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD)),
        Span::styled("Relancer  │  ", Style::default().fg(theme.text_muted)),
        Span::styled("[↑↓/PgUp/PgDn] ", Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD)),
        Span::styled("Défiler  │  ", Style::default().fg(theme.text_muted)),
        Span::styled("[Esc] ", Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD)),
        Span::styled("Fermer", Style::default().fg(theme.text_muted)),
    ]);
    let footer_p = Paragraph::new(footer_line).alignment(Alignment::Center);
    f.render_widget(footer_p, chunks[1]);

    if total_lines > visible_height {
        let mut scrollbar_state = ratatui::widgets::ScrollbarState::new(total_lines).position(scroll);
        let scrollbar = ratatui::widgets::Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_style(Style::default().fg(theme.border))
            .thumb_style(Style::default().fg(theme.accent));
        f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}
