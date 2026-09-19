use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::App;
use crate::ui::theme::ThemePalette;

/// Renders the active synchronization status section with stepper, transfer KPIs, and two-column diffs.
pub fn render_active_sync_section(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect) {
    let elapsed = if app.live.transfer.elapsed.is_empty() {
        "0s"
    } else {
        &app.live.transfer.elapsed
    };

    let bg = app.border_glyphs();
    let title_line = Line::from(vec![
        Span::styled(bg.top_left, Style::default().fg(theme.accent)),
        Span::styled("synchronization in progress", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled(bg.top_right, Style::default().fg(theme.accent)),
    ]);

    let right_title = Line::from(vec![
        Span::styled("⏱ ", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{} ", elapsed), Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(app.border_type())
        .border_style(Style::default().fg(theme.accent))
        .style(Style::default().bg(theme.card_bg))
        .title(title_line)
        .title(right_title.alignment(Alignment::Right));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 2 {
        return;
    }

    // 1. Synchronization phase stepper
    let phases = [
        "Listings",
        "Local Diffs",
        "Remote Diffs",
        "Applying",
        "Updating",
        "Done",
    ];
    let mut stepper_spans = Vec::new();
    for (i, name) in phases.iter().enumerate() {
        if i > 0 {
            stepper_spans.push(Span::styled(" → ", Style::default().fg(theme.border)));
        }
        let (icon, style) = if i < app.live.phase_index {
            ("✓", Style::default().fg(theme.green).add_modifier(Modifier::BOLD))
        } else if i == app.live.phase_index {
            ("●", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))
        } else {
            ("○", Style::default().fg(theme.text_muted))
        };
        stepper_spans.push(Span::styled(format!("{} {}", icon, name), style));
    }

    // 2. Transfer KPIs grid
    let done = if app.live.transfer.bytes_done.is_empty() { "—" } else { &app.live.transfer.bytes_done };
    let total = if app.live.transfer.bytes_total.is_empty() { "—" } else { &app.live.transfer.bytes_total };
    let pct_str = format!("{}%", app.live.overall_progress_pct());
    let speed = if app.live.transfer.speed.is_empty() { "—" } else { &app.live.transfer.speed };
    let checks = format!("{} / {}", app.live.transfer.checks_done, app.live.transfer.checks_total);
    let files = format!("{} / {}", app.live.transfer.files_done, app.live.transfer.files_total);

    let mut kpi_spans = vec![
        Span::styled("Transferred: ", Style::default().fg(theme.text_muted)),
        Span::styled(format!("{} ", done), Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)),
        Span::styled("│ Total: ", Style::default().fg(theme.text_muted)),
        Span::styled(format!("{} ", total), Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)),
        Span::styled("│ Progress: ", Style::default().fg(theme.text_muted)),
        Span::styled(format!("{} ", pct_str), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled("│ Speed: ", Style::default().fg(theme.text_muted)),
        Span::styled(format!("{} ", speed), Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
        Span::styled("│ Checks: ", Style::default().fg(theme.text_muted)),
        Span::styled(format!("{} ", checks), Style::default().fg(theme.text_bright)),
        Span::styled("│ Files: ", Style::default().fg(theme.text_muted)),
        Span::styled(files, Style::default().fg(theme.text_bright)),
    ];

    // Active file being transferred
    if let Some((_key, af)) = app.live.active_files.iter().next() {
        kpi_spans.push(Span::styled("│ Active: ", Style::default().fg(theme.text_muted)));
        kpi_spans.push(Span::styled(format!("{} ({}%)", af.name, af.pct), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)));
    }

    // Vertical layout:
    // 1. Phase stepper (1 line)
    // 2. Transfer KPIs (1 line)
    // 3. Side-by-side local & remote diffs (Path2 on left, Path1 on right)
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Stepper
            Constraint::Length(1), // KPIs
            Constraint::Min(1),    // Side-by-side changes
        ])
        .split(inner);

    f.render_widget(Paragraph::new(Line::from(stepper_spans)), v_chunks[0]);
    f.render_widget(Paragraph::new(Line::from(kpi_spans)), v_chunks[1]);

    if v_chunks[2].height > 0 {
        let h_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(v_chunks[2]);

        // Left column: Local changes (Path2)
        let mut loc_lines = vec![
            Line::from(Span::styled("Local changes (Path2):", Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD))),
        ];
        if app.live.changes_local_details.is_empty() && app.live.changes_local.is_empty() {
            loc_lines.push(Line::from(Span::styled("  No changes", Style::default().fg(theme.text_muted))));
        } else if !app.live.changes_local_details.is_empty() {
            for d in app.live.changes_local_details.iter().take(2) {
                let (badge_text, badge_color) = match d.action.to_lowercase().as_str() {
                    "new" | "added" | "ajouté" | "ajoute" | "copié" | "copie" => ("● Added", theme.green),
                    "deleted" | "supprimé" | "supprime" => ("● Deleted", theme.red),
                    _ => ("● Modified", theme.yellow),
                };
                loc_lines.push(Line::from(vec![
                    Span::styled(format!("  • {} ", d.path), Style::default().fg(theme.text_bright)),
                    Span::styled(badge_text, Style::default().fg(badge_color).add_modifier(Modifier::BOLD)),
                ]));
            }
            if app.live.changes_local_details.len() > 2 {
                loc_lines.push(Line::from(Span::styled(format!("  (+{} other(s))", app.live.changes_local_details.len() - 2), Style::default().fg(theme.text_muted))));
            }
        } else {
            for f in app.live.changes_local.iter().take(2) {
                loc_lines.push(Line::from(Span::styled(format!("  • {}", f), Style::default().fg(theme.text_bright))));
            }
            if app.live.changes_local.len() > 2 {
                loc_lines.push(Line::from(Span::styled(format!("  (+{} other(s))", app.live.changes_local.len() - 2), Style::default().fg(theme.text_muted))));
            }
        }
        f.render_widget(Paragraph::new(loc_lines), h_chunks[0]);

        // Right column: Remote changes (Path1)
        let mut rem_lines = vec![
            Line::from(Span::styled("Remote changes (Path1):", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD))),
        ];
        if app.live.changes_remote_details.is_empty() && app.live.changes_remote.is_empty() {
            rem_lines.push(Line::from(Span::styled("  No changes", Style::default().fg(theme.text_muted))));
        } else if !app.live.changes_remote_details.is_empty() {
            for d in app.live.changes_remote_details.iter().take(2) {
                let (badge_text, badge_color) = match d.action.to_lowercase().as_str() {
                    "new" | "added" | "ajouté" | "ajoute" | "copié" | "copie" => ("● Added", theme.green),
                    "deleted" | "supprimé" | "supprime" => ("● Deleted", theme.red),
                    _ => ("● Modified", theme.yellow),
                };
                rem_lines.push(Line::from(vec![
                    Span::styled(format!("  • {} ", d.path), Style::default().fg(theme.text_bright)),
                    Span::styled(badge_text, Style::default().fg(badge_color).add_modifier(Modifier::BOLD)),
                ]));
            }
            if app.live.changes_remote_details.len() > 2 {
                rem_lines.push(Line::from(Span::styled(format!("  (+{} other(s))", app.live.changes_remote_details.len() - 2), Style::default().fg(theme.text_muted))));
            }
        } else {
            for f in app.live.changes_remote.iter().take(2) {
                rem_lines.push(Line::from(Span::styled(format!("  • {}", f), Style::default().fg(theme.text_bright))));
            }
            if app.live.changes_remote.len() > 2 {
                rem_lines.push(Line::from(Span::styled(format!("  (+{} other(s))", app.live.changes_remote.len() - 2), Style::default().fg(theme.text_muted))));
            }
        }
        f.render_widget(Paragraph::new(rem_lines), h_chunks[1]);
    }
}
