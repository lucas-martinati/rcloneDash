use std::time::Instant;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use ratatui::widgets::BorderType;

use crate::config::{self, AppConfig};
use crate::fs_tree::{self, FileEntry};
use crate::monitor::{fetch_past_runs, spawn_log_streamer, PastRun, RunStatus, SharedStreamer, StreamerState};
use crate::systemd::{self, get_service_info, ServiceInfo, ServiceState};
use crate::term_caps::{TermCaps, TtyMode};
use crate::ui::theme::ThemeChoice;


pub mod filters;
pub mod settings;
pub mod files;
pub mod scroll;
pub mod events;

#[cfg(test)]
mod tests;

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
}

pub use crate::config::LogFilter;

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
    ConfirmResync,
    ConfirmCancel,
    ConfirmDelete(String),
    Help,
    HistoryDetails(usize),
    FirstRun(Box<FirstRunState>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FirstRunStep {
    RcloneCheck,
    RemoteSetup,
    GoogleCredentials,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RcloneInstallStatus {
    NotInstalled,
    Installing,
    Installed(String),
    Failed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstRunField {
    // Step 1
    InstallRcloneButton,
    ContinueButton,
    SkipRcloneButton,

    // Step 2 : choix du remote (tous providers)
    RemoteInput,
    RemoteContinueButton,
    RemoteSkipButton,

    // Step 3 (uniquement si remote Google Drive)
    ClientIdInput,
    ClientSecretInput,
    SaveCredentialsButton,
    SkipCredentialsButton,
    ToggleHelpButton,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstRunState {
    pub step: FirstRunStep,
    pub rclone_status: RcloneInstallStatus,
    pub remote_input: String,
    pub remote_cursor: usize,
    pub client_id: String,
    pub client_secret: String,
    pub active_field: FirstRunField,
    pub show_help: bool,
    pub client_id_cursor: usize,
    pub client_secret_cursor: usize,
}

impl FirstRunState {
    pub fn new(remote: &str) -> Self {
        let rclone_version = crate::installer::check_rclone_installed();
        let (existing_id, existing_secret) = crate::config::read_rclone_credentials(remote);
        let (status, step, field) = match rclone_version {
            Some(ver) => (
                RcloneInstallStatus::Installed(ver),
                FirstRunStep::RcloneCheck,
                FirstRunField::ContinueButton,
            ),
            None => (
                RcloneInstallStatus::NotInstalled,
                FirstRunStep::RcloneCheck,
                FirstRunField::InstallRcloneButton,
            ),
        };

        let client_id = existing_id.unwrap_or_default();
        let client_secret = existing_secret.unwrap_or_default();
        let client_id_cursor = client_id.len();
        let client_secret_cursor = client_secret.len();

        // Pré-remplit le champ remote : remote actuel s'il existe dans
        // rclone.conf, sinon le premier remote détecté, sinon le remote passé.
        let remote_input = {
            let trimmed = remote.trim();
            if crate::rclone::remote_exists(trimmed) {
                trimmed.to_string()
            } else if let Some(first) = crate::rclone::list_remotes().into_iter().next() {
                first.display_name()
            } else {
                trimmed.to_string()
            }
        };
        let remote_cursor = remote_input.chars().count();

        Self {
            step,
            rclone_status: status,
            remote_input,
            remote_cursor,
            client_id,
            client_secret,
            active_field: field,
            show_help: false,
            client_id_cursor,
            client_secret_cursor,
        }
    }

    /// `true` si l'étape Google Credentials a un sens pour le remote saisi.
    pub fn wants_google_step(&self) -> bool {
        crate::rclone::is_drive_remote(&self.remote_input)
    }
}

/// Sub-state for inline text editing within Settings or Filters modals.
/// Replaces the former booleans `is_editing_setting`, `is_editing_filter`,
/// `is_adding_filter` and their associated `String` buffers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditState {
    /// No editing is active.
    Idle,
    /// Editing a setting value (tab + index identify *which* setting, cursor is char offset).
    Setting { tab: usize, index: usize, buffer: String, cursor: usize },
    /// Editing an existing filter rule.
    Filter { index: usize, buffer: String },
    /// Adding a brand-new filter rule.
    AddingFilter { buffer: String },
}

pub use crate::config::STATS_INTERVAL_OPTIONS;

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
pub enum HitAction {
    ButtonSync,
    ButtonCancel,
    ButtonDryRun,
    ButtonFiles,
    ButtonFilters,
    ButtonPanel,
    StatsIntervalDec,
    StatsIntervalInc,
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
    CloseModal,
    MenuOption(usize),
    FileEntry(usize),
    ToggleLogsAuto,
    LogFilterPrev,
    LogFilterNext,
    LogFilterCycle,
    ButtonCopy,
    FilterArea,
    FilterRow(usize),
    FilterCycleType(usize),
    FilterStartEdit,
    FilterAdd,
    FilterDelete(usize),
    FilterOpenEditor,
    FirstRunInstall,
    FirstRunContinue,
    FirstRunSkipRclone,
    FirstRunRemoteInput,
    FirstRunRemoteContinue,
    FirstRunRemoteSkip,
    FirstRunClientId,
    FirstRunClientSecret,
    FirstRunSaveCredentials,
    FirstRunSkipCredentials,
    FirstRunToggleHelp,
    ToggleBox(usize),
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

/// Snapshot of an ongoing scrollbar drag, captured at the moment the user clicks
/// the track. `total` and `visible` are frozen at click time and may become
/// stale if new items arrive during the drag (e.g. incoming log lines); this
/// matches the behaviour of the previous implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollbarDragState {
    pub target: ScrollbarTarget,
    pub top_y: u16,
    pub track_height: u16,
    pub total: usize,
    pub visible: usize,
    pub grab_offset: u16,
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

/// Dual-layer spatial manager for clickable areas.
/// The `dashboard` layer is active when no modal is open;
/// the `modal` layer takes precedence when a modal is visible.
pub struct HitboxManager {
    pub dashboard: Vec<Hitbox>,
    pub modal: Vec<Hitbox>,
    pub active_modal_area: Option<Rect>,
}

impl HitboxManager {
    pub fn new() -> Self {
        Self {
            dashboard: Vec::with_capacity(64),
            modal: Vec::with_capacity(32),
            active_modal_area: None,
        }
    }

    /// Returns the appropriate hitbox layer for the current modal state.
    pub fn active_layer(&self, has_modal: bool) -> &[Hitbox] {
        if has_modal && !self.modal.is_empty() {
            &self.modal
        } else {
            &self.dashboard
        }
    }
}

pub struct App {
    pub running: bool,
    pub modal: Modal,
    pub config: AppConfig,
    pub current_theme: ThemeChoice,
    /// CLI `--tty-mode` override (wins over `config.tty_mode`, never saved).
    pub tty_mode_override: Option<TtyMode>,
    /// Resolved terminal capabilities (colors / live animation / ASCII).
    pub term_caps: TermCaps,
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

    // Active panel
    pub focused_panel: FocusedPanel,
    pub stats_interval_changed: bool,
    pub menu_selected_idx: usize,
    pub ctrl_mode: bool,
    pub recent_filter: String,
    pub is_filtering_recent: bool,

    // File explorer
    pub file_current_rel: String,
    pub file_entries: Vec<FileEntry>,
    pub file_selected_idx: usize,
    pub file_scroll_offset: usize,
    pub file_viewport_height: usize,

    // Scroll offsets and viewport heights for history, filters, logs, and recent files
    pub history_scroll_offset: usize,
    pub history_details_scroll: usize,
    pub history_viewport_height: usize,
    pub filter_scroll_offset: usize,
    pub filter_viewport_height: usize,
    pub recent_scroll_offset: usize,
    pub recent_viewport_height: usize,
    pub logs_viewport_height: usize,
    pub logs_total_wrapped: usize,

    // Settings
    pub settings_tab: usize,
    pub settings_selected_idx: usize,

    // Unified editing state (settings + filters)
    pub edit_state: EditState,

    // Dry-Run simulation
    pub dry_run_running: bool,
    pub dry_run_logs: Vec<String>,
    /// Lignes brutes (nettoyées ANSI) pour le parsing ; `dry_run_logs`
    /// contient la version mise en forme pour l'affichage. Le résumé
    /// (DryRunSummary) doit toujours être calculé sur les lignes brutes,
    /// car le formatage détruit la structure JSON attendue par le parseur.
    pub dry_run_raw_logs: Vec<String>,
    pub dry_run_scroll: usize,

    // Interactive item selection
    pub history_selected_file_idx: usize,
    pub recent_selected_idx: Option<usize>,
    pub active_scrollbar_drag: Option<ScrollbarDragState>,

    // Dual-layer hitbox manager (dashboard / modal)
    pub hit_mgr: HitboxManager,

    pub cloud_quota: Option<CloudQuota>,
    pub tracked_files_count: usize,
    pub available_update: Option<String>,

    // Unified async background event pipeline (replaces 7 polling std::sync::mpsc channels)
    bg_tx: tokio::sync::mpsc::UnboundedSender<BgEvent>,
    bg_rx: tokio::sync::mpsc::UnboundedReceiver<BgEvent>,
    bg_in_flight: BgInFlight,
    last_streamer_gen: u64,

    // Short-lived render I/O caches (keyed / TTL-bounded so direct field mutations stay fresh)
    disk_stats_cache: std::cell::RefCell<Option<DiskStatsCache>>,
    recent_size_cache: std::cell::RefCell<std::collections::HashMap<std::path::PathBuf, (Instant, String)>>,

    last_systemd_check: Instant,
    last_history_check: Instant,
    last_quota_check: Instant,
    last_file_count_check: Instant,
}

#[derive(Debug, Default)]
struct BgInFlight {
    systemd: bool,
    history: bool,
    quota: bool,
    file_count: bool,
    rclone_install: bool,
}

struct DiskStatsCache {
    local_dir: String,
    fetched_at: Instant,
    stats: (f64, f64, f64, f64),
}

/// Asynchronous notifications pushed by background tasks into the main UI loop.
#[derive(Debug)]
pub enum BgEvent {
    UpdateAvailable(String),
    QuotaFetched(CloudQuota),
    FileCounted(usize),
    ServiceInfoUpdated {
        info: ServiceInfo,
        running_elapsed_secs: Option<u64>,
    },
    PastRunsUpdated(Vec<PastRun>),
    DryRunCompleted {
        lines: Vec<String>,
        raw_lines: Vec<String>,
    },
    RcloneInstallFinished(Result<String, String>),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
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
        let log_filter = config.log_filter;
        #[cfg(not(test))]
        let (service_info, past_runs, live, last_streamer_gen) = {
            let info = get_service_info();
            let runs = fetch_past_runs(50);
            let initial_live = streamer
                .try_read()
                .map(|st| st.clone())
                .unwrap_or_default();
            let gen = initial_live.generation;
            (info, runs, initial_live, gen)
        };
        #[cfg(test)]
        let (service_info, past_runs, live, last_streamer_gen) = (
            ServiceInfo {
                state: ServiceState::Idle,
                ..Default::default()
            },
            Vec::new(),
            StreamerState::default(),
            0,
        );
        let filters = config::read_filters();
        let (bg_tx, bg_rx) = tokio::sync::mpsc::unbounded_channel();

        let mut app = Self {
            running: true,
            modal: Modal::None,
            config,
            current_theme,
            tty_mode_override: None,
            term_caps: TermCaps::full(),
            service_info,
            streamer,
            live,
            past_runs,
            selected_run_idx: None,
            filters,
            selected_filter_idx: 0,
            logs_scroll: 0,
            auto_scroll: true,
            log_filter,
            toast: None,

            focused_panel: FocusedPanel::RecentFiles,
            stats_interval_changed: false,
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
            edit_state: EditState::Idle,

            dry_run_running: false,
            dry_run_logs: Vec::new(),
            dry_run_raw_logs: Vec::new(),
            dry_run_scroll: 0,

            history_selected_file_idx: 0,
            recent_selected_idx: None,
            active_scrollbar_drag: None,

            hit_mgr: HitboxManager::new(),

            cloud_quota: config::load_quota_cache().map(|c| CloudQuota {
                total_bytes: c.total_bytes,
                used_bytes: c.used_bytes,
                free_bytes: c.free_bytes,
            }),
            tracked_files_count: 0,
            available_update: None,
            bg_tx,
            bg_rx,
            bg_in_flight: BgInFlight::default(),
            last_streamer_gen,
            disk_stats_cache: std::cell::RefCell::new(None),
            recent_size_cache: std::cell::RefCell::new(std::collections::HashMap::new()),

            last_systemd_check: Instant::now(),
            last_history_check: Instant::now(),
            last_quota_check: Instant::now().checked_sub(std::time::Duration::from_secs(350)).unwrap_or_else(Instant::now),
            last_file_count_check: Instant::now().checked_sub(std::time::Duration::from_secs(70)).unwrap_or_else(Instant::now),
        };

        // Real terminal capabilities in production; deterministic full
        // capabilities in tests (no `std::env` / `is_terminal()` leakage:
        // tests set `term_caps` by hand when they need other levels).
        #[cfg(not(test))]
        app.refresh_term_caps();

        #[cfg(not(test))]
        {
            let tx = app.bg_tx.clone();
            tokio::spawn(async move {
                if let Ok(Some(info)) = crate::updater::check_for_updates().await {
                    let _ = tx.send(BgEvent::UpdateAvailable(info.latest_version));
                }
            });
        }

        app.reload_files();

        #[cfg(not(test))]
        if app.config.first_run_completed != Some(true) {
            app.modal = Modal::FirstRun(Box::new(FirstRunState::new(&app.config.remote)));
        }

        app
    }

    pub fn start_rclone_install(&mut self) {
        if self.bg_in_flight.rclone_install {
            return;
        }
        if let Modal::FirstRun(ref mut state) = self.modal {
            state.rclone_status = RcloneInstallStatus::Installing;
        }
        self.bg_in_flight.rclone_install = true;
        let tx = self.bg_tx.clone();
        tokio::spawn(async move {
            let res = crate::installer::install_rclone_user().await;
            let _ = tx.send(BgEvent::RcloneInstallFinished(res));
        });
    }

    pub fn open_first_run(&mut self) {
        self.modal = Modal::FirstRun(Box::new(FirstRunState::new(&self.config.remote)));
    }

    /// Persiste le remote saisi dans le wizard : met à jour `config.remote`
    /// (si non-vide et différent), sauvegarde et recharge l'explorateur.
    /// Retourne `true` si l'étape Google Credentials a un sens (Drive).
    /// Chemin unique pour le clavier ET la souris.
    pub fn apply_first_run_remote(&mut self, remote_input: &str) -> bool {
        let raw = remote_input.trim();
        if !raw.is_empty() && raw != self.config.remote {
            self.config.remote = raw.to_string();
            let _ = config::save_config(&self.config);
            self.reload_files();
        }
        self.is_drive_remote()
    }

    /// Suite commune clavier/souris après "Continue" de l'étape remote :
    /// pré-remplit et affiche les credentials Google (Drive), ou termine
    /// directement le setup (autres providers).
    pub fn advance_from_remote_continue(&mut self) {
        let remote = if let Modal::FirstRun(ref state) = self.modal {
            state.remote_input.clone()
        } else {
            return;
        };
        if self.apply_first_run_remote(&remote) {
            if let Modal::FirstRun(ref mut state) = self.modal {
                let (id, secret) = config::read_rclone_credentials(&self.config.remote);
                state.client_id = id.unwrap_or_default();
                state.client_secret = secret.unwrap_or_default();
                state.client_id_cursor = state.client_id.chars().count();
                state.client_secret_cursor = state.client_secret.chars().count();
                state.step = FirstRunStep::GoogleCredentials;
                state.active_field = FirstRunField::ClientIdInput;
            }
        } else {
            self.config.first_run_completed = Some(true);
            let _ = config::save_config(&self.config);
            self.set_toast(format!("✔ Remote configured: {}", crate::rclone::describe_remote(&self.config.remote)));
            self.modal = Modal::None;
            self.hit_mgr.active_modal_area = None;
        }
    }

    /// Sauvegarde les credentials Google du wizard (chemin unique pour le
    /// clavier ET la souris : `remote_input` non-vide gagne, sinon on garde
    /// le remote configuré). Ferme le wizard en cas de succès.
    pub fn save_first_run_credentials(&mut self, remote_input: &str, client_id: &str, client_secret: &str) {
        let raw = remote_input.trim();
        let target = if raw.is_empty() {
            self.config.remote.clone()
        } else {
            raw.to_string()
        };
        if target != self.config.remote {
            self.config.remote = target.clone();
        }
        match config::write_rclone_credentials(&target, client_id, client_secret) {
            Ok(()) => {
                self.config.first_run_completed = Some(true);
                let _ = config::save_config(&self.config);
                self.modal = Modal::None;
                self.hit_mgr.active_modal_area = None;
                self.set_toast("✔ Google Drive credentials saved! Setup complete.");
            }
            Err(e) => {
                self.set_toast(format!("✗ Failed to save credentials: {}", e));
            }
        }
    }

    pub fn border_type(&self) -> BorderType {
        // Linux console / non-UTF-8: rounded/double/thick corners do not
        // exist there — plain box drawing (present in console fonts).
        if self.term_caps.ascii {
            return BorderType::Plain;
        }
        self.config.border_style.to_border_type()
    }

    /// Effective TTY mode: the CLI `--tty-mode` override wins over config.
    pub fn effective_tty_mode(&self) -> TtyMode {
        self.tty_mode_override.unwrap_or(self.config.tty_mode)
    }

    /// Recomputes [`Self::term_caps`] from the effective TTY mode and the
    /// real terminal. A no-op in tests (deterministic `full()` caps); an
    /// explicit `--tty-mode=off` override forces colors even piped.
    /// Single source of truth: every refresh goes through here, so the
    /// force flag can never be lost by a later refresh.
    pub fn refresh_term_caps(&mut self) {
        #[cfg(not(test))]
        {
            let forced = self.tty_mode_override == Some(TtyMode::Off);
            self.term_caps = TermCaps::resolve_forced(self.effective_tty_mode(), forced);
        }
    }

    /// Applies a CLI `--tty-mode` override (kept out of the saved config).
    pub fn apply_tty_override(&mut self, mode: TtyMode) {
        self.tty_mode_override = Some(mode);
        self.refresh_term_caps();
    }

    pub fn border_glyphs(&self) -> crate::config::BorderGlyphs {
        if self.term_caps.ascii {
            return crate::config::BorderStyleChoice::Sharp.glyphs();
        }
        self.config.border_style.glyphs()
    }

    /// Glyph set matching the resolved terminal capabilities.
    pub fn glyphs(&self) -> crate::term_caps::Glyphs {
        self.term_caps.glyphs()
    }

    // ───── EditState helpers ─────

    /// Returns `true` when the user is editing a setting text field.
    pub fn is_editing_setting(&self) -> bool {
        matches!(self.edit_state, EditState::Setting { .. })
    }

    /// Returns `true` when the user is editing or adding a filter rule.
    pub fn is_editing_filter(&self) -> bool {
        matches!(self.edit_state, EditState::Filter { .. } | EditState::AddingFilter { .. })
    }

    /// Returns `true` when the user is adding a **new** filter (not editing an existing one).
    pub fn is_adding_filter(&self) -> bool {
        matches!(self.edit_state, EditState::AddingFilter { .. })
    }

    /// Returns a reference to the current edit buffer, if any.
    pub fn edit_buffer(&self) -> &str {
        match &self.edit_state {
            EditState::Setting { buffer, .. } => buffer,
            EditState::Filter { buffer, .. } => buffer,
            EditState::AddingFilter { buffer } => buffer,
            EditState::Idle => "",
        }
    }

    /// Returns a mutable reference to the current edit buffer, if any.
    pub fn edit_buffer_mut(&mut self) -> Option<&mut String> {
        match &mut self.edit_state {
            EditState::Setting { buffer, .. } => Some(buffer),
            EditState::Filter { buffer, .. } => Some(buffer),
            EditState::AddingFilter { buffer } => Some(buffer),
            EditState::Idle => None,
        }
    }

    /// Returns the cursor position (in characters) within the current edit buffer.
    pub fn edit_cursor(&self) -> usize {
        match &self.edit_state {
            EditState::Setting { cursor, .. } => *cursor,
            _ => self.edit_buffer().chars().count(),
        }
    }

    pub fn is_box_visible(&self, box_num: usize) -> bool {
        match box_num {
            1 => self.config.show_cadrans,
            2 => self.config.show_metrics,
            3 => self.config.show_history,
            4 => self.config.show_logs,
            5 => self.config.show_recent,
            _ => false,
        }
    }

    pub fn any_box_visible(&self) -> bool {
        self.config.show_cadrans
            || self.config.show_metrics
            || self.config.show_history
            || self.config.show_logs
            || self.config.show_recent
    }

    pub fn toggle_box(&mut self, box_num: usize) {
        match box_num {
            1 => {
                self.config.show_cadrans = !self.config.show_cadrans;
                let state = if self.config.show_cadrans { "shown" } else { "hidden" };
                self.set_toast(format!("Disks & Cloud box {}", state));
            }
            2 => {
                self.config.show_metrics = !self.config.show_metrics;
                let state = if self.config.show_metrics { "shown" } else { "hidden" };
                self.set_toast(format!("Metrics box {}", state));
            }
            3 => {
                self.config.show_history = !self.config.show_history;
                let state = if self.config.show_history { "shown" } else { "hidden" };
                self.set_toast(format!("History box {}", state));
            }
            4 => {
                self.config.show_logs = !self.config.show_logs;
                let state = if self.config.show_logs { "shown" } else { "hidden" };
                self.set_toast(format!("Logs box {}", state));
            }
            5 => {
                self.config.show_recent = !self.config.show_recent;
                let state = if self.config.show_recent { "shown" } else { "hidden" };
                self.set_toast(format!("Recent Files box {}", state));
            }
            _ => return,
        }
        self.adjust_focused_panel();
        let _ = crate::config::save_config(&self.config);
    }

    pub fn set_log_filter(&mut self, filter: LogFilter) {
        self.log_filter = filter;
        self.config.log_filter = filter;
        let _ = crate::config::save_config(&self.config);
    }

    pub fn adjust_focused_panel(&mut self) {
        let is_current_visible = match self.focused_panel {
            FocusedPanel::History => self.config.show_history,
            FocusedPanel::Logs => self.config.show_logs,
            FocusedPanel::RecentFiles => self.config.show_recent,
        };
        if !is_current_visible {
            self.next_visible_panel();
        }
    }

    pub fn next_visible_panel(&mut self) {
        let mut cur = self.focused_panel;
        for _ in 0..3 {
            cur = cur.next();
            let visible = match cur {
                FocusedPanel::History => self.config.show_history,
                FocusedPanel::Logs => self.config.show_logs,
                FocusedPanel::RecentFiles => self.config.show_recent,
            };
            if visible {
                self.focused_panel = cur;
                break;
            }
        }
    }

    pub fn prev_visible_panel(&mut self) {
        let mut cur = self.focused_panel;
        for _ in 0..3 {
            cur = cur.prev();
            let visible = match cur {
                FocusedPanel::History => self.config.show_history,
                FocusedPanel::Logs => self.config.show_logs,
                FocusedPanel::RecentFiles => self.config.show_recent,
            };
            if visible {
                self.focused_panel = cur;
                break;
            }
        }
    }

    /// Awaits the next asynchronous background notification (cancel-safe for `tokio::select!`).
    pub async fn recv_bg_event(&mut self) -> Option<BgEvent> {
        self.bg_rx.recv().await
    }

    /// Drains all immediately available background events without blocking.
    pub fn drain_bg_events(&mut self) {
        while let Ok(ev) = self.bg_rx.try_recv() {
            self.handle_bg_event(ev);
        }
    }

    /// Applies a single background event to the application state.
    pub fn handle_bg_event(&mut self, event: BgEvent) {
        match event {
            BgEvent::UpdateAvailable(latest_ver) => {
                self.available_update = Some(latest_ver);
            }
            BgEvent::QuotaFetched(q) => {
                self.bg_in_flight.quota = false;
                let _ = config::save_quota_cache(&config::CloudQuotaCache {
                    total_bytes: q.total_bytes,
                    used_bytes: q.used_bytes,
                    free_bytes: q.free_bytes,
                });
                self.cloud_quota = Some(q);
            }
            BgEvent::FileCounted(count) => {
                self.bg_in_flight.file_count = false;
                self.tracked_files_count = count;
            }
            BgEvent::ServiceInfoUpdated {
                info,
                running_elapsed_secs,
            } => {
                self.bg_in_flight.systemd = false;
                self.service_info = info;
                if self.service_info.state == ServiceState::Idle
                    || self.service_info.state == ServiceState::Failed
                {
                    self.live.is_syncing = false;
                    self.live.sync_start = None;
                    self.live.transfer = crate::monitor::parser::TransferStats::default();
                    self.live.active_files.clear();
                    self.live.synced_files.clear();
                    self.live.changes_local.clear();
                    self.live.changes_remote.clear();
                    self.live.changes_local_details.clear();
                    self.live.changes_remote_details.clear();
                    self.live.path1_modified = false;
                    self.live.path2_modified = false;
                } else if self.service_info.state == ServiceState::Active && self.live.sync_start.is_none() {
                    let secs = running_elapsed_secs.unwrap_or(0);
                    self.live.sync_start = Some(
                        Instant::now()
                            .checked_sub(std::time::Duration::from_secs(secs))
                            .unwrap_or_else(Instant::now),
                    );
                }
            }
            BgEvent::PastRunsUpdated(runs) => {
                self.bg_in_flight.history = false;
                self.past_runs = runs;
            }
            BgEvent::DryRunCompleted { lines, raw_lines } => {
                if self.dry_run_running {
                    self.dry_run_logs = lines;
                    self.dry_run_raw_logs = raw_lines;
                    self.dry_run_running = false;
                    self.set_toast("✔ Dry-Run simulation completed!");
                }
            }
            BgEvent::RcloneInstallFinished(res) => {
                self.bg_in_flight.rclone_install = false;
                if let Modal::FirstRun(ref mut state) = self.modal {
                    match res {
                        Ok(msg) => {
                            state.rclone_status = RcloneInstallStatus::Installed(msg);
                            state.active_field = FirstRunField::ContinueButton;
                            self.set_toast("✔ Rclone installed successfully!");
                        }
                        Err(err) => {
                            state.rclone_status = RcloneInstallStatus::Failed(err);
                            state.active_field = FirstRunField::InstallRcloneButton;
                            self.set_toast("✗ Rclone installation failed");
                        }
                    }
                }
            }
        }
    }

    pub async fn on_tick(&mut self) {
        {
            let st = self.streamer.read().await;
            if st.generation != self.last_streamer_gen {
                self.live = st.clone();
                self.last_streamer_gen = st.generation;
            }
        }

        self.drain_bg_events();

        if self.last_systemd_check.elapsed().as_secs() >= 1 && !self.bg_in_flight.systemd {
            self.last_systemd_check = Instant::now();
            self.bg_in_flight.systemd = true;
            let tx = self.bg_tx.clone();
            let need_elapsed = self.live.sync_start.is_none();
            tokio::task::spawn_blocking(move || {
                let info = get_service_info();
                let running_elapsed_secs = if info.state == ServiceState::Active && need_elapsed {
                    crate::monitor::streamer::get_running_sync_elapsed_seconds()
                } else {
                    None
                };
                let _ = tx.send(BgEvent::ServiceInfoUpdated {
                    info,
                    running_elapsed_secs,
                });
            });
        }

        if self.last_history_check.elapsed().as_secs() >= 6 && !self.bg_in_flight.history {
            self.last_history_check = Instant::now();
            self.bg_in_flight.history = true;
            let tx = self.bg_tx.clone();
            tokio::task::spawn_blocking(move || {
                let runs = fetch_past_runs(50);
                let _ = tx.send(BgEvent::PastRunsUpdated(runs));
            });
        }

        if let Some((_, created)) = self.toast {
            if created.elapsed().as_secs() > 4 {
                self.toast = None;
            }
        }

        if self.last_quota_check.elapsed().as_secs() >= 300 && !self.bg_in_flight.quota {
            self.last_quota_check = Instant::now();
            self.bg_in_flight.quota = true;
            let remote = self.config.remote.clone();
            spawn_quota_fetch(remote, self.bg_tx.clone());
        }

        if self.last_file_count_check.elapsed().as_secs() >= 60 && !self.bg_in_flight.file_count {
            self.last_file_count_check = Instant::now();
            self.bg_in_flight.file_count = true;
            let base = config::expand_tilde(&self.config.local_dir);
            spawn_file_count(base, self.bg_tx.clone());
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

    /// `true` uniquement si le remote configuré est un Google Drive.
    /// Les options Drive (Client ID/Secret, flags --drive-*) sont proposées
    /// uniquement dans ce cas.
    pub fn is_drive_remote(&self) -> bool {
        crate::rclone::is_drive_remote(&self.config.remote)
    }

    /// Libellé convivial du remote configuré pour l'affichage.
    pub fn remote_display(&self) -> String {
        crate::rclone::describe_remote(&self.config.remote)
    }

    /// Liste des réglages visibles pour un onglet, en filtrant les options
    /// spécifiques à Google Drive quand le remote n'est pas un Drive.
    pub fn visible_settings_for_tab(&self, tab: usize) -> Vec<config::SettingId> {
        let cat = match config::SettingCategory::ALL.get(tab) {
            Some(c) => *c,
            None => return Vec::new(),
        };
        let is_drive = self.is_drive_remote();
        config::SettingId::for_category(cat)
            .into_iter()
            .filter(|s| !s.is_drive_only() || is_drive)
            .collect()
    }

    /// Réglage visible à (tab, idx) en tenant compte du filtrage Drive.
    pub fn visible_setting_at(&self, tab: usize, idx: usize) -> Option<config::SettingId> {
        self.visible_settings_for_tab(tab).get(idx).copied()
    }

    /// Nombre de réglages visibles pour un onglet (source de vérité pour
    /// la navigation clavier/souris dans Settings).
    pub fn visible_settings_count(&self, tab: usize) -> usize {
        self.visible_settings_for_tab(tab).len()
    }

    pub fn start_dry_run(&mut self) {
        self.dry_run_running = true;
        let remote_label = crate::rclone::describe_remote(&self.config.remote);
        self.dry_run_logs = vec![
            format!("Analyzing differences between local folder and {}...", remote_label),
            "Executing: rclone bisync --dry-run -v --tpslimit 8".to_string(),
            "This may take a moment...".to_string(),
        ];
        self.dry_run_raw_logs = Vec::new();
        self.dry_run_scroll = 0;
        self.modal = Modal::DryRun;
        self.set_toast("🛡 Dry-Run simulation started...");

        let remote = self.config.remote.clone();
        let drive_args = crate::rclone::drive_extra_args(&remote);
        let local_dir = config::expand_tilde(&self.config.local_dir).to_string_lossy().to_string();
        let filters_path = config::filters_file().to_string_lossy().to_string();
        let filters_exists = config::filters_file().exists();
        let rclone_conf_path = config::rclone_config_file();
        let tx = self.bg_tx.clone();

        tokio::spawn(async move {
            let mut args: Vec<String> = vec![
                "bisync".to_string(),
                remote,
                local_dir,
                "--dry-run".to_string(),
                "-v".to_string(),
                "--use-json-log".to_string(),
                "--tpslimit".to_string(),
                "8".to_string(),
            ];
            if filters_exists {
                args.push("--filter-from".to_string());
                args.push(filters_path);
            }
            args.extend(drive_args);
            args.extend([
                "--conflict-resolve".to_string(),
                "newer".to_string(),
                "--resilient".to_string(),
                "--recover".to_string(),
            ]);
            let mut cmd = tokio::process::Command::new("rclone");
            cmd.args(&args);
            if rclone_conf_path.exists() {
                cmd.arg("--config").arg(&rclone_conf_path);
            }
            let output = cmd.output().await;

            let mut lines = Vec::new();
            let mut raw_lines = Vec::new();
            match output {
                Ok(out) => {
                    let s_out = String::from_utf8_lossy(&out.stdout);
                    let s_err = String::from_utf8_lossy(&out.stderr);
                    for chunk in s_out.lines().chain(s_err.lines()) {
                        for raw_line in chunk.split('\r') {
                            // Copie brute pour DryRunSummary::from_logs (le
                            // formatage d'affichage détruit le JSON parsable).
                            let clean = crate::monitor::streamer::strip_ansi(raw_line).trim().to_string();
                            if !clean.is_empty() {
                                raw_lines.push(clean);
                            }
                            let formatted = crate::monitor::parser::format_log_line_for_display(raw_line);
                            lines.extend(formatted);
                        }
                    }
                    if lines.is_empty() {
                        lines.push("Everything is already in sync (no transfers detected in dry-run).".to_string());
                    }
                }
                Err(e) => {
                    let msg = format!("Error running rclone: {}", e);
                    raw_lines.push(msg.clone());
                    lines.push(msg);
                }
            }
            let _ = tx.send(BgEvent::DryRunCompleted { lines, raw_lines });
        });
    }

    pub fn step_stats_interval(&mut self, increase: bool) {
        let steps = STATS_INTERVAL_OPTIONS;
        let current = self.config.stats_interval.as_str();
        let pos = steps.iter().position(|&s| s == current).unwrap_or(0);
        let next_idx = if increase {
            (pos + 1).min(steps.len() - 1)
        } else {
            pos.saturating_sub(1)
        };
        let new_interval = steps[next_idx];
        if new_interval != current {
            self.config.stats_interval = new_interval.to_string();
            self.stats_interval_changed = true;
            let _ = config::save_config(&self.config);
            self.set_toast(format!("Rclone stats: {}", new_interval));
        }
    }

    pub fn stats_interval_duration(&self) -> std::time::Duration {
        self.config.stats_interval_duration()
    }

    pub fn can_dec_stats_interval(&self) -> bool {
        let steps = STATS_INTERVAL_OPTIONS;
        let current = self.config.stats_interval.as_str();
        let pos = steps.iter().position(|&s| s == current).unwrap_or(0);
        pos > 0
    }

    pub fn can_inc_stats_interval(&self) -> bool {
        let steps = STATS_INTERVAL_OPTIONS;
        let current = self.config.stats_interval.as_str();
        let pos = steps.iter().position(|&s| s == current).unwrap_or(0);
        pos < steps.len() - 1
    }

    pub fn next_theme(&mut self) {
        self.current_theme = self.current_theme.next();
        self.config.theme = Some(self.current_theme);
        let _ = config::save_config(&self.config);
        self.set_toast(format!("Active theme: {}", self.current_theme.name()));
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
        // 1. Live stream synced files
        for sf in self.live.synced_files.iter().rev() {
            list.push((sf.action.to_string(), sf.path.clone(), String::new(), sf.time.clone()));
        }
        // 2. Full history synced files (up to 100 files, web parity)
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

        let base = config::expand_tilde(&self.config.local_dir);
        let now = Instant::now();
        let ttl = std::time::Duration::from_secs(5);
        let mut size_cache = self.recent_size_cache.borrow_mut();
        if size_cache.len() > 256 {
            size_cache.retain(|_, (ts, _)| now.duration_since(*ts) < ttl);
            if size_cache.len() > 256 {
                size_cache.clear();
            }
        }
        for item in list.iter_mut() {
            let full = base.join(&item.1);
            if let Some((ts, cached)) = size_cache.get(&full) {
                if now.duration_since(*ts) < ttl {
                    item.2 = cached.clone();
                    continue;
                }
            }
            let formatted = if let Ok(meta) = std::fs::metadata(&full) {
                fs_tree::format_bytes(meta.len())
            } else {
                "--".to_string()
            };
            size_cache.insert(full, (now, formatted.clone()));
            item.2 = formatted;
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
        let ttl = std::time::Duration::from_secs(2);
        if let Some(ref cached) = *self.disk_stats_cache.borrow() {
            if cached.local_dir == self.config.local_dir && cached.fetched_at.elapsed() < ttl {
                return cached.stats;
            }
        }
        let stats = systemd::get_disk_usage(&self.config.local_dir);
        *self.disk_stats_cache.borrow_mut() = Some(DiskStatsCache {
            local_dir: self.config.local_dir.clone(),
            fetched_at: Instant::now(),
            stats,
        });
        stats
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

        if self.is_syncing() {
            let duration_s = self.live.sync_start.map(|s| s.elapsed().as_secs()).unwrap_or(0);
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

    /// Returns the timer interval in seconds (default 600s = 10min, or 0 if never).
    pub fn timer_cycle_seconds(&self) -> u64 {
        let interval = self.config.timer_interval.trim();
        if interval.eq_ignore_ascii_case("never") {
            return 0;
        }
        if interval.ends_with("min") {
            interval.trim_end_matches("min").parse::<u64>().unwrap_or(10) * 60
        } else if interval.ends_with('m') {
            interval.trim_end_matches('m').parse::<u64>().unwrap_or(10) * 60
        } else if interval.ends_with('h') {
            interval.trim_end_matches('h').parse::<u64>().unwrap_or(1) * 3600
        } else if interval.ends_with('s') {
            interval.trim_end_matches('s').parse::<u64>().unwrap_or(600)
        } else {
            interval.parse::<u64>().map(|m| m * 60).unwrap_or(600)
        }
    }

    /// Returns remaining seconds until the next sync, parsed from `service_info.timer_left`.
    pub fn timer_remaining_seconds(&self) -> Option<u64> {
        if self.config.timer_interval.eq_ignore_ascii_case("never") {
            return None;
        }
        let left = self.service_info.timer_left.trim();
        if left.is_empty() || left == "--" || left == "—" || left.eq_ignore_ascii_case("disabled") {
            return None;
        }
        if left.eq_ignore_ascii_case("imminent") {
            return Some(0);
        }
        if left.starts_with("after sync") || left.starts_with("in ~") {
            return Some(self.timer_cycle_seconds());
        }

        // Handle formats like "4m 12s", "12min", "45s", "12s left", "9min left", "1h 10m"
        let clean = left.trim_end_matches("left").trim();
        let mut total_secs: u64 = 0;
        let mut num_buf = String::new();
        let mut has_parsed = false;

        for c in clean.chars() {
            if c.is_ascii_digit() {
                num_buf.push(c);
            } else if c == 'h' || c == 'H' {
                if let Ok(v) = num_buf.parse::<u64>() {
                    total_secs += v * 3600;
                    has_parsed = true;
                }
                num_buf.clear();
            } else if c == 'm' || c == 'M' {
                if let Ok(v) = num_buf.parse::<u64>() {
                    total_secs += v * 60;
                    has_parsed = true;
                }
                num_buf.clear();
            } else if c == 's' || c == 'S' {
                if let Ok(v) = num_buf.parse::<u64>() {
                    total_secs += v;
                    has_parsed = true;
                }
                num_buf.clear();
            }
        }

        if !num_buf.is_empty() {
            if let Ok(v) = num_buf.parse::<u64>() {
                if !has_parsed {
                    total_secs = v * 60;
                    has_parsed = true;
                } else {
                    total_secs += v;
                }
            }
        }

        if has_parsed {
            Some(total_secs)
        } else {
            None
        }
    }

    /// Computes the countdown progress ratio from 0.0 (cycle just started) to 1.0 (imminent / sync due).
    pub fn timer_progress(&self) -> Option<f64> {
        let cycle = self.timer_cycle_seconds() as f64;
        if cycle <= 0.0 {
            return None;
        }
        let rem = self.timer_remaining_seconds()? as f64;
        Some((1.0 - (rem / cycle)).clamp(0.0, 1.0))
    }
}

fn spawn_quota_fetch(remote: String, tx: tokio::sync::mpsc::UnboundedSender<BgEvent>) {
    let rclone_conf_path = config::rclone_config_file();
    tokio::spawn(async move {
        if remote.trim().is_empty() {
            return;
        }
        let mut cmd = tokio::process::Command::new("rclone");
        cmd.args(["about", &remote, "--json"]);
        if rclone_conf_path.exists() {
            cmd.arg("--config").arg(&rclone_conf_path);
        }
        if let Ok(output) = cmd.output().await {
            if output.status.success() {
                if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                    let total = val.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
                    let used = val.get("used").and_then(|v| v.as_u64()).unwrap_or(0);
                    let free = val.get("free").and_then(|v| v.as_u64()).unwrap_or(0);
                    if total > 0 {
                        let _ = tx.send(BgEvent::QuotaFetched(CloudQuota {
                            total_bytes: total,
                            used_bytes: used,
                            free_bytes: free,
                        }));
                    }
                }
            }
        }
    });
}

/// Recursively counts non-hidden regular files using `entry.file_type()` (does NOT
/// follow symlinks, preventing infinite recursion on symlink cycles and avoiding
/// redundant `stat` syscalls on modern filesystems).
pub(crate) fn count_dir_files(p: &std::path::Path) -> usize {
    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(p) {
        for entry in entries.flatten() {
            let Ok(ft) = entry.file_type() else {
                continue;
            };
            if ft.is_dir() {
                if !entry.file_name().to_string_lossy().starts_with('.') {
                    count += count_dir_files(&entry.path());
                }
            } else if ft.is_file() {
                count += 1;
            }
        }
    }
    count
}

fn spawn_file_count(dir_path: std::path::PathBuf, tx: tokio::sync::mpsc::UnboundedSender<BgEvent>) {
    tokio::task::spawn_blocking(move || {
        let c = count_dir_files(&dir_path);
        let _ = tx.send(BgEvent::FileCounted(c));
    });
}

