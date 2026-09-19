use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

use crate::app::{App, FocusedPanel, HitAction, Hitbox};
use crate::ui::theme::ThemePalette;

/// Renders the Recent Files panel displaying recently transferred files with action badges.
pub fn render_recent_files_panel(
    f: &mut Frame,
    app: &App,
    theme: &ThemePalette,
    area: Rect,
    hitboxes: &mut Vec<Hitbox>,
) {
    hitboxes.push(Hitbox {
        rect: area,
        action: HitAction::RecentFilesArea,
    });

    let is_focused = app.focused_panel == FocusedPanel::RecentFiles;
    let border_color = if is_focused { theme.border_focus } else { theme.border_recent };

    let files_to_display = app.get_all_recent_files();

    let total_files = files_to_display.len();
    let cur_file = if let Some(sel) = app.recent_selected_idx { sel + 1 } else { 0 };

    let max_show = area.height.saturating_sub(3) as usize;
    let max_offset = files_to_display.len().saturating_sub(max_show);
    let offset = match app.recent_selected_idx {
        Some(sel) => {
            if sel < app.recent_scroll_offset {
                sel
            } else if max_show > 0 && sel >= app.recent_scroll_offset + max_show {
                sel.saturating_sub(max_show) + 1
            } else {
                app.recent_scroll_offset.min(max_offset)
            }
        }
        None => app.recent_scroll_offset.min(max_offset),
    };

    let highlight_bg = Color::Rgb(90, 32, 32);

    let rows: Vec<Row> = if files_to_display.is_empty() {
        vec![Row::new(vec![
            Cell::from(Span::styled(" No recently synchronized files", Style::default().fg(theme.text_muted))),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
        ])]
    } else {
        files_to_display
            .iter()
            .enumerate()
            .skip(offset)
            .take(max_show)
            .map(|(real_idx, (action, path, size, time))| {
                let is_dragging = app.is_dragging_scrollbar(crate::app::ScrollbarTarget::RecentFiles);
                let is_selected = !is_dragging && app.recent_selected_idx == Some(real_idx);
                let (badge_text, badge_color) = match action.as_str() {
                    "new" => ("● Added", theme.green),
                    "deleted" => ("● Deleted", theme.red),
                    _ => ("● Modified", theme.yellow),
                };

                let file_path = std::path::Path::new(path);
                let display_path = if app.ctrl_mode {
                    let parent = file_path.parent().and_then(|p| p.to_str()).unwrap_or("");
                    let parent_clean = parent.trim_start_matches('/').trim_end_matches('/');
                    if parent_clean.is_empty() {
                        "📁 ./".to_string()
                    } else {
                        format!("📁 {}/", parent_clean)
                    }
                } else {
                    normalize_display_path(path)
                };

                if is_selected {
                    let cursor = Span::styled("▶ ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
                    let badge = Span::styled(format!("{} ", badge_text), Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
                    let path_span = Span::styled(display_path, Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
                    let size_span = Span::styled(size, Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
                    let time_span = Span::styled(time, Style::default().fg(Color::White).add_modifier(Modifier::BOLD));

                    Row::new(vec![
                        Cell::from(Line::from(vec![cursor, badge])),
                        Cell::from(path_span),
                        Cell::from(size_span),
                        Cell::from(time_span),
                    ])
                    .style(Style::default().bg(highlight_bg))
                } else {
                    let cursor = Span::styled("  ", Style::default());
                    let badge = Span::styled(format!("{} ", badge_text), Style::default().fg(badge_color).add_modifier(Modifier::BOLD));
                    let path_span = if app.ctrl_mode {
                        Span::styled(display_path, Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD))
                    } else {
                        Span::styled(display_path, Style::default().fg(theme.text_bright))
                    };
                    let size_span = Span::styled(size, Style::default().fg(theme.text_muted));
                    let time_span = Span::styled(time, Style::default().fg(theme.text_muted));

                    Row::new(vec![
                        Cell::from(Line::from(vec![cursor, badge])),
                        Cell::from(path_span),
                        Cell::from(size_span),
                        Cell::from(time_span),
                    ])
                }
            })
            .collect()
    };

    // Hitboxes for visible recent files rows
    for (row_i, real_idx) in (offset..offset + max_show.min(files_to_display.len().saturating_sub(offset))).enumerate() {
        let row_y = area.y + 1 + row_i as u16;
        if row_y < area.y + area.height.saturating_sub(1) {
            hitboxes.push(Hitbox {
                rect: Rect {
                    x: area.x,
                    y: row_y,
                    width: area.width,
                    height: 1,
                },
                action: HitAction::RecentFile(real_idx),
            });
        }
    }

    let (up_col, down_col) = if total_files <= 1 {
        (theme.text_muted, theme.text_muted)
    } else {
        match app.recent_selected_idx {
            None => (theme.text_muted, theme.red),
            Some(0) => (theme.text_muted, if total_files > 1 { theme.red } else { theme.text_muted }),
            Some(i) if i >= total_files.saturating_sub(1) => (theme.red, theme.text_muted),
            Some(_) => (theme.red, theme.red),
        }
    };

    let (opn_col, key_col) = if app.recent_selected_idx.is_none() || total_files == 0 {
        (theme.text_muted, theme.text_muted)
    } else {
        (theme.text_bright, theme.red)
    };

    let bg = app.border_glyphs();
    let mut bottom_spans = vec![
        Span::styled(bg.bot_left, Style::default().fg(border_color)),
        Span::styled("↑", Style::default().fg(up_col).add_modifier(Modifier::BOLD)),
        Span::styled(" select ", Style::default().fg(Color::White)),
        Span::styled("↓", Style::default().fg(down_col).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{}{}", bg.bot_right, bg.bot_left), Style::default().fg(border_color)),
    ];
    bottom_spans.extend(crate::ui::keys::KeybindingRegistry::format_shortcut_label("↵", "open", key_col, opn_col));
    bottom_spans.push(Span::styled(bg.bot_right, Style::default().fg(border_color)));

    let left_bottom = Line::from(bottom_spans);
    let right_bottom = Line::from(vec![
        Span::styled(format!("{} {}/{} {}", bg.horizontal, cur_file, total_files, bg.horizontal), Style::default().fg(border_color).add_modifier(Modifier::BOLD)),
    ]);

    let mut top_spans = vec![
        Span::styled(bg.top_left, Style::default().fg(border_color)),
        Span::styled("recent files", Style::default().fg(border_color).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{}{}", bg.top_right, bg.top_left), Style::default().fg(border_color)),
    ];
    let filter_start_x = area.x + 14;
    let filter_w = if app.is_filtering_recent {
        top_spans.push(Span::styled("filter: ", Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD)));
        top_spans.push(Span::styled(&app.recent_filter, Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)));
        top_spans.push(Span::styled("█", Style::default().fg(theme.highlight)));
        8 + app.recent_filter.chars().count() as u16 + 1
    } else if !app.recent_filter.is_empty() {
        top_spans.push(Span::styled("filter: ", Style::default().fg(theme.text_muted)));
        top_spans.push(Span::styled(&app.recent_filter, Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)));
        top_spans.push(Span::styled(" (", Style::default().fg(theme.text_muted)));
        top_spans.push(Span::styled("f", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)));
        top_spans.push(Span::styled(")", Style::default().fg(theme.text_muted)));
        11 + app.recent_filter.chars().count() as u16
    } else {
        top_spans.extend(crate::ui::keys::KeybindingRegistry::format_shortcut_label("f", "filter", theme.red, theme.text_bright));
        6
    };

    top_spans.push(Span::styled(format!("{}{}", bg.top_right, bg.top_left), Style::default().fg(border_color)));
    let (folder_text, folder_color) = if app.ctrl_mode {
        ("folder [ON]", theme.yellow)
    } else {
        ("folder", theme.text_bright)
    };
    top_spans.extend(crate::ui::keys::KeybindingRegistry::format_shortcut_label("Ctrl+X", folder_text, theme.red, folder_color));
    top_spans.push(Span::styled(bg.top_right, Style::default().fg(border_color)));

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(app.border_type())
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.card_bg))
        .title(Line::from(top_spans))
        .title_bottom(left_bottom.alignment(Alignment::Left))
        .title_bottom(right_bottom.alignment(Alignment::Right));

    // Hitbox for the filter tab in the top header
    hitboxes.push(Hitbox {
        rect: Rect { x: filter_start_x, y: area.y, width: filter_w, height: 1 },
        action: HitAction::RecentFilterFocus,
    });

    // Hitbox for the Ctrl+X folder tab in the top header
    hitboxes.push(Hitbox {
        rect: Rect { x: filter_start_x + filter_w + 2, y: area.y, width: 6 + folder_text.chars().count() as u16, height: 1 },
        action: HitAction::ToggleCtrlMode,
    });

    if let Some(idx) = app.recent_selected_idx {
        let bottom_y = area.y + area.height.saturating_sub(1);
        hitboxes.push(Hitbox {
            rect: Rect { x: area.x + 13, y: bottom_y, width: 8, height: 1 },
            action: HitAction::RecentFile(idx),
        });
    }

    let header_title = if app.ctrl_mode { "PARENT FOLDER (CTRL MODE ACTIVE)" } else { "FILE PATH" };
    let table = Table::new(
        rows,
        [
            Constraint::Percentage(16),
            Constraint::Percentage(56),
            Constraint::Percentage(14),
            Constraint::Percentage(14),
        ],
    )
    .header(
        Row::new(vec!["ACTION", header_title, "SIZE", "TIME"])
            .style(Style::default().fg(if app.ctrl_mode { theme.yellow } else { theme.text_muted }).add_modifier(Modifier::BOLD)),
    )
    .block(outer_block);

    f.render_widget(table, area);

    crate::ui::render_scrollbar(
        f,
        area,
        files_to_display.len(),
        offset,
        max_show,
        theme,
        hitboxes,
        crate::app::ScrollbarTarget::RecentFiles,
    );
}

/// Normalizes file paths for clean dashboard display.
pub fn normalize_display_path(p: &str) -> String {
    let mut s = p.replace(" From ", " from ");
    if s.starts_with("From ") {
        s = format!("from {}", &s[5..]);
    }
    s.replace("/From ", "/from ")
}
