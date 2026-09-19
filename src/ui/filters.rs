use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, HitAction, Hitbox};
use crate::config;
use crate::ui::container::{centered_rect, render_modal_container, ModalContainerConfig};
use crate::ui::theme::ThemePalette;

pub fn render_filters_modal(f: &mut Frame, app: &App, theme: &ThemePalette, hitboxes: &mut Vec<Hitbox>) {
    let area = centered_rect(82, 74, f.area());

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

    let inner_area = render_modal_container(
        f,
        app,
        theme,
        area,
        ModalContainerConfig {
            title_prefix: "exclusion filters",
            title_color: Some(theme.purple),
            title_extra: Some(vec![Span::styled(format!(": {}", display_path), Style::default().fg(theme.text_muted))]),
            bottom_shortcuts: Some(bottom_shortcuts),
            counter: Some((cur_rule, total_rules)),
            border_color: theme.purple,
            show_close_button: true,
        },
        hitboxes,
    );

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

    let max_offset = total_count.saturating_sub(visible_height);
    let offset = app.filter_scroll_offset.min(max_offset);

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

                    let max_rule_w = (area.width.saturating_sub(12) as usize).max(5);
                    let display_rule = if rule_text.chars().count() > max_rule_w {
                        let truncated: String = rule_text.chars().take(max_rule_w - 1).collect();
                        format!("{}…", truncated)
                    } else {
                        rule_text.to_string()
                    };

                    let line = Line::from(vec![
                        cursor_span,
                        Span::styled(format!("{:2} │ ", i + 1), num_style),
                        Span::styled(sym, Style::default().fg(sym_fg).add_modifier(Modifier::BOLD)),
                        Span::styled(display_rule, rule_style),
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

    crate::ui::render_btop_scrollbar_pane(
        f,
        area,
        total_count,
        offset,
        visible_height,
        theme,
        hitboxes,
        crate::app::ScrollbarTarget::Filters,
    );
}

fn render_filters_help(f: &mut Frame, _app: &App, theme: &ThemePalette, area: Rect, _hitboxes: &mut Vec<Hitbox>) {
    let mut lines = Vec::new();
    lines.push(Line::from(vec![
        Span::styled(" FILTER SYNTAX ", Style::default().fg(theme.purple).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(" - ", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
        Span::styled("Exclusion", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
        Span::styled(" (skip sync)", Style::default().fg(theme.text_muted)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("    - /folder/**, - *.tmp", Style::default().fg(Color::Rgb(255, 175, 175))),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(" + ", Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
        Span::styled("Inclusion", Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
        Span::styled(" (force sync)", Style::default().fg(theme.text_muted)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("    + *.pdf, + /docs/**", Style::default().fg(Color::Rgb(175, 255, 195))),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(" # ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
        Span::styled("Comment / Note", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
        Span::styled(" (ignored)", Style::default().fg(theme.text_muted)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("    # Project archive rules", Style::default().fg(theme.text_muted).add_modifier(Modifier::ITALIC)),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("─".repeat(area.width.saturating_sub(2) as usize), Style::default().fg(theme.border))));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("• Exclusions ignore matching files", Style::default().fg(theme.text_bright))));
    lines.push(Line::from(Span::styled("  and folders during synchronization.", Style::default().fg(theme.text_muted))));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("• Inclusions take priority over", Style::default().fg(theme.text_bright))));
    lines.push(Line::from(Span::styled("  subsequent exclusion patterns.", Style::default().fg(theme.text_muted))));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("• All changes are saved automatically", Style::default().fg(theme.cyan))));
    lines.push(Line::from(Span::styled("  to ~/.config/rclone/gdrive-filters.txt", Style::default().fg(theme.text_muted))));
    lines.push(Line::from(Span::styled("  and applied on next bisync run.", Style::default().fg(theme.text_muted))));

    let p = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(theme.purple)),
    );
    f.render_widget(p, area);
}
