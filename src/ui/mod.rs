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
use header::render_header;
use popups::render_popups;

pub fn render(f: &mut Frame, app: &mut App) {
    // Réinitialiser les hitboxes pour cette frame
    let mut hitboxes = std::mem::take(&mut app.hitboxes);
    hitboxes.clear();

    let theme = app.current_theme.palette();

    // Calcul dynamique de la hauteur du viewport de l'explorateur et des filtres
    let file_vh = ((f.area().height as usize * 75) / 100).saturating_sub(5);
    app.file_viewport_height = file_vh.max(3);

    let filter_vh = ((f.area().height as usize * 72) / 100).saturating_sub(4);
    app.filter_viewport_height = filter_vh.max(3);

    // Fond global sombre style btop++
    f.render_widget(Block::default().style(Style::default().bg(theme.bg_main)), f.area());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // 1. En-tête / Titre + Statuts + Boutons
            Constraint::Min(14),   // 2. Tableau de bord centralisé Tout-en-Un
            Constraint::Length(1), // 3. Pied de page raccourcis
        ])
        .split(f.area());

    // 1. En-tête avec boutons interactifs
    render_header(f, app, &theme, chunks[0], &mut hitboxes);

    // 2. Dashboard centralisé (KPIs, Historique + Graphe, Logs live, Fichiers récents)
    render_dashboard(f, app, &theme, chunks[1], &mut hitboxes);

    // 3. Pied de page
    render_footer(f, app, &theme, chunks[2]);

    // 4. Modales overlay (Settings btop, Fichiers, Filtres, Confirmations)
    render_popups(f, app, &theme, &mut hitboxes);

    app.hitboxes = hitboxes;
}
