use chrono::Local;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::{Alert, AlertLevel, App, HitAction, Hitbox};
use crate::ui::theme::ThemePalette;

/// Renders the two system dials: Storage/Cloud on the left, Metrics/Reliability on the right.
pub fn render_system_cadrans(
    f: &mut Frame,
    app: &App,
    theme: &ThemePalette,
    area: Rect,
    hitboxes: &mut Vec<Hitbox>,
) {
    match (app.is_box_visible(1), app.is_box_visible(2)) {
        (true, true) => {
            let sub = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);
            render_disks_cloud_box(f, app, theme, sub[0], hitboxes);
            render_metrics_box(f, app, theme, sub[1], hitboxes);
        }
        (true, false) => {
            render_disks_cloud_box(f, app, theme, area, hitboxes);
        }
        (false, true) => {
            render_metrics_box(f, app, theme, area, hitboxes);
        }
        (false, false) => {}
    }
}

/// Renders the Disks & Cloud Storage status card.
pub fn render_disks_cloud_box(
    f: &mut Frame,
    app: &App,
    theme: &ThemePalette,
    area: Rect,
    hitboxes: &mut Vec<Hitbox>,
) {
    let bg = app.border_glyphs();
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(app.border_type())
        .border_style(Style::default().fg(theme.border_storage))
        .style(Style::default().bg(theme.card_bg))
        .title(Line::from(vec![
            Span::styled(bg.top_left, Style::default().fg(theme.border_storage)),
            Span::styled("¹", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
            Span::styled("disks & cloud", Style::default().fg(theme.border_storage).add_modifier(Modifier::BOLD)),
            Span::styled(bg.top_right, Style::default().fg(theme.border_storage)),
        ]))
        .title_bottom(
            Line::from(vec![
                Span::styled(format!("{} statvfs / bisync {}", bg.horizontal, bg.horizontal), Style::default().fg(theme.border_storage)),
            ])
            .alignment(Alignment::Right),
        );
    f.render_widget(outer_block, area);

    hitboxes.push(Hitbox {
        rect: Rect {
            x: area.x,
            y: area.y,
            width: 18,
            height: 1,
        },
        action: HitAction::ToggleBox(1),
    });

    let inner = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(2),
    };

    let bar_width = 8usize.min((inner.width.saturating_sub(48) / 2) as usize);

    // 1. Local disk stats via statvfs
    let (used_gb, free_gb, total_gb, pct) = app.local_disk_stats();
    let disk_bar = crate::ui::sparkline::render_gradient_bar(pct, bar_width, app.config.graph_style, theme);
    let mut line1_spans = vec![
        Span::styled("Disk:    ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
    ];
    line1_spans.extend(disk_bar);
    let disk_info = if inner.width >= 55 {
        format!(" {:>3.0}%  {:.1} GB / {:.1} GB ({:.1} GB free)", pct, used_gb, total_gb, free_gb)
    } else if inner.width >= 42 {
        format!(" {:>3.0}%  {:.0}G/{:.0}G ({:.0}G free)", pct, used_gb, total_gb, free_gb)
    } else {
        format!(" {:>3.0}%  {:.1}G free", pct, free_gb)
    };
    line1_spans.push(Span::styled(
        disk_info,
        Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD),
    ));

    // 2. Cloud storage quota
    let mut line2_spans = vec![
        Span::styled("Cloud:   ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
    ];
    if let Some(q) = &app.cloud_quota {
        let used_mb = q.used_bytes as f64 / (1024.0 * 1024.0);
        let total_tb = q.total_bytes as f64 / (1024.0 * 1024.0 * 1024.0 * 1024.0);
        let free_tb = q.free_bytes as f64 / (1024.0 * 1024.0 * 1024.0 * 1024.0);
        let q_pct = if q.total_bytes > 0 { (q.used_bytes as f64 / q.total_bytes as f64) * 100.0 } else { 0.0 };
        let cloud_bar = crate::ui::sparkline::render_gradient_bar(q_pct, bar_width, app.config.graph_style, theme);
        line2_spans.extend(cloud_bar);
        let used_str = if used_mb >= 1024.0 {
            format!("{:.1} GB", used_mb / 1024.0)
        } else {
            format!("{:.1} MB", used_mb)
        };
        let cloud_info = if inner.width >= 55 {
            format!(" {:>3.0}%  {} / {:.2} TB ({:.2} TB free)", q_pct, used_str, total_tb, free_tb)
        } else if inner.width >= 42 {
            format!(" {:>3.0}%  {}/{:.1}TB", q_pct, used_str, total_tb)
        } else {
            format!(" {:>3.0}%  {:.1}TB free", q_pct, free_tb)
        };
        line2_spans.push(Span::styled(
            cloud_info,
            Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD),
        ));
    } else {
        let remote_spans = crate::ui::theme::truncate_with_fade_spans(
            &app.config.remote,
            15,
            Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD),
            false,
            None,
        );
        let rem_len = app.config.remote.chars().count();
        line2_spans.extend(remote_spans);
        if rem_len < 15 {
            line2_spans.push(Span::styled(" ".repeat(15 - rem_len + 1), Style::default()));
        } else {
            line2_spans.push(Span::styled(" ", Style::default()));
        }
        line2_spans.push(Span::styled("· syncing quota...", Style::default().fg(theme.text_muted)));
    }

    // 3. Local folder & tracked file count
    let count = app.get_tracked_files_count();
    let count_str = if count > 0 {
        format!("{} tracked files", count)
    } else {
        "analyzing...".to_string()
    };
    let max_dir_len = (inner.width.saturating_sub(24) as usize).max(8);
    let dir_spans = crate::ui::theme::truncate_with_fade_spans(
        &app.config.local_dir,
        max_dir_len,
        Style::default().fg(theme.text_bright),
        true,
        None,
    );
    let mut line3_spans = vec![
        Span::styled("Folder:  ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
    ];
    line3_spans.extend(dir_spans);
    line3_spans.push(Span::styled(format!(" ({})", count_str), Style::default().fg(theme.text_muted)));

    // 4. Cloud Safety Net & bwlimit
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
            Span::styled(format!("{} ", app.config.local_dir), Style::default().fg(theme.text_bright)),
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

/// Renders the Metrics, Service State, and Reliability card.
pub fn render_metrics_box(
    f: &mut Frame,
    app: &App,
    theme: &ThemePalette,
    area: Rect,
    hitboxes: &mut Vec<Hitbox>,
) {
    let clock_str = Local::now().format("%H:%M:%S").to_string();
    let stats_interval_str = &app.config.stats_interval;

    // Hitboxes for stats interval stepper [ - ] and [ + ]
    let widget_len = (stats_interval_str.chars().count() + 4) as u16;
    let widget_x = area.x + area.width.saturating_sub(widget_len + 1);

    hitboxes.push(Hitbox {
        rect: Rect { x: widget_x + 1, y: area.y, width: 2, height: 1 },
        action: HitAction::StatsIntervalDec,
    });
    hitboxes.push(Hitbox {
        rect: Rect { x: widget_x + widget_len.saturating_sub(2), y: area.y, width: 2, height: 1 },
        action: HitAction::StatsIntervalInc,
    });

    let bg = app.border_glyphs();
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(app.border_type())
        .border_style(Style::default().fg(theme.border_sys))
        .style(Style::default().bg(theme.card_bg))
        .title(Line::from(vec![
            Span::styled(bg.top_left, Style::default().fg(theme.border_sys)),
            Span::styled("²", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
            Span::styled("metrics", Style::default().fg(theme.border_sys).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{} ", bg.horizontal), Style::default().fg(theme.border_sys)),
            Span::styled(clock_str, Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)),
            Span::styled(format!(" {}", bg.horizontal), Style::default().fg(theme.border_sys)),
        ]))
        .title({
            let dec_col = if app.can_dec_stats_interval() { theme.red } else { theme.text_muted };
            let inc_col = if app.can_inc_stats_interval() { theme.red } else { theme.text_muted };
            Line::from(vec![
                Span::styled(bg.top_left, Style::default().fg(theme.border_sys)),
                Span::styled("-", Style::default().fg(dec_col).add_modifier(Modifier::BOLD)),
                Span::styled(" ", Style::default().fg(theme.border_sys)),
                Span::styled(stats_interval_str, Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)),
                Span::styled(" ", Style::default().fg(theme.border_sys)),
                Span::styled("+", Style::default().fg(inc_col).add_modifier(Modifier::BOLD)),
                Span::styled(bg.top_right, Style::default().fg(theme.border_sys)),
            ])
            .alignment(Alignment::Right)
        });
    f.render_widget(outer_block, area);

    hitboxes.push(Hitbox {
        rect: Rect {
            x: area.x,
            y: area.y,
            width: 12,
            height: 1,
        },
        action: HitAction::ToggleBox(2),
    });

    let inner = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(2),
    };

    // 1. Rclone service status & schedules
    let is_active = app.is_syncing();
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

    // 2. Today statistics: successful syncs, errors, conflicts
    let (ok_today, err_today) = app.runs_today_stats();
    let conflicts = app.conflicts_today_stats();
    let line2_spans = vec![
        Span::styled("Today:     ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{} successful sync(s)", ok_today), Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
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

    // 3. 7-day reliability calculation & solid progress bar
    let (rate, _) = app.calculate_success_rate();
    let bar_width = 8usize.min((inner.width.saturating_sub(48) / 2) as usize);
    let rel_color = if rate >= 90.0 {
        theme.green
    } else if rate >= 70.0 {
        theme.yellow
    } else {
        theme.red
    };
    let rel_bar = crate::ui::sparkline::render_reliability_bar(rate, bar_width, app.config.graph_style, theme);

    let mut line3_spans = vec![
        Span::styled("Reliability: ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
    ];
    line3_spans.extend(rel_bar.clone());
    line3_spans.push(Span::styled(
        format!(" {:>3.0}% success rate (7 days)", rate),
        Style::default().fg(rel_color).add_modifier(Modifier::BOLD),
    ));

    let mut lines = vec![Line::from(line1_spans)];
    if inner.height >= 3 {
        lines.push(Line::from(line2_spans));
        lines.push(Line::from(line3_spans));
    } else if inner.height >= 2 {
        let mut line2_compact = vec![
            Span::styled("Today: ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{} ok ", ok_today), Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
            Span::styled("│ Rel: ", Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
        ];
        line2_compact.extend(rel_bar);
        line2_compact.push(Span::styled(
            format!(" {:>3.0}%", rate),
            Style::default().fg(rel_color).add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from(line2_compact));
    }

    let p = Paragraph::new(lines);
    f.render_widget(p, inner);
}

/// Renders a critical alert or warning banner across the dashboard.
pub fn render_alert_banner(f: &mut Frame, app: &App, alert: &Alert, theme: &ThemePalette, area: Rect) {
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
                .border_type(app.border_type())
                .border_style(Style::default().fg(border_color))
                .style(Style::default().bg(theme.card_bg)),
        );

    f.render_widget(p, area);
}
