use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use crate::ui::theme::ThemePalette;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyAction {
    Menu,
    Settings,
    Files,
    Filters,
    Help,
    QuickSync,
    DryRun,
    Resync,
    CancelSync,
    ToggleFolderMode,
    FilterRecent,
    TogglePauseLogs,
    DecTickRate,
    IncTickRate,
    Quit,
    CloseModal,
    Validate,
    CancelEdit,
}

#[allow(dead_code)]
pub struct KeybindingRegistry;

#[allow(dead_code)]
impl KeybindingRegistry {
    /// Retourne la représentation textuelle de la touche associée à l'action
    pub fn get_key_str(action: KeyAction) -> &'static str {
        match action {
            KeyAction::Menu => "m",
            KeyAction::Settings => "o",
            KeyAction::Files => "b",
            KeyAction::Filters => "e",
            KeyAction::Help => "h",
            KeyAction::QuickSync => "s",
            KeyAction::DryRun => "d",
            KeyAction::Resync => "r",
            KeyAction::CancelSync => "c",
            KeyAction::ToggleFolderMode => "Ctrl+X",
            KeyAction::FilterRecent => "f",
            KeyAction::TogglePauseLogs => "Espace",
            KeyAction::DecTickRate => "-",
            KeyAction::IncTickRate => "+",
            KeyAction::Quit => "q",
            KeyAction::CloseModal => "Échap / q",
            KeyAction::Validate => "Entrée",
            KeyAction::CancelEdit => "Échap",
        }
    }

    /// Génère un badge visuel stylisé pour une touche physique (style bloc de code / touche clavier)
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

    /// Génère un badge stylisé directement depuis l'action
    pub fn format_action_badge(action: KeyAction, theme: &ThemePalette) -> Vec<Span<'static>> {
        Self::format_key_badge(Self::get_key_str(action), theme)
    }

    /// Formate un libellé d'action btop++ avec mise en valeur de la touche :
    /// - Si `key` est un caractère présent dans `word` (ex: 'e' dans "edit", 'a' dans "add", 'd' dans "del", 't' dans "type", 'E' dans "Editor"),
    ///   ce caractère est mis en valeur avec `key_fg` et `BOLD` directement au sein du mot.
    /// - Si `key` n'est pas présent dans `word` ou s'il s'agit d'une touche spéciale (ex: "↵", "Esc", "Tab", "↑"),
    ///   la touche est affichée en préfixe suivie d'un espace et du mot.
    pub fn format_shortcut_label(key: &str, word: &str, key_fg: Color, text_fg: Color) -> Vec<Span<'static>> {
        if key.chars().count() == 1 {
            let key_char = key.chars().next().unwrap();
            let key_lower = key_char.to_lowercase().next().unwrap();
            let word_lower = word.to_lowercase();
            if let Some(byte_pos) = word_lower.find(key_lower) {
                let char_len = word[byte_pos..].chars().next().unwrap().len_utf8();
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
        // Lettre contenue dans le mot -> mise en valeur inline
        let spans_edit = KeybindingRegistry::format_shortcut_label("e", "edit", Color::Yellow, Color::White);
        assert_eq!(spans_edit.len(), 2);
        assert_eq!(spans_edit[0].content, "e");
        assert_eq!(spans_edit[0].style.fg, Some(Color::Yellow));
        assert_eq!(spans_edit[1].content, "dit");
        assert_eq!(spans_edit[1].style.fg, Some(Color::White));

        // Touche non contenue dans le mot -> préfixe
        let spans_save = KeybindingRegistry::format_shortcut_label("↵", "save", Color::Green, Color::White);
        assert_eq!(spans_save.len(), 2);
        assert_eq!(spans_save[0].content, "↵");
        assert_eq!(spans_save[0].style.fg, Some(Color::Green));
        assert_eq!(spans_save[1].content, " save");
        assert_eq!(spans_save[1].style.fg, Some(Color::White));

        // Touche multi-caractères (ex: "Esc") -> préfixe
        let spans_esc = KeybindingRegistry::format_shortcut_label("Esc", "cancel", Color::Red, Color::White);
        assert_eq!(spans_esc.len(), 2);
        assert_eq!(spans_esc[0].content, "Esc");
        assert_eq!(spans_esc[1].content, " cancel");
    }
}
