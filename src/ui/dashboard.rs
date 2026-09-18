use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Paragraph, Row, Table,
    },
    Frame,
};

use crate::app::{Alert, AlertLevel, App, FocusedPanel, HitAction, Hitbox};
use crate::monitor::history::RunStatus;
use crate::ui::theme::ThemePalette;

pub fn render_dashboard(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect, hitboxes: &mut Vec<Hitbox>) {
    let alerts = app.active_alerts();
    let show_alert = !alerts.is_empty();
    let show_active_sync = app.is_syncing();

    let mut constraints = Vec::new();
    // 1. Cadran Stockage & Métriques (6 lignes pour intégrer horloge, fréquence, quota et KPIs)
    constraints.push(Constraint::Length(6));

    // 2. Bannière d'alerte contextuelle si alerte active
    if show_alert {
        constraints.push(Constraint::Length(3));
    }

    // 3. Barre de sync active (Phase stepper + 7 KPIs + Fichiers modifiés précis)
    if show_active_sync {
        constraints.push(Constraint::Length(7));
    }

    // 4. Zone centrale (Historique + Logs)
    constraints.push(Constraint::Percentage(55));

    // 5. Zone inférieure (Fichiers récents)
    constraints.push(Constraint::Min(6));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let mut chunk_idx = 0;

    // 1. Cadrans Stockage & Métriques (2 cadrans spacieux)
    render_system_cadrans(f, app, theme, chunks[chunk_idx], hitboxes);
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

    // 4. Milieu : Historique + Logs
    let middle_area = chunks[chunk_idx];
    chunk_idx += 1;
    let bottom_area = chunks[chunk_idx];

    let middle_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(middle_area);

    render_history_panel(f, app, theme, middle_chunks[0], hitboxes);
    render_logs_panel(f, app, theme, middle_chunks[1], hitboxes);

    // 5. Fichiers récents
    render_recent_files_panel(f, app, theme, bottom_area, hitboxes);
}

/// Affiche 2 cadrans élégants (Stockage/Cloud à gauche, Métriques/Fiabilité à droite)
/// avec barres de progression horizontales à dégradé, zéro texte tronqué
fn render_system_cadrans(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect, hitboxes: &mut Vec<Hitbox>) {
    let sub = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_disks_cloud_box(f, app, theme, sub[0]);
    render_metrics_box(f, app, theme, sub[1], hitboxes);
}

fn render_disks_cloud_box(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect) {
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_storage))
        .style(Style::default().bg(theme.card_bg))
        .title(Line::from(vec![
            Span::styled("┐", Style::default().fg(theme.border_storage)),
            Span::styled("disks & cloud", Style::default().fg(theme.border_storage).add_modifier(Modifier::BOLD)),
            Span::styled("┌", Style::default().fg(theme.border_storage)),
        ]))
        .title_bottom(
            Line::from(vec![
                Span::styled("─ statvfs / bisync ─", Style::default().fg(theme.border_storage)),
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

    let bar_width = 8usize.min((inner.width.saturating_sub(45) / 2) as usize);

    // 1. Disque local
    let (used_gb, free_gb, total_gb, pct) = app.local_disk_stats();
    let disk_bar = crate::ui::sparkline::render_gradient_bar(pct, bar_width, theme);
    let mut line1_spans = vec![
        Span::styled("Disk:    ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
    ];
    line1_spans.extend(disk_bar);
    line1_spans.push(Span::styled(
        format!(" {:>3.0}%  {:.1} GB / {:.1} GB ({:.1} GB free)", pct, used_gb, total_gb, free_gb),
        Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD),
    ));

    // 2. Stockage Cloud réel
    let mut line2_spans = vec![
        Span::styled("Cloud:   ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
    ];
    if let Some(q) = &app.cloud_quota {
        let used_mb = q.used_bytes as f64 / (1024.0 * 1024.0);
        let total_tb = q.total_bytes as f64 / (1024.0 * 1024.0 * 1024.0 * 1024.0);
        let free_tb = q.free_bytes as f64 / (1024.0 * 1024.0 * 1024.0 * 1024.0);
        let q_pct = if q.total_bytes > 0 { (q.used_bytes as f64 / q.total_bytes as f64) * 100.0 } else { 0.0 };
        let cloud_bar = crate::ui::sparkline::render_gradient_bar(q_pct, bar_width, theme);
        line2_spans.extend(cloud_bar);
        let used_str = if used_mb >= 1024.0 {
            format!("{:.1} GB", used_mb / 1024.0)
        } else {
            format!("{:.1} MB", used_mb)
        };
        line2_spans.push(Span::styled(
            format!(" {:>3.0}%  {} / {:.2} TB ({:.2} TB free)", q_pct, used_str, total_tb, free_tb),
            Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD),
        ));
    } else {
        line2_spans.push(Span::styled(format!("{:<15}", &app.config.remote), Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD)));
        line2_spans.push(Span::styled("· syncing quota...", Style::default().fg(theme.text_muted)));
    }

    // 3. Dossier local & compteur de fichiers suivis
    let count = app.get_tracked_files_count();
    let count_str = if count > 0 {
        format!("{} tracked files", count)
    } else {
        "analyzing...".to_string()
    };
    let line3_spans = vec![
        Span::styled("Folder:  ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{} ", &app.config.local_dir), Style::default().fg(theme.text_bright)),
        Span::styled(format!("({})", count_str), Style::default().fg(theme.text_muted)),
    ];

    // 4. Filet Cloud & bwlimit
    let cloud_net = &app.service_info.cloud_safety_net;
    let bwlimit_str = app.config.bwlimit.as_deref().unwrap_or("Unlimited");
    let line4_spans = vec![
        Span::styled("Safety:  ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{} ", cloud_net), Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
        Span::styled(" │ bwlimit: ", Style::default().fg(theme.text_muted)),
        Span::styled(bwlimit_str, Style::default().fg(theme.text_bright)),
    ];

    let mut lines = vec![
        Line::from(line1_spans),
        Line::from(line2_spans),
    ];
    if inner.height >= 4 {
        lines.push(Line::from(line3_spans));
        lines.push(Line::from(line4_spans));
    } else {
        let line3_compact = vec![
            Span::styled("Folder:  ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{} ", &app.config.local_dir), Style::default().fg(theme.text_bright)),
            Span::styled(format!("({})", count_str), Style::default().fg(theme.text_muted)),
            Span::styled(" │ Safety: ", Style::default().fg(theme.text_muted)),
            Span::styled(format!("{} ", cloud_net), Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
            Span::styled("│ bw: ", Style::default().fg(theme.text_muted)),
            Span::styled(bwlimit_str, Style::default().fg(theme.text_bright)),
        ];
        lines.push(Line::from(line3_compact));
    }

    let p = Paragraph::new(lines);
    f.render_widget(p, inner);
}

fn render_metrics_box(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect, hitboxes: &mut Vec<Hitbox>) {
    let clock_str = chrono::Local::now().format("%H:%M:%S").to_string();
    let tick_ms = app.tick_rate_ms_live;
    let tick_str = format!("{}ms", tick_ms);

    // Hitboxes pour le stepper de fréquence [ - ] et [ + ]
    let tick_widget_len = (tick_str.chars().count() + 4) as u16;
    let tick_x = area.x + area.width.saturating_sub(tick_widget_len + 1);

    hitboxes.push(Hitbox {
        rect: Rect { x: tick_x + 1, y: area.y, width: 2, height: 1 },
        action: HitAction::TickRateDec,
    });
    hitboxes.push(Hitbox {
        rect: Rect { x: tick_x + tick_widget_len.saturating_sub(2), y: area.y, width: 2, height: 1 },
        action: HitAction::TickRateInc,
    });

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_sys))
        .style(Style::default().bg(theme.card_bg))
        .title(Line::from(vec![
            Span::styled("┐", Style::default().fg(theme.border_sys)),
            Span::styled("metrics", Style::default().fg(theme.border_sys).add_modifier(Modifier::BOLD)),
            Span::styled("┌── ", Style::default().fg(theme.border_sys)),
            Span::styled(clock_str, Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)),
            Span::styled(" ──", Style::default().fg(theme.border_sys)),
        ]))
        .title({
            let dec_col = if app.can_dec_tick_rate() { theme.red } else { theme.text_muted };
            let inc_col = if app.can_inc_tick_rate() { theme.red } else { theme.text_muted };
            Line::from(vec![
                Span::styled("┐", Style::default().fg(theme.border_sys)),
                Span::styled("-", Style::default().fg(dec_col).add_modifier(Modifier::BOLD)),
                Span::styled(" ", Style::default().fg(theme.border_sys)),
                Span::styled(tick_str, Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)),
                Span::styled(" ", Style::default().fg(theme.border_sys)),
                Span::styled("+", Style::default().fg(inc_col).add_modifier(Modifier::BOLD)),
                Span::styled("┌", Style::default().fg(theme.border_sys)),
            ])
            .alignment(Alignment::Right)
        });
    f.render_widget(outer_block, area);

    let inner = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(2),
    };

    // 1. Statut du service rclone & Horaires (transférés de l'ancien header)
    let is_active = app.service_info.state == crate::systemd::ServiceState::Active || app.live.is_syncing;
    let (status_dot, status_label, status_col) = if is_active {
        ("●", "SYNC ACTIVE", theme.green)
    } else {
        match app.service_info.state {
            crate::systemd::ServiceState::Idle => ("●", "IDLE", theme.cyan),
            crate::systemd::ServiceState::Failed => ("●", "FAILED", theme.red),
            _ => ("●", "UNKNOWN", theme.text_muted),
        }
    };
    let last_sync_str = app.past_runs.first()
        .map(|r| format!("{} ({})", r.time, r.duration))
        .unwrap_or_else(|| "—".to_string());

    let next_sync_str = if app.service_info.timer_left.is_empty() {
        "—".to_string()
    } else if !app.service_info.timer_next.is_empty()
        && app.service_info.timer_next != app.service_info.timer_left
        && !app.service_info.timer_left.contains(&app.service_info.timer_next)
    {
        format!("{} ({})", app.service_info.timer_next, app.service_info.timer_left)
    } else {
        app.service_info.timer_left.clone()
    };

    let line1_spans = vec![
        Span::styled("Status:   ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{} {} ", status_dot, status_label), Style::default().fg(status_col).add_modifier(Modifier::BOLD)),
        Span::styled("│ Last: ", Style::default().fg(theme.text_muted)),
        Span::styled(format!("{} ", last_sync_str), Style::default().fg(theme.text_bright)),
        Span::styled("│ Next: ", Style::default().fg(theme.text_muted)),
        Span::styled(next_sync_str, Style::default().fg(theme.text_bright)),
    ];

    // 2. Débit dynamique corrigé (sans la mention doublon « transfert clone actif »)
    let speed = if !app.live.transfer.speed.is_empty() && app.live.transfer.speed != "0 B/s" {
        app.live.transfer.speed.clone()
    } else if let Some((_, af)) = app.live.active_files.iter().find(|(_, f)| !f.speed.is_empty()) {
        af.speed.clone()
    } else {
        "0 B/s".to_string()
    };
    let speed_label = if is_active { "live" } else { "last transfer" };

    let line2_spans = vec![
        Span::styled("Speed:    ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{:<13}", speed), Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
        Span::styled(format!("({})", speed_label), Style::default().fg(theme.text_muted)),
    ];

    // 3. Aujourd'hui : distinction stricte erreurs vs conflits
    let (ok_today, err_today) = app.runs_today_stats();
    let conflicts = app.conflicts_today_stats();
    let line3_spans = vec![
        Span::styled("Today:   ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
        Span::styled(format!(" {} successful sync(s)", ok_today), Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
        Span::styled(" · ", Style::default().fg(theme.border)),
        Span::styled(
            format!("{} err.", err_today),
            Style::default().fg(if err_today > 0 { theme.red } else { theme.text_muted }).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" · ", Style::default().fg(theme.border)),
        Span::styled(
            format!("{} conflict(s)", conflicts),
            Style::default().fg(if conflicts > 0 { theme.red } else { theme.text_muted }).add_modifier(Modifier::BOLD),
        ),
    ];

    // 4. Fiabilité 7 jours avec barre à dégradé btop++
    let (rate, _) = app.calculate_success_rate();
    let bar_width = 8usize.min((inner.width.saturating_sub(45) / 2) as usize);
    let rel_bar = crate::ui::sparkline::render_gradient_bar(rate, bar_width, theme);

    let mut line4_spans = vec![
        Span::styled("Reliability: ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
    ];
    line4_spans.extend(rel_bar.clone());
    line4_spans.push(Span::styled(
        format!(" {:>3.0}% success rate (7 days)", rate),
        Style::default().fg(if rate >= 90.0 { theme.green } else { theme.yellow }).add_modifier(Modifier::BOLD),
    ));

    let mut lines = vec![Line::from(line1_spans)];
    if inner.height >= 4 {
        lines.push(Line::from(line2_spans));
        lines.push(Line::from(line3_spans));
        lines.push(Line::from(line4_spans));
    } else {
        let mut line2_compact = vec![
            Span::styled("Speed: ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{:<8} ", speed), Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
            Span::styled("│ Rel: ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
        ];
        line2_compact.extend(rel_bar);
        line2_compact.push(Span::styled(
            format!(" {:>3.0}% (7d)", rate),
            Style::default().fg(if rate >= 90.0 { theme.green } else { theme.yellow }).add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from(line2_compact));
        lines.push(Line::from(line3_spans));
    }

    let p = Paragraph::new(lines);
    f.render_widget(p, inner);
}

fn render_alert_banner(f: &mut Frame, alert: &Alert, theme: &ThemePalette, area: Rect) {
    let (border_color, icon) = match alert.level {
        AlertLevel::Error => (theme.red, " 🚨 CRITICAL ALERT: "),
        AlertLevel::Warning => (theme.yellow, " ⚠️ WARNING: "),
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
    let elapsed = if app.live.transfer.elapsed.is_empty() {
        "0s"
    } else {
        &app.live.transfer.elapsed
    };

    let title_line = Line::from(vec![
        Span::styled("┐", Style::default().fg(theme.accent)),
        Span::styled("synchronization in progress", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled("┌", Style::default().fg(theme.accent)),
    ]);

    let right_title = Line::from(vec![
        Span::styled("⏱ ", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{} ", elapsed), Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent))
        .style(Style::default().bg(theme.card_bg))
        .title(title_line)
        .title(right_title.alignment(Alignment::Right));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 2 {
        return;
    }

    // 1. Phase stepper
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

    // 2. Grille des 7 KPIs de transfert (Parité HTML complète)
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

    // Fichier actif en cours de transfert (intégré dans les KPIs)
    if let Some((_key, af)) = app.live.active_files.iter().next() {
        kpi_spans.push(Span::styled("│ Active: ", Style::default().fg(theme.text_muted)));
        kpi_spans.push(Span::styled(format!("{} ({}%)", af.name, af.pct), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)));
    }

    // Découpage vertical :
    // 1. Stepper de phase (1 ligne)
    // 2. Grille KPIs (1 ligne)
    // 3. Changements locaux & distants côte à côte sur deux colonnes (Path2 à gauche, Path1 à droite)
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Stepper
            Constraint::Length(1), // KPIs
            Constraint::Min(1),    // Changements côte à côte
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

        // Colonne gauche : Changements locaux (Path2)
        let mut loc_lines = vec![
            Line::from(Span::styled("Local changes (Path2):", Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD))),
        ];
        if app.live.changes_local_details.is_empty() && app.live.changes_local.is_empty() {
            loc_lines.push(Line::from(Span::styled("  No changes", Style::default().fg(theme.text_muted))));
        } else if !app.live.changes_local_details.is_empty() {
            for d in app.live.changes_local_details.iter().take(2) {
                let (badge_text, badge_color) = match d.action.to_lowercase().as_str() {
                    "new" | "ajouté" | "ajoute" | "copié" | "copie" => ("● Added", theme.green),
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

        // Colonne droite : Changements distants (Path1)
        let mut rem_lines = vec![
            Line::from(Span::styled("Remote changes (Path1):", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD))),
        ];
        if app.live.changes_remote_details.is_empty() && app.live.changes_remote.is_empty() {
            rem_lines.push(Line::from(Span::styled("  No changes", Style::default().fg(theme.text_muted))));
        } else if !app.live.changes_remote_details.is_empty() {
            for d in app.live.changes_remote_details.iter().take(2) {
                let (badge_text, badge_color) = match d.action.to_lowercase().as_str() {
                    "new" | "ajouté" | "ajoute" | "copié" | "copie" => ("● Added", theme.green),
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

fn render_history_panel(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect, hitboxes: &mut Vec<Hitbox>) {
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
            Constraint::Length(graph_height), // Graphe 2D de durées multi-lignes
            Constraint::Min(6),               // Tableau des runs
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

    let left_bottom = Line::from(vec![
        Span::styled("┘", Style::default().fg(border_color)),
        Span::styled("↑", Style::default().fg(up_col).add_modifier(Modifier::BOLD)),
        Span::styled(" select ", Style::default().fg(Color::White)),
        Span::styled("↓", Style::default().fg(down_col).add_modifier(Modifier::BOLD)),
        Span::styled("└┘", Style::default().fg(border_color)),
        Span::styled("details ", Style::default().fg(det_col)),
        Span::styled("↵", Style::default().fg(key_col).add_modifier(Modifier::BOLD)),
        Span::styled("└", Style::default().fg(border_color)),
    ]);
    let right_bottom = Line::from(vec![
        Span::styled(format!("─ {}/{} ─", cur_run, total_runs), Style::default().fg(border_color).add_modifier(Modifier::BOLD)),
    ]);

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.card_bg))
        .title(Line::from(vec![
            Span::styled("┐", Style::default().fg(border_color)),
            Span::styled("history", Style::default().fg(border_color).add_modifier(Modifier::BOLD)),
            Span::styled("┌", Style::default().fg(border_color)),
        ]))
        .title_bottom(left_bottom.alignment(Alignment::Left))
        .title_bottom(right_bottom.alignment(Alignment::Right));
    f.render_widget(outer_block, area);

    // Hitbox pour le bouton détails du bas du panneau historique (actif uniquement si run passé sélectionné)
    if let Some(idx) = app.selected_run_idx {
        if !(is_syncing && idx == 0) {
            let bottom_y = area.y + area.height.saturating_sub(1);
            hitboxes.push(Hitbox {
                rect: Rect { x: area.x + 11, y: bottom_y, width: 11, height: 1 },
                action: HitAction::HistoryRow(idx),
            });
        }
    }

    // 1. Graphe de durées multi-lignes (btop style) avec hitboxes sur chaque colonne
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

    // 2. Tableau des runs avec hitboxes et scroll offset
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
        let highlight_bg = ratatui::style::Color::Rgb(90, 32, 32);

        if is_syncing && item_idx == 0 {
            // Ligne de la synchronisation en cours
            let elapsed = if app.live.transfer.elapsed.is_empty() { "0s" } else { &app.live.transfer.elapsed };
            let time_str = "In progress";
            let pct_val = app.live.overall_progress_pct();
            let status_str = format!("● {}%", pct_val);
            let copied_count = app.live.synced_files.iter().filter(|f| f.action == "new" || f.action == "copied").count() + app.live.transfer.files_done as usize;
            let copied_val = copied_count.to_string();
            let mod_count = app.live.synced_files.iter().filter(|f| f.action == "modified").count();
            let mod_val = mod_count.to_string();
            let del_count = app.live.synced_files.iter().filter(|f| f.action == "deleted").count();
            let del_val = del_count.to_string();
            let err_val = "0";

            if is_selected {
                let cursor = Span::styled("▶ ", Style::default().fg(ratatui::style::Color::White).add_modifier(Modifier::BOLD));
                table_rows.push(Row::new(vec![
                    Cell::from(Line::from(vec![cursor, Span::styled(time_str, Style::default().fg(ratatui::style::Color::White).add_modifier(Modifier::BOLD))])),
                    Cell::from(Span::styled(status_str, Style::default().fg(ratatui::style::Color::White).add_modifier(Modifier::BOLD))),
                    Cell::from(Span::styled(copied_val, Style::default().fg(ratatui::style::Color::White).add_modifier(Modifier::BOLD))),
                    Cell::from(Span::styled(mod_val, Style::default().fg(ratatui::style::Color::White).add_modifier(Modifier::BOLD))),
                    Cell::from(Span::styled(del_val, Style::default().fg(ratatui::style::Color::White).add_modifier(Modifier::BOLD))),
                    Cell::from(Span::styled(elapsed, Style::default().fg(ratatui::style::Color::White))),
                    Cell::from(Span::styled(err_val, Style::default().fg(ratatui::style::Color::White).add_modifier(Modifier::BOLD))),
                ]).style(Style::default().bg(highlight_bg)));
            } else {
                let cursor = Span::styled("  ", Style::default());
                table_rows.push(Row::new(vec![
                    Cell::from(Line::from(vec![cursor, Span::styled(time_str, Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD))])),
                    Cell::from(Span::styled(status_str, Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD))),
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
                    RunStatus::Running => ("▶ Running", theme.cyan),
                };

                let time_short = if run.date == chrono::Local::now().format("%Y-%m-%d").to_string() {
                    format!("Today {}", run.time)
                } else {
                    format!("{} {}", run.date, run.time)
                };

                let err_color = if run.errors.is_empty() { theme.text_muted } else { theme.red };

                if is_selected {
                    let cursor = Span::styled("▶ ", Style::default().fg(ratatui::style::Color::White).add_modifier(Modifier::BOLD));
                    table_rows.push(Row::new(vec![
                        Cell::from(Line::from(vec![cursor, Span::styled(time_short, Style::default().fg(ratatui::style::Color::White).add_modifier(Modifier::BOLD))])),
                        Cell::from(Span::styled(status_badge, Style::default().fg(ratatui::style::Color::White).add_modifier(Modifier::BOLD))),
                        Cell::from(Span::styled(run.files_copied.len().to_string(), Style::default().fg(ratatui::style::Color::White).add_modifier(Modifier::BOLD))),
                        Cell::from(Span::styled(run.files_modified.len().to_string(), Style::default().fg(ratatui::style::Color::White).add_modifier(Modifier::BOLD))),
                        Cell::from(Span::styled(run.files_deleted.len().to_string(), Style::default().fg(ratatui::style::Color::White).add_modifier(Modifier::BOLD))),
                        Cell::from(Span::styled(&run.duration, Style::default().fg(ratatui::style::Color::White))),
                        Cell::from(Span::styled(run.errors.len().to_string(), Style::default().fg(ratatui::style::Color::White).add_modifier(Modifier::BOLD))),
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

    // Enregistrer les hitboxes des lignes pour les clics souris
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

    crate::ui::render_btop_scrollbar(
        f,
        area,
        total_runs,
        app.selected_run_idx.unwrap_or(0),
        visible_rows,
        theme,
        hitboxes,
        crate::app::ScrollbarTarget::History,
    );
}

fn render_logs_panel(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect, hitboxes: &mut Vec<Hitbox>) {
    // Enregistrer la zone des logs pour la molette
    hitboxes.push(Hitbox {
        rect: area,
        action: HitAction::LogsArea,
    });

    let is_focused = app.focused_panel == FocusedPanel::Logs;
    let border_color = if is_focused { theme.border_focus } else { theme.border_logs };

    let max_text_width = (area.width.saturating_sub(3) as usize).max(20);
    let mut all_wrapped: Vec<Line> = Vec::new();

    for line in &app.live.log_lines {
        if app.log_filter.matches(line) {
            all_wrapped.extend(wrap_and_colorize_log_line(line, max_text_width, theme));
        }
    }

    if all_wrapped.is_empty() {
        let msg = match app.log_filter {
            crate::app::LogFilter::All => " No logs available at the moment.",
            crate::app::LogFilter::Files => " No file transfers in recent logs.",
            crate::app::LogFilter::Problems => " No issues (errors/warnings) detected.",
        };
        all_wrapped.push(Line::from(vec![Span::styled(msg, Style::default().fg(theme.text_muted))]));
    }

    let total_lines = all_wrapped.len();

    let visible_height = area.height.saturating_sub(2) as usize;
    let max_scroll = total_lines.saturating_sub(visible_height);
    let effective_scroll = app.logs_scroll.min(max_scroll);

    let cur_line = if app.auto_scroll {
        total_lines
    } else {
        total_lines.saturating_sub(effective_scroll)
    };

    let (up_col, down_col) = if total_lines <= visible_height {
        (theme.text_muted, theme.text_muted)
    } else if effective_scroll == 0 {
        (theme.red, theme.text_muted)
    } else if effective_scroll >= max_scroll {
        (theme.text_muted, theme.red)
    } else {
        (theme.red, theme.red)
    };

    let left_bottom = Line::from(vec![
        Span::styled("┘", Style::default().fg(border_color)),
        Span::styled("↑", Style::default().fg(up_col).add_modifier(Modifier::BOLD)),
        Span::styled(" scroll ", Style::default().fg(Color::White)),
        Span::styled("↓", Style::default().fg(down_col).add_modifier(Modifier::BOLD)),
        Span::styled("└", Style::default().fg(border_color)),
    ]);
    let right_bottom = Line::from(vec![
        Span::styled(format!("─ {}/{} ─", cur_line, total_lines), Style::default().fg(border_color).add_modifier(Modifier::BOLD)),
    ]);

    let mut title_spans: Vec<Span> = Vec::new();
    title_spans.push(Span::styled("┐", Style::default().fg(border_color)));
    title_spans.push(Span::styled("logs", Style::default().fg(border_color).add_modifier(Modifier::BOLD)));
    title_spans.push(Span::styled("┌┐", Style::default().fg(border_color)));

    // area.x + 1 is start of block title inside border.
    // "┐" (1) + "logs" (4) + "┌┐" (2) = 7 chars, so selector starts at area.x + 1 + 7 = area.x + 8
    let mut cur_hit_x = area.x + 8;

    // Left arrow button: ←
    hitboxes.push(Hitbox {
        rect: Rect {
            x: cur_hit_x,
            y: area.y,
            width: 1,
            height: 1,
        },
        action: HitAction::LogFilterPrev,
    });
    title_spans.push(Span::styled("←", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)));
    cur_hit_x += 1;

    // Filter label: e.g. " All " / " Files " / " Problems "
    let filter_text = format!(" {} ", app.log_filter.label());
    let label_len = filter_text.chars().count() as u16;
    hitboxes.push(Hitbox {
        rect: Rect {
            x: cur_hit_x,
            y: area.y,
            width: label_len,
            height: 1,
        },
        action: HitAction::LogFilterCycle,
    });
    title_spans.push(Span::styled(
        filter_text,
        Style::default().fg(if is_focused { theme.green } else { Color::White }).add_modifier(Modifier::BOLD),
    ));
    cur_hit_x += label_len;

    // Right arrow button: →
    hitboxes.push(Hitbox {
        rect: Rect {
            x: cur_hit_x,
            y: area.y,
            width: 1,
            height: 1,
        },
        action: HitAction::LogFilterNext,
    });
    title_spans.push(Span::styled("→", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)));
    cur_hit_x += 1;

    title_spans.push(Span::styled("┌┐", Style::default().fg(border_color)));
    cur_hit_x += 2;

    let auto_text = if app.auto_scroll { "pause " } else { "auto " };
    let space_glyph = "␣";
    let status_text = if app.auto_scroll { " [ON]" } else { " [OFF]" };
    let auto_width = (auto_text.chars().count() + 1 + status_text.chars().count()) as u16;

    title_spans.push(Span::styled(auto_text, Style::default().fg(theme.text_bright)));
    title_spans.push(Span::styled(space_glyph, Style::default().fg(theme.red).add_modifier(Modifier::BOLD)));
    title_spans.push(Span::styled(
        status_text,
        Style::default().fg(if app.auto_scroll { theme.green } else { theme.yellow }).add_modifier(Modifier::BOLD),
    ));
    title_spans.push(Span::styled("┌", Style::default().fg(border_color)));

    hitboxes.push(Hitbox {
        rect: Rect {
            x: cur_hit_x,
            y: area.y,
            width: auto_width,
            height: 1,
        },
        action: HitAction::ToggleLogsAuto,
    });

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.card_bg))
        .title(Line::from(title_spans))
        .title_bottom(left_bottom.alignment(Alignment::Left))
        .title_bottom(right_bottom.alignment(Alignment::Right));

    let skip_count = if app.auto_scroll {
        max_scroll
    } else {
        max_scroll.saturating_sub(effective_scroll)
    };

    let lines: Vec<Line> = all_wrapped
        .into_iter()
        .skip(skip_count)
        .take(visible_height)
        .collect();

    let p = Paragraph::new(lines).block(outer_block);
    f.render_widget(p, area);

    // Scrollbar intégrée style btop++
    let current_pos = if app.auto_scroll {
        max_scroll
    } else {
        max_scroll.saturating_sub(effective_scroll)
    };
    crate::ui::render_btop_scrollbar(
        f,
        area,
        total_lines,
        current_pos,
        visible_height,
        theme,
        hitboxes,
        crate::app::ScrollbarTarget::Logs,
    );
}

fn render_recent_files_panel(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect, hitboxes: &mut Vec<Hitbox>) {
    hitboxes.push(Hitbox {
        rect: area,
        action: HitAction::RecentFilesArea,
    });

    let is_focused = app.focused_panel == FocusedPanel::RecentFiles;
    let border_color = if is_focused { theme.border_focus } else { theme.border_recent };

    // Alimentation de l'intégralité des fichiers récents (parité web)
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

    let highlight_bg = ratatui::style::Color::Rgb(90, 32, 32);

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
                    let cursor = Span::styled("▶ ", Style::default().fg(ratatui::style::Color::White).add_modifier(Modifier::BOLD));
                    let badge = Span::styled(format!("{} ", badge_text), Style::default().fg(ratatui::style::Color::White).add_modifier(Modifier::BOLD));
                    let path_span = Span::styled(display_path, Style::default().fg(ratatui::style::Color::White).add_modifier(Modifier::BOLD));
                    let size_span = Span::styled(size, Style::default().fg(ratatui::style::Color::White).add_modifier(Modifier::BOLD));
                    let time_span = Span::styled(time, Style::default().fg(ratatui::style::Color::White).add_modifier(Modifier::BOLD));

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

    let bottom_spans = vec![
        Span::styled("┘", Style::default().fg(border_color)),
        Span::styled("↑", Style::default().fg(up_col).add_modifier(Modifier::BOLD)),
        Span::styled(" select ", Style::default().fg(ratatui::style::Color::White)),
        Span::styled("↓", Style::default().fg(down_col).add_modifier(Modifier::BOLD)),
        Span::styled("└┘", Style::default().fg(border_color)),
        Span::styled("open ", Style::default().fg(opn_col)),
        Span::styled("↵", Style::default().fg(key_col).add_modifier(Modifier::BOLD)),
        Span::styled("└", Style::default().fg(border_color)),
    ];

    let left_bottom = Line::from(bottom_spans);
    let right_bottom = Line::from(vec![
        Span::styled(format!("─ {}/{} ─", cur_file, total_files), Style::default().fg(border_color).add_modifier(Modifier::BOLD)),
    ]);

    let mut top_spans = vec![
        Span::styled("┐", Style::default().fg(border_color)),
        Span::styled("recent files", Style::default().fg(border_color).add_modifier(Modifier::BOLD)),
        Span::styled("┌┐", Style::default().fg(border_color)),
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
        top_spans.push(Span::styled("f", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)));
        top_spans.push(Span::styled(" filter", Style::default().fg(theme.text_bright)));
        8
    };

    top_spans.push(Span::styled("┌┐", Style::default().fg(border_color)));
    top_spans.push(Span::styled("Ctrl+X", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)));
    let (dossier_text, dossier_color) = if app.ctrl_mode {
        (" folder [ON]", theme.yellow)
    } else {
        (" folder", theme.text_bright)
    };
    top_spans.push(Span::styled(dossier_text, Style::default().fg(dossier_color).add_modifier(Modifier::BOLD)));
    top_spans.push(Span::styled("┌", Style::default().fg(border_color)));

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.card_bg))
        .title(Line::from(top_spans))
        .title_bottom(left_bottom.alignment(Alignment::Left))
        .title_bottom(right_bottom.alignment(Alignment::Right));

    // Hitbox pour l'onglet filter dans le bandeau supérieur
    hitboxes.push(Hitbox {
        rect: Rect { x: filter_start_x, y: area.y, width: filter_w, height: 1 },
        action: HitAction::RecentFilterFocus,
    });

    // Hitbox pour l'onglet Ctrl+X dossier dans le bandeau supérieur
    hitboxes.push(Hitbox {
        rect: Rect { x: filter_start_x + filter_w + 2, y: area.y, width: 6 + dossier_text.chars().count() as u16, height: 1 },
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

    crate::ui::render_btop_scrollbar(
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

/// Découpe un texte pour éviter tout crop horizontal dans le terminal.
/// Conserve les mots si possible, et découpe proprement les mots trop longs sans panique UTF-8.
pub fn wrap_text(text: &str, first_max: usize, cont_max: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }

    let first_limit = first_max.max(10);
    let cont_limit = cont_max.max(10);

    let mut lines = Vec::new();
    let mut cur_line = String::new();
    let mut cur_limit = first_limit;

    for word in text.split(' ') {
        if word.is_empty() {
            if !cur_line.is_empty() && cur_line.chars().count() < cur_limit {
                cur_line.push(' ');
            }
            continue;
        }

        let cur_len = cur_line.chars().count();
        let word_len = word.chars().count();
        let needed = if cur_len == 0 { word_len } else { cur_len + 1 + word_len };

        if needed <= cur_limit {
            if cur_len > 0 {
                cur_line.push(' ');
            }
            cur_line.push_str(word);
        } else if word_len > cur_limit {
            if cur_len > 0 {
                lines.push(cur_line);
                cur_line = String::new();
                cur_limit = cont_limit;
            }
            let mut rem = word;
            while !rem.is_empty() {
                let count = rem.chars().count();
                if count <= cur_limit {
                    cur_line.push_str(rem);
                    break;
                } else {
                    let split_idx = rem.char_indices().nth(cur_limit).map(|(i, _)| i).unwrap_or(rem.len());
                    lines.push(rem[..split_idx].to_string());
                    rem = &rem[split_idx..];
                    cur_limit = cont_limit;
                }
            }
        } else {
            if !cur_line.is_empty() {
                lines.push(cur_line);
            }
            cur_line = word.to_string();
            cur_limit = cont_limit;
        }
    }

    if !cur_line.is_empty() || lines.is_empty() {
        lines.push(cur_line);
    }

    lines
}

pub fn count_wrapped_line(line: &str, max_width: usize) -> usize {
    if line.len() > 25 && line.chars().nth(4) == Some('-') && line.chars().nth(7) == Some('-') {
        let rest = &line[25..];
        let rest_first = max_width.saturating_sub(25);
        let cont = max_width.saturating_sub(4);
        wrap_text(rest, rest_first, cont).len()
    } else {
        let cont = max_width.saturating_sub(4);
        wrap_text(line, max_width, cont).len()
    }
}

pub fn count_wrapped_log_lines(lines: &std::collections::VecDeque<String>, filter: crate::app::LogFilter, max_width: usize) -> usize {
    let mut total = 0;
    for l in lines {
        if filter.matches(l) {
            total += count_wrapped_line(l, max_width);
        }
    }
    total
}

pub fn wrap_and_colorize_log_line(line: &str, max_width: usize, theme: &ThemePalette) -> Vec<Line<'static>> {
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
        let rest_first_limit = max_width.saturating_sub(25);
        let cont_limit = max_width.saturating_sub(4);

        let wrapped = wrap_text(rest, rest_first_limit, cont_limit);
        let mut lines = Vec::with_capacity(wrapped.len());

        for (i, part) in wrapped.into_iter().enumerate() {
            if i == 0 {
                lines.push(Line::from(vec![
                    Span::styled(ts.to_string(), Style::default().fg(theme.text_muted)),
                    Span::styled(part, style),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("  ↳ ".to_string(), Style::default().fg(theme.text_muted)),
                    Span::styled(part, style),
                ]));
            }
        }
        lines
    } else {
        let cont_limit = max_width.saturating_sub(4);
        let wrapped = wrap_text(line, max_width, cont_limit);
        let mut lines = Vec::with_capacity(wrapped.len());

        for (i, part) in wrapped.into_iter().enumerate() {
            if i == 0 {
                lines.push(Line::from(vec![Span::styled(part, style)]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("  ↳ ".to_string(), Style::default().fg(theme.text_muted)),
                    Span::styled(part, style),
                ]));
            }
        }
        lines
    }
}

#[allow(dead_code)]
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

pub fn normalize_display_path(p: &str) -> String {
    let mut s = p.replace(" From ", " from ");
    if s.starts_with("From ") {
        s = format!("from {}", &s[5..]);
    }
    s.replace("/From ", "/from ")
}
