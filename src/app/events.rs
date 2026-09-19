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
                if self.settings_tab == 0 {
                    if idx == 6 {
                        return Action::OpenFullLogs;
                    } else if idx == 5 {
                        self.modal = Modal::ConfirmResync;
                    } else if idx == 3 || idx == 4 {
                        if !self.is_editing_setting() || self.settings_selected_idx != idx {
                            self.start_editing_setting();
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
                if self.is_editing_setting() && idx != self.settings_selected_idx {
                    self.commit_setting_edit();
                }
                self.settings_selected_idx = idx;
                if self.settings_tab == 0 {
                    if idx == 6 {
                        return Action::OpenFullLogs;
                    } else if idx == 5 {
                        self.modal = Modal::ConfirmResync;
                    } else if idx == 3 || idx == 4 {
                        if !self.is_editing_setting() || self.settings_selected_idx != idx {
                            self.start_editing_setting();
                        }
                    } else {
                        self.cycle_setting(forward);
                    }
                } else {
                    self.cycle_setting(forward);
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
                self.log_filter = self.log_filter.prev();
                self.focused_panel = FocusedPanel::Logs;
                Action::None
            }
            HitAction::LogFilterNext | HitAction::LogFilterCycle => {
                self.log_filter = self.log_filter.next();
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
                        match key.code {
                            KeyCode::Esc | KeyCode::Enter => {
                                self.commit_setting_edit();
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
                                    self.start_editing_setting();
                                } else {
                                    self.cycle_setting(true);
                                }
                            } else {
                                self.cycle_setting(true);
                            }
                        }
                        KeyCode::Char('e') | KeyCode::Char(' ') => {
                            if self.settings_tab == 0 && (self.settings_selected_idx == 3 || self.settings_selected_idx == 4) {
                                self.start_editing_setting();
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
                                    self.start_editing_setting();
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
                                    self.start_editing_setting();
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
                Modal::None => {}
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
            // Tab / Shift+Tab: cycle active panel
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
