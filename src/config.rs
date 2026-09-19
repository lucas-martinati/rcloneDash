use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::ui::theme::ThemeChoice;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ContainerLayout {
    #[default]
    Default,
    RecentFirst,
    LogsTop,
    Inverted,
}

impl ContainerLayout {
    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn short_name(&self) -> &'static str {
        match self {
            ContainerLayout::Default => "Standard",
            ContainerLayout::RecentFirst => "Recent First",
            ContainerLayout::LogsTop => "Logs/Hist Top",
            ContainerLayout::Inverted => "Inverted",
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn short_name(&self) -> &'static str {
        match self {
            BorderStyleChoice::Rounded => "Rounded",
            BorderStyleChoice::Sharp => "Sharp",
            BorderStyleChoice::Double => "Double",
            BorderStyleChoice::Thick => "Thick",
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
    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn short_name(&self) -> &'static str {
        match self {
            GraphStyleChoice::Braille => "Braille",
            GraphStyleChoice::Blocks => "Blocks",
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

#[allow(dead_code)]
pub fn auto_resync_notice_marker() -> PathBuf {
    config_dir().join(".auto-resync-notice")
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

#[allow(dead_code)]
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
}
