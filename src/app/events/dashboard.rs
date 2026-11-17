use super::*;
use crate::ui::keys::{KeyAction, KeybindingRegistry};

impl App {
    pub(super) fn handle_key_dashboard(&mut self, key: &KeyEvent) -> Action {
        match key.code {
            // Enter on history: open selected run details
            // Enter on recent files: open file or folder (with Ctrl)
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
            // Esc or m opens main menu when no modal is open
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
                self.modal = Modal::DryRun;
                return Action::None;
            }
            KeyCode::Char('f') => {
                if self.focused_panel == FocusedPanel::Logs {
                    self.set_log_filter(self.log_filter.next());
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
            KeyCode::Char('1') => {
                self.toggle_box(1);
                return Action::None;
            }
            KeyCode::Char('2') => {
                self.toggle_box(2);
                return Action::None;
            }
            KeyCode::Char('3') => {
                self.toggle_box(3);
                return Action::None;
            }
            KeyCode::Char('4') => {
                self.toggle_box(4);
                return Action::None;
            }
            KeyCode::Char('5') => {
                self.toggle_box(5);
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
            KeyCode::Char(c)
                if KeybindingRegistry::get_key_str(KeyAction::ForceSync).starts_with(c) =>
            {
                self.modal = Modal::ConfirmSync;
                return Action::None;
            }
            KeyCode::Char(c)
                if KeybindingRegistry::get_key_str(KeyAction::Resync).starts_with(c) =>
            {
                self.modal = Modal::ConfirmResync;
                return Action::None;
            }
            KeyCode::Char('c') => {
                if self.is_syncing() {
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
            // Tab / Shift+Tab: cycle active panel
            KeyCode::Tab => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.prev_visible_panel();
                } else {
                    self.next_visible_panel();
                }
                return Action::None;
            }
            KeyCode::BackTab => {
                self.prev_visible_panel();
                return Action::None;
            }
            // +/- : dynamic rclone stats interval (- decreases, + increases)
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.step_stats_interval(true);
                return Action::None;
            }
            KeyCode::Char('-') | KeyCode::Char('_') => {
                self.step_stats_interval(false);
                return Action::None;
            }
            // Navigation directed by the active panel
            KeyCode::Up | KeyCode::Char('k') => match self.focused_panel {
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
            },
            KeyCode::Down | KeyCode::Char('j') => match self.focused_panel {
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
            },
            KeyCode::PageUp => match self.focused_panel {
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
            },
            KeyCode::PageDown => match self.focused_panel {
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
            },
            KeyCode::Home => match self.focused_panel {
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
            },
            KeyCode::End => match self.focused_panel {
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
            },
            KeyCode::Left if self.focused_panel == FocusedPanel::Logs => {
                self.set_log_filter(self.log_filter.prev());
                return Action::None;
            }
            KeyCode::Right if self.focused_panel == FocusedPanel::Logs => {
                self.set_log_filter(self.log_filter.next());
                return Action::None;
            }
            _ => {}
        }

        Action::None
    }
}
