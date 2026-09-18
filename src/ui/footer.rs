use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

use crate::app::{App, HitAction, Hitbox};
use crate::ui::theme::ThemePalette;

pub fn render_footer(
    f: &mut Frame,
    app: &App,
    theme: &ThemePalette,
    area: Rect,
    hitboxes: &mut Vec<Hitbox>,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(65),    // Raccourcis unifiés
            Constraint::Length(18), // Focus actif uniquement
        ])
        .split(area);

    // Commandes unifiées avec règle de formatage :
    // - Si la 1ère lettre est la touche : première lettre en rouge, reste en clair.
    // - Sinon : touche en rouge suivie d'un espace puis du nom de l'action.
    enum CmdFmt {
        FirstLetterRed(&'static str, &'static str, HitAction), // ("m", "enu", action)
        PrefixedKey(&'static str, &'static str, HitAction),    // ("e", "filtres", action)
    }

    let items = [
        CmdFmt::FirstLetterRed("m", "enu", HitAction::ButtonMenu),
        CmdFmt::FirstLetterRed("q", "uit", HitAction::ButtonQuit),
        CmdFmt::FirstLetterRed("s", "ync", HitAction::ButtonSync),
        CmdFmt::FirstLetterRed("d", "ry-run", HitAction::ButtonDryRun),
        CmdFmt::FirstLetterRed("f", "iles", HitAction::ButtonFiles),
        CmdFmt::PrefixedKey("e", "filtres", HitAction::ButtonFilters),
        CmdFmt::FirstLetterRed("o", "ptions", HitAction::ButtonSettings),
        CmdFmt::FirstLetterRed("t", "heme", HitAction::ButtonTheme),
        CmdFmt::PrefixedKey("Tab", "panel", HitAction::ButtonPanel),
        CmdFmt::PrefixedKey("?", "aide", HitAction::ButtonHelp),
    ];

    let mut spans: Vec<Span> = Vec::new();
    let mut cur_x = chunks[0].x;

    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", Style::default()));
            cur_x += 2;
        }

        match item {
            CmdFmt::FirstLetterRed(first, rest, action) => {
                let len = (first.chars().count() + rest.chars().count()) as u16;
                hitboxes.push(Hitbox {
                    rect: Rect {
                        x: cur_x,
                        y: area.y,
                        width: len,
                        height: 1,
                    },
                    action: *action,
                });
                spans.push(Span::styled(
                    *first,
                    Style::default().fg(theme.red).add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::styled(
                    *rest,
                    Style::default().fg(theme.text_bright),
                ));
                cur_x += len;
            }
            CmdFmt::PrefixedKey(key, action_name, action) => {
                let len = (key.chars().count() + 1 + action_name.chars().count()) as u16;
                hitboxes.push(Hitbox {
                    rect: Rect {
                        x: cur_x,
                        y: area.y,
                        width: len,
                        height: 1,
                    },
                    action: *action,
                });
                spans.push(Span::styled(
                    *key,
                    Style::default().fg(theme.red).add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::styled(
                    format!(" {}", action_name),
                    Style::default().fg(theme.text_bright),
                ));
                cur_x += len;
            }
        }
    }

    let p_left = Paragraph::new(Line::from(spans))
        .alignment(Alignment::Left)
        .block(Block::default().style(Style::default().bg(theme.footer_bg)));
    f.render_widget(p_left, chunks[0]);

    // Panneau actuellement actif
    let right_spans = vec![
        Span::styled("Focus: ", Style::default().fg(theme.text_muted)),
        Span::styled(
            format!("{} ", app.focused_panel.label()),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
    ];

    let p_right = Paragraph::new(Line::from(right_spans))
        .alignment(Alignment::Right)
        .block(Block::default().style(Style::default().bg(theme.footer_bg)));
    f.render_widget(p_right, chunks[1]);
}
