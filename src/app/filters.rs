use super::*;

impl App {
    pub fn ensure_filter_visible(&mut self, viewport_height: usize) {
        if viewport_height == 0 { return; }
        if self.selected_filter_idx < self.filter_scroll_offset {
            self.filter_scroll_offset = self.selected_filter_idx;
        } else if self.selected_filter_idx >= self.filter_scroll_offset + viewport_height {
            self.filter_scroll_offset = self.selected_filter_idx - viewport_height + 1;
        }
    }


    pub fn scroll_filter_down(&mut self, viewport_height: usize) {
        let total = if self.is_adding_filter { self.filters.len() + 1 } else { self.filters.len() };
        Self::scroll_list_step(&mut self.selected_filter_idx, &mut self.filter_scroll_offset, total, viewport_height, false);
    }

    pub fn scroll_filter_up(&mut self, viewport_height: usize) {
        let total = if self.is_adding_filter { self.filters.len() + 1 } else { self.filters.len() };
        Self::scroll_list_step(&mut self.selected_filter_idx, &mut self.filter_scroll_offset, total, viewport_height, true);
    }

    pub fn scroll_filter_view_down(&mut self, viewport_height: usize) {
        let total = if self.is_adding_filter { self.filters.len() + 1 } else { self.filters.len() };
        Self::scroll_list_view_step(&mut self.selected_filter_idx, &mut self.filter_scroll_offset, total, viewport_height, false);
    }

    pub fn scroll_filter_view_up(&mut self, viewport_height: usize) {
        let total = if self.is_adding_filter { self.filters.len() + 1 } else { self.filters.len() };
        Self::scroll_list_view_step(&mut self.selected_filter_idx, &mut self.filter_scroll_offset, total, viewport_height, true);
    }

    pub fn cycle_filter_type(&mut self, idx: usize) {
        if idx < self.filters.len() {
            self.selected_filter_idx = idx;
            let rule = &self.filters[idx];
            let trimmed = rule.trim();
            let new_rule = if trimmed.starts_with('+') {
                format!("- {}", trimmed.trim_start_matches('+').trim())
            } else if trimmed.starts_with('-') {
                format!("# {}", trimmed.trim_start_matches('-').trim())
            } else if trimmed.starts_with('#') {
                trimmed.trim_start_matches('#').trim().to_string()
            } else {
                format!("+ {}", trimmed)
            };
            self.filters[idx] = new_rule;
            let _ = config::save_filters(&self.filters);
            self.set_toast("✔ Filter rule type changed");
        }
    }

    pub fn start_editing_filter(&mut self) {
        if self.filters.is_empty() {
            self.start_adding_filter();
            return;
        }
        if self.selected_filter_idx < self.filters.len() {
            self.is_editing_filter = true;
            self.is_adding_filter = false;
            self.filter_edit_buffer = self.filters[self.selected_filter_idx].clone();
        }
    }

    pub fn start_adding_filter(&mut self) {
        self.is_editing_filter = true;
        self.is_adding_filter = true;
        self.filter_edit_buffer = "- ".to_string();
        self.selected_filter_idx = self.filters.len();
        let vp = self.filter_viewport_height;
        self.ensure_filter_visible(vp);
    }

    pub fn commit_filter_edit(&mut self) {
        if !self.is_editing_filter {
            return;
        }
        let trimmed = self.filter_edit_buffer.trim();
        if trimmed.is_empty() {
            self.cancel_filter_edit();
            return;
        }

        if self.is_adding_filter {
            self.filters.push(self.filter_edit_buffer.trim().to_string());
            self.selected_filter_idx = self.filters.len().saturating_sub(1);
            let _ = config::save_filters(&self.filters);
            self.set_toast("✔ Filter rule added");
        } else if self.selected_filter_idx < self.filters.len() {
            self.filters[self.selected_filter_idx] = self.filter_edit_buffer.trim().to_string();
            let _ = config::save_filters(&self.filters);
            self.set_toast("✔ Filter rule updated");
        }
        self.is_editing_filter = false;
        self.is_adding_filter = false;
        self.filter_edit_buffer.clear();
        let vp = self.filter_viewport_height;
        self.ensure_filter_visible(vp);
    }

    pub fn cancel_filter_edit(&mut self) {
        self.is_editing_filter = false;
        self.is_adding_filter = false;
        self.filter_edit_buffer.clear();
        if self.selected_filter_idx >= self.filters.len() && !self.filters.is_empty() {
            self.selected_filter_idx = self.filters.len() - 1;
        }
    }

    pub fn delete_selected_filter(&mut self) {
        if self.filters.is_empty() {
            return;
        }
        if self.selected_filter_idx < self.filters.len() {
            self.filters.remove(self.selected_filter_idx);
            if self.selected_filter_idx >= self.filters.len() && !self.filters.is_empty() {
                self.selected_filter_idx = self.filters.len() - 1;
            }
            let _ = config::save_filters(&self.filters);
            self.set_toast("✔ Filter rule deleted");
            let vp = self.filter_viewport_height;
            self.ensure_filter_visible(vp);
        }
    }

}
