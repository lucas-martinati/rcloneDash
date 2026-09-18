use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation,
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

    let mut constraints = Vec::new();
    // 1. Cadran Stockage & Métriques (6 lignes pour intégrer horloge, fréquence, quota et KPIs)
    constraints.push(Constraint::Length(6));

    // 2. Bannière d'alerte contextuelle si alerte active
    if show_alert {
        constraints.push(Constraint::Length(3));
    }

    // 3. Barre de sync active (Phase stepper + 7 KPIs + Fichiers modifiés précis)
    if show_active_sync {
        constraints.push(Constraint::Length(6));
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
        .border_type(BorderType::Plain)
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
        Span::styled("Disque:  ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
    ];
    line1_spans.extend(disk_bar);
    line1_spans.push(Span::styled(
        format!(" {:>3.0}%  {:.1} Go / {:.1} Go ({:.1} Go libres)", pct, used_gb, total_gb, free_gb),
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
            format!("{:.1} Go", used_mb / 1024.0)
        } else {
            format!("{:.1} Mo", used_mb)
        };
        line2_spans.push(Span::styled(
            format!(" {:>3.0}%  {} / {:.2} To ({:.2} To libres)", q_pct, used_str, total_tb, free_tb),
            Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD),
        ));
    } else {
        line2_spans.push(Span::styled(format!("{:<15}", &app.config.remote), Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD)));
        line2_spans.push(Span::styled("· synchronisation quota...", Style::default().fg(theme.text_muted)));
    }

    // 3. Dossier local & compteur de fichiers suivis (débloqué de 0)
    let count = app.get_tracked_files_count();
    let count_str = if count > 0 {
        format!("{} fichiers suivis", count)
    } else {
        "analyse...".to_string()
    };
    let line3_spans = vec![
        Span::styled("Dossier: ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{} ", &app.config.local_dir), Style::default().fg(theme.text_bright)),
        Span::styled(format!("({})", count_str), Style::default().fg(theme.text_muted)),
    ];

    // 4. Filet Cloud & bwlimit
    let cloud_net = &app.service_info.cloud_safety_net;
    let bwlimit_str = app.config.bwlimit.as_deref().unwrap_or("Illimité");
    let line4_spans = vec![
        Span::styled("Filet:   ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{} ", cloud_net), Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
        Span::styled(" │ bwlimit: ", Style::default().fg(theme.text_muted)),
        Span::styled(bwlimit_str, Style::default().fg(theme.text_bright)),
    ];

    let p = Paragraph::new(vec![
        Line::from(line1_spans),
        Line::from(line2_spans),
        Line::from(line3_spans),
        Line::from(line4_spans),
    ]);
    f.render_widget(p, inner);
}

fn render_metrics_box(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect, hitboxes: &mut Vec<Hitbox>) {
    let clock_str = chrono::Local::now().format("%H:%M:%S").to_string();
    let tick_ms = app.tick_rate_ms_live;
    let tick_str = format!("{}ms", tick_ms);

    // Hitboxes pour le stepper de fréquence [ - ] et [ + ]
    let tick_widget_len = (tick_str.chars().count() + 6) as u16;
    let tick_x = area.x + area.width.saturating_sub(tick_widget_len + 1);

    hitboxes.push(Hitbox {
        rect: Rect { x: tick_x, y: area.y, width: 3, height: 1 },
        action: HitAction::TickRateDec,
    });
    hitboxes.push(Hitbox {
        rect: Rect { x: tick_x + tick_widget_len.saturating_sub(3), y: area.y, width: 3, height: 1 },
        action: HitAction::TickRateInc,
    });

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(theme.border_sys))
        .style(Style::default().bg(theme.card_bg))
        .title(Line::from(vec![
            Span::styled("┐", Style::default().fg(theme.border_sys)),
            Span::styled("metrics", Style::default().fg(theme.border_sys).add_modifier(Modifier::BOLD)),
            Span::styled("┌── ", Style::default().fg(theme.border_sys)),
            Span::styled(clock_str, Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)),
            Span::styled(" ──", Style::default().fg(theme.border_sys)),
        ]))
        .title(
            Line::from(vec![
                Span::styled("┌", Style::default().fg(theme.border)),
                Span::styled("-", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
                Span::styled(" ", Style::default().fg(theme.border)),
                Span::styled(tick_str, Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)),
                Span::styled(" ", Style::default().fg(theme.border)),
                Span::styled("+", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
                Span::styled("┐─", Style::default().fg(theme.border)),
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

    // 1. Statut du service rclone & Horaires (transférés de l'ancien header)
    let is_active = app.service_info.state == crate::systemd::ServiceState::Active || app.live.is_syncing;
    let (status_dot, status_label, status_col) = if is_active {
        ("●", "SYNC ACTIVE", theme.green)
    } else {
        match app.service_info.state {
            crate::systemd::ServiceState::Idle => ("●", "EN ATTENTE", theme.cyan),
            crate::systemd::ServiceState::Failed => ("●", "ÉCHEC", theme.red),
            _ => ("●", "INCONNU", theme.text_muted),
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
        Span::styled("Statut:   ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{} {} ", status_dot, status_label), Style::default().fg(status_col).add_modifier(Modifier::BOLD)),
        Span::styled("│ Dernière: ", Style::default().fg(theme.text_muted)),
        Span::styled(format!("{} ", last_sync_str), Style::default().fg(theme.text_bright)),
        Span::styled("│ Prochaine: ", Style::default().fg(theme.text_muted)),
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
    let speed_label = if is_active { "instantané" } else { "dernier transfert" };

    let line2_spans = vec![
        Span::styled("Débit:    ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{:<13}", speed), Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
        Span::styled(format!("({})", speed_label), Style::default().fg(theme.text_muted)),
    ];

    // 3. Aujourd'hui : distinction stricte erreurs vs conflits
    let (ok_today, err_today) = app.runs_today_stats();
    let conflicts = app.conflicts_today_stats();
    let line3_spans = vec![
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

    // 4. Fiabilité 7 jours avec barre à dégradé btop++
    let (rate, _) = app.calculate_success_rate();
    let bar_width = 8usize.min((inner.width.saturating_sub(45) / 2) as usize);
    let rel_bar = crate::ui::sparkline::render_gradient_bar(rate, bar_width, theme);

    let mut line4_spans = vec![
        Span::styled("Fiabilité:", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
    ];
    line4_spans.extend(rel_bar);
    line4_spans.push(Span::styled(
        format!(" {:>3.0}% taux de réussite (7 jours)", rate),
        Style::default().fg(if rate >= 90.0 { theme.green } else { theme.yellow }).add_modifier(Modifier::BOLD),
    ));

    let p = Paragraph::new(vec![
        Line::from(line1_spans),
        Line::from(line2_spans),
        Line::from(line3_spans),
        Line::from(line4_spans),
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
    let elapsed = if app.live.transfer.elapsed.is_empty() {
        "0s"
    } else {
        &app.live.transfer.elapsed
    };

    let title_line = Line::from(vec![
        Span::styled("┐", Style::default().fg(theme.accent)),
        Span::styled("synchronisation en cours", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled("┌", Style::default().fg(theme.accent)),
    ]);

    let right_title = Line::from(vec![
        Span::styled("⏱ ", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{} ", elapsed), Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
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
        "Diffs locaux",
        "Diffs distants",
        "Application",
        "Mise à jour",
        "Terminé",
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
    let pct_str = format!("{}%", app.live.transfer.pct.min(100));
    let speed = if app.live.transfer.speed.is_empty() { "—" } else { &app.live.transfer.speed };
    let eta = if app.live.transfer.eta.is_empty() { "—" } else { &app.live.transfer.eta };
    let checks = format!("{} / {}", app.live.transfer.checks_done, app.live.transfer.checks_total);
    let files = format!("{} / {}", app.live.transfer.files_done, app.live.transfer.files_total);

    let kpi_spans = vec![
        Span::styled("Transféré: ", Style::default().fg(theme.text_muted)),
        Span::styled(format!("{} ", done), Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)),
        Span::styled("│ Total: ", Style::default().fg(theme.text_muted)),
        Span::styled(format!("{} ", total), Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)),
        Span::styled("│ Progr.: ", Style::default().fg(theme.text_muted)),
        Span::styled(format!("{} ", pct_str), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled("│ Vitesse: ", Style::default().fg(theme.text_muted)),
        Span::styled(format!("{} ", speed), Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
        Span::styled("│ ETA: ", Style::default().fg(theme.text_muted)),
        Span::styled(format!("{} ", eta), Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)),
        Span::styled("│ Vérifs: ", Style::default().fg(theme.text_muted)),
        Span::styled(format!("{} ", checks), Style::default().fg(theme.text_bright)),
        Span::styled("│ Fichiers: ", Style::default().fg(theme.text_muted)),
        Span::styled(files, Style::default().fg(theme.text_bright)),
    ];

    // 3. Changements locaux détectés avec détail précis
    let mut loc_spans = vec![
        Span::styled("Changements locaux (Path2) : ", Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD)),
    ];
    if app.live.changes_local_details.is_empty() && app.live.changes_local.is_empty() {
        loc_spans.push(Span::styled("Aucun changement", Style::default().fg(theme.text_muted)));
    } else if !app.live.changes_local_details.is_empty() {
        for (i, d) in app.live.changes_local_details.iter().take(3).enumerate() {
            if i > 0 { loc_spans.push(Span::styled(" · ", Style::default().fg(theme.border))); }
            loc_spans.push(Span::styled(format!("• {} ", d.path), Style::default().fg(theme.text_bright)));
            loc_spans.push(Span::styled(format!("[{}]", d.action), Style::default().fg(theme.cyan)));
        }
        if app.live.changes_local_details.len() > 3 {
            loc_spans.push(Span::styled(format!(" (+{} autres)", app.live.changes_local_details.len() - 3), Style::default().fg(theme.text_muted)));
        }
    } else {
        for (i, f) in app.live.changes_local.iter().take(3).enumerate() {
            if i > 0 { loc_spans.push(Span::styled(" · ", Style::default().fg(theme.border))); }
            loc_spans.push(Span::styled(format!("• {}", f), Style::default().fg(theme.text_bright)));
        }
        if app.live.changes_local.len() > 3 {
            loc_spans.push(Span::styled(format!(" (+{} autres)", app.live.changes_local.len() - 3), Style::default().fg(theme.text_muted)));
        }
    }

    // 4. Changements distants détectés avec détail précis
    let mut rem_spans = vec![
        Span::styled("Changements distants (Path1) : ", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)),
    ];
    if app.live.changes_remote_details.is_empty() && app.live.changes_remote.is_empty() {
        rem_spans.push(Span::styled("Aucun changement", Style::default().fg(theme.text_muted)));
    } else if !app.live.changes_remote_details.is_empty() {
        for (i, d) in app.live.changes_remote_details.iter().take(3).enumerate() {
            if i > 0 { rem_spans.push(Span::styled(" · ", Style::default().fg(theme.border))); }
            rem_spans.push(Span::styled(format!("• {} ", d.path), Style::default().fg(theme.text_bright)));
            rem_spans.push(Span::styled(format!("[{}]", d.action), Style::default().fg(theme.yellow)));
        }
        if app.live.changes_remote_details.len() > 3 {
            rem_spans.push(Span::styled(format!(" (+{} autres)", app.live.changes_remote_details.len() - 3), Style::default().fg(theme.text_muted)));
        }
    } else {
        for (i, f) in app.live.changes_remote.iter().take(3).enumerate() {
            if i > 0 { rem_spans.push(Span::styled(" · ", Style::default().fg(theme.border))); }
            rem_spans.push(Span::styled(format!("• {}", f), Style::default().fg(theme.text_bright)));
        }
        if app.live.changes_remote.len() > 3 {
            rem_spans.push(Span::styled(format!(" (+{} autres)", app.live.changes_remote.len() - 3), Style::default().fg(theme.text_muted)));
        }
    }

    let mut content_lines = vec![
        Line::from(stepper_spans),
        Line::from(kpi_spans),
        Line::from(loc_spans),
        Line::from(rem_spans),
    ];

    // 5. Si fichier actif en cours de transfert
    if let Some((_key, af)) = app.live.active_files.iter().next() {
        if inner.height >= 5 {
            let active_spans = vec![
                Span::styled("En transfert: ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Span::styled(format!("{} ", af.name), Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)),
                Span::styled(format!("({}%", af.pct), Style::default().fg(theme.highlight)),
                Span::styled(if af.speed.is_empty() { ")".to_string() } else { format!(" - {})", af.speed) }, Style::default().fg(theme.text_muted)),
            ];
            content_lines.push(Line::from(active_spans));
        }
    }

    let p = Paragraph::new(content_lines);
    f.render_widget(p, inner);
}

fn render_history_panel(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect, hitboxes: &mut Vec<Hitbox>) {
    let is_focused = app.focused_panel == FocusedPanel::History;
    let border_color = if is_focused { theme.border_focus } else { theme.border_history };
    let is_syncing = app.live.is_syncing;
    let total_runs = app.total_history_runs();
    let cur_run = if total_runs > 0 { app.selected_run_idx + 1 } else { 0 };

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

    let left_bottom = Line::from(vec![
        Span::styled("┘", Style::default().fg(border_color)),
        Span::styled("↑ select ↓", Style::default().fg(theme.text_bright)),
        Span::styled("└┘", Style::default().fg(border_color)),
        Span::styled("détails ", Style::default().fg(theme.text_bright)),
        Span::styled("↵", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
        Span::styled("└┘", Style::default().fg(border_color)),
        Span::styled("c", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
        Span::styled("ancel", Style::default().fg(theme.text_bright)),
        Span::styled("└", Style::default().fg(border_color)),
    ]);
    let right_bottom = Line::from(vec![
        Span::styled(format!("─ {}/{} ─", cur_run, total_runs), Style::default().fg(border_color).add_modifier(Modifier::BOLD)),
    ]);

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.card_bg))
        .title(Line::from(vec![
            Span::styled("┐", Style::default().fg(border_color)),
            Span::styled("history", Style::default().fg(border_color).add_modifier(Modifier::BOLD)),
            Span::styled("┌┐", Style::default().fg(border_color)),
            Span::styled("details ", Style::default().fg(theme.text_bright)),
            Span::styled("↵", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
            Span::styled("┌", Style::default().fg(border_color)),
        ]))
        .title_bottom(left_bottom.alignment(Alignment::Left))
        .title_bottom(right_bottom.alignment(Alignment::Right));
    f.render_widget(outer_block, area);

    // Hitbox pour les onglets du bas du panneau historique
    let bottom_y = area.y + area.height.saturating_sub(1);
    hitboxes.push(Hitbox {
        rect: Rect { x: area.x + 11, y: bottom_y, width: 11, height: 1 },
        action: HitAction::HistoryRow(app.selected_run_idx),
    });
    hitboxes.push(Hitbox {
        rect: Rect { x: area.x + 22, y: bottom_y, width: 8, height: 1 },
        action: HitAction::ButtonCancel,
    });

    // 1. Graphe de durées multi-lignes (btop style) avec hitboxes sur chaque colonne
    let past_selected = if is_syncing {
        if app.selected_run_idx > 0 { Some(app.selected_run_idx - 1) } else { None }
    } else if !app.past_runs.is_empty() {
        Some(app.selected_run_idx)
    } else {
        None
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
    let offset = if app.selected_run_idx < app.history_scroll_offset {
        app.selected_run_idx
    } else if visible_rows > 0 && app.selected_run_idx >= app.history_scroll_offset + visible_rows {
        app.selected_run_idx.saturating_sub(visible_rows) + 1
    } else {
        app.history_scroll_offset.min(max_offset)
    };

    let mut table_rows: Vec<Row> = Vec::new();
    let mut visible_indices: Vec<usize> = Vec::new();

    for item_idx in offset..(offset + visible_rows).min(total_runs) {
        visible_indices.push(item_idx);
        let is_selected = item_idx == app.selected_run_idx;
        let cursor = if is_selected {
            Span::styled("▶ ", Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD))
        } else {
            Span::styled("  ", Style::default())
        };

        if is_syncing && item_idx == 0 {
            // Ligne de la synchronisation en cours
            let elapsed = if app.live.transfer.elapsed.is_empty() { "0s" } else { &app.live.transfer.elapsed };
            let time_str = format!("En cours ({})", elapsed);
            let pct_val = app.live.transfer.pct.min(100);
            let speed_val = if app.live.transfer.speed.is_empty() { "--" } else { &app.live.transfer.speed };
            let status_str = format!("● {}% ({})", pct_val, speed_val);
            let copied_val = if app.live.transfer.bytes_done.is_empty() { "0 B".to_string() } else { app.live.transfer.bytes_done.clone() };
            let mod_val = if app.live.changes_remote.is_empty() { "0".to_string() } else { format!("{}", app.live.changes_remote.len()) };
            let chk_val = format!("{}/{}", app.live.transfer.files_done, app.live.transfer.files_total);
            let eta_val = if app.live.transfer.eta.is_empty() { "--".to_string() } else { format!("ETA:{}", app.live.transfer.eta) };

            table_rows.push(Row::new(vec![
                Cell::from(Line::from(vec![cursor, Span::styled(time_str, Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD))])),
                Cell::from(Span::styled(status_str, Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD))),
                Cell::from(Span::styled(copied_val, Style::default().fg(theme.green).add_modifier(Modifier::BOLD))),
                Cell::from(Span::styled(mod_val, Style::default().fg(theme.yellow))),
                Cell::from(Span::styled(chk_val, Style::default().fg(theme.accent))),
                Cell::from(Span::styled(elapsed, Style::default().fg(theme.text_bright))),
                Cell::from(Span::styled(eta_val, Style::default().fg(theme.text_muted))),
            ]));
        } else {
            let past_idx = if is_syncing { item_idx - 1 } else { item_idx };
            if let Some(run) = app.past_runs.get(past_idx) {
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

                table_rows.push(Row::new(vec![
                    Cell::from(Line::from(vec![cursor, Span::styled(time_short, row_style)])),
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
        Row::new(vec!["DÉMARRAGE", "STATUT", "COPIÉS", "MODIF.", "SUPPR.", "DURÉE", "ERR."])
            .style(Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
    );

    f.render_widget(table, chunks[1]);

    if total_runs > visible_rows {
        let mut scrollbar_state = ScrollbarState::new(total_runs)
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

    let visible_height = area.height.saturating_sub(2) as usize;
    let max_scroll = total_lines.saturating_sub(visible_height);
    let effective_scroll = app.logs_scroll.min(max_scroll);

    let cur_line = if app.auto_scroll {
        total_lines
    } else {
        total_lines.saturating_sub(effective_scroll)
    };

    let left_bottom = Line::from(vec![
        Span::styled("┘", Style::default().fg(border_color)),
        Span::styled("↑ scroll ↓", Style::default().fg(theme.text_bright)),
        Span::styled("└┘", Style::default().fg(border_color)),
        Span::styled(if app.auto_scroll { "pause " } else { "auto " }, Style::default().fg(theme.text_bright)),
        Span::styled("␣", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
        Span::styled("└", Style::default().fg(border_color)),
    ]);
    let right_bottom = Line::from(vec![
        Span::styled(format!("─ {}/{} ─", cur_line, total_lines), Style::default().fg(border_color).add_modifier(Modifier::BOLD)),
    ]);

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.card_bg))
        .title(Line::from(vec![
            Span::styled("┐", Style::default().fg(border_color)),
            Span::styled("logs", Style::default().fg(border_color).add_modifier(Modifier::BOLD)),
            Span::styled("┌┐", Style::default().fg(border_color)),
            Span::styled("auto ", Style::default().fg(theme.text_bright)),
            Span::styled("␣", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
            Span::styled(if app.auto_scroll { " [ON]" } else { " [OFF]" }, Style::default().fg(if app.auto_scroll { theme.green } else { theme.yellow }).add_modifier(Modifier::BOLD)),
            Span::styled("┌", Style::default().fg(border_color)),
        ]))
        .title_bottom(left_bottom.alignment(Alignment::Left))
        .title_bottom(right_bottom.alignment(Alignment::Right));

    let bottom_y = area.y + area.height.saturating_sub(1);
    hitboxes.push(Hitbox {
        rect: Rect { x: area.x + 11, y: bottom_y, width: 9, height: 1 },
        action: HitAction::ToggleLogsAuto,
    });

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

    // Alimentation de l'intégralité des fichiers récents (parité web)
    let files_to_display = app.get_all_recent_files();

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

                let file_path = std::path::Path::new(path);
                let (display_path, path_style) = if app.ctrl_mode {
                    let parent = file_path.parent().and_then(|p| p.to_str()).unwrap_or("");
                    let parent_clean = parent.trim_start_matches('/').trim_end_matches('/');
                    let formatted = if parent_clean.is_empty() {
                        "📁 ./".to_string()
                    } else {
                        format!("📁 {}/", parent_clean)
                    };
                    if is_selected {
                        (formatted, Style::default().fg(theme.yellow).bg(theme.border_focus).add_modifier(Modifier::BOLD))
                    } else {
                        (formatted, Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD))
                    }
                } else {
                    (path.clone(), row_style)
                };

                Row::new(vec![
                    Cell::from(Line::from(vec![
                        Span::styled(prefix, Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD)),
                        Span::styled(format!("{} ", badge_text), Style::default().fg(badge_color).add_modifier(Modifier::BOLD)),
                    ])),
                    Cell::from(Span::styled(display_path, path_style)),
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

    let mut bottom_spans = vec![
        Span::styled("┘", Style::default().fg(border_color)),
        Span::styled("↑ select ↓", Style::default().fg(theme.text_bright)),
        Span::styled("└┘", Style::default().fg(border_color)),
        Span::styled("open ", Style::default().fg(theme.text_bright)),
        Span::styled("↵", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
        Span::styled("└┘", Style::default().fg(border_color)),
    ];
    if app.ctrl_mode {
        bottom_spans.push(Span::styled("Ctrl+X", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)));
        bottom_spans.push(Span::styled(" dossier [ON]", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)));
    } else {
        bottom_spans.push(Span::styled("Ctrl+X", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)));
        bottom_spans.push(Span::styled(" dossier", Style::default().fg(theme.text_bright)));
    }
    bottom_spans.push(Span::styled("└", Style::default().fg(border_color)));

    let left_bottom = Line::from(bottom_spans);
    let right_bottom = Line::from(vec![
        Span::styled(format!("─ {}/{} ─", cur_file, total_files), Style::default().fg(border_color).add_modifier(Modifier::BOLD)),
    ]);

    let mut top_spans = vec![
        Span::styled("┐", Style::default().fg(border_color)),
        Span::styled("recent files", Style::default().fg(border_color).add_modifier(Modifier::BOLD)),
        Span::styled("┌┐", Style::default().fg(border_color)),
    ];
    if app.is_filtering_recent {
        top_spans.push(Span::styled("filter: ", Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD)));
        top_spans.push(Span::styled(&app.recent_filter, Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)));
        top_spans.push(Span::styled("█", Style::default().fg(theme.highlight)));
    } else if !app.recent_filter.is_empty() {
        top_spans.push(Span::styled("filter: ", Style::default().fg(theme.text_muted)));
        top_spans.push(Span::styled(&app.recent_filter, Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)));
        top_spans.push(Span::styled(" (", Style::default().fg(theme.text_muted)));
        top_spans.push(Span::styled("f", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)));
        top_spans.push(Span::styled(")", Style::default().fg(theme.text_muted)));
    } else {
        top_spans.push(Span::styled("f", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)));
        top_spans.push(Span::styled(" filter", Style::default().fg(theme.text_bright)));
    }
    top_spans.push(Span::styled("┌┐", Style::default().fg(border_color)));
    top_spans.push(Span::styled("open ", Style::default().fg(theme.text_bright)));
    top_spans.push(Span::styled("↵", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),);
    top_spans.push(Span::styled("┌┐", Style::default().fg(border_color)));
    top_spans.push(Span::styled("Ctrl+X", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)));
    top_spans.push(Span::styled(if app.ctrl_mode { " dossier [ON]" } else { " dossier" }, Style::default().fg(if app.ctrl_mode { theme.yellow } else { theme.text_bright }).add_modifier(Modifier::BOLD)));
    top_spans.push(Span::styled("┌", Style::default().fg(border_color)));

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.card_bg))
        .title(Line::from(top_spans))
        .title_bottom(left_bottom.alignment(Alignment::Left))
        .title_bottom(right_bottom.alignment(Alignment::Right));

    // Hitbox pour l'onglet filter dans le bandeau supérieur
    hitboxes.push(Hitbox {
        rect: Rect { x: area.x + 14, y: area.y, width: 12, height: 1 },
        action: HitAction::RecentFilterFocus,
    });

    let bottom_y = area.y + area.height.saturating_sub(1);
    hitboxes.push(Hitbox {
        rect: Rect { x: area.x + 11, y: bottom_y, width: 8, height: 1 },
        action: HitAction::RecentFile(app.recent_selected_idx),
    });
    hitboxes.push(Hitbox {
        rect: Rect { x: area.x + 20, y: bottom_y, width: if app.ctrl_mode { 17 } else { 12 }, height: 1 },
        action: HitAction::ToggleCtrlMode,
    });

    let header_title = if app.ctrl_mode { "DOSSIER PARENT (MODE CTRL ACTIF)" } else { "CHEMIN DU FICHIER" };
    let table = Table::new(
        rows,
        [
            Constraint::Percentage(16),
            Constraint::Percentage(68),
            Constraint::Percentage(16),
        ],
    )
    .header(
        Row::new(vec!["ACTION", header_title, "HEURE"])
            .style(Style::default().fg(if app.ctrl_mode { theme.yellow } else { theme.text_muted }).add_modifier(Modifier::BOLD)),
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
