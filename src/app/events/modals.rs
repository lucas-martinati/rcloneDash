use super::*;

impl App {
    pub(super) fn handle_key_menu(&mut self, key: &KeyEvent) -> Action {
        match key.code {
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
            KeyCode::Enter | KeyCode::Char(' ') => match self.menu_selected_idx {
                0 => self.modal = Modal::Settings,
                1 => self.modal = Modal::Help,
                2 => self.running = false,
                _ => {}
            },
            KeyCode::Char('o') => self.modal = Modal::Settings,
            KeyCode::Char('?') | KeyCode::Char('h') => self.modal = Modal::Help,
            KeyCode::Char('q') => self.running = false,
            _ => {}
        }
        Action::None
    }

    /// Ferme la modale de confirmation et affiche le toast du résultat.
    fn confirm_result(&mut self, result: Result<(), String>, ok_msg: impl Into<String>) {
        self.modal = Modal::None;
        match result {
            Ok(_) => self.set_toast(ok_msg),
            Err(e) => self.set_toast(format!("✗ Error: {}", e)),
        }
    }

    pub(super) fn handle_key_confirm_sync(&mut self, key: &KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('o') | KeyCode::Enter => {
                self.confirm_result(systemd::trigger_sync(), "✔ Forced sync initiated...");
            }
            KeyCode::Char('n') | KeyCode::Esc | KeyCode::Char('q') => {
                self.modal = Modal::None;
            }
            _ => {}
        }
        Action::None
    }

    pub(super) fn handle_key_confirm_resync(&mut self, key: &KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('o') | KeyCode::Enter => {
                self.confirm_result(systemd::trigger_resync(), "✔ Full resynchronization initiated (--resync)!");
            }
            KeyCode::Char('n') | KeyCode::Esc | KeyCode::Char('q') => {
                self.modal = Modal::None;
            }
            _ => {}
        }
        Action::None
    }

    pub(super) fn handle_key_confirm_cancel(&mut self, key: &KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('o') | KeyCode::Enter => {
                self.confirm_result(systemd::cancel_sync(), "✔ Active synchronization aborted!");
            }
            KeyCode::Char('n') | KeyCode::Esc | KeyCode::Char('q') => {
                self.modal = Modal::None;
            }
            _ => {}
        }
        Action::None
    }

    pub(super) fn handle_key_confirm_delete(&mut self, rel: &str, key: &KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('o') | KeyCode::Enter => {
                let base = config::expand_tilde(&self.config.local_dir);
                let res = fs_tree::delete_entry(&base, rel);
                let ok = res.is_ok();
                self.confirm_result(res, format!("✔ {} deleted", rel));
                if ok {
                    self.reload_files();
                }
            }
            KeyCode::Char('n') | KeyCode::Esc | KeyCode::Char('q') => {
                self.modal = Modal::None;
            }
            _ => {}
        }
        Action::None
    }

    /// Active l'option sélectionnée : édition (texte), action ou cycle.
    /// `cycle_dir` donne le sens du cycle pour les réglages à choix ;
    /// None = activation simple (touche Enter).
    fn activate_selected_setting(&mut self, cycle_dir: Option<bool>) -> Action {
        if let Some(setting) = self.visible_setting_at(self.settings_tab, self.settings_selected_idx) {
            match setting.kind() {
                config::SettingKind::TextInput => {
                    self.start_editing_setting();
                }
                config::SettingKind::Action => {
                    if setting == config::SettingId::LogJournalAction {
                        return Action::OpenFullLogs;
                    } else if setting == config::SettingId::ResyncAction {
                        self.modal = Modal::ConfirmResync;
                    }
                }
                config::SettingKind::Cycle => {
                    self.cycle_setting(cycle_dir.unwrap_or(true));
                }
            }
        }
        Action::None
    }

    pub(super) fn handle_key_settings(&mut self, key: &KeyEvent) -> Action {
        if self.is_editing_setting() {
            if key.modifiers.contains(KeyModifiers::CONTROL)
                && (key.code == KeyCode::Char('v') || key.code == KeyCode::Char('V'))
            {
                if let Some(clip) = crate::clipboard::paste_from_clipboard() {
                    let clean = clip.trim().replace(['\r', '\n'], "");
                    if let EditState::Setting { buffer, cursor, .. } = &mut self.edit_state {
                        insert_str_at_cursor(buffer, cursor, &clean);
                    }
                }
                return Action::None;
            }
            match key.code {
                KeyCode::Esc => self.cancel_setting_edit(),
                KeyCode::Enter => self.commit_setting_edit(),
                KeyCode::Left => {
                    if let EditState::Setting { cursor, .. } = &mut self.edit_state {
                        *cursor = cursor.saturating_sub(1);
                    }
                }
                KeyCode::Right => {
                    if let EditState::Setting { buffer, cursor, .. } = &mut self.edit_state {
                        let count = buffer.chars().count();
                        if *cursor < count {
                            *cursor += 1;
                        }
                    }
                }
                KeyCode::Home => {
                    if let EditState::Setting { cursor, .. } = &mut self.edit_state {
                        *cursor = 0;
                    }
                }
                KeyCode::End => {
                    if let EditState::Setting { buffer, cursor, .. } = &mut self.edit_state {
                        *cursor = buffer.chars().count();
                    }
                }
                KeyCode::Backspace => {
                    if let EditState::Setting { buffer, cursor, .. } = &mut self.edit_state {
                        backspace_at_cursor(buffer, cursor);
                    }
                }
                KeyCode::Delete => {
                    if let EditState::Setting { buffer, cursor, .. } = &mut self.edit_state {
                        delete_at_cursor(buffer, *cursor);
                    }
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if let EditState::Setting { buffer, cursor, .. } = &mut self.edit_state {
                        insert_char_at_cursor(buffer, cursor, c);
                    }
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
                return self.activate_selected_setting(None);
            }
            KeyCode::Char('e') | KeyCode::Char(' ') => {
                if let Some(setting) = self.visible_setting_at(self.settings_tab, self.settings_selected_idx) {
                    if setting.is_text_input() {
                        self.start_editing_setting();
                    } else if setting.is_cycle() {
                        self.cycle_setting(true);
                    }
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                return self.activate_selected_setting(Some(true));
            }
            KeyCode::Left | KeyCode::Char('h') => {
                return self.activate_selected_setting(Some(false));
            }
            _ => {}
        }
        Action::None
    }

    pub(super) fn handle_key_files(&mut self, key: &KeyEvent) -> Action {
        match key.code {
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
        }
        Action::None
    }

    pub(super) fn handle_key_filters(&mut self, key: &KeyEvent) -> Action {
        if self.is_editing_filter() {
            match key.code {
                KeyCode::Enter => self.commit_filter_edit(),
                KeyCode::Esc => self.cancel_filter_edit(),
                KeyCode::Backspace => {
                    if let Some(buf) = self.edit_buffer_mut() {
                        buf.pop();
                    }
                }
                KeyCode::Char(c) => {
                    if let Some(buf) = self.edit_buffer_mut() {
                        buf.push(c);
                    }
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
            KeyCode::End if !self.filters.is_empty() => {
                self.selected_filter_idx = self.filters.len() - 1;
                self.ensure_filter_visible(vp);
            }
            _ => {}
        }
        Action::None
    }

    pub(super) fn handle_key_dry_run(&mut self, key: &KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.modal = Modal::None;
            }
            KeyCode::Char('r') | KeyCode::Enter => {
                if !self.dry_run_running {
                    self.start_dry_run();
                }
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
        }
        Action::None
    }

    pub(super) fn handle_key_help(&mut self, key: &KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') | KeyCode::Enter => {
                self.modal = Modal::None;
            }
            _ => {}
        }
        Action::None
    }

    pub(super) fn handle_key_history_details(&mut self, run_idx: usize, key: &KeyEvent) -> Action {
        let (has_errors, files_len) = self
            .past_runs
            .get(run_idx)
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
            KeyCode::Char('y') | KeyCode::Char('c') if has_errors || files_len > 0 => {
                self.copy_history_errors(run_idx);
            }
            _ => {}
        }
        Action::None
    }
}
