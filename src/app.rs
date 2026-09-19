use std::time::Instant;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use ratatui::widgets::BorderType;

use crate::config::{self, AppConfig};
use crate::fs_tree::{self, FileEntry};
use crate::monitor::{fetch_past_runs, spawn_log_streamer, PastRun, RunStatus, SharedStreamer, StreamerState};
use crate::systemd::{self, get_service_info, ServiceInfo, ServiceState};
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
    #[allow(dead_code)]
    pub fn label(self) -> &'static str {
        match self {
            FocusedPanel::History => "Historique",
            FocusedPanel::Logs => "Logs",
            FocusedPanel::RecentFiles => "Récents",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFilter {
    All,       // Tout
    Files,     // Fichiers (opérations réussies sur des fichiers)
    Problems,  // Problèmes (erreurs, warnings)
}

impl LogFilter {
    pub fn next(self) -> Self {
        match self {
            LogFilter::All => LogFilter::Files,
            LogFilter::Files => LogFilter::Problems,
            LogFilter::Problems => LogFilter::All,
        }
    }
    pub fn prev(self) -> Self {
        match self {
            LogFilter::All => LogFilter::Problems,
            LogFilter::Files => LogFilter::All,
            LogFilter::Problems => LogFilter::Files,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            LogFilter::All => "All",
            LogFilter::Files => "Files",
            LogFilter::Problems => "Problems",
        }
    }
    #[allow(dead_code)]
    pub fn index(self) -> usize {
        match self {
            LogFilter::All => 0,
            LogFilter::Files => 1,
            LogFilter::Problems => 2,
        }
    }
    pub fn from_index(i: usize) -> Self {
        match i {
            1 => LogFilter::Files,
            2 => LogFilter::Problems,
            _ => LogFilter::All,
        }
    }
    /// Returns true if a log line passes this filter
    pub fn matches(self, line: &str) -> bool {
        match self {
            LogFilter::All => true,
            LogFilter::Files => {
                let ll = line.to_lowercase();
                ll.contains("copied")
                    || ll.contains("moved")
                    || ll.contains("deleted")
                    || ll.contains("transferred")
                    || ll.contains("bisync successful")
            }
            LogFilter::Problems => {
                let ll = line.to_lowercase();
                ll.contains("error")
                    || ll.contains("failed")
                    || ll.contains("fatal")
                    || ll.contains("errno")
                    || ll.contains("corrupt")
                    || ll.contains("warn")
                    || ll.contains("skipped")
                    || ll.contains("conflict")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    None,
    OpenEditor,
    OpenFullLogs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Modal {
    None,
    Menu,
    Settings,
    Files,
    Filters,
    DryRun,
    ConfirmSync,
    ConfirmDryRun,
    ConfirmResync,
    ConfirmCancel,
    ConfirmDelete(String),
    Help,
    HistoryDetails(usize),
}

pub use crate::config::TICK_RATE_STEPS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollbarTarget {
    Logs,
    History,
    RecentFiles,
    HistoryDetails(usize),
    Files,
    Filters,
    DryRun,
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
    ButtonHelp,
    ButtonTheme,
    ButtonPanel,
    TickRateDec,
    TickRateInc,
    SparklinePoint(usize),
    ToggleCtrlMode,
    HistoryRow(usize),
    HistoryFile(usize),
    RecentFile(usize),
    RecentFilterFocus,
    LogsArea,
    HistoryArea,
    RecentFilesArea,
    SettingOption(usize),
    SettingCycle(usize, bool),
    SettingsTab(usize),
    SaveSettings,
    CloseModal,
    MenuOption(usize),
    FileEntry(usize),
    FileOpen(usize),
    FileParent,
    ToggleLogsAuto,
    CopyLogs,
    LogFilterTab(usize),
    LogFilterPrev,
    LogFilterNext,
    LogFilterCycle,
    CopyHistoryErrors(usize),
    ButtonCopy,
    FilterArea,
    FilterRow(usize),
    FilterCycleType(usize),
    FilterStartEdit,
    FilterAdd,
    FilterDelete(usize),
    FilterOpenEditor,
    ScrollbarArrowUp(ScrollbarTarget),
    ScrollbarArrowDown(ScrollbarTarget),
    ScrollbarTrack {
        target: ScrollbarTarget,
        top_y: u16,
        track_height: u16,
        total: usize,
        visible: usize,
    },
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
    pub selected_run_idx: Option<usize>,
    pub filters: Vec<String>,
    pub selected_filter_idx: usize,
    pub logs_scroll: usize,
    pub auto_scroll: bool,
    pub log_filter: LogFilter,
    pub toast: Option<(String, Instant)>,

    // btop++ : panel actif et tick rate dynamique
    pub focused_panel: FocusedPanel,
    pub tick_rate_ms_live: u64,
    pub tick_rate_changed: bool,
    pub menu_selected_idx: usize,
    pub ctrl_mode: bool,
    pub recent_filter: String,
    pub is_filtering_recent: bool,

    // Explorateur de fichiers
    pub file_current_rel: String,
    pub file_entries: Vec<FileEntry>,
    pub file_selected_idx: usize,
    pub file_scroll_offset: usize,
    pub file_viewport_height: usize,

    // Scroll offsets et hauteurs de viewport pour historique, filtres, logs et récents
    pub history_scroll_offset: usize,
    pub history_details_scroll: usize,
    pub history_viewport_height: usize,
    pub filter_scroll_offset: usize,
    pub filter_viewport_height: usize,
    pub recent_scroll_offset: usize,
    pub recent_viewport_height: usize,
    pub logs_viewport_height: usize,
    pub logs_total_wrapped: usize,

    // Paramètres
    pub settings_tab: usize,
    pub settings_selected_idx: usize,
    pub is_editing_setting: bool,
    pub setting_edit_buffer: String,

    // Filtres d'exclusion (inline editing)
    pub is_editing_filter: bool,
    pub filter_edit_buffer: String,
    pub is_adding_filter: bool,

    // Simulation Dry-Run
    pub dry_run_running: bool,
    pub dry_run_logs: Vec<String>,
    pub dry_run_scroll: usize,
    pub dry_run_rx: Option<std::sync::mpsc::Receiver<Vec<String>>>,

    // Sélection d'éléments interactifs
    pub history_selected_file_idx: usize,
    pub recent_selected_idx: Option<usize>,
    pub active_scrollbar_drag: Option<(ScrollbarTarget, u16, u16, usize, usize)>,

    // Registre précis des hitboxes cliquables (au pixel près)
    pub hitboxes: Vec<Hitbox>,
    pub active_modal_area: Option<Rect>,

    pub cloud_quota: Option<CloudQuota>,
    pub tracked_files_count: usize,
    quota_rx: Option<std::sync::mpsc::Receiver<CloudQuota>>,
    file_count_rx: Option<std::sync::mpsc::Receiver<usize>>,

    last_systemd_check: Instant,
    last_history_check: Instant,
    last_quota_check: Instant,
    last_file_count_check: Instant,
}

#[derive(Debug, Clone, Default)]
pub struct CloudQuota {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
}

pub fn is_ctrl_x(key: &KeyEvent) -> bool {
    (matches!(key.code, KeyCode::Char('x') | KeyCode::Char('X')) && key.modifiers.contains(KeyModifiers::CONTROL))
        || key.code == KeyCode::Char('\u{18}')
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
            selected_run_idx: None,
            filters,
            selected_filter_idx: 0,
            logs_scroll: 0,
            auto_scroll: true,
            log_filter: LogFilter::All,
            toast: None,

            focused_panel: FocusedPanel::RecentFiles,
            tick_rate_ms_live: tick_rate,
            tick_rate_changed: false,
            menu_selected_idx: 0,
            ctrl_mode: false,
            recent_filter: String::new(),
            is_filtering_recent: false,

            file_current_rel: "".to_string(),
            file_entries: Vec::new(),
            file_selected_idx: 0,
            file_scroll_offset: 0,
            file_viewport_height: 15,

            history_scroll_offset: 0,
            history_details_scroll: 0,
            history_viewport_height: 8,
            filter_scroll_offset: 0,
            filter_viewport_height: 12,
            recent_scroll_offset: 0,
            recent_viewport_height: 6,
            logs_viewport_height: 10,
            logs_total_wrapped: 0,

            settings_tab: 0,
            settings_selected_idx: 0,
            is_editing_setting: false,
            setting_edit_buffer: String::new(),

            is_editing_filter: false,
            filter_edit_buffer: String::new(),
            is_adding_filter: false,

            dry_run_running: false,
            dry_run_logs: Vec::new(),
            dry_run_scroll: 0,
            dry_run_rx: None,

            history_selected_file_idx: 0,
            recent_selected_idx: None,
            active_scrollbar_drag: None,

            hitboxes: Vec::with_capacity(64),
            active_modal_area: None,

            cloud_quota: config::load_quota_cache().map(|c| CloudQuota {
                total_bytes: c.total_bytes,
                used_bytes: c.used_bytes,
                free_bytes: c.free_bytes,
            }),
            tracked_files_count: 0,
            quota_rx: None,
            file_count_rx: None,

            last_systemd_check: Instant::now(),
            last_history_check: Instant::now(),
            last_quota_check: Instant::now().checked_sub(std::time::Duration::from_secs(350)).unwrap_or_else(Instant::now),
            last_file_count_check: Instant::now().checked_sub(std::time::Duration::from_secs(70)).unwrap_or_else(Instant::now),
        };

        app.reload_files();
        app
    }

    #[allow(dead_code)]
    pub fn clear_hitboxes(&mut self) {
        self.hitboxes.clear();
    }

    pub fn border_type(&self) -> BorderType {
        self.config.border_style.to_border_type()
    }

    pub fn border_glyphs(&self) -> crate::config::BorderGlyphs {
        self.config.border_style.glyphs()
    }

    pub fn settings_items_count(&self) -> usize {
        if self.settings_tab == 0 { 7 } else { 6 }
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

        if self.last_systemd_check.elapsed().as_secs() >= 1 {
            self.service_info = get_service_info();
            self.last_systemd_check = Instant::now();
            if self.service_info.state == ServiceState::Idle || self.service_info.state == ServiceState::Failed {
                self.live.is_syncing = false;
                self.live.transfer = crate::monitor::parser::TransferStats::default();
            }
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

        if let Some(rx) = &self.quota_rx {
            if let Ok(q) = rx.try_recv() {
                let _ = config::save_quota_cache(&config::CloudQuotaCache {
                    total_bytes: q.total_bytes,
                    used_bytes: q.used_bytes,
                    free_bytes: q.free_bytes,
                });
                self.cloud_quota = Some(q);
            }
        }

        if let Some(rx) = &self.file_count_rx {
            if let Ok(count) = rx.try_recv() {
                self.tracked_files_count = count;
            }
        }

        if self.last_quota_check.elapsed().as_secs() >= 300 {
            self.last_quota_check = Instant::now();
            let (tx, rx) = std::sync::mpsc::channel();
            self.quota_rx = Some(rx);
            let remote = self.config.remote.clone();
            spawn_quota_fetch(remote, tx);
        }

        if self.last_file_count_check.elapsed().as_secs() >= 60 {
            self.last_file_count_check = Instant::now();
            let (tx, rx) = std::sync::mpsc::channel();
            self.file_count_rx = Some(rx);
            let base = config::expand_tilde(&self.config.local_dir);
            spawn_file_count(std::path::PathBuf::from(base), tx);
        }

        if self.dry_run_running {
            if let Some(rx) = &self.dry_run_rx {
                if let Ok(lines) = rx.try_recv() {
                    self.dry_run_logs = lines;
                    self.dry_run_running = false;
                    self.set_toast("✔ Dry-Run simulation completed!");
                }
            }
        }
    }

    pub fn get_tracked_files_count(&self) -> usize {
        if self.tracked_files_count > 0 {
            self.tracked_files_count
        } else if !self.file_entries.is_empty() {
            self.file_entries.iter().filter(|f| !f.is_dir).count()
        } else {
            0
        }
    }

    pub fn start_dry_run(&mut self) {
        self.dry_run_running = true;
        self.dry_run_logs = vec![
            "Analyzing differences between local folder and Google Drive...".to_string(),
            "Executing: rclone bisync --dry-run -v --tpslimit 8".to_string(),
            "This may take a moment...".to_string(),
        ];
        self.dry_run_scroll = 0;
        self.modal = Modal::DryRun;
        self.set_toast("🛡 Dry-Run simulation started...");

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
                        lines.push("Everything is already in sync (no transfers detected in dry-run).".to_string());
                    }
                }
                Err(e) => {
                    lines.push(format!("Error running rclone: {}", e));
                }
            }
            let _ = tx.send(lines);
        });
    }

    pub fn step_tick_rate(&mut self, faster: bool) {
        let steps = TICK_RATE_STEPS;
        let current = self.tick_rate_ms_live;
        let pos = steps
            .iter()
            .enumerate()
            .min_by_key(|(_, &s)| (s as i64 - current as i64).abs())
            .map(|(i, _)| i)
            .unwrap_or(11);
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
        }
    }

    pub fn can_dec_tick_rate(&self) -> bool {
        self.tick_rate_ms_live > TICK_RATE_STEPS[0]
    }

    pub fn can_inc_tick_rate(&self) -> bool {
        self.tick_rate_ms_live < TICK_RATE_STEPS[TICK_RATE_STEPS.len() - 1]
    }

    pub fn next_theme(&mut self) {
        self.current_theme = self.current_theme.next();
        self.config.theme = Some(self.current_theme);
        let _ = config::save_config(&self.config);
        self.set_toast(format!("Active theme: {}", self.current_theme.name()));
    }

    pub fn open_selected_file(&mut self, rel_path: &str) {
        let base = config::expand_tilde(&self.config.local_dir);
        let clean = rel_path.trim_start_matches('/');
        let full = if clean.is_empty() {
            base.clone()
        } else {
            base.join(clean)
        };
        if !full.exists() {
            self.set_toast("File deleted (not found)");
            return;
        }
        match fs_tree::open_with_xdg(&base, rel_path) {
            Ok(_) => self.set_toast(format!("✔ Opened: {}", rel_path)),
            Err(_) => self.set_toast("File deleted (not found)"),
        }
    }

    pub fn open_selected_folder(&mut self, rel_path: &str) {
        let base = config::expand_tilde(&self.config.local_dir);
        match fs_tree::open_folder_with_xdg(&base, rel_path) {
            Ok(_) => self.set_toast(format!("📁 Parent folder opened for {}", rel_path)),
            Err(e) => self.set_toast(format!("✗ Unable to open folder: {}", e)),
        }
    }

    pub fn is_syncing(&self) -> bool {
        (self.live.is_syncing || self.service_info.state == ServiceState::Active)
            && self.live.phase_index < 5
    }

    pub fn total_history_runs(&self) -> usize {
        if self.is_syncing() {
            self.past_runs.len() + 1
        } else {
            self.past_runs.len()
        }
    }

    pub fn open_history_file(&mut self, file_idx: usize) {
        let run_idx = match self.modal {
            Modal::HistoryDetails(idx) => idx,
            _ => {
                let cur = self.selected_run_idx.unwrap_or(0);
                if self.is_syncing() { cur.saturating_sub(1) } else { cur }
            }
        };
        let path_opt = self.past_runs.get(run_idx).and_then(|r| {
            r.all_affected_files().get(file_idx).map(|(_, p)| p.to_string())
        });
        if let Some(path) = path_opt {
            self.open_selected_file(&path);
        }
    }

    pub fn open_history_folder(&mut self, file_idx: usize) {
        let run_idx = match self.modal {
            Modal::HistoryDetails(idx) => idx,
            _ => {
                let cur = self.selected_run_idx.unwrap_or(0);
                if self.is_syncing() { cur.saturating_sub(1) } else { cur }
            }
        };
        let path_opt = self.past_runs.get(run_idx).and_then(|r| {
            r.all_affected_files().get(file_idx).map(|(_, p)| p.to_string())
        });
        if let Some(path) = path_opt {
            self.open_selected_folder(&path);
        }
    }

    pub fn get_all_recent_files(&self) -> Vec<(String, String, String, String)> {
        let mut list = Vec::new();
        // 1. Fichiers du live stream
        for sf in self.live.synced_files.iter().rev() {
            list.push((sf.action.clone(), sf.path.clone(), String::new(), sf.time.clone()));
        }
        // 2. Fichiers de l'historique complet (jusqu'à 100 fichiers, parité web)
        for run in &self.past_runs {
            if !run.synced_files.is_empty() {
                for (act, path, time) in run.synced_files.iter().rev() {
                    list.push((act.clone(), path.clone(), String::new(), time.clone()));
                }
            } else {
                for f in run.files_copied.iter().rev() {
                    list.push(("new".to_string(), f.clone(), String::new(), run.time.clone()));
                }
                for f in run.files_modified.iter().rev() {
                    list.push(("modified".to_string(), f.clone(), String::new(), run.time.clone()));
                }
                for f in run.files_deleted.iter().rev() {
                    list.push(("deleted".to_string(), f.clone(), String::new(), run.time.clone()));
                }
            }
            if list.len() >= 100 {
                break;
            }
        }
        if !self.recent_filter.is_empty() {
            let q = self.recent_filter.to_lowercase();
            list.retain(|(_, p, _, _)| p.to_lowercase().contains(&q));
        }
        if list.len() > 100 {
            list.truncate(100);
        }

        let base = std::path::PathBuf::from(config::expand_tilde(&self.config.local_dir));
        for item in list.iter_mut() {
            let full = base.join(&item.1);
            if let Ok(meta) = std::fs::metadata(&full) {
                item.2 = fs_tree::format_bytes(meta.len());
            } else {
                item.2 = "--".to_string();
            }
        }

        list
    }

    pub fn get_recent_files_list(&self) -> Vec<String> {
        self.get_all_recent_files().into_iter().map(|(_, p, _, _)| p).collect()
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

    pub fn copy_logs_to_clipboard(&mut self) {
        let filtered: Vec<&String> = self.live.log_lines.iter()
            .filter(|l| self.log_filter.matches(l))
            .collect();
        if filtered.is_empty() {
            self.set_toast("ℹ No logs to copy.");
            return;
        }
        let text = filtered.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n");
        let count = filtered.len();
        if crate::clipboard::copy_to_clipboard(&text) {
            self.set_toast(format!("📋 {} log lines copied to clipboard!", count));
        } else {
            self.set_toast("⚠ Failed to copy to clipboard.");
        }
    }

    /// Returns filtered log lines based on current log_filter
    pub fn filtered_log_lines(&self) -> Vec<&String> {
        self.live.log_lines.iter()
            .filter(|l| self.log_filter.matches(l))
            .collect()
    }

    pub fn copy_history_errors(&mut self, run_idx: usize) {
        if let Some(run) = self.past_runs.get(run_idx) {
            if !run.errors.is_empty() {
                let text = format!(
                    "Run #{} ({} {}) - {} errors detected:\n{}",
                    run.id,
                    run.date,
                    run.time,
                    run.errors.len(),
                    run.errors.iter().map(|e| format!("• {}", e)).collect::<Vec<_>>().join("\n")
                );
                if crate::clipboard::copy_to_clipboard(&text) {
                    self.set_toast(format!("📋 {} error(s) from run #{} copied!", run.errors.len(), run.id));
                } else {
                    self.set_toast("⚠ Failed to copy to clipboard.");
                }
            } else {
                let text = format!(
                    "Run #{} ({} {}) - Status: {:?} (Duration: {})\nFiles copied: {}\nFiles modified: {}\nFiles deleted: {}",
                    run.id,
                    run.date,
                    run.time,
                    run.status,
                    run.duration,
                    run.files_copied.len(),
                    run.files_modified.len(),
                    run.files_deleted.len()
                );
                if crate::clipboard::copy_to_clipboard(&text) {
                    self.set_toast(format!("📋 Run #{} details copied!", run.id));
                } else {
                    self.set_toast("⚠ Failed to copy to clipboard.");
                }
            }
        }
    }

    pub fn copy_selected_recent_file(&mut self) {
        if let Some(idx) = self.recent_selected_idx {
            let list = self.get_recent_files_list();
            if let Some(path) = list.get(idx) {
                if crate::clipboard::copy_to_clipboard(path) {
                    self.set_toast(format!("📋 Path copied: {}", path));
                }
            }
        }
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
                message: "Critical bisync error: corrupted or missing listings.".to_string(),
                hint: Some("Press 'r' to resynchronize (--resync)".to_string()),
            });
        }

        let failures = self.consecutive_failures();
        if failures >= 2 {
            alerts.push(Alert {
                level: AlertLevel::Error,
                message: format!("{} consecutive sync failures!", failures),
                hint: Some("Inspect logs to view error details".to_string()),
            });
        }

        if self.live.is_syncing {
            let duration_s = if let Some(start) = self.live.sync_start {
                start.elapsed().as_secs()
            } else if let Some(proc_secs) = crate::monitor::streamer::get_running_sync_elapsed_seconds() {
                proc_secs
            } else {
                0
            };
            if duration_s > 300 {
                alerts.push(Alert {
                    level: AlertLevel::Warning,
                    message: format!("Abnormally long sync duration ({} min {} s)", duration_s / 60, duration_s % 60),
                    hint: Some("Press 'c' to force abort if stuck".to_string()),
                });
            }
        }

        let disk = self.disk_pct();
        if disk > 90.0 {
            alerts.push(Alert {
                level: AlertLevel::Error,
                message: format!("Critical local disk space ({:.1}% used)!", disk),
                hint: Some("Free up space on the local partition".to_string()),
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
                for err in &run.errors {
                    if err.to_lowercase().contains("conflict") {
                        conflicts += 1;
                    }
                }
            }
        }
        conflicts
    }

    /// Ensure history scroll offset keeps the selected run visible
    #[allow(dead_code)]
    pub fn ensure_history_visible(&mut self, viewport_height: usize) {
        if viewport_height == 0 { return; }
        if let Some(idx) = self.selected_run_idx {
            if idx < self.history_scroll_offset {
                self.history_scroll_offset = idx;
            } else if idx >= self.history_scroll_offset + viewport_height {
                self.history_scroll_offset = idx - viewport_height + 1;
            }
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

    pub fn total_log_lines(&self) -> usize {
        if self.logs_total_wrapped > 0 {
            self.logs_total_wrapped
        } else {
            self.filtered_log_lines().len()
        }
    }

    /// Algorithme universel de navigation et défilement pour les listes indexées (RecentFiles, Filters, Files, History)
    pub fn scroll_list_step(
        selected_idx: &mut usize,
        scroll_offset: &mut usize,
        total: usize,
        viewport_height: usize,
        up: bool,
    ) {
        if total == 0 {
            return;
        }
        let vp = viewport_height.max(1);
        let max_offset = total.saturating_sub(vp);

        // Ajuste la sélection si elle était hors du champ visible
        if *selected_idx < *scroll_offset {
            *selected_idx = *scroll_offset;
        } else if *selected_idx >= *scroll_offset + vp {
            *selected_idx = (*scroll_offset + vp).saturating_sub(1);
        }

        if up {
            if *selected_idx > 0 {
                *selected_idx -= 1;
                if *selected_idx < *scroll_offset {
                    *scroll_offset = *selected_idx;
                }
            }
        } else if *selected_idx < total.saturating_sub(1) {
            *selected_idx += 1;
            if *selected_idx >= *scroll_offset + vp {
                *scroll_offset = (*selected_idx + 1).saturating_sub(vp).min(max_offset);
            }
        }
    }

    /// Algorithme universel de saut sur la scrollbar (clic sur la piste ou drag)
    pub fn scroll_list_jump(
        selected_idx: &mut usize,
        scroll_offset: &mut usize,
        total: usize,
        viewport_height: usize,
        ratio: f64,
    ) {
        if total == 0 {
            return;
        }
        let vp = viewport_height.max(1);
        let max_offset = total.saturating_sub(vp);
        let old_offset = *scroll_offset;
        let new_offset = ((ratio * max_offset as f64).round() as usize).min(max_offset);
        *scroll_offset = new_offset;
        if new_offset > old_offset || ratio >= 0.5 {
            *selected_idx = (new_offset + vp.saturating_sub(1)).min(total.saturating_sub(1));
        } else {
            *selected_idx = new_offset.min(total.saturating_sub(1));
        }
    }

    /// Version pour les listes où la sélection est optionnelle (RecentFiles, History)
    pub fn scroll_opt_list_step(
        selected_idx: &mut Option<usize>,
        scroll_offset: &mut usize,
        total: usize,
        viewport_height: usize,
        up: bool,
    ) {
        if total == 0 {
            return;
        }
        match selected_idx {
            None => {
                if !up {
                    *selected_idx = Some((*scroll_offset).min(total - 1));
                }
            }
            Some(ref mut idx) => {
                if up && *idx == 0 && *scroll_offset == 0 {
                    *selected_idx = None;
                    return;
                }
                Self::scroll_list_step(idx, scroll_offset, total, viewport_height, up);
            }
        }
    }

    pub fn scroll_opt_list_jump(
        selected_idx: &mut Option<usize>,
        scroll_offset: &mut usize,
        total: usize,
        viewport_height: usize,
        ratio: f64,
    ) {
        if total == 0 {
            return;
        }
        let mut current = selected_idx.unwrap_or(0);
        Self::scroll_list_jump(&mut current, scroll_offset, total, viewport_height, ratio);
        *selected_idx = Some(current);
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

    /// Ensure recent files scroll offset keeps selected file visible
    pub fn ensure_recent_visible(&mut self, viewport_height: usize) {
        let vp = viewport_height.max(1);
        if let Some(idx) = self.recent_selected_idx {
            if idx < self.recent_scroll_offset {
                self.recent_scroll_offset = idx;
            } else if idx >= self.recent_scroll_offset + vp {
                self.recent_scroll_offset = idx + 1 - vp;
            }
        }
    }

    pub fn scroll_recent_down(&mut self, viewport_height: usize) {
        let total = self.get_recent_files_list().len();
        Self::scroll_opt_list_step(&mut self.recent_selected_idx, &mut self.recent_scroll_offset, total, viewport_height, false);
    }

    pub fn scroll_recent_up(&mut self, viewport_height: usize) {
        let total = self.get_recent_files_list().len();
        Self::scroll_opt_list_step(&mut self.recent_selected_idx, &mut self.recent_scroll_offset, total, viewport_height, true);
    }

    pub fn scroll_history_down(&mut self, viewport_height: usize) {
        let total = self.total_history_runs();
        if total == 0 {
            return;
        }
        let vp = viewport_height.max(1);
        let max_offset = total.saturating_sub(vp);

        match self.selected_run_idx {
            None => {
                self.selected_run_idx = Some(self.history_scroll_offset.min(total - 1));
            }
            Some(mut idx) => {
                if idx < self.history_scroll_offset {
                    idx = self.history_scroll_offset;
                } else if idx >= self.history_scroll_offset + vp {
                    idx = (self.history_scroll_offset + vp).saturating_sub(1);
                }

                if self.history_scroll_offset < max_offset {
                    self.history_scroll_offset += 1;
                    self.selected_run_idx = Some((idx + 1).min(total - 1));
                } else if idx < total - 1 {
                    self.selected_run_idx = Some(idx + 1);
                }
            }
        }
    }

    pub fn scroll_history_up(&mut self, viewport_height: usize) {
        let total = self.total_history_runs();
        if total == 0 {
            return;
        }
        let vp = viewport_height.max(1);

        match self.selected_run_idx {
            None => {}
            Some(mut idx) => {
                if idx == 0 && self.history_scroll_offset == 0 {
                    self.selected_run_idx = None;
                    return;
                }
                if idx < self.history_scroll_offset {
                    idx = self.history_scroll_offset;
                } else if idx >= self.history_scroll_offset + vp {
                    idx = (self.history_scroll_offset + vp).saturating_sub(1);
                }

                if self.history_scroll_offset > 0 {
                    self.history_scroll_offset -= 1;
                    self.selected_run_idx = Some(idx.saturating_sub(1));
                } else if idx > 0 {
                    self.selected_run_idx = Some(idx - 1);
                }
            }
        }
    }

    pub fn is_dragging_scrollbar(&self, target: ScrollbarTarget) -> bool {
        if let Some((t, _, _, _, _)) = &self.active_scrollbar_drag {
            *t == target
        } else {
            false
        }
    }

    pub fn scroll_filter_down(&mut self, viewport_height: usize) {
        let total = if self.is_adding_filter { self.filters.len() + 1 } else { self.filters.len() };
        Self::scroll_list_step(&mut self.selected_filter_idx, &mut self.filter_scroll_offset, total, viewport_height, false);
    }

    pub fn scroll_filter_up(&mut self, viewport_height: usize) {
        let total = if self.is_adding_filter { self.filters.len() + 1 } else { self.filters.len() };
        Self::scroll_list_step(&mut self.selected_filter_idx, &mut self.filter_scroll_offset, total, viewport_height, true);
    }

    pub fn scroll_file_down(&mut self, viewport_height: usize) {
        let total = self.file_entries.len();
        Self::scroll_list_step(&mut self.file_selected_idx, &mut self.file_scroll_offset, total, viewport_height, false);
    }

    pub fn scroll_file_up(&mut self, viewport_height: usize) {
        let total = self.file_entries.len();
        Self::scroll_list_step(&mut self.file_selected_idx, &mut self.file_scroll_offset, total, viewport_height, true);
    }

    pub fn scroll_settings_down(&mut self) {
        let total = self.settings_items_count();
        if total == 0 { return; }
        if self.settings_selected_idx < total - 1 {
            self.settings_selected_idx += 1;
        }
    }

    pub fn scroll_settings_up(&mut self) {
        if self.settings_selected_idx > 0 {
            self.settings_selected_idx -= 1;
        }
    }

    pub fn dismiss_active_modal(&mut self) {
        if self.is_editing_setting {
            self.commit_setting_edit();
        }
        if self.is_editing_filter {
            self.cancel_filter_edit();
        }
        self.modal = Modal::None;
        self.active_modal_area = None;
    }

    pub fn commit_setting_edit(&mut self) {
        if !self.is_editing_setting {
            return;
        }
        let trimmed = self.setting_edit_buffer.trim().to_string();
        if self.settings_tab == 0 && self.settings_selected_idx == 3 {
            if !trimmed.is_empty() {
                if self.config.local_dir != trimmed {
                    self.config.local_dir = trimmed;
                    self.reload_files();
                    self.save_current_settings();
                    self.set_toast(format!("✔ Local directory updated: {}", self.config.local_dir));
                } else {
                    self.set_toast("✔ Local directory kept");
                }
            } else {
                self.set_toast("ℹ Empty value: keeping previous directory");
            }
        } else if self.settings_tab == 0 && self.settings_selected_idx == 4 {
            if !trimmed.is_empty() {
                if self.config.remote != trimmed {
                    self.config.remote = trimmed;
                    self.save_current_settings();
                    self.set_toast(format!("✔ Remote storage updated: {}", self.config.remote));
                } else {
                    self.set_toast("✔ Remote storage kept");
                }
            } else {
                self.set_toast("ℹ Empty value: keeping previous remote");
            }
        }
        self.is_editing_setting = false;
    }

    pub fn apply_scrollbar_step(&mut self, target: ScrollbarTarget, up: bool) {
        match target {
            ScrollbarTarget::Logs => {
                self.focused_panel = FocusedPanel::Logs;
                if up {
                    let visible_height = self.logs_viewport_height.max(3);
                    let max_scroll = self.total_log_lines().saturating_sub(visible_height);
                    if max_scroll > 0 && self.logs_scroll < max_scroll {
                        self.auto_scroll = false;
                        self.logs_scroll += 1;
                    }
                } else {
                    if self.logs_scroll > 0 {
                        self.logs_scroll = self.logs_scroll.saturating_sub(1);
                        if self.logs_scroll == 0 {
                            self.auto_scroll = true;
                        }
                    }
                }
            }
            ScrollbarTarget::History => {
                self.focused_panel = FocusedPanel::History;
                let vp = self.history_viewport_height.max(2);
                if up {
                    self.scroll_history_up(vp);
                } else {
                    self.scroll_history_down(vp);
                }
            }
            ScrollbarTarget::RecentFiles => {
                self.focused_panel = FocusedPanel::RecentFiles;
                let vp = self.recent_viewport_height.max(1);
                if up {
                    self.scroll_recent_up(vp);
                } else {
                    self.scroll_recent_down(vp);
                }
            }
            ScrollbarTarget::HistoryDetails(run_idx) => {
                if up {
                    if self.history_details_scroll > 0 {
                        self.history_details_scroll -= 1;
                    }
                } else {
                    let total_files = self.past_runs.get(run_idx).map(|r| r.all_affected_files().len()).unwrap_or(0);
                    let max_scroll = total_files.saturating_sub(5);
                    if self.history_details_scroll < max_scroll {
                        self.history_details_scroll += 1;
                    }
                }
            }
            ScrollbarTarget::Files => {
                let vp = self.file_viewport_height.max(1);
                if up {
                    self.scroll_file_up(vp);
                } else {
                    self.scroll_file_down(vp);
                }
            }
            ScrollbarTarget::Filters => {
                let vp = self.filter_viewport_height.max(1);
                if up {
                    self.scroll_filter_up(vp);
                } else {
                    self.scroll_filter_down(vp);
                }
            }
            ScrollbarTarget::DryRun => {
                if up {
                    self.dry_run_scroll = self.dry_run_scroll.saturating_sub(1);
                } else {
                    let max_dry = self.dry_run_logs.len().saturating_sub(5);
                    if self.dry_run_scroll < max_dry {
                        self.dry_run_scroll += 1;
                    }
                }
            }
        }
    }

    pub fn apply_scrollbar_jump(
        &mut self,
        target: ScrollbarTarget,
        click_offset: u16,
        track_height: u16,
        total: usize,
        visible: usize,
    ) {
        if total <= visible || track_height == 0 {
            return;
        }
        let max_scroll = total.saturating_sub(visible);
        let ratio = (click_offset as f64) / ((track_height.saturating_sub(1)).max(1) as f64);
        let ratio = ratio.clamp(0.0, 1.0);

        match target {
            ScrollbarTarget::Logs => {
                self.focused_panel = FocusedPanel::Logs;
                let pos = ((ratio * max_scroll as f64).round() as usize).min(max_scroll);
                let new_logs_scroll = max_scroll.saturating_sub(pos);
                self.logs_scroll = new_logs_scroll;
                self.auto_scroll = new_logs_scroll == 0;
            }
            ScrollbarTarget::History => {
                self.focused_panel = FocusedPanel::History;
                let vp = self.history_viewport_height.max(2);
                Self::scroll_opt_list_jump(&mut self.selected_run_idx, &mut self.history_scroll_offset, total, vp, ratio);
            }
            ScrollbarTarget::RecentFiles => {
                self.focused_panel = FocusedPanel::RecentFiles;
                let vp = self.recent_viewport_height.max(1);
                Self::scroll_opt_list_jump(&mut self.recent_selected_idx, &mut self.recent_scroll_offset, total, vp, ratio);
            }
            ScrollbarTarget::HistoryDetails(run_idx) => {
                self.history_details_scroll = ((ratio * max_scroll as f64).round() as usize).min(max_scroll);
                let total_files = self.past_runs.get(run_idx).map(|r| r.all_affected_files().len()).unwrap_or(0);
                if total_files > 0 {
                    let vp = visible.max(1);
                    if ratio >= 0.5 {
                        self.history_selected_file_idx = (self.history_details_scroll + vp.saturating_sub(1)).min(total_files.saturating_sub(1));
                    } else {
                        self.history_selected_file_idx = self.history_details_scroll.min(total_files.saturating_sub(1));
                    }
                }
            }
            ScrollbarTarget::Files => {
                let vp = self.file_viewport_height.max(1);
                Self::scroll_list_jump(&mut self.file_selected_idx, &mut self.file_scroll_offset, total, vp, ratio);
            }
            ScrollbarTarget::Filters => {
                let vp = self.filter_viewport_height.max(1);
                Self::scroll_list_jump(&mut self.selected_filter_idx, &mut self.filter_scroll_offset, total, vp, ratio);
            }
            ScrollbarTarget::DryRun => {
                self.dry_run_scroll = ((ratio * max_scroll as f64).round() as usize).min(max_scroll);
            }
        }
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent) -> Action {
        match mouse.kind {
            MouseEventKind::ScrollDown => {
                // 1. Si une modale est active : le scroll est confiné à celle-ci et aligné
                if self.modal != Modal::None {
                    match self.modal {
                        Modal::Filters => {
                            let vp = self.filter_viewport_height;
                            self.scroll_filter_down(vp);
                        }
                        Modal::Settings => {
                            self.scroll_settings_down();
                        }
                        Modal::Files => {
                            let vp = self.file_viewport_height;
                            self.scroll_file_down(vp);
                        }
                        Modal::HistoryDetails(past_idx) => {
                            let total_files = self.past_runs.get(past_idx).map(|r| r.all_affected_files().len()).unwrap_or(0);
                            let max_scroll = total_files.saturating_sub(5);
                            if self.history_details_scroll < max_scroll {
                                self.history_details_scroll += 1;
                            }
                        }
                        Modal::DryRun => {
                            let max_dry = self.dry_run_logs.len().saturating_sub(5);
                            if self.dry_run_scroll < max_dry {
                                self.dry_run_scroll += 1;
                            }
                        }
                        _ => {}
                    }
                    return Action::None;
                }

                // 2. Mode dashboard : scroll sur le conteneur sous la souris
                let col = mouse.column;
                let row = mouse.row;
                let mut handled = false;
                for hb in self.hitboxes.iter().rev() {
                    if hb.rect.x <= col && col < hb.rect.x + hb.rect.width
                        && hb.rect.y <= row && row < hb.rect.y + hb.rect.height
                    {
                        match hb.action {
                            HitAction::LogsArea => {
                                self.focused_panel = FocusedPanel::Logs;
                                if self.logs_scroll > 0 {
                                    self.logs_scroll = self.logs_scroll.saturating_sub(1);
                                    if self.logs_scroll == 0 {
                                        self.auto_scroll = true;
                                    }
                                }
                                handled = true;
                                break;
                            }
                            HitAction::HistoryArea | HitAction::HistoryRow(_) => {
                                self.focused_panel = FocusedPanel::History;
                                let vp = self.history_viewport_height.max(3);
                                self.scroll_history_down(vp);
                                handled = true;
                                break;
                            }
                            HitAction::RecentFilesArea | HitAction::RecentFile(_) => {
                                self.focused_panel = FocusedPanel::RecentFiles;
                                let vp = self.recent_viewport_height.max(1);
                                self.scroll_recent_down(vp);
                                handled = true;
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                if handled {
                    return Action::None;
                }
            }
            MouseEventKind::ScrollUp => {
                // 1. Si une modale est active : le scroll est confiné à celle-ci et aligné
                if self.modal != Modal::None {
                    match self.modal {
                        Modal::Filters => {
                            let vp = self.filter_viewport_height;
                            self.scroll_filter_up(vp);
                        }
                        Modal::Settings => {
                            self.scroll_settings_up();
                        }
                        Modal::Files => {
                            let vp = self.file_viewport_height;
                            self.scroll_file_up(vp);
                        }
                        Modal::HistoryDetails(_) => {
                            if self.history_details_scroll > 0 {
                                self.history_details_scroll -= 1;
                            }
                        }
                        Modal::DryRun => {
                            self.dry_run_scroll = self.dry_run_scroll.saturating_sub(1);
                        }
                        _ => {}
                    }
                    return Action::None;
                }

                // 2. Mode dashboard : scroll sur le conteneur sous la souris
                let col = mouse.column;
                let row = mouse.row;
                let mut handled = false;
                for hb in self.hitboxes.iter().rev() {
                    if hb.rect.x <= col && col < hb.rect.x + hb.rect.width
                        && hb.rect.y <= row && row < hb.rect.y + hb.rect.height
                    {
                        match hb.action {
                            HitAction::LogsArea => {
                                self.focused_panel = FocusedPanel::Logs;
                                let visible_height = self.logs_viewport_height.max(3);
                                let max_scroll = self.total_log_lines().saturating_sub(visible_height);
                                if max_scroll > 0 && self.logs_scroll < max_scroll {
                                    self.auto_scroll = false;
                                    self.logs_scroll += 1;
                                }
                                handled = true;
                                break;
                            }
                            HitAction::HistoryArea | HitAction::HistoryRow(_) => {
                                self.focused_panel = FocusedPanel::History;
                                let vp = self.history_viewport_height.max(3);
                                self.scroll_history_up(vp);
                                handled = true;
                                break;
                            }
                            HitAction::RecentFilesArea | HitAction::RecentFile(_) => {
                                self.focused_panel = FocusedPanel::RecentFiles;
                                let vp = self.recent_viewport_height.max(1);
                                self.scroll_recent_up(vp);
                                handled = true;
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                if handled {
                    return Action::None;
                }
                self.auto_scroll = false;
                let max_scroll = self.total_log_lines().saturating_sub(self.logs_viewport_height.max(3));
                self.logs_scroll = (self.logs_scroll + 1).min(max_scroll);
            }
            MouseEventKind::Down(MouseButton::Left) => {
                let col = mouse.column;
                let row = mouse.row;
                let is_ctrl = self.ctrl_mode || mouse.modifiers.contains(KeyModifiers::CONTROL);

                // 1. Si une modale est ouverte : gestion unifiée du clic extérieur (click-outside)
                if self.modal != Modal::None {
                    if let Some(modal_rect) = self.active_modal_area {
                        let inside = col >= modal_rect.x
                            && col < modal_rect.x + modal_rect.width
                            && row >= modal_rect.y
                            && row < modal_rect.y + modal_rect.height;

                        if !inside {
                            // Clic en dehors du panel actif : fermer la modale
                            self.dismiss_active_modal();
                            return Action::None;
                        }
                    }

                    // Clic à l'intérieur de la modale : tester uniquement les hitboxes de la modale
                    for hb in self.hitboxes.iter().rev() {
                        if hb.rect.x <= col && col < hb.rect.x + hb.rect.width
                            && hb.rect.y <= row && row < hb.rect.y + hb.rect.height
                        {
                            return self.execute_hit_action(hb.action, is_ctrl, row);
                        }
                    }
                    // Clic à l'intérieur mais sans hitbox : absorber sans fuite
                    return Action::None;
                }

                // 2. Mode dashboard standard (aucune modale active)
                for hb in self.hitboxes.iter().rev() {
                    if hb.rect.x <= col && col < hb.rect.x + hb.rect.width
                        && hb.rect.y <= row && row < hb.rect.y + hb.rect.height
                    {
                        return self.execute_hit_action(hb.action, is_ctrl, row);
                    }
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                let row = mouse.row;
                if let Some((target, top_y, track_height, total, visible)) = self.active_scrollbar_drag {
                    let click_offset = row.saturating_sub(top_y).min(track_height.saturating_sub(1));
                    self.apply_scrollbar_jump(target, click_offset, track_height, total, visible);
                    return Action::None;
                }
                let col = mouse.column;
                for hb in self.hitboxes.iter().rev() {
                    if hb.rect.x <= col && col < hb.rect.x + hb.rect.width
                        && hb.rect.y <= row && row < hb.rect.y + hb.rect.height
                    {
                        if let HitAction::ScrollbarTrack { target, top_y, track_height, total, visible } = hb.action {
                            self.active_scrollbar_drag = Some((target, top_y, track_height, total, visible));
                            let click_offset = row.saturating_sub(top_y).min(track_height.saturating_sub(1));
                            self.apply_scrollbar_jump(target, click_offset, track_height, total, visible);
                            return Action::None;
                        }
                    }
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.active_scrollbar_drag = None;
            }
            _ => {}
        }
        Action::None
    }

    pub fn execute_hit_action(&mut self, action: HitAction, is_ctrl: bool, row: u16) -> Action {
        match action {
            HitAction::ButtonMenu => {
                self.menu_selected_idx = 0;
                self.modal = Modal::Menu;
                Action::None
            }
            HitAction::ButtonSync => {
                self.modal = Modal::ConfirmSync;
                Action::None
            }
            HitAction::ButtonCancel => {
                self.modal = Modal::ConfirmCancel;
                Action::None
            }
            HitAction::ButtonDryRun => {
                self.modal = Modal::ConfirmDryRun;
                Action::None
            }
            HitAction::ButtonFiles => {
                self.modal = if self.modal == Modal::Files { Modal::None } else { Modal::Files };
                Action::None
            }
            HitAction::ButtonFilters => {
                self.modal = if self.modal == Modal::Filters { Modal::None } else { Modal::Filters };
                Action::None
            }
            HitAction::ButtonSettings => {
                self.modal = if self.modal == Modal::Settings { Modal::None } else { Modal::Settings };
                Action::None
            }
            HitAction::ButtonQuit => {
                self.running = false;
                Action::None
            }
            HitAction::ButtonHelp => {
                self.modal = if self.modal == Modal::Help { Modal::None } else { Modal::Help };
                Action::None
            }
            HitAction::ButtonTheme => {
                self.next_theme();
                Action::None
            }
            HitAction::ButtonPanel => {
                self.focused_panel = self.focused_panel.next();
                Action::None
            }
            HitAction::TickRateDec => {
                self.step_tick_rate(true);
                Action::None
            }
            HitAction::TickRateInc => {
                self.step_tick_rate(false);
                Action::None
            }
            HitAction::SparklinePoint(idx) => {
                if idx < self.past_runs.len() {
                    self.selected_run_idx = Some(idx);
                    let run = &self.past_runs[idx];
                    let st = match run.status {
                        RunStatus::Success => "✓ Success",
                        RunStatus::Failed => "✗ Error",
                        RunStatus::Skipped => "○ Skipped",
                        RunStatus::Running => "⟳ Running",
                    };
                    let files_count = run.files_copied.len() + run.files_modified.len() + run.files_deleted.len();
                    self.set_toast(format!("{} — {} · {} file(s) · {}", run.time, run.duration, files_count, st));
                }
                Action::None
            }
            HitAction::CloseModal => {
                self.dismiss_active_modal();
                Action::None
            }
            HitAction::ToggleCtrlMode => {
                self.ctrl_mode = !self.ctrl_mode;
                if self.ctrl_mode {
                    self.set_toast("✔ Folder Mode (Ctrl) ACTIVE: showing folders");
                } else {
                    self.set_toast("Standard File Mode");
                }
                Action::None
            }
            HitAction::MenuOption(idx) => {
                match idx {
                    0 => { self.modal = Modal::Settings; }
                    1 => { self.modal = Modal::Help; }
                    2 => { self.running = false; }
                    _ => {}
                }
                Action::None
            }
            HitAction::HistoryArea => {
                self.focused_panel = FocusedPanel::History;
                Action::None
            }
            HitAction::LogsArea => {
                self.focused_panel = FocusedPanel::Logs;
                Action::None
            }
            HitAction::RecentFilesArea => {
                self.focused_panel = FocusedPanel::RecentFiles;
                Action::None
            }
            HitAction::HistoryRow(idx) => {
                self.focused_panel = FocusedPanel::History;
                let total = self.total_history_runs();
                if idx < total {
                    if self.selected_run_idx == Some(idx) {
                        if self.is_syncing() && idx == 0 {
                            self.set_toast("ℹ Active sync - Details shown above");
                        } else {
                            let past_idx = if self.is_syncing() { idx.saturating_sub(1) } else { idx };
                            if past_idx < self.past_runs.len() {
                                self.history_details_scroll = 0;
                                self.history_selected_file_idx = 0;
                                self.modal = Modal::HistoryDetails(past_idx);
                            }
                        }
                    } else {
                        self.selected_run_idx = Some(idx);
                    }
                }
                Action::None
            }
            HitAction::HistoryFile(idx) => {
                self.history_selected_file_idx = idx;
                if is_ctrl {
                    self.open_history_folder(idx);
                } else {
                    self.open_history_file(idx);
                }
                Action::None
            }
            HitAction::RecentFile(idx) => {
                self.focused_panel = FocusedPanel::RecentFiles;
                if self.recent_selected_idx == Some(idx) {
                    if is_ctrl {
                        self.open_recent_folder(idx);
                    } else {
                        self.open_recent_file(idx);
                    }
                } else {
                    self.recent_selected_idx = Some(idx);
                }
                Action::None
            }
            HitAction::RecentFilterFocus => {
                self.focused_panel = FocusedPanel::RecentFiles;
                self.is_filtering_recent = true;
                Action::None
            }
            HitAction::SettingsTab(tab_idx) => {
                if self.is_editing_setting {
                    self.commit_setting_edit();
                }
                self.settings_tab = tab_idx;
                self.settings_selected_idx = 0;
                Action::None
            }
            HitAction::SettingOption(idx) => {
                if self.is_editing_setting && idx != self.settings_selected_idx {
                    self.commit_setting_edit();
                }
                self.settings_selected_idx = idx;
                if self.settings_tab == 0 {
                    if idx == 6 {
                        return Action::OpenFullLogs;
                    } else if idx == 5 {
                        self.modal = Modal::ConfirmResync;
                    } else if idx == 3 || idx == 4 {
                        if !self.is_editing_setting || self.settings_selected_idx != idx {
                            self.is_editing_setting = true;
                            self.setting_edit_buffer = if idx == 3 {
                                self.config.local_dir.clone()
                            } else {
                                self.config.remote.clone()
                            };
                        }
                    } else {
                        self.cycle_setting(true);
                    }
                } else {
                    self.cycle_setting(true);
                }
                Action::None
            }
            HitAction::SettingCycle(idx, forward) => {
                if self.is_editing_setting && idx != self.settings_selected_idx {
                    self.commit_setting_edit();
                }
                self.settings_selected_idx = idx;
                if self.settings_tab == 0 {
                    if idx == 6 {
                        return Action::OpenFullLogs;
                    } else if idx == 5 {
                        self.modal = Modal::ConfirmResync;
                    } else if idx == 3 || idx == 4 {
                        if !self.is_editing_setting || self.settings_selected_idx != idx {
                            self.is_editing_setting = true;
                            self.setting_edit_buffer = if idx == 3 {
                                self.config.local_dir.clone()
                            } else {
                                self.config.remote.clone()
                            };
                        }
                    } else {
                        self.cycle_setting(forward);
                    }
                } else {
                    self.cycle_setting(forward);
                }
                Action::None
            }
            HitAction::SaveSettings => {
                self.commit_setting_edit();
                self.save_current_settings();
                Action::None
            }
            HitAction::FileEntry(idx) => {
                self.file_selected_idx = idx;
                if is_ctrl {
                    if let Some(entry) = self.file_entries.get(idx) {
                        let base = config::expand_tilde(&self.config.local_dir);
                        let _ = fs_tree::open_folder_with_xdg(&base, &entry.rel_path);
                    }
                } else {
                    self.enter_selected_file_or_dir();
                }
                Action::None
            }
            HitAction::FileOpen(idx) => {
                self.file_selected_idx = idx;
                self.enter_selected_file_or_dir();
                Action::None
            }
            HitAction::FileParent => {
                self.parent_file_dir();
                Action::None
            }
            HitAction::ToggleLogsAuto => {
                self.auto_scroll = !self.auto_scroll;
                Action::None
            }
            HitAction::FilterRow(idx) => {
                if idx < self.filters.len() {
                    if self.is_editing_filter && self.selected_filter_idx != idx {
                        self.commit_filter_edit();
                    }
                    self.selected_filter_idx = idx;
                    let vp = self.filter_viewport_height;
                    self.ensure_filter_visible(vp);
                }
                Action::None
            }
            HitAction::FilterCycleType(idx) => {
                if !self.is_editing_filter {
                    self.cycle_filter_type(idx);
                }
                Action::None
            }
            HitAction::FilterStartEdit => {
                self.start_editing_filter();
                Action::None
            }
            HitAction::FilterAdd => {
                self.start_adding_filter();
                Action::None
            }
            HitAction::FilterDelete(idx) => {
                if idx < self.filters.len() {
                    self.selected_filter_idx = idx;
                    self.delete_selected_filter();
                }
                Action::None
            }
            HitAction::FilterOpenEditor => Action::OpenEditor,
            HitAction::FilterArea => Action::None,
            HitAction::CopyLogs => {
                self.copy_logs_to_clipboard();
                Action::None
            }
            HitAction::LogFilterTab(idx) => {
                self.log_filter = LogFilter::from_index(idx);
                self.focused_panel = FocusedPanel::Logs;
                Action::None
            }
            HitAction::LogFilterPrev => {
                self.log_filter = self.log_filter.prev();
                self.focused_panel = FocusedPanel::Logs;
                Action::None
            }
            HitAction::LogFilterNext | HitAction::LogFilterCycle => {
                self.log_filter = self.log_filter.next();
                self.focused_panel = FocusedPanel::Logs;
                Action::None
            }
            HitAction::CopyHistoryErrors(idx) => {
                self.copy_history_errors(idx);
                Action::None
            }
            HitAction::ButtonCopy => {
                match self.focused_panel {
                    FocusedPanel::Logs => self.copy_logs_to_clipboard(),
                    FocusedPanel::History => {
                        if let Some(sel) = self.selected_run_idx {
                            let past_idx = if self.is_syncing() { sel.saturating_sub(1) } else { sel };
                            self.copy_history_errors(past_idx);
                        } else {
                            self.copy_logs_to_clipboard();
                        }
                    }
                    FocusedPanel::RecentFiles => self.copy_selected_recent_file(),
                }
                Action::None
            }
            HitAction::ScrollbarArrowUp(target) => {
                self.apply_scrollbar_step(target, true);
                Action::None
            }
            HitAction::ScrollbarArrowDown(target) => {
                self.apply_scrollbar_step(target, false);
                Action::None
            }
            HitAction::ScrollbarTrack { target, top_y, track_height, total, visible } => {
                self.active_scrollbar_drag = Some((target, top_y, track_height, total, visible));
                let click_offset = row.saturating_sub(top_y).min(track_height.saturating_sub(1));
                self.apply_scrollbar_jump(target, click_offset, track_height, total, visible);
                Action::None
            }
        }
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
                let res = if self.ctrl_mode {
                    fs_tree::open_folder_with_xdg(&base, &entry.rel_path)
                } else {
                    fs_tree::open_with_xdg(&base, &entry.rel_path)
                };
                match res {
                    Ok(_) => {
                        if self.ctrl_mode {
                            self.set_toast(format!("✔ Folder opened for {}", entry.name));
                        } else {
                            self.set_toast(format!("✔ Opening {}", entry.name));
                        }
                    }
                    Err(e) => self.set_toast(format!("✗ Error: {}", e)),
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

    pub fn cycle_filter_type(&mut self, idx: usize) {
        if idx < self.filters.len() {
            self.selected_filter_idx = idx;
            let rule = &self.filters[idx];
            let trimmed = rule.trim();
            let new_rule = if trimmed.starts_with('+') {
                format!("- {}", trimmed.trim_start_matches('+').trim())
            } else if trimmed.starts_with('-') {
                format!("# {}", trimmed.trim_start_matches('-').trim())
            } else if trimmed.starts_with('#') {
                trimmed.trim_start_matches('#').trim().to_string()
            } else {
                format!("+ {}", trimmed)
            };
            self.filters[idx] = new_rule;
            let _ = config::save_filters(&self.filters);
            self.set_toast("✔ Filter rule type changed");
        }
    }

    pub fn start_editing_filter(&mut self) {
        if self.filters.is_empty() {
            self.start_adding_filter();
            return;
        }
        if self.selected_filter_idx < self.filters.len() {
            self.is_editing_filter = true;
            self.is_adding_filter = false;
            self.filter_edit_buffer = self.filters[self.selected_filter_idx].clone();
        }
    }

    pub fn start_adding_filter(&mut self) {
        self.is_editing_filter = true;
        self.is_adding_filter = true;
        self.filter_edit_buffer = "- ".to_string();
        self.selected_filter_idx = self.filters.len();
        let vp = self.filter_viewport_height;
        self.ensure_filter_visible(vp);
    }

    pub fn commit_filter_edit(&mut self) {
        if !self.is_editing_filter {
            return;
        }
        let trimmed = self.filter_edit_buffer.trim();
        if trimmed.is_empty() {
            self.cancel_filter_edit();
            return;
        }

        if self.is_adding_filter {
            self.filters.push(self.filter_edit_buffer.trim().to_string());
            self.selected_filter_idx = self.filters.len().saturating_sub(1);
            let _ = config::save_filters(&self.filters);
            self.set_toast("✔ Filter rule added");
        } else if self.selected_filter_idx < self.filters.len() {
            self.filters[self.selected_filter_idx] = self.filter_edit_buffer.trim().to_string();
            let _ = config::save_filters(&self.filters);
            self.set_toast("✔ Filter rule updated");
        }
        self.is_editing_filter = false;
        self.is_adding_filter = false;
        self.filter_edit_buffer.clear();
        let vp = self.filter_viewport_height;
        self.ensure_filter_visible(vp);
    }

    pub fn cancel_filter_edit(&mut self) {
        self.is_editing_filter = false;
        self.is_adding_filter = false;
        self.filter_edit_buffer.clear();
        if self.selected_filter_idx >= self.filters.len() && !self.filters.is_empty() {
            self.selected_filter_idx = self.filters.len() - 1;
        }
    }

    pub fn delete_selected_filter(&mut self) {
        if self.filters.is_empty() {
            return;
        }
        if self.selected_filter_idx < self.filters.len() {
            self.filters.remove(self.selected_filter_idx);
            if self.selected_filter_idx >= self.filters.len() && !self.filters.is_empty() {
                self.selected_filter_idx = self.filters.len() - 1;
            }
            let _ = config::save_filters(&self.filters);
            self.set_toast("✔ Filter rule deleted");
            let vp = self.filter_viewport_height;
            self.ensure_filter_visible(vp);
        }
    }

    pub fn cycle_setting(&mut self, forward: bool) {
        if self.settings_tab == 0 {
            // Rclone settings
            match self.settings_selected_idx {
                0 => {
                    let options = config::TIMER_INTERVAL_OPTIONS;
                    let pos = options.iter().position(|&o| o == self.config.timer_interval).unwrap_or(0);
                    let next = if forward { (pos + 1) % options.len() } else { (pos + options.len() - 1) % options.len() };
                    self.config.timer_interval = options[next].to_string();
                }
                1 => {
                    let options = config::FULL_SYNC_OPTIONS;
                    let pos = options.iter().position(|o| o.value == self.config.full_sync_interval).unwrap_or(0);
                    let next = if forward { (pos + 1) % options.len() } else { (pos + options.len() - 1) % options.len() };
                    self.config.full_sync_interval = options[next].value.to_string();
                }
                2 => {
                    let options = config::BWLIMIT_OPTIONS;
                    let cur = self.config.bwlimit.as_deref().unwrap_or("Disabled");
                    let cur = if cur == "Désactivé" { "Disabled" } else { cur };
                    let pos = options.iter().position(|&o| o == cur).unwrap_or(0);
                    let next = if forward { (pos + 1) % options.len() } else { (pos + options.len() - 1) % options.len() };
                    self.config.bwlimit = if options[next] == "Disabled" { None } else { Some(options[next].to_string()) };
                }
                5 => {
                    self.modal = Modal::ConfirmResync;
                }
                _ => {}
            }
            if self.settings_selected_idx <= 2 {
                self.save_current_settings();
            }
        } else {
            // UI settings
            match self.settings_selected_idx {
                0 => {
                    self.current_theme = if forward {
                        self.current_theme.next()
                    } else {
                        self.current_theme.prev()
                    };
                    self.config.theme = Some(self.current_theme);
                    self.set_toast(format!("Active theme: {}", self.current_theme.name()));
                }
                1 => {
                    self.config.container_layout = if forward {
                        self.config.container_layout.next()
                    } else {
                        self.config.container_layout.prev()
                    };
                    self.set_toast(format!("Container layout: {}", self.config.container_layout.name()));
                }
                2 => {
                    self.config.mid_panel_order = if forward {
                        self.config.mid_panel_order.next()
                    } else {
                        self.config.mid_panel_order.prev()
                    };
                    self.set_toast(format!("Mid-panel order: {}", self.config.mid_panel_order.name()));
                }
                3 => {
                    self.config.border_style = if forward {
                        self.config.border_style.next()
                    } else {
                        self.config.border_style.prev()
                    };
                    self.set_toast(format!("Border style: {}", self.config.border_style.name()));
                }
                4 => {
                    self.config.graph_style = if forward {
                        self.config.graph_style.next()
                    } else {
                        self.config.graph_style.prev()
                    };
                    self.set_toast(format!("Graph style: {}", self.config.graph_style.name()));
                }
                5 => {
                    let options = TICK_RATE_STEPS;
                    let cur = self.config.tick_rate_ms.unwrap_or(250);
                    let pos = options.iter().position(|&o| o == cur).unwrap_or(2);
                    let next = if forward { (pos + 1) % options.len() } else { (pos + options.len() - 1) % options.len() };
                    self.config.tick_rate_ms = Some(options[next]);
                    self.tick_rate_ms_live = options[next];
                    self.tick_rate_changed = true;
                }
                _ => {}
            }
            self.save_current_settings();
        }
    }

    pub fn save_current_settings(&mut self) {
        match config::save_config(&self.config) {
            Ok(_) => self.set_toast("✔ Settings saved to dash-config.json!"),
            Err(e) => self.set_toast(format!("✗ Save error: {}", e)),
        }
    }


    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        // Raccourci universel Ctrl+X pour basculer le mode dossier parent
        if is_ctrl_x(&key) {
            self.ctrl_mode = !self.ctrl_mode;
            if self.ctrl_mode {
                self.set_toast("📁 Folder mode active (folder paths shown)");
            } else {
                self.set_toast("📄 File mode active (file paths shown)");
            }
            return Action::None;
        }

        // Saisie en cours pour le filtre des fichiers récents (style btop)
        if self.is_filtering_recent && self.modal == Modal::None {
            match key.code {
                KeyCode::Esc => {
                    self.recent_filter.clear();
                    self.is_filtering_recent = false;
                    self.recent_selected_idx = None;
                    self.recent_scroll_offset = 0;
                    return Action::None;
                }
                KeyCode::Enter => {
                    self.is_filtering_recent = false;
                    return Action::None;
                }
                KeyCode::Backspace => {
                    self.recent_filter.pop();
                    self.recent_selected_idx = if self.get_recent_files_list().is_empty() { None } else { Some(0) };
                    self.recent_scroll_offset = 0;
                    return Action::None;
                }
                KeyCode::Char(c) => {
                    self.recent_filter.push(c);
                    self.recent_selected_idx = if self.get_recent_files_list().is_empty() { None } else { Some(0) };
                    self.recent_scroll_offset = 0;
                    return Action::None;
                }
                _ => return Action::None,
            }
        }

        // 1. Modales prioritaires
        if self.modal != Modal::None {
            // Touche universelle 'q' pour fermer n'importe quelle modale (sauf Menu où 'q' quitte l'application)
            if key.code == KeyCode::Char('q') && self.modal != Modal::Menu && !self.is_editing_setting && !self.is_editing_filter {
                self.modal = Modal::None;
                return Action::None;
            }

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
                    KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => {
                        if self.menu_selected_idx < 2 {
                            self.menu_selected_idx += 1;
                        }
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        match self.menu_selected_idx {
                            0 => { self.modal = Modal::Settings; }
                            1 => { self.modal = Modal::Help; }
                            2 => { self.running = false; }
                            _ => {}
                        }
                    }
                    KeyCode::Char('o') => { self.modal = Modal::Settings; }
                    KeyCode::Char('?') | KeyCode::Char('h') => { self.modal = Modal::Help; }
                    KeyCode::Char('q') => { self.running = false; }
                    _ => {}
                },
                Modal::ConfirmSync => match key.code {
                    KeyCode::Char('y') | KeyCode::Char('o') | KeyCode::Enter => {
                        self.modal = Modal::None;
                        match systemd::trigger_sync() {
                            Ok(_) => self.set_toast("✔ Forced sync initiated..."),
                            Err(e) => self.set_toast(format!("✗ Error: {}", e)),
                        }
                    }
                    KeyCode::Char('n') | KeyCode::Esc | KeyCode::Char('q') => {
                        self.modal = Modal::None;
                    }
                    _ => {}
                },
                Modal::ConfirmDryRun => match key.code {
                    KeyCode::Char('y') | KeyCode::Char('o') | KeyCode::Enter => {
                        self.start_dry_run();
                    }
                    KeyCode::Char('n') | KeyCode::Esc | KeyCode::Char('q') => {
                        self.modal = Modal::None;
                    }
                    _ => {}
                },
                Modal::ConfirmResync => match key.code {
                    KeyCode::Char('y') | KeyCode::Char('o') | KeyCode::Enter => {
                        self.modal = Modal::None;
                        match systemd::trigger_resync() {
                            Ok(_) => self.set_toast("✔ Full resynchronization initiated (--resync)!"),
                            Err(e) => self.set_toast(format!("✗ Error: {}", e)),
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
                            Ok(_) => self.set_toast("✔ Active synchronization aborted!"),
                            Err(e) => self.set_toast(format!("✗ Error: {}", e)),
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
                                    self.set_toast(format!("✔ {} deleted", rel_clone));
                                    self.reload_files();
                                }
                                Err(e) => self.set_toast(format!("✗ Error: {}", e)),
                            }
                        }
                        KeyCode::Char('n') | KeyCode::Esc | KeyCode::Char('q') => {
                            self.modal = Modal::None;
                        }
                        _ => {}
                    }
                }
                Modal::Settings => {
                    if self.is_editing_setting {
                        match key.code {
                            KeyCode::Esc | KeyCode::Enter => {
                                self.commit_setting_edit();
                            }
                            KeyCode::Backspace => {
                                self.setting_edit_buffer.pop();
                            }
                            KeyCode::Char(c) => {
                                self.setting_edit_buffer.push(c);
                            }
                            _ => {}
                        }
                        return Action::None;
                    }

                    match key.code {
                        KeyCode::Esc => {
                            self.modal = Modal::None;
                        }
                        KeyCode::Tab | KeyCode::BackTab => {
                            self.settings_tab = if self.settings_tab == 0 { 1 } else { 0 };
                            self.settings_selected_idx = 0;
                        }
                        KeyCode::Char('1') => {
                            if self.settings_tab != 0 {
                                self.settings_tab = 0;
                                self.settings_selected_idx = 0;
                            }
                        }
                        KeyCode::Char('2') => {
                            if self.settings_tab != 1 {
                                self.settings_tab = 1;
                                self.settings_selected_idx = 0;
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if self.settings_selected_idx > 0 {
                                self.settings_selected_idx -= 1;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            let max_count = self.settings_items_count();
                            if self.settings_selected_idx < max_count.saturating_sub(1) {
                                self.settings_selected_idx += 1;
                            }
                        }
                        KeyCode::Enter => {
                            if self.settings_tab == 0 {
                                if self.settings_selected_idx == 6 {
                                    return Action::OpenFullLogs;
                                } else if self.settings_selected_idx == 5 {
                                    self.modal = Modal::ConfirmResync;
                                } else if self.settings_selected_idx == 3 || self.settings_selected_idx == 4 {
                                    self.is_editing_setting = true;
                                    self.setting_edit_buffer = if self.settings_selected_idx == 3 {
                                        self.config.local_dir.clone()
                                    } else {
                                        self.config.remote.clone()
                                    };
                                } else {
                                    self.cycle_setting(true);
                                }
                            } else {
                                self.cycle_setting(true);
                            }
                        }
                        KeyCode::Char('e') | KeyCode::Char(' ') => {
                            if self.settings_tab == 0 && (self.settings_selected_idx == 3 || self.settings_selected_idx == 4) {
                                self.is_editing_setting = true;
                                self.setting_edit_buffer = if self.settings_selected_idx == 3 {
                                    self.config.local_dir.clone()
                                } else {
                                    self.config.remote.clone()
                                };
                            } else {
                                self.cycle_setting(true);
                            }
                        }
                        KeyCode::Right | KeyCode::Char('l') => {
                            if self.settings_tab == 0 {
                                if self.settings_selected_idx == 6 {
                                    return Action::OpenFullLogs;
                                } else if self.settings_selected_idx == 5 {
                                    self.modal = Modal::ConfirmResync;
                                } else if self.settings_selected_idx == 3 || self.settings_selected_idx == 4 {
                                    self.is_editing_setting = true;
                                    self.setting_edit_buffer = if self.settings_selected_idx == 3 {
                                        self.config.local_dir.clone()
                                    } else {
                                        self.config.remote.clone()
                                    };
                                } else {
                                    self.cycle_setting(true);
                                }
                            } else {
                                self.cycle_setting(true);
                            }
                        }
                        KeyCode::Left | KeyCode::Char('h') => {
                            if self.settings_tab == 0 {
                                if self.settings_selected_idx == 6 {
                                    return Action::OpenFullLogs;
                                } else if self.settings_selected_idx == 5 {
                                    self.modal = Modal::ConfirmResync;
                                } else if self.settings_selected_idx == 3 || self.settings_selected_idx == 4 {
                                    self.is_editing_setting = true;
                                    self.setting_edit_buffer = if self.settings_selected_idx == 3 {
                                        self.config.local_dir.clone()
                                    } else {
                                        self.config.remote.clone()
                                    };
                                } else {
                                    self.cycle_setting(false);
                                }
                            } else {
                                self.cycle_setting(false);
                            }
                        }
                        _ => {}
                    }
                }
                Modal::Files => match key.code {
                    KeyCode::Esc => {
                        self.modal = Modal::None;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        let vp = self.file_viewport_height;
                        self.scroll_file_up(vp);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        let vp = self.file_viewport_height;
                        self.scroll_file_down(vp);
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
                        self.enter_selected_file_or_dir();
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        if let Some(entry) = self.file_entries.get(self.file_selected_idx) {
                            if entry.is_dir {
                                self.enter_selected_file_or_dir();
                            }
                        }
                    }
                    KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h') => {
                        self.parent_file_dir();
                    }
                    KeyCode::Char('d') => {
                        if let Some(entry) = self.file_entries.get(self.file_selected_idx) {
                            let base = config::expand_tilde(&self.config.local_dir);
                            let _ = fs_tree::open_folder_with_xdg(&base, &entry.rel_path);
                        }
                    }
                    KeyCode::Char('o') => {
                        self.modal = Modal::Settings;
                    }
                    KeyCode::Char('x') => {
                        if let Some(entry) = self.file_entries.get(self.file_selected_idx) {
                            if !entry.name.starts_with("..") {
                                match fs_tree::add_exclude_rule(&entry.rel_path, entry.is_dir) {
                                    Ok(_) => {
                                        self.set_toast(format!("✔ Exclusion added: {}", entry.name));
                                        self.filters = config::read_filters();
                                        self.reload_files();
                                    }
                                    Err(e) => self.set_toast(format!("✗ Error: {}", e)),
                                }
                            }
                        }
                    }
                    KeyCode::Delete => {
                        if let Some(entry) = self.file_entries.get(self.file_selected_idx) {
                            if !entry.name.starts_with("..") {
                                self.modal = Modal::ConfirmDelete(entry.rel_path.clone());
                            }
                        }
                    }
                    _ => {}
                },
                Modal::Filters => {
                    if self.is_editing_filter {
                        match key.code {
                            KeyCode::Enter => {
                                self.commit_filter_edit();
                            }
                            KeyCode::Esc => {
                                self.cancel_filter_edit();
                            }
                            KeyCode::Backspace => {
                                self.filter_edit_buffer.pop();
                            }
                            KeyCode::Char(c) => {
                                self.filter_edit_buffer.push(c);
                            }
                            _ => {}
                        }
                        return Action::None;
                    }

                    let vp = self.filter_viewport_height;
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            self.modal = Modal::None;
                        }
                        KeyCode::Enter | KeyCode::Char('e') => {
                            self.start_editing_filter();
                        }
                        KeyCode::Char('a') | KeyCode::Char('+') => {
                            self.start_adding_filter();
                        }
                        KeyCode::Char('d') | KeyCode::Delete => {
                            self.delete_selected_filter();
                        }
                        KeyCode::Char('t') | KeyCode::Char(' ') => {
                            if !self.filters.is_empty() && self.selected_filter_idx < self.filters.len() {
                                self.cycle_filter_type(self.selected_filter_idx);
                            }
                        }
                        KeyCode::Char('E') => {
                            return Action::OpenEditor;
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            self.scroll_filter_up(vp);
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            self.scroll_filter_down(vp);
                        }
                        KeyCode::PageUp => {
                            self.selected_filter_idx = self.selected_filter_idx.saturating_sub(vp.max(5));
                            self.ensure_filter_visible(vp);
                        }
                        KeyCode::PageDown => {
                            if !self.filters.is_empty() {
                                let max = self.filters.len() - 1;
                                self.selected_filter_idx = (self.selected_filter_idx + vp.max(5)).min(max);
                                self.ensure_filter_visible(vp);
                            }
                        }
                        KeyCode::Home => {
                            self.selected_filter_idx = 0;
                            self.filter_scroll_offset = 0;
                        }
                        KeyCode::End => {
                            if !self.filters.is_empty() {
                                self.selected_filter_idx = self.filters.len() - 1;
                                self.ensure_filter_visible(vp);
                            }
                        }
                        _ => {}
                    }
                }
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
                    let (has_errors, files_len) = self.past_runs.get(run_idx_val)
                        .map(|r| (!r.errors.is_empty(), r.all_affected_files().len()))
                        .unwrap_or((false, 0));
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            self.modal = Modal::None;
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if files_len > 0 {
                                if self.history_selected_file_idx > 0 {
                                    self.history_selected_file_idx -= 1;
                                }
                                if self.history_selected_file_idx < self.history_details_scroll {
                                    self.history_details_scroll = self.history_selected_file_idx;
                                }
                            } else {
                                self.history_details_scroll = self.history_details_scroll.saturating_sub(1);
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if files_len > 0 {
                                if self.history_selected_file_idx < files_len - 1 {
                                    self.history_selected_file_idx += 1;
                                }
                                if self.history_selected_file_idx >= self.history_details_scroll + 15 {
                                    self.history_details_scroll = self.history_selected_file_idx.saturating_sub(14);
                                }
                            } else {
                                self.history_details_scroll = self.history_details_scroll.saturating_add(1);
                            }
                        }
                        KeyCode::PageUp => {
                            self.history_details_scroll = self.history_details_scroll.saturating_sub(10);
                            if files_len > 0 {
                                self.history_selected_file_idx = self.history_selected_file_idx.saturating_sub(10);
                            }
                        }
                        KeyCode::PageDown => {
                            self.history_details_scroll = self.history_details_scroll.saturating_add(10);
                            if files_len > 0 {
                                self.history_selected_file_idx = (self.history_selected_file_idx + 10).min(files_len - 1);
                            }
                        }
                        KeyCode::Home => {
                            self.history_details_scroll = 0;
                            self.history_selected_file_idx = 0;
                        }
                        KeyCode::End => {
                            if files_len > 0 {
                                self.history_selected_file_idx = files_len - 1;
                            }
                        }
                        KeyCode::Enter => {
                            if files_len > 0 {
                                if self.ctrl_mode || key.modifiers.contains(KeyModifiers::CONTROL) {
                                    self.open_history_folder(self.history_selected_file_idx);
                                } else {
                                    self.open_history_file(self.history_selected_file_idx);
                                }
                            }
                        }
                        KeyCode::Char('d') => {
                            if files_len > 0 {
                                self.open_history_folder(self.history_selected_file_idx);
                            }
                        }
                        KeyCode::Char('x') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            if files_len > 0 {
                                self.ctrl_mode = !self.ctrl_mode;
                                if self.ctrl_mode {
                                    self.set_toast("📁 Folder mode active (folder paths shown)");
                                } else {
                                    self.set_toast("📄 File mode active (file paths shown)");
                                }
                            }
                        }
                        KeyCode::Char('o') => {
                            self.modal = Modal::Settings;
                        }
                        KeyCode::Char('y') | KeyCode::Char('c') => {
                            if has_errors || files_len > 0 {
                                self.copy_history_errors(run_idx_val);
                            }
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
                        let total = self.total_history_runs();
                        if total > 0 {
                            if self.is_syncing() && self.selected_run_idx == Some(0) {
                                self.set_toast("ℹ Active sync - Details shown above");
                            } else if let Some(sel) = self.selected_run_idx {
                                let past_idx = if self.is_syncing() { sel.saturating_sub(1) } else { sel };
                                if past_idx < self.past_runs.len() {
                                    self.history_details_scroll = 0;
                                    self.history_selected_file_idx = 0;
                                    self.modal = Modal::HistoryDetails(past_idx);
                                }
                            }
                            return Action::None;
                        }
                    }
                    FocusedPanel::RecentFiles => {
                        if let Some(idx) = self.recent_selected_idx {
                            if key.modifiers.contains(KeyModifiers::CONTROL) || self.ctrl_mode {
                                self.open_recent_folder(idx);
                            } else {
                                self.open_recent_file(idx);
                            }
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
                self.modal = Modal::Settings;
                return Action::None;
            }
            KeyCode::Char('d') => {
                self.modal = Modal::ConfirmDryRun;
                return Action::None;
            }
            KeyCode::Char('f') => {
                if self.focused_panel == FocusedPanel::Logs {
                    self.log_filter = self.log_filter.next();
                    return Action::None;
                }
                self.focused_panel = FocusedPanel::RecentFiles;
                self.is_filtering_recent = true;
                return Action::None;
            }
            KeyCode::Char('/') => {
                self.focused_panel = FocusedPanel::RecentFiles;
                self.is_filtering_recent = true;
                return Action::None;
            }
            KeyCode::Char('1') if self.focused_panel == FocusedPanel::Logs => {
                self.log_filter = LogFilter::All;
                return Action::None;
            }
            KeyCode::Char('2') if self.focused_panel == FocusedPanel::Logs => {
                self.log_filter = LogFilter::Files;
                return Action::None;
            }
            KeyCode::Char('3') if self.focused_panel == FocusedPanel::Logs => {
                self.log_filter = LogFilter::Problems;
                return Action::None;
            }
            KeyCode::Char('b') | KeyCode::Char('B') | KeyCode::Char('p') | KeyCode::Char('P') => {
                self.modal = if self.modal == Modal::Files { Modal::None } else { Modal::Files };
                return Action::None;
            }
            KeyCode::Char('x') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.ctrl_mode = !self.ctrl_mode;
                if self.ctrl_mode {
                    self.set_toast("📁 Folder mode active (folder paths shown)");
                } else {
                    self.set_toast("📄 File mode active (file paths shown)");
                }
                return Action::None;
            }
            KeyCode::Char('e') => {
                self.modal = Modal::Filters;
                return Action::None;
            }
            KeyCode::Char('t') => {
                self.next_theme();
                return Action::None;
            }
            KeyCode::Char('?') | KeyCode::Char('h') => {
                self.modal = Modal::Help;
                return Action::None;
            }
            KeyCode::Char('s') => {
                self.modal = Modal::ConfirmSync;
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
                    self.set_toast("No active synchronization to cancel.");
                }
                return Action::None;
            }
            KeyCode::Char('y') => {
                match self.focused_panel {
                    FocusedPanel::Logs => {
                        self.copy_logs_to_clipboard();
                    }
                    FocusedPanel::History => {
                        if let Some(sel) = self.selected_run_idx {
                            let past_idx = if self.is_syncing() { sel.saturating_sub(1) } else { sel };
                            self.copy_history_errors(past_idx);
                        } else {
                            self.set_toast("ℹ No run selected in history.");
                        }
                    }
                    FocusedPanel::RecentFiles => {
                        self.copy_selected_recent_file();
                    }
                }
                return Action::None;
            }
            KeyCode::Char(' ') => {
                self.auto_scroll = !self.auto_scroll;
                if self.auto_scroll {
                    self.logs_scroll = 0;
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
                return Action::None;
            }
            KeyCode::BackTab => {
                self.focused_panel = self.focused_panel.prev();
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
                        let vp = self.history_viewport_height.max(3);
                        self.scroll_history_up(vp);
                    }
                    FocusedPanel::Logs => {
                        self.auto_scroll = false;
                        let max_scroll = self.total_log_lines().saturating_sub(self.logs_viewport_height.max(3));
                        self.logs_scroll = (self.logs_scroll + 1).min(max_scroll);
                    }
                    FocusedPanel::RecentFiles => {
                        let vp = self.recent_viewport_height.max(1);
                        self.scroll_recent_up(vp);
                    }
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                match self.focused_panel {
                    FocusedPanel::History => {
                        let vp = self.history_viewport_height.max(3);
                        self.scroll_history_down(vp);
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
                        let vp = self.recent_viewport_height.max(1);
                        self.scroll_recent_down(vp);
                    }
                }
            }
            KeyCode::PageUp => {
                match self.focused_panel {
                    FocusedPanel::History => {
                        if let Some(idx) = self.selected_run_idx {
                            self.selected_run_idx = Some(idx.saturating_sub(10));
                            let vp = self.history_viewport_height.max(3);
                            self.ensure_history_visible(vp);
                        }
                    }
                    FocusedPanel::Logs => {
                        self.auto_scroll = false;
                        let max_scroll = self.total_log_lines().saturating_sub(self.logs_viewport_height.max(3));
                        self.logs_scroll = (self.logs_scroll + 10).min(max_scroll);
                    }
                    FocusedPanel::RecentFiles => {
                        if let Some(idx) = self.recent_selected_idx {
                            self.recent_selected_idx = Some(idx.saturating_sub(10));
                            let vp = self.recent_viewport_height.max(1);
                            self.ensure_recent_visible(vp);
                        }
                    }
                }
            }
            KeyCode::PageDown => {
                match self.focused_panel {
                    FocusedPanel::History => {
                        let total = self.total_history_runs();
                        if total > 0 {
                            let cur = self.selected_run_idx.unwrap_or(0);
                            self.selected_run_idx = Some((cur + 10).min(total - 1));
                            let vp = self.history_viewport_height.max(3);
                            self.ensure_history_visible(vp);
                        }
                    }
                    FocusedPanel::Logs => {
                        self.logs_scroll = self.logs_scroll.saturating_sub(10);
                        if self.logs_scroll == 0 {
                            self.auto_scroll = true;
                        }
                    }
                    FocusedPanel::RecentFiles => {
                        let total = self.get_recent_files_list().len();
                        if total > 0 {
                            let cur = self.recent_selected_idx.unwrap_or(0);
                            self.recent_selected_idx = Some((cur + 10).min(total - 1));
                            let vp = self.recent_viewport_height.max(1);
                            self.ensure_recent_visible(vp);
                        }
                    }
                }
            }
            KeyCode::Home => {
                match self.focused_panel {
                    FocusedPanel::History => {
                        if self.total_history_runs() > 0 {
                            self.selected_run_idx = Some(0);
                        }
                        self.history_scroll_offset = 0;
                    }
                    FocusedPanel::Logs => {
                        self.auto_scroll = false;
                        let max_scroll = self.total_log_lines().saturating_sub(self.logs_viewport_height.max(3));
                        self.logs_scroll = max_scroll;
                    }
                    FocusedPanel::RecentFiles => {
                        self.recent_scroll_offset = 0;
                        if !self.get_recent_files_list().is_empty() {
                            self.recent_selected_idx = Some(0);
                        }
                    }
                }
            }
            KeyCode::End => {
                match self.focused_panel {
                    FocusedPanel::History => {
                        let total = self.total_history_runs();
                        if total > 0 {
                            self.selected_run_idx = Some(total - 1);
                            let vp = self.history_viewport_height.max(3);
                            self.ensure_history_visible(vp);
                        }
                    }
                    FocusedPanel::Logs => {
                        self.logs_scroll = 0;
                        self.auto_scroll = true;
                    }
                    FocusedPanel::RecentFiles => {
                        let total = self.get_recent_files_list().len();
                        if total > 0 {
                            self.recent_selected_idx = Some(total - 1);
                            let vp = self.recent_viewport_height.max(1);
                            self.ensure_recent_visible(vp);
                        }
                    }
                }
            }
            KeyCode::Left => {
                if self.focused_panel == FocusedPanel::Logs {
                    self.log_filter = self.log_filter.prev();
                    return Action::None;
                }
            }
            KeyCode::Right => {
                if self.focused_panel == FocusedPanel::Logs {
                    self.log_filter = self.log_filter.next();
                    return Action::None;
                }
            }
            _ => {}
        }

        Action::None
    }
}

fn spawn_quota_fetch(remote: String, tx: std::sync::mpsc::Sender<CloudQuota>) {
    tokio::spawn(async move {
        let mut cmd = tokio::process::Command::new("rclone");
        cmd.args(["about", &remote, "--json"]);
        if let Ok(output) = cmd.output().await {
            if output.status.success() {
                if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                    let total = val.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
                    let used = val.get("used").and_then(|v| v.as_u64()).unwrap_or(0);
                    let free = val.get("free").and_then(|v| v.as_u64()).unwrap_or(0);
                    if total > 0 {
                        let _ = tx.send(CloudQuota {
                            total_bytes: total,
                            used_bytes: used,
                            free_bytes: free,
                        });
                    }
                }
            }
        }
    });
}

fn count_dir_files(p: &std::path::Path) -> usize {
    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(p) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if !entry.file_name().to_string_lossy().starts_with('.') {
                    count += count_dir_files(&path);
                }
            } else if path.is_file() {
                count += 1;
            }
        }
    }
    count
}

fn spawn_file_count(dir_path: std::path::PathBuf, tx: std::sync::mpsc::Sender<usize>) {
    tokio::task::spawn_blocking(move || {
        let c = count_dir_files(&dir_path);
        let _ = tx.send(c);
    });
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

        // Trouver la hitbox du bouton Fichiers (Files) dans le footer
        let files_hb = app.hitboxes.iter().find(|h| h.action == HitAction::ButtonFiles);
        assert!(files_hb.is_some(), "Le bouton Files doit être présent");
        let hb = files_hb.unwrap();

        // Clic au centre de la hitbox Files
        let click_x = hb.rect.x + hb.rect.width / 2;
        let click_y = hb.rect.y + hb.rect.height / 2;

        let mouse_event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: click_x,
            row: click_y,
            modifiers: KeyModifiers::NONE,
        };

        app.handle_mouse(mouse_event);
        assert_eq!(app.modal, Modal::Files, "Le clic doit basculer la modal Fichiers");

        // Clic à nouveau pour fermer
        app.handle_mouse(mouse_event);
        assert_eq!(app.modal, Modal::None, "Le second clic doit fermer la modal Fichiers");

        // Tester également l'ouverture du Menu et le clic sur Option 0 (Options / Settings)
        app.modal = Modal::Menu;
        app.menu_selected_idx = 0;
        terminal.draw(|f| crate::ui::render(f, &mut app)).unwrap();
        let menu_opt0_hb = app.hitboxes.iter().find(|h| h.action == HitAction::MenuOption(0));
        assert!(menu_opt0_hb.is_some(), "L'option Menu 0 (Settings) doit avoir sa hitbox");
        let mhb = menu_opt0_hb.unwrap();
        let menu_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: mhb.rect.x + mhb.rect.width / 2,
            row: mhb.rect.y + mhb.rect.height / 2,
            modifiers: KeyModifiers::NONE,
        };
        app.handle_mouse(menu_click);
        assert_eq!(app.modal, Modal::Settings, "Le clic sur MenuOption(0) doit ouvrir Settings");
    }

    #[tokio::test]
    async fn test_mouse_scroll_logs() {
        let mut app = App::new();
        app.live.log_lines = (1..=30).map(|i| format!("log {}", i)).collect();
        app.logs_viewport_height = 10;
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
        assert_eq!(app.logs_scroll, 1);

        // Molette vers le bas dans les logs
        let scroll_down = MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 60,
            row: 10,
            modifiers: KeyModifiers::NONE,
        };
        app.handle_mouse(scroll_down);
        assert_eq!(app.logs_scroll, 0);
        assert!(app.auto_scroll, "Le défilement tout en bas doit réactiver l'auto-scroll");
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
                synced_files: vec![("new".to_string(), "file1.txt".to_string(), "22:00".to_string())],
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

        // First option is Options (Settings)
        assert_eq!(app.menu_selected_idx, 0);

        // Pressing Down selects second option (Help)
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
        app.tick_rate_ms_live = 2000;

        // '-' speeds up (lower ms: 2000 -> 1500 -> 1000)
        let minus_event = KeyEvent::new(KeyCode::Char('-'), KeyModifiers::NONE);
        app.handle_key(minus_event);
        assert_eq!(app.tick_rate_ms_live, 1500);

        app.handle_key(minus_event);
        assert_eq!(app.tick_rate_ms_live, 1000);

        // '+' slows down (higher ms)
        let plus_event = KeyEvent::new(KeyCode::Char('+'), KeyModifiers::NONE);
        app.handle_key(plus_event);
        assert_eq!(app.tick_rate_ms_live, 1500);

        app.handle_key(plus_event);
        assert_eq!(app.tick_rate_ms_live, 2000);
    }

    #[tokio::test]
    async fn test_settings_never_cycle() {
        let mut app = App::new();
        app.settings_tab = 0;
        app.settings_selected_idx = 1; // full_sync_interval
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

    #[tokio::test]
    async fn test_filter_modal_keyboard_and_mouse_scroll() {
        let mut app = App::new();
        app.filters = (1..=30).map(|i| format!("- /exclude_pattern_{}/**", i)).collect();
        app.modal = Modal::Filters;
        app.filter_viewport_height = 10;
        app.selected_filter_idx = 0;
        app.filter_scroll_offset = 0;

        // Register a Hitbox for FilterArea
        app.hitboxes.push(Hitbox {
            rect: Rect { x: 10, y: 10, width: 40, height: 15 },
            action: HitAction::FilterArea,
        });

        // 1. Keyboard Down navigation
        let down_key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        for _ in 0..12 {
            app.handle_key(down_key);
        }
        assert_eq!(app.selected_filter_idx, 12);
        assert!(app.filter_scroll_offset > 0, "filter_scroll_offset must have scrolled down");

        // 2. Keyboard PageUp
        let page_up = KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE);
        app.handle_key(page_up);
        assert_eq!(app.selected_filter_idx, 2);
        assert_eq!(app.filter_scroll_offset, 2);

        // 3. Mouse Wheel ScrollDown over FilterArea
        let scroll_down = MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 15,
            row: 15,
            modifiers: KeyModifiers::NONE,
        };
        app.handle_mouse(scroll_down);
        assert_eq!(app.selected_filter_idx, 3);

        // 4. Mouse Wheel ScrollUp over FilterArea
        let scroll_up = MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 15,
            row: 15,
            modifiers: KeyModifiers::NONE,
        };
        app.handle_mouse(scroll_up);
        assert_eq!(app.selected_filter_idx, 2);

        // 5. Left Click on FilterRow
        app.hitboxes.push(Hitbox {
            rect: Rect { x: 10, y: 12, width: 40, height: 1 },
            action: HitAction::FilterRow(20),
        });
        let click_row = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 15,
            row: 12,
            modifiers: KeyModifiers::NONE,
        };
        app.handle_mouse(click_row);
        assert_eq!(app.selected_filter_idx, 20);
        assert!(app.filter_scroll_offset >= 11, "filter_scroll_offset must be updated to keep row 20 visible");
    }

    #[tokio::test]
    async fn test_toggle_ctrl_mode() {
        let mut app = App::new();
        assert!(!app.ctrl_mode);

        // Click on ToggleCtrlMode hitbox toggles ctrl_mode
        app.hitboxes.push(Hitbox {
            rect: Rect { x: 30, y: 40, width: 15, height: 1 },
            action: HitAction::ToggleCtrlMode,
        });
        let click_ctrl = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 35,
            row: 40,
            modifiers: KeyModifiers::NONE,
        };
        app.handle_mouse(click_ctrl);
        assert!(app.ctrl_mode);
        assert!(app.toast.is_some());

        // Clicking again toggles back off
        app.handle_mouse(click_ctrl);
        assert!(!app.ctrl_mode);

        // Key Ctrl+X toggles on
        let ctrl_x = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL);
        app.handle_key(ctrl_x);
        assert!(app.ctrl_mode);

        // Key Ctrl+X toggles off
        app.handle_key(ctrl_x);
        assert!(!app.ctrl_mode);

        // Raw 0x18 keycode also works even while filtering
        app.is_filtering_recent = true;
        let raw_ctrl_x = KeyEvent::new(KeyCode::Char('\u{18}'), KeyModifiers::NONE);
        app.handle_key(raw_ctrl_x);
        assert!(app.ctrl_mode);
        app.handle_key(raw_ctrl_x);
        assert!(!app.ctrl_mode);
    }

    #[tokio::test]
    async fn test_tick_rate_stepping_and_settings_harmony() {
        let mut app = App::new();
        app.tick_rate_ms_live = 1000;

        // Step up
        app.step_tick_rate(false); // ralentit
        assert_eq!(app.tick_rate_ms_live, 1500);

        // Step down
        app.step_tick_rate(true); // accélère
        assert_eq!(app.tick_rate_ms_live, 1000);

        // Cycle via settings
        app.settings_tab = 1;
        app.settings_selected_idx = 5; // Taux de rafraîchissement UI
        app.cycle_setting(true);
        assert_eq!(app.tick_rate_ms_live, 1500);
        assert_eq!(app.config.tick_rate_ms, Some(1500));
    }

    #[tokio::test]
    async fn test_recent_filter_interactive() {
        let mut app = App::new();
        app.past_runs.clear();
        app.live.synced_files.clear();
        app.past_runs.push(crate::monitor::PastRun {
            id: 1,
            date: "2026-09-18".into(),
            time: "10:00".into(),
            duration: "5s".into(),
            status: RunStatus::Success,
            summary: String::new(),
            files_copied: vec!["Documents/alpha.txt".into(), "Pictures/photo.jpg".into()],
            files_modified: vec!["Code/main.rs".into()],
            files_deleted: vec![],
            errors: vec![],
            synced_files: vec![],
        });

        // Focus RecentFiles and press '/' to start filtering
        app.focused_panel = FocusedPanel::RecentFiles;
        let slash_key = KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE);
        app.handle_key(slash_key);
        assert!(app.is_filtering_recent);

        // Type "photo"
        for c in "photo".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        assert_eq!(app.recent_filter, "photo");
        let filtered = app.get_all_recent_files();
        assert_eq!(filtered.len(), 1);
        assert!(filtered[0].1.contains("photo.jpg"));

        // Press Enter to confirm filter
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(!app.is_filtering_recent);
        assert_eq!(app.recent_filter, "photo");

        // Focus and press '/', then Esc to clear
        app.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!app.is_filtering_recent);
        assert!(app.recent_filter.is_empty());
        assert_eq!(app.get_all_recent_files().len(), 3);
    }

    #[tokio::test]
    async fn test_total_history_runs_with_live_sync() {
        let mut app = App::new();
        app.service_info.state = ServiceState::Idle;
        app.live.is_syncing = false;
        app.past_runs.clear();
        assert_eq!(app.total_history_runs(), 0);

        app.past_runs.push(crate::monitor::PastRun {
            id: 1,
            date: "2026-09-18".into(),
            time: "10:00".into(),
            duration: "5s".into(),
            status: RunStatus::Success,
            summary: String::new(),
            files_copied: vec![],
            files_modified: vec![],
            files_deleted: vec![],
            errors: vec![],
            synced_files: vec![],
        });
        assert_eq!(app.total_history_runs(), 1);

        app.live.is_syncing = true;
        assert_eq!(app.total_history_runs(), 2);
    }

    #[tokio::test]
    async fn test_confirm_sync_and_dry_run_modals() {
        let mut app = App::new();

        // 1. Appuyer sur 's' -> ouvre la confirmation de sync
        app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::ConfirmSync);

        // 'n' ou Esc annule
        app.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::None);

        // 2. Appuyer sur 'd' -> ouvre la confirmation de dry-run
        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::ConfirmDryRun);

        // Esc annule
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::None);
    }

    #[tokio::test]
    async fn test_unambiguous_global_shortcuts() {
        let mut app = App::new();
        // Quand on est sur le panel RecentFiles, les touches globales ne doivent pas être capturées par d'anciennes actions
        app.focused_panel = FocusedPanel::RecentFiles;

        app.handle_key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::Settings);
        app.modal = Modal::None;

        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::ConfirmDryRun);
        app.modal = Modal::None;

        app.handle_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE));
        assert!(app.is_filtering_recent, "'f' sur RecentFiles doit activer le filtre");
        app.is_filtering_recent = false;

        // 'b' ouvre la modal Files (explorateur)
        app.focused_panel = FocusedPanel::History;
        app.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::Files);
        app.modal = Modal::None;

        // 'f' active le filtre même depuis un autre panel et focus RecentFiles
        app.focused_panel = FocusedPanel::History;
        app.handle_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE));
        assert!(app.is_filtering_recent);
        assert_eq!(app.focused_panel, FocusedPanel::RecentFiles);
        app.is_filtering_recent = false;

        app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::ConfirmSync);
        app.modal = Modal::None;
    }

    #[test]
    fn test_grayscale_palette() {
        let palette = crate::ui::theme::ThemeChoice::TokyoNight.palette();
        let gray = palette.to_grayscale();

        // Vérifier que chaque couleur RGB est en nuances de gris (r == g == b)
        if let ratatui::style::Color::Rgb(r, g, b) = gray.accent {
            assert_eq!(r, g);
            assert_eq!(g, b);
        } else {
            panic!("Expected RGB color");
        }

        if let ratatui::style::Color::Rgb(r, g, b) = gray.red {
            assert_eq!(r, g);
            assert_eq!(g, b);
        } else {
            panic!("Expected RGB color");
        }
    }

    #[tokio::test]
    async fn test_initial_unselected_and_btop_scroll() {
        let mut app = App::new();
        app.service_info.state = ServiceState::Idle;
        app.live.is_syncing = false;
        assert_eq!(app.recent_selected_idx, None);
        assert_eq!(app.selected_run_idx, None);

        // Add 5 past runs
        app.past_runs.clear();
        for i in 0..5 {
            app.past_runs.push(crate::monitor::PastRun {
                id: i,
                date: "2026-09-18".into(),
                time: format!("10:0{}", i),
                duration: "2s".into(),
                status: RunStatus::Success,
                summary: String::new(),
                files_copied: vec![],
                files_modified: vec![],
                files_deleted: vec![],
                errors: vec![],
                synced_files: vec![],
            });
        }

        // Viewport of 3 items
        app.history_viewport_height = 3;
        app.focused_panel = FocusedPanel::History;

        // Down from None -> Some(0)
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.selected_run_idx, Some(0));
        assert_eq!(app.history_scroll_offset, 0);

        // Up from 0 -> None (disappears)
        app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(app.selected_run_idx, None);

        // Down again -> Some(0)
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.selected_run_idx, Some(0));

        // Down: content remains below (max_offset = 5 - 3 = 2), so scroll_offset increments and selected_idx increments
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.history_scroll_offset, 1);
        assert_eq!(app.selected_run_idx, Some(1));
        // Visual position = 1 - 1 = 0 (fixed at the top of the viewport!)

        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.history_scroll_offset, 2);
        assert_eq!(app.selected_run_idx, Some(2));
        // Visual position = 2 - 2 = 0 (still fixed!)

        // Now max_offset (2) is reached. Down moves the cursor towards the end
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.history_scroll_offset, 2);
        assert_eq!(app.selected_run_idx, Some(3));

        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.history_scroll_offset, 2);
        assert_eq!(app.selected_run_idx, Some(4));

        // Clamped at end (cannot scroll past 4)
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.history_scroll_offset, 2);
        assert_eq!(app.selected_run_idx, Some(4));
    }

    #[tokio::test]
    async fn test_tick_rate_boundaries() {
        let mut app = App::new();
        app.tick_rate_ms_live = 100;
        assert!(!app.can_dec_tick_rate());
        assert!(app.can_inc_tick_rate());

        app.tick_rate_ms_live = 10000;
        assert!(app.can_dec_tick_rate());
        assert!(!app.can_inc_tick_rate());

        app.tick_rate_ms_live = 1000;
        assert!(app.can_dec_tick_rate());
        assert!(app.can_inc_tick_rate());
    }

    #[tokio::test]
    async fn test_settings_resync_modal_trigger() {
        let mut app = App::new();
        app.modal = Modal::Settings;
        app.settings_tab = 0;
        app.settings_selected_idx = 5; // Resynchronisation complète

        // Appuyer sur Entrée doit ouvrir ConfirmResync
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::ConfirmResync);

        // Annuler avec Esc
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::None);

        // Rouvrir et tester avec Flèche Droite
        app.modal = Modal::Settings;
        app.settings_tab = 0;
        app.settings_selected_idx = 5;
        app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::ConfirmResync);
    }

    #[tokio::test]
    async fn test_files_browser_left_right_navigation() {
        let mut app = App::new();
        app.modal = Modal::Files;
        app.file_entries = vec![
            crate::fs_tree::FileEntry {
                name: "dossier_a".into(),
                rel_path: "dossier_a".into(),
                is_dir: true,
                size: 0,
                mtime: "".into(),
                ignored: false,
            },
            crate::fs_tree::FileEntry {
                name: "fichier_b.txt".into(),
                rel_path: "fichier_b.txt".into(),
                is_dir: false,
                size: 100,
                mtime: "".into(),
                ignored: false,
            },
        ];
        app.file_selected_idx = 0; // dossier_a

        // Flèche droite sur un dossier entre dans le dossier
        app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
        assert_eq!(app.file_current_rel, "dossier_a");

        // Flèche gauche remonte au parent
        app.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        assert_eq!(app.file_current_rel, "");
    }

    #[test]
    fn test_normalize_display_path() {
        use crate::ui::dashboard::normalize_display_path;
        assert_eq!(
            normalize_display_path("Images/Screenshot From 2026-09-18 08-08-09.png"),
            "Images/Screenshot from 2026-09-18 08-08-09.png"
        );
        assert_eq!(
            normalize_display_path("Images/Screenshot from 2026-09-03 08-56-39.png"),
            "Images/Screenshot from 2026-09-03 08-56-39.png"
        );
        assert_eq!(
            normalize_display_path("From zero to hero.pdf"),
            "from zero to hero.pdf"
        );
    }

    #[tokio::test]
    async fn test_universal_q_closes_modals() {
        let mut app = App::new();

        // 1. Files modal se ferme avec 'q'
        app.modal = Modal::Files;
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::None);

        // 2. Filters modal se ferme avec 'q'
        app.modal = Modal::Filters;
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::None);

        // 3. Settings modal se ferme avec 'q'
        app.modal = Modal::Settings;
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::None);

        // 4. Help modal se ferme avec 'q'
        app.modal = Modal::Help;
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::None);

        // 5. Confirm modals se ferment avec 'q'
        app.modal = Modal::ConfirmSync;
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::None);

        app.modal = Modal::ConfirmResync;
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::None);

        app.modal = Modal::ConfirmCancel;
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert_eq!(app.modal, Modal::None);

        // 6. Menu btop : 'q' quitte l'application
        app.modal = Modal::Menu;
        app.running = true;
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(!app.running, "Dans le Menu, 'q' doit quitter l'application");

        // 7. Modal::None : 'q' quitte l'application
        app.modal = Modal::None;
        app.running = true;
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(!app.running, "Sur le Dashboard, 'q' doit quitter l'application");
    }

    #[tokio::test]
    async fn test_settings_interactive_string_edit() {
        let mut app = App::new();
        app.modal = Modal::Settings;
        app.settings_tab = 0;
        app.settings_selected_idx = 3; // Local directory
        app.config.local_dir = "/home/user/drive".to_string();

        // Appui sur Entrée pour entrer en mode édition
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(app.is_editing_setting);
        assert_eq!(app.setting_edit_buffer, "/home/user/drive");

        // Saisie de caractères : "/sub"
        for c in "/sub".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        assert_eq!(app.setting_edit_buffer, "/home/user/drive/sub");

        // Backspace
        app.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(app.setting_edit_buffer, "/home/user/drive/su");

        // Sortie avec Échap : ce qui est écrit dans le champ est sauvegardé (selon demande utilisateur)
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!app.is_editing_setting);
        assert_eq!(app.config.local_dir, "/home/user/drive/su");

        // Entrée en édition avec 'e', modification et validation avec Entrée
        app.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
        assert!(app.is_editing_setting);
        app.setting_edit_buffer = "/home/new/path".to_string();
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(!app.is_editing_setting);
        assert_eq!(app.config.local_dir, "/home/new/path");

        // Test sur le remote (option 4) : par exemple "GoogleDrive:"
        app.settings_tab = 0;
        app.settings_selected_idx = 4;
        app.config.remote = "GoogleDrive:".to_string();
        app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)); // Flèche droite active l'édition
        assert!(app.is_editing_setting);
        assert_eq!(app.setting_edit_buffer, "GoogleDrive:");

        // L'utilisateur ne modifie rien à la chaîne et sort : "GoogleDrive:" reste inchangé
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!app.is_editing_setting);
        assert_eq!(app.config.remote, "GoogleDrive:");
    }

    #[tokio::test]
    async fn test_scrollbar_mouse_interaction() {
        let mut app = App::new();
        app.logs_viewport_height = 5;
        app.live.log_lines = (0..20).map(|i| format!("log line {}", i)).collect();

        // 1. Clic sur flèche haut scrollbar logs
        assert_eq!(app.logs_scroll, 0);
        assert!(app.auto_scroll);
        app.apply_scrollbar_step(ScrollbarTarget::Logs, true);
        assert_eq!(app.logs_scroll, 1);
        assert!(!app.auto_scroll);

        // 2. Clic sur flèche bas scrollbar logs
        app.apply_scrollbar_step(ScrollbarTarget::Logs, false);
        assert_eq!(app.logs_scroll, 0);
        assert!(app.auto_scroll);

        // 3. Saut / Glissement (drag) sur la piste scrollbar
        // Piste de hauteur 10, offset 5 -> milieu
        app.apply_scrollbar_jump(ScrollbarTarget::Logs, 5, 10, 20, 5);
        assert!(app.logs_scroll > 0);
        assert!(!app.auto_scroll);

        // 4. Test ScrollbarTarget::History step & jump
        app.past_runs = (0..10).map(|i| crate::monitor::history::PastRun {
            id: i,
            date: "2026-09-18".into(),
            time: "12:00:00".into(),
            duration: "5s".into(),
            status: crate::monitor::history::RunStatus::Success,
            files_copied: vec![],
            files_modified: vec![],
            files_deleted: vec![],
            synced_files: vec![],
            errors: vec![],
            summary: "OK".into(),
        }).collect();
        app.history_viewport_height = 4;
        app.selected_run_idx = Some(0);

        app.apply_scrollbar_step(ScrollbarTarget::History, false);
        assert_eq!(app.selected_run_idx, Some(1));

        app.apply_scrollbar_step(ScrollbarTarget::History, true);
        assert_eq!(app.selected_run_idx, Some(0));

        app.apply_scrollbar_jump(ScrollbarTarget::History, 9, 10, 10, 4);
        assert_eq!(app.selected_run_idx, Some(9));
    }

    #[test]
    fn test_keybinding_registry() {
        use crate::ui::keys::{KeyAction, KeybindingRegistry};
        use crate::ui::theme::ThemeChoice;

        assert_eq!(KeybindingRegistry::get_key_str(KeyAction::Files), "b");
        assert_eq!(KeybindingRegistry::get_key_str(KeyAction::Filters), "e");
        assert_eq!(KeybindingRegistry::get_key_str(KeyAction::QuickSync), "s");
        assert_eq!(KeybindingRegistry::get_key_str(KeyAction::Resync), "r");
        assert_eq!(KeybindingRegistry::get_key_str(KeyAction::Quit), "q");
        assert_eq!(KeybindingRegistry::get_key_str(KeyAction::CloseModal), "Échap / q");

        let theme = ThemeChoice::TokyoNight.palette();
        let badge = KeybindingRegistry::format_key_badge("Ctrl+X", &theme);
        assert_eq!(badge.len(), 3);
        assert_eq!(badge[0].content, " [");
        assert_eq!(badge[1].content, " Ctrl+X ");
        assert_eq!(badge[2].content, "] ");
    }

    #[tokio::test]
    async fn test_recent_files_scroll_with_small_viewport() {
        let mut app = App::new();
        // Remplir 10 fichiers récents via past_runs
        app.past_runs = vec![crate::monitor::history::PastRun {
            id: 1,
            date: "2026-09-18".into(),
            time: "12:00:00".into(),
            duration: "5s".into(),
            status: crate::monitor::history::RunStatus::Success,
            files_copied: (0..10).map(|i| format!("file_{}.txt", i)).collect(),
            files_modified: vec![],
            files_deleted: vec![],
            synced_files: vec![],
            errors: vec![],
            summary: "OK".into(),
        }];

        // Simuler un conteneur rétréci par la synchronisation en cours (viewport de 2 lignes)
        let vp = 2;
        app.recent_viewport_height = vp;
        app.focused_panel = FocusedPanel::RecentFiles;

        // Premier appui sur Flèche Bas : sélectionne l'élément 0
        app.scroll_recent_down(vp);
        assert_eq!(app.recent_selected_idx, Some(0));
        assert_eq!(app.recent_scroll_offset, 0);

        // Défiler vers le bas à travers tous les éléments
        for expected_idx in 1..10 {
            app.scroll_recent_down(vp);
            assert_eq!(app.recent_selected_idx, Some(expected_idx));

            let sel = app.recent_selected_idx.unwrap();
            // L'élément sélectionné ne doit JAMAIS dépasser la fenêtre visible
            assert!(
                sel >= app.recent_scroll_offset,
                "L'élément sélectionné ({}) ne doit pas être inférieur à l'offset ({})",
                sel,
                app.recent_scroll_offset
            );
            assert!(
                sel < app.recent_scroll_offset + vp,
                "L'élément sélectionné ({}) ne doit pas dépasser le conteneur (offset {} + vp {})",
                sel,
                app.recent_scroll_offset,
                vp
            );
        }

        // Vérifier que le tout dernier élément (index 9) est bien atteint et visible
        assert_eq!(app.recent_selected_idx, Some(9));
        assert_eq!(app.recent_scroll_offset, 8); // Affiche les éléments 8 et 9 dans le conteneur de 2 lignes

        // Défiler vers le haut
        for expected_idx in (0..9).rev() {
            app.scroll_recent_up(vp);
            assert_eq!(app.recent_selected_idx, Some(expected_idx));

            let sel = app.recent_selected_idx.unwrap();
            assert!(
                sel >= app.recent_scroll_offset,
                "En remontant, l'élément sélectionné ({}) doit rester >= offset ({})",
                sel,
                app.recent_scroll_offset
            );
            assert!(
                sel < app.recent_scroll_offset + vp,
                "En remontant, l'élément sélectionné ({}) doit rester < offset {} + vp {}",
                sel,
                app.recent_scroll_offset,
                vp
            );
        }
    }

    #[tokio::test]
    async fn test_wrap_text_no_crop() {
        let long_line = "2026-09-18 13:40:12 ERROR : Failed to copy /home/user/very/long/path/to/my/awesome/document_with_special_data.pdf: corrupted file size mismatch 1243 vs 1255";
        let wrapped = crate::ui::dashboard::wrap_text(long_line, 50, 45);
        assert!(wrapped.len() >= 3);
        for (i, line) in wrapped.iter().enumerate() {
            let limit = if i == 0 { 50 } else { 45 };
            assert!(line.chars().count() <= limit, "Ligne {} dépasse la limite {}: {}", i, limit, line);
        }

        // Test de sécurité UTF-8 (aucun panic avec caractères accentués ou emojis)
        let utf8_line = "🚨 Événement critique détecté dans le répertoire /données/système/sécurité_avancée/fichier_spécial.json";
        let wrapped_utf8 = crate::ui::dashboard::wrap_text(utf8_line, 30, 25);
        assert!(wrapped_utf8.len() >= 2);
        for line in &wrapped_utf8 {
            assert!(!line.is_empty());
        }
    }

    #[tokio::test]
    async fn test_clipboard_copy_logs_and_history_errors() {
        let mut app = App::new();

        // 1. Copie des logs
        app.live.log_lines.push_back("2026-09-18 13:40:12 INFO : Démarrage".into());
        app.live.log_lines.push_back("2026-09-18 13:40:15 NOTICE : Synchro terminée".into());
        app.copy_logs_to_clipboard();
        assert!(app.toast.is_some());
        let (msg, _) = app.toast.as_ref().unwrap();
        assert!(msg.contains("2 log lines copied"));

        // 2. Copie des erreurs d'un run
        app.past_runs = vec![crate::monitor::history::PastRun {
            id: 42,
            date: "2026-09-18".into(),
            time: "13:30:00".into(),
            duration: "12s".into(),
            status: crate::monitor::history::RunStatus::Failed,
            files_copied: vec![],
            files_modified: vec![],
            files_deleted: vec![],
            synced_files: vec![],
            errors: vec![
                "Failed to sync /docs: Google Drive quota exceeded".into(),
                "Connection timeout after 30s".into(),
            ],
            summary: "2 erreurs".into(),
        }];

        app.copy_history_errors(0);
        let (msg, _) = app.toast.as_ref().unwrap();
        assert!(msg.contains("2 error(s) from run #42"));

        // 3. Raccourci y / c dans Modal::HistoryDetails
        app.modal = Modal::HistoryDetails(0);
        let action = app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('y'),
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(action, Action::None);
        let (msg, _) = app.toast.as_ref().unwrap();
        assert!(msg.contains("2 error(s) from run #42"));

        let action_c = app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('c'),
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(action_c, Action::None);
        assert!(app.toast.is_some());

        // 4. Raccourci y sur le dashboard quand Logs est focalisé
        app.modal = Modal::None;
        app.focused_panel = FocusedPanel::Logs;
        let action_dash_y = app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('y'),
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(action_dash_y, Action::None);
        let (msg, _) = app.toast.as_ref().unwrap();
        assert!(msg.contains("log lines copied"));
    }

    #[tokio::test]
    async fn test_sync_progress_and_history_details_states() {
        let mut app = App::new();

        // 1. Test overall_progress_pct
        app.live.is_syncing = true;
        app.live.phase_index = 0; // Listings
        assert_eq!(app.live.overall_progress_pct(), 10);
        app.live.phase_index = 1; // Diffs locaux
        assert_eq!(app.live.overall_progress_pct(), 25);
        app.live.phase_index = 2; // Diffs distants
        assert_eq!(app.live.overall_progress_pct(), 50);
        app.live.phase_index = 3; // Application (sans transfer stats: 70)
        assert_eq!(app.live.overall_progress_pct(), 70);
        app.live.transfer.pct = 85;
        // With explicit rclone transfer percentage:
        assert_eq!(app.live.overall_progress_pct(), 85);
        app.live.transfer.pct = 0;
        app.live.phase_index = 4; // Mise à jour
        assert_eq!(app.live.overall_progress_pct(), 95);
        app.live.phase_index = 5; // Terminé
        assert_eq!(app.live.overall_progress_pct(), 100);

        // 2. Test is_syncing and panel hiding
        app.live.is_syncing = true;
        app.live.phase_index = 2;
        assert!(app.is_syncing());

        // When phase reaches 5 (Terminé), is_syncing must be false
        app.live.phase_index = 5;
        assert!(!app.is_syncing());

        // When systemd service is Idle, is_syncing and transfer are reset
        app.live.is_syncing = true;
        app.live.phase_index = 3;
        app.live.transfer.pct = 50;
        app.service_info.state = crate::systemd::ServiceState::Idle;
        if app.service_info.state == crate::systemd::ServiceState::Idle || app.service_info.state == crate::systemd::ServiceState::Failed {
            app.live.is_syncing = false;
            app.live.transfer = crate::monitor::parser::TransferStats::default();
        }
        assert!(!app.is_syncing());
        assert_eq!(app.live.transfer.pct, 0);

        // 3. Test HistoryDetails modal command availability
        // Run with NO errors and NO files
        app.past_runs = vec![crate::monitor::history::PastRun {
            id: 1,
            date: "2026-09-18".into(),
            time: "14:00:00".into(),
            duration: "5s".into(),
            status: crate::monitor::history::RunStatus::Success,
            files_copied: vec![],
            files_modified: vec![],
            files_deleted: vec![],
            synced_files: vec![],
            errors: vec![],
            summary: "0 changement".into(),
        }];

        app.toast = None;
        app.modal = Modal::HistoryDetails(0);

        // Keys y/c should NOT copy anything (disabled / unusable)
        app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('y'),
            crossterm::event::KeyModifiers::NONE,
        ));
        assert!(app.toast.is_none());

        app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('c'),
            crossterm::event::KeyModifiers::NONE,
        ));
        assert!(app.toast.is_none());

        // Key d and Enter should NOT attempt to open files
        app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('d'),
            crossterm::event::KeyModifiers::NONE,
        ));
        assert!(app.toast.is_none());

        // Key q closes the modal
        app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('q'),
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(app.modal, Modal::None);
    }

    #[tokio::test]
    async fn test_scrollbar_cursor_hiding_and_open_commands() {
        let mut app = App::new();

        // 1. Test is_dragging_scrollbar
        assert!(!app.is_dragging_scrollbar(ScrollbarTarget::History));
        app.active_scrollbar_drag = Some((ScrollbarTarget::History, 5, 20, 50, 10));
        assert!(app.is_dragging_scrollbar(ScrollbarTarget::History));
        assert!(!app.is_dragging_scrollbar(ScrollbarTarget::RecentFiles));
        app.active_scrollbar_drag = None;
        assert!(!app.is_dragging_scrollbar(ScrollbarTarget::History));

        // 2. Test apply_scrollbar_jump positions cursor at top or bottom boundary
        app.history_viewport_height = 5;
        // Total 20 runs, viewport 5 -> max_offset = 15
        // Jump to bottom (offset 9 / 10 -> ratio 1.0)
        app.apply_scrollbar_jump(ScrollbarTarget::History, 9, 10, 20, 5);
        assert_eq!(app.history_scroll_offset, 15);
        // Cursor at bottom of visible items: 15 + 5 - 1 = 19
        assert_eq!(app.selected_run_idx, Some(19));

        // Jump to top (offset 0 / 10 -> ratio 0.0)
        app.apply_scrollbar_jump(ScrollbarTarget::History, 0, 10, 20, 5);
        assert_eq!(app.history_scroll_offset, 0);
        // Cursor at top of visible items: 0
        assert_eq!(app.selected_run_idx, Some(0));

        // 3. Test open file / folder command in history details
        app.past_runs = vec![crate::monitor::history::PastRun {
            id: 1,
            date: "2026-09-18".into(),
            time: "14:00:00".into(),
            duration: "5s".into(),
            status: crate::monitor::history::RunStatus::Success,
            files_copied: vec!["file1.txt".into()],
            files_modified: vec![],
            files_deleted: vec![],
            synced_files: vec![("new".into(), "file1.txt".into(), "14:00".into())],
            errors: vec![],
            summary: "1 fichier copié".into(),
        }];
        app.modal = Modal::HistoryDetails(0);
        app.history_selected_file_idx = 0;
        app.ctrl_mode = false;

        // Enter in file mode calls open_selected_file
        app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        ));
        assert!(app.toast.is_some());

        // Enter with Ctrl or ctrl_mode = true calls open_history_folder
        app.ctrl_mode = true;
        app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        ));
        let (msg, _) = app.toast.as_ref().unwrap();
        assert!(msg.contains("Parent folder") || msg.contains("Unable") || msg.contains("Opened"));
    }

    #[tokio::test]
    async fn test_log_filter_tabs_and_shortcuts() {
        let mut app = App::new();
        assert_eq!(app.log_filter, LogFilter::All);

        // Test next / prev
        assert_eq!(app.log_filter.next(), LogFilter::Files);
        assert_eq!(app.log_filter.next().next(), LogFilter::Problems);
        assert_eq!(app.log_filter.next().next().next(), LogFilter::All);
        assert_eq!(app.log_filter.prev(), LogFilter::Problems);

        // Matching logic
        let file_log = "2026-09-18 12:00:00 NOTICE: test.txt: Copied (new)";
        let err_log = "2026-09-18 12:00:00 ERROR: failed to copy: network error";
        let info_log = "2026-09-18 12:00:00 INFO: starting sync";

        assert!(LogFilter::All.matches(file_log));
        assert!(LogFilter::All.matches(err_log));
        assert!(LogFilter::All.matches(info_log));

        assert!(LogFilter::Files.matches(file_log));
        assert!(!LogFilter::Files.matches(err_log));
        assert!(!LogFilter::Files.matches(info_log));

        assert!(!LogFilter::Problems.matches(file_log));
        assert!(LogFilter::Problems.matches(err_log));
        assert!(!LogFilter::Problems.matches(info_log));

        // Keyboard switching when focused on Logs panel
        app.focused_panel = FocusedPanel::Logs;
        app.auto_scroll = true;

        // Right arrow moves to next tab
        app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Right,
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(app.log_filter, LogFilter::Files);
        assert!(app.auto_scroll); // auto-scroll preserved!

        // Left arrow moves to previous tab
        app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Left,
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(app.log_filter, LogFilter::All);

        // 'f' cycles filter tab
        app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('f'),
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(app.log_filter, LogFilter::Files);

        // Number keys 1-3 jump directly to filter
        app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('3'),
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(app.log_filter, LogFilter::Problems);

        // Mouse click on selector left/right arrows
        app.hitboxes = vec![
            Hitbox {
                rect: ratatui::layout::Rect { x: 10, y: 10, width: 1, height: 1 },
                action: HitAction::LogFilterPrev,
            },
            Hitbox {
                rect: ratatui::layout::Rect { x: 12, y: 10, width: 1, height: 1 },
                action: HitAction::LogFilterNext,
            },
        ];
        app.handle_mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 10,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        assert_eq!(app.log_filter, LogFilter::Files);

        app.handle_mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 12,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        assert_eq!(app.log_filter, LogFilter::Problems);

        // Setting 6 opens full logs
        app.modal = Modal::Settings;
        app.settings_tab = 0;
        app.settings_selected_idx = 6;
        let action = app.handle_key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(action, Action::OpenFullLogs);
    }

    #[tokio::test]
    async fn test_new_appearance_and_layout_settings_cycle() {
        let mut app = App::new();
        app.settings_tab = 1;
        app.config.container_layout = crate::config::ContainerLayout::Default;
        app.config.mid_panel_order = crate::config::MidPanelOrder::HistoryLogs;
        app.config.border_style = crate::config::BorderStyleChoice::Rounded;
        app.config.graph_style = crate::config::GraphStyleChoice::Blocks;

        // 1. ContainerLayout (idx 1)
        app.settings_selected_idx = 1;
        assert_eq!(app.config.container_layout, crate::config::ContainerLayout::Default);
        app.cycle_setting(true);
        assert_eq!(app.config.container_layout, crate::config::ContainerLayout::RecentFirst);
        app.cycle_setting(false);
        assert_eq!(app.config.container_layout, crate::config::ContainerLayout::Default);

        // 2. MidPanelOrder (idx 2)
        app.settings_selected_idx = 2;
        assert_eq!(app.config.mid_panel_order, crate::config::MidPanelOrder::HistoryLogs);
        app.cycle_setting(true);
        assert_eq!(app.config.mid_panel_order, crate::config::MidPanelOrder::LogsHistory);
        app.cycle_setting(true);
        assert_eq!(app.config.mid_panel_order, crate::config::MidPanelOrder::HistoryLogs);

        // 3. BorderStyle (idx 3)
        app.settings_selected_idx = 3;
        assert_eq!(app.config.border_style, crate::config::BorderStyleChoice::Rounded);
        assert_eq!(app.border_type(), BorderType::Rounded);
        app.cycle_setting(true);
        assert_eq!(app.config.border_style, crate::config::BorderStyleChoice::Sharp);
        assert_eq!(app.border_type(), BorderType::Plain);

        // 4. GraphStyle (idx 4)
        app.settings_selected_idx = 4;
        assert_eq!(app.config.graph_style, crate::config::GraphStyleChoice::Blocks);
        app.cycle_setting(true);
        assert_eq!(app.config.graph_style, crate::config::GraphStyleChoice::Braille);
        app.cycle_setting(true);
        assert_eq!(app.config.graph_style, crate::config::GraphStyleChoice::Blocks);
    }

    #[tokio::test]
    async fn test_settings_tab_switching() {
        let mut app = App::new();
        app.modal = Modal::Settings;
        assert_eq!(app.settings_tab, 0);
        assert_eq!(app.settings_selected_idx, 0);

        // Press Tab to switch to Tab 1 (UI)
        app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.settings_tab, 1);
        assert_eq!(app.settings_selected_idx, 0);

        // Press Tab again to switch back to Tab 0 (Rclone)
        app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.settings_tab, 0);

        // Direct switch with key '2'
        app.handle_key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE));
        assert_eq!(app.settings_tab, 1);

        // Direct switch with key '1'
        app.handle_key(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE));
        assert_eq!(app.settings_tab, 0);
    }

    #[tokio::test]
    async fn test_filter_type_cycling() {
        let mut app = App::new();
        app.modal = Modal::Filters;
        app.filters = vec![
            "- /my_folder/**".to_string(),
            "+ *.pdf".to_string(),
            "# ignored comment".to_string(),
        ];
        app.selected_filter_idx = 0;

        // '-' cycles to '#'
        app.cycle_filter_type(0);
        assert_eq!(app.filters[0], "# /my_folder/**");

        // '#' cycles to raw rule
        app.cycle_filter_type(0);
        assert_eq!(app.filters[0], "/my_folder/**");

        // raw rule cycles to '+'
        app.cycle_filter_type(0);
        assert_eq!(app.filters[0], "+ /my_folder/**");

        // '+' cycles back to '-'
        app.cycle_filter_type(0);
        assert_eq!(app.filters[0], "- /my_folder/**");

        // Test via key 't'
        app.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE));
        assert_eq!(app.filters[0], "# /my_folder/**");

        // Test via key ' ' (Space)
        app.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        assert_eq!(app.filters[0], "/my_folder/**");
    }

    #[tokio::test]
    async fn test_filter_inline_editing() {
        let mut app = App::new();
        app.modal = Modal::Filters;
        app.filters = vec![
            "- /test/**".to_string(),
            "+ *.txt".to_string(),
        ];
        app.selected_filter_idx = 0;

        // Press 'e' or Enter to start editing
        app.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
        assert!(app.is_editing_filter);
        assert!(!app.is_adding_filter);
        assert_eq!(app.filter_edit_buffer, "- /test/**");

        // Type additional characters
        app.handle_key(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE));
        assert_eq!(app.filter_edit_buffer, "- /test/**1");

        // Backspace
        app.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(app.filter_edit_buffer, "- /test/**");

        // Commit with Enter
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(!app.is_editing_filter);
        assert_eq!(app.filters[0], "- /test/**");

        // Start editing and cancel with Esc
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(app.is_editing_filter);
        app.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        assert_eq!(app.filter_edit_buffer, "- /test/**x");
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!app.is_editing_filter);
        assert_eq!(app.filters[0], "- /test/**"); // Reverted!
    }

    #[tokio::test]
    async fn test_filter_adding_and_deleting() {
        let mut app = App::new();
        app.modal = Modal::Filters;
        app.filters = vec![
            "- /old/**".to_string(),
        ];
        app.selected_filter_idx = 0;

        // Press 'a' to add a new filter
        app.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        assert!(app.is_editing_filter);
        assert!(app.is_adding_filter);
        assert_eq!(app.filter_edit_buffer, "- ");

        // Type rule content
        for c in "custom/**".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        assert_eq!(app.filter_edit_buffer, "- custom/**");

        // Commit with Enter
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(!app.is_editing_filter);
        assert_eq!(app.filters.len(), 2);
        assert_eq!(app.filters[1], "- custom/**");
        assert_eq!(app.selected_filter_idx, 1);

        // Delete with 'd'
        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert_eq!(app.filters.len(), 1);
        assert_eq!(app.filters[0], "- /old/**");
        assert_eq!(app.selected_filter_idx, 0);
    }

    #[tokio::test]
    async fn test_filter_mouse_actions() {
        let mut app = App::new();
        app.modal = Modal::Filters;
        app.filters = vec![
            "- /row0/**".to_string(),
            "+ /row1/**".to_string(),
        ];
        app.selected_filter_idx = 0;

        // HitAction::FilterAdd
        app.hitboxes = vec![
            Hitbox {
                rect: ratatui::layout::Rect { x: 50, y: 10, width: 10, height: 1 },
                action: HitAction::FilterAdd,
            },
        ];
        app.handle_mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 52,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        assert!(app.is_editing_filter);
        assert!(app.is_adding_filter);

        app.cancel_filter_edit();

        // HitAction::FilterDelete
        app.hitboxes = vec![
            Hitbox {
                rect: ratatui::layout::Rect { x: 50, y: 12, width: 10, height: 1 },
                action: HitAction::FilterDelete(1),
            },
        ];
        app.handle_mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 52,
            row: 12,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        assert_eq!(app.filters.len(), 1);
        assert_eq!(app.filters[0], "- /row0/**");
    }

    #[tokio::test]
    async fn test_timer_interval_options_cycle_forward_and_backward() {
        let mut app = App::new();
        app.modal = Modal::Settings;
        app.settings_tab = 0;
        app.settings_selected_idx = 0; // timer_interval
        app.config.timer_interval = "10min".to_string();

        // Forward cycle: 10min -> 15min -> 30min -> 1h -> 2h -> 4h -> 10min
        app.cycle_setting(true);
        assert_eq!(app.config.timer_interval, "15min");
        app.cycle_setting(true);
        assert_eq!(app.config.timer_interval, "30min");
        app.cycle_setting(true);
        assert_eq!(app.config.timer_interval, "1h");
        app.cycle_setting(true);
        assert_eq!(app.config.timer_interval, "2h"); // Visited 2h!
        app.cycle_setting(true);
        assert_eq!(app.config.timer_interval, "4h"); // Visited 4h!
        app.cycle_setting(true);
        assert_eq!(app.config.timer_interval, "10min");

        // Backward cycle: 10min -> 4h -> 2h -> 1h -> 30min -> 15min -> 10min
        app.cycle_setting(false);
        assert_eq!(app.config.timer_interval, "4h");
        app.cycle_setting(false);
        assert_eq!(app.config.timer_interval, "2h");
        app.cycle_setting(false);
        assert_eq!(app.config.timer_interval, "1h");
        app.cycle_setting(false);
        assert_eq!(app.config.timer_interval, "30min");
        app.cycle_setting(false);
        assert_eq!(app.config.timer_interval, "15min");
        app.cycle_setting(false);
        assert_eq!(app.config.timer_interval, "10min");
    }

    #[tokio::test]
    async fn test_click_outside_modal_dismissal() {
        let mut app = App::new();
        app.modal = Modal::Settings;
        app.active_modal_area = Some(ratatui::layout::Rect { x: 20, y: 10, width: 40, height: 20 });

        // Background hitbox that shouldn't be clicked when modal is open
        app.hitboxes = vec![
            Hitbox {
                rect: ratatui::layout::Rect { x: 5, y: 5, width: 10, height: 1 },
                action: HitAction::ButtonSync,
            },
        ];

        // Click outside the active modal area at (5, 5)
        let action = app.handle_mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 5,
            row: 5,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });

        // The modal should be dismissed, and ButtonSync must NOT be executed!
        assert_eq!(app.modal, Modal::None);
        assert_eq!(action, Action::None);

        // Re-open modal and click inside active_modal_area at (25, 15)
        app.modal = Modal::Settings;
        app.active_modal_area = Some(ratatui::layout::Rect { x: 20, y: 10, width: 40, height: 20 });
        let action2 = app.handle_mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 25,
            row: 15,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        // Modal is NOT dismissed by click inside
        assert_eq!(app.modal, Modal::Settings);
        assert_eq!(action2, Action::None);
    }

    #[tokio::test]
    async fn test_modal_scroll_isolation() {
        let mut app = App::new();
        app.modal = Modal::Filters;
        app.filters = vec![
            "- /row0/**".to_string(),
            "+ /row1/**".to_string(),
            "- /row2/**".to_string(),
        ];
        app.selected_filter_idx = 0;
        app.logs_scroll = 0;

        // ScrollDown while Filters modal is open
        app.handle_mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::ScrollDown,
            column: 10,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });

        // Filters selection scrolled to 1, while background logs remained 0!
        assert_eq!(app.selected_filter_idx, 1);
        assert_eq!(app.logs_scroll, 0);

        // ScrollUp while Filters modal is open
        app.handle_mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::ScrollUp,
            column: 10,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::NONE,
        });
        assert_eq!(app.selected_filter_idx, 0);
        assert_eq!(app.logs_scroll, 0);
    }

    #[tokio::test]
    async fn test_universal_scroll_primitives_and_scrollbar_harmony() {
        // 1. Direct unit test of scroll_list_step
        let mut sel = 0;
        let mut offset = 0;
        let total = 20;
        let vp = 5;

        // Scroll down 4 times (within viewport)
        for expected in 1..=4 {
            App::scroll_list_step(&mut sel, &mut offset, total, vp, false);
            assert_eq!(sel, expected);
            assert_eq!(offset, 0);
        }

        // Scroll down 5th time (hits viewport bottom, offset shifts)
        App::scroll_list_step(&mut sel, &mut offset, total, vp, false);
        assert_eq!(sel, 5);
        assert_eq!(offset, 1);

        // Scroll up
        App::scroll_list_step(&mut sel, &mut offset, total, vp, true);
        assert_eq!(sel, 4);
        assert_eq!(offset, 1);

        // 2. Direct unit test of scroll_list_jump
        App::scroll_list_jump(&mut sel, &mut offset, total, vp, 1.0);
        assert_eq!(offset, 15); // total(20) - vp(5) = 15
        assert_eq!(sel, 19);

        App::scroll_list_jump(&mut sel, &mut offset, total, vp, 0.0);
        assert_eq!(offset, 0);
        assert_eq!(sel, 0);

        // 3. Test ScrollbarTarget::Filters step and jump via App
        let mut app = App::new();
        app.filters = (0..20).map(|i| format!("- rule_{}", i)).collect();
        app.filter_viewport_height = 5;
        app.selected_filter_idx = 0;
        app.filter_scroll_offset = 0;

        app.apply_scrollbar_step(ScrollbarTarget::Filters, false);
        assert_eq!(app.selected_filter_idx, 1);

        app.apply_scrollbar_jump(ScrollbarTarget::Filters, 5, 10, 20, 5);
        assert!(app.filter_scroll_offset > 0);
        assert!(app.selected_filter_idx >= app.filter_scroll_offset);
    }
}


