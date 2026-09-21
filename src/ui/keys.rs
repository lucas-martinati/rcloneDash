use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use crate::ui::theme::ThemePalette;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyAction {
    Files,
    ForceSync,
    Resync,
    DecTickRate,
    IncTickRate,
    Validate,
    CancelEdit,
}

pub struct KeybindingRegistry;

impl KeybindingRegistry {
    /// Returns the text representation of the key associated with the action
    pub fn get_key_str(action: KeyAction) -> &'static str {
        match action {
            KeyAction::Files => "b",
            KeyAction::ForceSync => "s",
            KeyAction::Resync => "r",
            KeyAction::DecTickRate => "-",
            KeyAction::IncTickRate => "+",
            KeyAction::Validate => "Enter",
            KeyAction::CancelEdit => "Esc",
        }
    }

    /// Generates a styled visual badge for a physical key (code block / keycap style)
    pub fn format_key_badge(key_str: &str, theme: &ThemePalette) -> Vec<Span<'static>> {
        let key_owned = key_str.to_string();
        vec![
            Span::styled(" [", Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD)),
            Span::styled(
                format!(" {} ", key_owned),
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Rgb(48, 55, 78))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("] ", Style::default().fg(theme.highlight).add_modifier(Modifier::BOLD)),
        ]
    }

    /// Formats an action label with key highlighting:
    /// - If `key` is a single character present in `word` (e.g. 'e' in "edit", 'a' in "add", 'd' in "del", 't' in "type", 'E' in "Editor"),
    ///   this character is highlighted inline with `key_fg` and `BOLD`.
    /// - If `key` is not in `word` or is a special key (e.g. "↵", "Esc", "Tab", "↑"),
    ///   the key is displayed as a prefix followed by a space and the word.
    pub fn format_shortcut_label(key: &str, word: &str, key_fg: Color, text_fg: Color) -> Vec<Span<'static>> {
        if let Some(key_char) = key.chars().next() {
            if key.chars().count() == 1 {
                let key_lower = key_char.to_lowercase().next().unwrap_or(key_char);
                let word_lower = word.to_lowercase();
                if let Some(byte_pos) = word_lower.find(key_lower) {
                    if let Some(found_char) = word[byte_pos..].chars().next() {
                        let char_len = found_char.len_utf8();
                        let mut spans = Vec::new();
                        if byte_pos > 0 {
                            spans.push(Span::styled(word[..byte_pos].to_string(), Style::default().fg(text_fg)));
                        }
                        spans.push(Span::styled(
                            word[byte_pos..byte_pos + char_len].to_string(),
                            Style::default().fg(key_fg).add_modifier(Modifier::BOLD),
                        ));
                        if byte_pos + char_len < word.len() {
                            spans.push(Span::styled(word[byte_pos + char_len..].to_string(), Style::default().fg(text_fg)));
                        }
                        return spans;
                    }
                }
            }
        }

        vec![
            Span::styled(key.to_string(), Style::default().fg(key_fg).add_modifier(Modifier::BOLD)),
            Span::styled(format!(" {}", word), Style::default().fg(text_fg)),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_shortcut_label() {
        // Letter in word -> inline highlight
        let spans_edit = KeybindingRegistry::format_shortcut_label("e", "edit", Color::Yellow, Color::White);
        assert_eq!(spans_edit.len(), 2);
        assert_eq!(spans_edit[0].content, "e");
        assert_eq!(spans_edit[0].style.fg, Some(Color::Yellow));
        assert_eq!(spans_edit[1].content, "dit");
        assert_eq!(spans_edit[1].style.fg, Some(Color::White));

        // Key not in word -> prefix
        let spans_save = KeybindingRegistry::format_shortcut_label("↵", "save", Color::Green, Color::White);
        assert_eq!(spans_save.len(), 2);
        assert_eq!(spans_save[0].content, "↵");
        assert_eq!(spans_save[0].style.fg, Some(Color::Green));
        assert_eq!(spans_save[1].content, " save");
        assert_eq!(spans_save[1].style.fg, Some(Color::White));

        // Multi-character key (e.g. "Esc") -> prefix
        let spans_esc = KeybindingRegistry::format_shortcut_label("Esc", "cancel", Color::Red, Color::White);
        assert_eq!(spans_esc.len(), 2);
        assert_eq!(spans_esc[0].content, "Esc");
        assert_eq!(spans_esc[1].content, " cancel");
    }
}
