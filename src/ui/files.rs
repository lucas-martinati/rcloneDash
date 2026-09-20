use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use crate::app::{App, HitAction, Hitbox};
use crate::ui::container::{centered_rect, render_modal_container, ModalContainerConfig, NavArrowsConfig};
use crate::ui::keys::KeybindingRegistry;
use crate::ui::theme::ThemePalette;

pub fn render_files_modal(f: &mut Frame, app: &App, theme: &ThemePalette, hitboxes: &mut Vec<Hitbox>) {
    let area = centered_rect(80, 75, f.area());

    let title_path = if app.file_current_rel.is_empty() {
        app.config.local_dir.clone()
    } else {
        format!("{}/{}", app.config.local_dir, app.file_current_rel)
    };

    let total_files = app.file_entries.len();
    let cur_file = if total_files > 0 { app.file_selected_idx + 1 } else { 0 };

    let can_up = total_files > 1 && app.file_selected_idx > 0;
    let can_down = total_files > 1 && app.file_selected_idx < total_files.saturating_sub(1);

    let max_title_path_len = (area.width.saturating_sub(42) as usize).max(10);
    let path_spans = crate::ui::theme::truncate_with_fade_spans(
        &title_path,
        max_title_path_len,
        Style::default().fg(theme.text_muted),
        true,
        None,
    );
    let mut title_extra = vec![Span::styled(": ", Style::default().fg(theme.text_muted))];
    title_extra.extend(path_spans);

    let border_color = theme.border_history;

    let actions = vec![
        KeybindingRegistry::format_shortcut_label("←", "parent", theme.red, Color::White),
        KeybindingRegistry::format_shortcut_label("↵", "open", theme.red, Color::White),
    ];

    let inner_area = render_modal_container(
        f,
        app,
        theme,
        area,
        ModalContainerConfig {
            title_prefix: "file explorer",
            title_color: Some(theme.blue),
            title_extra: Some(title_extra),
            nav_arrows: Some(NavArrowsConfig {
                label: "select",
                up_active: can_up,
                down_active: can_down,
            }),
            action_shortcuts: Some(actions),
            counter: Some((cur_file, total_files)),
            border_color,
            show_close_button: true,
            ..Default::default()
        },
        hitboxes,
    );

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70),
            Constraint::Percentage(30),
        ])
        .split(inner_area);

    render_file_table(f, app, theme, main_chunks[0], hitboxes);
    render_file_actions(f, app, theme, main_chunks[1], hitboxes);
}

fn render_file_table(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect, hitboxes: &mut Vec<Hitbox>) {
    let visible_height = area.height.saturating_sub(3) as usize;
    let start_idx = app.file_scroll_offset;
    let end_idx = (start_idx + visible_height).min(app.file_entries.len());

    let rows: Vec<Row> = if app.file_entries.is_empty() {
        vec![Row::new(vec![
            Cell::from(Span::styled(" Empty folder", Style::default().fg(theme.text_muted))),
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
                let is_dragging = app.is_dragging_scrollbar(crate::app::ScrollbarTarget::Files);
                let is_selected = !is_dragging && actual_i == app.file_selected_idx;

                let cursor = if is_selected {
                    Span::styled("▶ ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))
                } else {
                    Span::styled("  ", Style::default())
                };

                let (type_badge, name_color) = if entry.is_dir {
                    (Span::styled("[DIR] ", Style::default().fg(theme.blue).add_modifier(Modifier::BOLD)), theme.blue)
                } else {
                    (Span::styled("[FILE] ", Style::default().fg(theme.text_muted)), theme.text_bright)
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
                    ("[IGNORED]", theme.red)
                } else {
                    ("[SYNC]", theme.green)
                };

                if is_selected {
                    let highlight_bg = Color::Rgb(90, 32, 32);
                    let type_str = if entry.is_dir { "[DIR] " } else { "[FILE] " };
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
        Row::new(vec!["File / Folder Name", "Size", "Modified", "Status"])
            .style(Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
    );

    f.render_widget(table, area);

    crate::ui::render_scrollbar_pane(
        f,
        area,
        app.file_entries.len(),
        start_idx,
        visible_height,
        theme,
        hitboxes,
        crate::app::ScrollbarTarget::Files,
    );
}

fn render_file_actions(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect, hitboxes: &mut Vec<Hitbox>) {
    let mut lines = Vec::new();

    if let Some(entry) = app.file_entries.get(app.file_selected_idx) {
        lines.push(Line::from(vec![
            Span::styled("Selected item:", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Name: ", Style::default().fg(theme.text_muted)),
            Span::styled(&entry.name, Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Type: ", Style::default().fg(theme.text_muted)),
            Span::styled(if entry.is_dir { "Folder" } else { "File" }, Style::default().fg(theme.cyan)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Size: ", Style::default().fg(theme.text_muted)),
            Span::styled(entry.size_formatted(), Style::default().fg(theme.text_bright)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Modified: ", Style::default().fg(theme.text_muted)),
            Span::styled(&entry.mtime, Style::default().fg(theme.text_muted)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Bisync filter: ", Style::default().fg(theme.text_muted)),
            Span::styled(
                if entry.ignored { "Excluded" } else { "Included" },
                Style::default().fg(if entry.ignored { theme.red } else { theme.green }),
            ),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Quick actions:", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" [→ / Enter] ", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)),
        Span::styled("Enter / Open", Style::default().fg(theme.text_bright)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" [← / Backspace] ", Style::default().fg(theme.blue).add_modifier(Modifier::BOLD)),
        Span::styled("Parent folder", Style::default().fg(theme.text_bright)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" [d] ", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)),
        Span::styled("Open in OS (xdg-open)", Style::default().fg(theme.text_bright)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" [x] ", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)),
        Span::styled("Add exclusion rule", Style::default().fg(theme.purple)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" [Delete] ", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
        Span::styled("Delete locally", Style::default().fg(theme.red)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" [Esc / q] ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
        Span::styled("Close window", Style::default().fg(theme.text_muted)),
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
        Span::styled(" [ Close (Esc / q) ] ", Style::default().fg(theme.text_bright).bg(theme.border)),
    ])).alignment(ratatui::layout::Alignment::Center);
    f.render_widget(close_p, close_btn_area);
}
