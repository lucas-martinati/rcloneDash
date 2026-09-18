use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

use crate::app::{App, HitAction, Hitbox};
use crate::systemd::ServiceState;
use crate::ui::theme::ThemePalette;

pub fn render_header(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect, hitboxes: &mut Vec<Hitbox>) {
    if area.height < 1 {
        return;
    }

    // Fond global
    f.render_widget(Block::default().style(Style::default().bg(theme.header_bg)), area);

    let is_active = app.service_info.state == ServiceState::Active || app.live.is_syncing;
    let is_wide = area.width >= 120;

    // --- LIGNE 1 : Barre de bordure btop++ avec onglets / boutons intégrés ---
    let mut line1_spans: Vec<Span> = Vec::new();
    let mut cur_x = area.x;

    // 1. Cadran Système ¹sys
    let sys_tab = if is_wide { "┌¹sys┐" } else { "┌¹s┐" };
    line1_spans.push(Span::styled("┌", Style::default().fg(theme.border)));
    line1_spans.push(Span::styled(if is_wide { "¹sys" } else { "¹s" }, Style::default().fg(theme.border_sys).add_modifier(Modifier::BOLD)));
    line1_spans.push(Span::styled("┐", Style::default().fg(theme.border)));
    cur_x += sys_tab.chars().count() as u16;

    // 2. Onglet Menu [m]
    let menu_label = if is_wide { "menu m" } else { "m" };
    let menu_len = (menu_label.chars().count() + 2) as u16;
    hitboxes.push(Hitbox {
        rect: Rect { x: cur_x, y: area.y, width: menu_len, height: 1 },
        action: HitAction::ButtonMenu,
    });
    line1_spans.push(Span::styled("┌", Style::default().fg(theme.border)));
    line1_spans.push(Span::styled(menu_label, Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD)));
    line1_spans.push(Span::styled("┐", Style::default().fg(theme.border)));
    cur_x += menu_len;

    // 3. Bouton Sync [s] ou Arrêter [c]
    let (sync_text, sync_color, sync_action) = if is_active {
        (if is_wide { "⏹ stop c" } else { "⏹ c" }, theme.red, HitAction::ButtonCancel)
    } else {
        (if is_wide { "⟳ sync s" } else { "⟳ s" }, theme.green, HitAction::ButtonSync)
    };
    let sync_len = (sync_text.chars().count() + 2) as u16;
    hitboxes.push(Hitbox {
        rect: Rect { x: cur_x, y: area.y, width: sync_len, height: 1 },
        action: sync_action,
    });
    line1_spans.push(Span::styled("┌", Style::default().fg(theme.border)));
    line1_spans.push(Span::styled(sync_text, Style::default().fg(sync_color).add_modifier(Modifier::BOLD)));
    line1_spans.push(Span::styled("┐", Style::default().fg(theme.border)));
    cur_x += sync_len;

    // 4. Bouton Dry-Run [d]
    let dry_text = if is_wide { "🛡 dry-run d" } else { "🛡 d" };
    let dry_len = (dry_text.chars().count() + 2) as u16;
    hitboxes.push(Hitbox {
        rect: Rect { x: cur_x, y: area.y, width: dry_len, height: 1 },
        action: HitAction::ButtonDryRun,
    });
    line1_spans.push(Span::styled("┌", Style::default().fg(theme.border)));
    line1_spans.push(Span::styled(dry_text, Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD)));
    line1_spans.push(Span::styled("┐", Style::default().fg(theme.border)));
    cur_x += dry_len;

    // 5. Bouton Fichiers [f]
    let files_text = if is_wide { "📁 files f" } else { "📁 f" };
    let files_len = (files_text.chars().count() + 2) as u16;
    hitboxes.push(Hitbox {
        rect: Rect { x: cur_x, y: area.y, width: files_len, height: 1 },
        action: HitAction::ButtonFiles,
    });
    line1_spans.push(Span::styled("┌", Style::default().fg(theme.border)));
    line1_spans.push(Span::styled(files_text, Style::default().fg(theme.blue).add_modifier(Modifier::BOLD)));
    line1_spans.push(Span::styled("┐", Style::default().fg(theme.border)));
    cur_x += files_len;

    // 6. Bouton Filtres [e]
    let filter_text = if is_wide { "⊘ filtres e" } else { "⊘ e" };
    let filter_len = (filter_text.chars().count() + 2) as u16;
    hitboxes.push(Hitbox {
        rect: Rect { x: cur_x, y: area.y, width: filter_len, height: 1 },
        action: HitAction::ButtonFilters,
    });
    line1_spans.push(Span::styled("┌", Style::default().fg(theme.border)));
    line1_spans.push(Span::styled(filter_text, Style::default().fg(theme.purple).add_modifier(Modifier::BOLD)));
    line1_spans.push(Span::styled("┐", Style::default().fg(theme.border)));
    cur_x += filter_len;

    // 7. Bouton Options [o]
    let opt_text = if is_wide { "⚙ options o" } else { "⚙ o" };
    let opt_len = (opt_text.chars().count() + 2) as u16;
    hitboxes.push(Hitbox {
        rect: Rect { x: cur_x, y: area.y, width: opt_len, height: 1 },
        action: HitAction::ButtonSettings,
    });
    line1_spans.push(Span::styled("┌", Style::default().fg(theme.border)));
    line1_spans.push(Span::styled(opt_text, Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)));
    line1_spans.push(Span::styled("┐", Style::default().fg(theme.border)));
    cur_x += opt_len;

    // Horloge digitale centrale & Stepper tick rate à droite
    let clock_str = chrono::Local::now().format("%H:%M:%S").to_string();
    let clock_badge = format!("─ {} ─", clock_str);
    let clock_len = clock_badge.chars().count() as u16;

    let tick_ms = app.tick_rate_ms_live;
    let tick_str = format!("{}ms", tick_ms);
    // Format: ┌- 250ms +┐
    let tick_widget_len = (tick_str.chars().count() + 6) as u16; // "┌- " + ms + " +┐"

    let right_start_x = area.x + area.width.saturating_sub(tick_widget_len);

    if right_start_x > cur_x + clock_len + 4 {
        let total_gap = right_start_x - cur_x;
        let left_pad = (total_gap.saturating_sub(clock_len)) / 2;
        let right_pad = total_gap.saturating_sub(clock_len).saturating_sub(left_pad);

        line1_spans.push(Span::styled("─".repeat(left_pad as usize), Style::default().fg(theme.border)));
        line1_spans.push(Span::styled(format!("─ {} ─", clock_str), Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)));
        line1_spans.push(Span::styled("─".repeat(right_pad as usize), Style::default().fg(theme.border)));
    } else if right_start_x > cur_x {
        line1_spans.push(Span::styled("─".repeat((right_start_x - cur_x) as usize), Style::default().fg(theme.border)));
    }

    // Tick rate widget: ┌- 250ms +┐
    hitboxes.push(Hitbox {
        rect: Rect { x: right_start_x, y: area.y, width: 3, height: 1 },
        action: HitAction::TickRateDec,
    });
    hitboxes.push(Hitbox {
        rect: Rect { x: right_start_x + tick_widget_len - 3, y: area.y, width: 3, height: 1 },
        action: HitAction::TickRateInc,
    });

    line1_spans.push(Span::styled("┌- ", Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD)));
    line1_spans.push(Span::styled(tick_str, Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)));
    line1_spans.push(Span::styled(" +┐", Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD)));

    let line1_p = Paragraph::new(Line::from(line1_spans));
    f.render_widget(line1_p, Rect { x: area.x, y: area.y, width: area.width, height: 1 });

    if area.height < 2 {
        return;
    }

    // --- LIGNE 2 : Statut en direct, Phase, Horaires & Alertes ---
    let pulse_cycle = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() / 500) % 2;
    let (status_dot, status_text, status_color) = if is_active {
        let dot = if pulse_cycle == 0 { "●" } else { "◉" };
        (dot, "SYNC ACTIVE", theme.green)
    } else {
        match app.service_info.state {
            ServiceState::Idle => ("●", "EN ATTENTE", theme.cyan),
            ServiceState::Failed => ("●", "ÉCHEC", theme.red),
            _ => ("●", "INCONNU", theme.text_muted),
        }
    };

    let last_sync_str = app.past_runs.first()
        .map(|r| format!("{} ({})", r.time, r.duration))
        .unwrap_or_else(|| "--".to_string());

    let next_sync_str = if app.service_info.timer_left.is_empty() {
        "--".to_string()
    } else {
        app.service_info.timer_left.clone()
    };

    let mut line2_spans = vec![
        Span::styled(" ⚡ ", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)),
        Span::styled("Rclone", Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)),
        Span::styled("Dash ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled("│ ", Style::default().fg(theme.border)),
        Span::styled(format!("{} {} ", status_dot, status_text), Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
        Span::styled("│ ", Style::default().fg(theme.border)),
    ];

    if is_active && !app.live.phase.is_empty() {
        line2_spans.push(Span::styled("Phase: ", Style::default().fg(theme.text_muted)));
        line2_spans.push(Span::styled(format!("{} ", app.live.phase), Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD)));
        line2_spans.push(Span::styled("│ ", Style::default().fg(theme.border)));
    }

    line2_spans.push(Span::styled("Dernière: ", Style::default().fg(theme.text_muted)));
    line2_spans.push(Span::styled(format!("{} ", last_sync_str), Style::default().fg(theme.text_bright)));
    line2_spans.push(Span::styled("│ ", Style::default().fg(theme.border)));
    line2_spans.push(Span::styled("Prochaine: ", Style::default().fg(theme.text_muted)));
    line2_spans.push(Span::styled(format!("{} ", next_sync_str), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)));

    // Toast message s'il existe
    if let Some((toast_msg, _)) = &app.toast {
        line2_spans.push(Span::styled("│ ", Style::default().fg(theme.border)));
        line2_spans.push(Span::styled(format!(" ★ {} ", toast_msg), Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD)));
    }

    let line2_p = Paragraph::new(Line::from(line2_spans));
    f.render_widget(line2_p, Rect { x: area.x, y: area.y + 1, width: area.width, height: 1 });
}
