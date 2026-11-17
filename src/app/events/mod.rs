use super::*;

mod dashboard;
mod first_run;
mod modals;
mod mouse;

/// Inserts a single character at `cursor` (character index) in `buf` and advances `cursor`.
pub(super) fn insert_char_at_cursor(buf: &mut String, cursor: &mut usize, c: char) {
    let mut chars: Vec<char> = buf.chars().collect();
    let cur = (*cursor).min(chars.len());
    chars.insert(cur, c);
    *buf = chars.into_iter().collect();
    *cursor = cur + 1;
}

/// Inserts a sanitized string slice at `cursor` (character index) in `buf` and advances `cursor`.
pub(super) fn insert_str_at_cursor(buf: &mut String, cursor: &mut usize, text: &str) {
    let mut chars: Vec<char> = buf.chars().collect();
    let cur = (*cursor).min(chars.len());
    let clean_chars: Vec<char> = text.chars().collect();
    let clean_len = clean_chars.len();
    chars.splice(cur..cur, clean_chars);
    *buf = chars.into_iter().collect();
    *cursor = cur + clean_len;
}

/// Deletes the character immediately before `cursor` (Backspace) and decrements `cursor`.
pub(super) fn backspace_at_cursor(buf: &mut String, cursor: &mut usize) {
    if *cursor > 0 {
        let mut chars: Vec<char> = buf.chars().collect();
        if *cursor <= chars.len() {
            chars.remove(*cursor - 1);
            *buf = chars.into_iter().collect();
            *cursor -= 1;
        }
    }
}

/// Deletes the character under `cursor` (Delete key) without moving `cursor`.
pub(super) fn delete_at_cursor(buf: &mut String, cursor: usize) {
    let mut chars: Vec<char> = buf.chars().collect();
    if cursor < chars.len() {
        chars.remove(cursor);
        *buf = chars.into_iter().collect();
    }
}

impl App {
    pub fn dismiss_active_modal(&mut self) {
        match &self.edit_state {
            EditState::Setting { .. } => self.commit_setting_edit(),
            EditState::Filter { .. } | EditState::AddingFilter { .. } => self.cancel_filter_edit(),
            EditState::Idle => {}
        }
        self.modal = Modal::None;
        self.hit_mgr.active_modal_area = None;
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        // Universal shortcut Ctrl+X to toggle parent directory mode
        if is_ctrl_x(&key) {
            self.ctrl_mode = !self.ctrl_mode;
            if self.ctrl_mode {
                self.set_toast("📁 Folder mode active (folder paths shown)");
            } else {
                self.set_toast("📄 File mode active (file paths shown)");
            }
            return Action::None;
        }

        // Text input active for recent files filter
        if self.is_filtering_recent && self.modal == Modal::None {
            return self.handle_key_recent_filter(&key);
        }

        // 1. Priority modals
        if self.modal != Modal::None {
            if matches!(self.modal, Modal::FirstRun(_)) {
                return self.handle_key_first_run(&key);
            }

            // Universal 'q' key to close any modal (except Menu where 'q' quits the application)
            if key.code == KeyCode::Char('q') && self.modal != Modal::Menu && self.edit_state == EditState::Idle {
                self.modal = Modal::None;
                return Action::None;
            }

            return match &self.modal {
                Modal::Menu => self.handle_key_menu(&key),
                Modal::ConfirmSync => self.handle_key_confirm_sync(&key),
                Modal::ConfirmResync => self.handle_key_confirm_resync(&key),
                Modal::ConfirmCancel => self.handle_key_confirm_cancel(&key),
                Modal::ConfirmDelete(rel) => {
                    let rel = rel.clone();
                    self.handle_key_confirm_delete(&rel, &key)
                }
                Modal::Settings => self.handle_key_settings(&key),
                Modal::Files => self.handle_key_files(&key),
                Modal::Filters => self.handle_key_filters(&key),
                Modal::DryRun => self.handle_key_dry_run(&key),
                Modal::Help => self.handle_key_help(&key),
                Modal::HistoryDetails(run_idx) => {
                    let idx = *run_idx;
                    self.handle_key_history_details(idx, &key)
                }
                Modal::FirstRun(_) | Modal::None => Action::None,
            };
        }

        // 2. Global Dashboard shortcuts
        self.handle_key_dashboard(&key)
    }

    fn handle_key_recent_filter(&mut self, key: &KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => {
                self.recent_filter.clear();
                self.is_filtering_recent = false;
                self.recent_selected_idx = None;
                self.recent_scroll_offset = 0;
            }
            KeyCode::Enter => {
                self.is_filtering_recent = false;
            }
            KeyCode::Backspace => {
                self.recent_filter.pop();
                self.recent_selected_idx = if self.get_recent_files_list().is_empty() { None } else { Some(0) };
                self.recent_scroll_offset = 0;
            }
            KeyCode::Char(c) => {
                self.recent_filter.push(c);
                self.recent_selected_idx = if self.get_recent_files_list().is_empty() { None } else { Some(0) };
                self.recent_scroll_offset = 0;
            }
            _ => {}
        }
        Action::None
    }
}
