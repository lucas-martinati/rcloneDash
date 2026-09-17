use std::time::Instant;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use crate::config::{self, AppConfig};
use crate::fs_tree::{self, FileEntry};
use crate::monitor::{fetch_past_runs, spawn_log_streamer, PastRun, RunStatus, SharedStreamer, StreamerState};
use crate::systemd::{self, get_service_info, ServiceInfo, ServiceState};
use crate::ui::settings::SETTINGS_ITEMS_COUNT;
use crate::ui::theme::ThemeChoice;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPanel {
    History,
    Logs,
    RecentFiles,
}

impl FocusedPanel {
    pub fn next(self) -> Self {
        match self {
            FocusedPanel::History => FocusedPanel::Logs,
            FocusedPanel::Logs => FocusedPanel::RecentFiles,
            FocusedPanel::RecentFiles => FocusedPanel::History,
        }
    }
    pub fn prev(self) -> Self {
        match self {
            FocusedPanel::History => FocusedPanel::RecentFiles,
            FocusedPanel::Logs => FocusedPanel::History,
            FocusedPanel::RecentFiles => FocusedPanel::Logs,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            FocusedPanel::History => "Historique",
            FocusedPanel::Logs => "Logs",
            FocusedPanel::RecentFiles => "Récents",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    None,
    OpenEditor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Modal {
    None,
    Menu,
    Settings,
    Files,
    Filters,
    DryRun,
    ConfirmResync,
    ConfirmCancel,
    ConfirmDelete(String),
    Help,
    HistoryDetails(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum HitAction {
    ButtonMenu,
    ButtonSync,
    ButtonCancel,
    ButtonDryRun,
    ButtonFiles,
    ButtonFilters,
    ButtonSettings,
    ButtonQuit,
    TickRateDec,
    TickRateInc,
    HistoryRow(usize),
    HistoryFile(usize),
    RecentFile(usize),
    LogsArea,
    RecentFilesArea,
    SettingOption(usize),
    SettingCycle(usize, bool),
    SaveSettings,
    CloseModal,
    MenuOption(usize),
    FileEntry(usize),
    FileOpen(usize),
    FileParent,
    ToggleLogsAuto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertLevel {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct Alert {
    pub level: AlertLevel,
    pub message: String,
    pub hint: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct Hitbox {
    pub rect: Rect,
    pub action: HitAction,
}

pub struct App {
    pub running: bool,
    pub modal: Modal,
    pub config: AppConfig,
    pub current_theme: ThemeChoice,
    pub service_info: ServiceInfo,
    pub streamer: SharedStreamer,
    pub live: StreamerState,
    pub past_runs: Vec<PastRun>,
    pub selected_run_idx: usize,
    pub filters: Vec<String>,
    pub selected_filter_idx: usize,
    pub logs_scroll: usize,
    pub auto_scroll: bool,
    pub toast: Option<(String, Instant)>,

    // btop++ : panel actif et tick rate dynamique
    pub focused_panel: FocusedPanel,
    pub tick_rate_ms_live: u64,
    pub tick_rate_changed: bool,
    pub menu_selected_idx: usize,

    // Explorateur de fichiers
    pub file_current_rel: String,
    pub file_entries: Vec<FileEntry>,
    pub file_selected_idx: usize,
    pub file_scroll_offset: usize,
    pub file_viewport_height: usize,

    // Scroll offsets pour historique et filtres (bug fixes)
    pub history_scroll_offset: usize,
    pub history_details_scroll: usize,
    pub filter_scroll_offset: usize,
    pub recent_scroll_offset: usize,

    // Paramètres
    pub settings_selected_idx: usize,

    // Simulation Dry-Run
    pub dry_run_running: bool,
    pub dry_run_logs: Vec<String>,
    pub dry_run_scroll: usize,
    pub dry_run_rx: Option<std::sync::mpsc::Receiver<Vec<String>>>,

    // Sélection d'éléments interactifs
    pub history_selected_file_idx: usize,
    pub recent_selected_idx: usize,

    // Registre précis des hitboxes cliquables (au pixel près)
    pub hitboxes: Vec<Hitbox>,

    last_systemd_check: Instant,
    last_history_check: Instant,
}

impl App {
    pub fn new() -> Self {
        let streamer = spawn_log_streamer();
        let config = config::load_config();
        let current_theme = config.theme.unwrap_or(ThemeChoice::TokyoNight);
        let tick_rate = config.tick_rate_ms.unwrap_or(250);
        let service_info = get_service_info();
        let past_runs = fetch_past_runs(50);
        let filters = config::read_filters();

        let mut app = Self {
            running: true,
            modal: Modal::None,
            config,
            current_theme,
            service_info,
            streamer,
            live: StreamerState::default(),
            past_runs,
            selected_run_idx: 0,
            filters,
            selected_filter_idx: 0,
            logs_scroll: 0,
            auto_scroll: true,
            toast: None,

            focused_panel: FocusedPanel::Logs,
            tick_rate_ms_live: tick_rate,
            tick_rate_changed: false,
            menu_selected_idx: 0,

            file_current_rel: "".to_string(),
            file_entries: Vec::new(),
            file_selected_idx: 0,
            file_scroll_offset: 0,
            file_viewport_height: 15,

            history_scroll_offset: 0,
            history_details_scroll: 0,
            filter_scroll_offset: 0,
            recent_scroll_offset: 0,

            settings_selected_idx: 0,

            dry_run_running: false,
            dry_run_logs: Vec::new(),
            dry_run_scroll: 0,
            dry_run_rx: None,

            history_selected_file_idx: 0,
            recent_selected_idx: 0,

            hitboxes: Vec::with_capacity(64),

            last_systemd_check: Instant::now(),
            last_history_check: Instant::now(),
        };

        app.reload_files();
        app
    }

    #[allow(dead_code)]
    pub fn clear_hitboxes(&mut self) {
        self.hitboxes.clear();
    }

    #[allow(dead_code)]
    pub fn register_hitbox(&mut self, rect: Rect, action: HitAction) {
        self.hitboxes.push(Hitbox { rect, action });
    }

    pub fn reload_files(&mut self) {
        let base = config::expand_tilde(&self.config.local_dir);
        if let Ok(entries) = fs_tree::list_directory(&base, &self.file_current_rel, &self.filters) {
            self.file_entries = entries;
            if self.file_selected_idx >= self.file_entries.len() {
                self.file_selected_idx = 0;
            }
        }
    }

    pub async fn on_tick(&mut self) {
        {
            let st = self.streamer.read().await;
            self.live = st.clone();
        }

        if self.last_systemd_check.elapsed().as_secs() >= 2 {
            self.service_info = get_service_info();
            self.last_systemd_check = Instant::now();
        }

        if self.last_history_check.elapsed().as_secs() >= 6 {
            self.past_runs = fetch_past_runs(50);
            self.last_history_check = Instant::now();
        }

        if let Some((_, created)) = self.toast {
            if created.elapsed().as_secs() > 4 {
                self.toast = None;
            }
        }

        if self.dry_run_running {
            if let Some(rx) = &self.dry_run_rx {
                if let Ok(lines) = rx.try_recv() {
                    self.dry_run_logs = lines;
                    self.dry_run_running = false;
                    self.set_toast("✔ Simulation Dry-Run terminée !");
                }
            }
        }
    }

    pub fn start_dry_run(&mut self) {
        self.dry_run_running = true;
        self.dry_run_logs = vec![
            "Analyse des différences entre le dossier local et Google Drive...".to_string(),
            "Exécution de : rclone bisync --dry-run -v --tpslimit 8".to_string(),
            "Cela peut prendre un instant...".to_string(),
        ];
        self.dry_run_scroll = 0;
        self.modal = Modal::DryRun;
        self.set_toast("🛡 Simulation Dry-Run démarrée...");

        let remote = self.config.remote.clone();
        let local_dir = config::expand_tilde(&self.config.local_dir).to_string_lossy().to_string();
        let filters_path = config::filters_file().to_string_lossy().to_string();

        let (tx, rx) = std::sync::mpsc::channel();
        self.dry_run_rx = Some(rx);

        std::thread::spawn(move || {
            let output = std::process::Command::new("rclone")
                .args(["bisync", &remote, &local_dir, "--dry-run", "-v", "--tpslimit", "8", "--filter-from", &filters_path])
                .output();

            let mut lines = Vec::new();
            match output {
                Ok(out) => {
                    let s_out = String::from_utf8_lossy(&out.stdout);
                    let s_err = String::from_utf8_lossy(&out.stderr);
                    for l in s_out.lines().chain(s_err.lines()) {
                        let trimmed = l.trim();
                        if !trimmed.is_empty() {
                            lines.push(trimmed.to_string());
                        }
                    }
                    if lines.is_empty() {
                        lines.push("Tout est déjà synchronisé (aucun transfert détecté en dry-run).".to_string());
                    }
                }
                Err(e) => {
                    lines.push(format!("Erreur d'exécution de rclone : {}", e));
                }
            }
            let _ = tx.send(lines);
        });
    }

    pub fn step_tick_rate(&mut self, faster: bool) {
        let steps = [50, 100, 250, 500, 1000, 2000];
        let current = self.tick_rate_ms_live;
        let pos = steps.iter().position(|&s| s == current).unwrap_or(2);
        let next_idx = if faster {
            pos.saturating_sub(1)
        } else {
            (pos + 1).min(steps.len() - 1)
        };
        let new_rate = steps[next_idx];
        if new_rate != current {
            self.tick_rate_ms_live = new_rate;
            self.tick_rate_changed = true;
            self.config.tick_rate_ms = Some(new_rate);
            let _ = config::save_config(&self.config);
            self.set_toast(format!("Fréquence : {} ms", new_rate));
        }
    }

    pub fn open_selected_file(&mut self, rel_path: &str) {
        let base = config::expand_tilde(&self.config.local_dir);
        match fs_tree::open_with_xdg(&base, rel_path) {
            Ok(_) => self.set_toast(format!("✔ Ouverture : {}", rel_path)),
            Err(e) => self.set_toast(format!("✗ Impossible d'ouvrir : {}", e)),
        }
    }

    pub fn open_selected_folder(&mut self, rel_path: &str) {
        let base = config::expand_tilde(&self.config.local_dir);
        match fs_tree::open_folder_with_xdg(&base, rel_path) {
            Ok(_) => self.set_toast(format!("📁 Dossier parent ouvert pour {}", rel_path)),
            Err(e) => self.set_toast(format!("✗ Impossible d'ouvrir dossier : {}", e)),
        }
    }

    pub fn open_history_file(&mut self, file_idx: usize) {
        let path_opt = self.past_runs.get(self.selected_run_idx).and_then(|r| {
            r.all_affected_files().get(file_idx).map(|(_, p)| p.to_string())
        });
        if let Some(path) = path_opt {
            self.open_selected_file(&path);
        }
    }

    pub fn open_history_folder(&mut self, file_idx: usize) {
        let path_opt = self.past_runs.get(self.selected_run_idx).and_then(|r| {
            r.all_affected_files().get(file_idx).map(|(_, p)| p.to_string())
        });
        if let Some(path) = path_opt {
            self.open_selected_folder(&path);
        }
    }

    pub fn get_recent_files_list(&self) -> Vec<String> {
        let mut list = Vec::new();
        for sf in self.live.synced_files.iter().rev().take(30) {
            list.push(sf.path.clone());
        }
        if list.is_empty() {
            for run in self.past_runs.iter().take(5) {
                for f in &run.files_copied {
                    list.push(f.clone());
                }
                for f in &run.files_modified {
                    list.push(f.clone());
                }
                for f in &run.files_deleted {
                    list.push(f.clone());
                }
            }
        }
        list
    }

    pub fn open_recent_file(&mut self, recent_idx: usize) {
        let list = self.get_recent_files_list();
        if let Some(path) = list.get(recent_idx) {
            self.open_selected_file(path);
        }
    }

    pub fn open_recent_folder(&mut self, recent_idx: usize) {
        let list = self.get_recent_files_list();
        if let Some(path) = list.get(recent_idx) {
            self.open_selected_folder(path);
        }
    }

    pub fn set_toast(&mut self, message: impl Into<String>) {
        self.toast = Some((message.into(), Instant::now()));
    }

    pub fn local_disk_stats(&self) -> (f64, f64, f64, f64) {
        systemd::get_disk_usage(&self.config.local_dir)
    }

    pub fn disk_pct(&self) -> f64 {
        self.local_disk_stats().3
    }

    pub fn consecutive_failures(&self) -> usize {
        let mut count = 0;
        for run in &self.past_runs {
            if run.status == RunStatus::Failed {
                count += 1;
            } else {
                break;
            }
        }
        count
    }

    pub fn active_alerts(&self) -> Vec<Alert> {
        let mut alerts = Vec::new();

        if self.live.resync_needed {
            alerts.push(Alert {
                level: AlertLevel::Error,
                message: "Erreur critique bisync : index corrompus ou inexistants.".to_string(),
                hint: Some("Appuyez sur 'r' pour resynchroniser (--resync)".to_string()),
            });
        }

        let failures = self.consecutive_failures();
        if failures >= 2 {
            alerts.push(Alert {
                level: AlertLevel::Error,
                message: format!("{} synchronisations consécutives en échec !", failures),
                hint: Some("Consultez les logs pour voir le détail des erreurs".to_string()),
            });
        }

        if self.live.is_syncing {
            if let Some(start) = self.live.sync_start {
                let duration_s = start.elapsed().as_secs();
                if duration_s > 300 {
                    alerts.push(Alert {
                        level: AlertLevel::Warning,
                        message: format!("Synchronisation anormalement longue ({} min {} s)", duration_s / 60, duration_s % 60),
                        hint: Some("Appuyez sur 'c' pour forcer l'annulation si bloqué".to_string()),
                    });
                }
            }
        }

        let disk = self.disk_pct();
        if disk > 90.0 {
            alerts.push(Alert {
                level: AlertLevel::Error,
                message: format!("Espace disque local critique ({:.1} % utilisé) !", disk),
                hint: Some("Libérez de l'espace sur la partition locale".to_string()),
            });
        }

        alerts
    }

    pub fn calculate_success_rate(&self) -> (f64, usize) {
        if self.past_runs.is_empty() {
            return (100.0, 0);
        }
        let total = self.past_runs.len();
        let mut ok = 0;
        for run in &self.past_runs {
            if run.status == RunStatus::Success || run.status == RunStatus::Skipped {
                ok += 1;
            }
        }
        let pct = (ok as f64 / total as f64) * 100.0;
        (pct, total)
    }

    pub fn runs_today_stats(&self) -> (usize, usize) {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let mut ok = 0;
        let mut err = 0;
        for run in &self.past_runs {
            if run.date == today {
                if run.status == RunStatus::Success || run.status == RunStatus::Skipped {
                    ok += 1;
                } else if run.status == RunStatus::Failed {
                    err += 1;
                }
            }
        }
        (ok, err)
    }

    pub fn conflicts_today_stats(&self) -> usize {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let mut conflicts = 0;
        for run in &self.past_runs {
            if run.date == today {
                conflicts += run.errors.len();
            }
        }
        conflicts
    }

    /// Ensure history scroll offset keeps the selected run visible
    #[allow(dead_code)]
    pub fn ensure_history_visible(&mut self, viewport_height: usize) {
        if viewport_height == 0 { return; }
        if self.selected_run_idx < self.history_scroll_offset {
            self.history_scroll_offset = self.selected_run_idx;
        } else if self.selected_run_idx >= self.history_scroll_offset + viewport_height {
            self.history_scroll_offset = self.selected_run_idx - viewport_height + 1;
        }
    }

    /// Clamp logs_scroll to prevent over-scrolling
    #[allow(dead_code)]
    pub fn clamp_logs_scroll(&mut self, total_lines: usize, visible_height: usize) {
        let max_scroll = total_lines.saturating_sub(visible_height);
        if self.logs_scroll > max_scroll {
            self.logs_scroll = max_scroll;
        }
    }

    /// Ensure filter scroll offset keeps selected filter visible
    #[allow(dead_code)]
    pub fn ensure_filter_visible(&mut self, viewport_height: usize) {
        if viewport_height == 0 { return; }
        if self.selected_filter_idx < self.filter_scroll_offset {
            self.filter_scroll_offset = self.selected_filter_idx;
        } else if self.selected_filter_idx >= self.filter_scroll_offset + viewport_height {
            self.filter_scroll_offset = self.selected_filter_idx - viewport_height + 1;
        }
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent) -> Action {
        match mouse.kind {
            MouseEventKind::ScrollDown => {
                let col = mouse.column;
                let row = mouse.row;
                for hb in self.hitboxes.iter().rev() {
                    if hb.rect.x <= col && col < hb.rect.x + hb.rect.width
                        && hb.rect.y <= row && row < hb.rect.y + hb.rect.height
                    {
                        match hb.action {
                            HitAction::LogsArea => {
                                self.logs_scroll = self.logs_scroll.saturating_sub(2);
                                if self.logs_scroll == 0 {
                                    self.auto_scroll = true;
                                }
                                return Action::None;
                            }
                            HitAction::HistoryRow(_) => {
                                if !self.past_runs.is_empty() && self.selected_run_idx < self.past_runs.len() - 1 {
                                    self.selected_run_idx += 1;
                                }
                                return Action::None;
                            }
                            HitAction::HistoryFile(_) => {
                                self.history_details_scroll = self.history_details_scroll.saturating_add(2);
                                return Action::None;
                            }
                            HitAction::RecentFilesArea | HitAction::RecentFile(_) => {
                                let list_len = self.get_recent_files_list().len();
                                if list_len > 0 && self.recent_selected_idx < list_len - 1 {
                                    self.recent_selected_idx += 1;
                                }
                                if self.recent_selected_idx >= self.recent_scroll_offset + 10 {
                                    self.recent_scroll_offset = self.recent_selected_idx.saturating_sub(9);
                                }
                                return Action::None;
                            }
                            HitAction::FileEntry(_) => {
                                if !self.file_entries.is_empty() && self.file_selected_idx < self.file_entries.len() - 1 {
                                    self.file_selected_idx += 1;
                                    let vp = self.file_viewport_height;
                                    if self.file_selected_idx >= self.file_scroll_offset + vp {
                                        self.file_scroll_offset = self.file_selected_idx - vp + 1;
                                    }
                                }
                                return Action::None;
                            }
                            _ => {}
                        }
                    }
                }
                if self.modal == Modal::DryRun {
                    self.dry_run_scroll = self.dry_run_scroll.saturating_add(2);
                    return Action::None;
                }
                // Défilement par défaut (logs)
                self.logs_scroll = self.logs_scroll.saturating_sub(2);
                if self.logs_scroll == 0 {
                    self.auto_scroll = true;
                }
            }
            MouseEventKind::ScrollUp => {
                let col = mouse.column;
                let row = mouse.row;
                for hb in self.hitboxes.iter().rev() {
                    if hb.rect.x <= col && col < hb.rect.x + hb.rect.width
                        && hb.rect.y <= row && row < hb.rect.y + hb.rect.height
                    {
                        match hb.action {
                            HitAction::LogsArea => {
                                self.auto_scroll = false;
                                self.logs_scroll = self.logs_scroll.saturating_add(2);
                                return Action::None;
                            }
                            HitAction::HistoryRow(_) => {
                                if self.selected_run_idx > 0 {
                                    self.selected_run_idx -= 1;
                                }
                                return Action::None;
                            }
                            HitAction::HistoryFile(_) => {
                                self.history_details_scroll = self.history_details_scroll.saturating_sub(2);
                                return Action::None;
                            }
                            HitAction::RecentFilesArea | HitAction::RecentFile(_) => {
                                if self.recent_selected_idx > 0 {
                                    self.recent_selected_idx -= 1;
                                }
                                if self.recent_selected_idx < self.recent_scroll_offset {
                                    self.recent_scroll_offset = self.recent_selected_idx;
                                }
                                return Action::None;
                            }
                            HitAction::FileEntry(_) => {
                                if self.file_selected_idx > 0 {
                                    self.file_selected_idx -= 1;
                                    if self.file_selected_idx < self.file_scroll_offset {
                                        self.file_scroll_offset = self.file_scroll_offset.saturating_sub(1);
                                    }
                                }
                                return Action::None;
                            }
                            _ => {}
                        }
                    }
                }
                if self.modal == Modal::DryRun {
                    self.dry_run_scroll = self.dry_run_scroll.saturating_sub(2);
                    return Action::None;
                }
                self.auto_scroll = false;
                self.logs_scroll = self.logs_scroll.saturating_add(2);
            }
            MouseEventKind::Down(MouseButton::Left) => {
                let col = mouse.column;
                let row = mouse.row;

                // Tester les hitboxes de la plus récente (modal en premier) à la plus ancienne
                for hb in self.hitboxes.iter().rev() {
                    if hb.rect.x <= col && col < hb.rect.x + hb.rect.width
                        && hb.rect.y <= row && row < hb.rect.y + hb.rect.height
                    {
                        match hb.action {
                            HitAction::ButtonMenu => {
                                self.menu_selected_idx = 0;
                                self.modal = Modal::Menu;
                                return Action::None;
                            }
                            HitAction::ButtonSync => {
                                match systemd::trigger_sync() {
                                    Ok(_) => self.set_toast("✔ Synchronisation forcée initiée..."),
                                    Err(e) => self.set_toast(format!("✗ Erreur : {}", e)),
                                }
                                return Action::None;
                            }
                            HitAction::ButtonCancel => {
                                self.modal = Modal::ConfirmCancel;
                                return Action::None;
                            }
                            HitAction::ButtonDryRun => {
                                self.start_dry_run();
                                return Action::None;
                            }
                            HitAction::ButtonFiles => {
                                self.modal = if self.modal == Modal::Files { Modal::None } else { Modal::Files };
                                return Action::None;
                            }
                            HitAction::ButtonFilters => {
                                self.modal = if self.modal == Modal::Filters { Modal::None } else { Modal::Filters };
                                return Action::None;
                            }
                            HitAction::ButtonSettings => {
                                self.modal = if self.modal == Modal::Settings { Modal::None } else { Modal::Settings };
                                return Action::None;
                            }
                            HitAction::ButtonQuit => {
                                self.running = false;
                                return Action::None;
                            }
                            HitAction::TickRateDec => {
                                self.step_tick_rate(true);
                                return Action::None;
                            }
                            HitAction::TickRateInc => {
                                self.step_tick_rate(false);
                                return Action::None;
                            }
                            HitAction::CloseModal => {
                                self.modal = Modal::None;
                                return Action::None;
                            }
                            HitAction::MenuOption(idx) => {
                                match idx {
                                    0 => { self.modal = Modal::Settings; }
                                    1 => { self.modal = Modal::Help; }
                                    2 => { self.running = false; }
                                    _ => {}
                                }
                                return Action::None;
                            }
                            HitAction::HistoryRow(idx) => {
                                if idx < self.past_runs.len() {
                                    if self.selected_run_idx == idx {
                                        self.history_details_scroll = 0;
                                        self.history_selected_file_idx = 0;
                                        self.modal = Modal::HistoryDetails(idx);
                                    } else {
                                        self.selected_run_idx = idx;
                                    }
                                }
                                return Action::None;
                            }
                            HitAction::HistoryFile(idx) => {
                                self.history_selected_file_idx = idx;
                                if mouse.modifiers.contains(KeyModifiers::CONTROL) {
                                    self.open_history_folder(idx);
                                } else {
                                    self.open_history_file(idx);
                                }
                                return Action::None;
                            }
                            HitAction::RecentFile(idx) => {
                                self.recent_selected_idx = idx;
                                if mouse.modifiers.contains(KeyModifiers::CONTROL) {
                                    self.open_recent_folder(idx);
                                } else {
                                    self.open_recent_file(idx);
                                }
                                return Action::None;
                            }
                            HitAction::SettingOption(idx) => {
                                self.settings_selected_idx = idx;
                                self.cycle_setting(true);
                                return Action::None;
                            }
                            HitAction::SettingCycle(idx, forward) => {
                                self.settings_selected_idx = idx;
                                self.cycle_setting(forward);
                                return Action::None;
                            }
                            HitAction::SaveSettings => {
                                self.save_current_settings();
                                return Action::None;
                            }
                            HitAction::FileEntry(idx) => {
                                self.file_selected_idx = idx;
                                if mouse.modifiers.contains(KeyModifiers::CONTROL) {
                                    if let Some(entry) = self.file_entries.get(idx) {
                                        let base = config::expand_tilde(&self.config.local_dir);
                                        let _ = fs_tree::open_folder_with_xdg(&base, &entry.rel_path);
                                    }
                                } else {
                                    self.enter_selected_file_or_dir();
                                }
                                return Action::None;
                            }
                            HitAction::FileOpen(idx) => {
                                self.file_selected_idx = idx;
                                self.enter_selected_file_or_dir();
                                return Action::None;
                            }
                            HitAction::FileParent => {
                                self.parent_file_dir();
                                return Action::None;
                            }
                            HitAction::ToggleLogsAuto => {
                                self.auto_scroll = !self.auto_scroll;
                                return Action::None;
                            }
                            _ => {}
                        }
                    }
                }

                // Clic en dehors d'une modale ouverte : fermeture
                if self.modal != Modal::None {
                    self.modal = Modal::None;
                }
            }
            _ => {}
        }
        Action::None
    }

    pub fn enter_selected_file_or_dir(&mut self) {
        if let Some(entry) = self.file_entries.get(self.file_selected_idx).cloned() {
            if entry.name.starts_with("..") {
                self.parent_file_dir();
            } else if entry.is_dir {
                self.file_current_rel = entry.rel_path;
                self.file_selected_idx = 0;
                self.file_scroll_offset = 0;
                self.reload_files();
            } else {
                let base = config::expand_tilde(&self.config.local_dir);
                match fs_tree::open_with_xdg(&base, &entry.rel_path) {
                    Ok(_) => self.set_toast(format!("✔ Ouverture de {}", entry.name)),
                    Err(e) => self.set_toast(format!("✗ Erreur : {}", e)),
                }
            }
        }
    }

    pub fn parent_file_dir(&mut self) {
        if !self.file_current_rel.is_empty() {
            let p = std::path::Path::new(&self.file_current_rel);
            if let Some(parent) = p.parent() {
                self.file_current_rel = parent.to_string_lossy().to_string();
            } else {
                self.file_current_rel = "".to_string();
            }
            self.file_selected_idx = 0;
            self.file_scroll_offset = 0;
            self.reload_files();
        }
    }

    pub fn cycle_setting(&mut self, forward: bool) {
        match self.settings_selected_idx {
            0 => {
                self.current_theme = if forward {
                    self.current_theme.next()
                } else {
                    self.current_theme.prev()
                };
                self.config.theme = Some(self.current_theme);
                self.set_toast(format!("Thème actif : {}", self.current_theme.name()));
            }
            1 => {
                let options = ["10min", "15min", "30min", "1h"];
                let pos = options.iter().position(|&o| o == self.config.timer_interval).unwrap_or(0);
                let next = if forward { (pos + 1) % options.len() } else { (pos + options.len() - 1) % options.len() };
                self.config.timer_interval = options[next].to_string();
            }
            2 => {
                let options = ["60", "120", "240", "360", "720", "1440", "never"];
                let pos = options.iter().position(|&o| o == self.config.full_sync_interval).unwrap_or(0);
                let next = if forward { (pos + 1) % options.len() } else { (pos + options.len() - 1) % options.len() };
                self.config.full_sync_interval = options[next].to_string();
            }
            3 => {
                let options = ["Désactivé", "5M", "10M", "20M", "50M"];
                let cur = self.config.bwlimit.as_deref().unwrap_or("Désactivé");
                let pos = options.iter().position(|&o| o == cur).unwrap_or(0);
                let next = if forward { (pos + 1) % options.len() } else { (pos + options.len() - 1) % options.len() };
                self.config.bwlimit = if options[next] == "Désactivé" { None } else { Some(options[next].to_string()) };
            }
            4 => {
                let options = [50, 100, 250, 500, 1000, 2000];
                let cur = self.config.tick_rate_ms.unwrap_or(250);
                let pos = options.iter().position(|&o| o == cur).unwrap_or(2);
                let next = if forward { (pos + 1) % options.len() } else { (pos + options.len() - 1) % options.len() };
                self.config.tick_rate_ms = Some(options[next]);
                self.tick_rate_ms_live = options[next];
                self.tick_rate_changed = true;
            }
            7 => {
                self.save_current_settings();
            }
            _ => {}
        }
    }

    pub fn save_current_settings(&mut self) {
        match config::save_config(&self.config) {
            Ok(_) => self.set_toast("✔ Paramètres enregistrés dans dash-config.json !"),
            Err(e) => self.set_toast(format!("✗ Erreur de sauvegarde : {}", e)),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        // 1. Modales prioritaires
        if self.modal != Modal::None {
            match &self.modal {
                Modal::Menu => match key.code {
                    KeyCode::Esc => {
                        self.modal = Modal::None;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if self.menu_selected_idx > 0 {
                            self.menu_selected_idx -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if self.menu_selected_idx < 2 {
                            self.menu_selected_idx += 1;
                        }
                    }
                    KeyCode::Enter => {
                        match self.menu_selected_idx {
                            0 => { self.modal = Modal::Settings; }
                            1 => { self.modal = Modal::Help; }
                            2 => { self.running = false; }
                            _ => {}
                        }
                    }
                    KeyCode::Char('o') | KeyCode::Char('s') => { self.modal = Modal::Settings; }
                    KeyCode::Char('?') | KeyCode::Char('h') => { self.modal = Modal::Help; }
                    KeyCode::Char('q') => { self.running = false; }
                    _ => {}
                },
                Modal::ConfirmResync => match key.code {
                    KeyCode::Char('y') | KeyCode::Char('o') | KeyCode::Enter => {
                        self.modal = Modal::None;
                        match systemd::trigger_resync() {
                            Ok(_) => self.set_toast("✔ Resynchronisation complète initiée (--resync) !"),
                            Err(e) => self.set_toast(format!("✗ Erreur : {}", e)),
                        }
                    }
                    KeyCode::Char('n') | KeyCode::Esc | KeyCode::Char('q') => {
                        self.modal = Modal::None;
                    }
                    _ => {}
                },
                Modal::ConfirmCancel => match key.code {
                    KeyCode::Char('y') | KeyCode::Char('o') | KeyCode::Enter => {
                        self.modal = Modal::None;
                        match systemd::cancel_sync() {
                            Ok(_) => self.set_toast("✔ Arrêt de la synchronisation en cours !"),
                            Err(e) => self.set_toast(format!("✗ Erreur : {}", e)),
                        }
                    }
                    KeyCode::Char('n') | KeyCode::Esc | KeyCode::Char('q') => {
                        self.modal = Modal::None;
                    }
                    _ => {}
                },
                Modal::ConfirmDelete(rel) => {
                    let rel_clone = rel.clone();
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('o') | KeyCode::Enter => {
                            self.modal = Modal::None;
                            let base = config::expand_tilde(&self.config.local_dir);
                            match fs_tree::delete_entry(&base, &rel_clone) {
                                Ok(_) => {
                                    self.set_toast(format!("✔ {} supprimé", rel_clone));
                                    self.reload_files();
                                }
                                Err(e) => self.set_toast(format!("✗ Erreur : {}", e)),
                            }
                        }
                        KeyCode::Char('n') | KeyCode::Esc | KeyCode::Char('q') => {
                            self.modal = Modal::None;
                        }
                        _ => {}
                    }
                }
                Modal::Settings => match key.code {
                    KeyCode::Esc => {
                        self.modal = Modal::None;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if self.settings_selected_idx > 0 {
                            self.settings_selected_idx -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if self.settings_selected_idx < SETTINGS_ITEMS_COUNT - 1 {
                            self.settings_selected_idx += 1;
                        }
                    }
                    KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                        self.cycle_setting(true);
                    }
                    KeyCode::Left | KeyCode::Char('h') => {
                        self.cycle_setting(false);
                    }
                    _ => {}
                },
                Modal::Files => match key.code {
                    KeyCode::Esc => {
                        self.modal = Modal::None;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if self.file_selected_idx > 0 {
                            self.file_selected_idx -= 1;
                            if self.file_selected_idx < self.file_scroll_offset {
                                self.file_scroll_offset = self.file_selected_idx;
                            }
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if !self.file_entries.is_empty() && self.file_selected_idx < self.file_entries.len() - 1 {
                            self.file_selected_idx += 1;
                            let vp = self.file_viewport_height;
                            if self.file_selected_idx >= self.file_scroll_offset + vp {
                                self.file_scroll_offset = self.file_selected_idx - vp + 1;
                            }
                        }
                    }
                    KeyCode::PageUp => {
                        let jump = self.file_viewport_height;
                        self.file_selected_idx = self.file_selected_idx.saturating_sub(jump);
                        if self.file_selected_idx < self.file_scroll_offset {
                            self.file_scroll_offset = self.file_selected_idx;
                        }
                    }
                    KeyCode::PageDown => {
                        let jump = self.file_viewport_height;
                        let max = if self.file_entries.is_empty() { 0 } else { self.file_entries.len() - 1 };
                        self.file_selected_idx = (self.file_selected_idx + jump).min(max);
                        if self.file_selected_idx >= self.file_scroll_offset + self.file_viewport_height {
                            self.file_scroll_offset = self.file_selected_idx - self.file_viewport_height + 1;
                        }
                    }
                    KeyCode::Home => {
                        self.file_selected_idx = 0;
                        self.file_scroll_offset = 0;
                    }
                    KeyCode::End => {
                        if !self.file_entries.is_empty() {
                            self.file_selected_idx = self.file_entries.len() - 1;
                            let vp = self.file_viewport_height;
                            if self.file_selected_idx >= vp {
                                self.file_scroll_offset = self.file_selected_idx - vp + 1;
                            }
                        }
                    }
                    KeyCode::Enter => {
                        if key.modifiers.contains(KeyModifiers::CONTROL) {
                            if let Some(entry) = self.file_entries.get(self.file_selected_idx) {
                                let base = config::expand_tilde(&self.config.local_dir);
                                match fs_tree::open_folder_with_xdg(&base, &entry.rel_path) {
                                    Ok(_) => self.set_toast(format!("📁 Dossier parent ouvert pour {}", entry.name)),
                                    Err(e) => self.set_toast(format!("✗ Erreur : {}", e)),
                                }
                            }
                        } else {
                            self.enter_selected_file_or_dir();
                        }
                    }
                    KeyCode::Backspace => {
                        self.parent_file_dir();
                    }
                    KeyCode::Char('o') => {
                        if let Some(entry) = self.file_entries.get(self.file_selected_idx) {
                            let base = config::expand_tilde(&self.config.local_dir);
                            if key.modifiers.contains(KeyModifiers::CONTROL) {
                                let _ = fs_tree::open_folder_with_xdg(&base, &entry.rel_path);
                            } else {
                                let _ = fs_tree::open_with_xdg(&base, &entry.rel_path);
                            }
                        }
                    }
                    KeyCode::Char('x') => {
                        if let Some(entry) = self.file_entries.get(self.file_selected_idx) {
                            if !entry.name.starts_with("..") {
                                match fs_tree::add_exclude_rule(&entry.rel_path, entry.is_dir) {
                                    Ok(_) => {
                                        self.set_toast(format!("✔ Exclusion ajoutée : {}", entry.name));
                                        self.filters = config::read_filters();
                                        self.reload_files();
                                    }
                                    Err(e) => self.set_toast(format!("✗ Erreur : {}", e)),
                                }
                            }
                        }
                    }
                    KeyCode::Char('d') => {
                        if let Some(entry) = self.file_entries.get(self.file_selected_idx) {
                            if !entry.name.starts_with("..") {
                                self.modal = Modal::ConfirmDelete(entry.rel_path.clone());
                            }
                        }
                    }
                    _ => {}
                },
                Modal::Filters => match key.code {
                    KeyCode::Esc => {
                        self.modal = Modal::None;
                    }
                    KeyCode::Char('e') => {
                        return Action::OpenEditor;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if self.selected_filter_idx > 0 {
                            self.selected_filter_idx -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if !self.filters.is_empty() && self.selected_filter_idx < self.filters.len() - 1 {
                            self.selected_filter_idx += 1;
                        }
                    }
                    KeyCode::PageUp => {
                        self.selected_filter_idx = self.selected_filter_idx.saturating_sub(10);
                        if self.selected_filter_idx < self.filter_scroll_offset {
                            self.filter_scroll_offset = self.selected_filter_idx;
                        }
                    }
                    KeyCode::PageDown => {
                        if !self.filters.is_empty() {
                            let max = self.filters.len() - 1;
                            self.selected_filter_idx = (self.selected_filter_idx + 10).min(max);
                        }
                    }
                    KeyCode::Home => {
                        self.selected_filter_idx = 0;
                        self.filter_scroll_offset = 0;
                    }
                    KeyCode::End => {
                        if !self.filters.is_empty() {
                            self.selected_filter_idx = self.filters.len() - 1;
                        }
                    }
                    _ => {}
                },
                Modal::DryRun => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.modal = Modal::None;
                    }
                    KeyCode::Char('r') => {
                        self.start_dry_run();
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.dry_run_scroll = self.dry_run_scroll.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.dry_run_scroll = self.dry_run_scroll.saturating_add(1);
                    }
                    KeyCode::PageUp => {
                        self.dry_run_scroll = self.dry_run_scroll.saturating_sub(10);
                    }
                    KeyCode::PageDown => {
                        self.dry_run_scroll = self.dry_run_scroll.saturating_add(10);
                    }
                    KeyCode::Home => {
                        self.dry_run_scroll = 0;
                    }
                    _ => {}
                },
                Modal::Help => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') | KeyCode::Enter => {
                        self.modal = Modal::None;
                    }
                    _ => {}
                },
                Modal::HistoryDetails(run_idx) => {
                    let run_idx_val = *run_idx;
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            self.modal = Modal::None;
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if self.history_selected_file_idx > 0 {
                                self.history_selected_file_idx -= 1;
                            }
                            if self.history_selected_file_idx < self.history_details_scroll {
                                self.history_details_scroll = self.history_selected_file_idx;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            let files_len = self.past_runs.get(run_idx_val).map(|r| r.all_affected_files().len()).unwrap_or(0);
                            if files_len > 0 && self.history_selected_file_idx < files_len - 1 {
                                self.history_selected_file_idx += 1;
                            }
                            if self.history_selected_file_idx >= self.history_details_scroll + 15 {
                                self.history_details_scroll = self.history_selected_file_idx.saturating_sub(14);
                            }
                        }
                        KeyCode::PageUp => {
                            self.history_details_scroll = self.history_details_scroll.saturating_sub(10);
                            self.history_selected_file_idx = self.history_selected_file_idx.saturating_sub(10);
                        }
                        KeyCode::PageDown => {
                            self.history_details_scroll = self.history_details_scroll.saturating_add(10);
                            let files_len = self.past_runs.get(run_idx_val).map(|r| r.all_affected_files().len()).unwrap_or(0);
                            if files_len > 0 {
                                self.history_selected_file_idx = (self.history_selected_file_idx + 10).min(files_len - 1);
                            }
                        }
                        KeyCode::Home => {
                            self.history_details_scroll = 0;
                            self.history_selected_file_idx = 0;
                        }
                        KeyCode::End => {
                            let files_len = self.past_runs.get(run_idx_val).map(|r| r.all_affected_files().len()).unwrap_or(0);
                            if files_len > 0 {
                                self.history_selected_file_idx = files_len - 1;
                            }
                        }
                        KeyCode::Enter => {
                            if key.modifiers.contains(KeyModifiers::CONTROL) {
                                self.open_history_folder(self.history_selected_file_idx);
                            } else {
                                self.open_history_file(self.history_selected_file_idx);
                            }
                        }
                        KeyCode::Char('d') => {
                            self.open_history_folder(self.history_selected_file_idx);
                        }
                        KeyCode::Char('o') => {
                            self.open_history_file(self.history_selected_file_idx);
                        }
                        _ => {}
                    }
                }
                Modal::None => {}
            }
            return Action::None;
        }

        // 2. Raccourcis globaux du Dashboard
        match key.code {
            // Entrée sur l'historique : ouvrir les détails du run sélectionné
            // Entrée sur les fichiers récents : ouvrir le fichier ou le dossier (avec Ctrl)
            KeyCode::Enter => {
                match self.focused_panel {
                    FocusedPanel::History => {
                        if !self.past_runs.is_empty() {
                            self.history_details_scroll = 0;
                            self.history_selected_file_idx = 0;
                            self.modal = Modal::HistoryDetails(self.selected_run_idx);
                            return Action::None;
                        }
                    }
                    FocusedPanel::RecentFiles => {
                        if key.modifiers.contains(KeyModifiers::CONTROL) {
                            self.open_recent_folder(self.recent_selected_idx);
                        } else {
                            self.open_recent_file(self.recent_selected_idx);
                        }
                        return Action::None;
                    }
                    _ => {}
                }
            }
            // Esc ou m ouvre le menu principal (style btop++) quand aucune modale n'est ouverte
            KeyCode::Esc | KeyCode::Char('m') => {
                self.menu_selected_idx = 0;
                self.modal = Modal::Menu;
                return Action::None;
            }
            KeyCode::Char('q') => {
                self.running = false;
                return Action::None;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.running = false;
                return Action::None;
            }
            KeyCode::Char('o') => {
                if self.focused_panel == FocusedPanel::RecentFiles {
                    self.open_recent_file(self.recent_selected_idx);
                } else {
                    self.modal = Modal::Settings;
                }
                return Action::None;
            }
            KeyCode::Char('d') => {
                if self.focused_panel == FocusedPanel::RecentFiles {
                    self.open_recent_folder(self.recent_selected_idx);
                } else {
                    self.start_dry_run();
                }
                return Action::None;
            }
            KeyCode::Char('f') => {
                self.modal = Modal::Files;
                return Action::None;
            }
            KeyCode::Char('e') => {
                self.modal = Modal::Filters;
                return Action::None;
            }
            KeyCode::Char('t') => {
                self.current_theme = self.current_theme.next();
                self.config.theme = Some(self.current_theme);
                self.set_toast(format!("Thème actif : {}", self.current_theme.name()));
                return Action::None;
            }
            KeyCode::Char('?') | KeyCode::Char('h') => {
                self.modal = Modal::Help;
                return Action::None;
            }
            KeyCode::Char('s') => {
                match systemd::trigger_sync() {
                    Ok(_) => self.set_toast("✔ Lancement de la synchronisation forcée..."),
                    Err(e) => self.set_toast(format!("✗ Erreur : {}", e)),
                }
                return Action::None;
            }
            KeyCode::Char('r') => {
                self.modal = Modal::ConfirmResync;
                return Action::None;
            }
            KeyCode::Char('c') => {
                if self.service_info.state == ServiceState::Active || self.live.is_syncing {
                    self.modal = Modal::ConfirmCancel;
                } else {
                    self.set_toast("Aucune synchronisation en cours à annuler.");
                }
                return Action::None;
            }
            KeyCode::Char(' ') => {
                self.auto_scroll = !self.auto_scroll;
                if self.auto_scroll {
                    self.logs_scroll = 0;
                    self.set_toast("Défilement auto : ACTIVÉ");
                } else {
                    self.set_toast("Défilement auto : EN PAUSE");
                }
                return Action::None;
            }
            // Tab / Shift+Tab : changer de panel actif
            KeyCode::Tab => {
                self.focused_panel = if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.focused_panel.prev()
                } else {
                    self.focused_panel.next()
                };
                self.set_toast(format!("Panel actif : {}", self.focused_panel.label()));
                return Action::None;
            }
            KeyCode::BackTab => {
                self.focused_panel = self.focused_panel.prev();
                self.set_toast(format!("Panel actif : {}", self.focused_panel.label()));
                return Action::None;
            }
            // +/- : tick rate dynamique (btop++ style: - accélère, + ralentit)
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.step_tick_rate(false); // ralentit
                return Action::None;
            }
            KeyCode::Char('-') | KeyCode::Char('_') => {
                self.step_tick_rate(true); // accélère
                return Action::None;
            }
            // Navigation dirigée par le panel actif
            KeyCode::Up | KeyCode::Char('k') => {
                match self.focused_panel {
                    FocusedPanel::History => {
                        if self.selected_run_idx > 0 {
                            self.selected_run_idx -= 1;
                        }
                    }
                    FocusedPanel::Logs => {
                        self.auto_scroll = false;
                        self.logs_scroll = self.logs_scroll.saturating_add(1);
                    }
                    FocusedPanel::RecentFiles => {
                        if self.recent_selected_idx > 0 {
                            self.recent_selected_idx -= 1;
                        }
                        if self.recent_selected_idx < self.recent_scroll_offset {
                            self.recent_scroll_offset = self.recent_selected_idx;
                        }
                    }
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                match self.focused_panel {
                    FocusedPanel::History => {
                        if !self.past_runs.is_empty() && self.selected_run_idx < self.past_runs.len() - 1 {
                            self.selected_run_idx += 1;
                        }
                    }
                    FocusedPanel::Logs => {
                        if self.logs_scroll > 0 {
                            self.logs_scroll = self.logs_scroll.saturating_sub(1);
                        }
                        if self.logs_scroll == 0 {
                            self.auto_scroll = true;
                        }
                    }
                    FocusedPanel::RecentFiles => {
                        let total = self.get_recent_files_list().len();
                        if total > 0 && self.recent_selected_idx < total - 1 {
                            self.recent_selected_idx += 1;
                        }
                        if self.recent_selected_idx >= self.recent_scroll_offset + 10 {
                            self.recent_scroll_offset = self.recent_selected_idx.saturating_sub(9);
                        }
                    }
                }
            }
            KeyCode::PageUp => {
                match self.focused_panel {
                    FocusedPanel::History => {
                        self.selected_run_idx = self.selected_run_idx.saturating_sub(10);
                    }
                    FocusedPanel::Logs => {
                        self.auto_scroll = false;
                        self.logs_scroll = self.logs_scroll.saturating_add(10);
                    }
                    FocusedPanel::RecentFiles => {
                        self.recent_scroll_offset = self.recent_scroll_offset.saturating_sub(10);
                        self.recent_selected_idx = self.recent_selected_idx.saturating_sub(10);
                    }
                }
            }
            KeyCode::PageDown => {
                match self.focused_panel {
                    FocusedPanel::History => {
                        if !self.past_runs.is_empty() {
                            self.selected_run_idx = (self.selected_run_idx + 10).min(self.past_runs.len() - 1);
                        }
                    }
                    FocusedPanel::Logs => {
                        self.logs_scroll = self.logs_scroll.saturating_sub(10);
                        if self.logs_scroll == 0 {
                            self.auto_scroll = true;
                        }
                    }
                    FocusedPanel::RecentFiles => {
                        self.recent_scroll_offset += 10;
                        let total = self.get_recent_files_list().len();
                        if total > 0 {
                            self.recent_selected_idx = (self.recent_selected_idx + 10).min(total - 1);
                        }
                    }
                }
            }
            KeyCode::Home => {
                match self.focused_panel {
                    FocusedPanel::History => {
                        self.selected_run_idx = 0;
                    }
                    FocusedPanel::Logs => {
                        self.auto_scroll = false;
                        let total = self.live.log_lines.len();
                        self.logs_scroll = total;
                    }
                    FocusedPanel::RecentFiles => {
                        self.recent_scroll_offset = 0;
                        self.recent_selected_idx = 0;
                    }
                }
            }
            KeyCode::End => {
                match self.focused_panel {
                    FocusedPanel::History => {
                        if !self.past_runs.is_empty() {
                            self.selected_run_idx = self.past_runs.len() - 1;
                        }
                    }
                    FocusedPanel::Logs => {
                        self.logs_scroll = 0;
                        self.auto_scroll = true;
                    }
                    FocusedPanel::RecentFiles => {
                        let total = self.get_recent_files_list().len();
                        if total > 0 {
                            self.recent_selected_idx = total - 1;
                        }
                    }
                }
            }
            _ => {}
        }

        Action::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitor::history::{PastRun, RunStatus};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

    #[tokio::test]
    async fn test_mouse_click_hitboxes() {
        let backend = TestBackend::new(130, 35);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();

        // Rendu pour générer les hitboxes précises
        terminal.draw(|f| crate::ui::render(f, &mut app)).unwrap();
        assert!(!app.hitboxes.is_empty(), "Les hitboxes doivent être enregistrées");

        // Trouver la hitbox du bouton Paramètres (Options)
        let settings_hb = app.hitboxes.iter().find(|h| h.action == HitAction::ButtonSettings);
        assert!(settings_hb.is_some(), "Le bouton Settings doit être présent");
        let hb = settings_hb.unwrap();

        // Clic au centre de la hitbox
        let click_x = hb.rect.x + hb.rect.width / 2;
        let click_y = hb.rect.y + hb.rect.height / 2;

        let mouse_event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: click_x,
            row: click_y,
            modifiers: KeyModifiers::NONE,
        };

        app.handle_mouse(mouse_event);
        assert_eq!(app.modal, Modal::Settings, "Le clic doit ouvrir la modal Paramètres");

        // Clic à nouveau pour fermer ou basculer
        app.handle_mouse(mouse_event);
        assert_eq!(app.modal, Modal::None, "Le clic doit fermer la modal Paramètres");
    }

    #[tokio::test]
    async fn test_mouse_scroll_logs() {
        let mut app = App::new();
        app.hitboxes.push(Hitbox {
            rect: Rect { x: 50, y: 5, width: 50, height: 15 },
            action: HitAction::LogsArea,
        });

        assert!(app.auto_scroll);
        // Molette vers le haut dans les logs
        let scroll_up = MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 60,
            row: 10,
            modifiers: KeyModifiers::NONE,
        };
        app.handle_mouse(scroll_up);
        assert!(!app.auto_scroll, "Le défilement vers le haut doit désactiver l'auto-scroll");
        assert_eq!(app.logs_scroll, 2);
    }

    #[tokio::test]
    async fn test_resize_resilience() {
        let sizes = [(40, 15), (60, 20), (80, 24), (100, 30), (140, 45), (200, 60)];
        let modals = vec![
            Modal::None,
            Modal::Menu,
            Modal::Help,
            Modal::Settings,
            Modal::Filters,
            Modal::Files,
            Modal::ConfirmResync,
            Modal::ConfirmCancel,
            Modal::ConfirmDelete("/test/file.txt".to_string()),
            Modal::HistoryDetails(0),
            Modal::DryRun,
        ];

        for (w, h) in sizes {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).unwrap();
            let mut app = App::new();

            // Add sample run for HistoryDetails
            app.past_runs.push(PastRun {
                id: 1,
                date: "2026-09-17".to_string(),
                time: "22:00".to_string(),
                duration: "1m 30s".to_string(),
                status: RunStatus::Success,
                summary: "Sync OK".to_string(),
                files_copied: vec!["file1.txt".to_string()],
                files_modified: vec![],
                files_deleted: vec![],
                errors: vec![],
            });

            for modal in &modals {
                app.modal = modal.clone();
                let res = terminal.draw(|f| crate::ui::render(f, &mut app));
                assert!(res.is_ok(), "Render failed for size {}x{} with modal {:?}", w, h, modal);
            }
        }
    }

    #[tokio::test]
    async fn test_menu_keyboard_and_mouse() {
        let mut app = App::new();
        // Pressing Esc on dashboard opens Menu
        let esc_event = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        app.handle_key(esc_event);
        assert_eq!(app.modal, Modal::Menu);

        // Pressing Down arrow selects second option (Help)
        let down_event = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        app.handle_key(down_event);
        assert_eq!(app.menu_selected_idx, 1);

        // Pressing Enter opens Help modal
        let enter_event = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        app.handle_key(enter_event);
        assert_eq!(app.modal, Modal::Help);

        // Pressing Esc closes Help modal
        app.handle_key(esc_event);
        assert_eq!(app.modal, Modal::None);
    }

    #[tokio::test]
    async fn test_tick_rate_stepping() {
        let mut app = App::new();
        app.tick_rate_ms_live = 250;

        // '-' speeds up (lower ms)
        let minus_event = KeyEvent::new(KeyCode::Char('-'), KeyModifiers::NONE);
        app.handle_key(minus_event);
        assert_eq!(app.tick_rate_ms_live, 100);

        app.handle_key(minus_event);
        assert_eq!(app.tick_rate_ms_live, 50);

        // Clamping at lowest
        app.handle_key(minus_event);
        assert_eq!(app.tick_rate_ms_live, 50);

        // '+' slows down (higher ms)
        let plus_event = KeyEvent::new(KeyCode::Char('+'), KeyModifiers::NONE);
        app.handle_key(plus_event);
        assert_eq!(app.tick_rate_ms_live, 100);

        app.handle_key(plus_event);
        assert_eq!(app.tick_rate_ms_live, 250);

        app.handle_key(plus_event);
        assert_eq!(app.tick_rate_ms_live, 500);

        app.handle_key(plus_event);
        assert_eq!(app.tick_rate_ms_live, 1000);

        app.handle_key(plus_event);
        assert_eq!(app.tick_rate_ms_live, 2000);

        // Clamping at highest
        app.handle_key(plus_event);
        assert_eq!(app.tick_rate_ms_live, 2000);
    }

    #[tokio::test]
    async fn test_settings_never_cycle() {
        let mut app = App::new();
        app.settings_selected_idx = 2; // full_sync_interval
        app.config.full_sync_interval = "1440".to_string();

        // Cycling forward from 1440 must reach "never"
        app.cycle_setting(true);
        assert_eq!(app.config.full_sync_interval, "never");

        // Cycling forward from "never" must wrap around to "60"
        app.cycle_setting(true);
        assert_eq!(app.config.full_sync_interval, "60");
    }

    #[tokio::test]
    async fn test_dry_run_and_cancel_modals() {
        let mut app = App::new();

        // Trigger dry-run modal
        app.start_dry_run();
        assert_eq!(app.modal, Modal::DryRun);
        assert!(app.dry_run_running);

        // Esc closes DryRun modal
        let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        app.handle_key(esc);
        assert_eq!(app.modal, Modal::None);

        // 'c' opens ConfirmCancel modal if syncing
        app.live.is_syncing = true;
        let c_key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE);
        app.handle_key(c_key);
        assert_eq!(app.modal, Modal::ConfirmCancel);

        // 'n' cancels ConfirmCancel modal
        let n_key = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE);
        app.handle_key(n_key);
        assert_eq!(app.modal, Modal::None);
    }
}
