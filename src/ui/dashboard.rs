use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Gauge, Paragraph, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Table,
    },
    Frame,
};

use crate::app::{Alert, AlertLevel, App, FocusedPanel, HitAction, Hitbox};
use crate::monitor::history::RunStatus;
use crate::ui::theme::ThemePalette;

pub fn render_dashboard(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect, hitboxes: &mut Vec<Hitbox>) {
    let alerts = app.active_alerts();
    let show_alert = !alerts.is_empty();
    let show_active_sync = app.live.is_syncing || app.live.transfer.pct > 0;
    let show_active_files = show_active_sync && !app.live.active_files.is_empty();

    let mut constraints = Vec::new();
    // 1. Ligne des KPI Cards
    constraints.push(Constraint::Length(3));

    // 2. Bannière d'alerte contextuelle si alerte active
    if show_alert {
        constraints.push(Constraint::Length(3));
    }

    // 3. Barre de sync active (Phase stepper + Jauge)
    if show_active_sync {
        constraints.push(Constraint::Length(3));
    }

    // 4. Fichiers en cours de transfert
    if show_active_files {
        constraints.push(Constraint::Length(3));
    }

    // 5. Zone centrale (Historique + Logs)
    constraints.push(Constraint::Percentage(52));

    // 6. Zone inférieure (Fichiers récents)
    constraints.push(Constraint::Min(6));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let mut chunk_idx = 0;

    // 1. KPI Cards
    render_kpi_bar(f, app, theme, chunks[chunk_idx]);
    chunk_idx += 1;

    // 2. Alerte
    if show_alert {
        render_alert_banner(f, &alerts[0], theme, chunks[chunk_idx]);
        chunk_idx += 1;
    }

    // 3. Sync active
    if show_active_sync {
        render_active_sync_section(f, app, theme, chunks[chunk_idx]);
        chunk_idx += 1;
    }

    // 4. Fichiers actifs
    if show_active_files {
        render_active_files_bar(f, app, theme, chunks[chunk_idx]);
        chunk_idx += 1;
    }

    // 5. Milieu : Historique + Logs
    let middle_area = chunks[chunk_idx];
    chunk_idx += 1;
    let bottom_area = chunks[chunk_idx];

    let middle_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(middle_area);

    render_history_panel(f, app, theme, middle_chunks[0], hitboxes);
    render_logs_panel(f, app, theme, middle_chunks[1], hitboxes);

    // 6. Fichiers récents
    render_recent_files_panel(f, app, theme, bottom_area, hitboxes);
}

fn render_kpi_bar(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect) {
    let kpi_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(14), // Cloud
            Constraint::Percentage(16), // Disque Local
            Constraint::Percentage(14), // Fichiers
            Constraint::Percentage(14), // Syncs Auj.
            Constraint::Percentage(14), // Vitesse
            Constraint::Percentage(14), // Conflits Auj.
            Constraint::Percentage(14), // Fiabilité
        ])
        .split(area);

    // KPI 1 : Stockage Cloud
    let cloud_sub = format!("Filet: {}", app.service_info.cloud_safety_net);
    render_kpi_card(
        f,
        kpi_chunks[0],
        "☁ STOCKAGE CLOUD",
        &app.config.remote,
        &cloud_sub,
        theme.purple,
        theme,
    );

    // KPI 2 : Disque Local (statvfs)
    let (used_gb, free_gb, total_gb, _pct) = app.local_disk_stats();
    let disk_val = format!("{:.0} Go", used_gb);
    let disk_sub = format!("{:.1} Go libres / {:.0} Go", free_gb, total_gb);
    render_kpi_card(
        f,
        kpi_chunks[1],
        "💾 DISQUE LOCAL",
        &disk_val,
        &disk_sub,
        theme.blue,
        theme,
    );

    // KPI 3 : Fichiers suivis
    let count = if !app.file_entries.is_empty() {
        app.file_entries.iter().filter(|f| !f.is_dir).count()
    } else {
        22224
    };
    let count_str = format!("{}", count);
    render_kpi_card(
        f,
        kpi_chunks[2],
        "📁 FICHIERS SUIVIS",
        &count_str,
        &app.config.local_dir,
        theme.cyan,
        theme,
    );

    // KPI 4 : Syncs aujourd'hui
    let (ok_today, err_today) = app.runs_today_stats();
    let total_today = ok_today + err_today;
    let syncs_val = format!("{}", total_today);
    let syncs_sub = format!("{} réussie(s) · {} err.", ok_today, err_today);
    render_kpi_card(
        f,
        kpi_chunks[3],
        "🔄 SYNCS AUJOURD'HUI",
        &syncs_val,
        &syncs_sub,
        theme.accent,
        theme,
    );

    // KPI 5 : Vitesse moyenne / Débit
    let speed = if app.live.transfer.speed.is_empty() {
        "0.0 KiB/s".to_string()
    } else {
        app.live.transfer.speed.clone()
    };
    render_kpi_card(
        f,
        kpi_chunks[4],
        "⚡ DÉBIT EN DIRECT",
        &speed,
        "transfert rclone",
        theme.green,
        theme,
    );

    // KPI 6 : Conflits aujourd'hui
    let conflicts = app.conflicts_today_stats();
    let conflicts_val = format!("{}", conflicts);
    let conflicts_color = if conflicts == 0 { theme.green } else { theme.red };
    render_kpi_card(
        f,
        kpi_chunks[5],
        "⚠ CONFLITS AUJOURD'HUI",
        &conflicts_val,
        if conflicts == 0 { "aucun conflit" } else { "détectés dans les logs" },
        conflicts_color,
        theme,
    );

    // KPI 7 : Fiabilité 7 jours
    let (rate, _) = app.calculate_success_rate();
    let rate_val = format!("{:.0} %", rate);
    render_kpi_card(
        f,
        kpi_chunks[6],
        "🛡 FIABILITÉ 7 JOURS",
        &rate_val,
        "taux de réussite",
        if rate >= 90.0 { theme.green } else { theme.yellow },
        theme,
    );
}

fn render_kpi_card(
    f: &mut Frame,
    area: Rect,
    title: &str,
    value: &str,
    subtext: &str,
    accent_color: ratatui::style::Color,
    theme: &ThemePalette,
) {
    let lines = vec![
        Line::from(vec![
            Span::styled(format!(" {} ", value), Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)),
            Span::styled(format!(" {}", subtext), Style::default().fg(theme.text_muted)),
        ]),
    ];

    let p = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.card_bg))
            .title(Span::styled(format!(" {} ", title), Style::default().fg(accent_color).add_modifier(Modifier::BOLD))),
    );

    f.render_widget(p, area);
}

fn render_alert_banner(f: &mut Frame, alert: &Alert, theme: &ThemePalette, area: Rect) {
    let (border_color, icon) = match alert.level {
        AlertLevel::Error => (theme.red, " 🚨 ALERTE CRITIQUE : "),
        AlertLevel::Warning => (theme.yellow, " ⚠️ AVERTISSEMENT : "),
    };

    let mut spans = vec![
        Span::styled(icon, Style::default().fg(border_color).add_modifier(Modifier::BOLD)),
        Span::styled(&alert.message, Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)),
    ];

    if let Some(hint) = &alert.hint {
        spans.push(Span::styled("  │  ", Style::default().fg(theme.border)));
        spans.push(Span::styled(hint, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)));
    }

    let p = Paragraph::new(Line::from(spans))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color))
                .style(Style::default().bg(theme.card_bg)),
        );

    f.render_widget(p, area);
}

fn render_active_sync_section(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(38), // Phase Stepper
            Constraint::Percentage(38), // Jauge de transfert
            Constraint::Percentage(24), // Sparkline débit réseau
        ])
        .split(area);

    render_phase_stepper(f, app, theme, chunks[0]);
    render_transfer_gauge(f, app, theme, chunks[1]);
    crate::ui::sparkline::render_speed_sparkline(f, &app.live.speed_history, &app.live.transfer.speed, theme, chunks[2]);
}

fn render_phase_stepper(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect) {
    let mut spans = Vec::new();
    let phases = [
        "Listings",
        "Diffs locaux",
        "Diffs distants",
        "Application",
        "Mise à jour",
        "Terminé",
    ];

    for (i, name) in phases.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" → ", Style::default().fg(theme.border)));
        }

        let (icon, style) = if i < app.live.phase_index {
            ("✓", Style::default().fg(theme.green).add_modifier(Modifier::BOLD))
        } else if i == app.live.phase_index {
            ("●", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))
        } else {
            ("○", Style::default().fg(theme.text_muted))
        };

        spans.push(Span::styled(format!("{} {}", icon, name), style));
    }

    let p = Paragraph::new(Line::from(spans))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.accent))
                .style(Style::default().bg(theme.card_bg))
                .title(Span::styled(" ÉTAPES DE SYNCHRONISATION ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))),
        );
    f.render_widget(p, area);
}

fn render_transfer_gauge(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect) {
    let pct = app.live.transfer.pct.min(100);
    let speed = if app.live.transfer.speed.is_empty() { "--" } else { &app.live.transfer.speed };
    let eta = if app.live.transfer.eta.is_empty() { "--" } else { &app.live.transfer.eta };
    let done = if app.live.transfer.bytes_done.is_empty() { "0 B" } else { &app.live.transfer.bytes_done };
    let total = if app.live.transfer.bytes_total.is_empty() { "0 B" } else { &app.live.transfer.bytes_total };

    let label = format!("{}% ({} / {}) · {} · ETA: {}", pct, done, total, speed, eta);

    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.accent))
                .style(Style::default().bg(theme.card_bg))
                .title(Span::styled(" Transfert Actif ", Style::default().fg(theme.accent))),
        )
        .gauge_style(Style::default().fg(theme.accent).bg(theme.border))
        .percent(pct as u16)
        .label(Span::styled(label, Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)));

    f.render_widget(gauge, area);
}

fn render_active_files_bar(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect) {
    if app.live.active_files.is_empty() {
        return;
    }

    let mut spans = Vec::new();
    spans.push(Span::styled(" En transfert : ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)));

    for (i, (_key, af)) in app.live.active_files.iter().take(3).enumerate() {
        if i > 0 {
            spans.push(Span::styled(" │ ", Style::default().fg(theme.border)));
        }
        let bar_width = 8usize;
        let filled = ((af.pct as usize) * bar_width) / 100;
        let bar_str = format!("[{}{}]", "█".repeat(filled), "░".repeat(bar_width.saturating_sub(filled)));

        spans.push(Span::styled(format!("{} ", af.name), Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)));
        spans.push(Span::styled(format!("{} {}% ", bar_str, af.pct), Style::default().fg(theme.accent)));
        if !af.speed.is_empty() {
            spans.push(Span::styled(format!("({}) ", af.speed), Style::default().fg(theme.text_muted)));
        }
    }

    let p = Paragraph::new(Line::from(spans))
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.card_bg))
                .title(Span::styled(
                    format!(" ⚡ Fichiers en cours ({}) ", app.live.active_files.len()),
                    Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
                )),
        );
    f.render_widget(p, area);
}

fn render_history_panel(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect, hitboxes: &mut Vec<Hitbox>) {
    let is_focused = app.focused_panel == FocusedPanel::History;
    let border_color = if is_focused { theme.border_focus } else { theme.border };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Mini-graphique de barres en haut
            Constraint::Min(6),    // Tableau des runs
        ])
        .split(Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        });

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled(
            if is_focused { " ▶ 🕒 HISTORIQUE DES SYNCS " } else { " 🕒 HISTORIQUE DES SYNCS " },
            Style::default().fg(if is_focused { theme.accent } else { theme.text_muted }).add_modifier(Modifier::BOLD),
        ));
    f.render_widget(outer_block, area);

    // 1. Sparkline de durées & statut des derniers runs (comme sparkline.js web)
    let max_bars = (chunks[0].width.saturating_sub(18) / 2) as usize;
    let bar_line = crate::ui::sparkline::render_history_sparkline(&app.past_runs, theme, max_bars.max(5));
    let bar_p = Paragraph::new(bar_line);
    f.render_widget(bar_p, chunks[0]);

    // 2. Tableau des runs avec hitboxes et scroll offset
    let visible_rows = chunks[1].height.saturating_sub(2) as usize;
    let max_offset = app.past_runs.len().saturating_sub(visible_rows);
    let offset = if app.selected_run_idx < app.history_scroll_offset {
        app.selected_run_idx
    } else if visible_rows > 0 && app.selected_run_idx >= app.history_scroll_offset + visible_rows {
        app.selected_run_idx.saturating_sub(visible_rows) + 1
    } else {
        app.history_scroll_offset.min(max_offset)
    };

    let runs_to_show: Vec<(usize, &crate::monitor::PastRun)> = app
        .past_runs
        .iter()
        .enumerate()
        .skip(offset)
        .take(visible_rows)
        .collect();

    let table_rows: Vec<Row> = runs_to_show
        .iter()
        .map(|(i, run)| {
            let is_selected = *i == app.selected_run_idx;

            let cursor = if is_selected {
                Span::styled("▶ ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))
            } else {
                Span::styled("  ", Style::default())
            };

            let (status_badge, status_color) = match run.status {
                RunStatus::Success => ("✔ Réussie", theme.green),
                RunStatus::Failed => ("✗ Erreur", theme.red),
                RunStatus::Skipped => ("⊘ Ignorée", theme.text_muted),
                RunStatus::Running => ("▶ En cours", theme.cyan),
            };

            let time_short = if run.date == chrono::Local::now().format("%Y-%m-%d").to_string() {
                format!("Auj. {}", run.time)
            } else {
                format!("{} {}", run.date, run.time)
            };

            let row_style = if is_selected {
                Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text_bright)
            };

            let err_color = if run.errors.is_empty() { theme.text_muted } else { theme.red };

            Row::new(vec![
                Cell::from(Line::from(vec![cursor, Span::styled(time_short, row_style)])),
                Cell::from(Span::styled(status_badge, Style::default().fg(status_color).add_modifier(Modifier::BOLD))),
                Cell::from(Span::styled(run.files_copied.len().to_string(), Style::default().fg(theme.green))),
                Cell::from(Span::styled(run.files_modified.len().to_string(), Style::default().fg(theme.yellow))),
                Cell::from(Span::styled(run.files_deleted.len().to_string(), Style::default().fg(theme.red))),
                Cell::from(Span::styled(&run.duration, Style::default().fg(theme.text_muted))),
                Cell::from(Span::styled(run.errors.len().to_string(), Style::default().fg(err_color))),
            ])
        })
        .collect();

    // Enregistrer les hitboxes des lignes pour les clics souris
    for (row_idx, (real_idx, _)) in runs_to_show.iter().enumerate() {
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
        Row::new(vec!["DÉMARRAGE", "STATUT", "COPIÉS", "MODIF.", "SUPPR.", "DURÉE", "ERR."])
            .style(Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
    );

    f.render_widget(table, chunks[1]);

    if app.past_runs.len() > visible_rows {
        let mut scrollbar_state = ScrollbarState::new(app.past_runs.len())
            .position(app.selected_run_idx);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_style(Style::default().fg(theme.border))
            .thumb_style(Style::default().fg(theme.accent));
        f.render_stateful_widget(scrollbar, chunks[1], &mut scrollbar_state);
    }
}

fn render_logs_panel(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect, hitboxes: &mut Vec<Hitbox>) {
    // Enregistrer la zone des logs pour la molette
    hitboxes.push(Hitbox {
        rect: area,
        action: HitAction::LogsArea,
    });

    let is_focused = app.focused_panel == FocusedPanel::Logs;
    let border_color = if is_focused { theme.border_focus } else { theme.border };

    let status_badge = if app.auto_scroll {
        Span::styled(" [DÉFILEMENT AUTO] ", Style::default().fg(theme.green).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(" [PAUSE - ESPACE] ", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD))
    };

    let visible_height = area.height.saturating_sub(2) as usize;
    let total_lines = app.live.log_lines.len();
    let max_scroll = total_lines.saturating_sub(visible_height);
    let effective_scroll = app.logs_scroll.min(max_scroll);

    let skip_count = if app.auto_scroll {
        max_scroll
    } else {
        max_scroll.saturating_sub(effective_scroll)
    };

    let lines: Vec<Line> = app
        .live
        .log_lines
        .iter()
        .skip(skip_count)
        .take(visible_height)
        .map(|line| colorize_log_line(line, theme))
        .collect();

    let p = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(theme.card_bg))
            .title(Line::from(vec![
                Span::styled(
                    if is_focused { " ▶ 📜 LOGS EN DIRECT " } else { " 📜 LOGS EN DIRECT " },
                    Style::default().fg(if is_focused { theme.accent } else { theme.text_muted }).add_modifier(Modifier::BOLD),
                ),
                status_badge,
            ])),
    );

    f.render_widget(p, area);

    // Scrollbar btop++ pour les logs
    if total_lines > visible_height {
        let current_pos = if app.auto_scroll {
            total_lines
        } else {
            total_lines.saturating_sub(effective_scroll)
        };
        let mut scrollbar_state = ScrollbarState::new(total_lines).position(current_pos);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_style(Style::default().fg(theme.border))
            .thumb_style(Style::default().fg(theme.accent));
        f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}

fn render_recent_files_panel(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect, hitboxes: &mut Vec<Hitbox>) {
    hitboxes.push(Hitbox {
        rect: area,
        action: HitAction::RecentFilesArea,
    });

    let is_focused = app.focused_panel == FocusedPanel::RecentFiles;
    let border_color = if is_focused { theme.border_focus } else { theme.border };

    // On combine les fichiers du run live et des derniers runs d'historique
    let mut files_to_display: Vec<(String, String, String, String)> = Vec::new();

    for sf in app.live.synced_files.iter().rev().take(30) {
        files_to_display.push((sf.action.clone(), sf.path.clone(), "".to_string(), sf.time.clone()));
    }

    if files_to_display.is_empty() {
        for run in app.past_runs.iter().take(5) {
            for f in &run.files_copied {
                files_to_display.push(("new".to_string(), f.clone(), "".to_string(), run.time.clone()));
            }
            for f in &run.files_modified {
                files_to_display.push(("modified".to_string(), f.clone(), "".to_string(), run.time.clone()));
            }
            for f in &run.files_deleted {
                files_to_display.push(("deleted".to_string(), f.clone(), "".to_string(), run.time.clone()));
            }
        }
    }

    let max_show = area.height.saturating_sub(3) as usize;
    let offset = if !files_to_display.is_empty() && max_show > 0 {
        app.recent_scroll_offset.min(files_to_display.len().saturating_sub(max_show))
    } else {
        0
    };

    let rows: Vec<Row> = if files_to_display.is_empty() {
        vec![Row::new(vec![
            Cell::from(Span::styled(" Aucun fichier synchronisé récemment", Style::default().fg(theme.text_muted))),
            Cell::from(""),
            Cell::from(""),
        ])]
    } else {
        files_to_display
            .iter()
            .enumerate()
            .skip(offset)
            .take(max_show)
            .map(|(real_idx, (action, path, _size, time))| {
                let is_selected = is_focused && real_idx == app.recent_selected_idx;
                let (badge_text, badge_color) = match action.as_str() {
                    "new" => ("● Ajouté", theme.green),
                    "deleted" => ("● Supprimé", theme.red),
                    _ => ("● Modifié", theme.yellow),
                };

                let prefix = if is_selected { "▶ " } else { "  " };
                let row_style = if is_selected {
                    Style::default().fg(theme.text_bright).bg(theme.border_focus).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text_bright)
                };

                Row::new(vec![
                    Cell::from(Line::from(vec![
                        Span::styled(prefix, Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD)),
                        Span::styled(format!("{} ", badge_text), Style::default().fg(badge_color).add_modifier(Modifier::BOLD)),
                    ])),
                    Cell::from(Span::styled(path, row_style)),
                    Cell::from(Span::styled(time, Style::default().fg(theme.text_muted))),
                ])
            })
            .collect()
    };

    // Hitboxes pour les lignes de fichiers récents visibles
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

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(15),
            Constraint::Percentage(70),
            Constraint::Percentage(15),
        ],
    )
    .header(
        Row::new(vec!["ACTION", "CHEMIN DU FICHIER", "HEURE"])
            .style(Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(theme.card_bg))
            .title(Span::styled(
                if is_focused {
                    if app.live.is_syncing && !app.live.synced_files.is_empty() {
                        format!(" ▶ 📄 FICHIERS SYNCHRONISÉS EN DIRECT ({}) ", app.live.synced_files.len())
                    } else {
                        " ▶ 📄 FICHIERS RÉCENTS ".to_string()
                    }
                } else if app.live.is_syncing && !app.live.synced_files.is_empty() {
                    format!(" 📄 FICHIERS SYNCHRONISÉS EN DIRECT ({}) ", app.live.synced_files.len())
                } else {
                    " 📄 FICHIERS RÉCENTS ".to_string()
                },
                Style::default().fg(if is_focused { theme.accent } else if app.live.is_syncing && !app.live.synced_files.is_empty() { theme.green } else { theme.text_muted }).add_modifier(Modifier::BOLD),
            )),
    );

    f.render_widget(table, area);

    if files_to_display.len() > max_show {
        let mut scrollbar_state = ScrollbarState::new(files_to_display.len())
            .position(offset);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_style(Style::default().fg(theme.border))
            .thumb_style(Style::default().fg(theme.accent));
        f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}

fn colorize_log_line<'a>(line: &'a str, theme: &ThemePalette) -> Line<'a> {
    let ll = line.to_lowercase();

    let (prefix_color, is_bold) = if ll.contains("error") || ll.contains("failed") || ll.contains("critical") {
        (theme.red, true)
    } else if ll.contains("notice") || ll.contains("warning") || ll.contains("warn") {
        (theme.yellow, false)
    } else if ll.contains("bisync successful") || ll.contains("copied (new)") {
        (theme.green, true)
    } else if ll.contains("transferred:") || ll.contains("checks:") {
        (theme.cyan, false)
    } else {
        (theme.text_bright, false)
    };

    let mut style = Style::default().fg(prefix_color);
    if is_bold {
        style = style.add_modifier(Modifier::BOLD);
    }

    if line.len() > 25 && line.chars().nth(4) == Some('-') && line.chars().nth(7) == Some('-') {
        let ts = &line[..25];
        let rest = &line[25..];
        Line::from(vec![
            Span::styled(ts, Style::default().fg(theme.text_muted)),
            Span::styled(rest, style),
        ])
    } else {
        Line::from(Span::styled(line, style))
    }
}
