use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Clear, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table},
    Frame,
};

use crate::app::{App, HitAction, Hitbox};
use crate::ui::theme::ThemePalette;

pub fn render_files_modal(f: &mut Frame, app: &App, theme: &ThemePalette, hitboxes: &mut Vec<Hitbox>) {
    let area = centered_rect(80, 75, f.area());
    f.render_widget(Clear, area);

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70),
            Constraint::Percentage(30),
        ])
        .split(Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        });

    let title_path = if app.file_current_rel.is_empty() {
        app.config.local_dir.clone()
    } else {
        format!("{}/{}", app.config.local_dir, app.file_current_rel)
    };

    let total_files = app.file_entries.len();
    let cur_file = if total_files > 0 { app.file_selected_idx + 1 } else { 0 };

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(theme.border_history))
        .style(Style::default().bg(theme.card_bg))
        .title(Line::from(vec![
            Span::styled("┌📁 explorateur", Style::default().fg(theme.blue).add_modifier(Modifier::BOLD)),
            Span::styled(format!(": {}┐", title_path), Style::default().fg(theme.text_muted)),
            Span::styled("┌ouvrir: ↵┐", Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD)),
            Span::styled("┌fermer: Esc┐", Style::default().fg(theme.text_muted)),
        ]))
        .title_bottom(
            Line::from(vec![
                Span::styled("↑/↓", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
                Span::styled(" naviguer  ", Style::default().fg(theme.text_muted)),
                Span::styled("↵", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
                Span::styled(" ouvrir  ", Style::default().fg(theme.text_muted)),
                Span::styled("d", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
                Span::styled(" dossier  ", Style::default().fg(theme.text_muted)),
                Span::styled("Esc", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
                Span::styled(" fermer ", Style::default().fg(theme.text_muted)),
                Span::styled(format!("─ {}/{} ", cur_file, total_files), Style::default().fg(theme.border_history).add_modifier(Modifier::BOLD)),
            ])
            .alignment(Alignment::Right),
        );
    f.render_widget(outer_block, area);

    render_file_table(f, app, theme, main_chunks[0], hitboxes);
    render_file_actions(f, app, theme, main_chunks[1], hitboxes);
}

fn render_file_table(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect, hitboxes: &mut Vec<Hitbox>) {
    let visible_height = area.height.saturating_sub(3) as usize;
    let start_idx = app.file_scroll_offset;
    let end_idx = (start_idx + visible_height).min(app.file_entries.len());

    let rows: Vec<Row> = if app.file_entries.is_empty() {
        vec![Row::new(vec![
            Cell::from(Span::styled(" Dossier vide", Style::default().fg(theme.text_muted))),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
        ])]
    } else {
        app.file_entries[start_idx..end_idx]
            .iter()
            .enumerate()
            .map(|(rel_i, entry)| {
                let actual_i = start_idx + rel_i;
                let is_selected = actual_i == app.file_selected_idx;

                let cursor = if is_selected {
                    Span::styled("▶ ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))
                } else {
                    Span::styled("  ", Style::default())
                };

                let (type_badge, name_color) = if entry.is_dir {
                    (Span::styled("[DIR] ", Style::default().fg(theme.blue).add_modifier(Modifier::BOLD)), theme.blue)
                } else {
                    (Span::styled("[FIC] ", Style::default().fg(theme.text_muted)), theme.text_bright)
                };

                let name_style = if is_selected {
                    Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(name_color)
                };

                let (status_badge, status_color) = if entry.ignored {
                    ("[IGNORÉ]", theme.red)
                } else {
                    ("[SYNC]", theme.green)
                };

                Row::new(vec![
                    Cell::from(Line::from(vec![cursor, type_badge, Span::styled(&entry.name, name_style)])),
                    Cell::from(Span::styled(entry.size_formatted(), Style::default().fg(theme.text_muted))),
                    Cell::from(Span::styled(&entry.mtime, Style::default().fg(theme.text_muted))),
                    Cell::from(Span::styled(status_badge, Style::default().fg(status_color))),
                ])
            })
            .collect()
    };

    // Hitboxes pour les lignes de fichiers
    for (rel_i, _) in app.file_entries[start_idx..end_idx].iter().enumerate() {
        let actual_i = start_idx + rel_i;
        let row_y = area.y + 1 + rel_i as u16;
        if row_y < area.y + area.height {
            hitboxes.push(Hitbox {
                rect: Rect {
                    x: area.x,
                    y: row_y,
                    width: area.width,
                    height: 1,
                },
                action: HitAction::FileEntry(actual_i),
            });
        }
    }

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(55),
            Constraint::Percentage(15),
            Constraint::Percentage(20),
            Constraint::Percentage(10),
        ],
    )
    .header(
        Row::new(vec!["Nom du fichier / dossier", "Taille", "Modifié", "Statut"])
            .style(Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
    );

    f.render_widget(table, area);

    if app.file_entries.len() > visible_height {
        let mut scrollbar_state = ScrollbarState::new(app.file_entries.len())
            .position(app.file_selected_idx);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_style(Style::default().fg(theme.border))
            .thumb_style(Style::default().fg(theme.accent));
        f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}

fn render_file_actions(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect, hitboxes: &mut Vec<Hitbox>) {
    let mut lines = Vec::new();

    if let Some(entry) = app.file_entries.get(app.file_selected_idx) {
        lines.push(Line::from(vec![
            Span::styled("Élément sélectionné :", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Nom : ", Style::default().fg(theme.text_muted)),
            Span::styled(&entry.name, Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Type : ", Style::default().fg(theme.text_muted)),
            Span::styled(if entry.is_dir { "Dossier" } else { "Fichier" }, Style::default().fg(theme.cyan)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Taille : ", Style::default().fg(theme.text_muted)),
            Span::styled(entry.size_formatted(), Style::default().fg(theme.text_bright)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Modifié : ", Style::default().fg(theme.text_muted)),
            Span::styled(&entry.mtime, Style::default().fg(theme.text_muted)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Filtre bisync : ", Style::default().fg(theme.text_muted)),
            Span::styled(
                if entry.ignored { "Exclu" } else { "Inclus" },
                Style::default().fg(if entry.ignored { theme.red } else { theme.green }),
            ),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Actions rapides :", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" [Entrée/Double-clic] ", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)),
        Span::styled("Ouvrir / Entrer", Style::default().fg(theme.text_bright)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" [o] ", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)),
        Span::styled("Ouvrir dans l'OS (xdg-open)", Style::default().fg(theme.text_bright)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" [x] ", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)),
        Span::styled("Ajouter règle d'exclusion", Style::default().fg(theme.purple)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" [Suppr] ", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
        Span::styled("Supprimer localement", Style::default().fg(theme.red)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" [Backspace] ", Style::default().fg(theme.blue).add_modifier(Modifier::BOLD)),
        Span::styled("Dossier parent", Style::default().fg(theme.text_bright)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" [Échap] ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
        Span::styled("Fermer fenêtre", Style::default().fg(theme.text_muted)),
    ]));

    // Bouton de fermeture en bas
    let close_btn_area = Rect {
        x: area.x + 2,
        y: area.y + area.height.saturating_sub(3),
        width: area.width.saturating_sub(4),
        height: 2,
    };
    hitboxes.push(Hitbox {
        rect: close_btn_area,
        action: HitAction::CloseModal,
    });

    let p = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(theme.border)),
    );
    f.render_widget(p, area);

    let close_p = Paragraph::new(Line::from(vec![
        Span::styled(" [ Fermer (Échap) ] ", Style::default().fg(theme.text_bright).bg(theme.border)),
    ])).alignment(ratatui::layout::Alignment::Center);
    f.render_widget(close_p, close_btn_area);
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
