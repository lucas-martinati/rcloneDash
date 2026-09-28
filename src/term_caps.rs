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

    /// Non-terminal pipe: no colors, no animation. Glyphs stay Unicode: a
    /// pipe preserves UTF-8 bytes, the ASCII fallback is only for
    /// charset-limited terminals (Linux console, non-UTF-8 locale), never
    /// for pipes.
    pub fn plain() -> Self {
        Self {
            color: ColorLevel::None,
            live: false,
            ascii: false,
        }
    }

    /// Pure detection from an env getter and an `is_terminal` flag.
    ///
    /// - `live` is exactly `is_terminal` (except on `TERM=dumb`, which
    ///   cannot animate either).
    /// - `color`: piped stdout (not a terminal) has no color capability ->
    ///   `None`; `TERM=dumb` likewise; `NO_COLOR` present and non-empty -> `None`; `TERM=linux`
    ///   -> `Ansi16`; `COLORTERM` containing `truecolor`/`24bit` ->
    ///   `TrueColor`; otherwise `Ansi256`. (An explicit `TtyMode::Off` can
    ///   still force colors, like `ls --color=always`.)
    /// - `ascii`: `TERM=linux`, or the locale (`LC_ALL`, then `LC_CTYPE`,
    ///   then `LANG`) is set to a non-UTF-8 value.
    pub fn detect_from(env: &impl Fn(&str) -> Option<String>, is_terminal: bool) -> Self {
        let term = env("TERM").unwrap_or_default();
        let is_linux_console = term.trim().eq_ignore_ascii_case("linux");
        // A dumb terminal can do neither colors nor in-place animation.
        let dumb = term.trim().eq_ignore_ascii_case("dumb");

        // `NO_COLOR` counts when present and non-empty; an empty value
        // leaves colors alone.
        let no_color = env("NO_COLOR").is_some_and(|v| !v.is_empty());
        let color = if !is_terminal || dumb || no_color {
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
            live: is_terminal && !dumb,
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
    /// - `On`: caps colors at `Ansi16` + ASCII (stays in color, like btop++),
    ///   but never re-enables colors a pipe or `NO_COLOR` took away.
    /// - `Off`: lifts colors to `TrueColor` + Unicode wherever the terminal
    ///   allows any color at all; stays at `None` on pipes or `NO_COLOR`.
    ///   Only an explicit CLI flag forces colors there (see `apply_tty_override`
    ///   / `cli_style`, like `ls --color=always`). Animation (`live`)
    ///   always follows detection.
    pub fn resolve_from(
        mode: TtyMode,
        env: &impl Fn(&str) -> Option<String>,
        is_terminal: bool,
    ) -> Self {
        let detected = Self::detect_from(env, is_terminal);
        match mode {
            TtyMode::Auto => detected,
            TtyMode::On => Self {
                color: detected.color.min(ColorLevel::Ansi16),
                live: detected.live,
                ascii: true,
            },
            TtyMode::Off => Self {
                color: if detected.color == ColorLevel::None {
                    ColorLevel::None
                } else {
                    ColorLevel::TrueColor
                },
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

/// Standard xterm color-cube channel levels, shared by both conversion
/// directions so round-trips stay consistent.
const CUBE_LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];

/// Maps an `Rgb` triplet to the nearest xterm 256-color palette index:
/// color cube entries 16-231, gray ramp 232-255 for equal components
/// (pure black → 16, near-white → 231).
fn rgb_to_ansi256_index(r: u8, g: u8, b: u8) -> u8 {
    if r == g && g == b {
        // Gray ramp covers 8-238; past its midpoint with white (246) the
        // cube's white end (231) is closer. Capped so the index can never
        // overflow past 255 (248 used to compute 232 + 24).
        if r < 8 {
            16
        } else if r > 246 {
            231
        } else {
            232 + (r - 8) / 10
        }
    } else {
        16 + 36 * quantize_channel(r) + 6 * quantize_channel(g) + quantize_channel(b)
    }
}

/// Quantizes one channel to a color-cube level 0-5 using the real xterm
/// levels (0, 95, 135, 175, 215, 255), not uniform steps: e.g. 95 lands on
/// level 1, not 2.
fn quantize_channel(v: u8) -> u8 {
    let mut best = 0;
    let mut best_dist = u16::MAX;
    for (i, l) in CUBE_LEVELS.iter().enumerate() {
        let dist = (v as i16 - *l as i16).unsigned_abs();
        if dist < best_dist {
            best_dist = dist;
            best = i as u8;
        }
    }
    best
}

/// Recovers the RGB triplet behind an xterm 256-color index (standard cube
/// levels and gray ramp), so `Indexed` colors can join the ANSI16 search.
fn ansi256_index_to_rgb(idx: u8) -> (u8, u8, u8) {
    match idx {
        0..=15 => ANSI16_TABLE[idx as usize].1,
        16..=231 => {
            let i = idx - 16;
            (
                CUBE_LEVELS[(i / 36) as usize],
                CUBE_LEVELS[((i % 36) / 6) as usize],
                CUBE_LEVELS[(i % 6) as usize],
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

/// Background style for a selected row or banner. The color is kept as-is
/// (the frame-wide buffer pass reduces it to the terminal palette), except
/// in `Ansi16` where the maroon banner would degrade to a muddy dark gray:
/// like btop in TTY mode, selections use plain `Red` there. With colors off
/// (`NO_COLOR`), `REVERSED` keeps the selection distinguishable.
pub fn selected_style(bg: Color, level: ColorLevel) -> ratatui::style::Style {
    use ratatui::style::Modifier;
    let bg = match level {
        ColorLevel::Ansi16 => Color::Red,
        _ => bg,
    };
    let mut style = ratatui::style::Style::default().bg(bg);
    if level == ColorLevel::None {
        style = style.add_modifier(Modifier::REVERSED);
    }
    style
}

/// Centralized glyph set: every Unicode character drawn by the updater or
/// the dashboard goes through here, so TTY mode can swap in pure-ASCII
/// fallbacks without touching call sites. The Unicode variants are exactly
/// the characters historically used, hence no visual change by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Glyphs {
    pub bar_fill: &'static str,
    pub bar_empty: &'static str,
    pub diamond: &'static str,
    pub bullet_active: &'static str,
    pub bullet_idle: &'static str,
    pub check: &'static str,
    pub cross: &'static str,
    pub spark: &'static str,
    pub arrow_left: &'static str,
    pub arrow_right: &'static str,
    pub arrow_up: &'static str,
    pub arrow_down: &'static str,
    pub enter: &'static str,
    pub vline: &'static str,
    pub hline: &'static str,
    pub corner_tl: &'static str,
    pub corner_bl: &'static str,
    pub tee_top: &'static str,
    pub tee_bottom: &'static str,
    pub tee_left: &'static str,
    pub tee_right: &'static str,
    /// Stepper "done" tick (distinct from [`Glyphs::check`]).
    pub done: &'static str,
    /// Alternate cross used by history status badges.
    pub fail: &'static str,
    /// "Skipped" badge marker.
    pub skip: &'static str,
    /// Inline-edit row marker.
    pub edit: &'static str,
    /// Filled status dot.
    pub dot: &'static str,
    /// Hollow status dot (pending step).
    pub dot_open: &'static str,
    /// CLI one-shot decorations (kept identical in Unicode mode).
    pub rocket: &'static str,
    pub ok_emoji: &'static str,
    pub fail_emoji: &'static str,
    pub warn: &'static str,
    /// Activity bolt.
    pub bolt: &'static str,
    /// In-progress hourglass (centered badge use only, width differs).
    pub hourglass: &'static str,
    /// Plain warning sign (distinct from [`Glyphs::warn`]).
    pub warn_plain: &'static str,
    /// Bidirectional swap arrow.
    pub swap: &'static str,
    /// Refresh banner icon.
    pub refresh: &'static str,
    /// Space-key cap label.
    pub space: &'static str,
    /// Truncation marker.
    pub ellipsis: &'static str,
}

impl Glyphs {
    pub fn new(ascii: bool) -> Self {
        if ascii {
            Self::ascii()
        } else {
            Self::unicode()
        }
    }

    pub fn unicode() -> Self {
        Self {
            bar_fill: "█",
            bar_empty: "░",
            diamond: "◇",
            bullet_active: "▶",
            bullet_idle: "•",
            check: "✔",
            cross: "✖",
            spark: "✨",
            arrow_left: "←",
            arrow_right: "→",
            arrow_up: "↑",
            arrow_down: "↓",
            enter: "↵",
            vline: "│",
            hline: "─",
            corner_tl: "┌",
            corner_bl: "└",
            tee_top: "┬",
            tee_bottom: "┴",
            tee_left: "├",
            tee_right: "┤",
            done: "✓",
            fail: "✗",
            skip: "⊘",
            edit: "✎",
            dot: "●",
            dot_open: "○",
            rocket: "🚀",
            ok_emoji: "✅",
            fail_emoji: "❌",
            warn: "⚠️",
            bolt: "⚡",
            hourglass: "⏳",
            warn_plain: "⚠",
            swap: "⇄",
            refresh: "⟳",
            space: "␣",
            ellipsis: "…",
        }
    }

    /// Pure-ASCII fallbacks (`is_ascii()` holds for every field). Display
    /// widths match the Unicode variants one-for-one, except `spark`: `✨`
    /// is a double-width emoji only ever used at end of line, where
    /// alignment does not matter.
    pub fn ascii() -> Self {
        Self {
            bar_fill: "#",
            bar_empty: "-",
            diamond: "o",
            bullet_active: ">",
            bullet_idle: "*",
            check: "v",
            cross: "x",
            spark: "*",
            arrow_left: "<",
            arrow_right: ">",
            arrow_up: "^",
            arrow_down: "v",
            enter: ">",
            vline: "|",
            hline: "-",
            corner_tl: "+",
            corner_bl: "+",
            tee_top: "+",
            tee_bottom: "+",
            tee_left: "+",
            tee_right: "+",
            done: "v",
            fail: "x",
            skip: "-",
            edit: ">",
            dot: "o",
            dot_open: "-",
            rocket: "*",
            ok_emoji: "v",
            fail_emoji: "x",
            warn: "!",
            bolt: "*",
            hourglass: "...",
            warn_plain: "!",
            swap: "<>",
            refresh: "@",
            space: "Space",
            ellipsis: ".",
        }
    }

    /// Tab index digit (`¹`/`²` in the settings tab bar).
    pub fn tab_digit(ascii: bool, n: usize) -> &'static str {
        if ascii {
            match n {
                1 => "1",
                2 => "2",
                3 => "3",
                4 => "4",
                5 => "5",
                6 => "6",
                7 => "7",
                8 => "8",
                _ => "9",
            }
        } else {
            match n {
                1 => "¹",
                2 => "²",
                3 => "³",
                4 => "⁴",
                5 => "⁵",
                6 => "⁶",
                7 => "⁷",
                8 => "⁸",
                _ => "⁹",
            }
        }
    }

    /// All drawable fields, for exhaustive tests.
    pub fn all_fields(&self) -> [&'static str; 38] {
        [
            self.bar_fill,
            self.bar_empty,
            self.diamond,
            self.bullet_active,
            self.bullet_idle,
            self.check,
            self.cross,
            self.spark,
            self.arrow_left,
            self.arrow_right,
            self.arrow_up,
            self.arrow_down,
            self.enter,
            self.vline,
            self.hline,
            self.corner_tl,
            self.corner_bl,
            self.tee_top,
            self.tee_bottom,
            self.tee_left,
            self.tee_right,
            self.done,
            self.fail,
            self.skip,
            self.edit,
            self.dot,
            self.dot_open,
            self.rocket,
            self.ok_emoji,
            self.fail_emoji,
            self.warn,
            self.bolt,
            self.hourglass,
            self.warn_plain,
            self.swap,
            self.refresh,
            self.space,
            self.ellipsis,
        ]
    }
}

impl TermCaps {
    /// Glyph set matching these capabilities (ASCII on Linux console /
    /// non-UTF-8 locale).
    pub fn glyphs(&self) -> Glyphs {
        Glyphs::new(self.ascii)
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
    fn test_empty_no_color_keeps_colors() {
        // An empty NO_COLOR does not count: colors follow the terminal.
        let env = env_of(&[("TERM", "xterm-256color"), ("NO_COLOR", "")]);
        let caps = TermCaps::detect_from(&env, true);
        assert_eq!(caps.color, ColorLevel::Ansi256);
        assert!(caps.live);
    }

    #[test]
    fn test_dumb_terminal_has_neither_color_nor_animation() {
        let env = env_of(&[("TERM", "dumb")]);
        let caps = TermCaps::detect_from(&env, true);
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
    fn test_tty_mode_on_never_reenables_colors() {
        // `on` caps at Ansi16: a pipe or NO_COLOR keeps color at None, so
        // `cmd --tty-mode=on | tee log` never leaks ANSI into the log.
        let env = env_of(&[("TERM", "xterm-256color"), ("COLORTERM", "truecolor")]);
        let caps = TermCaps::resolve_from(TtyMode::On, &env, false);
        assert_eq!(caps.color, ColorLevel::None);
        assert!(!caps.live);
        assert!(caps.ascii);

        let env = env_of(&[("TERM", "xterm-256color"), ("NO_COLOR", "1")]);
        let caps = TermCaps::resolve_from(TtyMode::On, &env, true);
        assert_eq!(caps.color, ColorLevel::None);
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
    fn test_tty_mode_off_stays_plain_without_capability() {
        // Polite Off: no colors on pipes or NO_COLOR (only an explicit CLI
        // flag forces them there).
        let env = env_of(&[("TERM", "xterm-256color"), ("COLORTERM", "truecolor")]);
        let caps = TermCaps::resolve_from(TtyMode::Off, &env, false);
        assert_eq!(caps.color, ColorLevel::None);
        assert!(!caps.live);

        let env = env_of(&[("TERM", "xterm-256color"), ("NO_COLOR", "1")]);
        let caps = TermCaps::resolve_from(TtyMode::Off, &env, true);
        assert_eq!(caps.color, ColorLevel::None);
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
        // Channels snap to the real xterm levels: 95 is exactly level 1.
        assert_eq!(
            downgrade_color(Color::Rgb(95, 0, 0), ColorLevel::Ansi256),
            Color::Indexed(52)
        );
    }
    #[test]
    fn test_gray_ramp_never_overflows() {
        // Every gray must land on a valid palette index (16-231 cube ends,
        // 232-255 ramp). Guards the former 232 + 24 overflow at gray 248.
        for v in 0..=255u8 {
            let idx = rgb_to_ansi256_index(v, v, v);
            assert!((16..=255).contains(&idx), "gray {} -> index {}", v, idx);
        }
        assert_eq!(rgb_to_ansi256_index(0, 0, 0), 16);
        assert_eq!(rgb_to_ansi256_index(7, 7, 7), 16);
        assert_eq!(rgb_to_ansi256_index(8, 8, 8), 232);
        assert_eq!(rgb_to_ansi256_index(246, 246, 246), 255);
        assert_eq!(rgb_to_ansi256_index(247, 247, 247), 231);
        assert_eq!(rgb_to_ansi256_index(248, 248, 248), 231);
        assert_eq!(rgb_to_ansi256_index(255, 255, 255), 231);
    }

    #[test]
    fn test_selected_style_reversed_without_colors() {
        use ratatui::style::Modifier;
        // NO_COLOR: raw background plus REVERSED so the selection stays
        // distinguishable (the frame-wide buffer pass resets the color).
        let s = selected_style(Color::Rgb(95, 30, 30), ColorLevel::None);
        assert_eq!(s.bg, Some(Color::Rgb(95, 30, 30)));
        assert!(s.add_modifier.contains(Modifier::REVERSED));
        // Ansi16 forces plain Red (btop TTY style), no extra modifier.
        let s = selected_style(Color::Rgb(95, 30, 30), ColorLevel::Ansi16);
        assert_eq!(s.bg, Some(Color::Red));
        assert!(!s.add_modifier.contains(Modifier::REVERSED));
        // Otherwise the banner color passes through untouched.
        let s = selected_style(Color::Rgb(95, 30, 30), ColorLevel::TrueColor);
        assert_eq!(s.bg, Some(Color::Rgb(95, 30, 30)));
        assert!(!s.add_modifier.contains(Modifier::REVERSED));
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

    #[test]
    fn test_glyphs_unicode_keeps_historical_characters() {
        // No visual change by default: the Unicode set must be exactly the
        // characters the UI has always drawn.
        let g = Glyphs::unicode();
        assert_eq!(g.bar_fill, "█");
        assert_eq!(g.bar_empty, "░");
        assert_eq!(g.diamond, "◇");
        assert_eq!(g.bullet_active, "▶");
        assert_eq!(g.bullet_idle, "•");
        assert_eq!(g.check, "✔");
        assert_eq!(g.cross, "✖");
        assert_eq!(g.spark, "✨");
        assert_eq!(g.arrow_left, "←");
        assert_eq!(g.arrow_right, "→");
        assert_eq!(g.arrow_up, "↑");
        assert_eq!(g.arrow_down, "↓");
        assert_eq!(g.enter, "↵");
        assert_eq!(g.vline, "│");
        assert_eq!(g.hline, "─");
        assert_eq!(g.corner_tl, "┌");
        assert_eq!(g.corner_bl, "└");
        assert_eq!(g.done, "✓");
        assert_eq!(g.fail, "✗");
        assert_eq!(g.skip, "⊘");
        assert_eq!(g.edit, "✎");
        assert_eq!(g.dot, "●");
        assert_eq!(g.dot_open, "○");
        assert_eq!(g.rocket, "🚀");
        assert_eq!(g.ok_emoji, "✅");
        assert_eq!(g.fail_emoji, "❌");
        assert_eq!(g.warn, "⚠️");
        assert_eq!(g.bolt, "⚡");
        assert_eq!(g.hourglass, "⏳");
        assert_eq!(g.warn_plain, "⚠");
        assert_eq!(g.swap, "⇄");
        assert_eq!(g.refresh, "⟳");
        assert_eq!(g.space, "␣");
        assert_eq!(g.ellipsis, "…");
    }

    #[test]
    fn test_glyphs_ascii_is_pure_ascii() {
        let g = Glyphs::ascii();
        for field in g.all_fields() {
            assert!(!field.is_empty(), "ASCII glyph must not be empty");
            assert!(
                field.is_ascii(),
                "ASCII fallback must be pure ASCII, got {:?}",
                field
            );
        }
        assert_eq!(Glyphs::tab_digit(true, 1), "1");
        assert_eq!(Glyphs::tab_digit(true, 2), "2");
        assert!(Glyphs::tab_digit(true, 2).is_ascii());
    }

    #[test]
    fn test_glyphs_ascii_keeps_display_width() {
        use unicode_width::UnicodeWidthStr;
        let uni = Glyphs::unicode();
        let asc = Glyphs::ascii();
        // Every ASCII fallback is width 1, so alignment-critical pairs keep
        // their width one-for-one. Double-width Unicode decorations (✨, 🚀,
        // ⚠️) only ever end a line, where alignment does not matter; the
        // remaining exceptions only appear in flowing text or centered
        // badges: hourglass ("..."), swap ("<>") and the Space key ("Space").
        const WIDTH_FLEXIBLE: [(&str, &str); 3] = [("⏳", "..."), ("⇄", "<>"), ("␣", "Space")];
        for (u, a) in uni.all_fields().iter().zip(asc.all_fields().iter()) {
            if let Some((_, expected)) = WIDTH_FLEXIBLE.iter().find(|(wu, _)| wu == u) {
                assert_eq!(a, expected);
                continue;
            }
            assert_eq!(
                a.width(),
                1,
                "ASCII fallback {:?} must be width 1 (was {:?})",
                a,
                u
            );
        }
        assert_eq!(Glyphs::tab_digit(false, 1).width(), 1);
        assert_eq!(Glyphs::tab_digit(true, 1).width(), 1);
    }

    #[test]
    fn test_term_caps_resolve_ascii_glyphs() {
        let env = env_of(&[("TERM", "linux")]);
        let caps = TermCaps::detect_from(&env, true);
        assert!(caps.ascii);
        assert_eq!(caps.glyphs(), Glyphs::ascii());
        let env = env_of(&[("TERM", "xterm-256color"), ("LANG", "en_US.UTF-8")]);
        let caps = TermCaps::detect_from(&env, true);
        assert_eq!(caps.glyphs(), Glyphs::unicode());
    }
}
