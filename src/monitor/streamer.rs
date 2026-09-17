use std::collections::{HashMap, VecDeque};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::RwLock;
use crate::monitor::parser::{
    parse_active_file, parse_synced_file, parse_transfer_stats, is_resync_trigger,
    ActiveFile, SyncedFile, TransferStats,
};

#[allow(dead_code)]
pub const PHASES: &[&str] = &[
    "1. Listings",
    "2. Diffs Locaux",
    "3. Diffs Distants",
    "4. Application",
    "5. Mise à jour",
    "6. Terminé",
];

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
}

impl Default for StreamerState {
    fn default() -> Self {
        Self {
            is_syncing: false,
            sync_start: None,
            phase: "En attente".to_string(),
            phase_index: 0,
            transfer: TransferStats::default(),
            active_files: HashMap::new(),
            synced_files: Vec::new(),
            log_lines: VecDeque::with_capacity(600),
            resync_needed: false,
            speed_history: vec![0; 40],
        }
    }
}

pub type SharedStreamer = Arc<RwLock<StreamerState>>;

pub fn spawn_log_streamer() -> SharedStreamer {
    let state = Arc::new(RwLock::new(StreamerState::default()));
    let state_clone = Arc::clone(&state);

    tokio::spawn(async move {
        loop {
            let mut cmd = Command::new("journalctl");
            cmd.args([
                "--user",
                "-f",
                "-u",
                "rclone-bisync.service",
                "--output=short-iso",
                "-n",
                "200",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

            match cmd.spawn() {
                Ok(mut child) => {
                    if let Some(stdout) = child.stdout.take() {
                        let mut reader = BufReader::new(stdout).lines();
                        while let Ok(Some(line)) = reader.next_line().await {
                            let trimmed = line.trim();
                            if trimmed.is_empty() {
                                continue;
                            }
                            let mut st = state_clone.write().await;
                            parse_stream_line(trimmed, &mut st);
                        }
                    }
                    let _ = child.wait().await;
                }
                Err(_) => {
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
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
        || ll.contains("synching path1 with path2")
        || ll.contains("bisyncing with");

    if is_start {
        if !state.is_syncing {
            state.is_syncing = true;
            state.sync_start = Some(Instant::now());
            state.phase = "1. Listings".to_string();
            state.phase_index = 0;
            state.active_files.clear();
            state.synced_files.clear();
            state.transfer = TransferStats::default();
        }
        return;
    }

    // Détection d'activité sync si is_syncing n'a pas encore été basculé
    if !state.is_syncing && (
        ll.contains("transferred:") || ll.contains("checks:") || ll.contains("transferring:")
        || ll.contains("copying path") || ll.contains("elapsed time:")
    ) {
        state.is_syncing = true;
        state.phase = "4. Application".to_string();
        state.phase_index = 3;
        if state.sync_start.is_none() {
            state.sync_start = Some(Instant::now());
        }
    }

    // Détection de la phase du pipeline
    if ll.contains("updating listings") || ll.contains("updating path") {
        state.phase = "5. Mise à jour".to_string();
        state.phase_index = 4;
    } else if ll.contains("transferring:") || ll.contains("transferred:") || ll.contains("copying") || ll.contains("copied (") {
        if state.phase_index < 3 {
            state.phase = "4. Application".to_string();
            state.phase_index = 3;
        }
    } else if ll.contains("path2: checking") || ll.contains("path2: matching") || ll.contains("validating listings for path2") {
        if state.phase_index < 2 {
            state.phase = "3. Diffs Distants".to_string();
            state.phase_index = 2;
        }
    } else if ll.contains("path1: checking") || ll.contains("path1: matching") || ll.contains("validating listings for path1") {
        if state.phase_index < 1 {
            state.phase = "2. Diffs Locaux".to_string();
            state.phase_index = 1;
        }
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

    // Fin de synchronisation
    if ll.contains("bisync successful") {
        state.phase = "6. Terminé (Succès)".to_string();
        state.phase_index = 5;
        state.is_syncing = false;
        state.active_files.clear();
    } else if (ll.contains("systemd") && ll.contains("finished") && ll.contains("rclone-bisync"))
        || (ll.contains("rclone-bisync-guard") && ll.contains("aucun changement")) {
        state.is_syncing = false;
        state.active_files.clear();
    } else if ll.contains("systemd") && ll.contains("failed") && ll.contains("rclone-bisync") {
        state.is_syncing = false;
        state.active_files.clear();
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
    let unit = parts.get(1).unwrap_or(&"").to_lowercase();

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
