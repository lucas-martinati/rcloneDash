use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

use crate::app::{App, HitAction, Hitbox};
use crate::ui::theme::ThemePalette;

pub mod cadrans;
pub mod history;
pub mod logs;
pub mod recent;
pub mod sync;

pub use cadrans::*;
pub use history::*;
pub use logs::*;
pub use recent::*;
pub use sync::*;

#[derive(Debug, Clone, Copy)]
pub struct DashboardLayout {
    pub cadrans_area: Rect,
    pub alert_area: Option<Rect>,
    pub sync_area: Option<Rect>,
    pub history_area: Rect,
    pub logs_area: Rect,
    pub recent_area: Rect,
}

/// Computes dynamic dashboard layout geometry based on active alerts, ongoing syncs, and container layout preferences.
pub fn compute_dashboard_layout(area: Rect, app: &App) -> DashboardLayout {
    let show_alert = !app.active_alerts().is_empty();
    let show_active_sync = app.is_syncing();

    let cadrans_h: u16 = if app.is_box_visible(1) || app.is_box_visible(2) { 6 } else { 0 };
    let sys_h: u16 = cadrans_h + if show_alert { 3 } else { 0 } + if show_active_sync { 7 } else { 0 };

    let show_mid = app.is_box_visible(3) || app.is_box_visible(4);
    let show_recent = app.is_box_visible(5);

    let rem_h = area.height.saturating_sub(sys_h);

    let (mid_h, recent_h) = match (show_mid, show_recent) {
        (true, true) => {
            let m = (rem_h * 62 / 100).max(8);
            let r = rem_h.saturating_sub(m).max(4);
            (m, r)
        }
        (true, false) => (rem_h, 0),
        (false, true) => (0, rem_h),
        (false, false) => (0, 0),
    };

    let sys_constraint = if !show_mid && !show_recent {
        Constraint::Min(sys_h)
    } else {
        Constraint::Length(sys_h)
    };

    let (sys_rect, mid_rect, recent_rect) = match app.config.container_layout {
        crate::config::ContainerLayout::Default => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    sys_constraint,
                    Constraint::Length(mid_h),
                    Constraint::Min(recent_h),
                ])
                .split(area);
            (chunks[0], chunks[1], chunks[2])
        }
        crate::config::ContainerLayout::RecentFirst => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    sys_constraint,
                    Constraint::Length(recent_h),
                    Constraint::Min(mid_h),
                ])
                .split(area);
            (chunks[0], chunks[2], chunks[1])
        }
        crate::config::ContainerLayout::LogsTop => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(mid_h),
                    sys_constraint,
                    Constraint::Min(recent_h),
                ])
                .split(area);
            (chunks[1], chunks[0], chunks[2])
        }
        crate::config::ContainerLayout::Inverted => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(recent_h),
                    Constraint::Length(mid_h),
                    sys_constraint,
                ])
                .split(area);
            (chunks[2], chunks[1], chunks[0])
        }
    };

    // Subdivide sys_rect into cadrans, alert, sync
    let mut sys_constraints = Vec::new();
    if app.is_box_visible(1) || app.is_box_visible(2) {
        if !show_alert && !show_active_sync {
            sys_constraints.push(Constraint::Min(6));
        } else {
            sys_constraints.push(Constraint::Length(6));
        }
    }
    if show_alert {
        sys_constraints.push(Constraint::Length(3));
    }
    if show_active_sync {
        sys_constraints.push(Constraint::Length(7));
    }

    let (cadrans_area, alert_area, sync_area) = if sys_constraints.is_empty() || sys_rect.height == 0 {
        (Rect::default(), None, None)
    } else {
        let sys_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(sys_constraints)
            .split(sys_rect);

        let mut idx = 0;
        let c_area = if app.is_box_visible(1) || app.is_box_visible(2) {
            let a = sys_chunks[idx];
            idx += 1;
            a
        } else {
            Rect::default()
        };
        let al_area = if show_alert {
            let a = sys_chunks[idx];
            idx += 1;
            Some(a)
        } else {
            None
        };
        let sy_area = if show_active_sync {
            Some(sys_chunks[idx])
        } else {
            None
        };
        (c_area, al_area, sy_area)
    };

    // Subdivide mid_rect horizontally according to mid_panel_order and visibility
    let (history_area, logs_area) = match (app.is_box_visible(3), app.is_box_visible(4)) {
        (true, true) => {
            let mid_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(mid_rect);
            match app.config.mid_panel_order {
                crate::config::MidPanelOrder::HistoryLogs => (mid_chunks[0], mid_chunks[1]),
                crate::config::MidPanelOrder::LogsHistory => (mid_chunks[1], mid_chunks[0]),
            }
        }
        (true, false) => (mid_rect, Rect::default()),
        (false, true) => (Rect::default(), mid_rect),
        (false, false) => (Rect::default(), Rect::default()),
    };

    let recent_area = if app.is_box_visible(5) {
        recent_rect
    } else {
        Rect::default()
    };

    DashboardLayout {
        cadrans_area,
        alert_area,
        sync_area,
        history_area,
        logs_area,
        recent_area,
    }
}

/// Renders the complete Dashboard view with responsive dynamic layouts.
pub fn render_dashboard(
    f: &mut Frame,
    app: &App,
    theme: &ThemePalette,
    area: Rect,
    hitboxes: &mut Vec<Hitbox>,
) {
    if !app.any_box_visible() {
        render_empty_dashboard(f, app, theme, area, hitboxes);
        return;
    }

    let layout = compute_dashboard_layout(area, app);

    // 1. Storage & Metrics dials
    if (app.is_box_visible(1) || app.is_box_visible(2)) && layout.cadrans_area.height >= 4 {
        render_system_cadrans(f, app, theme, layout.cadrans_area, hitboxes);
    }

    // 2. Alert banner (if any active)
    if let Some(alert_area) = layout.alert_area {
        let alerts = app.active_alerts();
        if !alerts.is_empty() {
            render_alert_banner(f, app, &alerts[0], theme, alert_area);
        }
    }

    // 3. Active synchronization section (if currently running)
    if let Some(sync_area) = layout.sync_area {
        render_active_sync_section(f, app, theme, sync_area);
    }

    // 4. Middle section: History & Live Logs
    if app.is_box_visible(3) && layout.history_area.height >= 4 && layout.history_area.width >= 10 {
        render_history_panel(f, app, theme, layout.history_area, hitboxes);
    }
    if app.is_box_visible(4) && layout.logs_area.height >= 4 && layout.logs_area.width >= 10 {
        render_logs_panel(f, app, theme, layout.logs_area, hitboxes);
    }

    // 5. Bottom section: Recent Files
    if app.is_box_visible(5) && layout.recent_area.height >= 3 && layout.recent_area.width >= 10 {
        render_recent_files_panel(f, app, theme, layout.recent_area, hitboxes);
    }
}

/// Renders an empty dashboard state when all boxes are hidden (btop++ style).
pub fn render_empty_dashboard(
    f: &mut Frame,
    _app: &App,
    theme: &ThemePalette,
    area: Rect,
    hitboxes: &mut Vec<Hitbox>,
) {
    use crate::ui::container::centered_fixed_rect;
    use crate::ui::menu::LOGO_RCLONEDASH;
    use ratatui::text::{Line, Span};
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::widgets::Paragraph;
    use ratatui::layout::Alignment;

    let is_wide = area.width >= 86;
    let logo_w: u16 = 78.min(area.width);
    let logo_h: u16 = if is_wide { 6 } else { 5 };
    let total_h: u16 = logo_h + 11;
    let block_w = logo_w.max(34);

    let box_area = centered_fixed_rect(block_w, total_h.min(area.height), area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(logo_h), // 1. Logo
            Constraint::Length(1),      // 2. Version
            Constraint::Length(1),      // 3. Subtitle "No boxes shown!"
            Constraint::Length(1),      // Space
            Constraint::Length(7),      // 4. Menu options (5 boxes + esc + q)
            Constraint::Min(0),
        ])
        .split(box_area);

    // 1. Logo
    let logo_lines: Vec<Line> = LOGO_RCLONEDASH
        .iter()
        .take(logo_h as usize)
        .map(|(color, line_str)| {
            Line::from(Span::styled(*line_str, Style::default().fg(*color).add_modifier(Modifier::BOLD)))
        })
        .collect();
    f.render_widget(Paragraph::new(logo_lines).alignment(Alignment::Center), chunks[0]);

    // 2. Version
    let ver_line = Line::from(vec![
        Span::styled(format!("v{}", crate::config::APP_VERSION), Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD)),
    ]);
    f.render_widget(Paragraph::new(ver_line).alignment(Alignment::Center), chunks[1]);

    // 3. Subtitle
    let subtitle = Line::from(vec![
        Span::styled("No boxes shown!", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
    ]);
    f.render_widget(Paragraph::new(subtitle).alignment(Alignment::Center), chunks[2]);

    // 4. Options list (btop++ style)
    let options = [
        ("1", "Show DISKS & CLOUD box", HitAction::ToggleBox(1)),
        ("2", "Show METRICS box", HitAction::ToggleBox(2)),
        ("3", "Show HISTORY box", HitAction::ToggleBox(3)),
        ("4", "Show LOGS box", HitAction::ToggleBox(4)),
        ("5", "Show RECENT box", HitAction::ToggleBox(5)),
        ("esc", "Show menu", HitAction::MenuOption(0)),
        ("q", "Quit", HitAction::MenuOption(2)),
    ];

    let max_label_len = options.iter().map(|(_, l, _)| l.chars().count()).max().unwrap_or(0);
    let key_w: usize = 3;
    let total_w = (key_w + 3 + max_label_len) as u16;
    let opt_area = chunks[4];
    let start_x = opt_area.x + opt_area.width.saturating_sub(total_w) / 2;

    let mut opt_lines = Vec::new();

    for (row_idx, (key, label, action)) in options.into_iter().enumerate() {
        let line = Line::from(vec![
            Span::styled(format!("{:>width$}", key, width = key_w), Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
            Span::styled(" | ", Style::default().fg(theme.border)),
            Span::styled(format!("{:<width$}", label, width = max_label_len), Style::default().fg(Color::White)),
        ]);
        opt_lines.push(line);

        let y = opt_area.y + row_idx as u16;
        if y < opt_area.y + opt_area.height {
            hitboxes.push(Hitbox {
                rect: Rect {
                    x: start_x,
                    y,
                    width: total_w,
                    height: 1,
                },
                action,
            });
        }
    }

    f.render_widget(Paragraph::new(opt_lines).alignment(Alignment::Center), opt_area);
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::config::{ContainerLayout, MidPanelOrder};

    #[tokio::test]
    async fn test_compute_dashboard_layout_permutations() {
        let mut app = App::new();
        app.config.show_cadrans = true;
        app.config.show_metrics = true;
        app.config.show_history = true;
        app.config.show_logs = true;
        app.config.show_recent = true;
        app.service_info.state = crate::systemd::ServiceState::Idle;
        app.live.is_syncing = false;

        let total_area = Rect { x: 0, y: 0, width: 120, height: 40 };

        // 1. Default layout: Cadrans -> Mid -> Recent
        app.config.container_layout = ContainerLayout::Default;
        app.config.mid_panel_order = MidPanelOrder::HistoryLogs;
        let layout = compute_dashboard_layout(total_area, &app);
        assert!(layout.cadrans_area.y < layout.history_area.y);
        assert!(layout.history_area.y < layout.recent_area.y);
        assert_eq!(layout.history_area.y, layout.logs_area.y);
        assert!(layout.history_area.x < layout.logs_area.x);

        // 2. RecentFirst layout: Cadrans -> Recent -> Mid
        app.config.container_layout = ContainerLayout::RecentFirst;
        let layout = compute_dashboard_layout(total_area, &app);
        assert!(layout.cadrans_area.y < layout.recent_area.y);
        assert!(layout.recent_area.y < layout.history_area.y);

        // 3. LogsTop layout: Mid -> Cadrans -> Recent
        app.config.container_layout = ContainerLayout::LogsTop;
        let layout = compute_dashboard_layout(total_area, &app);
        assert!(layout.history_area.y < layout.cadrans_area.y);
        assert!(layout.cadrans_area.y < layout.recent_area.y);

        // 4. Inverted layout: Recent -> Mid -> Cadrans
        app.config.container_layout = ContainerLayout::Inverted;
        let layout = compute_dashboard_layout(total_area, &app);
        assert!(layout.recent_area.y < layout.history_area.y);
        assert!(layout.history_area.y < layout.cadrans_area.y);

        // 5. Mid panel swap: LogsHistory -> logs on the left, history on the right
        app.config.mid_panel_order = MidPanelOrder::LogsHistory;
        let layout = compute_dashboard_layout(total_area, &app);
        assert!(layout.logs_area.x < layout.history_area.x);
    }
}
