use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use crate::app::{App, HitAction, Hitbox};
use crate::config;
use crate::ui::theme::ThemePalette;

pub fn render_filters_modal(f: &mut Frame, app: &App, theme: &ThemePalette, hitboxes: &mut Vec<Hitbox>) {
    let area = centered_rect(75, 70, f.area());
    f.render_widget(Clear, area);

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60),
            Constraint::Percentage(40),
        ])
        .split(Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        });

    let filepath = config::filters_file().display().to_string();
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.purple))
        .style(Style::default().bg(theme.card_bg))
        .title(Span::styled(format!(" ⊘ RÈGLES D'EXCLUSION : {} ", filepath), Style::default().fg(theme.purple).add_modifier(Modifier::BOLD)));
    f.render_widget(outer_block, area);

    render_rules_list(f, app, theme, main_chunks[0]);
    render_filters_help(f, app, theme, main_chunks[1], hitboxes);
}

fn render_rules_list(f: &mut Frame, app: &App, theme: &ThemePalette, area: Rect) {
    let visible_height = area.height as usize;
    if visible_height == 0 {
        return;
    }

    let offset = if app.selected_filter_idx < app.filter_scroll_offset {
        app.selected_filter_idx
    } else if app.selected_filter_idx >= app.filter_scroll_offset + visible_height {
        app.selected_filter_idx.saturating_sub(visible_height) + 1
    } else {
        app.filter_scroll_offset
    };

    let items: Vec<ListItem> = if app.filters.is_empty() {
        vec![ListItem::new(Line::from(vec![
            Span::styled(" Aucun fichier gdrive-filters.txt trouvé ou fichier vide.", Style::default().fg(theme.text_muted)),
        ]))]
    } else {
        app.filters
            .iter()
            .enumerate()
            .skip(offset)
            .take(visible_height)
            .map(|(i, rule)| {
                let trimmed = rule.trim();
                let is_selected = i == app.selected_filter_idx;

                let (color, prefix) = if trimmed.starts_with('+') {
                    (theme.green, "[INCLUDE]")
                } else if trimmed.starts_with('-') {
                    (theme.red, "[EXCLUDE]")
                } else if trimmed.starts_with('#') {
                    (theme.text_muted, "[COMMENT]")
                } else {
                    (theme.text_bright, "[RULE]   ")
                };

                let cursor = if is_selected {
                    Span::styled("▶ ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))
                } else {
                    Span::styled("  ", Style::default())
                };

                let line = Line::from(vec![
                    cursor,
                    Span::styled(format!("{:2} │ ", i + 1), Style::default().fg(theme.text_muted)),
                    Span::styled(format!("{} ", prefix), Style::default().fg(color).add_modifier(Modifier::BOLD)),
                    Span::styled(rule, Style::default().fg(if is_selected { theme.text_bright } else { theme.text_muted })),
                ]);

                ListItem::new(line)
            })
            .collect()
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::NONE),
    );

    f.render_widget(list, area);

    if app.filters.len() > visible_height {
        let mut scrollbar_state = ScrollbarState::new(app.filters.len())
            .position(app.selected_filter_idx);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_style(Style::default().fg(theme.border))
            .thumb_style(Style::default().fg(theme.accent));
        f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}

fn render_filters_help(f: &mut Frame, _app: &App, theme: &ThemePalette, area: Rect, hitboxes: &mut Vec<Hitbox>) {
    let text = vec![
        Line::from(Span::styled("Syntaxe des règles :", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  - /mon_dossier/**  ", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
            Span::styled("Exclut tout", Style::default().fg(theme.text_muted)),
        ]),
        Line::from(vec![
            Span::styled("  - *.tmp            ", Style::default().fg(theme.red).add_modifier(Modifier::BOLD)),
            Span::styled("Exclut les .tmp", Style::default().fg(theme.text_muted)),
        ]),
        Line::from(vec![
            Span::styled("  + *.pdf            ", Style::default().fg(theme.green).add_modifier(Modifier::BOLD)),
            Span::styled("Force inclusion", Style::default().fg(theme.text_muted)),
        ]),
        Line::from(""),
        Line::from(Span::styled("─".repeat(25), Style::default().fg(theme.border))),
        Line::from(""),
        Line::from(Span::styled("Appuyez sur 'e' pour éditer", Style::default().fg(theme.yellow).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled("dans votre $EDITOR (nano/nvim/vim).", Style::default().fg(theme.text_bright))),
        Line::from(""),
        Line::from(Span::styled("Les règles s'appliquent dès la", Style::default().fg(theme.text_muted))),
        Line::from(Span::styled("prochaine synchronisation bisync.", Style::default().fg(theme.text_muted))),
    ];

    let close_btn_area = Rect {
        x: area.x + 2,
        y: area.y + area.height.saturating_sub(3),
        width: area.width.saturating_sub(4),
        height: 2,
    };
    hitboxes.push(Hitbox {
        rect: close_btn_area,
        action: HitAction::CloseModal,
    });

    let p = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(theme.border)),
    );
    f.render_widget(p, area);

    let close_p = Paragraph::new(Line::from(vec![
        Span::styled(" [ Fermer (Échap) ] ", Style::default().fg(theme.text_bright).bg(theme.border)),
    ])).alignment(Alignment::Center);
    f.render_widget(close_p, close_btn_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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
