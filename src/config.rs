use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::ui::theme::ThemeChoice;

/// Version centralisée de l'application rcloneDash (synchronisée depuis Cargo.toml)
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Formateur unifié pour afficher la liste des options avec indication claire de la valeur active.
/// Prise en charge automatique d'une grille à 2 colonnes alignée pour les listes étendues (> 8 options).
pub fn format_setting_options_list<T: AsRef<str>>(choices: &[T], current: &str) -> String {
    if choices.is_empty() {
        return String::new();
    }

    let is_item_active = |c: &str| -> bool {
        let norm_c = c.trim().replace(' ', "");
        let norm_cur = current.trim().replace(' ', "");
        norm_c.eq_ignore_ascii_case(&norm_cur)
            || (c == "Disabled" && (current.is_empty() || current.eq_ignore_ascii_case("Disabled")))
    };

    if choices.len() <= 8 {
        choices
            .iter()
            .map(|c| {
                let s = c.as_ref();
                if is_item_active(s) {
                    format!("  ▶ {} (active)", s)
                } else {
                    format!("  • {}", s)
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        let num_cols = if choices.len() > 14 { 3 } else { 2 };
        let num_rows = (choices.len() + num_cols - 1) / num_cols;
        let mut lines = Vec::with_capacity(num_rows);

        let formatted: Vec<(String, bool)> = choices
            .iter()
            .map(|c| {
                let s = c.as_ref();
                let active = is_item_active(s);
                (s.to_string(), active)
            })
            .collect();

        // Calculer la largeur de chaque colonne pour un alignement parfait
        let mut col_widths = vec![0usize; num_cols];
        for col in 0..num_cols {
            for r in 0..num_rows {
                let idx = r + col * num_rows;
                if idx < formatted.len() {
                    let (s, active) = &formatted[idx];
                    let len = if col == 0 {
                        if *active { format!("  ▶ {} (active)", s).chars().count() } else { format!("  • {}", s).chars().count() }
                    } else {
                        if *active { format!("▶ {} (active)", s).chars().count() } else { format!("• {}", s).chars().count() }
                    };
                    col_widths[col] = col_widths[col].max(len);
                }
            }
            col_widths[col] += 2; // Marge de séparation inter-colonnes
        }

        for r in 0..num_rows {
            let mut line = String::new();
            for col in 0..num_cols {
                let idx = r + col * num_rows;
                if idx < formatted.len() {
                    let (s, active) = &formatted[idx];
                    let item = if col == 0 {
                        if *active { format!("  ▶ {} (active)", s) } else { format!("  • {}", s) }
                    } else {
                        if *active { format!("▶ {} (active)", s) } else { format!("• {}", s) }
                    };

                    if col + 1 < num_cols && (r + (col + 1) * num_rows) < formatted.len() {
                        let pad = col_widths[col].saturating_sub(item.chars().count());
                        line.push_str(&format!("{}{:pad$}", item, "", pad = pad));
                    } else {
                        line.push_str(&item);
                    }
                }
            }
            lines.push(line);
        }

        lines.join("\n")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ContainerLayout {
    #[default]
    Default,
    RecentFirst,
    LogsTop,
    Inverted,
}

impl ContainerLayout {
    pub fn all() -> &'static [ContainerLayout] {
        &[
            ContainerLayout::Default,
            ContainerLayout::RecentFirst,
            ContainerLayout::LogsTop,
            ContainerLayout::Inverted,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            ContainerLayout::Default => "Standard (Storage → Mid → Recent)",
            ContainerLayout::RecentFirst => "Recent First (Storage → Recent → Mid)",
            ContainerLayout::LogsTop => "Logs / Hist Top (Mid → Storage → Recent)",
            ContainerLayout::Inverted => "Inverted (Recent → Mid → Storage)",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            ContainerLayout::Default => ContainerLayout::RecentFirst,
            ContainerLayout::RecentFirst => ContainerLayout::LogsTop,
            ContainerLayout::LogsTop => ContainerLayout::Inverted,
            ContainerLayout::Inverted => ContainerLayout::Default,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            ContainerLayout::Default => ContainerLayout::Inverted,
            ContainerLayout::RecentFirst => ContainerLayout::Default,
            ContainerLayout::LogsTop => ContainerLayout::RecentFirst,
            ContainerLayout::Inverted => ContainerLayout::LogsTop,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MidPanelOrder {
    #[default]
    HistoryLogs,
    LogsHistory,
}

impl MidPanelOrder {
    pub fn all() -> &'static [MidPanelOrder] {
        &[MidPanelOrder::HistoryLogs, MidPanelOrder::LogsHistory]
    }

    pub fn name(&self) -> &'static str {
        match self {
            MidPanelOrder::HistoryLogs => "History │ Logs",
            MidPanelOrder::LogsHistory => "Logs │ History",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            MidPanelOrder::HistoryLogs => MidPanelOrder::LogsHistory,
            MidPanelOrder::LogsHistory => MidPanelOrder::HistoryLogs,
        }
    }

    pub fn prev(&self) -> Self {
        self.next()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BorderStyleChoice {
    #[default]
    Rounded,
    Sharp,
    Double,
    Thick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BorderGlyphs {
    pub top_left: &'static str,
    pub top_right: &'static str,
    pub bot_left: &'static str,
    pub bot_right: &'static str,
    pub horizontal: &'static str,
}

impl BorderStyleChoice {
    pub fn all() -> &'static [BorderStyleChoice] {
        &[
            BorderStyleChoice::Rounded,
            BorderStyleChoice::Sharp,
            BorderStyleChoice::Double,
            BorderStyleChoice::Thick,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            BorderStyleChoice::Rounded => "Rounded (Curves)",
            BorderStyleChoice::Sharp => "Sharp (Square)",
            BorderStyleChoice::Double => "Double (Retro)",
            BorderStyleChoice::Thick => "Thick (Bold)",
        }
    }

    pub fn to_border_type(&self) -> ratatui::widgets::BorderType {
        match self {
            BorderStyleChoice::Rounded => ratatui::widgets::BorderType::Rounded,
            BorderStyleChoice::Sharp => ratatui::widgets::BorderType::Plain,
            BorderStyleChoice::Double => ratatui::widgets::BorderType::Double,
            BorderStyleChoice::Thick => ratatui::widgets::BorderType::Thick,
        }
    }

    pub fn glyphs(&self) -> BorderGlyphs {
        match self {
            BorderStyleChoice::Rounded | BorderStyleChoice::Sharp => BorderGlyphs {
                top_left: "┐",
                top_right: "┌",
                bot_left: "┘",
                bot_right: "└",
                horizontal: "─",
            },
            BorderStyleChoice::Double => BorderGlyphs {
                top_left: "╗",
                top_right: "╔",
                bot_left: "╝",
                bot_right: "╚",
                horizontal: "═",
            },
            BorderStyleChoice::Thick => BorderGlyphs {
                top_left: "┓",
                top_right: "┏",
                bot_left: "┛",
                bot_right: "┗",
                horizontal: "━",
            },
        }
    }

    pub fn next(&self) -> Self {
        match self {
            BorderStyleChoice::Rounded => BorderStyleChoice::Sharp,
            BorderStyleChoice::Sharp => BorderStyleChoice::Double,
            BorderStyleChoice::Double => BorderStyleChoice::Thick,
            BorderStyleChoice::Thick => BorderStyleChoice::Rounded,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            BorderStyleChoice::Rounded => BorderStyleChoice::Thick,
            BorderStyleChoice::Sharp => BorderStyleChoice::Rounded,
            BorderStyleChoice::Double => BorderStyleChoice::Sharp,
            BorderStyleChoice::Thick => BorderStyleChoice::Double,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GraphStyleChoice {
    #[default]
    #[serde(alias = "Ascii")]
    Braille,
    Blocks,
}

impl GraphStyleChoice {
    pub fn all() -> &'static [GraphStyleChoice] {
        &[
            GraphStyleChoice::Braille,
            GraphStyleChoice::Blocks,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            GraphStyleChoice::Braille => "Braille (⡀⣀⣄⣤⣦⣶⣷⣿)",
            GraphStyleChoice::Blocks => "Blocks ( ▂▃▄▅▆▇█)",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            GraphStyleChoice::Braille => GraphStyleChoice::Blocks,
            GraphStyleChoice::Blocks => GraphStyleChoice::Braille,
        }
    }

    pub fn prev(&self) -> Self {
        self.next()
    }
}

/// Interval options for bisync timer (single source of truth)
pub const TIMER_INTERVAL_OPTIONS: &[&str] = &["10min", "15min", "30min", "1h", "2h", "4h"];

/// Cloud safety net / full sync intervals (single source of truth)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FullSyncOption {
    pub value: &'static str,
    pub label: &'static str,
}

pub const FULL_SYNC_OPTIONS: &[FullSyncOption] = &[
    FullSyncOption { value: "60", label: "1h (Recommended)" },
    FullSyncOption { value: "120", label: "2h" },
    FullSyncOption { value: "240", label: "4h" },
    FullSyncOption { value: "360", label: "6h" },
    FullSyncOption { value: "720", label: "12h" },
    FullSyncOption { value: "1440", label: "24h (1 day)" },
    FullSyncOption { value: "never", label: "Never (Local)" },
];

pub fn full_sync_label(val: &str) -> &'static str {
    for opt in FULL_SYNC_OPTIONS {
        if opt.value == val {
            return opt.label;
        }
    }
    "Custom"
}

/// Bandwidth limit presets (single source of truth)
pub const BWLIMIT_OPTIONS: &[&str] = &["Disabled", "5M", "10M", "20M", "50M"];

/// UI tick rate steps in milliseconds (single source of truth)
pub const TICK_RATE_STEPS: &[u64] = &[
    100, 200, 250, 300, 400, 500, 600, 700, 800, 900, 1000,
    1500, 2000, 2500, 3000, 4000, 5000, 6000, 7000, 8000, 9000, 10000,
];

/// Liste canonique des options de fréquence de rafraîchissement UI (source unique de vérité)
pub fn tick_rate_options() -> Vec<String> {
    TICK_RATE_STEPS.iter().map(|s| format!("{}ms", s)).collect()
}

/// Registre canonique et centralisé de tous les réglages disponibles dans l'application.
/// Définit pour chaque paramètre ses étiquettes, descriptions et sa liste d'options
/// directement liée aux enums et constantes canoniques (sans duplication).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingId {
    // Onglet 0 : Rclone
    TimerInterval,
    CloudSafetyNet,
    BandwidthLimit,
    LocalDirectory,
    RemoteStorage,
    ResyncAction,
    LogJournalAction,

    // Onglet 1 : UI & Apparence
    ColorTheme,
    ContainerLayout,
    MidPanelOrder,
    BorderStyle,
    GraphStyle,
    TickRate,
}

impl SettingId {
    pub fn from_tab_and_idx(tab: usize, idx: usize) -> Option<Self> {
        match (tab, idx) {
            (0, 0) => Some(Self::TimerInterval),
            (0, 1) => Some(Self::CloudSafetyNet),
            (0, 2) => Some(Self::BandwidthLimit),
            (0, 3) => Some(Self::LocalDirectory),
            (0, 4) => Some(Self::RemoteStorage),
            (0, 5) => Some(Self::ResyncAction),
            (0, 6) => Some(Self::LogJournalAction),
            (1, 0) => Some(Self::ColorTheme),
            (1, 1) => Some(Self::ContainerLayout),
            (1, 2) => Some(Self::MidPanelOrder),
            (1, 3) => Some(Self::BorderStyle),
            (1, 4) => Some(Self::GraphStyle),
            (1, 5) => Some(Self::TickRate),
            _ => None,
        }
    }

    pub fn tab_count(tab: usize) -> usize {
        if tab == 0 { 7 } else { 6 }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::TimerInterval => "Bisync timer interval",
            Self::CloudSafetyNet => "Cloud safety net",
            Self::BandwidthLimit => "Bandwidth limit",
            Self::LocalDirectory => "Local directory",
            Self::RemoteStorage => "Remote storage",
            Self::ResyncAction => "Full resynchronization",
            Self::LogJournalAction => "Full rclone log journal",
            Self::ColorTheme => "Color theme",
            Self::ContainerLayout => "Container layout",
            Self::MidPanelOrder => "Mid-panel order",
            Self::BorderStyle => "Border style",
            Self::GraphStyle => "Graph style",
            Self::TickRate => "UI Loop frequency",
        }
    }

    pub fn desc_title(&self) -> &'static str {
        match self {
            Self::TimerInterval => "Bisync timer interval.",
            Self::CloudSafetyNet => "Cloud safety net.",
            Self::BandwidthLimit => "Bandwidth limit (bwlimit).",
            Self::LocalDirectory => "Monitored local directory.",
            Self::RemoteStorage => "Remote cloud storage.",
            Self::ResyncAction => "Full resynchronization (--resync).",
            Self::LogJournalAction => "Full rclone log journal (rclone-bisync).",
            Self::ColorTheme => "Color theme.",
            Self::ContainerLayout => "Container layout.",
            Self::MidPanelOrder => "Mid-panel order.",
            Self::BorderStyle => "Border style.",
            Self::GraphStyle => "Graph style.",
            Self::TickRate => "UI Loop frequency (tick rate).",
        }
    }

    pub fn desc_intro(&self) -> &'static str {
        match self {
            Self::TimerInterval => "Frequency of automatic checks and synchronization managed by systemd.\n\nConfigures how often rclone-bisync.timer wakes up to inspect changes.\nRecommended: 15min for an ideal balance between responsiveness and CPU usage.",
            Self::CloudSafetyNet => "Maximum time elapsed before running a full bidirectional sync.\n\nEnsures files created or updated remotely from another computer or the web interface are retrieved, even if no local changes were detected.\n'Never' triggers sync only upon local changes.",
            Self::BandwidthLimit => "Maximum allowed transfer speed for rclone.\n\nPreserves your internet connection by limiting network bandwidth used by rclone.\nStored in bwlimit.env and injected into the systemd service.\nValue 'Disabled' uses 100% of available bandwidth.",
            Self::LocalDirectory => "Path to the root local directory synchronized with cloud storage.\n\nContains your local data replicated by bisync.",
            Self::RemoteStorage => "Remote storage name configured in ~/.config/rclone/rclone.conf.\n\nUsed for cloud quota inquiries, remote listings, and bidirectional synchronization.",
            Self::ResyncAction => "In case of critical bisync errors or corrupted sync listings, this action rebuilds listing databases by comparing the local directory and Google Drive (keeping the newest files: --resync-mode newer).",
            Self::LogJournalAction => "Opens the complete rclone-bisync systemd journal log in your external viewer (less or configured editor).\n\nAllows navigating the full history, searching text, and inspecting detailed file transfers.",
            Self::ColorTheme => "Sets the color theme applied across the entire dashboard.\n\nEach theme dynamically adapts borders, text, and btop++ gradient charts.",
            Self::ContainerLayout => "Reorder the main dashboard containers to match your preferred workflow.\n\nApplies instantly across the entire dashboard.",
            Self::MidPanelOrder => "Horizontal placement of the middle section containers.\n\nAll keyboard shortcuts and mouse interactions adapt automatically.",
            Self::BorderStyle => "Customize the box-drawing character style for all cards, panels, and modal dialogs.\n\nPersisted across sessions in dash-config.json.",
            Self::GraphStyle => "Select the glyph set used to render multiline activity and speed sparkline charts.\n\nPersisted across sessions in dash-config.json.",
            Self::TickRate => "Refresh rate of the TUI display engine in milliseconds.\n\nControls smoothness of log scrolling, metrics calculation, and micro-animations.",
        }
    }

    /// Source de vérité unique pour les choix des paramètres : renvoie exactement la même liste
    /// utilisée pour le défilement et la sélection.
    pub fn choices(&self) -> Option<Vec<String>> {
        match self {
            Self::TimerInterval => Some(TIMER_INTERVAL_OPTIONS.iter().map(|s| s.to_string()).collect()),
            Self::CloudSafetyNet => Some(FULL_SYNC_OPTIONS.iter().map(|o| o.label.to_string()).collect()),
            Self::BandwidthLimit => Some(BWLIMIT_OPTIONS.iter().map(|s| s.to_string()).collect()),
            Self::ColorTheme => Some(ThemeChoice::all().iter().map(|t| t.name().to_string()).collect()),
            Self::ContainerLayout => Some(ContainerLayout::all().iter().map(|l| l.name().to_string()).collect()),
            Self::MidPanelOrder => Some(MidPanelOrder::all().iter().map(|m| m.name().to_string()).collect()),
            Self::BorderStyle => Some(BorderStyleChoice::all().iter().map(|b| b.name().to_string()).collect()),
            Self::GraphStyle => Some(GraphStyleChoice::all().iter().map(|g| g.name().to_string()).collect()),
            Self::TickRate => Some(tick_rate_options()),
            _ => None,
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub remote: String,
    pub local_dir: String,
    pub timer_interval: String,
    pub full_sync_interval: String,
    pub bwlimit: Option<String>,
    pub theme: Option<ThemeChoice>,
    pub tick_rate_ms: Option<u64>,
    #[serde(default)]
    pub container_layout: ContainerLayout,
    #[serde(default)]
    pub mid_panel_order: MidPanelOrder,
    #[serde(default)]
    pub border_style: BorderStyleChoice,
    #[serde(default)]
    pub graph_style: GraphStyleChoice,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            remote: "GoogleDrive:".to_string(),
            local_dir: "~/GoogleDrive".to_string(),
            timer_interval: "10min".to_string(),
            full_sync_interval: "60".to_string(),
            bwlimit: None,
            theme: Some(ThemeChoice::TokyoNight),
            tick_rate_ms: Some(250),
            container_layout: ContainerLayout::Default,
            mid_panel_order: MidPanelOrder::HistoryLogs,
            border_style: BorderStyleChoice::Rounded,
            graph_style: GraphStyleChoice::Braille,
        }
    }
}

pub fn expand_tilde<P: AsRef<Path>>(path: P) -> PathBuf {
    let p = path.as_ref();
    if p.starts_with("~") {
        if let Some(home) = dirs_home() {
            if p == Path::new("~") {
                return home;
            }
            if let Ok(stripped) = p.strip_prefix("~/") {
                return home.join(stripped);
            }
        }
    }
    p.to_path_buf()
}

pub fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

pub fn config_dir() -> PathBuf {
    if let Ok(custom) = std::env::var("RCLONEDASH_CONFIG_DIR") {
        return PathBuf::from(custom);
    }
    #[cfg(test)]
    {
        return std::env::temp_dir().join("rclonedash_test_config");
    }
    #[cfg(not(test))]
    expand_tilde("~/.config/rclone")
}

pub fn config_file() -> PathBuf {
    config_dir().join("dash-config.json")
}

pub fn filters_file() -> PathBuf {
    config_dir().join("gdrive-filters.txt")
}

pub fn bwlimit_file() -> PathBuf {
    config_dir().join("bwlimit.env")
}

pub fn force_sync_marker() -> PathBuf {
    config_dir().join(".force-sync")
}

pub fn resync_marker() -> PathBuf {
    config_dir().join(".resync")
}

pub fn last_full_sync_marker() -> PathBuf {
    config_dir().join(".last-full-sync")
}

pub fn load_config() -> AppConfig {
    let path = config_file();
    let mut cfg = AppConfig::default();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(loaded) = serde_json::from_str::<AppConfig>(&content) {
                cfg = loaded;
            }
        }
    }
    if let Some(bw) = read_bwlimit() {
        cfg.bwlimit = Some(bw);
    }
    cfg
}

pub fn save_config(cfg: &AppConfig) -> Result<(), String> {
    let path = config_file();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let json = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())?;

    if let Some(bw) = &cfg.bwlimit {
        save_bwlimit(bw)?;
    } else {
        let _ = save_bwlimit("");
    }

    #[cfg(not(test))]
    {
        update_systemd_timer_interval(&cfg.timer_interval);
    }

    Ok(())
}

#[cfg(not(test))]
pub fn update_systemd_timer_interval(interval: &str) {
    if let Some(home) = dirs_home() {
        let timer_path = home.join(".config/systemd/user/rclone-bisync.timer");
        if timer_path.exists() {
            if let Ok(content) = fs::read_to_string(&timer_path) {
                let re = regex::Regex::new(r"(?m)^(OnUnitActiveSec|OnUnitInactiveSec)=.*$").unwrap();
                let updated = re.replace_all(&content, format!("OnUnitInactiveSec={}", interval));
                if updated != content {
                    let _ = fs::write(&timer_path, updated.as_bytes());
                    let _ = std::process::Command::new("systemctl")
                        .args(["--user", "daemon-reload"])
                        .output();
                    let _ = std::process::Command::new("systemctl")
                        .args(["--user", "restart", "rclone-bisync.timer"])
                        .output();
                }
            }
        }
    }
}

pub fn read_bwlimit() -> Option<String> {
    let path = bwlimit_file();
    if path.exists() {
        if let Ok(c) = fs::read_to_string(path) {
            for line in c.lines() {
                if let Some(val) = line.strip_prefix("RCLONE_BWLIMIT=") {
                    return Some(val.trim().to_string());
                }
            }
        }
    }
    None
}

pub fn save_bwlimit(limit: &str) -> Result<(), String> {
    let path = bwlimit_file();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let content = if limit.trim().is_empty() || limit == "Désactivé" {
        "".to_string()
    } else {
        format!("RCLONE_BWLIMIT={}\n", limit.trim())
    };
    fs::write(path, content).map_err(|e| e.to_string())
}

pub fn read_filters() -> Vec<String> {
    let path = filters_file();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            return content.lines().map(|s| s.to_string()).collect();
        }
    }
    Vec::new()
}

pub fn save_filters(filters: &[String]) -> Result<(), String> {
    let path = filters_file();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let content = if filters.is_empty() {
        "".to_string()
    } else {
        filters.join("\n") + "\n"
    };
    fs::write(path, content).map_err(|e| e.to_string())
}

pub fn quota_cache_file() -> PathBuf {
    config_dir().join(".quota-cache.json")
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CloudQuotaCache {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
}

pub fn load_quota_cache() -> Option<CloudQuotaCache> {
    let path = quota_cache_file();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(cached) = serde_json::from_str::<CloudQuotaCache>(&content) {
                return Some(cached);
            }
        }
    }
    None
}

pub fn save_quota_cache(quota: &CloudQuotaCache) -> Result<(), String> {
    let path = quota_cache_file();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let json = serde_json::to_string(quota).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::widgets::BorderType;

    #[test]
    fn test_appearance_enums_cycling() {
        // ContainerLayout
        let l = ContainerLayout::Default;
        assert_eq!(l.next(), ContainerLayout::RecentFirst);
        assert_eq!(l.next().next(), ContainerLayout::LogsTop);
        assert_eq!(l.next().next().next(), ContainerLayout::Inverted);
        assert_eq!(l.next().next().next().next(), ContainerLayout::Default);
        assert_eq!(l.prev(), ContainerLayout::Inverted);

        // MidPanelOrder
        let m = MidPanelOrder::HistoryLogs;
        assert_eq!(m.next(), MidPanelOrder::LogsHistory);
        assert_eq!(m.next().next(), MidPanelOrder::HistoryLogs);
        assert_eq!(m.prev(), MidPanelOrder::LogsHistory);

        // BorderStyleChoice
        let b = BorderStyleChoice::Rounded;
        assert_eq!(b.to_border_type(), BorderType::Rounded);
        assert_eq!(b.next(), BorderStyleChoice::Sharp);
        assert_eq!(b.next().to_border_type(), BorderType::Plain);
        assert_eq!(b.next().next(), BorderStyleChoice::Double);
        assert_eq!(b.next().next().to_border_type(), BorderType::Double);
        assert_eq!(b.next().next().next(), BorderStyleChoice::Thick);
        assert_eq!(b.next().next().next().to_border_type(), BorderType::Thick);
        assert_eq!(b.prev(), BorderStyleChoice::Thick);

        // GraphStyleChoice
        let g = GraphStyleChoice::Braille;
        assert_eq!(g.next(), GraphStyleChoice::Blocks);
        assert_eq!(g.next().next(), GraphStyleChoice::Braille);
        assert_eq!(g.prev(), GraphStyleChoice::Blocks);

        // BorderGlyphs
        assert_eq!(BorderStyleChoice::Double.glyphs().horizontal, "═");
        assert_eq!(BorderStyleChoice::Double.glyphs().top_left, "╗");
        assert_eq!(BorderStyleChoice::Thick.glyphs().horizontal, "━");
        assert_eq!(BorderStyleChoice::Thick.glyphs().top_left, "┓");
    }

    #[test]
    fn test_config_backward_compatibility_deserialization() {
        let legacy_json = r#"{
            "remote": "gdrive:",
            "local_dir": "~/Documents",
            "timer_interval": "15min",
            "full_sync_interval": "120"
        }"#;

        let config: AppConfig = serde_json::from_str(legacy_json).expect("Must deserialize legacy config without new fields");
        assert_eq!(config.container_layout, ContainerLayout::Default);
        assert_eq!(config.mid_panel_order, MidPanelOrder::HistoryLogs);
        assert_eq!(config.border_style, BorderStyleChoice::Rounded);
        assert_eq!(config.graph_style, GraphStyleChoice::Braille);

        // Verify that legacy "Ascii" value is automatically aliased to Braille
        let legacy_ascii_json = r#"{
            "remote": "gdrive:",
            "local_dir": "~/Documents",
            "timer_interval": "15min",
            "full_sync_interval": "120",
            "graph_style": "Ascii"
        }"#;
        let config_ascii: AppConfig = serde_json::from_str(legacy_ascii_json).expect("Must deserialize legacy Ascii graph_style");
        assert_eq!(config_ascii.graph_style, GraphStyleChoice::Braille);
    }

    #[test]
    fn test_config_roundtrip_with_new_appearance_fields() {
        let mut config = AppConfig::default();
        config.container_layout = ContainerLayout::LogsTop;
        config.mid_panel_order = MidPanelOrder::LogsHistory;
        config.border_style = BorderStyleChoice::Double;
        config.graph_style = GraphStyleChoice::Braille;

        let json = serde_json::to_string(&config).expect("Must serialize");
        let deserialized: AppConfig = serde_json::from_str(&json).expect("Must deserialize");

        assert_eq!(deserialized.container_layout, ContainerLayout::LogsTop);
        assert_eq!(deserialized.mid_panel_order, MidPanelOrder::LogsHistory);
        assert_eq!(deserialized.border_style, BorderStyleChoice::Double);
        assert_eq!(deserialized.graph_style, GraphStyleChoice::Braille);
    }

    #[test]
    fn test_cloud_quota_cache_roundtrip() {
        let quota = CloudQuotaCache {
            total_bytes: 1_000_000_000_000,
            used_bytes: 400_000_000_000,
            free_bytes: 600_000_000_000,
        };
        let json = serde_json::to_string(&quota).unwrap();
        let loaded: CloudQuotaCache = serde_json::from_str(&json).unwrap();
        assert_eq!(quota, loaded);
    }

    #[test]
    fn test_settings_dynamic_descriptions_sync() {
        assert!(SettingId::TimerInterval.desc_intro().contains("rclone-bisync.timer"));
        assert!(SettingId::CloudSafetyNet.desc_intro().contains("bidirectional sync"));
        assert!(SettingId::BandwidthLimit.desc_intro().contains("bwlimit.env"));
        assert!(SettingId::ColorTheme.desc_intro().contains("dashboard"));
        assert!(SettingId::ContainerLayout.desc_intro().contains("containers"));
        assert!(SettingId::BorderStyle.desc_intro().contains("dash-config.json"));
        assert!(SettingId::GraphStyle.desc_intro().contains("sparkline"));
    }

    #[test]
    fn test_setting_choices_single_source_of_truth() {
        // TICK_RATE_STEPS doit contenir explicitement 250 (valeur par défaut)
        assert!(TICK_RATE_STEPS.contains(&250));
        assert_eq!(TICK_RATE_STEPS.len(), 22);

        // tick_rate_options() doit être strictement le reflet de TICK_RATE_STEPS
        let tick_opts = tick_rate_options();
        assert_eq!(tick_opts.len(), 22);
        for (i, &step) in TICK_RATE_STEPS.iter().enumerate() {
            assert_eq!(tick_opts[i], format!("{}ms", step));
        }

        // SettingId::TickRate.choices() renvoie exactement la même liste
        assert_eq!(SettingId::TickRate.choices(), Some(tick_opts));

        // Vérification de la source de vérité pour tous les paramètres à choix
        assert_eq!(
            SettingId::TimerInterval.choices().unwrap(),
            TIMER_INTERVAL_OPTIONS.iter().map(|s| s.to_string()).collect::<Vec<_>>()
        );
        assert_eq!(
            SettingId::CloudSafetyNet.choices().unwrap(),
            FULL_SYNC_OPTIONS.iter().map(|o| o.label.to_string()).collect::<Vec<_>>()
        );
        assert_eq!(
            SettingId::BandwidthLimit.choices().unwrap(),
            BWLIMIT_OPTIONS.iter().map(|s| s.to_string()).collect::<Vec<_>>()
        );
        assert_eq!(
            SettingId::ColorTheme.choices().unwrap(),
            ThemeChoice::all().iter().map(|t| t.name().to_string()).collect::<Vec<_>>()
        );
        assert_eq!(
            SettingId::ContainerLayout.choices().unwrap(),
            ContainerLayout::all().iter().map(|l| l.name().to_string()).collect::<Vec<_>>()
        );
        assert_eq!(
            SettingId::MidPanelOrder.choices().unwrap(),
            MidPanelOrder::all().iter().map(|m| m.name().to_string()).collect::<Vec<_>>()
        );
        assert_eq!(
            SettingId::BorderStyle.choices().unwrap(),
            BorderStyleChoice::all().iter().map(|b| b.name().to_string()).collect::<Vec<_>>()
        );
        assert_eq!(
            SettingId::GraphStyle.choices().unwrap(),
            GraphStyleChoice::all().iter().map(|g| g.name().to_string()).collect::<Vec<_>>()
        );

        // Les réglages texte / action n'ont pas de liste de choix
        assert!(SettingId::LocalDirectory.choices().is_none());
        assert!(SettingId::RemoteStorage.choices().is_none());
        assert!(SettingId::ResyncAction.choices().is_none());
        assert!(SettingId::LogJournalAction.choices().is_none());
    }

    #[test]
    fn test_format_setting_options_list_columns_and_precision() {
        // Liste courte (<= 8 éléments) : 1 colonne verticale
        let timer_formatted = format_setting_options_list(TIMER_INTERVAL_OPTIONS, "15min");
        assert_eq!(timer_formatted.lines().count(), 6);
        assert!(timer_formatted.contains("  ▶ 15min (active)"));
        assert!(timer_formatted.contains("  • 10min"));

        // Liste longue (22 éléments tick rate) : grille à 3 colonnes compacte (8 lignes)
        let choices = tick_rate_options();
        let formatted_100 = format_setting_options_list(&choices, "100ms");
        assert_eq!(formatted_100.lines().count(), 8);

        // Doit marquer '100ms' active mais PAS '1000ms' ni '10000ms'
        assert!(formatted_100.contains("▶ 100ms (active)"));
        assert!(!formatted_100.contains("▶ 1000ms (active)"));
        assert!(!formatted_100.contains("▶ 10000ms (active)"));
        assert!(formatted_100.contains("• 1000ms"));
        assert!(formatted_100.contains("• 10000ms"));

        // Doit marquer '250ms' active mais PAS '2500ms'
        let formatted_250 = format_setting_options_list(&choices, "250ms");
        assert!(formatted_250.contains("▶ 250ms (active)"));
        assert!(!formatted_250.contains("▶ 2500ms (active)"));
        assert!(formatted_250.contains("• 2500ms"));
    }
}
