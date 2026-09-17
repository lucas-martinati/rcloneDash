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
            ThemeChoice::TokyoNight => "Tokyo Night",
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
                accent: Color::Rgb(122, 162, 247),     // Tokyo Blue
                blue: Color::Rgb(125, 207, 255),       // Sky Blue
                cyan: Color::Rgb(115, 218, 202),       // Teal
                green: Color::Rgb(158, 206, 106),      // Vivid Green
                yellow: Color::Rgb(224, 175, 104),     // Warm Amber
                orange: Color::Rgb(255, 158, 100),     // Soft Orange
                red: Color::Rgb(247, 118, 142),        // Tokyo Pink/Red
                purple: Color::Rgb(187, 154, 247),     // Lavender
                border: Color::Rgb(41, 46, 66),        // Subtle Border
                border_focus: Color::Rgb(122, 162, 247),
                card_bg: Color::Rgb(26, 27, 38),       // Night BG
                bg_main: Color::Rgb(16, 16, 28),       // Darker than card_bg
                header_bg: Color::Rgb(22, 23, 34),
                footer_bg: Color::Rgb(22, 23, 34),
                highlight: Color::Rgb(247, 118, 142),  // Red accent for key hints
                separator: Color::Rgb(52, 58, 82),
                text_bright: Color::Rgb(192, 202, 245),
                text_muted: Color::Rgb(86, 95, 137),
            },
            ThemeChoice::CatppuccinMocha => ThemePalette {
                accent: Color::Rgb(137, 180, 250),     // Blue
                blue: Color::Rgb(116, 199, 236),       // Sapphire
                cyan: Color::Rgb(148, 226, 213),       // Teal
                green: Color::Rgb(166, 227, 161),      // Green
                yellow: Color::Rgb(249, 226, 175),     // Yellow
                orange: Color::Rgb(250, 179, 135),     // Peach
                red: Color::Rgb(243, 139, 168),        // Red
                purple: Color::Rgb(203, 166, 247),     // Mauve
                border: Color::Rgb(49, 50, 68),        // Surface0
                border_focus: Color::Rgb(137, 180, 250),
                card_bg: Color::Rgb(24, 24, 37),       // Base
                bg_main: Color::Rgb(17, 17, 27),       // Mantle
                header_bg: Color::Rgb(20, 20, 32),
                footer_bg: Color::Rgb(20, 20, 32),
                highlight: Color::Rgb(243, 139, 168),  // Red
                separator: Color::Rgb(59, 60, 78),
                text_bright: Color::Rgb(205, 214, 244),// Text
                text_muted: Color::Rgb(108, 112, 134), // Overlay0
            },
            ThemeChoice::Nord => ThemePalette {
                accent: Color::Rgb(136, 192, 208),     // Frost Cyan
                blue: Color::Rgb(129, 161, 193),       // Frost Blue
                cyan: Color::Rgb(143, 188, 187),       // Frost Teal
                green: Color::Rgb(163, 190, 140),      // Aurora Green
                yellow: Color::Rgb(235, 203, 139),     // Aurora Yellow
                orange: Color::Rgb(208, 135, 112),     // Aurora Orange
                red: Color::Rgb(191, 97, 106),         // Aurora Red
                purple: Color::Rgb(180, 142, 173),     // Aurora Purple
                border: Color::Rgb(59, 66, 82),        // Polar Night 2
                border_focus: Color::Rgb(136, 192, 208),
                card_bg: Color::Rgb(46, 52, 64),       // Polar Night 1
                bg_main: Color::Rgb(36, 40, 50),
                header_bg: Color::Rgb(40, 45, 58),
                footer_bg: Color::Rgb(40, 45, 58),
                highlight: Color::Rgb(191, 97, 106),   // Aurora Red
                separator: Color::Rgb(67, 76, 94),
                text_bright: Color::Rgb(236, 239, 244),// Snow Storm
                text_muted: Color::Rgb(144, 153, 167),
            },
            ThemeChoice::GruvboxDark => ThemePalette {
                accent: Color::Rgb(254, 128, 25),      // Bright Orange
                blue: Color::Rgb(131, 165, 152),       // Bright Blue
                cyan: Color::Rgb(142, 192, 124),       // Bright Aqua
                green: Color::Rgb(184, 187, 38),       // Bright Green
                yellow: Color::Rgb(250, 189, 47),      // Bright Yellow
                orange: Color::Rgb(254, 128, 25),
                red: Color::Rgb(251, 73, 52),          // Bright Red
                purple: Color::Rgb(211, 134, 155),     // Bright Purple
                border: Color::Rgb(60, 56, 54),        // Dark 1
                border_focus: Color::Rgb(254, 128, 25),
                card_bg: Color::Rgb(40, 40, 40),       // Dark 0
                bg_main: Color::Rgb(29, 32, 33),       // Hard Dark
                header_bg: Color::Rgb(35, 36, 35),
                footer_bg: Color::Rgb(35, 36, 35),
                highlight: Color::Rgb(251, 73, 52),    // Bright Red
                separator: Color::Rgb(80, 73, 69),
                text_bright: Color::Rgb(235, 219, 178),// Light 1
                text_muted: Color::Rgb(146, 131, 116), // Gray
            },
            ThemeChoice::Dracula => ThemePalette {
                accent: Color::Rgb(189, 147, 249),     // Purple
                blue: Color::Rgb(139, 233, 253),       // Cyan
                cyan: Color::Rgb(139, 233, 253),
                green: Color::Rgb(80, 250, 123),       // Green
                yellow: Color::Rgb(241, 250, 140),     // Yellow
                orange: Color::Rgb(255, 184, 108),     // Orange
                red: Color::Rgb(255, 85, 85),          // Red
                purple: Color::Rgb(255, 121, 198),     // Pink
                border: Color::Rgb(68, 71, 90),        // Current Line
                border_focus: Color::Rgb(189, 147, 249),
                card_bg: Color::Rgb(40, 42, 54),       // Background
                bg_main: Color::Rgb(30, 31, 41),
                header_bg: Color::Rgb(34, 36, 48),
                footer_bg: Color::Rgb(34, 36, 48),
                highlight: Color::Rgb(255, 85, 85),    // Red
                separator: Color::Rgb(80, 83, 105),
                text_bright: Color::Rgb(248, 248, 242),// Foreground
                text_muted: Color::Rgb(98, 114, 164),  // Comment
            },
            ThemeChoice::MonokaiPro => ThemePalette {
                accent: Color::Rgb(169, 220, 103),     // Green
                blue: Color::Rgb(120, 220, 232),       // Cyan
                cyan: Color::Rgb(120, 220, 232),
                green: Color::Rgb(169, 220, 103),      // Green
                yellow: Color::Rgb(255, 216, 102),     // Yellow
                orange: Color::Rgb(252, 152, 103),     // Orange
                red: Color::Rgb(255, 97, 136),         // Red
                purple: Color::Rgb(171, 157, 242),     // Purple
                border: Color::Rgb(58, 56, 63),
                border_focus: Color::Rgb(169, 220, 103),
                card_bg: Color::Rgb(45, 42, 46),
                bg_main: Color::Rgb(33, 31, 35),
                header_bg: Color::Rgb(38, 36, 40),
                footer_bg: Color::Rgb(38, 36, 40),
                highlight: Color::Rgb(255, 97, 136),   // Red
                separator: Color::Rgb(72, 69, 76),
                text_bright: Color::Rgb(247, 241, 255),
                text_muted: Color::Rgb(114, 112, 122),
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
}
