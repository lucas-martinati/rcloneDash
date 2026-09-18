use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::{App, HitAction, Hitbox};
use crate::ui::theme::ThemePalette;

const ASCII_BORDER: border::Set = border::Set {
    top_left: "+",
    top_right: "+",
    bottom_left: "+",
    bottom_right: "+",
    vertical_left: "|",
    vertical_right: "|",
    horizontal_top: "-",
    horizontal_bottom: "-",
};

pub fn render_menu_modal(f: &mut Frame, app: &App, theme: &ThemePalette, hitboxes: &mut Vec<Hitbox>) {
    let screen = f.area();
    let is_compact = screen.height < 20 || screen.width < 64;

    let width = if is_compact { 52.min(screen.width) } else { 62.min(screen.width) };
    let height = if is_compact { 12.min(screen.height) } else { 18.min(screen.height) };

    let area = centered_fixed_rect(width, height, screen);
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(ASCII_BORDER)
        .border_style(Style::default().fg(theme.accent))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled("+-- Menu Principal (Esc pour fermer) --+", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if is_compact {
        render_compact_menu(f, inner, app, theme, hitboxes);
    } else {
        render_full_ascii_menu(f, inner, app, theme, hitboxes);
    }
}

fn render_full_ascii_menu(f: &mut Frame, area: Rect, app: &App, theme: &ThemePalette, hitboxes: &mut Vec<Hitbox>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // 1. Bannière pur ASCII
            Constraint::Length(1), // 2. Sous-titre version
            Constraint::Length(1), // Espace
            Constraint::Length(2), // 3. Bouton Aide
            Constraint::Length(2), // 4. Bouton Quitter
            Constraint::Min(1),    // 5. Raccourcis navigation
        ])
        .split(area);

    // 1. Bannière en pur ASCII art (caractères standard uniquement)
    let banner_lines = [
        (r"  ____      _                   ____            _     ", theme.accent),
        (r" |  _ \ ___| | ___  _ __   ___ |  _ \  __ _ ___| |__  ", theme.accent),
        (r" | |_) / __| |/ _ \| '_ \ / _ \| | | |/ _` / __| '_ \ ", theme.purple),
        (r" |  _ < (__| | (_) | | | |  __/| |_| | (_| \__ \ | | |", theme.purple),
        (r" |_| \_\___|_|\___/|_| |_|\___||____/ \__,_|___/_| |_|", theme.blue),
    ];

    let banner_spans: Vec<Line> = banner_lines
        .iter()
        .map(|(text, color)| {
            Line::from(Span::styled(*text, Style::default().fg(*color).add_modifier(Modifier::BOLD)))
        })
        .collect();

    let banner_p = Paragraph::new(banner_spans).alignment(Alignment::Center);
    f.render_widget(banner_p, chunks[0]);

    // 2. Sous-titre
    let sub_line = Line::from(vec![
        Span::styled("- ", Style::default().fg(theme.border)),
        Span::styled("D A S H", Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD)),
        Span::styled(" v0.1.0 ", Style::default().fg(theme.text_muted)),
        Span::styled("-", Style::default().fg(theme.border)),
    ]);
    let sub_p = Paragraph::new(sub_line).alignment(Alignment::Center);
    f.render_widget(sub_p, chunks[1]);

    // 2 boutons uniquement : Aide et Quitter
    let buttons = [
        ("h", "Aide & Raccourcis", 0, theme.green),
        ("q", "Quitter rcloneDash", 1, theme.red),
    ];

    for (key, label, idx, accent_col) in buttons {
        let is_selected = app.menu_selected_idx == idx;
        let btn_area = chunks[3 + idx];

        hitboxes.push(Hitbox {
            rect: btn_area,
            action: HitAction::MenuOption(idx),
        });

        render_menu_button(f, btn_area, key, label, is_selected, accent_col, theme);
    }

    // Bas de page : Navigation
    let nav_line = Line::from(vec![
        Span::styled("[UP/DOWN/Tab] ", Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD)),
        Span::styled("Naviguer  ", Style::default().fg(theme.text_muted)),
        Span::styled("[Enter/Space] ", Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD)),
        Span::styled("Valider  ", Style::default().fg(theme.text_muted)),
        Span::styled("[Esc] ", Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD)),
        Span::styled("Fermer", Style::default().fg(theme.text_muted)),
    ]);
    let nav_p = Paragraph::new(nav_line).alignment(Alignment::Center);
    f.render_widget(nav_p, chunks[5]);
}

fn render_compact_menu(f: &mut Frame, area: Rect, app: &App, theme: &ThemePalette, hitboxes: &mut Vec<Hitbox>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Bannière ASCII simple
            Constraint::Length(1), // Espace
            Constraint::Length(2), // Bouton Aide
            Constraint::Length(2), // Bouton Quitter
            Constraint::Min(1),    // Navigation
        ])
        .split(area);

    let compact_banner = vec![
        Line::from(Span::styled("+-----------------------------+", Style::default().fg(theme.border))),
        Line::from(Span::styled("|      R C L O N E D A S H    |", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled("+-----------------------------+", Style::default().fg(theme.border))),
    ];
    let banner_p = Paragraph::new(compact_banner).alignment(Alignment::Center);
    f.render_widget(banner_p, chunks[0]);

    let buttons = [
        ("h", "Aide & Raccourcis", 0, theme.green),
        ("q", "Quitter rcloneDash", 1, theme.red),
    ];

    for (key, label, idx, accent_col) in buttons {
        let is_selected = app.menu_selected_idx == idx;
        let btn_area = chunks[2 + idx];

        hitboxes.push(Hitbox {
            rect: btn_area,
            action: HitAction::MenuOption(idx),
        });

        render_menu_button(f, btn_area, key, label, is_selected, accent_col, theme);
    }

    let nav_line = Line::from(vec![
        Span::styled("[UP/DOWN] ", Style::default().fg(theme.highlight)),
        Span::styled("Naviguer | ", Style::default().fg(theme.text_muted)),
        Span::styled("[Enter] ", Style::default().fg(theme.highlight)),
        Span::styled("Valider | ", Style::default().fg(theme.text_muted)),
        Span::styled("[Esc] ", Style::default().fg(theme.highlight)),
        Span::styled("Fermer", Style::default().fg(theme.text_muted)),
    ]);
    let nav_p = Paragraph::new(nav_line).alignment(Alignment::Center);
    f.render_widget(nav_p, chunks[4]);
}

fn render_menu_button(
    f: &mut Frame,
    area: Rect,
    key: &str,
    label: &str,
    is_selected: bool,
    accent_col: Color,
    theme: &ThemePalette,
) {
    let is_quit = key == "q";
    let (border_col, bg_col, text_style, key_style) = if is_selected {
        if is_quit {
            (
                theme.red,
                Color::Rgb(70, 20, 25),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            )
        } else {
            (
                theme.highlight,
                theme.border_focus,
                Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD),
                Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD),
            )
        }
    } else {
        (
            theme.border,
            theme.card_bg,
            Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD),
            Style::default().fg(accent_col).add_modifier(Modifier::BOLD),
        )
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(ASCII_BORDER)
        .border_style(Style::default().fg(border_col))
        .style(Style::default().bg(bg_col));

    let content_area = block.inner(area);
    f.render_widget(block, area);

    let prefix = if is_selected { "> " } else { "  " };
    let line = Line::from(vec![
        Span::styled(prefix, Style::default().fg(if is_quit && is_selected { Color::White } else { theme.highlight }).add_modifier(Modifier::BOLD)),
        Span::styled(format!("[{}] ", key), key_style),
        Span::styled(label, text_style),
    ]);

    let p = Paragraph::new(line).alignment(Alignment::Center);
    f.render_widget(p, content_area);
}

fn centered_fixed_rect(width: u16, height: u16, r: Rect) -> Rect {
    let x = r.x + r.width.saturating_sub(width) / 2;
    let y = r.y + r.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width: width.min(r.width),
        height: height.min(r.height),
    }
}
