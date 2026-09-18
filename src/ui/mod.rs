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

    // Calcul dynamique des hauteurs de viewport pour navigation et scroll fluide
    let total_h = f.area().height as usize;
    let file_vh = ((total_h * 75) / 100).saturating_sub(5);
    app.file_viewport_height = file_vh.max(3);

    let filter_vh = ((total_h * 72) / 100).saturating_sub(4);
    app.filter_viewport_height = filter_vh.max(3);

    let mid_h = total_h.saturating_sub(10) * 45 / 100;
    app.history_viewport_height = mid_h.saturating_sub(4).max(3);
    app.logs_viewport_height = mid_h.saturating_sub(2).max(3);

    let bot_h = total_h.saturating_sub(10) * 35 / 100;
    app.recent_viewport_height = bot_h.saturating_sub(3).max(3);

    // Fond global sombre style btop++
    f.render_widget(Block::default().style(Style::default().bg(dashboard_theme.bg_main)), f.area());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(14),   // 1. Tableau de bord centralisé Tout-en-Un
            Constraint::Length(1), // 2. Pied de page raccourcis
        ])
        .split(f.area());

    // 1. Dashboard centralisé (KPIs, Metrics, Historique + Graphe, Logs live, Fichiers récents)
    render_dashboard(f, app, &dashboard_theme, chunks[0], &mut hitboxes);

    // 2. Pied de page
    render_footer(f, app, &dashboard_theme, chunks[1], &mut hitboxes);

    // 4. Modales overlay (Settings btop, Fichiers, Filtres, Confirmations)
    render_popups(f, app, &theme, &mut hitboxes);

    app.hitboxes = hitboxes;
}
