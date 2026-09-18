use ratatui::{
    layout::{Alignment, Rect},
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
    // Commandes unifiées : chaque raccourci a sa touche d'activation en rouge
    struct CmdItem {
        prefix: &'static str,
        key: &'static str,
        suffix: &'static str,
        action: HitAction,
    }

    let sync_or_cancel_item = if app.live.is_syncing {
        CmdItem { prefix: "", key: "c", suffix: "ancel", action: HitAction::ButtonCancel }
    } else {
        CmdItem { prefix: "", key: "s", suffix: "ync", action: HitAction::ButtonSync }
    };

    let items = [
        sync_or_cancel_item,
        CmdItem { prefix: "", key: "d", suffix: "ry-run", action: HitAction::ButtonDryRun },
        CmdItem { prefix: "", key: "b", suffix: "rowse", action: HitAction::ButtonFiles },
        CmdItem { prefix: "filtr", key: "e", suffix: "s", action: HitAction::ButtonFilters },
        CmdItem { prefix: "cop", key: "y", suffix: "", action: HitAction::ButtonCopy },
        CmdItem { prefix: "", key: "Tab", suffix: " panel", action: HitAction::ButtonPanel },
    ];

    let mut spans: Vec<Span> = Vec::new();
    let mut cur_x = area.x;

    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", Style::default()));
            cur_x += 2;
        }

        let len = (item.prefix.chars().count() + item.key.chars().count() + item.suffix.chars().count()) as u16;
        hitboxes.push(Hitbox {
            rect: Rect {
                x: cur_x,
                y: area.y,
                width: len,
                height: 1,
            },
            action: item.action,
        });

        if !item.prefix.is_empty() {
            spans.push(Span::styled(item.prefix, Style::default().fg(theme.text_bright)));
        }
        spans.push(Span::styled(
            item.key,
            Style::default().fg(theme.red).add_modifier(Modifier::BOLD),
        ));
        if !item.suffix.is_empty() {
            spans.push(Span::styled(item.suffix, Style::default().fg(theme.text_bright)));
        }
        cur_x += len;
    }

    let p = Paragraph::new(Line::from(spans))
        .alignment(Alignment::Left)
        .block(Block::default().style(Style::default().bg(theme.footer_bg)));
    f.render_widget(p, area);
}
