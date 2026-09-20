use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::{App, Hitbox, Modal};
use crate::ui::container::{centered_fixed_rect, centered_rect, render_modal_container, ModalContainerConfig, NavArrowsConfig};
use crate::ui::keys::KeybindingRegistry;
use crate::ui::files::render_files_modal;
use crate::ui::filters::render_filters_modal;
use crate::ui::history::render_history_details_modal;
use crate::ui::menu::render_menu_modal;
use crate::ui::settings::render_settings_modal;
use crate::ui::theme::ThemePalette;

pub fn render_popups(f: &mut Frame, app: &App, theme: &ThemePalette, hitboxes: &mut Vec<Hitbox>) {
    // 1. Modales
    match &app.modal {
        Modal::FirstRun(state) => {
            crate::ui::first_run::render_first_run_modal(f, app, state, theme, hitboxes);
        }
        Modal::Menu => {
            render_menu_modal(f, app, theme, hitboxes);
        }
        Modal::Settings => {
            render_settings_modal(f, app, theme, hitboxes);
        }
        Modal::Files => {
            render_files_modal(f, app, theme, hitboxes);
        }
        Modal::Filters => {
            render_filters_modal(f, app, theme, hitboxes);
        }
        Modal::HistoryDetails(idx) => {
            render_history_details_modal(f, app, *idx, theme, hitboxes);
        }
        Modal::DryRun => {
            render_dry_run_modal(f, app, theme, hitboxes);
        }
        Modal::ConfirmSync => {
            let area = centered_rect(58, 25, f.area());
            let inner = render_modal_container(
                f,
                app,
                theme,
                area,
                ModalContainerConfig {
                    title_prefix: " Synchronization ",
                    border_color: theme.accent,
                    show_close_button: true,
                    ..Default::default()
                },
                hitboxes,
            );

            let text = vec![
                Line::from(Span::styled("TRIGGER SYNCHRONIZATION NOW?", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))),
                Line::from(""),
                Line::from("This action immediately triggers the rclone-bisync service"),
                Line::from("to synchronize all local and remote changes."),
                Line::from(""),
                Line::from(vec![
                    Span::styled(" [Y / Enter] Confirm ", Style::default().fg(theme.card_bg).bg(theme.green).add_modifier(Modifier::BOLD)),
                    Span::styled("    ", Style::default()),
                    Span::styled(" [N / Esc] Cancel ", Style::default().fg(theme.text_bright).bg(theme.border)),
                ]),
            ];

            let p = Paragraph::new(text).alignment(Alignment::Center);
            f.render_widget(p, inner);
        }

        Modal::ConfirmResync => {
            let area = centered_rect(60, 30, f.area());
            let inner = render_modal_container(
                f,
                app,
                theme,
                area,
                ModalContainerConfig {
                    title_prefix: " Confirmation required ",
                    border_color: theme.yellow,
                    show_close_button: true,
                    ..Default::default()
                },
                hitboxes,
            );

            let text = vec![
                Line::from(Span::styled("WARNING: FULL RESYNCHRONIZATION", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD))),
                Line::from(""),
                Line::from("This operation will rebuild local and remote listings"),
                Line::from("with the --resync flag to resolve a conflict or inconsistency."),
                Line::from(""),
                Line::from(Span::styled("Do you want to run this command now?", Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD))),
                Line::from(""),
                Line::from(vec![
                    Span::styled(" [Y] Yes / Confirm ", Style::default().fg(theme.card_bg).bg(theme.red).add_modifier(Modifier::BOLD)),
                    Span::styled("    ", Style::default()),
                    Span::styled(" [N] No / Cancel (Esc) ", Style::default().fg(theme.text_bright).bg(theme.border)),
                ]),
            ];

            let p = Paragraph::new(text).alignment(Alignment::Center);
            f.render_widget(p, inner);
        }
        Modal::ConfirmCancel => {
            let area = centered_rect(55, 25, f.area());
            let inner = render_modal_container(
                f,
                app,
                theme,
                area,
                ModalContainerConfig {
                    title_prefix: " Force Stop ",
                    border_color: theme.red,
                    show_close_button: true,
                    ..Default::default()
                },
                hitboxes,
            );

            let text = vec![
                Line::from(Span::styled("ABORT SYNCHRONIZATION?", Style::default().fg(theme.red).add_modifier(Modifier::BOLD))),
                Line::from(""),
                Line::from("The active rclone-bisync process will be terminated immediately."),
                Line::from(""),
                Line::from(vec![
                    Span::styled(" [Y] Yes, abort ", Style::default().fg(theme.card_bg).bg(theme.red).add_modifier(Modifier::BOLD)),
                    Span::styled("    ", Style::default()),
                    Span::styled(" [N] Continue (Esc) ", Style::default().fg(theme.text_bright).bg(theme.border)),
                ]),
            ];

            let p = Paragraph::new(text).alignment(Alignment::Center);
            f.render_widget(p, inner);
        }
        Modal::ConfirmDelete(rel_path) => {
            let area = centered_rect(55, 25, f.area());
            let inner = render_modal_container(
                f,
                app,
                theme,
                area,
                ModalContainerConfig {
                    title_prefix: " Deletion ",
                    border_color: theme.red,
                    show_close_button: true,
                    ..Default::default()
                },
                hitboxes,
            );

            let text = vec![
                Line::from(Span::styled("DELETE THIS LOCAL FILE?", Style::default().fg(theme.red).add_modifier(Modifier::BOLD))),
                Line::from(""),
                Line::from(Span::styled(rel_path.as_str(), Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD))),
                Line::from(""),
                Line::from(vec![
                    Span::styled(" [Y] Delete ", Style::default().fg(theme.card_bg).bg(theme.red).add_modifier(Modifier::BOLD)),
                    Span::styled("    ", Style::default()),
                    Span::styled(" [N] Cancel (Esc) ", Style::default().fg(theme.text_bright).bg(theme.border)),
                ]),
            ];

            let p = Paragraph::new(text).alignment(Alignment::Center);
            f.render_widget(p, inner);
        }
        Modal::Help => {
            let screen = f.area();
            let is_wide = screen.width >= 86;
            let logo_h: u16 = if is_wide { 6 } else { 5 };
            let box_w = if is_wide { 86.min(screen.width) } else { 78.min(screen.width) };
            let box_h = 24.min(screen.height.saturating_sub(logo_h + 3));

            let total_h = logo_h + 1 + box_h;
            let container_area = centered_fixed_rect(box_w, total_h, screen);

            let v_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(logo_h),
                    Constraint::Length(1),
                    Constraint::Length(box_h),
                ])
                .split(container_area);

            // 1. Logo 3D RCLONEDASH au-dessus de la boîte d'aide
            crate::ui::menu::render_logo(f, v_chunks[0]);

            // Version
            let mut ver_spans = vec![
                Span::styled(format!("v{}", crate::config::APP_VERSION), Style::default().fg(Color::Rgb(165, 170, 185)).add_modifier(Modifier::BOLD | Modifier::ITALIC)),
            ];
            if let Some(newer) = &app.available_update {
                ver_spans.push(Span::raw("  "));
                ver_spans.push(Span::styled(
                    format!("(🚀 v{} available)", newer),
                    Style::default().fg(Color::Rgb(250, 200, 50)).add_modifier(Modifier::BOLD),
                ));
            }
            let ver_line = Line::from(ver_spans);
            f.render_widget(Paragraph::new(ver_line).alignment(Alignment::Center), v_chunks[1]);

            // 2. Boîte d'aide
            let help_box_area = v_chunks[2];
            let inner = render_modal_container(
                f,
                app,
                theme,
                help_box_area,
                ModalContainerConfig {
                    title_prefix: "help",
                    border_color: theme.red,
                    show_close_button: true,
                    ..Default::default()
                },
                hitboxes,
            );

            let help_items = [
                ("Mouse 1", "Clicks buttons and selects in lists/panels."),
                ("Mouse scroll", "Scrolls any scrollable list/logs under cursor."),
                ("Esc, m", "Toggles main menu / Closes active modal."),
                ("q", "Closes active modal / In dashboard: quits app."),
                ("o", "Shows options / settings panel."),
                ("F1, ?, h", "Shows this help window."),
                ("s", "Triggers bisync synchronization (confirmation)."),
                ("d", "Runs bisync dry-run simulation (confirmation)."),
                ("r", "Full resync (--resync) repair mode (confirmation)."),
                ("c", "Cancels active synchronization run."),
                ("b, p", "Opens file browser modal (explorer)."),
                ("f, /", "In Recent files: search filter."),
                ("e", "Opens exclusion rules editor (gdrive-filters.txt)."),
                ("Ctrl+X", "Toggles parent directory mode (shows folder paths)."),
                ("Enter", "Opens selected file / Validates actions."),
                ("Ctrl+Enter", "Opens containing folder in system file manager (xdg)."),
                ("Spacebar", "Pauses / resumes logs auto-scroll."),
                ("y", "Copies focused panel (logs, history errors, or file)."),
                ("+ , -", "Speeds up / slows down UI tick rate interval."),
                ("Tab, Shift+Tab", "Cycles active dashboard panel focus."),
                ("t", "Cycles color theme."),
                ("ctrl + c", "Force terminates the program."),
            ];

            let mut lines = Vec::new();
            // Ligne d'en-tête
            lines.push(Line::from(vec![
                Span::styled("   Key:                 ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::styled("Description:", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]));
            lines.push(Line::from(""));

            for (k, desc) in help_items {
                lines.push(Line::from(vec![
                    Span::styled(format!("   {:20} ", k), Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
                    Span::styled(desc, Style::default().fg(Color::Rgb(215, 220, 230))),
                ]));
            }

            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled("   For bug reporting and project updates, visit:", Style::default().fg(theme.text_muted))));
            lines.push(Line::from(Span::styled("   https://github.com/lucas-martinati/rcloneDash", Style::default().fg(theme.cyan).add_modifier(Modifier::UNDERLINED))));

            let p = Paragraph::new(lines).alignment(Alignment::Left);
            f.render_widget(p, inner);
        }
        Modal::None => {}
    }

    // 2. Toast notification flottant (rendu AU-DESSUS des modales pour être toujours visible)
    if let Some((msg, _)) = &app.toast {
        let area = f.area();
        let toast_width = (msg.len() as u16 + 6).min(area.width.saturating_sub(4));
        let toast_area = Rect {
            x: area.width.saturating_sub(toast_width + 2),
            y: 1,
            width: toast_width,
            height: 3,
        };

        f.render_widget(Clear, toast_area);
        let toast_p = Paragraph::new(Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled(msg, Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(app.border_type())
                .border_style(Style::default().fg(theme.accent))
                .style(Style::default().bg(theme.card_bg)),
        );
        f.render_widget(toast_p, toast_area);
    }
}

#[derive(Debug, Clone, Default)]
struct DryRunSummary {
    path2_new: usize,
    path2_modified: usize,
    path2_deleted: usize,
    path1_new: usize,
    path1_modified: usize,
    path1_deleted: usize,
    checks: usize,
    elapsed: String,
    bytes: String,
    has_errors: bool,
}

impl DryRunSummary {
    fn from_logs(logs: &[String]) -> Self {
        let mut s = Self::default();

        for raw_line in logs {
            let line = crate::monitor::streamer::strip_ansi(raw_line);
            let ll = line.to_ascii_lowercase();

            if ll.contains("error:") || ll.contains("fatal:") || ll.contains("error running rclone") {
                s.has_errors = true;
            }

            if let Some(pos) = ll.find("checks:") {
                let rest = &ll[pos + 7..];
                if let Some(slash_pos) = rest.find('/') {
                    let done_str = rest[..slash_pos].trim();
                    if let Ok(c) = done_str.parse::<usize>() {
                        s.checks = c;
                    }
                }
            }

            if let Some(pos) = ll.find("elapsed time:") {
                let rest = line[pos + 13..].trim();
                s.elapsed = rest.to_string();
            }

            if ll.contains("transferred:") && (ll.contains("b /") || ll.contains("ib /")) {
                if let Some(pos) = ll.find("transferred:") {
                    let rest = line[pos + 12..].trim();
                    if let Some(comma_pos) = rest.find(',') {
                        s.bytes = rest[..comma_pos].trim().to_string();
                    }
                }
            }

            if ll.contains("path1") || ll.contains("path2") {
                let is_local = ll.contains("path2:") || ll.contains("- path2");
                let is_remote = ll.contains("path1:") || ll.contains("- path1");

                if is_local {
                    if ll.contains("file is new") {
                        s.path2_new += 1;
                    } else if ll.contains("file changed") {
                        s.path2_modified += 1;
                    } else if ll.contains("file was deleted") || ll.contains("file deleted") {
                        s.path2_deleted += 1;
                    }
                } else if is_remote {
                    if ll.contains("file is new") {
                        s.path1_new += 1;
                    } else if ll.contains("file changed") {
                        s.path1_modified += 1;
                    } else if ll.contains("file was deleted") || ll.contains("file deleted") {
                        s.path1_deleted += 1;
                    }
                }
            }
        }

        s
    }

    fn total_changes(&self) -> usize {
        self.path1_new + self.path1_modified + self.path1_deleted +
        self.path2_new + self.path2_modified + self.path2_deleted
    }
}

fn render_dry_run_modal(f: &mut Frame, app: &App, theme: &ThemePalette, hitboxes: &mut Vec<Hitbox>) {
    let area = centered_rect(82, 85, f.area());

    let summary = DryRunSummary::from_logs(&app.dry_run_logs);

    let status_str = if app.dry_run_running {
        "⏳ Simulation in progress..."
    } else if app.dry_run_logs.is_empty() {
        "○ Ready to simulate"
    } else if summary.has_errors {
        "⚠ Finished with errors"
    } else {
        "✔ Simulation completed"
    };

    let total_lines = app.dry_run_logs.len();

    // Approximate log height for outer scroll arrows/counter
    let approx_log_height = area.height.saturating_sub(10) as usize;
    let approx_max_scroll = total_lines.saturating_sub(approx_log_height);
    let scroll = app.dry_run_scroll.min(approx_max_scroll);

    let action_shortcuts = if app.dry_run_running {
        None
    } else if app.dry_run_logs.is_empty() {
        Some(vec![
            KeybindingRegistry::format_shortcut_label("↵, r", "start", theme.green, Color::White),
        ])
    } else {
        Some(vec![
            KeybindingRegistry::format_shortcut_label("r", "rerun", theme.green, Color::White),
        ])
    };

    let inner = render_modal_container(
        f,
        app,
        theme,
        area,
        ModalContainerConfig {
            title_prefix: "🛡 Dry-Run Simulation",
            title_color: Some(theme.cyan),
            title_extra: Some(vec![
                Span::styled(format!(" │ {} ", status_str), Style::default().fg(theme.cyan)),
            ]),
            nav_arrows: Some(NavArrowsConfig {
                label: "scroll",
                up_active: scroll > 0,
                down_active: scroll < approx_max_scroll,
            }),
            action_shortcuts,
            counter: Some((scroll + 1, total_lines.max(1))),
            border_color: theme.cyan,
            show_close_button: true,
            ..Default::default()
        },
        hitboxes,
    );

    if inner.height < 4 {
        return;
    }

    // Split inner into:
    // 1. Summary Card (5 lines if height >= 14, else 3 lines)
    // 2. Execution Logs (remainder)
    let summary_height = if inner.height >= 14 { 5 } else { 3 };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(summary_height),
            Constraint::Min(4),
        ])
        .split(inner);

    // 1. Render Summary Card
    f.render_widget(Clear, chunks[0]);
    let summary_block = Block::default()
        .borders(Borders::ALL)
        .border_type(app.border_type())
        .border_style(Style::default().fg(theme.border))
        .title(Line::from(vec![
            Span::styled(" 📊 ", Style::default().fg(theme.cyan)),
            Span::styled("Simulation Summary", Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)),
            Span::styled(" ", Style::default()),
        ]));

    let s_inner = summary_block.inner(chunks[0]);
    f.render_widget(summary_block, chunks[0]);
    f.render_widget(Clear, s_inner);

    if s_inner.height > 0 {
        let mut summary_lines = Vec::new();

        if app.dry_run_logs.is_empty() && !app.dry_run_running {
            // Line 1: Ready status
            summary_lines.push(Line::from(vec![
                Span::styled("○ ", Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD)),
                Span::styled("Ready to simulate: ", Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD)),
                Span::styled("Non-destructive preview without modifying any files.", Style::default().fg(theme.text_bright)),
            ]));

            // Line 2: Scope description
            if s_inner.height >= 2 {
                summary_lines.push(Line::from(vec![
                    Span::styled("💻 Local (Path 2) ", Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD)),
                    Span::styled("⇄ ", Style::default().fg(theme.text_muted)),
                    Span::styled("☁ Remote (Path 1)", Style::default().fg(theme.blue).add_modifier(Modifier::BOLD)),
                    Span::styled("   │   Filter rules and rclone config active", Style::default().fg(theme.text_muted)),
                ]));
            }


        } else {
            // Line 1: Overall status banner
            let status_spans = if app.dry_run_running {
                vec![
                    Span::styled("● ", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)),
                    Span::styled("Simulation in progress: ", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)),
                    Span::styled("rclone bisync --dry-run analyzing differences...", Style::default().fg(theme.text_bright)),
                ]
            } else if summary.has_errors {
                vec![
                    Span::styled("✖ ", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
                    Span::styled("Errors detected during simulation ", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
                    Span::styled("(see details in execution logs below)", Style::default().fg(theme.text_muted)),
                ]
            } else if summary.total_changes() == 0 {
                vec![
                    Span::styled("✔ ", Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
                    Span::styled("Folders are in sync: ", Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
                    Span::styled("No differences detected between Local & Remote.", Style::default().fg(theme.text_bright)),
                ]
            } else {
                let p2_tot = summary.path2_new + summary.path2_modified + summary.path2_deleted;
                let p1_tot = summary.path1_new + summary.path1_modified + summary.path1_deleted;
                vec![
                    Span::styled("⚡ ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("{} planned difference(s) detected: ", summary.total_changes()), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("{} local action(s), {} remote action(s)", p2_tot, p1_tot), Style::default().fg(theme.text_bright)),
                ]
            };
            summary_lines.push(Line::from(status_spans));

            // Line 2: Breakdown by path
            if s_inner.height >= 2 {
                summary_lines.push(Line::from(vec![
                    Span::styled("💻 Local (Path 2): ", Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("+{} new ", summary.path2_new), Style::default().fg(if summary.path2_new > 0 { theme.green } else { theme.text_muted }).add_modifier(Modifier::BOLD)),
                    Span::styled("│ ", Style::default().fg(theme.border)),
                    Span::styled(format!("~{} mod ", summary.path2_modified), Style::default().fg(if summary.path2_modified > 0 { theme.yellow } else { theme.text_muted })),
                    Span::styled("│ ", Style::default().fg(theme.border)),
                    Span::styled(format!("-{} del", summary.path2_deleted), Style::default().fg(if summary.path2_deleted > 0 { theme.red } else { theme.text_muted })),
                    Span::styled("   │   ", Style::default().fg(theme.border)),
                    Span::styled("☁ Remote (Path 1): ", Style::default().fg(theme.blue).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("+{} new ", summary.path1_new), Style::default().fg(if summary.path1_new > 0 { theme.green } else { theme.text_muted }).add_modifier(Modifier::BOLD)),
                    Span::styled("│ ", Style::default().fg(theme.border)),
                    Span::styled(format!("~{} mod ", summary.path1_modified), Style::default().fg(if summary.path1_modified > 0 { theme.yellow } else { theme.text_muted })),
                    Span::styled("│ ", Style::default().fg(theme.border)),
                    Span::styled(format!("-{} del", summary.path1_deleted), Style::default().fg(if summary.path1_deleted > 0 { theme.red } else { theme.text_muted })),
                ]));
            }

            // Line 3: Meta checks & volume
            if s_inner.height >= 3 {
                let elapsed_str = if summary.elapsed.is_empty() { "—" } else { &summary.elapsed };
                let bytes_str = if summary.bytes.is_empty() { "0 B" } else { &summary.bytes };
                summary_lines.push(Line::from(vec![
                    Span::styled("Checks: ", Style::default().fg(theme.text_muted)),
                    Span::styled(format!("{} ", summary.checks), Style::default().fg(theme.text_bright)),
                    Span::styled("│ Volume: ", Style::default().fg(theme.text_muted)),
                    Span::styled(format!("{} ", bytes_str), Style::default().fg(theme.text_bright)),
                    Span::styled("│ Elapsed: ", Style::default().fg(theme.text_muted)),
                    Span::styled(format!("{} ", elapsed_str), Style::default().fg(theme.yellow)),
                ]));
            }
        }

        f.render_widget(Paragraph::new(summary_lines), s_inner);
    }

    // 2. Render Execution Logs
    f.render_widget(Clear, chunks[1]);
    let logs_block = Block::default()
        .borders(Borders::ALL)
        .border_type(app.border_type())
        .border_style(Style::default().fg(theme.border))
        .title(Line::from(vec![
            Span::styled(" 📜 ", Style::default().fg(theme.cyan)),
            Span::styled("Execution Logs", Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)),
            Span::styled(" ", Style::default()),
        ]));

    let logs_inner = logs_block.inner(chunks[1]);
    f.render_widget(logs_block, chunks[1]);
    f.render_widget(Clear, logs_inner);

    let actual_visible_height = logs_inner.height as usize;
    let actual_max_scroll = total_lines.saturating_sub(actual_visible_height);
    let actual_scroll = app.dry_run_scroll.min(actual_max_scroll);

    let lines: Vec<Line> = if app.dry_run_logs.is_empty() {
        if app.dry_run_running {
            vec![Line::from(Span::styled("Initializing dry-run simulation...", Style::default().fg(theme.text_muted)))]
        } else {
            vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("  🛡  ", Style::default().fg(theme.cyan)),
                    Span::styled("DRY-RUN SIMULATION", Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)),
                    Span::styled(" — Bidirectional Sync Preview", Style::default().fg(theme.text_muted)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("  • ", Style::default().fg(theme.cyan)),
                    Span::styled("Executes ", Style::default().fg(theme.text_bright)),
                    Span::styled("rclone bisync --dry-run", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)),
                    Span::styled(" with your active configuration and filters.", Style::default().fg(theme.text_bright)),
                ]),
                Line::from(vec![
                    Span::styled("  • ", Style::default().fg(theme.cyan)),
                    Span::styled("Compares Local (Path 2) and Remote (Path 1) safely without modifying any files.", Style::default().fg(theme.text_muted)),
                ]),
                Line::from(vec![
                    Span::styled("  • ", Style::default().fg(theme.cyan)),
                    Span::styled("Planned additions, modifications, and deletions will be listed here in real-time.", Style::default().fg(theme.text_muted)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("  ➔ Press ", Style::default().fg(theme.text_muted)),
                    Span::styled("[Enter]", Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
                    Span::styled(" or ", Style::default().fg(theme.text_muted)),
                    Span::styled("[r]", Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
                    Span::styled(" to start simulation", Style::default().fg(theme.text_muted)),
                ]),
            ]
        }
    } else {
        app.dry_run_logs
            .iter()
            .skip(actual_scroll)
            .take(actual_visible_height)
            .map(|l| crate::ui::logs::colorize_log_line(l, theme))
            .collect()
    };

    f.render_widget(Paragraph::new(lines), logs_inner);

    if !app.dry_run_logs.is_empty() {
        crate::ui::render_scrollbar(
            f,
            logs_inner,
            total_lines,
            actual_scroll,
            actual_visible_height,
            theme,
            hitboxes,
            crate::app::ScrollbarTarget::DryRun,
        );
    }
}
