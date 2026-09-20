use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::App;
use crate::ui::theme::ThemePalette;

/// Renders the active synchronization status section with stepper, transfer KPIs, and two-column diffs.
pub fn render_active_sync_section(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect) {
    f.render_widget(Clear, area);

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
        "Remote Diffs",
        "Local Diffs",
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

    // 2. Dynamic Phase & Transfer KPIs
    let pct_str = format!("{}%", app.live.overall_progress_pct());
    let kpi_spans = if app.live.phase_index < 3 {
        // Research / Listings / Diffs phases: only show relevant discovery metrics
        let checks = format!("{} / {}", app.live.transfer.checks_done, app.live.transfer.checks_total);
        let (phase_name, status_text) = match app.live.phase_index {
            0 => ("Listings", "Building directory listings..."),
            1 => ("Remote Diffs (Path1)", "Checking differences on remote..."),
            _ => ("Local Diffs (Path2)", "Checking differences on local..."),
        };
        vec![
            Span::styled("Phase: ", Style::default().fg(theme.text_muted)),
            Span::styled(format!("{} ", phase_name), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("│ Checks: ", Style::default().fg(theme.text_muted)),
            Span::styled(format!("{} ", checks), Style::default().fg(theme.text_bright)),
            Span::styled("│ Progress: ", Style::default().fg(theme.text_muted)),
            Span::styled(format!("{} ", pct_str), Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
            Span::styled("│ Status: ", Style::default().fg(theme.text_muted)),
            Span::styled(status_text, Style::default().fg(theme.yellow)),
        ]
    } else if app.live.phase_index == 3 {
        // Applying / Copying phase: show transfer speed, volume, and files
        let done = if app.live.transfer.bytes_done.is_empty() { "0 B" } else { &app.live.transfer.bytes_done };
        let total = if app.live.transfer.bytes_total.is_empty() { "0 B" } else { &app.live.transfer.bytes_total };
        let speed = if app.live.transfer.speed.is_empty() { "0 B/s" } else { &app.live.transfer.speed };
        let files = format!("{} / {}", app.live.transfer.files_done, app.live.transfer.files_total);

        let mut spans = vec![
            Span::styled("Transferred: ", Style::default().fg(theme.text_muted)),
            Span::styled(format!("{} / {} ", done, total), Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)),
            Span::styled("│ Speed: ", Style::default().fg(theme.text_muted)),
            Span::styled(format!("{} ", speed), Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
            Span::styled("│ Files: ", Style::default().fg(theme.text_muted)),
            Span::styled(format!("{} ", files), Style::default().fg(theme.text_bright)),
            Span::styled("│ Progress: ", Style::default().fg(theme.text_muted)),
            Span::styled(pct_str, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        ];
        if !app.live.transfer.eta.is_empty() {
            spans.push(Span::styled(" │ ETA: ", Style::default().fg(theme.text_muted)));
            spans.push(Span::styled(&app.live.transfer.eta, Style::default().fg(theme.yellow)));
        }
        spans
    } else {
        // Updating listings / Done phase:
        let done = if app.live.transfer.bytes_done.is_empty() { "—" } else { &app.live.transfer.bytes_done };
        let files = format!("{} / {}", app.live.transfer.files_done, app.live.transfer.files_total);
        vec![
            Span::styled("Phase: ", Style::default().fg(theme.text_muted)),
            Span::styled("Updating Listings ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("│ Files: ", Style::default().fg(theme.text_muted)),
            Span::styled(format!("{} ", files), Style::default().fg(theme.text_bright)),
            Span::styled("│ Transferred: ", Style::default().fg(theme.text_muted)),
            Span::styled(format!("{} ", done), Style::default().fg(theme.text_bright)),
            Span::styled("│ Progress: ", Style::default().fg(theme.text_muted)),
            Span::styled(format!("{} ", pct_str), Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
            Span::styled("│ Status: ", Style::default().fg(theme.text_muted)),
            Span::styled("Finalizing and saving listings...", Style::default().fg(theme.yellow)),
        ]
    };

    let active_file = app.live.active_files.values().next();
    let has_active_file = active_file.is_some() && app.live.phase_index == 3;

    // Vertical layout:
    // 1. Phase stepper (1 line)
    // 2. Transfer KPIs (1 line)
    // 3. Active file progress bar (1 line, if present)
    // 4. Side-by-side local & remote diffs (Path2 on left, Path1 on right)
    let v_chunks = if has_active_file && inner.height >= 5 {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Stepper
                Constraint::Length(1), // KPIs
                Constraint::Length(1), // Active file bar
                Constraint::Min(1),    // Side-by-side changes
            ])
            .split(inner)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Stepper
                Constraint::Length(1), // KPIs
                Constraint::Min(1),    // Side-by-side changes
            ])
            .split(inner)
    };

    f.render_widget(Paragraph::new(Line::from(stepper_spans)), v_chunks[0]);
    f.render_widget(Paragraph::new(Line::from(kpi_spans)), v_chunks[1]);

    let diff_chunk = if has_active_file && inner.height >= 5 {
        if let Some(af) = active_file {
            let other_count = app.live.active_files.len().saturating_sub(1);
            let af_spans = format_active_file_spans(af, theme, inner.width, other_count);
            f.render_widget(Paragraph::new(Line::from(af_spans)), v_chunks[2]);
        }
        v_chunks[3]
    } else {
        v_chunks[2]
    };

    if diff_chunk.height > 0 {
        let h_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(diff_chunk);

        f.render_widget(Clear, h_chunks[0]);
        f.render_widget(Clear, h_chunks[1]);

        let col_width = h_chunks[0].width as usize;
        let max_p_len = col_width.saturating_sub(16).max(8);

        // Left column: Local changes (Path2)
        let is_loc_mod = app.live.path2_modified || !app.live.changes_local_details.is_empty() || !app.live.changes_local.is_empty();
        let loc_status_span = if is_loc_mod {
            Span::styled("● Modified", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD))
        } else {
            Span::styled("● No changes", Style::default().fg(theme.text_muted))
        };
        let mut loc_lines = vec![
            Line::from(vec![
                Span::styled("Local changes (Path2): ", Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD)),
                loc_status_span,
            ]),
        ];
        if app.live.changes_local_details.is_empty() && app.live.changes_local.is_empty() {
            if is_loc_mod {
                loc_lines.push(Line::from(Span::styled("  Changes detected", Style::default().fg(theme.text_muted))));
            } else {
                loc_lines.push(Line::from(Span::styled("  No changes", Style::default().fg(theme.text_muted))));
            }
        } else if !app.live.changes_local_details.is_empty() {
            for d in app.live.changes_local_details.iter().take(2) {
                let (badge_text, badge_color) = match d.action.to_lowercase().as_str() {
                    "new" | "added" | "ajouté" | "ajoute" | "copié" | "copie" => ("● Added", theme.green),
                    "deleted" | "supprimé" | "supprime" => ("● Deleted", theme.red),
                    _ => ("● Modified", theme.yellow),
                };
                let display_p = if d.path.chars().count() > max_p_len {
                    let skip = d.path.chars().count() - max_p_len + 1;
                    format!("…{}", d.path.chars().skip(skip).collect::<String>())
                } else {
                    d.path.clone()
                };
                loc_lines.push(Line::from(vec![
                    Span::styled(format!("  • {} ", display_p), Style::default().fg(theme.text_bright)),
                    Span::styled(badge_text, Style::default().fg(badge_color).add_modifier(Modifier::BOLD)),
                ]));
            }
            if app.live.changes_local_details.len() > 2 {
                loc_lines.push(Line::from(Span::styled(format!("  (+{} other(s))", app.live.changes_local_details.len() - 2), Style::default().fg(theme.text_muted))));
            }
        } else {
            for f_name in app.live.changes_local.iter().take(2) {
                let display_p = if f_name.chars().count() > max_p_len {
                    let skip = f_name.chars().count() - max_p_len + 1;
                    format!("…{}", f_name.chars().skip(skip).collect::<String>())
                } else {
                    f_name.clone()
                };
                loc_lines.push(Line::from(Span::styled(format!("  • {}", display_p), Style::default().fg(theme.text_bright))));
            }
            if app.live.changes_local.len() > 2 {
                loc_lines.push(Line::from(Span::styled(format!("  (+{} other(s))", app.live.changes_local.len() - 2), Style::default().fg(theme.text_muted))));
            }
        }
        f.render_widget(Paragraph::new(loc_lines), h_chunks[0]);

        // Right column: Remote changes (Path1)
        let is_rem_mod = app.live.path1_modified || !app.live.changes_remote_details.is_empty() || !app.live.changes_remote.is_empty();
        let rem_status_span = if is_rem_mod {
            Span::styled("● Modified", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD))
        } else {
            Span::styled("● No changes", Style::default().fg(theme.text_muted))
        };
        let mut rem_lines = vec![
            Line::from(vec![
                Span::styled("Remote changes (Path1): ", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)),
                rem_status_span,
            ]),
        ];
        if app.live.changes_remote_details.is_empty() && app.live.changes_remote.is_empty() {
            if is_rem_mod {
                rem_lines.push(Line::from(Span::styled("  Changes detected", Style::default().fg(theme.text_muted))));
            } else {
                rem_lines.push(Line::from(Span::styled("  No changes", Style::default().fg(theme.text_muted))));
            }
        } else if !app.live.changes_remote_details.is_empty() {
            for d in app.live.changes_remote_details.iter().take(2) {
                let (badge_text, badge_color) = match d.action.to_lowercase().as_str() {
                    "new" | "added" | "ajouté" | "ajoute" | "copié" | "copie" => ("● Added", theme.green),
                    "deleted" | "supprimé" | "supprime" => ("● Deleted", theme.red),
                    _ => ("● Modified", theme.yellow),
                };
                let display_p = if d.path.chars().count() > max_p_len {
                    let skip = d.path.chars().count() - max_p_len + 1;
                    format!("…{}", d.path.chars().skip(skip).collect::<String>())
                } else {
                    d.path.clone()
                };
                rem_lines.push(Line::from(vec![
                    Span::styled(format!("  • {} ", display_p), Style::default().fg(theme.text_bright)),
                    Span::styled(badge_text, Style::default().fg(badge_color).add_modifier(Modifier::BOLD)),
                ]));
            }
            if app.live.changes_remote_details.len() > 2 {
                rem_lines.push(Line::from(Span::styled(format!("  (+{} other(s))", app.live.changes_remote_details.len() - 2), Style::default().fg(theme.text_muted))));
            }
        } else {
            for f_name in app.live.changes_remote.iter().take(2) {
                let display_p = if f_name.chars().count() > max_p_len {
                    let skip = f_name.chars().count() - max_p_len + 1;
                    format!("…{}", f_name.chars().skip(skip).collect::<String>())
                } else {
                    f_name.clone()
                };
                rem_lines.push(Line::from(Span::styled(format!("  • {}", display_p), Style::default().fg(theme.text_bright))));
            }
            if app.live.changes_remote.len() > 2 {
                rem_lines.push(Line::from(Span::styled(format!("  (+{} other(s))", app.live.changes_remote.len() - 2), Style::default().fg(theme.text_muted))));
            }
        }
        f.render_widget(Paragraph::new(rem_lines), h_chunks[1]);
    }
}

fn format_active_file_spans<'a>(
    af: &'a crate::monitor::parser::ActiveFile,
    theme: &ThemePalette,
    max_width: u16,
    other_count: usize,
) -> Vec<Span<'a>> {
    let pct = af.pct.min(100);
    let bar_len = 14usize;
    let filled_len = (pct as usize * bar_len) / 100;
    let empty_len = bar_len.saturating_sub(filled_len);

    let filled_str: String = "█".repeat(filled_len);
    let empty_str: String = "░".repeat(empty_len);

    let speed_suffix = if !af.speed.is_empty() {
        format!(" ({})", af.speed)
    } else {
        String::new()
    };
    let other_suffix = if other_count > 0 {
        format!(" (+{} other{})", other_count, if other_count > 1 { "s" } else { "" })
    } else {
        String::new()
    };
    let pct_text = format!(" {}%{}", pct, speed_suffix);

    // Reserved width for prefix "  ⚡ Active: ", bar "[...]", pct_text, and other_suffix
    let fixed_width = 12 + (bar_len + 2) + pct_text.chars().count() + other_suffix.chars().count();
    let avail_for_name = (max_width as usize).saturating_sub(fixed_width).max(8);

    let display_name = if af.name.chars().count() > avail_for_name {
        let skip = af.name.chars().count() - avail_for_name + 1;
        format!("…{}", af.name.chars().skip(skip).collect::<String>())
    } else {
        af.name.clone()
    };

    let mut spans = vec![
        Span::styled("  ⚡ Active: ", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{} ", display_name), Style::default().fg(theme.text_bright)),
        Span::styled("[", Style::default().fg(theme.border)),
        Span::styled(filled_str, Style::default().fg(theme.green)),
        Span::styled(empty_str, Style::default().fg(theme.border)),
        Span::styled("]", Style::default().fg(theme.border)),
        Span::styled(pct_text, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
    ];
    if !other_suffix.is_empty() {
        spans.push(Span::styled(other_suffix, Style::default().fg(theme.text_muted)));
    }
    spans
}
