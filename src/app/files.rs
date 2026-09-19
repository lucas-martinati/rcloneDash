use super::*;

impl App {
    pub fn reload_files(&mut self) {
        let base = config::expand_tilde(&self.config.local_dir);
        if let Ok(entries) = fs_tree::list_directory(&base, &self.file_current_rel, &self.filters) {
            self.file_entries = entries;
            if self.file_selected_idx >= self.file_entries.len() {
                self.file_selected_idx = 0;
            }
        }
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

    pub fn scroll_file_down(&mut self, viewport_height: usize) {
        let total = self.file_entries.len();
        Self::scroll_list_step(&mut self.file_selected_idx, &mut self.file_scroll_offset, total, viewport_height, false);
    }

    pub fn scroll_file_up(&mut self, viewport_height: usize) {
        let total = self.file_entries.len();
        Self::scroll_list_step(&mut self.file_selected_idx, &mut self.file_scroll_offset, total, viewport_height, true);
    }

    pub fn scroll_file_view_down(&mut self, viewport_height: usize) {
        let total = self.file_entries.len();
        Self::scroll_list_view_step(&mut self.file_selected_idx, &mut self.file_scroll_offset, total, viewport_height, false);
    }

    pub fn scroll_file_view_up(&mut self, viewport_height: usize) {
        let total = self.file_entries.len();
        Self::scroll_list_view_step(&mut self.file_selected_idx, &mut self.file_scroll_offset, total, viewport_height, true);
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

}
