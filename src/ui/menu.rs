use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::{App, HitAction, Hitbox};
use crate::ui::theme::ThemePalette;

// Logo standard ANSI Shadow pour RCLONEDASH
pub const LOGO_RCLONEDASH: [(Color, &str); 6] = [
    (Color::Rgb(230, 37, 37), "██████╗  ██████╗██╗      ██████╗ ███╗   ██╗███████╗██████╗  █████╗ ███████╗██╗  ██╗"),
    (Color::Rgb(205, 33, 33), "██╔══██╗██╔════╝██║     ██╔═══██╗████╗  ██║██╔════╝██╔══██╗██╔══██╗██╔════╝██║  ██║"),
    (Color::Rgb(179, 29, 29), "██████╔╝██║     ██║     ██║   ██║██╔██╗ ██║█████╗  ██║  ██║███████║███████╗███████║"),
    (Color::Rgb(154, 25, 25), "██╔══██╗██║     ██║     ██║   ██║██║╚██╗██║██╔══╝  ██║  ██║██╔══██║╚════██║██╔══██║"),
    (Color::Rgb(128, 20, 20), "██║  ██║╚██████╗███████╗╚██████╔╝██║ ╚████║███████╗██████╔╝██║  ██║███████║██║  ██║"),
    (Color::Rgb(80, 15, 15),  "╚═╝  ╚═╝ ╚═════╝╚══════╝ ╚═════╝ ╚═╝  ╚═══╝╚══════╝╚═════╝ ╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝"),
];

// Boutons du menu btop++ en état normal (simple trait fin)
const MENU_NORMAL: [[&str; 3]; 3] = [
    [
        "┌─┐┌─┐┌┬┐┬┌─┐┌┐┌┌─┐",
        "│ │├─┘ │ ││ ││││└─┐",
        "└─┘┴   ┴ ┴└─┘┘└┘└─┘",
    ],
    [
        "┬ ┬┌─┐┬  ┌─┐",
        "├─┤├┤ │  ├─┘",
        "┴ ┴└─┘┴─┘┴  ",
    ],
    [
        "┌─┐ ┬ ┬ ┬┌┬┐",
        "│─┼┐│ │ │ │ ",
        "└─┘└└─┘ ┴ ┴ ",
    ],
];

// Boutons du menu btop++ en état sélectionné (double trait 3D)
const MENU_SELECTED: [[&str; 3]; 3] = [
    [
        "╔═╗╔═╗╔╦╗╦╔═╗╔╗╔╔═╗",
        "║ ║╠═╝ ║ ║║ ║║║║╚═╗",
        "╚═╝╩   ╩ ╩╚═╝╝╚╝╚═╝",
    ],
    [
        "╦ ╦╔═╗╦  ╔═╗",
        "╠═╣╠╣ ║  ╠═╝",
        "╩ ╩╚═╝╩═╝╩  ",
    ],
    [
        "╔═╗ ╦ ╦ ╦╔╦╗ ",
        "║═╬╗║ ║ ║ ║  ",
        "╚═╝╚╚═╝ ╩ ╩  ",
    ],
];

const BUTTON_WIDTHS: [u16; 3] = [19, 12, 12];

const COLORS_SELECTED: [Color; 3] = [
    Color::Rgb(230, 37, 37),
    Color::Rgb(179, 29, 29),
    Color::Rgb(128, 20, 20),
];

const COLORS_NORMAL: [Color; 3] = [
    Color::Rgb(204, 204, 204),
    Color::Rgb(170, 170, 170),
    Color::Rgb(128, 128, 128),
];

pub fn render_menu_modal(f: &mut Frame, app: &App, _theme: &ThemePalette, hitboxes: &mut Vec<Hitbox>) {
    let screen = f.area();

    // Largeur dynamique : 85 si grand écran, sinon 66
    let is_wide = screen.width >= 86;
    let logo_height: u16 = if is_wide { 6 } else { 5 };
    let width = if is_wide { 85 } else { 66.min(screen.width) };
    let height = (logo_height + 13).min(screen.height);

    let area = centered_fixed_rect(width, height, screen);

    // Découpage vertical sans conteneur noir : flotte librement au-dessus du dashboard grisé
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(logo_height), // 1. Grand titre RCLONEDASH 3D
            Constraint::Length(1),           // 2. Version
            Constraint::Length(1),           // Espace
            Constraint::Length(3),           // 3. OPTIONS (3 lignes)
            Constraint::Length(1),           // Espace
            Constraint::Length(3),           // 4. HELP (3 lignes)
            Constraint::Length(1),           // Espace
            Constraint::Length(3),           // 5. QUIT (3 lignes)
            Constraint::Min(0),
        ])
        .split(area);

    // 1. Rendu du logo sans fond noir
    render_logo(f, chunks[0]);

    // 2. Version alignée
    let mut ver_spans = vec![
        Span::styled(format!("v{}", crate::config::APP_VERSION), Style::default().fg(Color::Rgb(165, 170, 185)).add_modifier(Modifier::BOLD | Modifier::ITALIC)),
    ];
    if let Some(newer) = &app.available_update {
        ver_spans.push(Span::raw("  "));
        ver_spans.push(Span::styled(
            format!("(🚀 v{} available! Run: rclonedash --update)", newer),
            Style::default().fg(Color::Rgb(250, 200, 50)).add_modifier(Modifier::BOLD),
        ));
    }
    let ver_line = Line::from(ver_spans);
    let ver_p = Paragraph::new(ver_line).alignment(Alignment::Center);
    f.render_widget(ver_p, chunks[1]);

    // 3. Les 3 boutons ASCII — dimensionnés exactement à leur largeur pour supprimer les barres noires
    let items = [
        (0, chunks[3]),
        (1, chunks[5]),
        (2, chunks[7]),
    ];

    for (idx, row_area) in items {
        let is_selected = app.menu_selected_idx == idx;
        let btn_w = BUTTON_WIDTHS[idx];
        let btn_x = row_area.x + (row_area.width.saturating_sub(btn_w)) / 2;
        let btn_area = Rect {
            x: btn_x,
            y: row_area.y,
            width: btn_w,
            height: 3,
        };

        hitboxes.push(Hitbox {
            rect: btn_area,
            action: HitAction::MenuOption(idx),
        });

        render_ascii_button(f, btn_area, idx, is_selected);
    }
}

pub fn render_logo(f: &mut Frame, area: Rect) {
    let mut header_lines = Vec::new();
    for (z, (fg, line_str)) in LOGO_RCLONEDASH.iter().enumerate() {
        let bg_val = (120u32).saturating_sub((z as u32) * 12) as u8;
        let bg_color = Color::Rgb(bg_val, bg_val, bg_val);
        let mut spans = Vec::new();
        for ch in line_str.chars() {
            if ch == '█' {
                spans.push(Span::styled(ch.to_string(), Style::default().fg(*fg).add_modifier(Modifier::BOLD)));
            } else if ch != ' ' {
                spans.push(Span::styled(ch.to_string(), Style::default().fg(bg_color)));
            } else {
                spans.push(Span::raw(" "));
            }
        }
        header_lines.push(Line::from(spans));
    }
    let header_p = Paragraph::new(header_lines).alignment(Alignment::Center);
    f.render_widget(header_p, area);
}

fn render_ascii_button(
    f: &mut Frame,
    area: Rect,
    idx: usize,
    is_selected: bool,
) {
    let lines = if is_selected {
        MENU_SELECTED[idx]
    } else {
        MENU_NORMAL[idx]
    };

    let colors = if is_selected {
        COLORS_SELECTED
    } else {
        COLORS_NORMAL
    };

    let p_lines: Vec<Line> = lines
        .iter()
        .enumerate()
        .map(|(row, line)| {
            let color = colors.get(row).copied().unwrap_or(Color::White);
            Line::from(Span::styled(
                *line,
                Style::default()
                    .fg(color)
                    .add_modifier(if is_selected { Modifier::BOLD } else { Modifier::empty() }),
            ))
        })
        .collect();

    let p = Paragraph::new(p_lines);
    f.render_widget(p, area);
}

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
