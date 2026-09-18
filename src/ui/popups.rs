use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
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
    // 1. Modales
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
        Modal::ConfirmSync => {
            let area = centered_rect(58, 25, f.area());
            f.render_widget(Clear, area);

            let text = vec![
                Line::from(Span::styled("LANCER LA SYNCHRONISATION MAINTENANT ?", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))),
                Line::from(""),
                Line::from("Cette action déclenche immédiatement le service rclone-bisync"),
                Line::from("pour synchroniser tous les changements locaux et distants."),
                Line::from(""),
                Line::from(vec![
                    Span::styled(" [O / Entrée] Confirmer ", Style::default().fg(theme.card_bg).bg(theme.green).add_modifier(Modifier::BOLD)),
                    Span::styled("    ", Style::default()),
                    Span::styled(" [N / Échap] Annuler ", Style::default().fg(theme.text_bright).bg(theme.border)),
                ]),
            ];

            let p = Paragraph::new(text)
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(theme.accent))
                        .style(Style::default().bg(theme.card_bg))
                        .title(Span::styled(" Synchronisation ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))),
                );
            f.render_widget(p, area);
        }
        Modal::ConfirmDryRun => {
            let area = centered_rect(58, 25, f.area());
            f.render_widget(Clear, area);

            let text = vec![
                Line::from(Span::styled("LANCER UNE SIMULATION DRY-RUN ?", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD))),
                Line::from(""),
                Line::from("Exécute une vérification à blanc (--dry-run)."),
                Line::from("Aucun fichier ne sera copié, modifié ou supprimé."),
                Line::from(""),
                Line::from(vec![
                    Span::styled(" [O / Entrée] Démarrer ", Style::default().fg(theme.card_bg).bg(theme.yellow).add_modifier(Modifier::BOLD)),
                    Span::styled("    ", Style::default()),
                    Span::styled(" [N / Échap] Annuler ", Style::default().fg(theme.text_bright).bg(theme.border)),
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
                        .title(Span::styled(" Simulation Dry-Run ", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD))),
                );
            f.render_widget(p, area);
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
            let screen = f.area();
            let is_wide = screen.width >= 86;
            let logo_h: u16 = if is_wide { 6 } else { 5 };
            let box_w = if is_wide { 86.min(screen.width) } else { 78.min(screen.width) };
            let box_h = 24.min(screen.height.saturating_sub(logo_h + 3));

            let total_h = logo_h + 1 + box_h;
            let container_area = crate::ui::menu::centered_fixed_rect(box_w, total_h, screen);

            let v_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(logo_h),
                    Constraint::Length(1),
                    Constraint::Length(box_h),
                ])
                .split(container_area);

            // 1. Logo 3D RCLONEDASH au-dessus de la boîte d'aide
            crate::ui::menu::render_btop_logo(f, v_chunks[0]);

            // Version
            let ver_line = Line::from(vec![
                Span::styled("v1.0.0", Style::default().fg(Color::Rgb(165, 170, 185)).add_modifier(Modifier::BOLD | Modifier::ITALIC)),
            ]);
            f.render_widget(Paragraph::new(ver_line).alignment(Alignment::Center), v_chunks[1]);

            // 2. Boîte d'aide style btop++
            let help_box_area = v_chunks[2];
            f.render_widget(Clear, help_box_area);

            let help_items = [
                ("Mouse 1", "Clicks buttons and selects in lists/panels."),
                ("Mouse scroll", "Scrolls any scrollable list/logs under cursor."),
                ("Esc, m", "Toggles main menu / Closes active modal."),
                ("q", "Closes active modal / In dashboard: quits app."),
                ("o", "Shows options / settings panel."),
                ("F1, ?, h", "Shows this help window."),
                ("s", "Triggers bisync synchronization (confirmation)."),
                ("d", "Runs bisync dry-run simulation (confirmation)."),
                ("r", "Full resync (--resync) repair mode (confirmation)."),
                ("c", "Cancels active synchronization run."),
                ("b, p", "Opens file browser modal (explorer)."),
                ("f, /", "In Recent files: search filter."),
                ("e", "Opens exclusion rules editor (gdrive-filters.txt)."),
                ("Ctrl+X", "Toggles parent directory mode (shows folder paths)."),
                ("Enter", "Opens selected file / Validates actions."),
                ("Ctrl+Enter", "Opens containing folder in system file manager (xdg)."),
                ("Spacebar", "Pauses / resumes logs auto-scroll."),
                ("y", "Copies focused panel (logs, history errors, or file)."),
                ("+ , -", "Speeds up / slows down UI tick rate interval."),
                ("Tab, Shift+Tab", "Cycles active dashboard panel focus."),
                ("t", "Cycles color theme."),
                ("ctrl + c", "Force terminates the program."),
            ];

            let mut lines = Vec::new();
            // Ligne d'en-tête
            lines.push(Line::from(vec![
                Span::styled("   Key:                 ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::styled("Description:", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]));
            lines.push(Line::from(""));

            for (k, desc) in help_items {
                lines.push(Line::from(vec![
                    Span::styled(format!("   {:20} ", k), Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
                    Span::styled(desc, Style::default().fg(Color::Rgb(215, 220, 230))),
                ]));
            }

            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled("   For bug reporting and project updates, visit:", Style::default().fg(theme.text_muted))));
            lines.push(Line::from(Span::styled("   https://github.com/lucas-martinati/rcloneDash", Style::default().fg(theme.cyan).add_modifier(Modifier::UNDERLINED))));

            let p = Paragraph::new(lines)
                .alignment(Alignment::Left)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(theme.red))
                        .style(Style::default().bg(theme.card_bg))
                        .title(Line::from(vec![
                            Span::styled("┐", Style::default().fg(theme.red)),
                            Span::styled("help", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
                            Span::styled("┌", Style::default().fg(theme.red)),
                        ]))
                        .title(
                            Line::from(vec![
                                Span::styled("┐", Style::default().fg(theme.red)),
                                Span::styled("Esc, q", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
                                Span::styled(" fermer", Style::default().fg(theme.text_bright)),
                                Span::styled("┌", Style::default().fg(theme.red)),
                            ])
                            .alignment(Alignment::Right),
                        ),
                );
            f.render_widget(p, help_box_area);

            hitboxes.push(Hitbox {
                rect: Rect {
                    x: help_box_area.x + help_box_area.width.saturating_sub(14),
                    y: help_box_area.y,
                    width: 12,
                    height: 1,
                },
                action: HitAction::CloseModal,
            });
        }
        Modal::None => {}
    }

    // 2. Toast notification flottant (rendu AU-DESSUS des modales pour être toujours visible)
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
        Span::styled("[Esc / q] ", Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD)),
        Span::styled("Fermer", Style::default().fg(theme.text_muted)),
    ]);
    let footer_p = Paragraph::new(footer_line).alignment(Alignment::Center);
    f.render_widget(footer_p, chunks[1]);

    crate::ui::render_btop_scrollbar(
        f,
        chunks[0],
        total_lines,
        scroll,
        visible_height,
        theme,
        hitboxes,
        crate::app::ScrollbarTarget::DryRun,
    );
}
