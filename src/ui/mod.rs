pub mod dashboard;
pub mod files;
pub mod filters;
pub mod footer;
pub mod header;
pub mod history;
pub mod logs;
pub mod menu;
pub mod popups;
pub mod settings;
pub mod sparkline;
pub mod theme;
pub mod keys;

use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Style,
    widgets::Block,
    Frame,
};

use crate::app::App;
use dashboard::render_dashboard;
use footer::render_footer;
use popups::render_popups;

pub fn render(f: &mut Frame, app: &mut App) {
    // Réinitialiser les hitboxes pour cette frame
    let mut hitboxes = std::mem::take(&mut app.hitboxes);
    hitboxes.clear();

    let theme = app.current_theme.palette();

    // Quand le menu, les réglages ou l'aide sont ouverts, tout le dashboard d'arrière-plan devient noir et blanc / monochrome
    let dashboard_theme = if matches!(app.modal, crate::app::Modal::Menu | crate::app::Modal::Settings | crate::app::Modal::Help) {
        theme.to_grayscale()
    } else {
        theme.clone()
    };

    // Fond global sombre style btop++
    f.render_widget(Block::default().style(Style::default().bg(dashboard_theme.bg_main)), f.area());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(14),   // 1. Tableau de bord centralisé Tout-en-Un
            Constraint::Length(1), // 2. Pied de page raccourcis
        ])
        .split(f.area());

    // Calcul dynamique précis des hauteurs de viewport pour navigation et scroll fluide
    let total_h = f.area().height as usize;
    let file_vh = ((total_h * 75) / 100).saturating_sub(5);
    app.file_viewport_height = file_vh.max(3);

    let filter_vh = ((total_h * 72) / 100).saturating_sub(4);
    app.filter_viewport_height = filter_vh.max(3);

    let dash_area = chunks[0];
    let show_alert = !app.active_alerts().is_empty();
    let show_active_sync = app.live.is_syncing || app.live.transfer.pct > 0;

    let mut dash_constraints = Vec::new();
    dash_constraints.push(Constraint::Length(6)); // Stockage & Métriques
    if show_alert {
        dash_constraints.push(Constraint::Length(3));
    }
    if show_active_sync {
        dash_constraints.push(Constraint::Length(7));
    }
    dash_constraints.push(Constraint::Percentage(55)); // Historique + Logs
    dash_constraints.push(Constraint::Min(6)); // Fichiers récents

    let dash_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(dash_constraints)
        .split(dash_area);

    let mid_idx = dash_chunks.len() - 2;
    let bot_idx = dash_chunks.len() - 1;
    let mid_area = dash_chunks[mid_idx];
    let bot_area = dash_chunks[bot_idx];

    let graph_height = if mid_area.height >= 20 { 4 } else if mid_area.height >= 16 { 3 } else { 2 };
    let hist_table_height = (mid_area.height.saturating_sub(2 + graph_height)).saturating_sub(2) as usize;
    app.history_viewport_height = hist_table_height.max(2);
    app.logs_viewport_height = mid_area.height.saturating_sub(2) as usize;

    let recent_table_height = bot_area.height.saturating_sub(3) as usize;
    app.recent_viewport_height = recent_table_height.max(1);

    // 1. Dashboard centralisé (KPIs, Metrics, Historique + Graphe, Logs live, Fichiers récents)
    render_dashboard(f, app, &dashboard_theme, chunks[0], &mut hitboxes);

    // 2. Pied de page
    render_footer(f, app, &dashboard_theme, chunks[1], &mut hitboxes);

    // 4. Modales overlay (Settings btop, Fichiers, Filtres, Confirmations)
    render_popups(f, app, &theme, &mut hitboxes);

    app.hitboxes = hitboxes;
}

/// Scrollbar intégrée dans les conteneurs (style btop++)
/// Dessinée directement dans la colonne droite intérieure (x = area.x + area.width - 2)
/// avec flèche ↑ en haut, flèche ↓ en bas, et curseur plein █ au niveau de la position.
pub fn render_btop_scrollbar(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    total: usize,
    pos: usize,
    visible: usize,
    theme: &crate::ui::theme::ThemePalette,
) {
    if total <= visible || area.height < 4 || area.width < 5 {
        return;
    }
    let scroll_x = area.x + area.width.saturating_sub(2);
    let top_y = area.y + 1;
    let bot_y = area.y + area.height.saturating_sub(2);

    if bot_y <= top_y + 1 {
        return;
    }

    let track_height = (bot_y - top_y - 1) as usize;
    let buf = f.buffer_mut();

    // Flèche haut
    buf.set_string(scroll_x, top_y, "↑", Style::default().fg(theme.text_muted));
    // Flèche bas
    buf.set_string(scroll_x, bot_y, "↓", Style::default().fg(theme.text_muted));

    if track_height == 0 {
        return;
    }

    // Effacer la colonne de piste entre les flèches
    for y in (top_y + 1)..bot_y {
        buf.set_string(scroll_x, y, " ", Style::default());
    }

    // Calcul du curseur
    let thumb_size = ((visible as f64 / total as f64) * track_height as f64).round().max(1.0) as usize;
    let max_scroll = total.saturating_sub(visible);
    let effective_pos = pos.min(max_scroll);
    let thumb_start = if max_scroll > 0 {
        ((effective_pos as f64 / max_scroll as f64) * (track_height.saturating_sub(thumb_size)) as f64).round() as usize
    } else {
        0
    };

    let thumb_style = Style::default().fg(ratatui::style::Color::Rgb(200, 205, 215));
    for i in 0..thumb_size {
        let y = top_y + 1 + (thumb_start + i) as u16;
        if y < bot_y {
            buf.set_string(scroll_x, y, "█", thumb_style);
        }
    }
}
