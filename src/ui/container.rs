use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear},
    Frame,
};

use crate::app::{App, HitAction, Hitbox, Modal};
use crate::ui::theme::ThemePalette;

#[derive(Clone, Copy, Debug)]
pub struct NavArrowsConfig {
    pub label: &'static str,
    pub up_active: bool,
    pub down_active: bool,
}

pub struct ModalContainerConfig<'a> {
    pub title_prefix: &'a str,
    pub title_color: Option<Color>,
    pub title_extra: Option<Vec<Span<'a>>>,
    pub nav_arrows: Option<NavArrowsConfig>,
    pub action_shortcuts: Option<Vec<Vec<Span<'a>>>>,
    pub bottom_shortcuts: Option<Line<'a>>,
    pub counter: Option<(usize, usize)>, // (current 1-based, total)
    pub border_color: Color,
    pub show_close_button: bool,
}

impl<'a> Default for ModalContainerConfig<'a> {
    fn default() -> Self {
        Self {
            title_prefix: "",
            title_color: None,
            title_extra: None,
            nav_arrows: None,
            action_shortcuts: None,
            bottom_shortcuts: None,
            counter: None,
            border_color: Color::White,
            show_close_button: true,
        }
    }
}

/// Helper pour centrer une boîte modale en pourcentages
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Helper pour centrer une boîte modale avec largeur et hauteur fixes
pub fn centered_fixed_rect(width: u16, height: u16, r: Rect) -> Rect {
    let x = r.x + r.width.saturating_sub(width) / 2;
    let y = r.y + r.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width: width.min(r.width),
        height: height.min(r.height),
    }
}

/// Calcule la zone écran totale occupée par une modale active (pour gestion du click-outside et scroll).
pub fn compute_active_modal_area(modal: &Modal, screen: Rect) -> Option<Rect> {
    match modal {
        Modal::None => None,
        Modal::Menu => {
            let is_wide = screen.width >= 86;
            let width = if is_wide { 86.min(screen.width) } else { 72.min(screen.width) };
            let logo_height: u16 = if is_wide { 6 } else { 5 };
            let height = (logo_height + 13).min(screen.height);
            Some(centered_fixed_rect(width, height, screen))
        }
        Modal::Settings => {
            let is_wide = screen.width >= 88;
            let logo_h: u16 = if is_wide { 6 } else { 5 };
            let show_logo = screen.height >= 32;
            let box_w = if screen.width >= 96 {
                88.min(screen.width.saturating_sub(4))
            } else if screen.width >= 86 {
                82.min(screen.width.saturating_sub(2))
            } else {
                76.min(screen.width.saturating_sub(2))
            };
            let box_h = if show_logo {
                23.min(screen.height.saturating_sub(logo_h + 3))
            } else {
                23.min(screen.height.saturating_sub(2))
            };
            let total_h = if show_logo { logo_h + 1 + box_h } else { box_h };
            Some(centered_fixed_rect(box_w, total_h, screen))
        }
        Modal::Files => Some(centered_rect(80, 75, screen)),
        Modal::Filters => Some(centered_rect(82, 74, screen)),
        Modal::HistoryDetails(_) => Some(centered_rect(78, 74, screen)),
        Modal::DryRun => Some(centered_rect(82, 80, screen)),
        Modal::ConfirmSync | Modal::ConfirmDryRun => Some(centered_rect(58, 25, screen)),
        Modal::ConfirmResync => Some(centered_rect(60, 30, screen)),
        Modal::ConfirmCancel | Modal::ConfirmDelete(_) => Some(centered_rect(55, 25, screen)),
        Modal::Help => {
            let is_wide = screen.width >= 86;
            let logo_h: u16 = if is_wide { 6 } else { 5 };
            let box_w = if is_wide { 86.min(screen.width) } else { 78.min(screen.width) };
            let box_h = 24.min(screen.height.saturating_sub(logo_h + 3));
            let total_h = logo_h + 1 + box_h;
            Some(centered_fixed_rect(box_w, total_h, screen))
        }
    }
}

/// Helper pour générer un titre incrusté dans une bordure avec les bons caractères de jonction
pub fn format_border_title<'a>(
    glyphs: crate::config::BorderGlyphs,
    border_color: Color,
    title: &'a str,
    title_color: Option<Color>,
    extra: Option<Vec<Span<'a>>>,
) -> Line<'a> {
    let t_col = title_color.unwrap_or(border_color);
    let mut spans = vec![
        Span::styled(glyphs.top_left, Style::default().fg(border_color)),
        Span::styled(title, Style::default().fg(t_col).add_modifier(Modifier::BOLD)),
    ];
    if let Some(extra_spans) = extra {
        spans.extend(extra_spans);
    }
    spans.push(Span::styled(glyphs.top_right, Style::default().fg(border_color)));
    Line::from(spans)
}

/// Rendu du conteneur/wrapper de base standardisé pour tous les panels et modales
pub fn render_modal_container<'a>(
    f: &mut Frame,
    app: &App,
    theme: &ThemePalette,
    area: Rect,
    cfg: ModalContainerConfig<'a>,
    hitboxes: &mut Vec<Hitbox>,
) -> Rect {
    // 1. Isoler le fond
    f.render_widget(Clear, area);

    let bg = app.border_glyphs();
    let border_col = cfg.border_color;

    // 2. Titre gauche standardisé
    let title_line = format_border_title(bg, border_col, cfg.title_prefix, cfg.title_color, cfg.title_extra);

    let mut outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(app.border_type())
        .border_style(Style::default().fg(border_col))
        .style(Style::default().bg(theme.card_bg))
        .title(title_line);

    // 3. Bouton de fermeture uniforme en haut à droite
    if cfg.show_close_button {
        let mut close_spans = vec![Span::styled(bg.top_left, Style::default().fg(border_col))];
        close_spans.extend(crate::ui::keys::KeybindingRegistry::format_shortcut_label("Esc, q", "close", theme.red, theme.text_bright));
        close_spans.push(Span::styled(bg.top_right, Style::default().fg(border_col)));
        let close_title = Line::from(close_spans).alignment(Alignment::Right);
        outer_block = outer_block.title(close_title);

        // Enregistrer la hitbox de fermeture standardisée
        hitboxes.push(Hitbox {
            rect: Rect {
                x: area.x + area.width.saturating_sub(14),
                y: area.y,
                width: 12,
                height: 1,
            },
            action: HitAction::CloseModal,
        });
    }

    // 4. Titre bas : raccourcis à gauche
    let bottom_shortcuts = if let Some(bottom_shortcuts) = cfg.bottom_shortcuts {
        Some(bottom_shortcuts)
    } else if cfg.nav_arrows.is_some() || cfg.action_shortcuts.is_some() {
        let sep = format!("{}{}", bg.bot_right, bg.bot_left);
        let mut spans: Vec<Span<'a>> = Vec::new();

        if let Some(nav) = cfg.nav_arrows {
            spans.push(Span::styled(bg.bot_left, Style::default().fg(border_col)));
            let up_col = if nav.up_active { theme.red } else { theme.text_muted };
            let down_col = if nav.down_active { theme.red } else { theme.text_muted };
            spans.push(Span::styled("↑", Style::default().fg(up_col).add_modifier(Modifier::BOLD)));
            spans.push(Span::styled(format!(" {} ", nav.label), Style::default().fg(Color::White)));
            spans.push(Span::styled("↓", Style::default().fg(down_col).add_modifier(Modifier::BOLD)));
        }

        if let Some(actions) = cfg.action_shortcuts {
            for action_spans in actions {
                if spans.is_empty() {
                    spans.push(Span::styled(bg.bot_left, Style::default().fg(border_col)));
                } else {
                    spans.push(Span::styled(sep.clone(), Style::default().fg(border_col)));
                }
                spans.extend(action_spans);
            }
        }

        if !spans.is_empty() {
            spans.push(Span::styled(bg.bot_right, Style::default().fg(border_col)));
            Some(Line::from(spans))
        } else {
            None
        }
    } else {
        None
    };

    if let Some(line) = bottom_shortcuts {
        outer_block = outer_block.title_bottom(line.alignment(Alignment::Left));
    }

    // 5. Titre bas : compteur pagination à droite
    if let Some((cur, total)) = cfg.counter {
        outer_block = outer_block.title_bottom(
            Line::from(vec![
                Span::styled(
                    format!("{} {}/{} {}", bg.horizontal, cur, total, bg.horizontal),
                    Style::default().fg(border_col).add_modifier(Modifier::BOLD),
                ),
            ])
            .alignment(Alignment::Right),
        );
    }

    f.render_widget(outer_block, area);

    // Retourne la zone intérieure utilisable
    Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    }
}
