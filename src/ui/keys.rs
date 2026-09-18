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
}
