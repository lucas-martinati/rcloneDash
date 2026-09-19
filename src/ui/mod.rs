pub mod container;
pub mod dashboard;
pub mod files;
pub mod filters;
pub mod footer;
pub mod history;
pub mod logs;
pub mod menu;
pub mod popups;
pub mod settings;
pub mod sparkline;
pub mod theme;
pub mod keys;
pub mod scrollbar;

pub use container::compute_active_modal_area;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
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

    let filter_vh = ((total_h * 74) / 100).saturating_sub(2);
    app.filter_viewport_height = filter_vh.max(3);

    let dash_area = chunks[0];
    let dash_layout = dashboard::compute_dashboard_layout(dash_area, app);

    let graph_height = if dash_layout.history_area.height >= 20 { 4 } else if dash_layout.history_area.height >= 16 { 3 } else { 2 };
    let hist_table_height = (dash_layout.history_area.height.saturating_sub(2 + graph_height)).saturating_sub(2) as usize;
    app.history_viewport_height = hist_table_height.max(2);
    app.logs_viewport_height = dash_layout.logs_area.height.saturating_sub(2) as usize;
    let logs_width = (dash_layout.logs_area.width.saturating_sub(3) as usize).max(20);
    app.logs_total_wrapped = dashboard::count_wrapped_log_lines(&app.live.log_lines, app.log_filter, logs_width);

    let recent_table_height = dash_layout.recent_area.height.saturating_sub(3) as usize;
    app.recent_viewport_height = recent_table_height.max(1);

    // 1. Dashboard centralisé (KPIs, Metrics, Historique + Graphe, Logs live, Fichiers récents)
    render_dashboard(f, app, &dashboard_theme, chunks[0], &mut hitboxes);

    // 2. Pied de page
    render_footer(f, app, &dashboard_theme, chunks[1], &mut hitboxes);

    // 4. Modales overlay (Settings btop, Fichiers, Filtres, Confirmations)
    let mut modal_hitboxes = Vec::with_capacity(32);
    render_popups(f, app, &theme, &mut modal_hitboxes);

    app.active_modal_area = compute_active_modal_area(&app.modal, f.area());
    app.modal_hitboxes = modal_hitboxes.clone();
    hitboxes.extend(modal_hitboxes);
    app.hitboxes = hitboxes;
}

/// Rendu d'une scrollbar btop++ personnalisée avec coordonnées explicites de colonne et de bornes verticales
pub fn render_scrollbar_custom(
    f: &mut Frame,
    scroll_x: u16,
    top_y: u16,
    bot_y: u16,
    total: usize,
    pos: usize,
    visible: usize,
    theme: &crate::ui::theme::ThemePalette,
    hitboxes: &mut Vec<crate::app::Hitbox>,
    target: crate::app::ScrollbarTarget,
) {
    if total <= visible || bot_y <= top_y + 1 {
        return;
    }
    let track_height = (bot_y - top_y - 1) as usize;
    let buf = f.buffer_mut();

    // Flèche haut
    buf.set_string(scroll_x, top_y, "↑", Style::default().fg(theme.text_muted));
    hitboxes.push(crate::app::Hitbox {
        rect: Rect { x: scroll_x, y: top_y, width: 1, height: 1 },
        action: crate::app::HitAction::ScrollbarArrowUp(target),
    });

    // Flèche bas
    buf.set_string(scroll_x, bot_y, "↓", Style::default().fg(theme.text_muted));
    hitboxes.push(crate::app::Hitbox {
        rect: Rect { x: scroll_x, y: bot_y, width: 1, height: 1 },
        action: crate::app::HitAction::ScrollbarArrowDown(target),
    });

    if track_height == 0 {
        return;
    }

    // Effacer la colonne de piste entre les flèches
    for y in (top_y + 1)..bot_y {
        buf.set_string(scroll_x, y, " ", Style::default());
    }

    // Hitbox pour toute la piste (clic ou drag)
    hitboxes.push(crate::app::Hitbox {
        rect: Rect {
            x: scroll_x,
            y: top_y + 1,
            width: 1,
            height: track_height as u16,
        },
        action: crate::app::HitAction::ScrollbarTrack {
            target,
            top_y: top_y + 1,
            track_height: track_height as u16,
            total,
            visible,
        },
    });

    // Calcul unifié de la géométrie du curseur
    let geom = scrollbar::ScrollbarGeometry::compute(total, visible, track_height, pos);

    let thumb_style = Style::default().fg(ratatui::style::Color::Rgb(200, 205, 215));
    for i in 0..geom.thumb_size {
        let y = top_y + 1 + (geom.thumb_start + i) as u16;
        if y < bot_y {
            buf.set_string(scroll_x, y, "█", thumb_style);
        }
    }
}

/// Scrollbar intégrée dans les conteneurs avec bordure standard (style btop++)
/// Dessinée directement dans la colonne droite intérieure (x = area.x + area.width - 2)
pub fn render_scrollbar(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    total: usize,
    pos: usize,
    visible: usize,
    theme: &crate::ui::theme::ThemePalette,
    hitboxes: &mut Vec<crate::app::Hitbox>,
    target: crate::app::ScrollbarTarget,
) {
    if total <= visible || area.height < 4 || area.width < 5 {
        return;
    }
    let scroll_x = area.x + area.width.saturating_sub(2);
    let top_y = area.y + 1;
    let bot_y = area.y + area.height.saturating_sub(2);
    render_scrollbar_custom(f, scroll_x, top_y, bot_y, total, pos, visible, theme, hitboxes, target);
}

/// Scrollbar intégrée dans un sous-panneau intérieur (sans bordure propre, e.g. volet gauche de filtres ou explorer)
/// Dessinée à l'extrême droite du panneau (x = pane.x + pane.width - 1) sur toute sa hauteur (top_y = pane.y, bot_y = pane.y + pane.height - 1)
pub fn render_scrollbar_pane(
    f: &mut Frame,
    pane: ratatui::layout::Rect,
    total: usize,
    pos: usize,
    visible: usize,
    theme: &crate::ui::theme::ThemePalette,
    hitboxes: &mut Vec<crate::app::Hitbox>,
    target: crate::app::ScrollbarTarget,
) {
    if total <= visible || pane.height < 4 || pane.width < 2 {
        return;
    }
    let scroll_x = pane.x + pane.width.saturating_sub(1);
    let top_y = pane.y;
    let bot_y = pane.y + pane.height.saturating_sub(1);
    render_scrollbar_custom(f, scroll_x, top_y, bot_y, total, pos, visible, theme, hitboxes, target);
}
