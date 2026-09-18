use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, HitAction, Hitbox};
use crate::config;
use crate::ui::theme::ThemePalette;

pub fn render_filters_modal(f: &mut Frame, app: &App, theme: &ThemePalette, hitboxes: &mut Vec<Hitbox>) {
    let area = centered_rect(80, 72, f.area());
    f.render_widget(Clear, area);

    let filepath = config::filters_file().display().to_string();
    let total_rules = app.filters.len();
    let cur_rule = if total_rules > 0 { app.selected_filter_idx + 1 } else { 0 };

    let (up_col, down_col) = if total_rules <= 1 {
        (theme.text_muted, theme.text_muted)
    } else if app.selected_filter_idx == 0 {
        (theme.text_muted, theme.red)
    } else if app.selected_filter_idx >= total_rules.saturating_sub(1) {
        (theme.red, theme.text_muted)
    } else {
        (theme.red, theme.red)
    };

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(theme.border_storage))
        .style(Style::default().bg(theme.card_bg))
        .title(Line::from(vec![
            Span::styled("┌⊘ filtres d'exclusion", Style::default().fg(theme.purple).add_modifier(Modifier::BOLD)),
            Span::styled(format!(": {}┐", filepath), Style::default().fg(theme.text_muted)),
            Span::styled("┌éditer: e┐", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)),
            Span::styled("┌fermer: Esc┐", Style::default().fg(theme.text_muted)),
        ]))
        .title_bottom(
            Line::from(vec![
                Span::styled("↑", Style::default().fg(up_col).add_modifier(Modifier::BOLD)),
                Span::styled("/", Style::default().fg(theme.text_muted)),
                Span::styled("↓", Style::default().fg(down_col).add_modifier(Modifier::BOLD)),
                Span::styled(" naviguer  ", Style::default().fg(theme.text_muted)),
                Span::styled("e", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
                Span::styled(" éditer  ", Style::default().fg(theme.text_muted)),
                Span::styled("Esc", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
                Span::styled(" fermer ", Style::default().fg(theme.text_muted)),
                Span::styled(format!("─ {}/{} ", cur_rule, total_rules), Style::default().fg(theme.border_storage).add_modifier(Modifier::BOLD)),
            ])
            .alignment(Alignment::Right),
        );
    f.render_widget(outer_block, area);

    let inner_area = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(62),
            Constraint::Percentage(38),
        ])
        .split(inner_area);

    render_rules_list(f, app, theme, main_chunks[0], hitboxes);
    render_filters_help(f, app, theme, main_chunks[1], hitboxes);
}

fn render_rules_list(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect, hitboxes: &mut Vec<Hitbox>) {
    let visible_height = area.height as usize;
    if visible_height == 0 {
        return;
    }

    // Hitbox pour toute la zone de liste (pour la molette)
    hitboxes.push(Hitbox {
        rect: area,
        action: HitAction::FilterArea,
    });

    let offset = if app.selected_filter_idx < app.filter_scroll_offset {
        app.selected_filter_idx
    } else if app.selected_filter_idx >= app.filter_scroll_offset + visible_height {
        app.selected_filter_idx.saturating_sub(visible_height) + 1
    } else {
        app.filter_scroll_offset
    };

    let items: Vec<ListItem> = if app.filters.is_empty() {
        vec![ListItem::new(Line::from(vec![
            Span::styled(" Aucun fichier gdrive-filters.txt trouvé ou fichier vide.", Style::default().fg(theme.text_muted)),
        ]))]
    } else {
        app.filters
            .iter()
            .enumerate()
            .skip(offset)
            .take(visible_height)
            .map(|(i, rule)| {
                let trimmed = rule.trim();
                let is_selected = i == app.selected_filter_idx;

                let (color, prefix) = if trimmed.starts_with('+') {
                    (theme.green, "[INCLUDE]")
                } else if trimmed.starts_with('-') {
                    (theme.red, "[EXCLUDE]")
                } else if trimmed.starts_with('#') {
                    (theme.text_muted, "[COMMENT]")
                } else {
                    (theme.cyan, "[RULE]   ")
                };

                if is_selected {
                    let highlight_bg = Color::Rgb(90, 32, 32);
                    let line = Line::from(vec![
                        Span::styled("▶ ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                        Span::styled(format!("{:2} │ ", i + 1), Style::default().fg(Color::White)),
                        Span::styled(format!("{} ", prefix), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                        Span::styled(rule, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    ]);
                    // Enregistrer la hitbox de la ligne
                    let row_y = area.y + (i.saturating_sub(offset)) as u16;
                    if row_y < area.y + area.height {
                        hitboxes.push(Hitbox {
                            rect: Rect {
                                x: area.x,
                                y: row_y,
                                width: area.width,
                                height: 1,
                            },
                            action: HitAction::FilterRow(i),
                        });
                    }
                    ListItem::new(line).style(Style::default().bg(highlight_bg))
                } else {
                    let line = Line::from(vec![
                        Span::styled("  ", Style::default()),
                        Span::styled(format!("{:2} │ ", i + 1), Style::default().fg(theme.text_muted)),
                        Span::styled(format!("{} ", prefix), Style::default().fg(color).add_modifier(Modifier::BOLD)),
                        Span::styled(rule, Style::default().fg(theme.text_muted)),
                    ]);
                    // Enregistrer la hitbox de la ligne
                    let row_y = area.y + (i.saturating_sub(offset)) as u16;
                    if row_y < area.y + area.height {
                        hitboxes.push(Hitbox {
                            rect: Rect {
                                x: area.x,
                                y: row_y,
                                width: area.width,
                                height: 1,
                            },
                            action: HitAction::FilterRow(i),
                        });
                    }
                    ListItem::new(line)
                }
            })
            .collect()
    };

    let list = List::new(items).block(Block::default().borders(Borders::NONE));
    f.render_widget(list, area);

    crate::ui::render_btop_scrollbar(f, area, app.filters.len(), app.selected_filter_idx, visible_height, theme);
}

fn render_filters_help(f: &mut Frame, _app: &App, theme: &ThemePalette, area: Rect, hitboxes: &mut Vec<Hitbox>) {
    let text = vec![
        Line::from(Span::styled("┌syntaxe des filtres┐", Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  - /mon_dossier/**  ", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
            Span::styled("Exclut dossier", Style::default().fg(theme.text_muted)),
        ]),
        Line::from(vec![
            Span::styled("  - *.tmp            ", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
            Span::styled("Exclut extension", Style::default().fg(theme.text_muted)),
        ]),
        Line::from(vec![
            Span::styled("  + *.pdf            ", Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
            Span::styled("Force inclusion", Style::default().fg(theme.text_muted)),
        ]),
        Line::from(vec![
            Span::styled("  # commentaire      ", Style::default().fg(theme.text_muted)),
            Span::styled("Ligne ignorée", Style::default().fg(theme.text_muted)),
        ]),
        Line::from(""),
        Line::from(Span::styled("─".repeat(area.width.saturating_sub(4) as usize), Style::default().fg(theme.border))),
        Line::from(""),
        Line::from(vec![
            Span::styled("Appuyez sur ", Style::default().fg(theme.text_bright)),
            Span::styled("e", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)),
            Span::styled(" pour éditer avec votre", Style::default().fg(theme.text_bright)),
        ]),
        Line::from(Span::styled("éditeur terminal ($EDITOR : nano, nvim, vim).", Style::default().fg(theme.text_muted))),
        Line::from(""),
        Line::from(Span::styled("Prise en compte :", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled("Dès la prochaine exécution bisync.", Style::default().fg(theme.text_muted))),
    ];

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

    let p = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(theme.border)),
    );
    f.render_widget(p, area);

    let close_p = Paragraph::new(Line::from(vec![
        Span::styled(" [ Fermer (Échap) ] ", Style::default().fg(theme.text_bright).bg(theme.border)),
    ])).alignment(Alignment::Center);
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
