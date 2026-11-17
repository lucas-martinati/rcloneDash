use super::*;

impl App {
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
                if let Some(drag) = self.active_scrollbar_drag {
                    self.apply_scrollbar_drag_to_row(drag, row);
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
                            let drag = ScrollbarDragState { target, top_y, track_height, total, visible, grab_offset };
                            self.active_scrollbar_drag = Some(drag);
                            self.apply_scrollbar_drag_to_row(drag, row);
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
                self.modal = Modal::DryRun;
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
            HitAction::StatsIntervalDec => {
                self.step_stats_interval(false);
                Action::None
            }
            HitAction::StatsIntervalInc => {
                self.step_stats_interval(true);
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
                if let Some(setting) = self.visible_setting_at(self.settings_tab, idx) {
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
                if let Some(setting) = self.visible_setting_at(self.settings_tab, idx) {
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
                    state.step = FirstRunStep::RemoteSetup;
                    state.active_field = FirstRunField::RemoteInput;
                    state.remote_cursor = state.remote_input.chars().count();
                }
                Action::None
            }
            HitAction::FirstRunRemoteInput => {
                if let Modal::FirstRun(ref mut state) = self.modal {
                    state.active_field = FirstRunField::RemoteInput;
                    state.remote_cursor = state.remote_input.chars().count();
                }
                Action::None
            }
            HitAction::FirstRunRemoteContinue => {
                self.advance_from_remote_continue();
                Action::None
            }
            HitAction::FirstRunRemoteSkip => {
                self.config.first_run_completed = Some(true);
                let _ = config::save_config(&self.config);
                self.set_toast("✔ Setup skipped. You can configure the remote anytime in Settings.");
                self.modal = Modal::None;
                self.hit_mgr.active_modal_area = None;
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
                    let (remote, id, sec) = (
                        state.remote_input.clone(),
                        state.client_id.clone(),
                        state.client_secret.clone(),
                    );
                    self.save_first_run_credentials(&remote, &id, &sec);
                } else {
                    self.modal = Modal::None;
                    self.hit_mgr.active_modal_area = None;
                }
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
                let drag = ScrollbarDragState { target, top_y, track_height, total, visible, grab_offset };
                self.active_scrollbar_drag = Some(drag);
                self.apply_scrollbar_drag_to_row(drag, row);
                Action::None
            }
        }
    }
}
