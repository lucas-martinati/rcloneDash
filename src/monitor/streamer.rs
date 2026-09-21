use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::RwLock;

pub use crate::monitor::events::{
    ActiveFile, ModifiedFileDetail, SyncEvent, SyncedFile, TransferStats,
};
use crate::monitor::parser::{format_log_line_for_display, parse_log_line};
pub use crate::monitor::parser::strip_ansi;

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
        // Nouvelle observation : un resync requis ne vaut que pour le run
        // précédent (l'erreur refera surface si elle persiste). On ne
        // l'efface volontairement PAS dans mark_finished pour ne pas masquer
        // une erreur critique du run qui vient de se terminer.
        self.resync_needed = false;
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
        // 3. Avancement dynamique selon la phase du pipeline bisync
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

    /// Applique un événement universel de synchronisation à l'état du streamer
    pub fn apply_event(&mut self, event: SyncEvent) {
        match event {
            SyncEvent::SyncStarted { .. } => {
                self.reset_for_new_sync();
            }
            SyncEvent::PhaseChanged(phase) => {
                self.phase = phase.display_name().to_string();
                self.phase_index = phase.index();
            }
            SyncEvent::PathModified { is_local } => {
                if is_local {
                    self.path2_modified = true;
                } else {
                    self.path1_modified = true;
                }
            }
            SyncEvent::StatsUpdated(stats) => {
                if !self.is_syncing {
                    self.reset_for_new_sync();
                }
                if !stats.speed.is_empty() {
                    let speed_val = parse_speed_kibs(&stats.speed);
                    if speed_val > 0 {
                        if self.speed_history.len() >= 40 {
                            self.speed_history.remove(0);
                        }
                        self.speed_history.push(speed_val);
                    }
                }
                self.transfer = stats;
            }
            SyncEvent::ActiveTransferUpdated(active) => {
                if !self.is_syncing {
                    self.reset_for_new_sync();
                }
                self.active_files.insert(active.name.clone(), active);
            }
            SyncEvent::FileSynced(synced) => {
                if !self.synced_files.iter().any(|f| f.path == synced.path) {
                    self.synced_files.push(synced);
                }
            }
            SyncEvent::DiffFound { is_local, detail } => {
                if is_local {
                    self.path2_modified = true;
                    if !self.changes_local.contains(&detail.path) {
                        self.changes_local.push(detail.path.clone());
                    }
                    if let Some(existing) = self.changes_local_details.iter_mut().find(|d| d.path == detail.path) {
                        existing.action = detail.action;
                    } else {
                        self.changes_local_details.push(detail);
                    }
                } else {
                    self.path1_modified = true;
                    if !self.changes_remote.contains(&detail.path) {
                        self.changes_remote.push(detail.path.clone());
                    }
                    if let Some(existing) = self.changes_remote_details.iter_mut().find(|d| d.path == detail.path) {
                        existing.action = detail.action;
                    } else {
                        self.changes_remote_details.push(detail);
                    }
                }
            }
            SyncEvent::ResyncRequired(_) => {
                self.resync_needed = true;
            }
            SyncEvent::SyncCompleted { .. } => {
                self.mark_finished();
            }
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
                    for disp in format_log_line_for_display(&clean) {
                        if initial_state.log_lines.len() >= 500 {
                            initial_state.log_lines.pop_front();
                        }
                        initial_state.log_lines.push_back(disp);
                    }
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
    // Formatage propre pour affichage dans le panneau des logs
    for disp in format_log_line_for_display(line) {
        if state.log_lines.len() >= 500 {
            state.log_lines.pop_front();
        }
        state.log_lines.push_back(disp);
    }

    // Consommation des événements normalisés
    let events = parse_log_line(line);
    for event in events {
        state.apply_event(event);
    }

    // Nettoyage des fichiers actifs expirés (> 4s)
    let now = Instant::now();
    state.active_files.retain(|_, v| now.duration_since(v.last_seen).as_secs_f32() < 4.0);
}

pub fn parse_speed_kibs(speed_str: &str) -> u64 {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitor::events::FileAction;
    use crate::monitor::parser::parse_diff_file;

    #[test]
    fn test_strip_ansi() {
        let raw = "\x1b[34mPath2\x1b[0m \x1b[35m\x1b[31mFile was deleted\x1b[0m\x1b[0m - \x1b[36maaa/song.mp3\x1b[0m\r";
        assert_eq!(strip_ansi(raw), "Path2 File was deleted - aaa/song.mp3");

        let rgb = "\x1b[38;2;255;100;50mColored\x1b[0m text\x1b[2K\r";
        assert_eq!(strip_ansi(rgb), "Colored text");

        let osc = "\x1b]0;rclone-title\x07Clean text";
        assert_eq!(strip_ansi(osc), "Clean text");
    }

    #[test]
    fn test_parse_diff_file_with_ansi_codes() {
        let raw = "INFO  : - \x1b[34mPath2\x1b[0m    \x1b[35m\x1b[31mFile was deleted\x1b[0m\x1b[0m          - \x1b[36maaa/réveil/AH CA NN PAS DARABES DANS MA FRANCE.mp3\x1b[0m";
        let res = parse_diff_file(raw).unwrap();
        assert!(res.0); // is_local
        assert_eq!(res.1.path, "aaa/réveil/AH CA NN PAS DARABES DANS MA FRANCE.mp3");
        assert_eq!(res.1.action, FileAction::Deleted);
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
        assert_eq!(res1.1.action, FileAction::New);

        let l2 = "Path2: file changed - /home/lucas-m54/GoogleDrive/config.json";
        let res2 = parse_diff_file(l2).unwrap();
        assert!(res2.0); // Path2 is local
        assert_eq!(res2.1.path, "/home/lucas-m54/GoogleDrive/config.json");
        assert_eq!(res2.1.action, FileAction::Modified);

        let l3 = "Path1: queue copy to Path2 - notes.txt";
        let res3 = parse_diff_file(l3).unwrap();
        assert!(!res3.0);
        assert_eq!(res3.1.path, "notes.txt");
        assert_eq!(res3.1.action, FileAction::Copied);

        let l4 = "Path2: file was deleted - old_backup.zip";
        let res4 = parse_diff_file(l4).unwrap();
        assert!(res4.0);
        assert_eq!(res4.1.path, "old_backup.zip");
        assert_eq!(res4.1.action, FileAction::Deleted);

        let l5 = "- Path2    File is new               - AAAA/réveil/Debout - Leo Succulent.mp3";
        let res5 = parse_diff_file(l5).unwrap();
        assert!(res5.0);
        assert_eq!(res5.1.path, "AAAA/réveil/Debout - Leo Succulent.mp3");
        assert_eq!(res5.1.action, FileAction::New);

        let l6 = "- Path1    Queue delete              - GoogleDrive{hruw5}:/AAAA/réveil/Debout - Leo Succulent.mp3";
        let res6 = parse_diff_file(l6).unwrap();
        assert!(!res6.0);
        assert_eq!(res6.1.path, "AAAA/réveil/Debout - Leo Succulent.mp3");
        assert_eq!(res6.1.action, FileAction::Deleted);
    }

    #[test]
    fn test_parse_diff_file_resilience_and_edge_cases() {
        let l1 = "- Path2    File is new               - musique/🎉 fête & café - soirée été 2026.mp3";
        let res1 = parse_diff_file(l1).expect("should parse unicode emoji filename");
        assert!(res1.0);
        assert_eq!(res1.1.path, "musique/🎉 fête & café - soirée été 2026.mp3");
        assert_eq!(res1.1.action, FileAction::New);

        let l2 = "path2: file deleted - archive.tar";
        let res2 = parse_diff_file(l2).expect("should parse lowercase path2 and 'file deleted'");
        assert!(res2.0);
        assert_eq!(res2.1.path, "archive.tar");
        assert_eq!(res2.1.action, FileAction::Deleted);

        let l3 = "Path1: Queue copy to Path2 - remote:/folder - sub - file - name.txt";
        let res3 = parse_diff_file(l3).expect("should parse file with multiple dashes");
        assert!(!res3.0);
        assert_eq!(res3.1.path, "folder - sub - file - name.txt");
        assert_eq!(res3.1.action, FileAction::Copied);

        assert_eq!(parse_diff_file(""), None);
        assert_eq!(parse_diff_file("Path1: file is new"), None);
        assert_eq!(parse_diff_file("Path2: file changed - "), None);
        assert_eq!(parse_diff_file("random line without paths"), None);
        assert_eq!(parse_diff_file("\x00\x01\x02\u{FF}\u{FE}"), None);
    }

    #[test]
    fn test_streamer_parse_stream_line_robustness() {
        let mut st = StreamerState::default();

        parse_stream_line("", &mut st);
        parse_stream_line("   ", &mut st);
        parse_stream_line("DEBUG: random log with no meaning", &mut st);
        parse_stream_line("Building path1 and path2 listings", &mut st);
        assert!(st.is_syncing);
        assert_eq!(st.phase_index, 0);

        parse_stream_line("Path1: checking for diffs", &mut st);
        assert_eq!(st.phase_index, 1);

        parse_stream_line("Path2: checking for diffs", &mut st);
        assert_eq!(st.phase_index, 2);

        parse_stream_line("Applying changes", &mut st);
        assert_eq!(st.phase_index, 3);

        parse_stream_line("Updating listings", &mut st);
        assert_eq!(st.phase_index, 4);

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
            action: FileAction::New,
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
