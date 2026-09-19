use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

use crate::app::{App, Hitbox};
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

    let sys_h: u16 = 6 + if show_alert { 3 } else { 0 } + if show_active_sync { 7 } else { 0 };
    let rem_h = area.height.saturating_sub(sys_h);
    let mid_h = (rem_h * 62 / 100).max(8);
    let recent_h = rem_h.saturating_sub(mid_h).max(4);

    let (sys_rect, mid_rect, recent_rect) = match app.config.container_layout {
        crate::config::ContainerLayout::Default => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(sys_h),
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
                    Constraint::Length(sys_h),
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
                    Constraint::Length(sys_h),
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
                    Constraint::Min(sys_h),
                ])
                .split(area);
            (chunks[2], chunks[1], chunks[0])
        }
    };

    // Subdivide sys_rect into cadrans, alert, sync
    let mut sys_constraints = vec![Constraint::Length(6)];
    if show_alert {
        sys_constraints.push(Constraint::Length(3));
    }
    if show_active_sync {
        sys_constraints.push(Constraint::Length(7));
    }
    let sys_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(sys_constraints)
        .split(sys_rect);

    let cadrans_area = sys_chunks[0];
    let mut idx = 1;
    let alert_area = if show_alert {
        let a = sys_chunks[idx];
        idx += 1;
        Some(a)
    } else {
        None
    };
    let sync_area = if show_active_sync {
        Some(sys_chunks[idx])
    } else {
        None
    };

    // Subdivide mid_rect horizontally according to mid_panel_order
    let mid_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(mid_rect);

    let (history_area, logs_area) = match app.config.mid_panel_order {
        crate::config::MidPanelOrder::HistoryLogs => (mid_chunks[0], mid_chunks[1]),
        crate::config::MidPanelOrder::LogsHistory => (mid_chunks[1], mid_chunks[0]),
    };

    DashboardLayout {
        cadrans_area,
        alert_area,
        sync_area,
        history_area,
        logs_area,
        recent_area: recent_rect,
    }
}

/// Main dashboard rendering entry point.
pub fn render_dashboard(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect, hitboxes: &mut Vec<Hitbox>) {
    let layout = compute_dashboard_layout(area, app);

    // 1. Storage & Metrics dials
    render_system_cadrans(f, app, theme, layout.cadrans_area, hitboxes);

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
    render_history_panel(f, app, theme, layout.history_area, hitboxes);
    render_logs_panel(f, app, theme, layout.logs_area, hitboxes);

    // 5. Bottom section: Recent Files
    render_recent_files_panel(f, app, theme, layout.recent_area, hitboxes);
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::config::{ContainerLayout, MidPanelOrder};

    #[tokio::test]
    async fn test_compute_dashboard_layout_permutations() {
        let mut app = App::new();
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
