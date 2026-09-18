use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Clear, Paragraph, Row, Table},
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

    let (up_col, down_col) = if total_files <= 1 {
        (theme.text_muted, theme.text_muted)
    } else if app.file_selected_idx == 0 {
        (theme.text_muted, theme.red)
    } else if app.file_selected_idx >= total_files.saturating_sub(1) {
        (theme.red, theme.text_muted)
    } else {
        (theme.red, theme.red)
    };

    let title_line = Line::from(vec![
        Span::styled("┌📁 explorateur", Style::default().fg(theme.blue).add_modifier(Modifier::BOLD)),
        Span::styled(format!(": {}┐", title_path), Style::default().fg(theme.text_muted)),
        Span::styled("┌ouvrir: ↵┐", Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD)),
        Span::styled("┌fermer: Esc, q┐", Style::default().fg(theme.text_muted)),
    ]);

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(theme.border_history))
        .style(Style::default().bg(theme.card_bg))
        .title(title_line)
        .title_bottom(
            Line::from(vec![
                Span::styled("↑", Style::default().fg(up_col).add_modifier(Modifier::BOLD)),
                Span::styled("/", Style::default().fg(theme.text_muted)),
                Span::styled("↓", Style::default().fg(down_col).add_modifier(Modifier::BOLD)),
                Span::styled(" naviguer  ", Style::default().fg(theme.text_muted)),
                Span::styled("←", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
                Span::styled(" parent  ", Style::default().fg(theme.text_muted)),
                Span::styled("→", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
                Span::styled("/", Style::default().fg(theme.text_muted)),
                Span::styled("↵", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
                Span::styled(" entrer  ", Style::default().fg(theme.text_muted)),
                Span::styled("d", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
                Span::styled(" dossier  ", Style::default().fg(theme.text_muted)),
                Span::styled("Esc, q", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
                Span::styled(" fermer ", Style::default().fg(theme.text_muted)),
                Span::styled(format!("─ {}/{} ─", cur_file, total_files), Style::default().fg(theme.border_history).add_modifier(Modifier::BOLD)),
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

                let display_name = if app.ctrl_mode && !entry.is_dir {
                    let p = std::path::Path::new(&entry.rel_path);
                    let parent = p.parent().and_then(|p| p.to_str()).unwrap_or("");
                    let parent_clean = parent.trim_start_matches('/').trim_end_matches('/');
                    if parent_clean.is_empty() {
                        "📁 ./".to_string()
                    } else {
                        format!("📁 {}/", parent_clean)
                    }
                } else {
                    entry.name.clone()
                };

                let (status_badge, status_color) = if entry.ignored {
                    ("[IGNORÉ]", theme.red)
                } else {
                    ("[SYNC]", theme.green)
                };

                if is_selected {
                    let highlight_bg = Color::Rgb(90, 32, 32);
                    let type_str = if entry.is_dir { "[DIR] " } else { "[FIC] " };
                    Row::new(vec![
                        Cell::from(Line::from(vec![
                            cursor,
                            Span::styled(type_str, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                            Span::styled(display_name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                        ])),
                        Cell::from(Span::styled(entry.size_formatted(), Style::default().fg(Color::White))),
                        Cell::from(Span::styled(&entry.mtime, Style::default().fg(Color::White))),
                        Cell::from(Span::styled(status_badge, Style::default().fg(Color::White).add_modifier(Modifier::BOLD))),
                    ])
                    .style(Style::default().bg(highlight_bg))
                } else {
                    let name_style = if app.ctrl_mode && !entry.is_dir {
                        Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(name_color)
                    };
                    Row::new(vec![
                        Cell::from(Line::from(vec![cursor, type_badge, Span::styled(display_name, name_style)])),
                        Cell::from(Span::styled(entry.size_formatted(), Style::default().fg(theme.text_muted))),
                        Cell::from(Span::styled(&entry.mtime, Style::default().fg(theme.text_muted))),
                        Cell::from(Span::styled(status_badge, Style::default().fg(status_color))),
                    ])
                }
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

    crate::ui::render_btop_scrollbar(
        f,
        area,
        app.file_entries.len(),
        app.file_selected_idx,
        visible_height,
        theme,
    );
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
        Span::styled(" [→ / Entrée] ", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)),
        Span::styled("Entrer / Ouvrir", Style::default().fg(theme.text_bright)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" [← / Backspace] ", Style::default().fg(theme.blue).add_modifier(Modifier::BOLD)),
        Span::styled("Dossier parent", Style::default().fg(theme.text_bright)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" [d] ", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)),
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
        Span::styled(" [Échap / q] ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
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
        Span::styled(" [ Fermer (Échap / q) ] ", Style::default().fg(theme.text_bright).bg(theme.border)),
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
