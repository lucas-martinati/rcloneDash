use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::config;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceState {
    Active,
    Idle,
    Failed,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub state: ServiceState,
    pub active_substate: String,
    pub timer_next: String,
    pub timer_left: String,
    pub cloud_safety_net: String,
}

impl Default for ServiceInfo {
    fn default() -> Self {
        Self {
            state: ServiceState::Unknown,
            active_substate: "inconnu".to_string(),
            timer_next: "--".to_string(),
            timer_left: "--".to_string(),
            cloud_safety_net: "--".to_string(),
        }
    }
}

pub fn get_service_info() -> ServiceInfo {
    let mut info = ServiceInfo::default();

    // 1. Statut du service
    let output = Command::new("systemctl")
        .args(["--user", "is-active", "rclone-bisync.service"])
        .output();

    if let Ok(out) = output {
        let status = String::from_utf8_lossy(&out.stdout).trim().to_lowercase();
        match status.as_str() {
            "active" | "activating" => {
                info.state = ServiceState::Active;
                info.active_substate = "En cours d'exécution".to_string();
            }
            "failed" => {
                info.state = ServiceState::Failed;
                info.active_substate = "Échec (Failed)".to_string();
            }
            "inactive" | "deactivating" => {
                info.state = ServiceState::Idle;
                info.active_substate = "En veille".to_string();
            }
            _ => {
                info.state = ServiceState::Idle;
                info.active_substate = status;
            }
        }
    }

    // 2. Statut du timer
    let timer_out = Command::new("systemctl")
        .args(["--user", "list-timers", "--no-legend", "rclone-bisync.timer"])
        .output();

    if let Ok(out) = timer_out {
        let text = String::from_utf8_lossy(&out.stdout);
        let line = text.trim();
        if !line.is_empty() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            // Format typique: NEXT LEFT LAST PASSED UNIT ACTIVATES
            // Ex: "Fri 2026-09-18 00:10:00 CEST  8min left ..."
            if parts.len() >= 5 {
                // Recherche de "left"
                if let Some(left_idx) = parts.iter().position(|&w| w == "left") {
                    if left_idx > 0 {
                        info.timer_left = parts[left_idx - 1].to_string();
                    }
                } else if parts.len() >= 4 {
                    info.timer_left = parts[3].to_string();
                }
                info.timer_next = parts[..3.min(parts.len())].join(" ");
            } else {
                info.timer_left = line.to_string();
            }
        } else {
            info.timer_left = "Désactivé".to_string();
        }
    }

    // 3. Filet de sécurité Cloud (sync complet périodique)
    let cfg = config::load_config();
    if cfg.full_sync_interval == "never" {
        info.cloud_safety_net = "Désactivé".to_string();
    } else {
        let stamp_path = config::last_full_sync_marker();
        if stamp_path.exists() {
            if let Ok(content) = fs::read_to_string(&stamp_path) {
                if let Ok(ts) = content.trim().parse::<u64>() {
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    let diff_min = if now >= ts { (now - ts) / 60 } else { 0 };
                    let full_interval: u64 = cfg.full_sync_interval.parse().unwrap_or(60);
                    if diff_min >= full_interval {
                        info.cloud_safety_net = "Sync complet dû".to_string();
                    } else {
                        let left = full_interval - diff_min;
                        info.cloud_safety_net = format!("dans {} min", left);
                    }
                }
            }
        } else {
            info.cloud_safety_net = "Au prochain run".to_string();
        }
    }

    info
}

pub fn trigger_sync() -> Result<(), String> {
    let marker = config::force_sync_marker();
    if let Some(parent) = marker.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&marker, "");

    let out = Command::new("systemctl")
        .args(["--user", "start", "--no-block", "rclone-bisync.service"])
        .output()
        .map_err(|e| e.to_string())?;

    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn trigger_resync() -> Result<(), String> {
    let resync_marker = config::resync_marker();
    let force_marker = config::force_sync_marker();

    if let Some(parent) = resync_marker.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::remove_file(&force_marker);
    let _ = fs::write(&resync_marker, "");

    let out = Command::new("systemctl")
        .args(["--user", "start", "--no-block", "rclone-bisync.service"])
        .output()
        .map_err(|e| e.to_string())?;

    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn cancel_sync() -> Result<(), String> {
    let out = Command::new("systemctl")
        .args(["--user", "stop", "--no-block", "rclone-bisync.service"])
        .output()
        .map_err(|e| e.to_string())?;

    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn get_disk_usage(path: &str) -> (f64, f64, f64, f64) {
    use std::ffi::CString;
    use std::mem::MaybeUninit;

    let expanded = config::expand_tilde(path);
    let c_path = match CString::new(expanded.to_string_lossy().as_bytes()) {
        Ok(p) => p,
        Err(_) => return (0.0, 0.0, 0.0, 0.0),
    };

    let mut stat: MaybeUninit<libc::statvfs> = MaybeUninit::uninit();
    if unsafe { libc::statvfs(c_path.as_ptr(), stat.as_mut_ptr()) } == 0 {
        let stat = unsafe { stat.assume_init() };
        let frsize = stat.f_frsize as f64;
        let total_bytes = stat.f_blocks as f64 * frsize;
        let free_bytes = stat.f_bavail as f64 * frsize;
        let _used_bytes = (total_bytes - stat.f_bfree as f64 * frsize).max(0.0);
        let g = 1024.0 * 1024.0 * 1024.0;
        let total_gb = (total_bytes / g * 10.0).round() / 10.0;
        let free_gb = (free_bytes / g * 10.0).round() / 10.0;
        let used_gb = ((total_bytes - free_bytes) / g * 10.0).round() / 10.0;
        let pct = if total_gb > 0.0 { (used_gb / total_gb * 100.0).round() } else { 0.0 };
        (used_gb, free_gb, total_gb, pct)
    } else {
        (0.0, 0.0, 0.0, 0.0)
    }
}

