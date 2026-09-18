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
    // 1. Cadran Stockage & Métriques (btop++ Disks/Mem style)
    constraints.push(Constraint::Length(5));

    // 2. Bannière d'alerte contextuelle si alerte active
    if show_alert {
        constraints.push(Constraint::Length(3));
    }

    // 3. Barre de sync active (Phase stepper + Jauge + Sparkline)
    if show_active_sync {
        constraints.push(Constraint::Length(3));
    }

    // 4. Fichiers en cours de transfert
    if show_active_files {
        constraints.push(Constraint::Length(3));
    }

    // 5. Zone centrale (Historique + Logs)
    constraints.push(Constraint::Percentage(55));

    // 6. Zone inférieure (Fichiers récents)
    constraints.push(Constraint::Min(6));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let mut chunk_idx = 0;

    // 1. Cadrans Stockage & Métriques (2 cadrans spacieux type btop++)
    render_system_cadrans(f, app, theme, chunks[chunk_idx]);
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

/// Affiche 2 cadrans élégants (Stockage/Cloud à gauche, Métriques/Fiabilité à droite)
/// avec barres de progression horizontales à dégradé, zéro texte tronqué
fn render_system_cadrans(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect) {
    let sub = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_disks_cloud_box(f, app, theme, sub[0]);
    render_metrics_box(f, app, theme, sub[1]);
}

fn render_disks_cloud_box(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect) {
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(theme.border_storage))
        .style(Style::default().bg(theme.card_bg))
        .title(Line::from(vec![
            Span::styled("┌²disks & cloud", Style::default().fg(theme.border_storage).add_modifier(Modifier::BOLD)),
            Span::styled("┐", Style::default().fg(theme.border)),
        ]))
        .title_bottom(
            Line::from(vec![
                Span::styled("statvfs / bisync ─┘", Style::default().fg(theme.border_storage)),
            ])
            .alignment(Alignment::Right),
        );
    f.render_widget(outer_block, area);

    let inner = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(2),
    };

    let (used_gb, free_gb, total_gb, pct) = app.local_disk_stats();
    let bar_width = 10usize.min((inner.width.saturating_sub(42) / 2) as usize);
    let disk_bar = crate::ui::sparkline::render_gradient_bar(pct, bar_width, theme);

    let mut line1_spans = vec![
        Span::styled("Disque:  ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
    ];
    line1_spans.extend(disk_bar);
    line1_spans.push(Span::styled(
        format!(" {:>3.0}%  {:.0} Go / {:.0} Go ({:.1} Go libres)", pct, used_gb, total_gb, free_gb),
        Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD),
    ));

    let cloud_net = &app.service_info.cloud_safety_net;
    let bwlimit_str = app.config.bwlimit.as_deref().unwrap_or("Illimité");
    let line2_spans = vec![
        Span::styled("Cloud:   ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{:<15}", &app.config.remote), Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Filet: ", Style::default().fg(theme.text_muted)),
        Span::styled(format!("{} ", cloud_net), Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
        Span::styled("│ bwlimit: ", Style::default().fg(theme.text_muted)),
        Span::styled(bwlimit_str, Style::default().fg(theme.text_bright)),
    ];

    let count = if !app.file_entries.is_empty() {
        app.file_entries.iter().filter(|f| !f.is_dir).count()
    } else {
        22224
    };
    let line3_spans = vec![
        Span::styled("Dossier: ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{} ", &app.config.local_dir), Style::default().fg(theme.text_bright)),
        Span::styled(format!("({} fichiers suivis)", count), Style::default().fg(theme.text_muted)),
    ];

    let p = Paragraph::new(vec![
        Line::from(line1_spans),
        Line::from(line2_spans),
        Line::from(line3_spans),
    ]);
    f.render_widget(p, inner);
}

fn render_metrics_box(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect) {
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(theme.border_sys))
        .style(Style::default().bg(theme.card_bg))
        .title(Line::from(vec![
            Span::styled("┌³metrics", Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD)),
            Span::styled("┐", Style::default().fg(theme.border)),
        ]))
        .title_bottom(
            Line::from(vec![
                Span::styled("systemd rclone ─┘", Style::default().fg(theme.border_sys)),
            ])
            .alignment(Alignment::Right),
        );
    f.render_widget(outer_block, area);

    let inner = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(2),
    };

    let speed = if app.live.transfer.speed.is_empty() {
        "0.0 KiB/s".to_string()
    } else {
        app.live.transfer.speed.clone()
    };
    let is_syncing = app.live.is_syncing;
    let line1_spans = vec![
        Span::styled("Débit:    ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{:<13}", speed), Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
        Span::styled("transfert rclone  │  ", Style::default().fg(theme.text_muted)),
        Span::styled(
            if is_syncing { "● Actif" } else { "○ En attente" },
            Style::default().fg(if is_syncing { theme.green } else { theme.text_muted }).add_modifier(Modifier::BOLD),
        ),
    ];

    let (ok_today, err_today) = app.runs_today_stats();
    let conflicts = app.conflicts_today_stats();
    let line2_spans = vec![
        Span::styled("Aujourd'hui:", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
        Span::styled(format!(" {} sync réussie(s)", ok_today), Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
        Span::styled(" · ", Style::default().fg(theme.border)),
        Span::styled(
            format!("{} err.", err_today),
            Style::default().fg(if err_today > 0 { theme.red } else { theme.text_muted }).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" · ", Style::default().fg(theme.border)),
        Span::styled(
            format!("{} conflit(s)", conflicts),
            Style::default().fg(if conflicts > 0 { theme.red } else { theme.text_muted }).add_modifier(Modifier::BOLD),
        ),
    ];

    let (rate, _) = app.calculate_success_rate();
    let rel_bar_width = 10usize.min((inner.width.saturating_sub(42) / 2) as usize);
    let rel_bar = crate::ui::sparkline::render_gradient_bar(rate, rel_bar_width, theme);

    let mut line3_spans = vec![
        Span::styled("Fiabilité:", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
    ];
    line3_spans.extend(rel_bar);
    line3_spans.push(Span::styled(
        format!(" {:>3.0}% taux de réussite (7 jours)", rate),
        Style::default().fg(if rate >= 90.0 { theme.green } else { theme.yellow }).add_modifier(Modifier::BOLD),
    ));

    let p = Paragraph::new(vec![
        Line::from(line1_spans),
        Line::from(line2_spans),
        Line::from(line3_spans),
    ]);
    f.render_widget(p, inner);
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
                .border_type(BorderType::Plain)
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
                .border_type(BorderType::Plain)
                .border_style(Style::default().fg(theme.accent))
                .style(Style::default().bg(theme.card_bg))
                .title(Line::from(vec![
                    Span::styled("┌ÉTAPES DE SYNCHRONISATION", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                    Span::styled("┐", Style::default().fg(theme.border)),
                ])),
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
                .border_type(BorderType::Plain)
                .border_style(Style::default().fg(theme.accent))
                .style(Style::default().bg(theme.card_bg))
                .title(Line::from(vec![
                    Span::styled("┌Transfert Actif", Style::default().fg(theme.accent)),
                    Span::styled("┐", Style::default().fg(theme.border)),
                ])),
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
                .border_type(BorderType::Plain)
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.card_bg))
                .title(Line::from(vec![
                    Span::styled(format!("┌⚡ Fichiers en cours ({})", app.live.active_files.len()), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                    Span::styled("┐", Style::default().fg(theme.border)),
                ])),
        );
    f.render_widget(p, area);
}

fn render_history_panel(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect, hitboxes: &mut Vec<Hitbox>) {
    let is_focused = app.focused_panel == FocusedPanel::History;
    let border_color = if is_focused { theme.border_focus } else { theme.border_history };
    let total_runs = app.past_runs.len();
    let cur_run = if total_runs > 0 { app.selected_run_idx + 1 } else { 0 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Sparkline de durées & statut en haut
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
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.card_bg))
        .title(Line::from(vec![
            Span::styled("┌⁴history", Style::default().fg(theme.border_history).add_modifier(Modifier::BOLD)),
            Span::styled("┐", Style::default().fg(theme.border)),
            Span::styled("┌details ↵", Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD)),
            Span::styled("┐", Style::default().fg(theme.border)),
            Span::styled(format!("─── [{} runs]┐", total_runs), Style::default().fg(theme.border)),
        ]))
        .title_bottom(
            Line::from(vec![
                Span::styled("↑/↓ select  ↵ détails  c stop ", Style::default().fg(theme.text_muted)),
                Span::styled(format!("─ run {}/{}┘", cur_run, total_runs), Style::default().fg(theme.border_history).add_modifier(Modifier::BOLD)),
            ])
            .alignment(Alignment::Right),
        );
    f.render_widget(outer_block, area);

    // 1. Sparkline de durées & statut des derniers runs
    let mut dur_spans = vec![
        Span::styled("Graphe durées : ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
    ];
    let max_bars = chunks[0].width.saturating_sub(18) as usize;
    let bar_line = crate::ui::sparkline::render_history_sparkline(&app.past_runs, theme, max_bars.max(5));
    dur_spans.extend(bar_line.spans);
    let bar_p = Paragraph::new(Line::from(dur_spans));
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
                Span::styled("▶ ", Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD))
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
            .thumb_style(Style::default().fg(theme.highlight));
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
    let border_color = if is_focused { theme.border_focus } else { theme.border_logs };
    let total_lines = app.live.log_lines.len();

    let auto_badge = if app.auto_scroll {
        ("auto: ON (space)", theme.green)
    } else {
        ("PAUSE (space)", theme.yellow)
    };

    let visible_height = area.height.saturating_sub(2) as usize;
    let max_scroll = total_lines.saturating_sub(visible_height);
    let effective_scroll = app.logs_scroll.min(max_scroll);

    let cur_line = if app.auto_scroll {
        total_lines
    } else {
        total_lines.saturating_sub(effective_scroll)
    };

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.card_bg))
        .title(Line::from(vec![
            Span::styled("┌⁵logs", Style::default().fg(theme.border_logs).add_modifier(Modifier::BOLD)),
            Span::styled("┐", Style::default().fg(theme.border)),
            Span::styled(format!("┌{}", auto_badge.0), Style::default().fg(auto_badge.1).add_modifier(Modifier::BOLD)),
            Span::styled("┐", Style::default().fg(theme.border)),
            Span::styled(format!("─── [{} lignes]┐", total_lines), Style::default().fg(theme.border)),
        ]))
        .title_bottom(
            Line::from(vec![
                Span::styled("↑/↓ scroll  Space pause  Home/End ", Style::default().fg(theme.text_muted)),
                Span::styled(format!("─ ligne {}/{}┘", cur_line, total_lines), Style::default().fg(theme.border_logs).add_modifier(Modifier::BOLD)),
            ])
            .alignment(Alignment::Right),
        );

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

    let p = Paragraph::new(lines).block(outer_block);
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
            .thumb_style(Style::default().fg(theme.highlight));
        f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}

fn render_recent_files_panel(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect, hitboxes: &mut Vec<Hitbox>) {
    hitboxes.push(Hitbox {
        rect: area,
        action: HitAction::RecentFilesArea,
    });

    let is_focused = app.focused_panel == FocusedPanel::RecentFiles;
    let border_color = if is_focused { theme.border_focus } else { theme.border_recent };

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

    let total_files = files_to_display.len();
    let cur_file = if total_files > 0 { app.recent_selected_idx + 1 } else { 0 };

    let max_show = area.height.saturating_sub(3) as usize;
    let offset = if !files_to_display.is_empty() && max_show > 0 {
        app.recent_scroll_offset.min(files_to_display.len().saturating_sub(max_show))
    } else {
        0
    };

    let rows: Vec<Row> = if files_to_display.is_empty() {
        vec![Row::new(vec![
            Cell::from(Span::styled(" Aucun fichier récemment synchronisé", Style::default().fg(theme.text_muted))),
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

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.card_bg))
        .title(Line::from(vec![
            Span::styled("┌⁶recent files", Style::default().fg(theme.border_recent).add_modifier(Modifier::BOLD)),
            Span::styled("┐", Style::default().fg(theme.border)),
            Span::styled("┌ouvrir ↵", Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD)),
            Span::styled("┐", Style::default().fg(theme.border)),
            Span::styled("┌dossier: d / Ctrl+↵", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)),
            Span::styled("┐", Style::default().fg(theme.border)),
            Span::styled(format!("─── [{} fichiers]┐", total_files), Style::default().fg(theme.border)),
        ]))
        .title_bottom(
            Line::from(vec![
                Span::styled("↑/↓ select  ↵ open  d dossier  Tab panel ", Style::default().fg(theme.text_muted)),
                Span::styled(format!("─ fic {}/{}┘", cur_file, total_files), Style::default().fg(theme.border_recent).add_modifier(Modifier::BOLD)),
            ])
            .alignment(Alignment::Right),
        );

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(16),
            Constraint::Percentage(68),
            Constraint::Percentage(16),
        ],
    )
    .header(
        Row::new(vec!["ACTION", "CHEMIN DU FICHIER", "HEURE"])
            .style(Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
    )
    .block(outer_block);

    f.render_widget(table, area);

    if files_to_display.len() > max_show {
        let mut scrollbar_state = ScrollbarState::new(files_to_display.len())
            .position(offset);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_style(Style::default().fg(theme.border))
            .thumb_style(Style::default().fg(theme.highlight));
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
