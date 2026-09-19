#![allow(dead_code)]

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, HitAction, Hitbox};
use crate::monitor::history::RunStatus;
use crate::ui::container::{centered_rect, render_modal_container, ModalContainerConfig, NavArrowsConfig};
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
    render_run_details(f, app, app.selected_run_idx.unwrap_or(0), theme, chunks[1], &mut Vec::new());
}

fn render_run_list(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect) {
    let items: Vec<ListItem> = if app.past_runs.is_empty() {
        vec![ListItem::new(Line::from(vec![
            Span::styled(" No runs recorded in journalctl", Style::default().fg(theme.text_muted)),
        ]))]
    } else {
        app.past_runs
            .iter()
            .enumerate()
            .map(|(i, run)| {
                let is_selected = app.selected_run_idx == Some(i);

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
                .border_type(app.border_type())
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.card_bg))
                .title(Span::styled(" Synchronization History ", Style::default().fg(theme.accent))),
        );

    f.render_widget(list, area);

    let visible_height = area.height.saturating_sub(3) as usize;
    let mut dummy_hitboxes = Vec::new();
    crate::ui::render_btop_scrollbar(
        f,
        area,
        app.past_runs.len(),
        app.selected_run_idx.unwrap_or(0),
        visible_height,
        theme,
        &mut dummy_hitboxes,
        crate::app::ScrollbarTarget::History,
    );
}

pub fn render_history_details_modal(f: &mut Frame, app: &App, run_idx: usize, theme: &ThemePalette, hitboxes: &mut Vec<Hitbox>) {
    let area = centered_rect(78, 74, f.area());
    f.render_widget(ratatui::widgets::Clear, area);
    render_run_details(f, app, run_idx, theme, area, hitboxes);
}

pub fn render_run_details(f: &mut Frame, app: &App, run_idx: usize, theme: &ThemePalette, area: Rect, hitboxes: &mut Vec<Hitbox>) {
    if app.past_runs.is_empty() || run_idx >= app.past_runs.len() {
        let p = Paragraph::new("No information available.")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(app.border_type())
                    .border_style(Style::default().fg(theme.border))
                    .style(Style::default().bg(theme.card_bg))
                    .title(" Run Details "),
            );
        f.render_widget(p, area);
        return;
    }

    let run = &app.past_runs[run_idx];
    let mut all_lines: Vec<(Line, Option<usize>)> = Vec::new(); // (line, option<file_idx>)

    // En-tête du run
    all_lines.push((Line::from(vec![
        Span::styled(" Synchronization #", Style::default().fg(theme.text_muted)),
        Span::styled(format!("{} ", run.id), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled(" on ", Style::default().fg(theme.text_muted)),
        Span::styled(format!("{} at {} ", run.date, run.time), Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)),
        Span::styled("│ Duration: ", Style::default().fg(theme.text_muted)),
        Span::styled(format!("{} ", run.duration), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
    ]), None));

    let (status_str, status_color) = match run.status {
        RunStatus::Success => ("✔ SUCCESS", theme.green),
        RunStatus::Failed => ("✗ FAILED", theme.red),
        RunStatus::Skipped => ("⊘ SKIPPED (No changes)", theme.text_muted),
        RunStatus::Running => ("▶ RUNNING", theme.cyan),
    };

    all_lines.push((Line::from(vec![
        Span::styled(" Status: ", Style::default().fg(theme.text_muted)),
        Span::styled(status_str, Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
    ]), None));

    all_lines.push((Line::from(Span::styled("─".repeat(area.width.saturating_sub(4) as usize), Style::default().fg(theme.border))), None));

    // Erreurs
    if !run.errors.is_empty() {
        all_lines.push((Line::from(vec![
            Span::styled(format!(" 🚨 DETECTED ERRORS ({}):", run.errors.len()), Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
        ]), None));

        let max_err_width = (area.width.saturating_sub(6) as usize).max(30);
        let first_limit = max_err_width.saturating_sub(5);
        let cont_limit = max_err_width.saturating_sub(7);

        for err in &run.errors {
            let wrapped = crate::ui::dashboard::wrap_text(err, first_limit, cont_limit);
            for (i, part) in wrapped.into_iter().enumerate() {
                if i == 0 {
                    all_lines.push((Line::from(vec![
                        Span::styled("   • ", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
                        Span::styled(part, Style::default().fg(theme.red)),
                    ]), None));
                } else {
                    all_lines.push((Line::from(vec![
                        Span::styled("     ↳ ", Style::default().fg(theme.text_muted)),
                        Span::styled(part, Style::default().fg(theme.red)),
                    ]), None));
                }
            }
        }
        all_lines.push((Line::from(Span::styled("─".repeat(area.width.saturating_sub(4) as usize), Style::default().fg(theme.border))), None));
    }

    let affected_files = run.all_affected_files();
    if !affected_files.is_empty() {
        all_lines.push((Line::from(vec![
            Span::styled(format!(" 📁 AFFECTED FILES ({}):", affected_files.len()), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        ]), None));

        for (idx, (action, path)) in affected_files.iter().enumerate() {
            let is_dragging = app.is_dragging_scrollbar(crate::app::ScrollbarTarget::HistoryDetails(run_idx));
            let is_selected = !is_dragging && idx == app.history_selected_file_idx;
            let (badge_icon, badge_color) = match *action {
                "new" => ("[+] Copied", theme.green),
                "deleted" => ("[-] Deleted", theme.red),
                _ => ("[~] Modified", theme.yellow),
            };

            let prefix = if is_selected { " ▶ " } else { "   " };

            let display_text = if app.ctrl_mode {
                let file_path = std::path::Path::new(*path);
                let parent = file_path.parent().and_then(|p| p.to_str()).unwrap_or("");
                let parent_clean = parent.trim_start_matches('/').trim_end_matches('/');
                if parent_clean.is_empty() {
                    "📁 ./".to_string()
                } else {
                    format!("📁 {}/", parent_clean)
                }
            } else {
                crate::ui::dashboard::normalize_display_path(path)
            };

            if is_selected {
                let highlight_bg = Color::Rgb(90, 32, 32);
                all_lines.push((Line::from(vec![
                    Span::styled(prefix, Style::default().fg(Color::White).bg(highlight_bg).add_modifier(Modifier::BOLD)),
                    Span::styled(format!(" {:<11} ", badge_icon), Style::default().fg(Color::White).bg(highlight_bg).add_modifier(Modifier::BOLD)),
                    Span::styled(display_text, Style::default().fg(Color::White).bg(highlight_bg).add_modifier(Modifier::BOLD)),
                ]), Some(idx)));
            } else {
                let path_style = if app.ctrl_mode {
                    Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text_bright)
                };
                all_lines.push((Line::from(vec![
                    Span::styled(prefix, Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD)),
                    Span::styled(format!(" {:<11} ", badge_icon), Style::default().fg(badge_color).add_modifier(Modifier::BOLD)),
                    Span::styled(display_text, path_style),
                ]), Some(idx)));
            }
        }
    } else if run.errors.is_empty() {
        all_lines.push((Line::from(Span::styled(
            "   No files were modified during this synchronization cycle.",
            Style::default().fg(theme.text_muted),
        )), None));
    }

    let has_errors = !run.errors.is_empty();
    let has_files = !affected_files.is_empty();

    let total_lines = all_lines.len();
    let visible_height = area.height.saturating_sub(4) as usize;
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll = app.history_details_scroll.min(max_scroll);

    let mut actions = Vec::new();

    if has_files {
        actions.push(crate::ui::keys::KeybindingRegistry::format_shortcut_label("↵", "Open", theme.highlight, Color::White));
        let mode_label = if app.ctrl_mode { "File" } else { "Folder" };
        actions.push(crate::ui::keys::KeybindingRegistry::format_shortcut_label("Ctrl+X", mode_label, theme.highlight, Color::White));
    }

    if has_errors {
        actions.push(crate::ui::keys::KeybindingRegistry::format_shortcut_label("c", "Copy errors", theme.red, Color::White));
    } else if has_files {
        actions.push(crate::ui::keys::KeybindingRegistry::format_shortcut_label("y", "Copy", theme.green, Color::White));
    }

    let inner = render_modal_container(
        f,
        app,
        theme,
        area,
        ModalContainerConfig {
            title_prefix: "run details",
            title_color: Some(theme.blue),
            title_extra: Some(vec![Span::styled(format!(": #{} ({} {})", run.id, run.date, run.time), Style::default().fg(theme.text_muted))]),
            nav_arrows: Some(NavArrowsConfig {
                label: "select",
                up_active: scroll > 0,
                down_active: scroll < max_scroll,
            }),
            action_shortcuts: Some(actions),
            counter: Some((scroll + 1, total_lines.max(1))),
            border_color: theme.border_history,
            show_close_button: true,
            ..Default::default()
        },
        hitboxes,
    );

    // Enregistrer les hitboxes pour les fichiers visibles
    let display_items: Vec<&(Line, Option<usize>)> = all_lines
        .iter()
        .skip(scroll)
        .take(inner.height as usize)
        .collect();

    for (row_i, (_, file_opt)) in display_items.iter().enumerate() {
        if let Some(file_idx) = file_opt {
            let row_y = inner.y + row_i as u16;
            if row_y < inner.y + inner.height {
                hitboxes.push(Hitbox {
                    rect: Rect {
                        x: inner.x,
                        y: row_y,
                        width: inner.width,
                        height: 1,
                    },
                    action: HitAction::HistoryFile(*file_idx),
                });
            }
        }
    }

    let display_lines: Vec<Line> = display_items.iter().map(|(l, _)| l.clone()).collect();
    let p = Paragraph::new(display_lines);
    f.render_widget(p, inner);

    crate::ui::render_btop_scrollbar(
        f,
        area,
        total_lines,
        scroll,
        visible_height,
        theme,
        hitboxes,
        crate::app::ScrollbarTarget::HistoryDetails(run_idx),
    );
}
