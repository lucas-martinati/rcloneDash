use regex::Regex;
use serde::Deserialize;
use std::sync::LazyLock;
use std::time::Instant;

pub use crate::monitor::events::{
    ActiveFile, FileAction, ModifiedFileDetail, SyncEvent, SyncPhase, SyncedFile, TransferStats,
};

// =============================================================================
// Modèles Serde pour les logs JSON officiels de rclone (--use-json-log / slog)
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct RcloneJsonLog {
    pub time: Option<String>,
    pub msg: Option<String>,
    pub object: Option<String>,
    pub stats: Option<RcloneStats>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RcloneStats {
    pub bytes: Option<i64>,
    pub total_bytes: Option<i64>,
    pub speed: Option<f64>,
    pub eta: Option<f64>,
    pub elapsed_time: Option<f64>,
    pub errors: Option<u32>,
    pub checks: Option<u32>,
    pub total_checks: Option<u32>,
    pub transfers: Option<u32>,
    pub total_transfers: Option<u32>,
    pub transferring: Option<Vec<RcloneTransferring>>,
}

#[derive(Debug, Deserialize)]
pub struct RcloneTransferring {
    pub name: String,
    pub percentage: Option<u8>,
    pub speed: Option<f64>,
}

// =============================================================================
// Fonctions de formatage universelles (sans regex)
// =============================================================================

pub fn format_bytes(bytes: i64) -> String {
    if bytes <= 0 {
        return "0 B".to_string();
    }
    let b = bytes as f64;
    const KIB: f64 = 1024.0;
    const MIB: f64 = 1024.0 * 1024.0;
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    const TIB: f64 = 1024.0 * 1024.0 * 1024.0 * 1024.0;

    if b >= TIB {
        format!("{:.3} TiB", b / TIB)
    } else if b >= GIB {
        format!("{:.3} GiB", b / GIB)
    } else if b >= MIB {
        format!("{:.3} MiB", b / MIB)
    } else if b >= KIB {
        format!("{:.3} KiB", b / KIB)
    } else {
        format!("{} B", bytes)
    }
}

pub fn format_speed(bytes_per_sec: f64) -> String {
    if bytes_per_sec <= 0.0 {
        return "0 B/s".to_string();
    }
    const KIB: f64 = 1024.0;
    const MIB: f64 = 1024.0 * 1024.0;
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;

    if bytes_per_sec >= GIB {
        format!("{:.3} GiB/s", bytes_per_sec / GIB)
    } else if bytes_per_sec >= MIB {
        format!("{:.3} MiB/s", bytes_per_sec / MIB)
    } else if bytes_per_sec >= KIB {
        format!("{:.3} KiB/s", bytes_per_sec / KIB)
    } else {
        format!("{:.0} B/s", bytes_per_sec)
    }
}

pub fn format_eta(eta_secs: f64) -> String {
    if eta_secs < 0.0 || eta_secs.is_nan() || eta_secs.is_infinite() {
        return "-".to_string();
    }
    let secs = eta_secs.round() as u64;
    if secs == 0 {
        "0s".to_string()
    } else if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        let m = secs / 60;
        let s = secs % 60;
        if s == 0 {
            format!("{}m", m)
        } else {
            format!("{}m{}s", m, s)
        }
    } else {
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        format!("{}h{}m", h, m)
    }
}

pub fn format_elapsed(elapsed_secs: f64) -> String {
    if elapsed_secs <= 0.0 {
        return "0.0s".to_string();
    }
    if elapsed_secs < 60.0 {
        format!("{:.1}s", elapsed_secs)
    } else {
        let secs = elapsed_secs.floor() as u64;
        let m = secs / 60;
        let s = secs % 60;
        format!("{}m{}s", m, s)
    }
}

// =============================================================================
// Expressions régulières héritées (pour rétrocompatibilité et fallback texte)
// =============================================================================

static RE_SYNCED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:.*?\s+)?(?:INFO|NOTICE|DEBUG|WARN|WARNING)\s*:\s+(.*?):\s+(Copied \(new\)|Copied \(replaced existing\)|Updated modification time in destination|Deleted|Updated file)").unwrap()
});

static RE_TRANSFER_BYTES: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)transferred:\s+([\d.]+\s*\S+)\s*/\s*([\d.]+\s*\S+),\s*(\d+|-)\s*%?").unwrap()
});

static RE_SPEED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)([\d.]+\s*\S+/s)").unwrap()
});

static RE_ETA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)ETA\s+(\S+)").unwrap()
});

static RE_FILES: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)transferred:\s+(\d+)\s*/\s*(\d+),?").unwrap()
});

static RE_CHECKS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)checks:\s+(\d+)\s*/\s*(\d+)").unwrap()
});

static RE_ELAPSED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)elapsed time:\s*(\S+)").unwrap()
});

static RE_ACTIVE_FULL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\*\s+(.+?):\s*(\d+)%\s*/([^,]+),\s*([^,]+),\s*(\S+)").unwrap()
});

static RE_ACTIVE_SHORT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\*\s+(.+?):\s*(\d+)%\s*/(\S+)").unwrap()
});

static RE_ACTIVE_STATUS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\*\s+(.+?):\s*(checking|transferring)\s*$").unwrap()
});

// =============================================================================
// Parseurs spécifiques
// =============================================================================

/// Détection d'un message critique demandant un resync
pub fn is_resync_trigger(line: &str) -> bool {
    let ll = line.to_lowercase();
    ll.contains("cannot find prior")
        || ll.contains("must run --resync")
        || ll.contains("--resync to recover")
        || ll.contains("path1 and path2 are out of sync")
        || ll.contains("prior or current is not in sync")
}

/// Parsing hérité d'un fichier synchronisé via regex
pub fn parse_synced_file(line: &str) -> Option<SyncedFile> {
    if let Some(caps) = RE_SYNCED.captures(line) {
        let fpath = caps.get(1)?.as_str().trim().to_string();
        let act = caps.get(2)?.as_str().trim();
        let action = if act.contains("Copied (new)") {
            FileAction::New
        } else if act.contains("Deleted") {
            FileAction::Deleted
        } else {
            FileAction::Modified
        };
        let now = chrono::Local::now().format("%H:%M:%S").to_string();
        return Some(SyncedFile {
            path: fpath,
            action,
            time: now,
        });
    }
    None
}

/// Parsing hérité des métriques de transfert via regex
pub fn parse_transfer_stats(line: &str, stats: &mut TransferStats) {
    let ll = line.to_lowercase();

    if let Some(caps) = RE_TRANSFER_BYTES.captures(&ll) {
        if let (Some(d), Some(t), Some(p)) = (caps.get(1), caps.get(2), caps.get(3)) {
            stats.bytes_done = d.as_str().trim().to_string();
            stats.bytes_total = t.as_str().trim().to_string();
            let p_str = p.as_str().trim();
            stats.pct = p_str.parse().unwrap_or(0);
        }
    }

    if let Some(caps) = RE_FILES.captures(&ll) {
        if let (Some(d), Some(t)) = (caps.get(1), caps.get(2)) {
            stats.files_done = d.as_str().parse().unwrap_or(0);
            stats.files_total = t.as_str().parse().unwrap_or(0);
        }
    }

    if let Some(caps) = RE_CHECKS.captures(&ll) {
        if let (Some(d), Some(t)) = (caps.get(1), caps.get(2)) {
            stats.checks_done = d.as_str().parse().unwrap_or(0);
            stats.checks_total = t.as_str().parse().unwrap_or(0);
        }
    }

    if let Some(caps) = RE_SPEED.captures(line) {
        if let Some(s) = caps.get(1) {
            stats.speed = s.as_str().trim().to_string();
        }
    }

    if let Some(caps) = RE_ETA.captures(line) {
        if let Some(e) = caps.get(1) {
            stats.eta = e.as_str().trim().to_string();
        }
    }

    if let Some(caps) = RE_ELAPSED.captures(&ll) {
        if let Some(el) = caps.get(1) {
            stats.elapsed = el.as_str().trim().to_string();
        }
    }
}

/// Parsing hérité des transferts actifs via regex
pub fn parse_active_file(line: &str) -> Option<ActiveFile> {
    if let Some(caps) = RE_ACTIVE_FULL.captures(line) {
        let name = caps.get(1)?.as_str().trim().to_string();
        let pct = caps.get(2)?.as_str().parse().unwrap_or(0);
        let speed = caps.get(4).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
        return Some(ActiveFile {
            name,
            pct,
            speed,
            last_seen: Instant::now(),
        });
    }

    if let Some(caps) = RE_ACTIVE_SHORT.captures(line) {
        let name = caps.get(1)?.as_str().trim().to_string();
        let pct = caps.get(2)?.as_str().parse().unwrap_or(0);
        return Some(ActiveFile {
            name,
            pct,
            speed: "".to_string(),
            last_seen: Instant::now(),
        });
    }

    if let Some(caps) = RE_ACTIVE_STATUS.captures(line) {
        let name = caps.get(1)?.as_str().trim().to_string();
        return Some(ActiveFile {
            name,
            pct: 0,
            speed: "".to_string(),
            last_seen: Instant::now(),
        });
    }

    None
}

/// Parsing d'une ligne de diff pour extraire le fichier et l'action
pub fn parse_diff_file(raw_line: &str) -> Option<(bool, ModifiedFileDetail)> {
    let clean_line = strip_ansi(raw_line);
    let ll = clean_line.to_ascii_lowercase();
    if !ll.contains("path1") && !ll.contains("path2") {
        return None;
    }

    let (action, action_str) = if ll.contains("file is new") {
        (FileAction::New, "file is new")
    } else if ll.contains("file changed") {
        (FileAction::Modified, "file changed")
    } else if ll.contains("file was deleted") || ll.contains("file deleted") {
        (FileAction::Deleted, if ll.contains("file was deleted") { "file was deleted" } else { "file deleted" })
    } else if ll.contains("queue copy") {
        (FileAction::Copied, "queue copy")
    } else if ll.contains("queue delete") {
        (FileAction::Deleted, "queue delete")
    } else {
        return None;
    };

    let action_pos = ll.find(action_str)?;
    let after_action = &clean_line[action_pos + action_str.len()..];
    let delim_pos = after_action.find(" - ")?;
    let mut fname = after_action[delim_pos + 3..].trim();
    if fname.is_empty() {
        return None;
    }
    // Strip remote prefix if any (e.g. GoogleDrive{...}:/)
    if let Some(colon_pos) = fname.find("}:/") {
        fname = &fname[colon_pos + 3..];
    } else if let Some(colon_pos) = fname.find(":/") {
        fname = &fname[colon_pos + 2..];
    }

    let fname_lower = fname.to_ascii_lowercase();
    if fname_lower.contains("path1") || fname_lower.contains("path2") || fname.contains('"') {
        return None;
    }

    let is_local = if ll.contains("path2:") || ll.contains("- path2") {
        true
    } else if ll.contains("path1:") || ll.contains("- path1") {
        false
    } else {
        ll.contains("path2") && !ll.contains("path1")
    };

    Some((
        is_local,
        ModifiedFileDetail {
            path: fname.to_string(),
            action,
        },
    ))
}

pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if let Some(&next) = chars.peek() {
                if next == '[' {
                    chars.next();
                    for code in chars.by_ref() {
                        if (0x40..=0x7E).contains(&(code as u32)) {
                            break;
                        }
                    }
                } else if next == ']' {
                    chars.next();
                    while let Some(osc) = chars.next() {
                        if osc == '\x07' || (osc == '\x1b' && chars.peek() == Some(&'\\')) {
                            if osc == '\x1b' {
                                chars.next();
                            }
                            break;
                        }
                    }
                } else if next == '(' || next == ')' {
                    chars.next();
                    chars.next();
                }
            }
        } else if c == '\t' {
            out.push_str("    ");
        } else if c == '\n' || (c != '\r' && !c.is_control()) {
            out.push(c);
        }
    }
    out
}

// =============================================================================
// Parseur Universel Hybride (JSON + Fallback Texte)
// =============================================================================

/// Formate une ligne de log (JSON structuré ou texte brut) en une ou plusieurs lignes
/// lisibles par un humain pour affichage dans le panneau des logs du dashboard.
pub fn format_log_line_for_display(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    // Si la ligne contient un objet JSON (même précédé de "INFO  : " ou préfixe journalctl)
    if let Some(idx) = trimmed.find('{') {
        let json_part = &trimmed[idx..];
        if let Ok(json_log) = serde_json::from_str::<RcloneJsonLog>(json_part) {
            let mut result = Vec::new();

            let time_prefix = if let Some(t_str) = json_log.time.as_deref() {
                if t_str.len() >= 19 && (t_str.chars().nth(10) == Some('T') || t_str.chars().nth(10) == Some(' ')) {
                    let d = &t_str[..10];
                    let t = &t_str[11..19];
                    format!("{} {}  ", d, t)
                } else if !t_str.is_empty() {
                    format!("{}  ", t_str)
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            // A. Fichier synchronisé (ex: "Documents/rapport.pdf: Copied (new)")
            if let Some(obj) = json_log.object {
                let msg = json_log.msg.as_deref().unwrap_or("");
                let msg_lower = msg.to_lowercase();
                if msg_lower.contains("directory") || msg_lower.contains("setmodtime") {
                    return Vec::new();
                }
                result.push(format!("{}{}: {}", time_prefix, obj, msg));
                return result;
            }

            // B. Message avec stats de transfert (ex: "\nTransferred: ...\nChecks: ...\n")
            if json_log.stats.is_some() {
                if let Some(msg) = json_log.msg.as_deref() {
                    let clean = strip_ansi(msg);
                    for sub in clean.lines() {
                        let sub_trim = sub.trim();
                        if !sub_trim.is_empty() {
                            result.push(sub_trim.to_string());
                        }
                    }
                    if !result.is_empty() {
                        return result;
                    }
                }
            }

            // C. Message informatif standard (ex: "Building Path1 and Path2 listings", "Bisync successful")
            if let Some(msg) = json_log.msg.as_deref() {
                let clean = strip_ansi(msg);
                let clean_trim = clean.trim();
                let clean_lower = clean_trim.to_lowercase();
                if clean_lower.contains("directory modification time") || clean_lower.contains("setmodtime") {
                    return Vec::new();
                }
                if !clean_trim.is_empty() {
                    result.push(format!("{}{}", time_prefix, clean_trim));
                    return result;
                }
            }

            return result;
        }
    }

    // Ligne texte brute non-JSON (ou JSON invalide) : nettoyée des codes ANSI
    let clean = strip_ansi(trimmed);
    let clean_lower = clean.to_lowercase();
    if clean_lower.contains("set directory modification time") || clean_lower.contains("setmodtime") {
        return Vec::new();
    }
    let lines: Vec<String> = clean
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    if lines.is_empty() {
        Vec::new()
    } else {
        lines
    }
}

/// Analyse une ligne de log quelconque (JSON structuré ou texte brut) et produit
/// un ou plusieurs événements normalisés `SyncEvent`.
pub fn parse_log_line(line: &str) -> Vec<SyncEvent> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    // 1. Essai de parsing JSON si la ligne contient `{`
    if let Some(pos) = trimmed.find('{') {
        let json_part = &trimmed[pos..];
        if let Ok(json_log) = serde_json::from_str::<RcloneJsonLog>(json_part) {
            return parse_json_event(json_log);
        }
    }

    // 2. Repli automatique sur le parsing textuel hérité
    parse_legacy_text_event(trimmed)
}

/// Convertit un objet JSON rclone (`RcloneJsonLog`) en événements `SyncEvent`
fn parse_json_event(log: RcloneJsonLog) -> Vec<SyncEvent> {
    let mut events = Vec::new();

    // A. Statistiques globales et transferts actifs
    if let Some(stats) = log.stats {
        if let Some(err_count) = stats.errors {
            if err_count > 0 {
                events.push(SyncEvent::SyncCompleted {
                    success: false,
                    error_msg: Some(format!("rclone reported {} error(s)", err_count)),
                });
            }
        }

        let bytes_done_num = stats.bytes.unwrap_or(0);
        let bytes_total_num = stats.total_bytes.unwrap_or(0);
        let pct = if bytes_total_num > 0 {
            ((bytes_done_num as f64 / bytes_total_num as f64) * 100.0).clamp(0.0, 100.0) as u8
        } else {
            0
        };

        let transfer = TransferStats {
            bytes_done: format_bytes(bytes_done_num).to_lowercase(),
            bytes_total: format_bytes(bytes_total_num).to_lowercase(),
            pct,
            speed: format_speed(stats.speed.unwrap_or(0.0)),
            eta: stats.eta.map(format_eta).unwrap_or_else(|| "-".to_string()),
            files_done: stats.transfers.unwrap_or(0),
            files_total: stats.total_transfers.unwrap_or(0),
            checks_done: stats.checks.unwrap_or(0),
            checks_total: stats.total_checks.unwrap_or(0),
            elapsed: stats.elapsed_time.map(format_elapsed).unwrap_or_else(|| "0.0s".to_string()),
        };
        events.push(SyncEvent::StatsUpdated(transfer));

        if let Some(transferring) = stats.transferring {
            for t in transferring {
                let speed_str = t.speed.map(format_speed).unwrap_or_default();
                events.push(SyncEvent::ActiveTransferUpdated(ActiveFile {
                    name: t.name,
                    pct: t.percentage.unwrap_or(0),
                    speed: speed_str,
                    last_seen: Instant::now(),
                }));
            }
        }
    }

    // B. Fichier synchronisé
    if let Some(obj) = log.object {
        let msg = log.msg.as_deref().unwrap_or("");
        let msg_lower = msg.to_lowercase();

        // Ignorer les opérations de métadonnées sur les répertoires
        if !msg_lower.contains("directory") && !msg_lower.contains("setmodtime") {
            let action = if msg.contains("Copied (new)") {
                Some(FileAction::New)
            } else if msg.contains("Deleted") {
                Some(FileAction::Deleted)
            } else if msg.contains("Copied (replaced existing)")
                || msg.contains("Updated file")
                || msg.contains("Updated modification time")
                || msg.contains("file changed")
            {
                Some(FileAction::Modified)
            } else {
                None
            };

            if let Some(action) = action {
                let time = if let Some(t_str) = log.time.as_deref() {
                    if let Some(pos) = t_str.find('T') {
                        t_str[pos + 1..].chars().take(8).collect()
                    } else {
                        chrono::Local::now().format("%H:%M:%S").to_string()
                    }
                } else {
                    chrono::Local::now().format("%H:%M:%S").to_string()
                };

                events.push(SyncEvent::FileSynced(SyncedFile {
                    path: obj,
                    action,
                    time,
                }));
            }
        }
    }

    // C. Messages informatifs / phases / diffs
    if let Some(msg) = log.msg {
        let clean = strip_ansi(&msg);
        let ll = clean.to_lowercase();

        if is_resync_trigger(&clean) {
            events.push(SyncEvent::ResyncRequired(clean.clone()));
        }

        if ll.contains("synching path1") || ll.contains("bisyncing with") || ll.contains("building path1 and path2 listings") {
            events.push(SyncEvent::SyncStarted { reason: None });
            events.push(SyncEvent::PhaseChanged(SyncPhase::Listings));
        }

        if ll.contains("path1 was modified") || ll.contains("differences found on path1") {
            events.push(SyncEvent::PathModified { is_local: false });
        }
        if ll.contains("path2 was modified") || ll.contains("differences found on path2") {
            events.push(SyncEvent::PathModified { is_local: true });
        }

        if ll.contains("updating listings") || ll.contains("updating path") {
            events.push(SyncEvent::PhaseChanged(SyncPhase::Updating));
        } else if ll.contains("applying changes") || ll.contains("synching path1 to path2") || ll.contains("synching path2 to path1") {
            events.push(SyncEvent::PhaseChanged(SyncPhase::Applying));
        } else if ll.contains("path2 checking for diffs") || ll.contains("validating listings for path2") {
            events.push(SyncEvent::PhaseChanged(SyncPhase::LocalDiffs));
        } else if ll.contains("path1 checking for diffs") || ll.contains("validating listings for path1") {
            events.push(SyncEvent::PhaseChanged(SyncPhase::RemoteDiffs));
        }

        if let Some((is_local, detail)) = parse_diff_file(&clean) {
            events.push(SyncEvent::DiffFound { is_local, detail });
        }

        if ll.contains("bisync successful") {
            events.push(SyncEvent::PhaseChanged(SyncPhase::Done));
            events.push(SyncEvent::SyncCompleted { success: true, error_msg: None });
        } else if ll.contains("bisync error:") || ll.contains("bisync aborted") {
            events.push(SyncEvent::SyncCompleted {
                success: false,
                error_msg: Some(clean),
            });
        }
    }

    events
}

/// Convertit une ligne textuelle héritée en événements `SyncEvent`
fn parse_legacy_text_event(line: &str) -> Vec<SyncEvent> {
    let mut events = Vec::new();
    let clean = strip_ansi(line);
    let ll = clean.to_lowercase();

    // Démarrage de synchronisation
    let is_start = (ll.contains("systemd") && (ll.contains("starting") || ll.contains("started")) && ll.contains("rclone-bisync"))
        || (ll.contains("rclone-bisync-guard") && ll.contains("lancement du bisync"))
        || ll.contains("rclonedash: lancement du bisync")
        || ll.contains("synching path1")
        || ll.contains("bisyncing with")
        || ll.contains("building path1 and path2 listings");

    if is_start {
        events.push(SyncEvent::SyncStarted {
            reason: Some(clean.clone()),
        });
        events.push(SyncEvent::PhaseChanged(SyncPhase::Listings));
    }

    // Modification d'un côté
    if ll.contains("path1 was modified") || ll.contains("differences found on path1") {
        events.push(SyncEvent::PathModified { is_local: false });
    }
    if ll.contains("path2 was modified") || ll.contains("differences found on path2") {
        events.push(SyncEvent::PathModified { is_local: true });
    }

    // Changement de phase
    if ll.contains("updating listings") || ll.contains("updating path") {
        events.push(SyncEvent::PhaseChanged(SyncPhase::Updating));
    } else if ll.contains("applying changes")
        || ll.contains("synching path1 to path2")
        || ll.contains("synching path2 to path1")
        || (ll.contains("copying") && !ll.contains("copying path") && !ll.contains("queue copy"))
    {
        events.push(SyncEvent::PhaseChanged(SyncPhase::Applying));
    } else if ll.contains("path2 checking for diffs")
        || ll.contains("path2: checking")
        || ll.contains("validating listings for path2")
        || ll.contains("differences found on path2")
    {
        events.push(SyncEvent::PhaseChanged(SyncPhase::LocalDiffs));
    } else if ll.contains("path1 checking for diffs")
        || ll.contains("path1: checking")
        || ll.contains("validating listings for path1")
        || ll.contains("differences found on path1")
    {
        events.push(SyncEvent::PhaseChanged(SyncPhase::RemoteDiffs));
    }

    // Stats de transfert
    let mut stats = TransferStats::default();
    parse_transfer_stats(line, &mut stats);
    if !stats.bytes_done.is_empty()
        || stats.files_done > 0
        || stats.checks_done > 0
        || !stats.speed.is_empty()
        || !stats.elapsed.is_empty()
    {
        events.push(SyncEvent::StatsUpdated(stats));
    }

    // Fichier actif
    if let Some(active) = parse_active_file(line) {
        events.push(SyncEvent::ActiveTransferUpdated(active));
    }

    // Fichier synchronisé
    if let Some(synced) = parse_synced_file(line) {
        events.push(SyncEvent::FileSynced(synced));
    }

    // Diff détecté
    if let Some((is_local, detail)) = parse_diff_file(line) {
        events.push(SyncEvent::DiffFound { is_local, detail });
    }

    // Resync trigger
    if is_resync_trigger(line) {
        events.push(SyncEvent::ResyncRequired(clean.clone()));
    }

    // Fin de synchronisation
    let is_finished = ll.contains("bisync successful")
        || (ll.contains("systemd") && (ll.contains("finished") || ll.contains("stopped") || ll.contains("deactivated")) && ll.contains("rclone-bisync"))
        || (ll.contains("rclone-bisync-guard") && ll.contains("aucun changement"))
        || (ll.contains("systemd") && ll.contains("failed") && ll.contains("rclone-bisync"))
        || ll.contains("bisync error:")
        || ll.contains("bisync aborted");

    if is_finished {
        let success = !ll.contains("failed") && !ll.contains("error:") && !ll.contains("aborted");
        events.push(SyncEvent::PhaseChanged(SyncPhase::Done));
        events.push(SyncEvent::SyncCompleted {
            success,
            error_msg: if !success { Some(clean) } else { None },
        });
    }

    events
}

// =============================================================================
// Tests unitaires
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_log_line_for_display() {
        // Ligne avec préfixe "INFO  : " et stats JSON
        let l1 = r#"INFO  : {"time":"2026-09-21T09:45:44.586549258+02:00","level":"info","msg":"\nTransferred:   \t          0 B / 0 B, -, 0 B/s, ETA -\nChecks:                42 / 42, 100%, Listed 147\nElapsed time:         2.0s\n\n","stats":{"bytes":0,"checks":42}}"#;
        let res1 = format_log_line_for_display(l1);
        assert_eq!(res1.len(), 3);
        assert!(res1[0].contains("Transferred:"));
        assert!(res1[1].contains("Checks:"));
        assert!(res1[2].contains("Elapsed time:"));

        // Ligne avec fichier copié
        let l2 = r#"INFO  : {"time":"2026-09-21T09:45:45.000000000+02:00","level":"info","msg":"Copied (new)","object":"folder/file.txt"}"#;
        let res2 = format_log_line_for_display(l2);
        assert_eq!(res2.len(), 1);
        assert_eq!(res2[0], "2026-09-21 09:45:45  folder/file.txt: Copied (new)");

        // Ligne informative standard
        let l3 = r#"INFO  : {"time":"2026-09-21T09:45:46.000000000+02:00","level":"info","msg":"Building Path1 and Path2 listings"}"#;
        let res3 = format_log_line_for_display(l3);
        assert_eq!(res3.len(), 1);
        assert_eq!(res3[0], "2026-09-21 09:45:46  Building Path1 and Path2 listings");

        // Ligne texte brute
        let l4 = "rclone-bisync.service: Failed with result 'signal'.";
        let res4 = format_log_line_for_display(l4);
        assert_eq!(res4.len(), 1);
        assert_eq!(res4[0], l4);

        // Ligne texte brute avec séquences ANSI
        let l5 = "\u{1b}[31mErreur critique système\u{1b}[0m";
        let res5 = format_log_line_for_display(l5);
        assert_eq!(res5.len(), 1);
        assert_eq!(res5[0], "Erreur critique système");
    }

    #[test]
    fn test_parse_json_log_stats_errors() {
        let raw_json = r#"{"time":"2026-09-21T09:40:42.000000000+02:00","level":"error","msg":"","stats":{"errors":2}}"#;
        let events = parse_log_line(raw_json);
        assert!(events.contains(&SyncEvent::SyncCompleted {
            success: false,
            error_msg: Some("rclone reported 2 error(s)".to_string()),
        }));
    }

    #[test]
    fn test_parse_json_log_stats_and_active_files() {
        let raw_json = r#"{"time":"2026-09-21T09:40:42.278823843+02:00","level":"info","msg":"","stats":{"bytes":23097344,"checks":1,"elapsedTime":2.2,"errors":0,"eta":2,"speed":10499916.0,"totalBytes":52428800,"totalChecks":1,"totalTransfers":1,"transferring":[{"bytes":23097344,"eta":2,"name":"large.bin","percentage":44,"size":52428800,"speed":10558376.0}],"transfers":0}}"#;

        let events = parse_log_line(raw_json);
        assert!(!events.is_empty(), "Should produce events from JSON log");

        let mut has_stats = false;
        let mut has_active = false;

        for ev in events {
            match ev {
                SyncEvent::StatsUpdated(st) => {
                    has_stats = true;
                    assert_eq!(st.pct, 44);
                    assert!(st.speed.contains("MiB/s"));
                    assert_eq!(st.eta, "2s");
                    assert_eq!(st.checks_done, 1);
                    assert_eq!(st.checks_total, 1);
                }
                SyncEvent::ActiveTransferUpdated(act) => {
                    has_active = true;
                    assert_eq!(act.name, "large.bin");
                    assert_eq!(act.pct, 44);
                    assert!(act.speed.contains("MiB/s"));
                }
                _ => {}
            }
        }

        assert!(has_stats, "Must have StatsUpdated event");
        assert!(has_active, "Must have ActiveTransferUpdated event");
    }

    #[test]
    fn test_parse_json_log_copied_file() {
        let raw_json = r#"{"time":"2026-09-21T09:40:47.395205362+02:00","level":"info","msg":"Copied (new)","size":6,"object":"folder/doc.pdf"}"#;

        let events = parse_log_line(raw_json);
        assert_eq!(events.len(), 1);

        match &events[0] {
            SyncEvent::FileSynced(sf) => {
                assert_eq!(sf.path, "folder/doc.pdf");
                assert_eq!(sf.action, FileAction::New);
                assert_eq!(sf.time, "09:40:47");
            }
            _ => panic!("Expected FileSynced event"),
        }
    }

    #[test]
    fn test_parse_json_log_ignores_directory_modtime() {
        let raw_json = r#"{"time":"2026-09-21T09:40:47.395205362+02:00","level":"info","msg":"Set directory modification time (using SetModTime)","object":"Cours/BUT_Info_S3"}"#;
        let events = parse_log_line(raw_json);
        assert!(events.is_empty(), "Directory modification time must NOT be treated as a synced file");

        let display_lines = format_log_line_for_display(raw_json);
        assert!(display_lines.is_empty(), "Directory modification time must NOT be displayed in logs");
    }

    #[test]
    fn test_parse_json_log_phases_and_diffs() {
        let json_diff = r#"{"time":"2026-09-21T09:40:47.000000000+02:00","level":"info","msg":"- \u001b[36mPath1\u001b[0m \u001b[35m\u001b[32mFile is new\u001b[0m\u001b[0m - \u001b[36mfile_a.txt\u001b[0m"}"#;
        let events = parse_log_line(json_diff);
        assert_eq!(events.len(), 1);
        match &events[0] {
            SyncEvent::DiffFound { is_local, detail } => {
                assert!(!is_local); // Path1 is remote
                assert_eq!(detail.path, "file_a.txt");
                assert_eq!(detail.action, FileAction::New);
            }
            _ => panic!("Expected DiffFound event"),
        }

        let json_done = r#"{"time":"2026-09-21T09:40:48.000000000+02:00","level":"info","msg":"Bisync successful"}"#;
        let done_events = parse_log_line(json_done);
        assert!(done_events.contains(&SyncEvent::PhaseChanged(SyncPhase::Done)));
        assert!(done_events.contains(&SyncEvent::SyncCompleted { success: true, error_msg: None }));
    }

    #[test]
    fn test_format_helpers() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.000 KiB");
        assert_eq!(format_bytes(1048576), "1.000 MiB");
        assert_eq!(format_bytes(1073741824), "1.000 GiB");

        assert_eq!(format_speed(0.0), "0 B/s");
        assert_eq!(format_speed(1048576.0), "1.000 MiB/s");

        assert_eq!(format_eta(0.0), "0s");
        assert_eq!(format_eta(45.0), "45s");
        assert_eq!(format_eta(125.0), "2m5s");
        assert_eq!(format_eta(3660.0), "1h1m");

        assert_eq!(format_elapsed(0.0), "0.0s");
        assert_eq!(format_elapsed(12.34), "12.3s");
        assert_eq!(format_elapsed(65.0), "1m5s");
    }

    #[test]
    fn test_parse_synced_files() {
        let l1 = "2026/09/17 21:30:00 INFO  : Documents/rapport.pdf: Copied (new)";
        let res1 = parse_synced_file(l1).expect("should parse new file");
        assert_eq!(res1.path, "Documents/rapport.pdf");
        assert_eq!(res1.action, FileAction::New);

        let l2 = "2026/09/17 21:30:01 INFO  : Images/photo.jpg: Copied (replaced existing)";
        let res2 = parse_synced_file(l2).expect("should parse modified file");
        assert_eq!(res2.path, "Images/photo.jpg");
        assert_eq!(res2.action, FileAction::Modified);

        let l3 = "2026/09/17 21:30:02 INFO  : Old/archive.zip: Deleted";
        let res3 = parse_synced_file(l3).expect("should parse deleted file");
        assert_eq!(res3.path, "Old/archive.zip");
        assert_eq!(res3.action, FileAction::Deleted);

        let l4 = "random irrelevant log line";
        assert!(parse_synced_file(l4).is_none());
    }

    #[test]
    fn test_parse_synced_files_future_log_formats_and_levels() {
        let l1 = "2026/09/17 21:30:00 NOTICE: Documents/rapport.pdf: Copied (new)";
        let res1 = parse_synced_file(l1).expect("should parse NOTICE log level");
        assert_eq!(res1.path, "Documents/rapport.pdf");
        assert_eq!(res1.action, FileAction::New);

        let l2 = "DEBUG : Images/photo.png: Updated file";
        let res2 = parse_synced_file(l2).expect("should parse DEBUG log level");
        assert_eq!(res2.path, "Images/photo.png");
        assert_eq!(res2.action, FileAction::Modified);

        let l3 = "2026-09-20T17:46:29.123456+02:00 INFO : music/track.flac: Deleted";
        let res3 = parse_synced_file(l3).expect("should parse ISO8601 timestamp");
        assert_eq!(res3.path, "music/track.flac");
        assert_eq!(res3.action, FileAction::Deleted);

        let l4 = "INFO: backup/data.tar.gz: Copied (new)";
        let res4 = parse_synced_file(l4).expect("should parse without timestamp");
        assert_eq!(res4.path, "backup/data.tar.gz");
        assert_eq!(res4.action, FileAction::New);
    }

    #[test]
    fn test_parse_transfer_stats() {
        let mut stats = TransferStats::default();

        parse_transfer_stats(
            "Transferred:   45.200 MiB / 120.500 MiB, 37%, 2.400 MiB/s, ETA 31s",
            &mut stats,
        );
        assert_eq!(stats.bytes_done, "45.200 mib");
        assert_eq!(stats.bytes_total, "120.500 mib");
        assert_eq!(stats.pct, 37);
        assert_eq!(stats.speed, "2.400 MiB/s");
        assert_eq!(stats.eta, "31s");

        parse_transfer_stats("Transferred:             4 / 12, 33%", &mut stats);
        assert_eq!(stats.files_done, 4);
        assert_eq!(stats.files_total, 12);

        parse_transfer_stats("Checks:                128 / 128, 100%", &mut stats);
        assert_eq!(stats.checks_done, 128);
        assert_eq!(stats.checks_total, 128);

        parse_transfer_stats("Elapsed time:        1m12.3s", &mut stats);
        assert_eq!(stats.elapsed, "1m12.3s");
    }

    #[test]
    fn test_parse_transfer_stats_edge_cases_and_future_variations() {
        let mut stats = TransferStats::default();

        parse_transfer_stats("TRANSFERRED: 1.250 GiB / 5.000 GiB, - %", &mut stats);
        assert_eq!(stats.bytes_done, "1.250 gib");
        assert_eq!(stats.bytes_total, "5.000 gib");
        assert_eq!(stats.pct, 0);

        parse_transfer_stats("transferred: 0 B / 0 B, -, 0 B/s, ETA -", &mut stats);
        assert_eq!(stats.bytes_done, "0 b");
        assert_eq!(stats.bytes_total, "0 b");
        assert_eq!(stats.speed, "0 B/s");
        assert_eq!(stats.eta, "-");

        parse_transfer_stats("transferred: 10 / 10, 100%, 1.250 GiB/s, ETA 0s", &mut stats);
        assert_eq!(stats.files_done, 10);
        assert_eq!(stats.files_total, 10);
        assert_eq!(stats.speed, "1.250 GiB/s");
        assert_eq!(stats.eta, "0s");

        parse_transfer_stats("checks: 0 / 500", &mut stats);
        assert_eq!(stats.checks_done, 0);
        assert_eq!(stats.checks_total, 500);

        parse_transfer_stats("", &mut stats);
        parse_transfer_stats("transferred: abc / def, xyz%", &mut stats);
        parse_transfer_stats("checks: NaN / Infinity", &mut stats);
    }

    #[test]
    fn test_parse_active_file() {
        let l1 = "* Documents/video.mp4: 45% /1.234Mi, 4.5Mi/s, 2m3s";
        let res1 = parse_active_file(l1).expect("should parse full active transfer");
        assert_eq!(res1.name, "Documents/video.mp4");
        assert_eq!(res1.pct, 45);
        assert_eq!(res1.speed, "4.5Mi/s");

        let l2 = "* Audio/album.flac: checking";
        let res2 = parse_active_file(l2).expect("should parse checking file");
        assert_eq!(res2.name, "Audio/album.flac");
        assert_eq!(res2.pct, 0);

        let l3 = "* Images/photo.png: 80% /500Ki";
        let res3 = parse_active_file(l3).expect("should parse short active transfer");
        assert_eq!(res3.name, "Images/photo.png");
        assert_eq!(res3.pct, 80);

        assert!(parse_active_file("").is_none());
        assert!(parse_active_file("* ").is_none());
        assert!(parse_active_file("* invalid line without percentage").is_none());
    }

    #[test]
    fn test_resync_trigger() {
        assert!(is_resync_trigger("ERROR : Bisync error: must run --resync to recover"));
        assert!(is_resync_trigger("Fatal: Path1 and Path2 are out of sync"));
        assert!(is_resync_trigger("prior or current is not in sync"));
        assert!(!is_resync_trigger("INFO  : Bisync successful"));
        assert!(!is_resync_trigger(""));
    }
}
