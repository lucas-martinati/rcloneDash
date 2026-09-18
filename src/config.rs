use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::ui::theme::ThemeChoice;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub remote: String,
    pub local_dir: String,
    pub timer_interval: String,
    pub full_sync_interval: String,
    pub bwlimit: Option<String>,
    pub theme: Option<ThemeChoice>,
    pub tick_rate_ms: Option<u64>,
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
    }
    Ok(())
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
