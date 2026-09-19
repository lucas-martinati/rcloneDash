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

}
