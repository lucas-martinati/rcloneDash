use super::*;

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

    pub fn handle_mouse(&mut self, mouse: MouseEvent) -> Action {
        match mouse.kind {
            MouseEventKind::ScrollDown => {
                // 1. If a modal is active: scroll is confined to it and aligned
                if self.modal != Modal::None {
                    match self.modal {
                        Modal::Filters => {
                            let vp = self.filter_viewport_height;
                            self.scroll_filter_view_down(vp);
                        }
                        Modal::Settings => {
                            self.scroll_settings_down();
                        }
                        Modal::Files => {
                            let vp = self.file_viewport_height;
                            self.scroll_file_view_down(vp);
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

                // 2. Dashboard mode: scroll on container under cursor
                let col = mouse.column;
                let row = mouse.row;
                let mut handled = false;
                for hb in self.hit_mgr.dashboard.iter().rev() {
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
                                self.scroll_recent_view_down(vp);
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
                // 1. If a modal is active: scroll is confined to it and aligned
                if self.modal != Modal::None {
                    match self.modal {
                        Modal::Filters => {
                            let vp = self.filter_viewport_height;
                            self.scroll_filter_view_up(vp);
                        }
                        Modal::Settings => {
                            self.scroll_settings_up();
                        }
                        Modal::Files => {
                            let vp = self.file_viewport_height;
                            self.scroll_file_view_up(vp);
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

                // 2. Dashboard mode: scroll on container under cursor
                let col = mouse.column;
                let row = mouse.row;
                let mut handled = false;
                for hb in self.hit_mgr.dashboard.iter().rev() {
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
                                self.scroll_recent_view_up(vp);
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

                // 1. If a modal is open: unified click-outside handling
                if self.modal != Modal::None {
                    if let Some(modal_rect) = self.hit_mgr.active_modal_area {
                        let inside = col >= modal_rect.x
                            && col < modal_rect.x + modal_rect.width
                            && row >= modal_rect.y
                            && row < modal_rect.y + modal_rect.height;

                        if !inside {
                            // Click outside active panel: close modal
                            self.dismiss_active_modal();
                            return Action::None;
                        }
                    }

                    let hitboxes_to_search = self.hit_mgr.active_layer(true);
                    for hb in hitboxes_to_search.iter().rev() {
                        if hb.rect.x <= col && col < hb.rect.x + hb.rect.width
                            && hb.rect.y <= row && row < hb.rect.y + hb.rect.height
                        {
                            return self.execute_hit_action(hb.action, is_ctrl, row);
                        }
                    }
                    // Click inside modal but not on a hitbox: absorb without leaking
                    return Action::None;
                }

                // 2. Standard dashboard mode (no active modal)
                for hb in self.hit_mgr.dashboard.iter().rev() {
                    if hb.rect.x <= col && col < hb.rect.x + hb.rect.width
                        && hb.rect.y <= row && row < hb.rect.y + hb.rect.height
                    {
                        return self.execute_hit_action(hb.action, is_ctrl, row);
                    }
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                let row = mouse.row;
                if let Some((target, top_y, track_height, total, visible, grab_offset)) = self.active_scrollbar_drag {
                    self.apply_scrollbar_drag_to_row(target, row, top_y, track_height, total, visible, grab_offset);
                    return Action::None;
                }
                let col = mouse.column;
                let has_modal = self.modal != Modal::None;
                let hitboxes_to_search = self.hit_mgr.active_layer(has_modal);
                for hb in hitboxes_to_search.iter().rev() {
                    if hb.rect.x <= col && col < hb.rect.x + hb.rect.width
                        && hb.rect.y <= row && row < hb.rect.y + hb.rect.height
                    {
                        if let HitAction::ScrollbarTrack { target, top_y, track_height, total, visible } = hb.action {
                            let grab_offset = self.compute_scrollbar_grab_offset(target, row, top_y, track_height, total, visible);
                            self.active_scrollbar_drag = Some((target, top_y, track_height, total, visible, grab_offset));
                            self.apply_scrollbar_drag_to_row(target, row, top_y, track_height, total, visible, grab_offset);
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
            HitAction::ButtonPanel => {
                self.next_visible_panel();
                Action::None
            }
            HitAction::ToggleBox(num) => {
                self.toggle_box(num);
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
                if self.is_editing_setting() {
                    self.commit_setting_edit();
                }
                self.settings_tab = tab_idx;
                self.settings_selected_idx = 0;
                Action::None
            }
            HitAction::SettingOption(idx) => {
                if self.is_editing_setting() && idx != self.settings_selected_idx {
                    self.commit_setting_edit();
                }
                self.settings_selected_idx = idx;
                if let Some(setting) = config::SettingId::from_tab_and_idx(self.settings_tab, idx) {
                    match setting.kind() {
                        config::SettingKind::TextInput => {
                            if !self.is_editing_setting() || self.settings_selected_idx != idx {
                                self.start_editing_setting();
                            }
                        }
                        config::SettingKind::Action => {
                            if setting == config::SettingId::LogJournalAction {
                                return Action::OpenFullLogs;
                            } else if setting == config::SettingId::ResyncAction {
                                self.modal = Modal::ConfirmResync;
                            }
                        }
                        config::SettingKind::Cycle => {
                            self.cycle_setting(true);
                        }
                    }
                }
                Action::None
            }
            HitAction::SettingCycle(idx, forward) => {
                if self.is_editing_setting() && idx != self.settings_selected_idx {
                    self.commit_setting_edit();
                }
                self.settings_selected_idx = idx;
                if let Some(setting) = config::SettingId::from_tab_and_idx(self.settings_tab, idx) {
                    match setting.kind() {
                        config::SettingKind::TextInput => {
                            if !self.is_editing_setting() || self.settings_selected_idx != idx {
                                self.start_editing_setting();
                            }
                        }
                        config::SettingKind::Action => {
                            if setting == config::SettingId::LogJournalAction {
                                return Action::OpenFullLogs;
                            } else if setting == config::SettingId::ResyncAction {
                                self.modal = Modal::ConfirmResync;
                            }
                        }
                        config::SettingKind::Cycle => {
                            self.cycle_setting(forward);
                        }
                    }
                }
                Action::None
            }
            HitAction::FirstRunInstall => {
                self.start_rclone_install();
                Action::None
            }
            HitAction::FirstRunContinue | HitAction::FirstRunSkipRclone => {
                if let Modal::FirstRun(ref mut state) = self.modal {
                    state.step = FirstRunStep::GoogleCredentials;
                    state.active_field = FirstRunField::ClientIdInput;
                }
                Action::None
            }
            HitAction::FirstRunClientId => {
                if let Modal::FirstRun(ref mut state) = self.modal {
                    state.active_field = FirstRunField::ClientIdInput;
                    state.client_id_cursor = state.client_id.chars().count();
                }
                Action::None
            }
            HitAction::FirstRunClientSecret => {
                if let Modal::FirstRun(ref mut state) = self.modal {
                    state.active_field = FirstRunField::ClientSecretInput;
                    state.client_secret_cursor = state.client_secret.chars().count();
                }
                Action::None
            }
            HitAction::FirstRunSaveCredentials => {
                if let Modal::FirstRun(ref state) = self.modal {
                    let _ = config::write_rclone_credentials(&self.config.remote, &state.client_id, &state.client_secret);
                    self.config.first_run_completed = Some(true);
                    let _ = config::save_config(&self.config);
                    self.set_toast("✔ Google Drive credentials saved! Setup complete.");
                }
                self.modal = Modal::None;
                self.hit_mgr.active_modal_area = None;
                Action::None
            }
            HitAction::FirstRunSkipCredentials => {
                self.config.first_run_completed = Some(true);
                let _ = config::save_config(&self.config);
                self.set_toast("✔ Setup skipped. You can configure credentials anytime in Settings.");
                self.modal = Modal::None;
                self.hit_mgr.active_modal_area = None;
                Action::None
            }
            HitAction::FirstRunToggleHelp => {
                if let Modal::FirstRun(ref mut state) = self.modal {
                    state.show_help = !state.show_help;
                }
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
            HitAction::ToggleLogsAuto => {
                self.auto_scroll = !self.auto_scroll;
                Action::None
            }
            HitAction::FilterRow(idx) => {
                if idx < self.filters.len() {
                    if self.is_editing_filter() && self.selected_filter_idx != idx {
                        self.commit_filter_edit();
                    }
                    self.selected_filter_idx = idx;
                    let vp = self.filter_viewport_height;
                    self.ensure_filter_visible(vp);
                }
                Action::None
            }
            HitAction::FilterCycleType(idx) => {
                if !self.is_editing_filter() {
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
            HitAction::LogFilterPrev => {
                self.set_log_filter(self.log_filter.prev());
                self.focused_panel = FocusedPanel::Logs;
                Action::None
            }
            HitAction::LogFilterNext | HitAction::LogFilterCycle => {
                self.set_log_filter(self.log_filter.next());
                self.focused_panel = FocusedPanel::Logs;
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
                let grab_offset = self.compute_scrollbar_grab_offset(target, row, top_y, track_height, total, visible);
                self.active_scrollbar_drag = Some((target, top_y, track_height, total, visible, grab_offset));
                self.apply_scrollbar_drag_to_row(target, row, top_y, track_height, total, visible, grab_offset);
                Action::None
            }
        }
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

        // 1. Priority modals
        if self.modal != Modal::None {
            // FirstRun modal keyboard handling
            if let Modal::FirstRun(ref mut state) = self.modal {
                enum FirstRunKeyAction {
                    None,
                    InstallRclone,
                    SaveCredentials(String, String),
                    SkipCredentials,
                }

                let mut action_to_do = FirstRunKeyAction::None;

                match state.step {
                    FirstRunStep::RcloneCheck => match key.code {
                        KeyCode::Enter => {
                            match state.rclone_status {
                                RcloneInstallStatus::Installed(_) => {
                                    state.step = FirstRunStep::GoogleCredentials;
                                    state.active_field = FirstRunField::ClientIdInput;
                                }
                                RcloneInstallStatus::NotInstalled | RcloneInstallStatus::Failed(_) => {
                                    if state.active_field == FirstRunField::SkipRcloneButton {
                                        state.step = FirstRunStep::GoogleCredentials;
                                        state.active_field = FirstRunField::ClientIdInput;
                                    } else {
                                        action_to_do = FirstRunKeyAction::InstallRclone;
                                    }
                                }
                                RcloneInstallStatus::Installing => {}
                            }
                        }
                        KeyCode::Tab | KeyCode::Right | KeyCode::Down => {
                            if !matches!(state.rclone_status, RcloneInstallStatus::Installed(_) | RcloneInstallStatus::Installing) {
                                state.active_field = match state.active_field {
                                    FirstRunField::InstallRcloneButton => FirstRunField::SkipRcloneButton,
                                    _ => FirstRunField::InstallRcloneButton,
                                };
                            }
                        }
                        KeyCode::BackTab | KeyCode::Left | KeyCode::Up => {
                            if !matches!(state.rclone_status, RcloneInstallStatus::Installed(_) | RcloneInstallStatus::Installing) {
                                state.active_field = match state.active_field {
                                    FirstRunField::SkipRcloneButton => FirstRunField::InstallRcloneButton,
                                    _ => FirstRunField::SkipRcloneButton,
                                };
                            }
                        }
                        KeyCode::Esc => {
                            state.step = FirstRunStep::GoogleCredentials;
                            state.active_field = FirstRunField::ClientIdInput;
                        }
                        _ => {}
                    },
                    FirstRunStep::GoogleCredentials => match key.code {
                        KeyCode::Tab => {
                            state.active_field = match state.active_field {
                                FirstRunField::ClientIdInput => FirstRunField::ClientSecretInput,
                                FirstRunField::ClientSecretInput => FirstRunField::SaveCredentialsButton,
                                FirstRunField::SaveCredentialsButton => FirstRunField::SkipCredentialsButton,
                                FirstRunField::SkipCredentialsButton => FirstRunField::ToggleHelpButton,
                                FirstRunField::ToggleHelpButton => FirstRunField::ClientIdInput,
                                _ => FirstRunField::ClientIdInput,
                            };
                        }
                        KeyCode::BackTab => {
                            state.active_field = match state.active_field {
                                FirstRunField::ClientIdInput => FirstRunField::ToggleHelpButton,
                                FirstRunField::ClientSecretInput => FirstRunField::ClientIdInput,
                                FirstRunField::SaveCredentialsButton => FirstRunField::ClientSecretInput,
                                FirstRunField::SkipCredentialsButton => FirstRunField::SaveCredentialsButton,
                                FirstRunField::ToggleHelpButton => FirstRunField::SkipCredentialsButton,
                                _ => FirstRunField::ClientIdInput,
                            };
                        }
                        KeyCode::Down => {
                            state.active_field = match state.active_field {
                                FirstRunField::ClientIdInput => FirstRunField::ClientSecretInput,
                                FirstRunField::ClientSecretInput => FirstRunField::SaveCredentialsButton,
                                FirstRunField::SaveCredentialsButton
                                | FirstRunField::SkipCredentialsButton
                                | FirstRunField::ToggleHelpButton => FirstRunField::ClientIdInput,
                                _ => FirstRunField::ClientIdInput,
                            };
                        }
                        KeyCode::Up => {
                            state.active_field = match state.active_field {
                                FirstRunField::ClientIdInput => FirstRunField::SaveCredentialsButton,
                                FirstRunField::ClientSecretInput => FirstRunField::ClientIdInput,
                                FirstRunField::SaveCredentialsButton
                                | FirstRunField::SkipCredentialsButton
                                | FirstRunField::ToggleHelpButton => FirstRunField::ClientSecretInput,
                                _ => FirstRunField::ClientIdInput,
                            };
                        }
                        KeyCode::Enter => {
                            match state.active_field {
                                FirstRunField::ClientIdInput => {
                                    state.active_field = FirstRunField::ClientSecretInput;
                                }
                                FirstRunField::ClientSecretInput => {
                                    state.active_field = FirstRunField::SaveCredentialsButton;
                                }
                                FirstRunField::SaveCredentialsButton => {
                                    action_to_do = FirstRunKeyAction::SaveCredentials(state.client_id.clone(), state.client_secret.clone());
                                }
                                FirstRunField::SkipCredentialsButton => {
                                    action_to_do = FirstRunKeyAction::SkipCredentials;
                                }
                                FirstRunField::ToggleHelpButton => {
                                    state.show_help = !state.show_help;
                                }
                                _ => {}
                            }
                        }
                        KeyCode::Esc => {
                            action_to_do = FirstRunKeyAction::SkipCredentials;
                        }
                        KeyCode::Char('?') | KeyCode::Char('h') if state.active_field != FirstRunField::ClientIdInput && state.active_field != FirstRunField::ClientSecretInput => {
                            state.show_help = !state.show_help;
                        }
                        KeyCode::Left => {
                            match state.active_field {
                                FirstRunField::ClientIdInput => {
                                    if state.client_id_cursor > 0 {
                                        state.client_id_cursor -= 1;
                                    }
                                }
                                FirstRunField::ClientSecretInput => {
                                    if state.client_secret_cursor > 0 {
                                        state.client_secret_cursor -= 1;
                                    }
                                }
                                FirstRunField::SaveCredentialsButton => {
                                    state.active_field = FirstRunField::ToggleHelpButton;
                                }
                                FirstRunField::SkipCredentialsButton => {
                                    state.active_field = FirstRunField::SaveCredentialsButton;
                                }
                                FirstRunField::ToggleHelpButton => {
                                    state.active_field = FirstRunField::SkipCredentialsButton;
                                }
                                _ => {}
                            }
                        }
                        KeyCode::Right => {
                            match state.active_field {
                                FirstRunField::ClientIdInput => {
                                    if state.client_id_cursor < state.client_id.chars().count() {
                                        state.client_id_cursor += 1;
                                    }
                                }
                                FirstRunField::ClientSecretInput => {
                                    if state.client_secret_cursor < state.client_secret.chars().count() {
                                        state.client_secret_cursor += 1;
                                    }
                                }
                                FirstRunField::SaveCredentialsButton => {
                                    state.active_field = FirstRunField::SkipCredentialsButton;
                                }
                                FirstRunField::SkipCredentialsButton => {
                                    state.active_field = FirstRunField::ToggleHelpButton;
                                }
                                FirstRunField::ToggleHelpButton => {
                                    state.active_field = FirstRunField::SaveCredentialsButton;
                                }
                                _ => {}
                            }
                        }
                        KeyCode::Home if state.active_field == FirstRunField::ClientIdInput => {
                            state.client_id_cursor = 0;
                        }
                        KeyCode::Home if state.active_field == FirstRunField::ClientSecretInput => {
                            state.client_secret_cursor = 0;
                        }
                        KeyCode::End if state.active_field == FirstRunField::ClientIdInput => {
                            state.client_id_cursor = state.client_id.chars().count();
                        }
                        KeyCode::End if state.active_field == FirstRunField::ClientSecretInput => {
                            state.client_secret_cursor = state.client_secret.chars().count();
                        }
                        KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            if let Some(pasted) = crate::clipboard::paste_from_clipboard() {
                                let clean = pasted.trim().replace(['\n', '\r'], "");
                                let clean_chars: Vec<char> = clean.chars().collect();
                                let clean_len = clean_chars.len();
                                match state.active_field {
                                    FirstRunField::ClientIdInput => {
                                        let mut chars: Vec<char> = state.client_id.chars().collect();
                                        let cur = state.client_id_cursor.min(chars.len());
                                        chars.splice(cur..cur, clean_chars);
                                        state.client_id = chars.into_iter().collect();
                                        state.client_id_cursor = cur + clean_len;
                                    }
                                    FirstRunField::ClientSecretInput => {
                                        let mut chars: Vec<char> = state.client_secret.chars().collect();
                                        let cur = state.client_secret_cursor.min(chars.len());
                                        chars.splice(cur..cur, clean_chars);
                                        state.client_secret = chars.into_iter().collect();
                                        state.client_secret_cursor = cur + clean_len;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        KeyCode::Backspace => {
                            match state.active_field {
                                FirstRunField::ClientIdInput => {
                                    if state.client_id_cursor > 0 {
                                        let mut chars: Vec<char> = state.client_id.chars().collect();
                                        if state.client_id_cursor <= chars.len() {
                                            chars.remove(state.client_id_cursor - 1);
                                            state.client_id = chars.into_iter().collect();
                                            state.client_id_cursor -= 1;
                                        }
                                    }
                                }
                                FirstRunField::ClientSecretInput => {
                                    if state.client_secret_cursor > 0 {
                                        let mut chars: Vec<char> = state.client_secret.chars().collect();
                                        if state.client_secret_cursor <= chars.len() {
                                            chars.remove(state.client_secret_cursor - 1);
                                            state.client_secret = chars.into_iter().collect();
                                            state.client_secret_cursor -= 1;
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        KeyCode::Delete => {
                            match state.active_field {
                                FirstRunField::ClientIdInput => {
                                    let mut chars: Vec<char> = state.client_id.chars().collect();
                                    if state.client_id_cursor < chars.len() {
                                        chars.remove(state.client_id_cursor);
                                        state.client_id = chars.into_iter().collect();
                                    }
                                }
                                FirstRunField::ClientSecretInput => {
                                    let mut chars: Vec<char> = state.client_secret.chars().collect();
                                    if state.client_secret_cursor < chars.len() {
                                        chars.remove(state.client_secret_cursor);
                                        state.client_secret = chars.into_iter().collect();
                                    }
                                }
                                _ => {}
                            }
                        }
                        KeyCode::Char(c) => {
                            match state.active_field {
                                FirstRunField::ClientIdInput => {
                                    let mut chars: Vec<char> = state.client_id.chars().collect();
                                    let cur = state.client_id_cursor.min(chars.len());
                                    chars.insert(cur, c);
                                    state.client_id = chars.into_iter().collect();
                                    state.client_id_cursor = cur + 1;
                                }
                                FirstRunField::ClientSecretInput => {
                                    let mut chars: Vec<char> = state.client_secret.chars().collect();
                                    let cur = state.client_secret_cursor.min(chars.len());
                                    chars.insert(cur, c);
                                    state.client_secret = chars.into_iter().collect();
                                    state.client_secret_cursor = cur + 1;
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    },
                }

                match action_to_do {
                    FirstRunKeyAction::InstallRclone => {
                        self.start_rclone_install();
                    }
                    FirstRunKeyAction::SaveCredentials(id, sec) => {
                        let _ = config::write_rclone_credentials(&self.config.remote, &id, &sec);
                        self.config.first_run_completed = Some(true);
                        let _ = config::save_config(&self.config);
                        self.modal = Modal::None;
                        self.hit_mgr.active_modal_area = None;
                        self.set_toast("✔ Google Drive credentials saved! Setup complete.");
                    }
                    FirstRunKeyAction::SkipCredentials => {
                        self.config.first_run_completed = Some(true);
                        let _ = config::save_config(&self.config);
                        self.modal = Modal::None;
                        self.hit_mgr.active_modal_area = None;
                        self.set_toast("✔ Setup skipped. You can configure credentials anytime in Settings.");
                    }
                    FirstRunKeyAction::None => {}
                }

                return Action::None;
            }

            // Universal 'q' key to close any modal (except Menu where 'q' quits the application)
            if key.code == KeyCode::Char('q') && self.modal != Modal::Menu && self.edit_state == EditState::Idle {
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
                    if self.is_editing_setting() {
                        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('v') {
                            if let Some(clip) = crate::clipboard::paste_from_clipboard() {
                                let clean = clip.trim().replace(['\r', '\n'], "");
                                if let EditState::Setting { buffer, cursor, .. } = &mut self.edit_state {
                                    let mut chars: Vec<char> = buffer.chars().collect();
                                    let cur = (*cursor).min(chars.len());
                                    let clean_chars: Vec<char> = clean.chars().collect();
                                    let clean_len = clean_chars.len();
                                    chars.splice(cur..cur, clean_chars);
                                    *buffer = chars.into_iter().collect();
                                    *cursor = cur + clean_len;
                                }
                            }
                            return Action::None;
                        }
                        match key.code {
                            KeyCode::Esc => {
                                self.cancel_setting_edit();
                            }
                            KeyCode::Enter => {
                                self.commit_setting_edit();
                            }
                            KeyCode::Left => {
                                if let EditState::Setting { cursor, .. } = &mut self.edit_state {
                                    if *cursor > 0 {
                                        *cursor -= 1;
                                    }
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
                                    if *cursor > 0 {
                                        let mut chars: Vec<char> = buffer.chars().collect();
                                        if *cursor <= chars.len() {
                                            chars.remove(*cursor - 1);
                                            *buffer = chars.into_iter().collect();
                                            *cursor -= 1;
                                        }
                                    }
                                }
                            }
                            KeyCode::Delete => {
                                if let EditState::Setting { buffer, cursor, .. } = &mut self.edit_state {
                                    let mut chars: Vec<char> = buffer.chars().collect();
                                    if *cursor < chars.len() {
                                        chars.remove(*cursor);
                                        *buffer = chars.into_iter().collect();
                                    }
                                }
                            }
                            KeyCode::Char(c) => {
                                if !key.modifiers.contains(KeyModifiers::CONTROL) {
                                    if let EditState::Setting { buffer, cursor, .. } = &mut self.edit_state {
                                        let mut chars: Vec<char> = buffer.chars().collect();
                                        let cur = (*cursor).min(chars.len());
                                        chars.insert(cur, c);
                                        *buffer = chars.into_iter().collect();
                                        *cursor = cur + 1;
                                    }
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
                            if let Some(setting) = config::SettingId::from_tab_and_idx(self.settings_tab, self.settings_selected_idx) {
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
                                        self.cycle_setting(true);
                                    }
                                }
                            }
                        }
                        KeyCode::Char('e') | KeyCode::Char(' ') => {
                            if let Some(setting) = config::SettingId::from_tab_and_idx(self.settings_tab, self.settings_selected_idx) {
                                if setting.is_text_input() {
                                    self.start_editing_setting();
                                } else if setting.is_cycle() {
                                    self.cycle_setting(true);
                                }
                            }
                        }
                        KeyCode::Right | KeyCode::Char('l') => {
                            if let Some(setting) = config::SettingId::from_tab_and_idx(self.settings_tab, self.settings_selected_idx) {
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
                                        self.cycle_setting(true);
                                    }
                                }
                            }
                        }
                        KeyCode::Left | KeyCode::Char('h') => {
                            if let Some(setting) = config::SettingId::from_tab_and_idx(self.settings_tab, self.settings_selected_idx) {
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
                                        self.cycle_setting(false);
                                    }
                                }
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
                    if self.is_editing_filter() {
                        match key.code {
                            KeyCode::Enter => {
                                self.commit_filter_edit();
                            }
                            KeyCode::Esc => {
                                self.cancel_filter_edit();
                            }
                            KeyCode::Backspace => {
                                if let Some(buf) = self.edit_buffer_mut() { buf.pop(); }
                            }
                            KeyCode::Char(c) => {
                                if let Some(buf) = self.edit_buffer_mut() { buf.push(c); }
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
                Modal::FirstRun(_) | Modal::None => {}
            }
            return Action::None;
        }

        // 2. Global Dashboard shortcuts
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
                self.modal = Modal::ConfirmDryRun;
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
            // +/- : dynamic tick rate (- speeds up, + slows down)
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.step_tick_rate(false); // slows down
                return Action::None;
            }
            KeyCode::Char('-') | KeyCode::Char('_') => {
                self.step_tick_rate(true); // speeds up
                return Action::None;
            }
            // Navigation directed by the active panel
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
                    self.set_log_filter(self.log_filter.prev());
                    return Action::None;
                }
            }
            KeyCode::Right => {
                if self.focused_panel == FocusedPanel::Logs {
                    self.set_log_filter(self.log_filter.next());
                    return Action::None;
                }
            }
            _ => {}
        }

        Action::None
    }

}
