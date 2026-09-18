#![allow(dead_code)]

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use crate::app::{App, HitAction, Hitbox};
use crate::monitor::history::RunStatus;
use crate::ui::theme::ThemePalette;

pub fn render_history(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Percentage(55),
        ])
        .split(area);

    render_run_list(f, app, theme, chunks[0]);
    render_run_details(f, app, app.selected_run_idx, theme, chunks[1], &mut Vec::new());
}

fn render_run_list(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect) {
    let items: Vec<ListItem> = if app.past_runs.is_empty() {
        vec![ListItem::new(Line::from(vec![
            Span::styled(" Aucune exécution enregistrée dans journalctl", Style::default().fg(theme.text_muted)),
        ]))]
    } else {
        app.past_runs
            .iter()
            .enumerate()
            .map(|(i, run)| {
                let is_selected = i == app.selected_run_idx;

                let (status_badge, status_color) = match run.status {
                    RunStatus::Success => ("✔", theme.green),
                    RunStatus::Failed => ("✗", theme.red),
                    RunStatus::Skipped => ("⊘", theme.text_muted),
                    RunStatus::Running => ("▶", theme.accent),
                };

                let cursor = if is_selected {
                    Span::styled("▶ ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))
                } else {
                    Span::styled("  ", Style::default())
                };

                let title_style = if is_selected {
                    Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text_bright)
                };

                let line1 = Line::from(vec![
                    cursor,
                    Span::styled(format!("{} ", status_badge), Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("{} {} ", run.date, run.time), title_style),
                    Span::styled(format!("({}) ", run.duration), Style::default().fg(theme.text_muted)),
                ]);

                let line2 = Line::from(vec![
                    Span::styled("     ", Style::default()),
                    Span::styled(&run.summary, Style::default().fg(if is_selected { theme.cyan } else { theme.text_muted })),
                ]);

                ListItem::new(vec![line1, line2])
            })
            .collect()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.card_bg))
                .title(Span::styled(" Historique des synchronisations ", Style::default().fg(theme.accent))),
        );

    f.render_widget(list, area);

    // Scrollbar latérale
    if app.past_runs.len() > area.height.saturating_sub(3) as usize {
        let mut scrollbar_state = ScrollbarState::new(app.past_runs.len())
            .position(app.selected_run_idx);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_style(Style::default().fg(theme.border))
            .thumb_style(Style::default().fg(theme.accent));
        f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}

pub fn render_history_details_modal(f: &mut Frame, app: &App, run_idx: usize, theme: &ThemePalette, hitboxes: &mut Vec<Hitbox>) {
    let area = centered_rect(82, 78, f.area());
    f.render_widget(ratatui::widgets::Clear, area);
    render_run_details(f, app, run_idx, theme, area, hitboxes);
}

pub fn render_run_details(f: &mut Frame, app: &App, run_idx: usize, theme: &ThemePalette, area: Rect, hitboxes: &mut Vec<Hitbox>) {
    if app.past_runs.is_empty() || run_idx >= app.past_runs.len() {
        let p = Paragraph::new("Aucune information disponible.")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.border))
                    .style(Style::default().bg(theme.card_bg))
                    .title(" Détails du Run "),
            );
        f.render_widget(p, area);
        return;
    }

    let run = &app.past_runs[run_idx];
    let mut all_lines: Vec<(Line, Option<usize>)> = Vec::new(); // (line, option<file_idx>)

    // En-tête du run
    all_lines.push((Line::from(vec![
        Span::styled(" Synchronisation #", Style::default().fg(theme.text_muted)),
        Span::styled(format!("{} ", run.id), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled(" du ", Style::default().fg(theme.text_muted)),
        Span::styled(format!("{} à {} ", run.date, run.time), Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)),
        Span::styled("│ Durée : ", Style::default().fg(theme.text_muted)),
        Span::styled(format!("{} ", run.duration), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
    ]), None));

    let (status_str, status_color) = match run.status {
        RunStatus::Success => ("✔ RÉUSSIE", theme.green),
        RunStatus::Failed => ("✗ EN ERREUR", theme.red),
        RunStatus::Skipped => ("⊘ IGNORÉE (Aucun changement)", theme.text_muted),
        RunStatus::Running => ("▶ EN COURS", theme.cyan),
    };

    all_lines.push((Line::from(vec![
        Span::styled(" Statut : ", Style::default().fg(theme.text_muted)),
        Span::styled(status_str, Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
    ]), None));

    all_lines.push((Line::from(Span::styled("─".repeat(area.width.saturating_sub(4) as usize), Style::default().fg(theme.border))), None));

    // Erreurs
    if !run.errors.is_empty() {
        all_lines.push((Line::from(Span::styled(
            format!(" 🚨 ERREURS DÉTECTÉES ({}) :", run.errors.len()),
            Style::default().fg(theme.red).add_modifier(Modifier::BOLD),
        )), None));
        for err in &run.errors {
            all_lines.push((Line::from(Span::styled(format!("   • {}", err), Style::default().fg(theme.red))), None));
        }
        all_lines.push((Line::from(Span::styled("─".repeat(area.width.saturating_sub(4) as usize), Style::default().fg(theme.border))), None));
    }

    let affected_files = run.all_affected_files();
    if !affected_files.is_empty() {
        all_lines.push((Line::from(vec![
            Span::styled(format!(" 📁 FICHIERS AFFECTÉS ({}) :", affected_files.len()), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("  (Enter: ouvrir fichier, Ctrl+Enter / d: ouvrir dossier)", Style::default().fg(theme.text_muted)),
        ]), None));

        for (idx, (action, path)) in affected_files.iter().enumerate() {
            let is_selected = idx == app.history_selected_file_idx;
            let (badge_icon, badge_color) = match *action {
                "new" => ("[+] Copié", theme.green),
                "deleted" => ("[-] Suppr", theme.red),
                _ => ("[~] Modif", theme.yellow),
            };

            let prefix = if is_selected { " ▶ " } else { "   " };
            let row_style = if is_selected {
                Style::default().fg(theme.text_bright).bg(theme.border_focus).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text_bright)
            };

            all_lines.push((Line::from(vec![
                Span::styled(prefix, Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" {:<9} ", badge_icon), Style::default().fg(badge_color).add_modifier(Modifier::BOLD)),
                Span::styled(*path, row_style),
            ]), Some(idx)));
        }
    } else if run.errors.is_empty() {
        all_lines.push((Line::from(Span::styled(
            "   Aucun fichier n'a été modifié durant ce cycle de synchronisation.",
            Style::default().fg(theme.text_muted),
        )), None));
    }

    let visible_height = area.height.saturating_sub(5) as usize;
    let total_lines = all_lines.len();
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll = app.history_details_scroll.min(max_scroll);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(theme.border_history))
        .style(Style::default().bg(theme.card_bg))
        .title(Line::from(vec![
            Span::styled("┌🔍 détails run", Style::default().fg(theme.blue).add_modifier(Modifier::BOLD)),
            Span::styled(format!(": #{} ({} {})┐", run.id, run.date, run.time), Style::default().fg(theme.text_muted)),
            Span::styled("┌fermer: Esc┐", Style::default().fg(theme.text_muted)),
        ]))
        .title_bottom(
            Line::from(vec![
                Span::styled("↑/↓ défiler  ↵ ouvrir  d dossier  Esc fermer ", Style::default().fg(theme.text_muted)),
                Span::styled(format!("─ ligne {}/{}┘", scroll + 1, total_lines.max(1)), Style::default().fg(theme.border_history).add_modifier(Modifier::BOLD)),
            ])
            .alignment(Alignment::Right),
        );

    let inner = block.inner(area);
    f.render_widget(block, area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

    // Enregistrer les hitboxes pour les fichiers visibles
    let display_items: Vec<&(Line, Option<usize>)> = all_lines
        .iter()
        .skip(scroll)
        .take(visible_height)
        .collect();

    for (row_i, (_, file_opt)) in display_items.iter().enumerate() {
        if let Some(file_idx) = file_opt {
            let row_y = layout[0].y + row_i as u16;
            if row_y < layout[0].y + layout[0].height {
                hitboxes.push(Hitbox {
                    rect: Rect {
                        x: layout[0].x,
                        y: row_y,
                        width: layout[0].width,
                        height: 1,
                    },
                    action: HitAction::HistoryFile(*file_idx),
                });
            }
        }
    }

    let display_lines: Vec<Line> = display_items.iter().map(|(l, _)| l.clone()).collect();
    let p = Paragraph::new(display_lines);
    f.render_widget(p, layout[0]);

    let footer_line = Line::from(vec![
        Span::styled("[Enter] ", Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD)),
        Span::styled("Ouvrir fichier  │  ", Style::default().fg(theme.text_muted)),
        Span::styled("[Ctrl+Enter / d] ", Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD)),
        Span::styled("Dossier parent  │  ", Style::default().fg(theme.text_muted)),
        Span::styled("[↑↓] ", Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD)),
        Span::styled("Sélectionner  │  ", Style::default().fg(theme.text_muted)),
        Span::styled("[Esc] ", Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD)),
        Span::styled("Fermer", Style::default().fg(theme.text_muted)),
    ]);
    let footer_p = Paragraph::new(footer_line).alignment(ratatui::layout::Alignment::Center);
    f.render_widget(footer_p, layout[1]);

    if total_lines > visible_height {
        let mut scrollbar_state = ScrollbarState::new(total_lines).position(scroll);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_style(Style::default().fg(theme.border))
            .thumb_style(Style::default().fg(theme.accent));
        f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
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
