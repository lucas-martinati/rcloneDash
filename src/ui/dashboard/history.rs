use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use crate::app::{App, FocusedPanel, HitAction, Hitbox};
use crate::monitor::events::FileAction;
use crate::monitor::history::RunStatus;
use crate::ui::theme::ThemePalette;

/// Renders the History Panel with 2D duration bar graph, interactive runs table, and scrollbar.
pub fn render_history_panel(
    f: &mut Frame,
    app: &App,
    theme: &ThemePalette,
    area: Rect,
    hitboxes: &mut Vec<Hitbox>,
) {
    let is_focused = app.focused_panel == FocusedPanel::History;
    let border_color = if is_focused { theme.border_focus } else { theme.border_history };
    let is_syncing = app.is_syncing();
    let total_runs = app.total_history_runs();
    let cur_run = if let Some(sel) = app.selected_run_idx { sel + 1 } else { 0 };

    hitboxes.push(Hitbox {
        rect: area,
        action: HitAction::HistoryArea,
    });

    let graph_height = if area.height >= 20 { 4 } else if area.height >= 16 { 3 } else { 2 };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(graph_height), // 2D multiline duration graph
            Constraint::Min(6),               // Runs table
        ])
        .split(Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        });

    let (up_col, down_col) = if total_runs <= 1 {
        (theme.text_muted, theme.text_muted)
    } else {
        match app.selected_run_idx {
            None => (theme.text_muted, theme.red),
            Some(0) => (theme.text_muted, if total_runs > 1 { theme.red } else { theme.text_muted }),
            Some(i) if i >= total_runs.saturating_sub(1) => (theme.red, theme.text_muted),
            Some(_) => (theme.red, theme.red),
        }
    };

    let is_running_sync_selected = is_syncing && app.selected_run_idx == Some(0);
    let (det_col, key_col) = if total_runs == 0 || app.selected_run_idx.is_none() || is_running_sync_selected {
        (theme.text_muted, theme.text_muted)
    } else {
        (theme.text_bright, theme.red)
    };

    let bg = app.border_glyphs();
    let mut left_bottom_spans = vec![
        Span::styled(bg.bot_left, Style::default().fg(border_color)),
        Span::styled("↑", Style::default().fg(up_col).add_modifier(Modifier::BOLD)),
        Span::styled(" select ", Style::default().fg(Color::White)),
        Span::styled("↓", Style::default().fg(down_col).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{}{}", bg.bot_right, bg.bot_left), Style::default().fg(border_color)),
    ];
    left_bottom_spans.extend(crate::ui::keys::KeybindingRegistry::format_shortcut_label("↵", "details", key_col, det_col));
    left_bottom_spans.push(Span::styled(bg.bot_right, Style::default().fg(border_color)));
    let left_bottom = Line::from(left_bottom_spans);
    let right_bottom = Line::from(vec![
        Span::styled(format!("{} {}/{} {}", bg.horizontal, cur_run, total_runs, bg.horizontal), Style::default().fg(border_color).add_modifier(Modifier::BOLD)),
    ]);

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(app.border_type())
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.card_bg))
        .title(Line::from(vec![
            Span::styled(bg.top_left, Style::default().fg(border_color)),
            Span::styled("³", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
            Span::styled("history", Style::default().fg(border_color).add_modifier(Modifier::BOLD)),
            Span::styled(bg.top_right, Style::default().fg(border_color)),
        ]))
        .title_bottom(left_bottom.alignment(Alignment::Left))
        .title_bottom(right_bottom.alignment(Alignment::Right));
    f.render_widget(outer_block, area);

    hitboxes.push(Hitbox {
        rect: Rect {
            x: area.x,
            y: area.y,
            width: 12,
            height: 1,
        },
        action: HitAction::ToggleBox(3),
    });

    // Hitbox for the bottom details button (active only when a past run is selected)
    if let Some(idx) = app.selected_run_idx {
        if !(is_syncing && idx == 0) {
            let bottom_y = area.y + area.height.saturating_sub(1);
            hitboxes.push(Hitbox {
                rect: Rect { x: area.x + 11, y: bottom_y, width: 11, height: 1 },
                action: HitAction::HistoryRow(idx),
            });
        }
    }

    // 1. Multiline duration bar chart with per-column hitboxes
    let past_selected = match app.selected_run_idx {
        Some(idx) => {
            if is_syncing {
                if idx > 0 { Some(idx - 1) } else { None }
            } else if !app.past_runs.is_empty() {
                Some(idx)
            } else {
                None
            }
        }
        None => None,
    };
    let (lines, col_hitboxes) = crate::ui::sparkline::render_history_graph_multiline(
        &app.past_runs,
        theme,
        chunks[0].width as usize,
        chunks[0].height as usize,
        past_selected,
        app.config.graph_style,
    );
    let bar_p = Paragraph::new(lines);
    f.render_widget(bar_p, chunks[0]);

    for (col_x, col_w, orig_idx) in col_hitboxes {
        let row_idx_target = if is_syncing { orig_idx + 1 } else { orig_idx };
        hitboxes.push(Hitbox {
            rect: Rect {
                x: chunks[0].x + col_x as u16,
                y: chunks[0].y + 1,
                width: col_w as u16,
                height: chunks[0].height.saturating_sub(1),
            },
            action: HitAction::HistoryRow(row_idx_target),
        });
    }

    // 2. Runs table with hitboxes and scroll offset
    let visible_rows = chunks[1].height.saturating_sub(2) as usize;
    let max_offset = total_runs.saturating_sub(visible_rows);
    let offset = match app.selected_run_idx {
        Some(sel) => {
            if sel < app.history_scroll_offset {
                sel
            } else if visible_rows > 0 && sel >= app.history_scroll_offset + visible_rows {
                sel.saturating_sub(visible_rows) + 1
            } else {
                app.history_scroll_offset.min(max_offset)
            }
        }
        None => app.history_scroll_offset.min(max_offset),
    };

    let mut table_rows: Vec<Row> = Vec::new();
    let mut visible_indices: Vec<usize> = Vec::new();

    for item_idx in offset..(offset + visible_rows).min(total_runs) {
        visible_indices.push(item_idx);
        let is_dragging = app.is_dragging_scrollbar(crate::app::ScrollbarTarget::History);
        let is_selected = !is_dragging && app.selected_run_idx == Some(item_idx);
        let highlight_bg = Color::Rgb(90, 32, 32);

        if is_syncing && item_idx == 0 {
            // In-progress synchronization row
            let elapsed = if app.live.transfer.elapsed.is_empty() { "0s" } else { &app.live.transfer.elapsed };
            let time_str = "In progress";
            let has_transfer_info = app.live.phase_index >= 3
                && (app.live.transfer.pct > 0 || app.live.transfer.files_total > 0 || !app.live.transfer.speed.is_empty());

            let status_str = if has_transfer_info {
                format!("● {}%", app.live.overall_progress_pct())
            } else if app.live.phase_index == 0 {
                "● Listing".to_string()
            } else {
                "● Scanning".to_string()
            };
            let copied_count = app.live.synced_files.iter().filter(|f| f.action == FileAction::New || f.action == FileAction::Copied).count().max(app.live.transfer.files_done as usize);
            let copied_val = copied_count.to_string();
            let mod_count = app.live.synced_files.iter().filter(|f| f.action == FileAction::Modified).count();
            let mod_val = mod_count.to_string();
            let del_count = app.live.synced_files.iter().filter(|f| f.action == FileAction::Deleted).count();
            let del_val = del_count.to_string();
            let err_val = "0";

            if is_selected {
                let cursor = Span::styled("▶ ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
                table_rows.push(Row::new(vec![
                    Cell::from(Line::from(vec![cursor, Span::styled(time_str, Style::default().fg(Color::White).add_modifier(Modifier::BOLD))])),
                    Cell::from(Span::styled(status_str, Style::default().fg(Color::White).add_modifier(Modifier::BOLD))),
                    Cell::from(Span::styled(copied_val, Style::default().fg(Color::White).add_modifier(Modifier::BOLD))),
                    Cell::from(Span::styled(mod_val, Style::default().fg(Color::White).add_modifier(Modifier::BOLD))),
                    Cell::from(Span::styled(del_val, Style::default().fg(Color::White).add_modifier(Modifier::BOLD))),
                    Cell::from(Span::styled(elapsed, Style::default().fg(Color::White))),
                    Cell::from(Span::styled(err_val, Style::default().fg(Color::White).add_modifier(Modifier::BOLD))),
                ]).style(Style::default().bg(highlight_bg)));
            } else {
                let cursor = Span::styled("  ", Style::default());
                table_rows.push(Row::new(vec![
                    Cell::from(Line::from(vec![cursor, Span::styled(time_str, Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD))])),
                    Cell::from(Span::styled(status_str, Style::default().fg(if has_transfer_info { theme.cyan } else { theme.yellow }).add_modifier(Modifier::BOLD))),
                    Cell::from(Span::styled(copied_val, Style::default().fg(theme.green).add_modifier(Modifier::BOLD))),
                    Cell::from(Span::styled(mod_val, Style::default().fg(theme.yellow))),
                    Cell::from(Span::styled(del_val, Style::default().fg(theme.red))),
                    Cell::from(Span::styled(elapsed, Style::default().fg(theme.text_bright))),
                    Cell::from(Span::styled(err_val, Style::default().fg(theme.text_muted))),
                ]));
            }
        } else {
            let past_idx = if is_syncing { item_idx - 1 } else { item_idx };
            if let Some(run) = app.past_runs.get(past_idx) {
                let (status_badge, status_color) = match run.status {
                    RunStatus::Success => ("✔ Success", theme.green),
                    RunStatus::Failed => ("✗ Error", theme.red),
                    RunStatus::Skipped => ("⊘ Skipped", theme.text_muted),
                };

                let time_short = if run.date == chrono::Local::now().format("%Y-%m-%d").to_string() {
                    format!("Today {}", run.time)
                } else {
                    format!("{} {}", run.date, run.time)
                };

                let err_color = if run.errors.is_empty() { theme.text_muted } else { theme.red };

                if is_selected {
                    let cursor = Span::styled("▶ ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
                    table_rows.push(Row::new(vec![
                        Cell::from(Line::from(vec![cursor, Span::styled(time_short, Style::default().fg(Color::White).add_modifier(Modifier::BOLD))])),
                        Cell::from(Span::styled(status_badge, Style::default().fg(Color::White).add_modifier(Modifier::BOLD))),
                        Cell::from(Span::styled(run.files_copied.len().to_string(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD))),
                        Cell::from(Span::styled(run.files_modified.len().to_string(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD))),
                        Cell::from(Span::styled(run.files_deleted.len().to_string(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD))),
                        Cell::from(Span::styled(&run.duration, Style::default().fg(Color::White))),
                        Cell::from(Span::styled(run.errors.len().to_string(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD))),
                    ]).style(Style::default().bg(highlight_bg)));
                } else {
                    let cursor = Span::styled("  ", Style::default());
                    table_rows.push(Row::new(vec![
                        Cell::from(Line::from(vec![cursor, Span::styled(time_short, Style::default().fg(theme.text_bright))])),
                        Cell::from(Span::styled(status_badge, Style::default().fg(status_color).add_modifier(Modifier::BOLD))),
                        Cell::from(Span::styled(run.files_copied.len().to_string(), Style::default().fg(theme.green))),
                        Cell::from(Span::styled(run.files_modified.len().to_string(), Style::default().fg(theme.yellow))),
                        Cell::from(Span::styled(run.files_deleted.len().to_string(), Style::default().fg(theme.red))),
                        Cell::from(Span::styled(&run.duration, Style::default().fg(theme.text_muted))),
                        Cell::from(Span::styled(run.errors.len().to_string(), Style::default().fg(err_color))),
                    ]));
                }
            }
        }
    }

    // Register row hitboxes for mouse click selection
    for (row_idx, real_idx) in visible_indices.iter().enumerate() {
        let row_y = chunks[1].y + 1 + row_idx as u16;
        if row_y < chunks[1].y + chunks[1].height {
            hitboxes.push(Hitbox {
                rect: Rect {
                    x: chunks[1].x,
                    y: row_y,
                    width: chunks[1].width,
                    height: 1,
                },
                action: HitAction::HistoryRow(*real_idx),
            });
        }
    }

    let table = Table::new(
        table_rows,
        [
            Constraint::Percentage(24),
            Constraint::Percentage(20),
            Constraint::Percentage(11),
            Constraint::Percentage(11),
            Constraint::Percentage(11),
            Constraint::Percentage(14),
            Constraint::Percentage(9),
        ],
    )
    .header(
        Row::new(vec!["START", "STATUS", "COPIED", "MODIF.", "DELETED", "DURATION", "ERR."])
            .style(Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
    );

    f.render_widget(table, chunks[1]);

    crate::ui::render_scrollbar(
        f,
        area,
        total_runs,
        offset,
        visible_rows,
        theme,
        hitboxes,
        crate::app::ScrollbarTarget::History,
    );
}
