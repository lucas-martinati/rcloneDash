//! Terminal capabilities for the btop++-style "TTY mode".
//!
//! Three independent axes, deliberately kept separate:
//! - `color`: which ANSI palette the terminal understands
//!   (`NO_COLOR` removes escape codes but is *not* a "no UI" mode).
//! - `live`: whether in-place animation (`\r`, `\x1b[K`) is possible,
//!   i.e. stdout really is a terminal.
//! - `ascii`: whether Unicode glyphs must be replaced by ASCII fallbacks
//!   (Linux console, non-UTF-8 locale).
//!
//! Detection is a pure function of an env getter so unit tests never touch
//! the real process environment (parallel tests must never mutate
//! `std::env`).

use ratatui::style::Color;
use serde::{Deserialize, Serialize};

/// Color palette level understood by the terminal, ordered from weakest to
/// strongest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum ColorLevel {
    /// `NO_COLOR` is set: emit no ANSI escape codes at all.
    None,
    /// Linux console (`TERM=linux`): 16 base ANSI colors, still in color.
    Ansi16,
    /// Common modern terminals without truecolor advertisement.
    #[default]
    Ansi256,
    /// `COLORTERM` advertises `truecolor` or `24bit`.
    TrueColor,
}

impl ColorLevel {
    /// `true` unless `NO_COLOR` asked for plain output.
    pub fn supports_ansi(self) -> bool {
        !matches!(self, ColorLevel::None)
    }
}

/// The three independent terminal capability axes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TermCaps {
    pub color: ColorLevel,
    pub live: bool,
    pub ascii: bool,
}

impl TermCaps {
    /// Full modern terminal: truecolor, live animation, Unicode glyphs.
    pub fn full() -> Self {
        Self {
            color: ColorLevel::TrueColor,
            live: true,
            ascii: false,
        }
    }

    /// Non-terminal pipe: no colors, no animation, ASCII glyphs.
    pub fn plain() -> Self {
        Self {
            color: ColorLevel::None,
            live: false,
            ascii: true,
        }
    }

    /// Pure detection from an env getter and an `is_terminal` flag.
    ///
    /// - `live` is exactly `is_terminal`.
    /// - `color`: piped stdout (not a terminal) has no color capability ->
    ///   `None`; `NO_COLOR` set -> `None`; `TERM=linux` -> `Ansi16`;
    ///   `COLORTERM` containing `truecolor`/`24bit` -> `TrueColor`;
    ///   otherwise `Ansi256`. (An explicit `TtyMode::Off` can still force
    ///   colors, like `ls --color=always`.)
    /// - `ascii`: `TERM=linux`, or the locale (`LC_ALL`, then `LC_CTYPE`,
    ///   then `LANG`) is set to a non-UTF-8 value.
    pub fn detect_from(env: &impl Fn(&str) -> Option<String>, is_terminal: bool) -> Self {
        let term = env("TERM").unwrap_or_default();
        let is_linux_console = term.trim().eq_ignore_ascii_case("linux");

        let color = if !is_terminal || env("NO_COLOR").is_some() {
            ColorLevel::None
        } else if is_linux_console {
            ColorLevel::Ansi16
        } else {
            let colorterm = env("COLORTERM").unwrap_or_default().to_lowercase();
            if colorterm.contains("truecolor") || colorterm.contains("24bit") {
                ColorLevel::TrueColor
            } else {
                ColorLevel::Ansi256
            }
        };

        Self {
            color,
            live: is_terminal,
            ascii: is_linux_console || locale_is_not_utf8(env),
        }
    }

    /// Detection against the real process environment.
    pub fn detect() -> Self {
        use std::io::IsTerminal;
        Self::detect_from(
            &|k: &str| std::env::var(k).ok(),
            std::io::stdout().is_terminal(),
        )
    }

    /// Resolves an explicit [`TtyMode`] on top of detection:
    /// - `Auto`: detection unchanged.
    /// - `On`: forces `Ansi16` + ASCII (stays in color, like btop++).
    /// - `Off`: forces `TrueColor` (or the detected level if stronger) +
    ///   Unicode. Animation (`live`) always follows detection.
    pub fn resolve_from(
        mode: TtyMode,
        env: &impl Fn(&str) -> Option<String>,
        is_terminal: bool,
    ) -> Self {
        let detected = Self::detect_from(env, is_terminal);
        match mode {
            TtyMode::Auto => detected,
            TtyMode::On => Self {
                color: ColorLevel::Ansi16,
                live: detected.live,
                ascii: true,
            },
            TtyMode::Off => Self {
                color: detected.color.max(ColorLevel::TrueColor),
                live: detected.live,
                ascii: false,
            },
        }
    }

    /// [`TtyMode`] resolution against the real process environment.
    pub fn resolve(mode: TtyMode) -> Self {
        use std::io::IsTerminal;
        Self::resolve_from(
            mode,
            &|k: &str| std::env::var(k).ok(),
            std::io::stdout().is_terminal(),
        )
    }
}

/// `true` when an explicitly set locale does not advertise UTF-8.
/// Lookup order follows the usual precedence: `LC_ALL`, then `LC_CTYPE`,
/// then `LANG`. An unset locale defaults to UTF-8 (no fallback).
fn locale_is_not_utf8(env: &impl Fn(&str) -> Option<String>) -> bool {
    let locale = env("LC_ALL")
        .filter(|v| !v.trim().is_empty())
        .or_else(|| env("LC_CTYPE").filter(|v| !v.trim().is_empty()))
        .or_else(|| env("LANG").filter(|v| !v.trim().is_empty()));
    match locale {
        None => false,
        Some(v) => {
            let lower = v.to_lowercase();
            !(lower.contains("utf-8") || lower.contains("utf8"))
        }
    }
}

/// User-facing TTY mode switch: `auto | on | off` (default `auto`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TtyMode {
    #[default]
    Auto,
    On,
    Off,
}

impl TtyMode {
    pub fn all() -> &'static [TtyMode] {
        &[TtyMode::Auto, TtyMode::On, TtyMode::Off]
    }

    pub fn as_str(self) -> &'static str {
        match self {
            TtyMode::Auto => "auto",
            TtyMode::On => "on",
            TtyMode::Off => "off",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            TtyMode::Auto => "Auto (detect)",
            TtyMode::On => "On (16 colors + ASCII)",
            TtyMode::Off => "Off (full color + Unicode)",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "auto" => Some(TtyMode::Auto),
            "on" => Some(TtyMode::On),
            "off" => Some(TtyMode::Off),
            _ => None,
        }
    }

    /// Canonical option labels, single source of truth for the setting UI.
    pub fn options() -> Vec<String> {
        Self::all().iter().map(|m| m.as_str().to_string()).collect()
    }

    pub fn next(self) -> Self {
        match self {
            TtyMode::Auto => TtyMode::On,
            TtyMode::On => TtyMode::Off,
            TtyMode::Off => TtyMode::Auto,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            TtyMode::Auto => TtyMode::Off,
            TtyMode::On => TtyMode::Auto,
            TtyMode::Off => TtyMode::On,
        }
    }
}

/// Explicit reference table of the 16 base ANSI colors with their RGB
/// values. These match the definitions used by the theme helpers
/// (`ThemePalette` utilities map the same names to the same triplets), so a
/// pure primary like `Rgb(255, 0, 0)` unambiguously resolves to `Red`.
pub const ANSI16_TABLE: [(Color, (u8, u8, u8)); 16] = [
    (Color::Black, (0, 0, 0)),
    (Color::Red, (255, 0, 0)),
    (Color::Green, (0, 255, 0)),
    (Color::Yellow, (255, 255, 0)),
    (Color::Blue, (0, 0, 255)),
    (Color::Magenta, (255, 0, 255)),
    (Color::Cyan, (0, 255, 255)),
    (Color::Gray, (128, 128, 128)),
    (Color::DarkGray, (64, 64, 64)),
    (Color::LightRed, (255, 100, 100)),
    (Color::LightGreen, (100, 255, 100)),
    (Color::LightYellow, (255, 255, 100)),
    (Color::LightBlue, (100, 100, 255)),
    (Color::LightMagenta, (255, 100, 255)),
    (Color::LightCyan, (100, 255, 255)),
    (Color::White, (255, 255, 255)),
];

/// Maps an `Rgb` triplet to the nearest xterm 256-color palette index:
/// color cube entries 16-231, gray ramp 232-255 for equal components
/// (pure black → 16, near-white → 231).
fn rgb_to_ansi256_index(r: u8, g: u8, b: u8) -> u8 {
    if r == g && g == b {
        if r < 8 {
            16
        } else if r > 248 {
            231
        } else {
            232 + (r - 8) / 10
        }
    } else {
        // Quantize each channel to 0-5 with rounding.
        let q = |v: u8| ((v as u16 * 5 + 127) / 255) as u8;
        16 + 36 * q(r) + 6 * q(g) + q(b)
    }
}

/// Recovers the RGB triplet behind an xterm 256-color index (standard cube
/// levels and gray ramp), so `Indexed` colors can join the ANSI16 search.
fn ansi256_index_to_rgb(idx: u8) -> (u8, u8, u8) {
    match idx {
        0..=15 => ANSI16_TABLE[idx as usize].1,
        16..=231 => {
            const LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];
            let i = idx - 16;
            (
                LEVELS[(i / 36) as usize],
                LEVELS[((i % 36) / 6) as usize],
                LEVELS[(i % 6) as usize],
            )
        }
        _ => {
            let v = 8 + 10 * (idx - 232);
            (v, v, v)
        }
    }
}

/// Nearest base ANSI color by euclidean distance in RGB space. Ties resolve
/// to the earliest table entry (plain colors before bright ones).
fn nearest_ansi16(r: u8, g: u8, b: u8) -> Color {
    let mut best = ANSI16_TABLE[0].0;
    let mut best_dist = u32::MAX;
    for (color, (tr, tg, tb)) in ANSI16_TABLE {
        let dr = r as i32 - tr as i32;
        let dg = g as i32 - tg as i32;
        let db = b as i32 - tb as i32;
        let dist = (dr * dr + dg * dg + db * db) as u32;
        if dist < best_dist {
            best_dist = dist;
            best = color;
        }
    }
    best
}

/// Reduces a single color to what `level` can display:
///
/// - `TrueColor`: identity.
/// - `Ansi256`: `Rgb` becomes the nearest xterm 256-color `Indexed` entry;
///   named colors and `Reset` pass through untouched.
/// - `Ansi16`: `Rgb` (or `Indexed`, via its RGB value) becomes the nearest
///   of the 16 base ANSI colors; named colors and `Reset` pass through.
/// - `None`: any concrete color becomes `Reset` (honest `NO_COLOR`).
pub fn downgrade_color(color: Color, level: ColorLevel) -> Color {
    match level {
        ColorLevel::TrueColor => color,
        // Honest NO_COLOR: everything becomes the terminal default.
        ColorLevel::None => Color::Reset,
        ColorLevel::Ansi256 => match color {
            Color::Rgb(r, g, b) => Color::Indexed(rgb_to_ansi256_index(r, g, b)),
            other => other,
        },
        ColorLevel::Ansi16 => match color {
            Color::Rgb(r, g, b) => nearest_ansi16(r, g, b),
            Color::Indexed(idx) => {
                let (r, g, b) = ansi256_index_to_rgb(idx);
                nearest_ansi16(r, g, b)
            }
            other => other,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env_of(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |k: &str| map.get(k).cloned()
    }

    #[test]
    fn test_linux_console_forces_ansi16_ascii() {
        let env = env_of(&[("TERM", "linux"), ("LANG", "en_US.UTF-8")]);
        let caps = TermCaps::detect_from(&env, true);
        assert_eq!(caps.color, ColorLevel::Ansi16);
        assert!(caps.live);
        assert!(caps.ascii);
    }

    #[test]
    fn test_truecolor_advertised() {
        let env = env_of(&[
            ("TERM", "xterm-256color"),
            ("COLORTERM", "truecolor"),
            ("LANG", "en_US.UTF-8"),
        ]);
        let caps = TermCaps::detect_from(&env, true);
        assert_eq!(caps.color, ColorLevel::TrueColor);
        assert!(caps.live);
        assert!(!caps.ascii);
    }

    #[test]
    fn test_24bit_colorterm_is_truecolor() {
        let env = env_of(&[("TERM", "xterm-256color"), ("COLORTERM", "24bit")]);
        let caps = TermCaps::detect_from(&env, true);
        assert_eq!(caps.color, ColorLevel::TrueColor);
    }

    #[test]
    fn test_plain_terminal_defaults_to_ansi256() {
        let env = env_of(&[("TERM", "xterm-256color"), ("LANG", "en_US.UTF-8")]);
        let caps = TermCaps::detect_from(&env, true);
        assert_eq!(caps.color, ColorLevel::Ansi256);
        assert!(caps.live);
        assert!(!caps.ascii);
    }

    #[test]
    fn test_no_color_removes_ansi_but_keeps_live_bar() {
        // NO_COLOR kills colors yet the bar stays alive on a real terminal.
        let env = env_of(&[("TERM", "xterm-256color"), ("NO_COLOR", "1")]);
        let caps = TermCaps::detect_from(&env, true);
        assert_eq!(caps.color, ColorLevel::None);
        assert!(!caps.color.supports_ansi());
        assert!(caps.live);
    }

    #[test]
    fn test_pipe_removes_both_color_and_animation() {
        // Piped stdout has no color capability and no in-place animation.
        let env = env_of(&[("TERM", "xterm-256color"), ("COLORTERM", "truecolor")]);
        let caps = TermCaps::detect_from(&env, false);
        assert_eq!(caps.color, ColorLevel::None);
        assert!(!caps.live);
    }

    #[test]
    fn test_no_color_on_pipe_is_fully_plain() {
        let env = env_of(&[("NO_COLOR", "")]);
        let caps = TermCaps::detect_from(&env, false);
        assert_eq!(caps.color, ColorLevel::None);
        assert!(!caps.live);
    }

    #[test]
    fn test_c_locale_forces_ascii() {
        let env = env_of(&[("TERM", "xterm-256color"), ("LANG", "C")]);
        let caps = TermCaps::detect_from(&env, true);
        assert!(caps.ascii);
        assert_eq!(caps.color, ColorLevel::Ansi256);
    }

    #[test]
    fn test_lc_all_takes_precedence_over_lang() {
        let env = env_of(&[("LC_ALL", "C"), ("LANG", "en_US.UTF-8")]);
        let caps = TermCaps::detect_from(&env, true);
        assert!(caps.ascii);

        let env = env_of(&[("LC_ALL", "en_US.UTF-8"), ("LANG", "C")]);
        let caps = TermCaps::detect_from(&env, true);
        assert!(!caps.ascii);
    }

    #[test]
    fn test_unset_locale_defaults_to_unicode() {
        let env = env_of(&[("TERM", "xterm-256color")]);
        let caps = TermCaps::detect_from(&env, true);
        assert!(!caps.ascii);
    }

    #[test]
    fn test_tty_mode_on_forces_ansi16_ascii() {
        let env = env_of(&[
            ("TERM", "xterm-256color"),
            ("COLORTERM", "truecolor"),
            ("LANG", "en_US.UTF-8"),
        ]);
        let caps = TermCaps::resolve_from(TtyMode::On, &env, true);
        assert_eq!(caps.color, ColorLevel::Ansi16);
        assert!(caps.ascii);
        assert!(caps.live);
    }

    #[test]
    fn test_tty_mode_off_forces_truecolor_unicode() {
        let env = env_of(&[("TERM", "linux"), ("LANG", "C")]);
        let caps = TermCaps::resolve_from(TtyMode::Off, &env, true);
        assert_eq!(caps.color, ColorLevel::TrueColor);
        assert!(!caps.ascii);
        assert!(caps.live);
    }

    #[test]
    fn test_tty_mode_auto_is_detection() {
        let env = env_of(&[("TERM", "linux")]);
        let auto = TermCaps::resolve_from(TtyMode::Auto, &env, true);
        let detected = TermCaps::detect_from(&env, true);
        assert_eq!(auto, detected);
    }

    #[test]
    fn test_tty_mode_parse_and_cycle() {
        assert_eq!(TtyMode::parse("auto"), Some(TtyMode::Auto));
        assert_eq!(TtyMode::parse("ON"), Some(TtyMode::On));
        assert_eq!(TtyMode::parse(" off "), Some(TtyMode::Off));
        assert_eq!(TtyMode::parse("yes"), None);
        assert_eq!(TtyMode::Auto.next(), TtyMode::On);
        assert_eq!(TtyMode::On.prev(), TtyMode::Auto);
        assert_eq!(TtyMode::options(), vec!["auto", "on", "off"]);
    }

    #[test]
    fn test_downgrade_truecolor_is_identity() {
        let rgb = Color::Rgb(125, 207, 255);
        assert_eq!(downgrade_color(rgb, ColorLevel::TrueColor), rgb);
        assert_eq!(
            downgrade_color(Color::Indexed(117), ColorLevel::TrueColor),
            Color::Indexed(117)
        );
        assert_eq!(
            downgrade_color(Color::Red, ColorLevel::TrueColor),
            Color::Red
        );
    }

    #[test]
    fn test_downgrade_ansi256_maps_rgb_to_indexed() {
        // Pure xterm primaries land on exact cube entries.
        assert_eq!(
            downgrade_color(Color::Rgb(255, 0, 0), ColorLevel::Ansi256),
            Color::Indexed(196)
        );
        // TokyoNight accent: cube (2, 4, 5) -> 16 + 72 + 24 + 5.
        assert_eq!(
            downgrade_color(Color::Rgb(125, 207, 255), ColorLevel::Ansi256),
            Color::Indexed(117)
        );
        // Equal components take the gray ramp; pure black takes entry 16.
        assert_eq!(
            downgrade_color(Color::Rgb(128, 128, 128), ColorLevel::Ansi256),
            Color::Indexed(244)
        );
        assert_eq!(
            downgrade_color(Color::Rgb(0, 0, 0), ColorLevel::Ansi256),
            Color::Indexed(16)
        );
        // Named colors and Reset pass through.
        assert_eq!(downgrade_color(Color::Red, ColorLevel::Ansi256), Color::Red);
        assert_eq!(
            downgrade_color(Color::Indexed(7), ColorLevel::Ansi256),
            Color::Indexed(7)
        );
        assert_eq!(
            downgrade_color(Color::Reset, ColorLevel::Ansi256),
            Color::Reset
        );
    }

    #[test]
    fn test_downgrade_ansi16_maps_to_named_colors() {
        assert_eq!(
            downgrade_color(Color::Rgb(0, 0, 0), ColorLevel::Ansi16),
            Color::Black
        );
        assert_eq!(
            downgrade_color(Color::Rgb(255, 255, 255), ColorLevel::Ansi16),
            Color::White
        );
        assert_eq!(
            downgrade_color(Color::Rgb(255, 0, 0), ColorLevel::Ansi16),
            Color::Red
        );
        assert_eq!(
            downgrade_color(Color::Rgb(0, 255, 0), ColorLevel::Ansi16),
            Color::Green
        );
        // Indexed entries rejoin the search through their RGB value.
        assert_eq!(
            downgrade_color(Color::Indexed(196), ColorLevel::Ansi16),
            Color::Red
        );
        // Named colors and Reset pass through.
        assert_eq!(
            downgrade_color(Color::LightCyan, ColorLevel::Ansi16),
            Color::LightCyan
        );
        assert_eq!(
            downgrade_color(Color::Reset, ColorLevel::Ansi16),
            Color::Reset
        );
    }

    #[test]
    fn test_downgrade_none_resets_everything() {
        assert_eq!(
            downgrade_color(Color::Rgb(125, 207, 255), ColorLevel::None),
            Color::Reset
        );
        assert_eq!(downgrade_color(Color::Red, ColorLevel::None), Color::Reset);
        assert_eq!(
            downgrade_color(Color::Indexed(117), ColorLevel::None),
            Color::Reset
        );
        assert_eq!(
            downgrade_color(Color::Reset, ColorLevel::None),
            Color::Reset
        );
    }
}
