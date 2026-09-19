use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

use crate::app::{App, HitAction, Hitbox};
use crate::ui::keys::KeybindingRegistry;
use crate::ui::theme::ThemePalette;

pub fn render_footer(
    f: &mut Frame,
    app: &App,
    theme: &ThemePalette,
    area: Rect,
    hitboxes: &mut Vec<Hitbox>,
) {
    let sync_or_cancel = if app.live.is_syncing {
        ("c", "cancel", HitAction::ButtonCancel)
    } else {
        ("s", "sync", HitAction::ButtonSync)
    };

    let items = [
        sync_or_cancel,
        ("d", "dry-run", HitAction::ButtonDryRun),
        ("b", "browse", HitAction::ButtonFiles),
        ("e", "filters", HitAction::ButtonFilters),
        ("y", "copy", HitAction::ButtonCopy),
        ("Tab", "panel", HitAction::ButtonPanel),
    ];

    let mut spans: Vec<Span> = Vec::new();
    let mut cur_x = area.x;

    for (i, (key, word, action)) in items.into_iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", Style::default()));
            cur_x += 2;
        }

        let label_spans = KeybindingRegistry::format_shortcut_label(key, word, theme.red, theme.text_bright);
        let len: u16 = label_spans.iter().map(|s| s.content.chars().count() as u16).sum();

        hitboxes.push(Hitbox {
            rect: Rect {
                x: cur_x,
                y: area.y,
                width: len,
                height: 1,
            },
            action,
        });

        spans.extend(label_spans);
        cur_x += len;
    }

    let p = Paragraph::new(Line::from(spans))
        .alignment(Alignment::Left)
        .block(Block::default().style(Style::default().bg(theme.footer_bg)));
    f.render_widget(p, area);
}
