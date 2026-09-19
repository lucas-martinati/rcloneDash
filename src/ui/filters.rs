use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, HitAction, Hitbox};
use crate::config;
use crate::ui::theme::ThemePalette;

pub fn render_filters_modal(f: &mut Frame, app: &App, theme: &ThemePalette, hitboxes: &mut Vec<Hitbox>) {
    let area = centered_rect(82, 74, f.area());
    f.render_widget(Clear, area);

    let filepath = config::filters_file().display().to_string();
    let total_rules = if app.is_adding_filter { app.filters.len() + 1 } else { app.filters.len() };
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

    let bg = app.border_glyphs();
    let max_path_len = (area.width.saturating_sub(42) as usize).max(10);
    let display_path = if filepath.len() > max_path_len {
        format!("…{}", &filepath[filepath.len().saturating_sub(max_path_len - 1)..])
    } else {
        filepath.clone()
    };

    use crate::ui::keys::KeybindingRegistry;

    let sep = format!("{}{}", bg.bot_right, bg.bot_left);
    let mut bottom_spans = vec![
        Span::styled(bg.bot_left, Style::default().fg(theme.purple)),
        Span::styled("↑", Style::default().fg(up_col).add_modifier(Modifier::BOLD)),
        Span::styled(" select ", Style::default().fg(Color::White)),
        Span::styled("↓", Style::default().fg(down_col).add_modifier(Modifier::BOLD)),
    ];

    if app.is_editing_filter {
        bottom_spans.push(Span::styled(&sep, Style::default().fg(theme.purple)));
        bottom_spans.extend(KeybindingRegistry::format_shortcut_label("↵", "save", theme.green, Color::White));
        bottom_spans.push(Span::styled(&sep, Style::default().fg(theme.purple)));
        bottom_spans.extend(KeybindingRegistry::format_shortcut_label("Esc", "cancel", theme.red, Color::White));
        bottom_spans.push(Span::styled(&sep, Style::default().fg(theme.purple)));
        bottom_spans.extend(KeybindingRegistry::format_shortcut_label("Backspace", "del", theme.yellow, Color::White));
        bottom_spans.push(Span::styled(bg.bot_right, Style::default().fg(theme.purple)));
    } else {
        bottom_spans.push(Span::styled(&sep, Style::default().fg(theme.purple)));
        bottom_spans.extend(KeybindingRegistry::format_shortcut_label("e", "edit", theme.yellow, Color::White));
        bottom_spans.push(Span::styled(&sep, Style::default().fg(theme.purple)));
        bottom_spans.extend(KeybindingRegistry::format_shortcut_label("a", "add", theme.green, Color::White));
        bottom_spans.push(Span::styled(&sep, Style::default().fg(theme.purple)));
        bottom_spans.extend(KeybindingRegistry::format_shortcut_label("d", "del", theme.red, Color::White));
        bottom_spans.push(Span::styled(&sep, Style::default().fg(theme.purple)));
        bottom_spans.extend(KeybindingRegistry::format_shortcut_label("t", "type", theme.cyan, Color::White));
        bottom_spans.push(Span::styled(&sep, Style::default().fg(theme.purple)));
        bottom_spans.extend(KeybindingRegistry::format_shortcut_label("E", "Editor", theme.purple, Color::White));
        bottom_spans.push(Span::styled(bg.bot_right, Style::default().fg(theme.purple)));
    }

    let bottom_shortcuts = Line::from(bottom_spans);

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(app.border_type())
        .border_style(Style::default().fg(theme.purple))
        .style(Style::default().bg(theme.card_bg))
        .title(Line::from(vec![
            Span::styled(bg.top_left, Style::default().fg(theme.purple)),
            Span::styled("exclusion filters", Style::default().fg(theme.purple).add_modifier(Modifier::BOLD)),
            Span::styled(format!(": {}", display_path), Style::default().fg(theme.text_muted)),
            Span::styled(bg.top_right, Style::default().fg(theme.purple)),
        ]))
        .title(
            Line::from(vec![
                Span::styled(bg.top_left, Style::default().fg(theme.purple)),
                Span::styled("Esc, q", Style::default().fg(theme.purple).add_modifier(Modifier::BOLD)),
                Span::styled(" close", Style::default().fg(theme.text_bright)),
                Span::styled(bg.top_right, Style::default().fg(theme.purple)),
            ])
            .alignment(Alignment::Right),
        )
        .title_bottom(bottom_shortcuts.alignment(Alignment::Left))
        .title_bottom(
            Line::from(vec![
                Span::styled(format!("{} {}/{} {}", bg.horizontal, cur_rule, total_rules, bg.horizontal), Style::default().fg(theme.purple).add_modifier(Modifier::BOLD)),
            ])
            .alignment(Alignment::Right),
        );
    f.render_widget(outer_block, area);

    hitboxes.push(Hitbox {
        rect: Rect {
            x: area.x + area.width.saturating_sub(14),
            y: area.y,
            width: 12,
            height: 1,
        },
        action: HitAction::CloseModal,
    });

    if !app.is_editing_filter {
        let bottom_y = area.y + area.height.saturating_sub(1);
        hitboxes.push(Hitbox {
            rect: Rect { x: area.x + 13, y: bottom_y, width: 4, height: 1 },
            action: HitAction::FilterStartEdit,
        });
        hitboxes.push(Hitbox {
            rect: Rect { x: area.x + 19, y: bottom_y, width: 3, height: 1 },
            action: HitAction::FilterAdd,
        });
        hitboxes.push(Hitbox {
            rect: Rect { x: area.x + 24, y: bottom_y, width: 3, height: 1 },
            action: HitAction::FilterDelete(app.selected_filter_idx),
        });
        hitboxes.push(Hitbox {
            rect: Rect { x: area.x + 29, y: bottom_y, width: 4, height: 1 },
            action: HitAction::FilterCycleType(app.selected_filter_idx),
        });
        hitboxes.push(Hitbox {
            rect: Rect { x: area.x + 35, y: bottom_y, width: 6, height: 1 },
            action: HitAction::FilterOpenEditor,
        });
    }

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

    // Hitbox pour toute la zone de liste (molette)
    hitboxes.push(Hitbox {
        rect: area,
        action: HitAction::FilterArea,
    });

    let total_count = if app.is_adding_filter { app.filters.len() + 1 } else { app.filters.len() };

    let offset = if app.selected_filter_idx < app.filter_scroll_offset {
        app.selected_filter_idx
    } else if app.selected_filter_idx >= app.filter_scroll_offset + visible_height {
        app.selected_filter_idx.saturating_sub(visible_height) + 1
    } else {
        app.filter_scroll_offset
    };

    let items: Vec<ListItem> = if total_count == 0 {
        vec![
            ListItem::new(Line::from(vec![
                Span::styled(" No filter rules in gdrive-filters.txt.", Style::default().fg(theme.text_muted)),
            ])),
            ListItem::new(Line::from(vec![
                Span::styled(" Press ", Style::default().fg(theme.text_muted)),
                Span::styled("[a]", Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
                Span::styled(" to add your first exclusion rule.", Style::default().fg(theme.text_bright)),
            ])),
        ]
    } else {
        (offset..(offset + visible_height).min(total_count))
            .map(|i| {
                let is_adding_this = app.is_adding_filter && i == app.filters.len();
                let is_editing_this = app.is_editing_filter && (is_adding_this || (!app.is_adding_filter && i == app.selected_filter_idx));
                let is_dragging = app.is_dragging_scrollbar(crate::app::ScrollbarTarget::Filters);
                let is_selected = !is_dragging && i == app.selected_filter_idx;

                let row_y = area.y + (i.saturating_sub(offset)) as u16;
                if row_y < area.y + area.height && !is_adding_this {
                    // Clic sur l'indicateur ou numéro pour cycler le type
                    hitboxes.push(Hitbox {
                        rect: Rect {
                            x: area.x,
                            y: row_y,
                            width: 8,
                            height: 1,
                        },
                        action: HitAction::FilterCycleType(i),
                    });

                    // Clic sur le corps de la règle pour sélectionner la ligne
                    hitboxes.push(Hitbox {
                        rect: Rect {
                            x: area.x + 8,
                            y: row_y,
                            width: area.width.saturating_sub(8),
                            height: 1,
                        },
                        action: HitAction::FilterRow(i),
                    });
                }

                if is_editing_this {
                    let buf = &app.filter_edit_buffer;
                    let text_col = if buf.starts_with('-') {
                        Color::Rgb(255, 175, 175)
                    } else if buf.starts_with('+') {
                        Color::Rgb(170, 255, 190)
                    } else if buf.starts_with('#') {
                        theme.text_muted
                    } else {
                        theme.text_bright
                    };

                    let line = Line::from(vec![
                        Span::styled("✎ ", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)),
                        Span::styled(format!("{:2} │ ", i + 1), Style::default().fg(theme.yellow)),
                        Span::styled("[ ", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)),
                        Span::styled(buf.clone(), Style::default().fg(text_col).add_modifier(Modifier::BOLD)),
                        Span::styled("█", Style::default().fg(theme.yellow).add_modifier(Modifier::RAPID_BLINK)),
                        Span::styled(" ]", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)),
                    ]);
                    ListItem::new(line).style(Style::default().bg(Color::Rgb(45, 30, 60)))
                } else {
                    let rule = &app.filters[i];
                    let trimmed = rule.trim();

                    let (sym, sym_fg, rule_text, rule_style) = if trimmed.starts_with('-') {
                        let rest = trimmed.strip_prefix('-').unwrap_or(trimmed).trim_start();
                        (
                            "- ",
                            theme.red,
                            rest,
                            if is_selected {
                                Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(Color::Rgb(255, 160, 160))
                            },
                        )
                    } else if trimmed.starts_with('+') {
                        let rest = trimmed.strip_prefix('+').unwrap_or(trimmed).trim_start();
                        (
                            "+ ",
                            theme.green,
                            rest,
                            if is_selected {
                                Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(Color::Rgb(160, 255, 180))
                            },
                        )
                    } else if trimmed.starts_with('#') {
                        let rest = trimmed.strip_prefix('#').unwrap_or(trimmed).trim_start();
                        (
                            "# ",
                            theme.text_muted,
                            rest,
                            if is_selected {
                                Style::default().fg(Color::Rgb(220, 220, 230)).add_modifier(Modifier::ITALIC)
                            } else {
                                Style::default().fg(theme.text_muted).add_modifier(Modifier::ITALIC)
                            },
                        )
                    } else {
                        (
                            "· ",
                            theme.cyan,
                            trimmed,
                            if is_selected {
                                Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(theme.text_bright)
                            },
                        )
                    };

                    let num_style = if is_selected {
                        Style::default().fg(theme.purple).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.separator)
                    };

                    let cursor_span = if is_selected {
                        Span::styled("▶ ", Style::default().fg(theme.purple).add_modifier(Modifier::BOLD))
                    } else {
                        Span::styled("  ", Style::default())
                    };

                    let line = Line::from(vec![
                        cursor_span,
                        Span::styled(format!("{:2} │ ", i + 1), num_style),
                        Span::styled(sym, Style::default().fg(sym_fg).add_modifier(Modifier::BOLD)),
                        Span::styled(rule_text, rule_style),
                    ]);

                    let row_style = if is_selected {
                        Style::default().bg(Color::Rgb(55, 32, 65))
                    } else {
                        Style::default()
                    };

                    ListItem::new(line).style(row_style)
                }
            })
            .collect()
    };

    let list = List::new(items).block(Block::default().borders(Borders::NONE));
    f.render_widget(list, area);

    crate::ui::render_btop_scrollbar(
        f,
        area,
        total_count,
        app.selected_filter_idx,
        visible_height,
        theme,
        hitboxes,
        crate::app::ScrollbarTarget::Filters,
    );
}

fn render_filters_help(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect, hitboxes: &mut Vec<Hitbox>) {
    let bg = app.border_glyphs();

    let lines = vec![
        Line::from(vec![
            Span::styled(bg.top_left, Style::default().fg(theme.cyan)),
            Span::styled("filter syntax", Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD)),
            Span::styled(bg.top_right, Style::default().fg(theme.cyan)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" - ", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
            Span::styled("Exclusion", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
            Span::styled(" (skip sync)", Style::default().fg(theme.text_muted)),
        ]),
        Line::from(vec![
            Span::styled("    - /folder/**, - *.tmp", Style::default().fg(Color::Rgb(255, 175, 175))),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" + ", Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
            Span::styled("Inclusion", Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
            Span::styled(" (force sync)", Style::default().fg(theme.text_muted)),
        ]),
        Line::from(vec![
            Span::styled("    + *.pdf, + /docs/**", Style::default().fg(Color::Rgb(175, 255, 195))),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" # ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
            Span::styled("Comment / Note", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
            Span::styled(" (ignored)", Style::default().fg(theme.text_muted)),
        ]),
        Line::from(vec![
            Span::styled("    # Project archive rules", Style::default().fg(theme.text_muted).add_modifier(Modifier::ITALIC)),
        ]),
        Line::from(""),
        Line::from(Span::styled(format!(" {} rules & persistence {}", bg.horizontal, bg.horizontal), Style::default().fg(theme.border))),
        Line::from(""),
        Line::from(Span::styled("• Exclusions ignore matching files", Style::default().fg(theme.text_bright))),
        Line::from(Span::styled("  and folders during synchronization.", Style::default().fg(theme.text_muted))),
        Line::from(""),
        Line::from(Span::styled("• Inclusions take priority over", Style::default().fg(theme.text_bright))),
        Line::from(Span::styled("  subsequent exclusion patterns.", Style::default().fg(theme.text_muted))),
        Line::from(""),
        Line::from(Span::styled("• All changes are saved automatically", Style::default().fg(theme.cyan))),
        Line::from(Span::styled("  to ~/.config/rclone/gdrive-filters.txt", Style::default().fg(theme.text_muted))),
        Line::from(Span::styled("  and applied on next bisync run.", Style::default().fg(theme.text_muted))),
    ];

    let close_btn_area = Rect {
        x: area.x + 2,
        y: area.y + area.height.saturating_sub(2),
        width: area.width.saturating_sub(4),
        height: 1,
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
