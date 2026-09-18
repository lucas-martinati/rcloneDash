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
    top_left: "┌",
    top_right: "┐",
    bottom_left: "└",
    bottom_right: "┘",
    vertical_left: "│",
    vertical_right: "│",
    horizontal_top: "─",
    horizontal_bottom: "─",
};

const ASCII_OPTIONS_5: [&str; 5] = [
    r"  ___  ____ _____ ___ ___  _   _ ____  ",
    r" / _ \|  _ \_   _|_ _/ _ \| \ | / ___| ",
    r"| | | | |_) || |  | | | | |  \| \___ \ ",
    r"| |_| |  __/ | |  | | |_| | |\  |___) |",
    r" \___/|_|    |_| |___|\___/|_| \_|____/ ",
];

const ASCII_HELP_5: [&str; 5] = [
    r" _   _  _____  _     ____  ",
    r"| | | || ____|| |   |  _ \ ",
    r"| |_| ||  _|  | |   | |_) |",
    r"|  _  || |___ | |___|  __/ ",
    r"|_| |_||_____||_____|_|    ",
];

const ASCII_QUIT_5: [&str; 5] = [
    r"  ___   _   _  ___  _____ ",
    r" / _ \ | | | ||_ _||_   _|",
    r"| | | || | | | | |   | |  ",
    r"| |_| || |_| | | |   | |  ",
    r" \__\_| \___/ |___|  |_|  ",
];

const ASCII_OPTIONS_3: [&str; 3] = [
    r"  ___  ___ _____ ___ ___  _  _ ___ ",
    r" / _ \| _ \_   _|_ _/ _ \| \| / __|",
    r" \___/|  _/ | |  | | \___/|_|\_|___/",
];

const ASCII_HELP_3: [&str; 3] = [
    r"  _  _ ___ _    ___ ",
    r" | || | __| |  | _ \",
    r" |_||_|___|____|  _/",
];

const ASCII_QUIT_3: [&str; 3] = [
    r"  ___  _   _ ___ _____ ",
    r" / _ \| | | |_ _|_   _|",
    r" \__\_\\___/|___| |_|  ",
];

pub fn render_menu_modal(f: &mut Frame, app: &App, theme: &ThemePalette, hitboxes: &mut Vec<Hitbox>) {
    let screen = f.area();
    let is_compact = screen.height < 24 || screen.width < 58;

    let width = if is_compact { 48.min(screen.width) } else { 54.min(screen.width) };
    let height = if is_compact { 15.min(screen.height) } else { 22.min(screen.height) };

    let area = centered_fixed_rect(width, height, screen);
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(ASCII_BORDER)
        .border_style(Style::default().fg(theme.accent))
        .style(Style::default().bg(theme.card_bg))
        .title(Line::from(vec![
            Span::styled("┌", Style::default().fg(theme.border)),
            Span::styled("menu", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("┐", Style::default().fg(theme.border)),
        ]))
        .title_bottom(
            Line::from(vec![
                Span::styled("Esc", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
                Span::styled(" fermer ─ ", Style::default().fg(theme.border)),
            ])
            .alignment(Alignment::Right),
        );

    let inner = block.inner(area);
    f.render_widget(block, area);

    if is_compact {
        render_compact_ascii_menu(f, inner, app, theme, hitboxes);
    } else {
        render_standard_ascii_menu(f, inner, app, theme, hitboxes);
    }
}

fn render_standard_ascii_menu(f: &mut Frame, area: Rect, app: &App, theme: &ThemePalette, hitboxes: &mut Vec<Hitbox>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // 1. OPTIONS
            Constraint::Length(1), // Espace
            Constraint::Length(5), // 2. HELP
            Constraint::Length(1), // Espace
            Constraint::Length(5), // 3. QUIT
            Constraint::Min(1),    // Navigation
        ])
        .split(area);

    let items = [
        (0, &ASCII_OPTIONS_5[..], theme.highlight),
        (1, &ASCII_HELP_5[..], theme.green),
        (2, &ASCII_QUIT_5[..], theme.red),
    ];

    for (idx, ascii_art, accent_color) in items {
        let chunk_idx = idx * 2;
        let item_area = chunks[chunk_idx];
        let is_selected = app.menu_selected_idx == idx;

        hitboxes.push(Hitbox {
            rect: item_area,
            action: HitAction::MenuOption(idx),
        });

        render_ascii_item(f, item_area, ascii_art, is_selected, accent_color, theme);
    }

    let nav_line = Line::from(vec![
        Span::styled("↑/↓ ", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
        Span::styled("naviguer  │  ", Style::default().fg(theme.text_muted)),
        Span::styled("↵ ", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
        Span::styled("valider  │  ", Style::default().fg(theme.text_muted)),
        Span::styled("Esc ", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
        Span::styled("fermer", Style::default().fg(theme.text_muted)),
    ]);
    let nav_p = Paragraph::new(nav_line).alignment(Alignment::Center);
    f.render_widget(nav_p, chunks[5]);
}

fn render_compact_ascii_menu(f: &mut Frame, area: Rect, app: &App, theme: &ThemePalette, hitboxes: &mut Vec<Hitbox>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // 1. OPTIONS
            Constraint::Length(1), // Espace
            Constraint::Length(3), // 2. HELP
            Constraint::Length(1), // Espace
            Constraint::Length(3), // 3. QUIT
            Constraint::Min(1),    // Navigation
        ])
        .split(area);

    let items = [
        (0, &ASCII_OPTIONS_3[..], theme.highlight),
        (1, &ASCII_HELP_3[..], theme.green),
        (2, &ASCII_QUIT_3[..], theme.red),
    ];

    for (idx, ascii_art, accent_color) in items {
        let chunk_idx = idx * 2;
        let item_area = chunks[chunk_idx];
        let is_selected = app.menu_selected_idx == idx;

        hitboxes.push(Hitbox {
            rect: item_area,
            action: HitAction::MenuOption(idx),
        });

        render_ascii_item(f, item_area, ascii_art, is_selected, accent_color, theme);
    }

    let nav_line = Line::from(vec![
        Span::styled("↑/↓ ", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
        Span::styled("naviguer  ", Style::default().fg(theme.text_muted)),
        Span::styled("↵ ", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
        Span::styled("valider", Style::default().fg(theme.text_muted)),
    ]);
    let nav_p = Paragraph::new(nav_line).alignment(Alignment::Center);
    f.render_widget(nav_p, chunks[5]);
}

fn render_ascii_item(
    f: &mut Frame,
    area: Rect,
    lines: &[&str],
    is_selected: bool,
    active_color: Color,
    theme: &ThemePalette,
) {
    let style = if is_selected {
        Style::default().fg(active_color).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.separator)
    };

    let paragraph_lines: Vec<Line> = lines
        .iter()
        .map(|line| Line::from(Span::styled(*line, style)))
        .collect();

    let p = Paragraph::new(paragraph_lines).alignment(Alignment::Center);
    f.render_widget(p, area);
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
