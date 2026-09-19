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
                info.active_substate = "Running".to_string();
            }
            "failed" => {
                info.state = ServiceState::Failed;
                info.active_substate = "Failed".to_string();
            }
            "inactive" | "deactivating" => {
                info.state = ServiceState::Idle;
                info.active_substate = "Idle".to_string();
            }
            _ => {
                info.state = ServiceState::Idle;
                info.active_substate = status;
            }
        }
    }

    // 2. Statut du timer
    let timer_out = Command::new("systemctl")
        .args(["--user", "list-timers", "--no-legend", "-l", "rclone-bisync.timer"])
        .output();

    if let Ok(out) = timer_out {
        let text = String::from_utf8_lossy(&out.stdout);
        let line = text.trim();
        if !line.is_empty() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let cfg_interval = config::load_config().timer_interval;
            let interval_mins: u64 = if cfg_interval.ends_with("min") {
                cfg_interval.trim_end_matches("min").parse().unwrap_or(10)
            } else if cfg_interval.ends_with('h') {
                cfg_interval.trim_end_matches('h').parse::<u64>().unwrap_or(1) * 60
            } else {
                10
            };

            if parts.is_empty() || parts[0] == "-" || parts[0] == "n/a" {
                // Le timer est inactif ou en attente d'inactivité du service
                // On cherche l'heure de la dernière exécution dans la ligne (ex: "Fri 2026-09-18 08:08:58 CEST")
                let last_time_opt = parts.iter().find(|p| p.contains(':') && p.len() >= 5);
                if let Some(lt) = last_time_opt {
                    if let Ok(last_chrono) = chrono::NaiveTime::parse_from_str(lt, "%H:%M:%S") {
                        let next_dt = last_chrono + chrono::Duration::minutes(interval_mins as i64);
                        let now = chrono::Local::now().time();
                        let diff_secs = (next_dt - now).num_seconds();
                        if diff_secs > 0 {
                            let mins = diff_secs / 60;
                            let secs = diff_secs % 60;
                            info.timer_next = next_dt.format("%H:%M:%S").to_string();
                            info.timer_left = format!("{}m {:02}s", mins, secs);
                        } else if info.state == ServiceState::Active {
                            info.timer_next = "In progress".to_string();
                            info.timer_left = format!("after sync ({})", cfg_interval);
                        } else {
                            info.timer_next = next_dt.format("%H:%M:%S").to_string();
                            info.timer_left = "imminent".to_string();
                        }
                    } else {
                        info.timer_next = format!("~{}", cfg_interval);
                        info.timer_left = format!("in ~{}", cfg_interval);
                    }
                } else {
                    info.timer_next = format!("~{}", cfg_interval);
                    info.timer_left = format!("in ~{}", cfg_interval);
                }
            } else {
                // Date/Heure programmée présente dans parts
                // Format: Day Date Time Timezone ...
                let next_time = if parts.len() >= 3 && parts[2].contains(':') {
                    parts[2]
                } else if parts.len() >= 2 && parts[1].contains(':') {
                    parts[1]
                } else {
                    parts[0]
                };
                info.timer_next = next_time.to_string();

                // Recherche du décompte (ex: "9min left" ou "374ms")
                if let Some(left_idx) = parts.iter().position(|&w| w == "left") {
                    if left_idx > 0 {
                        info.timer_left = parts[left_idx - 1].to_string();
                    }
                } else if parts.len() >= 5 && parts[4] != "-" && !parts[4].is_empty() {
                    info.timer_left = parts[4].to_string();
                } else {
                    info.timer_left = next_time.to_string();
                }
            }
        } else {
            info.timer_left = "Disabled".to_string();
            info.timer_next = "Disabled".to_string();
        }
    }

    // 3. Filet de sécurité Cloud (sync complet périodique)
    let cfg = config::load_config();
    if cfg.full_sync_interval == "never" {
        info.cloud_safety_net = "Disabled".to_string();
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
                        info.cloud_safety_net = "Full sync due".to_string();
                    } else {
                        let left = full_interval - diff_min;
                        info.cloud_safety_net = format!("in {} min", left);
                    }
                }
            }
        } else {
            info.cloud_safety_net = "On next run".to_string();
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

