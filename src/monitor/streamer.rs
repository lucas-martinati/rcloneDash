use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::RwLock;
use crate::monitor::parser::{
    parse_active_file, parse_synced_file, parse_transfer_stats, is_resync_trigger,
    ActiveFile, SyncedFile, TransferStats,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModifiedFileDetail {
    pub path: String,
    pub action: String, // "nouveau", "modifié", "supprimé"
}

#[derive(Debug, Clone)]
pub struct StreamerState {
    pub is_syncing: bool,
    pub sync_start: Option<Instant>,
    pub phase: String,
    pub phase_index: usize,
    pub transfer: TransferStats,
    pub active_files: HashMap<String, ActiveFile>,
    pub synced_files: Vec<SyncedFile>,
    pub log_lines: VecDeque<String>,
    pub resync_needed: bool,
    pub speed_history: Vec<u64>,
    pub changes_local: Vec<String>,
    pub changes_remote: Vec<String>,
    pub changes_local_details: Vec<ModifiedFileDetail>,
    pub changes_remote_details: Vec<ModifiedFileDetail>,
    pub path1_modified: bool,
    pub path2_modified: bool,
}

impl StreamerState {
    pub fn reset_for_new_sync(&mut self) {
        self.is_syncing = true;
        self.sync_start = Some(Instant::now());
        self.phase = "1. Listings".to_string();
        self.phase_index = 0;
        self.transfer = TransferStats::default();
        self.active_files.clear();
        self.synced_files.clear();
        self.changes_local.clear();
        self.changes_remote.clear();
        self.changes_local_details.clear();
        self.changes_remote_details.clear();
        self.path1_modified = false;
        self.path2_modified = false;
    }

    pub fn mark_finished(&mut self) {
        self.is_syncing = false;
        self.sync_start = None;
        self.phase = "6. Done".to_string();
        self.phase_index = 5;
        self.transfer = TransferStats::default();
        self.active_files.clear();
        self.synced_files.clear();
        self.changes_local.clear();
        self.changes_remote.clear();
        self.changes_local_details.clear();
        self.changes_remote_details.clear();
        self.path1_modified = false;
        self.path2_modified = false;
    }

    pub fn overall_progress_pct(&self) -> u8 {
        if !self.is_syncing {
            return 0;
        }
        if self.phase_index >= 5 {
            return 100;
        }
        // 1. Pourcentage réel rapporté par rclone pour les données transférées
        if self.transfer.pct > 0 {
            return self.transfer.pct.clamp(1, 99);
        }
        // 2. Pourcentage basé sur les fichiers transférés
        if self.transfer.files_total > 0 && self.transfer.files_done > 0 {
            let file_pct = ((self.transfer.files_done as f64 / self.transfer.files_total as f64) * 100.0) as u8;
            if file_pct > 0 {
                return file_pct.clamp(1, 99);
            }
        }
        // 3. Avancement dynamique selon la phase du pipeline bisync (sans saut intempestif sur un fichier actif)
        match self.phase_index {
            0 => {
                if let Some(start) = self.sync_start {
                    let secs = start.elapsed().as_secs();
                    (10 + secs.min(10)).min(20) as u8
                } else {
                    10
                }
            }
            1 => 25,
            2 => {
                if self.transfer.checks_total > 0 && self.transfer.checks_done > 0 {
                    let check_ratio = self.transfer.checks_done as f64 / self.transfer.checks_total as f64;
                    (40.0 + check_ratio * 20.0).round() as u8
                } else {
                    50
                }
            }
            3 => 70,
            4 => 95,
            _ => 100,
        }
    }
}

impl Default for StreamerState {
    fn default() -> Self {
        Self {
            is_syncing: false,
            sync_start: None,
            phase: "Idle".to_string(),
            phase_index: 0,
            transfer: TransferStats::default(),
            active_files: HashMap::new(),
            synced_files: Vec::new(),
            log_lines: VecDeque::with_capacity(600),
            resync_needed: false,
            speed_history: vec![0; 40],
            changes_local: Vec::new(),
            changes_remote: Vec::new(),
            changes_local_details: Vec::new(),
            changes_remote_details: Vec::new(),
            path1_modified: false,
            path2_modified: false,
        }
    }
}

pub type SharedStreamer = Arc<RwLock<StreamerState>>;

pub fn get_running_sync_elapsed_seconds() -> Option<u64> {
    let output = std::process::Command::new("ps")
        .args(["-eo", "etimes,args"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.contains("rclone") && (line.contains("bisync") || line.contains("sync"))
            && !line.contains("grep") && !line.contains("journalctl") && !line.contains("rclonedash")
        {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(first) = parts.first() {
                if let Ok(secs) = first.parse::<u64>() {
                    return Some(secs);
                }
            }
        }
    }
    None
}

pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if let Some(&next) = chars.peek() {
                if next == '[' {
                    chars.next(); // consume '['
                    for code in chars.by_ref() {
                        if (0x40..=0x7E).contains(&(code as u32)) {
                            break;
                        }
                    }
                } else if next == ']' {
                    chars.next(); // consume ']'
                    while let Some(osc) = chars.next() {
                        if osc == '\x07' || (osc == '\x1b' && chars.peek() == Some(&'\\')) {
                            if osc == '\x1b' {
                                chars.next();
                            }
                            break;
                        }
                    }
                } else if next == '(' || next == ')' {
                    chars.next(); // consume '(' or ')'
                    chars.next(); // consume charset
                }
            }
        } else if c != '\r' {
            out.push(c);
        }
    }
    out
}

pub fn spawn_log_streamer() -> SharedStreamer {
    let mut initial_state = StreamerState::default();
    let service = "rclone-bisync.service";

    // 1. Pré-remplir le tampon d'historique des logs (200 dernières lignes)
    if let Ok(out) = std::process::Command::new("journalctl")
        .args(["--user", "-u", service, "-n", "200", "--output=cat"])
        .output()
    {
        let text = String::from_utf8_lossy(&out.stdout);
        for raw_line in text.lines() {
            for line in raw_line.split('\r') {
                let clean = strip_ansi(line).trim().to_string();
                if !clean.is_empty() {
                    if initial_state.log_lines.len() >= 500 {
                        initial_state.log_lines.pop_front();
                    }
                    initial_state.log_lines.push_back(clean);
                }
            }
        }
    }

    // 2. Initialisation de l'état : vérifier si rclone tourne déjà en ce moment
    let is_running = {
        let out = std::process::Command::new("systemctl")
            .args(["--user", "is-active", service])
            .output();
        if let Ok(o) = out {
            String::from_utf8_lossy(&o.stdout).trim() == "active"
        } else {
            false
        }
    };

    if is_running {
        initial_state.is_syncing = true;
        let elapsed_ps = get_running_sync_elapsed_seconds().unwrap_or(0);
        initial_state.sync_start = Some(Instant::now().checked_sub(std::time::Duration::from_secs(elapsed_ps)).unwrap_or_else(Instant::now));
        initial_state.phase = "1. Listings".to_string();
        initial_state.phase_index = 0;
    }

    let state = Arc::new(RwLock::new(initial_state));

    // 3. Tâche de streaming en temps réel (uniquement les nouvelles lignes avec -n 0)
    let state_clone = Arc::clone(&state);
    tokio::spawn(async move {
        loop {
            let mut child = match tokio::process::Command::new("journalctl")
                .args(["--user", "-u", service, "-f", "-n", "0", "--output=cat"])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .spawn()
            {
                Ok(c) => c,
                Err(_) => {
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    continue;
                }
            };

            if let Some(stdout) = child.stdout.take() {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();

                while let Ok(Some(raw_line)) = lines.next_line().await {
                    for line in raw_line.split('\r') {
                        let clean = strip_ansi(line).trim().to_string();
                        if !clean.is_empty() {
                            let mut st = state_clone.write().await;
                            parse_stream_line(&clean, &mut st);
                        }
                    }
                }
            }

            let _ = child.wait().await;
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    });

    state
}

fn parse_stream_line(line: &str, state: &mut StreamerState) {
    let ll = line.to_lowercase();

    // Ajout à l'historique des logs (tampon circulaire 500 lignes)
    if state.log_lines.len() >= 500 {
        state.log_lines.pop_front();
    }
    state.log_lines.push_back(line.to_string());

    // Détection début de synchronisation
    let is_start = (ll.contains("systemd") && (ll.contains("starting") || ll.contains("started")) && ll.contains("rclone-bisync"))
        || (ll.contains("rclone-bisync-guard") && ll.contains("lancement du bisync"))
        || ll.contains("rclonedash: lancement du bisync")
        || ll.contains("synching path1")
        || ll.contains("bisyncing with")
        || ll.contains("building path1 and path2 listings");

    if is_start {
        state.reset_for_new_sync();
        return;
    }

    // Détection d'activité sync si is_syncing n'a pas encore été basculé
    if !state.is_syncing && (
        ll.contains("transferred:") || ll.contains("checks:") || ll.contains("transferring:")
        || ll.contains("copying path") || ll.contains("elapsed time:")
        || ll.contains("building path1 and path2 listings")
        || ll.contains("checking for diffs")
    ) {
        state.reset_for_new_sync();
    }

    // Détection si un chemin complet est marqué comme modifié
    if ll.contains("path1 was modified") || ll.contains("path1: path was modified") || ll.contains("differences found on path1") {
        state.path1_modified = true;
    }
    if ll.contains("path2 was modified") || ll.contains("path2: path was modified") || ll.contains("differences found on path2") {
        state.path2_modified = true;
    }

    // Détection de la phase du pipeline
    if ll.contains("updating listings") || ll.contains("updating path") {
        state.phase = "5. Updating".to_string();
        state.phase_index = 4;
    } else if (
        ll.contains("applying changes")
        || ll.contains("synching path1 to path2")
        || ll.contains("synching path2 to path1")
        || (ll.contains("copying") && !ll.contains("copying path") && !ll.contains("queue copy"))
        || ll.contains("copied (")
        || ll.contains("deleted (")
        || state.transfer.files_done > 0
        || (state.transfer.files_total > 0 && state.transfer.pct > 0)
        || !state.active_files.is_empty()
    ) && state.phase_index < 3 {
        state.phase = "4. Applying".to_string();
        state.phase_index = 3;
    } else if (
        ll.contains("path2 checking for diffs")
        || ll.contains("path2: checking")
        || ll.contains("validating listings for path2")
        || ll.contains("differences found on path2")
        || ll.contains("path2 was modified")
    ) && state.phase_index < 3 {
        state.phase = "3. Local Diffs".to_string();
        state.phase_index = 2;
    } else if (
        ll.contains("path1 checking for diffs")
        || ll.contains("path1: checking")
        || ll.contains("validating listings for path1")
        || ll.contains("differences found on path1")
        || ll.contains("path1 was modified")
    ) && state.phase_index < 2 {
        state.phase = "2. Remote Diffs".to_string();
        state.phase_index = 1;
    }

    // Parsing métriques de transfert
    parse_transfer_stats(line, &mut state.transfer);

    // Extraction de la vitesse pour le Sparkline
    if !state.transfer.speed.is_empty() {
        let speed_val = parse_speed_kibs(&state.transfer.speed);
        if speed_val > 0 {
            if state.speed_history.len() >= 40 {
                state.speed_history.remove(0);
            }
            state.speed_history.push(speed_val);
        }
    }

    // Fichiers actifs
    if let Some(active) = parse_active_file(line) {
        state.active_files.insert(active.name.clone(), active);
    }

    // Fichier synchronisé
    if let Some(synced) = parse_synced_file(line) {
        if !state.synced_files.iter().any(|f| f.path == synced.path) {
            state.synced_files.push(synced);
        }
    }

    // Détection besoin de resync critique
    if is_resync_trigger(line) {
        state.resync_needed = true;
    }

    // Changements détectés Path1 (distant) / Path2 (local) avec détail précis
    if let Some((is_local, detail)) = parse_diff_file(line) {
        if is_local {
            state.path2_modified = true;
            if !state.changes_local.contains(&detail.path) {
                state.changes_local.push(detail.path.clone());
            }
            if let Some(existing) = state.changes_local_details.iter_mut().find(|d| d.path == detail.path) {
                existing.action = detail.action;
            } else {
                state.changes_local_details.push(detail);
            }
        } else {
            state.path1_modified = true;
            if !state.changes_remote.contains(&detail.path) {
                state.changes_remote.push(detail.path.clone());
            }
            if let Some(existing) = state.changes_remote_details.iter_mut().find(|d| d.path == detail.path) {
                existing.action = detail.action;
            } else {
                state.changes_remote_details.push(detail);
            }
        }
    }

    // Fin de synchronisation
    let is_finished = ll.contains("bisync successful")
        || (ll.contains("systemd") && (ll.contains("finished") || ll.contains("stopped") || ll.contains("deactivated")) && ll.contains("rclone-bisync"))
        || (ll.contains("rclone-bisync-guard") && ll.contains("aucun changement"))
        || (ll.contains("systemd") && ll.contains("failed") && ll.contains("rclone-bisync"))
        || ll.contains("bisync error:")
        || ll.contains("bisync aborted");

    if is_finished {
        state.mark_finished();
    }

    // Nettoyage des fichiers actifs expirés (> 4s)
    let now = Instant::now();
    state.active_files.retain(|_, v| now.duration_since(v.last_seen).as_secs_f32() < 4.0);
}

fn parse_speed_kibs(speed_str: &str) -> u64 {
    // Exemples: "1.234 MiB/s", "500 KiB/s", "2.100 GiB/s", "100 B/s"
    let parts: Vec<&str> = speed_str.split_whitespace().collect();
    if parts.is_empty() {
        return 0;
    }
    let val: f64 = parts[0].parse().unwrap_or(0.0);
    let unit = if parts.len() > 1 { parts[1].to_lowercase() } else { "".to_string() };

    if unit.contains("gib") {
        (val * 1024.0 * 1024.0) as u64
    } else if unit.contains("mib") {
        (val * 1024.0) as u64
    } else if unit.contains("kib") {
        val as u64
    } else {
        (val / 1024.0) as u64
    }
}

fn parse_diff_file(raw_line: &str) -> Option<(bool, ModifiedFileDetail)> {
    let clean_line = strip_ansi(raw_line);
    let ll = clean_line.to_ascii_lowercase();
    if !ll.contains("path1") && !ll.contains("path2") {
        return None;
    }

    // Must be an actual file change line from rclone bisync
    let (action, action_str) = if ll.contains("file is new") {
        ("new".to_string(), "file is new")
    } else if ll.contains("file changed") {
        ("modified".to_string(), "file changed")
    } else if ll.contains("file was deleted") {
        ("deleted".to_string(), "file was deleted")
    } else if ll.contains("file deleted") {
        ("deleted".to_string(), "file deleted")
    } else if ll.contains("queue copy") {
        ("copied".to_string(), "queue copy")
    } else if ll.contains("queue delete") {
        ("deleted".to_string(), "queue delete")
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

    Some((is_local, ModifiedFileDetail {
        path: fname.to_string(),
        action,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_ansi() {
        let raw = "\x1b[34mPath2\x1b[0m \x1b[35m\x1b[31mFile was deleted\x1b[0m\x1b[0m - \x1b[36maaa/song.mp3\x1b[0m\r";
        assert_eq!(strip_ansi(raw), "Path2 File was deleted - aaa/song.mp3");

        // 256-color and RGB codes
        let rgb = "\x1b[38;2;255;100;50mColored\x1b[0m text\x1b[2K\r";
        assert_eq!(strip_ansi(rgb), "Colored text");

        // OSC sequence
        let osc = "\x1b]0;rclone-title\x07Clean text";
        assert_eq!(strip_ansi(osc), "Clean text");
    }

    #[test]
    fn test_parse_diff_file_with_ansi_codes() {
        let raw = "INFO  : - \x1b[34mPath2\x1b[0m    \x1b[35m\x1b[31mFile was deleted\x1b[0m\x1b[0m          - \x1b[36maaa/réveil/AH CA NN PAS DARABES DANS MA FRANCE.mp3\x1b[0m";
        let res = parse_diff_file(raw).unwrap();
        assert!(res.0); // is_local
        assert_eq!(res.1.path, "aaa/réveil/AH CA NN PAS DARABES DANS MA FRANCE.mp3");
        assert_eq!(res.1.action, "deleted");
    }

    #[test]
    fn test_parse_diff_file_rejects_matching_and_synching_headers() {
        let l1 = "Matching Path1 \"GoogleDrive:\" vs Path2 \"/home/lucas-m54/GoogleDrive/\"";
        let l2 = "Synching Path1 \"GoogleDrive:\" with Path2 \"/home/lucas-m54/GoogleDrive/\"";
        assert_eq!(parse_diff_file(l1), None);
        assert_eq!(parse_diff_file(l2), None);
    }

    #[test]
    fn test_parse_diff_file_valid_entries() {
        let l1 = "Path1: file is new - installer_wifi_lorraine.py";
        let res1 = parse_diff_file(l1).unwrap();
        assert!(!res1.0); // Path1 is remote
        assert_eq!(res1.1.path, "installer_wifi_lorraine.py");
        assert_eq!(res1.1.action, "new");

        let l2 = "Path2: file changed - /home/lucas-m54/GoogleDrive/config.json";
        let res2 = parse_diff_file(l2).unwrap();
        assert!(res2.0); // Path2 is local
        assert_eq!(res2.1.path, "/home/lucas-m54/GoogleDrive/config.json");
        assert_eq!(res2.1.action, "modified");

        let l3 = "Path1: queue copy to Path2 - notes.txt";
        let res3 = parse_diff_file(l3).unwrap();
        assert!(!res3.0);
        assert_eq!(res3.1.path, "notes.txt");
        assert_eq!(res3.1.action, "copied");

        let l4 = "Path2: file was deleted - old_backup.zip";
        let res4 = parse_diff_file(l4).unwrap();
        assert!(res4.0);
        assert_eq!(res4.1.path, "old_backup.zip");
        assert_eq!(res4.1.action, "deleted");

        // Filename containing dashes inside the name
        let l5 = "- Path2    File is new               - AAAA/réveil/Debout - Leo Succulent.mp3";
        let res5 = parse_diff_file(l5).unwrap();
        assert!(res5.0);
        assert_eq!(res5.1.path, "AAAA/réveil/Debout - Leo Succulent.mp3");
        assert_eq!(res5.1.action, "new");

        // Remote queue delete with remote prefix
        let l6 = "- Path1    Queue delete              - GoogleDrive{hruw5}:/AAAA/réveil/Debout - Leo Succulent.mp3";
        let res6 = parse_diff_file(l6).unwrap();
        assert!(!res6.0);
        assert_eq!(res6.1.path, "AAAA/réveil/Debout - Leo Succulent.mp3");
        assert_eq!(res6.1.action, "deleted");
    }

    #[test]
    fn test_parse_diff_file_resilience_and_edge_cases() {
        // Unicode and special characters in path
        let l1 = "- Path2    File is new               - musique/🎉 fête & café - soirée été 2026.mp3";
        let res1 = parse_diff_file(l1).expect("should parse unicode emoji filename");
        assert!(res1.0);
        assert_eq!(res1.1.path, "musique/🎉 fête & café - soirée été 2026.mp3");
        assert_eq!(res1.1.action, "new");

        // Lowercase path2 and alternative phrasing "file deleted"
        let l2 = "path2: file deleted - archive.tar";
        let res2 = parse_diff_file(l2).expect("should parse lowercase path2 and 'file deleted'");
        assert!(res2.0);
        assert_eq!(res2.1.path, "archive.tar");
        assert_eq!(res2.1.action, "deleted");

        // Multiple dashes in filename with remote prefix
        let l3 = "Path1: Queue copy to Path2 - remote:/folder - sub - file - name.txt";
        let res3 = parse_diff_file(l3).expect("should parse file with multiple dashes");
        assert!(!res3.0);
        assert_eq!(res3.1.path, "folder - sub - file - name.txt");
        assert_eq!(res3.1.action, "copied");

        // Malformed lines must never panic and return None gracefully
        assert_eq!(parse_diff_file(""), None);
        assert_eq!(parse_diff_file("Path1: file is new"), None);
        assert_eq!(parse_diff_file("Path2: file changed - "), None);
        assert_eq!(parse_diff_file("random line without paths"), None);
        assert_eq!(parse_diff_file("\x00\x01\x02\u{FF}\u{FE}"), None);
    }

    #[test]
    fn test_streamer_parse_stream_line_robustness() {
        let mut st = StreamerState::default();

        // Feed various lines including garbage and verify no panics
        parse_stream_line("", &mut st);
        parse_stream_line("   ", &mut st);
        parse_stream_line("DEBUG: random log with no meaning", &mut st);
        parse_stream_line("Building path1 and path2 listings", &mut st);
        assert!(st.is_syncing);
        assert_eq!(st.phase_index, 0);

        // Path 1 diffs
        parse_stream_line("Path1: checking for diffs", &mut st);
        assert_eq!(st.phase_index, 1);

        // Path 2 diffs
        parse_stream_line("Path2: checking for diffs", &mut st);
        assert_eq!(st.phase_index, 2);

        // Applying changes
        parse_stream_line("Applying changes", &mut st);
        assert_eq!(st.phase_index, 3);

        // Updating listings
        parse_stream_line("Updating listings", &mut st);
        assert_eq!(st.phase_index, 4);

        // Done
        parse_stream_line("Bisync successful", &mut st);
        assert!(!st.is_syncing);
        assert_eq!(st.phase_index, 5);
    }

    #[test]
    fn test_streamer_state_reset_lifecycle() {
        let mut st = StreamerState {
            is_syncing: true,
            sync_start: Some(Instant::now()),
            path2_modified: true,
            ..Default::default()
        };
        st.changes_local_details.push(ModifiedFileDetail {
            path: "test.txt".to_string(),
            action: "new".to_string(),
        });
        st.mark_finished();
        assert!(!st.is_syncing);
        assert!(st.sync_start.is_none());
        assert!(!st.path2_modified);
        assert!(st.changes_local_details.is_empty());
        assert_eq!(st.phase_index, 5);

        st.reset_for_new_sync();
        assert!(st.is_syncing);
        assert!(st.sync_start.is_some());
        assert_eq!(st.phase_index, 0);
    }

    #[test]
    fn test_parse_speed_kibs() {
        assert_eq!(parse_speed_kibs("1.5 MiB/s"), 1536);
        assert_eq!(parse_speed_kibs("500 KiB/s"), 500);
        assert_eq!(parse_speed_kibs("1 GiB/s"), 1048576);
        assert_eq!(parse_speed_kibs(""), 0);
    }
}
