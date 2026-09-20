use super::*;

impl App {
    pub fn settings_items_count(&self) -> usize {
        config::SettingId::tab_count(self.settings_tab)
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

    pub fn commit_setting_edit(&mut self) {
        let (tab, idx, trimmed) = match &self.edit_state {
            EditState::Setting { tab, index, buffer, .. } => (*tab, *index, buffer.trim().to_string()),
            _ => return,
        };

        let setting = match config::SettingId::from_tab_and_idx(tab, idx) {
            Some(s) => s,
            None => {
                self.edit_state = EditState::Idle;
                return;
            }
        };

        match setting {
            config::SettingId::LocalDirectory => {
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
            }
            config::SettingId::RemoteStorage => {
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
            config::SettingId::GoogleClientId => {
                let (old_id, old_secret) = config::read_rclone_credentials(&self.config.remote);
                let secret = old_secret.unwrap_or_default();
                if old_id.as_deref() != Some(&trimmed) {
                    if let Err(e) = config::write_rclone_credentials(&self.config.remote, &trimmed, &secret) {
                        self.set_toast(format!("✗ Failed to update Client ID: {}", e));
                    } else {
                        self.set_toast("✔ Google Client ID updated in rclone.conf!");
                    }
                } else {
                    self.set_toast("✔ Google Client ID unchanged");
                }
            }
            config::SettingId::GoogleClientSecret => {
                let (old_id, old_secret) = config::read_rclone_credentials(&self.config.remote);
                let id = old_id.unwrap_or_default();
                if old_secret.as_deref() != Some(&trimmed) {
                    if let Err(e) = config::write_rclone_credentials(&self.config.remote, &id, &trimmed) {
                        self.set_toast(format!("✗ Failed to update Client Secret: {}", e));
                    } else {
                        self.set_toast("✔ Google Client Secret updated in rclone.conf!");
                    }
                } else {
                    self.set_toast("✔ Google Client Secret unchanged");
                }
            }
            _ => {}
        }
        self.edit_state = EditState::Idle;
    }

    pub fn cancel_setting_edit(&mut self) {
        self.edit_state = EditState::Idle;
        self.set_toast("ℹ Edit cancelled");
    }

    /// Enter editing mode for the currently selected setting (text fields only).
    pub fn start_editing_setting(&mut self) {
        let setting = match config::SettingId::from_tab_and_idx(self.settings_tab, self.settings_selected_idx) {
            Some(s) if s.is_text_input() => s,
            _ => return,
        };

        let buffer = match setting {
            config::SettingId::LocalDirectory => self.config.local_dir.clone(),
            config::SettingId::RemoteStorage => self.config.remote.clone(),
            config::SettingId::GoogleClientId => {
                config::read_rclone_credentials(&self.config.remote).0.unwrap_or_default()
            }
            config::SettingId::GoogleClientSecret => {
                config::read_rclone_credentials(&self.config.remote).1.unwrap_or_default()
            }
            _ => String::new(),
        };

        let cursor = buffer.chars().count();
        self.edit_state = EditState::Setting {
            tab: self.settings_tab,
            index: self.settings_selected_idx,
            buffer,
            cursor,
        };
    }

    pub fn setting_value(&self, setting: config::SettingId) -> String {
        match setting {
            config::SettingId::TimerInterval => {
                if self.config.timer_interval == "never" {
                    "Never (Paused)".to_string()
                } else {
                    self.config.timer_interval.clone()
                }
            }
            config::SettingId::CloudSafetyNet => config::full_sync_label(&self.config.full_sync_interval).to_string(),
            config::SettingId::BandwidthLimit => self.config.bwlimit.as_deref().unwrap_or("Disabled").to_string(),
            config::SettingId::StatsInterval => self.config.stats_interval.clone(),
            config::SettingId::LocalDirectory => self.config.local_dir.clone(),
            config::SettingId::RemoteStorage => self.config.remote.clone(),
            config::SettingId::ResyncAction => "Run (--resync)".to_string(),
            config::SettingId::LogJournalAction => "Open logs".to_string(),
            config::SettingId::GoogleClientId => {
                let (id, _) = config::read_rclone_credentials(&self.config.remote);
                match id {
                    Some(id) if !id.is_empty() => {
                        let prefix: String = id.chars().take(16).collect();
                        format!("{}…", prefix)
                    }
                    _ => "(not set)".to_string(),
                }
            }
            config::SettingId::GoogleClientSecret => {
                let (_, secret) = config::read_rclone_credentials(&self.config.remote);
                match secret {
                    Some(s) if !s.is_empty() => "••••••••••••".to_string(),
                    _ => "(not set)".to_string(),
                }
            }
            config::SettingId::ColorTheme => self.current_theme.name().to_string(),
            config::SettingId::ContainerLayout => self.config.container_layout.name().to_string(),
            config::SettingId::MidPanelOrder => self.config.mid_panel_order.name().to_string(),
            config::SettingId::BorderStyle => self.config.border_style.name().to_string(),
            config::SettingId::GraphStyle => self.config.graph_style.name().to_string(),
        }
    }

    pub fn cycle_setting(&mut self, forward: bool) {
        let setting = match config::SettingId::from_tab_and_idx(self.settings_tab, self.settings_selected_idx) {
            Some(s) => s,
            None => return,
        };

        match setting {
            config::SettingId::TimerInterval => {
                let options = config::TIMER_INTERVAL_OPTIONS;
                let pos = options.iter().position(|&o| o == self.config.timer_interval).unwrap_or(0);
                let next = if forward { (pos + 1) % options.len() } else { (pos + options.len() - 1) % options.len() };
                self.config.timer_interval = options[next].to_string();
                self.save_current_settings();
            }
            config::SettingId::CloudSafetyNet => {
                let options = config::FULL_SYNC_OPTIONS;
                let pos = options.iter().position(|o| o.value == self.config.full_sync_interval).unwrap_or(0);
                let next = if forward { (pos + 1) % options.len() } else { (pos + options.len() - 1) % options.len() };
                self.config.full_sync_interval = options[next].value.to_string();
                self.save_current_settings();
            }
            config::SettingId::BandwidthLimit => {
                let options = config::BWLIMIT_OPTIONS;
                let cur = self.config.bwlimit.as_deref().unwrap_or("Disabled");
                let cur = if cur == "Désactivé" { "Disabled" } else { cur };
                let pos = options.iter().position(|&o| o == cur).unwrap_or(0);
                let next = if forward { (pos + 1) % options.len() } else { (pos + options.len() - 1) % options.len() };
                self.config.bwlimit = if options[next] == "Disabled" { None } else { Some(options[next].to_string()) };
                self.save_current_settings();
            }
            config::SettingId::StatsInterval => {
                let options = STATS_INTERVAL_OPTIONS;
                let cur = self.config.stats_interval.as_str();
                let pos = options.iter().position(|&o| o == cur).unwrap_or(0);
                let next = if forward { (pos + 1) % options.len() } else { (pos + options.len() - 1) % options.len() };
                self.config.stats_interval = options[next].to_string();
                self.stats_interval_changed = true;
                self.set_toast(format!("Rclone stats: {}", self.config.stats_interval));
                self.save_current_settings();
            }
            config::SettingId::ResyncAction => {
                self.modal = Modal::ConfirmResync;
            }
            config::SettingId::ColorTheme => {
                self.current_theme = if forward { self.current_theme.next() } else { self.current_theme.prev() };
                self.config.theme = Some(self.current_theme);
                self.set_toast(format!("Active theme: {}", self.current_theme.name()));
                self.save_current_settings();
            }
            config::SettingId::ContainerLayout => {
                self.config.container_layout = if forward { self.config.container_layout.next() } else { self.config.container_layout.prev() };
                self.set_toast(format!("Container layout: {}", self.config.container_layout.name()));
                self.save_current_settings();
            }
            config::SettingId::MidPanelOrder => {
                self.config.mid_panel_order = if forward { self.config.mid_panel_order.next() } else { self.config.mid_panel_order.prev() };
                self.set_toast(format!("Mid-panel order: {}", self.config.mid_panel_order.name()));
                self.save_current_settings();
            }
            config::SettingId::BorderStyle => {
                self.config.border_style = if forward { self.config.border_style.next() } else { self.config.border_style.prev() };
                self.set_toast(format!("Border style: {}", self.config.border_style.name()));
                self.save_current_settings();
            }
            config::SettingId::GraphStyle => {
                self.config.graph_style = if forward { self.config.graph_style.next() } else { self.config.graph_style.prev() };
                self.set_toast(format!("Graph style: {}", self.config.graph_style.name()));
                self.save_current_settings();
            }
            _ => {}
        }
    }

    pub fn save_current_settings(&mut self) {
        match config::save_config(&self.config) {
            Ok(_) => self.set_toast("✔ Settings saved to dash-config.json!"),
            Err(e) => self.set_toast(format!("✗ Save error: {}", e)),
        }
    }

}
