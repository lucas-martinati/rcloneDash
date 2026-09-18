use ratatui::style::Color;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeChoice {
    TokyoNight,
    CatppuccinMocha,
    Nord,
    GruvboxDark,
    Dracula,
    MonokaiPro,
}

#[allow(dead_code)]
impl ThemeChoice {
    #[allow(dead_code)]
    pub fn all() -> &'static [ThemeChoice] {
        &[
            ThemeChoice::TokyoNight,
            ThemeChoice::CatppuccinMocha,
            ThemeChoice::Nord,
            ThemeChoice::GruvboxDark,
            ThemeChoice::Dracula,
            ThemeChoice::MonokaiPro,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            ThemeChoice::TokyoNight => "Tokyo Night (btop default)",
            ThemeChoice::CatppuccinMocha => "Catppuccin Mocha",
            ThemeChoice::Nord => "Nord Frost",
            ThemeChoice::GruvboxDark => "Gruvbox Dark",
            ThemeChoice::Dracula => "Dracula",
            ThemeChoice::MonokaiPro => "Monokai Pro",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            ThemeChoice::TokyoNight => ThemeChoice::CatppuccinMocha,
            ThemeChoice::CatppuccinMocha => ThemeChoice::Nord,
            ThemeChoice::Nord => ThemeChoice::GruvboxDark,
            ThemeChoice::GruvboxDark => ThemeChoice::Dracula,
            ThemeChoice::Dracula => ThemeChoice::MonokaiPro,
            ThemeChoice::MonokaiPro => ThemeChoice::TokyoNight,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            ThemeChoice::TokyoNight => ThemeChoice::MonokaiPro,
            ThemeChoice::CatppuccinMocha => ThemeChoice::TokyoNight,
            ThemeChoice::Nord => ThemeChoice::CatppuccinMocha,
            ThemeChoice::GruvboxDark => ThemeChoice::Nord,
            ThemeChoice::Dracula => ThemeChoice::GruvboxDark,
            ThemeChoice::MonokaiPro => ThemeChoice::Dracula,
        }
    }

    pub fn palette(&self) -> ThemePalette {
        match self {
            ThemeChoice::TokyoNight => ThemePalette {
                accent: Color::Rgb(125, 207, 255),       // Sky Blue / Cyan
                blue: Color::Rgb(122, 162, 247),         // Tokyo Blue
                cyan: Color::Rgb(115, 218, 202),         // Teal
                green: Color::Rgb(158, 206, 106),        // Vivid Green
                yellow: Color::Rgb(224, 175, 104),       // Warm Amber
                orange: Color::Rgb(255, 158, 100),       // Soft Orange
                red: Color::Rgb(247, 118, 142),          // Coral / Red
                purple: Color::Rgb(187, 154, 247),       // Lavender
                border: Color::Rgb(70, 75, 95),          // Subtle Slate Grey
                border_focus: Color::Rgb(224, 175, 104), // Warm Amber / Gold
                border_sys: Color::Rgb(115, 218, 202),   // Cyan / Teal
                border_storage: Color::Rgb(187, 154, 247), // Lavender
                border_history: Color::Rgb(122, 162, 247), // Tokyo Blue
                border_logs: Color::Rgb(247, 118, 142),    // Coral / Salmon
                border_recent: Color::Rgb(158, 206, 106),  // Green
                card_bg: Color::Reset,                   // Transparent / Terminal Black
                bg_main: Color::Reset,
                header_bg: Color::Reset,
                footer_bg: Color::Reset,
                highlight: Color::Rgb(125, 207, 255),    // Sky Blue for key hints
                separator: Color::Rgb(52, 58, 82),
                text_bright: Color::Rgb(240, 242, 250),
                text_muted: Color::Rgb(105, 115, 150),
            },
            ThemeChoice::CatppuccinMocha => ThemePalette {
                accent: Color::Rgb(137, 180, 250),       // Blue
                blue: Color::Rgb(116, 199, 236),         // Sapphire
                cyan: Color::Rgb(148, 226, 213),         // Teal
                green: Color::Rgb(166, 227, 161),        // Green
                yellow: Color::Rgb(249, 226, 175),       // Yellow
                orange: Color::Rgb(250, 179, 135),       // Peach
                red: Color::Rgb(243, 139, 168),          // Red
                purple: Color::Rgb(203, 166, 247),       // Mauve
                border: Color::Rgb(75, 78, 100),         // Surface1
                border_focus: Color::Rgb(249, 226, 175), // Catppuccin Yellow / Gold
                border_sys: Color::Rgb(148, 226, 213),
                border_storage: Color::Rgb(203, 166, 247),
                border_history: Color::Rgb(137, 180, 250),
                border_logs: Color::Rgb(243, 139, 168),
                border_recent: Color::Rgb(166, 227, 161),
                card_bg: Color::Reset,
                bg_main: Color::Reset,
                header_bg: Color::Reset,
                footer_bg: Color::Reset,
                highlight: Color::Rgb(243, 139, 168),    // Red
                separator: Color::Rgb(59, 60, 78),
                text_bright: Color::Rgb(240, 243, 255),
                text_muted: Color::Rgb(125, 130, 155),
            },
            ThemeChoice::Nord => ThemePalette {
                accent: Color::Rgb(136, 192, 208),       // Frost Cyan
                blue: Color::Rgb(129, 161, 193),         // Frost Blue
                cyan: Color::Rgb(143, 188, 187),         // Frost Teal
                green: Color::Rgb(163, 190, 140),        // Aurora Green
                yellow: Color::Rgb(235, 203, 139),       // Aurora Yellow
                orange: Color::Rgb(208, 135, 112),       // Aurora Orange
                red: Color::Rgb(191, 97, 106),           // Aurora Red
                purple: Color::Rgb(180, 142, 173),       // Aurora Purple
                border: Color::Rgb(76, 86, 106),         // Polar Night 3
                border_focus: Color::Rgb(235, 203, 139), // Aurora Yellow / Amber
                border_sys: Color::Rgb(143, 188, 187),
                border_storage: Color::Rgb(180, 142, 173),
                border_history: Color::Rgb(129, 161, 193),
                border_logs: Color::Rgb(191, 97, 106),
                border_recent: Color::Rgb(163, 190, 140),
                card_bg: Color::Reset,
                bg_main: Color::Reset,
                header_bg: Color::Reset,
                footer_bg: Color::Reset,
                highlight: Color::Rgb(136, 192, 208),
                separator: Color::Rgb(67, 76, 94),
                text_bright: Color::Rgb(240, 244, 250),
                text_muted: Color::Rgb(130, 140, 160),
            },
            ThemeChoice::GruvboxDark => ThemePalette {
                accent: Color::Rgb(142, 192, 124),       // Aqua (calm & legible)
                blue: Color::Rgb(131, 165, 152),         // Blue
                cyan: Color::Rgb(142, 192, 124),         // Aqua
                green: Color::Rgb(184, 187, 38),         // Green
                yellow: Color::Rgb(250, 189, 47),        // Yellow
                orange: Color::Rgb(254, 128, 25),        // Orange
                red: Color::Rgb(251, 73, 52),            // Red
                purple: Color::Rgb(211, 134, 155),       // Purple
                border: Color::Rgb(80, 73, 69),          // Dark 2 subtle grey
                border_focus: Color::Rgb(250, 189, 47),  // Gruvbox Bright Yellow / Gold
                border_sys: Color::Rgb(142, 192, 124),   // Aqua
                border_storage: Color::Rgb(211, 134, 155),// Purple
                border_history: Color::Rgb(131, 165, 152),// Blue
                border_logs: Color::Rgb(251, 73, 52),    // Red
                border_recent: Color::Rgb(184, 187, 38),  // Green
                card_bg: Color::Reset,                   // Pure black terminal
                bg_main: Color::Reset,
                header_bg: Color::Reset,
                footer_bg: Color::Reset,
                highlight: Color::Rgb(250, 189, 47),    // Yellow
                separator: Color::Rgb(80, 73, 69),
                text_bright: Color::Rgb(245, 235, 210),  // Light Cream
                text_muted: Color::Rgb(150, 135, 120),
            },
            ThemeChoice::Dracula => ThemePalette {
                accent: Color::Rgb(189, 147, 249),       // Purple
                blue: Color::Rgb(139, 233, 253),         // Cyan
                cyan: Color::Rgb(139, 233, 253),
                green: Color::Rgb(80, 250, 123),         // Green
                yellow: Color::Rgb(241, 250, 140),       // Yellow
                orange: Color::Rgb(255, 184, 108),       // Orange
                red: Color::Rgb(255, 85, 85),            // Red
                purple: Color::Rgb(255, 121, 198),       // Pink
                border: Color::Rgb(80, 85, 110),         // Current Line grey
                border_focus: Color::Rgb(241, 250, 140), // Dracula Yellow
                border_sys: Color::Rgb(139, 233, 253),
                border_storage: Color::Rgb(255, 121, 198),
                border_history: Color::Rgb(139, 233, 253),// Cyan
                border_logs: Color::Rgb(255, 85, 85),
                border_recent: Color::Rgb(80, 250, 123),
                card_bg: Color::Reset,
                bg_main: Color::Reset,
                header_bg: Color::Reset,
                footer_bg: Color::Reset,
                highlight: Color::Rgb(255, 121, 198),
                separator: Color::Rgb(80, 83, 105),
                text_bright: Color::Rgb(248, 248, 242),
                text_muted: Color::Rgb(130, 140, 175),
            },
            ThemeChoice::MonokaiPro => ThemePalette {
                accent: Color::Rgb(169, 220, 103),       // Green
                blue: Color::Rgb(120, 220, 232),         // Cyan
                cyan: Color::Rgb(120, 220, 232),
                green: Color::Rgb(169, 220, 103),        // Green
                yellow: Color::Rgb(255, 216, 102),       // Yellow
                orange: Color::Rgb(252, 152, 103),       // Orange
                red: Color::Rgb(255, 97, 136),           // Red
                purple: Color::Rgb(171, 157, 242),       // Purple
                border: Color::Rgb(75, 72, 82),
                border_focus: Color::Rgb(255, 216, 102), // Monokai Pro Yellow / Gold
                border_sys: Color::Rgb(120, 220, 232),
                border_storage: Color::Rgb(171, 157, 242),
                border_history: Color::Rgb(120, 220, 232),
                border_logs: Color::Rgb(255, 97, 136),
                border_recent: Color::Rgb(169, 220, 103),
                card_bg: Color::Reset,
                bg_main: Color::Reset,
                header_bg: Color::Reset,
                footer_bg: Color::Reset,
                highlight: Color::Rgb(255, 216, 102),
                separator: Color::Rgb(72, 69, 76),
                text_bright: Color::Rgb(250, 245, 255),
                text_muted: Color::Rgb(135, 130, 145),
            },
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct ThemePalette {
    pub accent: Color,
    pub blue: Color,
    pub cyan: Color,
    pub green: Color,
    pub yellow: Color,
    pub orange: Color,
    pub red: Color,
    pub purple: Color,
    pub border: Color,
    pub border_focus: Color,
    pub border_sys: Color,
    pub border_storage: Color,
    pub border_history: Color,
    pub border_logs: Color,
    pub border_recent: Color,
    pub card_bg: Color,
    pub bg_main: Color,
    pub header_bg: Color,
    pub footer_bg: Color,
    pub highlight: Color,
    pub separator: Color,
    pub text_bright: Color,
    pub text_muted: Color,
}

impl ThemePalette {
    /// Retourne une couleur de dégradé pour la vitesse (style btop++)
    #[allow(dead_code)]
    pub fn speed_gradient_color(&self, speed_kibs: u64) -> Color {
        if speed_kibs > 20_000 {
            self.red
        } else if speed_kibs > 5_000 {
            self.orange
        } else if speed_kibs > 1_000 {
            self.yellow
        } else if speed_kibs > 200 {
            self.green
        } else {
            self.cyan
        }
    }

    /// Convertit l'ensemble des couleurs en nuances de gris (monochrome style btop++)
    pub fn to_grayscale(&self) -> ThemePalette {
        let to_gray = |c: Color| -> Color {
            match c {
                Color::Rgb(r, g, b) => {
                    let lum = ((r as u32 * 299 + g as u32 * 587 + b as u32 * 114) / 1000) as u8;
                    // Léger assombrissement (85%) pour faire ressortir les modales et le menu coloré
                    let dim = ((lum as u32 * 85) / 100) as u8;
                    Color::Rgb(dim, dim, dim)
                }
                Color::Reset => Color::Reset,
                _ => Color::DarkGray,
            }
        };

        ThemePalette {
            accent: to_gray(self.accent),
            blue: to_gray(self.blue),
            cyan: to_gray(self.cyan),
            green: to_gray(self.green),
            yellow: to_gray(self.yellow),
            orange: to_gray(self.orange),
            red: to_gray(self.red),
            purple: to_gray(self.purple),
            border: to_gray(self.border),
            border_focus: to_gray(self.border_focus),
            border_sys: to_gray(self.border_sys),
            border_storage: to_gray(self.border_storage),
            border_history: to_gray(self.border_history),
            border_logs: to_gray(self.border_logs),
            border_recent: to_gray(self.border_recent),
            card_bg: self.card_bg,
            bg_main: self.bg_main,
            header_bg: self.header_bg,
            footer_bg: self.footer_bg,
            highlight: to_gray(self.highlight),
            separator: to_gray(self.separator),
            text_bright: to_gray(self.text_bright),
            text_muted: to_gray(self.text_muted),
        }
    }
}
