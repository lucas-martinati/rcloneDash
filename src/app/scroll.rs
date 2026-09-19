use super::*;

impl App {
    pub fn ensure_history_visible(&mut self, viewport_height: usize) {
        if viewport_height == 0 { return; }
        if let Some(idx) = self.selected_run_idx {
            if idx < self.history_scroll_offset {
                self.history_scroll_offset = idx;
            } else if idx >= self.history_scroll_offset + viewport_height {
                self.history_scroll_offset = idx - viewport_height + 1;
            }
        }
    }

    pub fn total_log_lines(&self) -> usize {
        if self.logs_total_wrapped > 0 {
            self.logs_total_wrapped
        } else {
            self.filtered_log_lines().len()
        }
    }

    pub fn scroll_list_step(
        selected_idx: &mut usize,
        scroll_offset: &mut usize,
        total: usize,
        viewport_height: usize,
        up: bool,
    ) {
        if total == 0 {
            return;
        }
        let vp = viewport_height.max(1);
        let max_offset = total.saturating_sub(vp);

        // Adjust selection if it was outside the visible viewport
        if *selected_idx < *scroll_offset {
            *selected_idx = *scroll_offset;
        } else if *selected_idx >= *scroll_offset + vp {
            *selected_idx = (*scroll_offset + vp).saturating_sub(1);
        }

        if up {
            if *selected_idx > 0 {
                *selected_idx -= 1;
                if *selected_idx < *scroll_offset {
                    *scroll_offset = *selected_idx;
                }
            }
        } else if *selected_idx < total.saturating_sub(1) {
            *selected_idx += 1;
            if *selected_idx >= *scroll_offset + vp {
                *scroll_offset = (*selected_idx + 1).saturating_sub(vp).min(max_offset);
            }
        }
    }

    pub fn scroll_list_jump(
        selected_idx: &mut usize,
        scroll_offset: &mut usize,
        total: usize,
        viewport_height: usize,
        ratio: f64,
    ) {
        if total == 0 {
            return;
        }
        let vp = viewport_height.max(1);
        let max_offset = total.saturating_sub(vp);
        let new_offset = ((ratio * max_offset as f64).round() as usize).min(max_offset);
        *scroll_offset = new_offset;

        if ratio >= 1.0 {
            *selected_idx = (new_offset + vp.saturating_sub(1)).min(total.saturating_sub(1));
        } else if ratio <= 0.0 {
            *selected_idx = new_offset.min(total.saturating_sub(1));
        } else if *selected_idx < new_offset {
            *selected_idx = new_offset;
        } else if *selected_idx >= new_offset + vp {
            *selected_idx = (new_offset + vp).saturating_sub(1).min(total.saturating_sub(1));
        }
    }

    pub fn scroll_opt_list_step(
        selected_idx: &mut Option<usize>,
        scroll_offset: &mut usize,
        total: usize,
        viewport_height: usize,
        up: bool,
    ) {
        if total == 0 {
            return;
        }
        match selected_idx {
            None => {
                if !up {
                    *selected_idx = Some((*scroll_offset).min(total - 1));
                }
            }
            Some(ref mut idx) => {
                if up && *idx == 0 && *scroll_offset == 0 {
                    *selected_idx = None;
                    return;
                }
                Self::scroll_list_step(idx, scroll_offset, total, viewport_height, up);
            }
        }
    }

    pub fn scroll_opt_list_jump(
        selected_idx: &mut Option<usize>,
        scroll_offset: &mut usize,
        total: usize,
        viewport_height: usize,
        ratio: f64,
    ) {
        if total == 0 {
            return;
        }
        let vp = viewport_height.max(1);
        let max_offset = total.saturating_sub(vp);
        let new_offset = ((ratio * max_offset as f64).round() as usize).min(max_offset);
        *scroll_offset = new_offset;

        if ratio >= 1.0 {
            let target_sel = (new_offset + vp.saturating_sub(1)).min(total.saturating_sub(1));
            *selected_idx = Some(target_sel);
        } else if ratio <= 0.0 {
            *selected_idx = Some(new_offset.min(total.saturating_sub(1)));
        } else if let Some(ref mut idx) = selected_idx {
            if *idx < new_offset {
                *idx = new_offset;
            } else if *idx >= new_offset + vp {
                *idx = (new_offset + vp).saturating_sub(1).min(total.saturating_sub(1));
            }
        }
    }


    pub fn scroll_list_view_step(
        selected_idx: &mut usize,
        scroll_offset: &mut usize,
        total: usize,
        viewport_height: usize,
        up: bool,
    ) {
        if total == 0 {
            return;
        }
        let vp = viewport_height.max(1);
        let max_offset = total.saturating_sub(vp);

        if up {
            if *scroll_offset > 0 {
                *scroll_offset -= 1;
                *selected_idx = selected_idx.saturating_sub(1);
            } else if *selected_idx > 0 {
                *selected_idx -= 1;
            }
            if *selected_idx >= *scroll_offset + vp {
                *selected_idx = (*scroll_offset + vp).saturating_sub(1).min(total.saturating_sub(1));
            }
        } else {
            if *scroll_offset < max_offset {
                *scroll_offset += 1;
                *selected_idx = (*selected_idx + 1).min(total.saturating_sub(1));
            } else if *selected_idx < total.saturating_sub(1) {
                *selected_idx += 1;
            }
            if *selected_idx < *scroll_offset {
                *selected_idx = *scroll_offset;
            }
        }
    }

    pub fn scroll_opt_list_view_step(
        selected_idx: &mut Option<usize>,
        scroll_offset: &mut usize,
        total: usize,
        viewport_height: usize,
        up: bool,
    ) {
        if total == 0 {
            return;
        }
        let vp = viewport_height.max(1);
        let max_offset = total.saturating_sub(vp);

        if up {
            if *scroll_offset > 0 {
                *scroll_offset -= 1;
                if let Some(ref mut idx) = selected_idx {
                    *idx = idx.saturating_sub(1);
                }
            } else if let Some(ref mut idx) = selected_idx {
                if *idx > 0 {
                    *idx -= 1;
                } else if *scroll_offset == 0 {
                    *selected_idx = None;
                    return;
                }
            }
            if let Some(ref mut idx) = selected_idx {
                if *idx >= *scroll_offset + vp {
                    *idx = (*scroll_offset + vp).saturating_sub(1).min(total.saturating_sub(1));
                }
            }
        } else {
            if *scroll_offset < max_offset {
                *scroll_offset += 1;
                if let Some(ref mut idx) = selected_idx {
                    *idx = (*idx + 1).min(total.saturating_sub(1));
                }
            } else if let Some(ref mut idx) = selected_idx {
                if *idx < total.saturating_sub(1) {
                    *idx += 1;
                }
            }
            if let Some(ref mut idx) = selected_idx {
                if *idx < *scroll_offset {
                    *idx = *scroll_offset;
                }
            }
        }
    }


    pub fn ensure_recent_visible(&mut self, viewport_height: usize) {
        let vp = viewport_height.max(1);
        if let Some(idx) = self.recent_selected_idx {
            if idx < self.recent_scroll_offset {
                self.recent_scroll_offset = idx;
            } else if idx >= self.recent_scroll_offset + vp {
                self.recent_scroll_offset = idx + 1 - vp;
            }
        }
    }

    pub fn scroll_recent_down(&mut self, viewport_height: usize) {
        let total = self.get_recent_files_list().len();
        Self::scroll_opt_list_step(&mut self.recent_selected_idx, &mut self.recent_scroll_offset, total, viewport_height, false);
    }

    pub fn scroll_recent_up(&mut self, viewport_height: usize) {
        let total = self.get_recent_files_list().len();
        Self::scroll_opt_list_step(&mut self.recent_selected_idx, &mut self.recent_scroll_offset, total, viewport_height, true);
    }

    pub fn scroll_recent_view_down(&mut self, viewport_height: usize) {
        let total = self.get_recent_files_list().len();
        Self::scroll_opt_list_view_step(&mut self.recent_selected_idx, &mut self.recent_scroll_offset, total, viewport_height, false);
    }

    pub fn scroll_recent_view_up(&mut self, viewport_height: usize) {
        let total = self.get_recent_files_list().len();
        Self::scroll_opt_list_view_step(&mut self.recent_selected_idx, &mut self.recent_scroll_offset, total, viewport_height, true);
    }

    pub fn scroll_history_down(&mut self, viewport_height: usize) {
        let total = self.total_history_runs();
        if total == 0 {
            return;
        }
        let vp = viewport_height.max(1);
        let max_offset = total.saturating_sub(vp);

        match self.selected_run_idx {
            None => {
                self.selected_run_idx = Some(self.history_scroll_offset.min(total - 1));
            }
            Some(mut idx) => {
                if idx < self.history_scroll_offset {
                    idx = self.history_scroll_offset;
                } else if idx >= self.history_scroll_offset + vp {
                    idx = (self.history_scroll_offset + vp).saturating_sub(1);
                }

                if self.history_scroll_offset < max_offset {
                    self.history_scroll_offset += 1;
                    self.selected_run_idx = Some((idx + 1).min(total - 1));
                } else if idx < total - 1 {
                    self.selected_run_idx = Some(idx + 1);
                }
            }
        }
    }

    pub fn scroll_history_up(&mut self, viewport_height: usize) {
        let total = self.total_history_runs();
        if total == 0 {
            return;
        }
        let vp = viewport_height.max(1);

        match self.selected_run_idx {
            None => {}
            Some(mut idx) => {
                if idx == 0 && self.history_scroll_offset == 0 {
                    self.selected_run_idx = None;
                    return;
                }
                if idx < self.history_scroll_offset {
                    idx = self.history_scroll_offset;
                } else if idx >= self.history_scroll_offset + vp {
                    idx = (self.history_scroll_offset + vp).saturating_sub(1);
                }

                if self.history_scroll_offset > 0 {
                    self.history_scroll_offset -= 1;
                    self.selected_run_idx = Some(idx.saturating_sub(1));
                } else if idx > 0 {
                    self.selected_run_idx = Some(idx - 1);
                }
            }
        }
    }

    pub fn is_dragging_scrollbar(&self, target: ScrollbarTarget) -> bool {
        if let Some((t, _, _, _, _, _)) = &self.active_scrollbar_drag {
            *t == target
        } else {
            false
        }
    }

    pub fn apply_scrollbar_step(&mut self, target: ScrollbarTarget, up: bool) {
        match target {
            ScrollbarTarget::Logs => {
                self.focused_panel = FocusedPanel::Logs;
                if up {
                    let visible_height = self.logs_viewport_height.max(3);
                    let max_scroll = self.total_log_lines().saturating_sub(visible_height);
                    if max_scroll > 0 && self.logs_scroll < max_scroll {
                        self.auto_scroll = false;
                        self.logs_scroll += 1;
                    }
                } else {
                    if self.logs_scroll > 0 {
                        self.logs_scroll = self.logs_scroll.saturating_sub(1);
                        if self.logs_scroll == 0 {
                            self.auto_scroll = true;
                        }
                    }
                }
            }
            ScrollbarTarget::History => {
                self.focused_panel = FocusedPanel::History;
                let vp = self.history_viewport_height.max(2);
                if up {
                    self.scroll_history_up(vp);
                } else {
                    self.scroll_history_down(vp);
                }
            }
            ScrollbarTarget::RecentFiles => {
                self.focused_panel = FocusedPanel::RecentFiles;
                let vp = self.recent_viewport_height.max(1);
                if up {
                    self.scroll_recent_view_up(vp);
                } else {
                    self.scroll_recent_view_down(vp);
                }
            }
            ScrollbarTarget::HistoryDetails(run_idx) => {
                if up {
                    if self.history_details_scroll > 0 {
                        self.history_details_scroll -= 1;
                    }
                } else {
                    let total_files = self.past_runs.get(run_idx).map(|r| r.all_affected_files().len()).unwrap_or(0);
                    let max_scroll = total_files.saturating_sub(5);
                    if self.history_details_scroll < max_scroll {
                        self.history_details_scroll += 1;
                    }
                }
            }
            ScrollbarTarget::Files => {
                let vp = self.file_viewport_height.max(1);
                if up {
                    self.scroll_file_view_up(vp);
                } else {
                    self.scroll_file_view_down(vp);
                }
            }
            ScrollbarTarget::Filters => {
                let vp = self.filter_viewport_height.max(1);
                if up {
                    self.scroll_filter_view_up(vp);
                } else {
                    self.scroll_filter_view_down(vp);
                }
            }
            ScrollbarTarget::DryRun => {
                if up {
                    self.dry_run_scroll = self.dry_run_scroll.saturating_sub(1);
                } else {
                    let max_dry = self.dry_run_logs.len().saturating_sub(5);
                    if self.dry_run_scroll < max_dry {
                        self.dry_run_scroll += 1;
                    }
                }
            }
        }
    }

    pub fn get_scrollbar_current_pos(&self, target: ScrollbarTarget, total: usize, visible: usize) -> usize {
        let max_scroll = total.saturating_sub(visible);
        match target {
            ScrollbarTarget::Logs => {
                if self.auto_scroll {
                    max_scroll
                } else {
                    max_scroll.saturating_sub(self.logs_scroll)
                }
            }
            ScrollbarTarget::History => self.history_scroll_offset.min(max_scroll),
            ScrollbarTarget::RecentFiles => self.recent_scroll_offset.min(max_scroll),
            ScrollbarTarget::HistoryDetails(_) => self.history_details_scroll.min(max_scroll),
            ScrollbarTarget::Files => self.file_scroll_offset.min(max_scroll),
            ScrollbarTarget::Filters => self.filter_scroll_offset.min(max_scroll),
            ScrollbarTarget::DryRun => self.dry_run_scroll.min(max_scroll),
        }
    }

    pub fn compute_scrollbar_grab_offset(
        &self,
        target: ScrollbarTarget,
        row: u16,
        top_y: u16,
        track_height: u16,
        total: usize,
        visible: usize,
    ) -> u16 {
        if total <= visible || track_height == 0 {
            return 0;
        }
        let current_pos = self.get_scrollbar_current_pos(target, total, visible);
        let geom = crate::ui::scrollbar::ScrollbarGeometry::compute(total, visible, track_height as usize, current_pos);

        let click_offset = (row.saturating_sub(top_y) as usize).min((track_height.saturating_sub(1)) as usize);
        if click_offset >= geom.thumb_start && click_offset < geom.thumb_start + geom.thumb_size {
            // Click directly on thumb: preserve exact grab offset under cursor
            (click_offset - geom.thumb_start) as u16
        } else {
            // Click on track: center thumb on clicked position
            (geom.thumb_size / 2) as u16
        }
    }

    pub fn apply_scrollbar_drag_to_row(
        &mut self,
        target: ScrollbarTarget,
        row: u16,
        top_y: u16,
        track_height: u16,
        total: usize,
        visible: usize,
        grab_offset: u16,
    ) {
        if total <= visible || track_height == 0 {
            return;
        }
        let current_pos = self.get_scrollbar_current_pos(target, total, visible);
        let geom = crate::ui::scrollbar::ScrollbarGeometry::compute(total, visible, track_height as usize, current_pos);

        if geom.available_travel == 0 {
            return;
        }

        let ratio = if row <= top_y + grab_offset {
            0.0
        } else if row >= top_y + grab_offset + geom.available_travel as u16 {
            1.0
        } else {
            let click_offset = row.saturating_sub(top_y);
            let target_thumb_start = (click_offset.saturating_sub(grab_offset) as usize).min(geom.available_travel);
            (target_thumb_start as f64 / geom.available_travel as f64).clamp(0.0, 1.0)
        };

        self.apply_scrollbar_ratio(target, ratio, total, visible);
    }


    pub fn apply_scrollbar_ratio(
        &mut self,
        target: ScrollbarTarget,
        ratio: f64,
        total: usize,
        visible: usize,
    ) {
        if total <= visible {
            return;
        }
        let max_scroll = total.saturating_sub(visible);
        let ratio = ratio.clamp(0.0, 1.0);

        match target {
            ScrollbarTarget::Logs => {
                self.focused_panel = FocusedPanel::Logs;
                let pos = ((ratio * max_scroll as f64).round() as usize).min(max_scroll);
                let new_logs_scroll = max_scroll.saturating_sub(pos);
                self.logs_scroll = new_logs_scroll;
                self.auto_scroll = new_logs_scroll == 0;
            }
            ScrollbarTarget::History => {
                self.focused_panel = FocusedPanel::History;
                let vp = self.history_viewport_height.max(2);
                Self::scroll_opt_list_jump(&mut self.selected_run_idx, &mut self.history_scroll_offset, total, vp, ratio);
            }
            ScrollbarTarget::RecentFiles => {
                self.focused_panel = FocusedPanel::RecentFiles;
                let vp = self.recent_viewport_height.max(1);
                Self::scroll_opt_list_jump(&mut self.recent_selected_idx, &mut self.recent_scroll_offset, total, vp, ratio);
            }
            ScrollbarTarget::HistoryDetails(run_idx) => {
                self.history_details_scroll = ((ratio * max_scroll as f64).round() as usize).min(max_scroll);
                let total_files = self.past_runs.get(run_idx).map(|r| r.all_affected_files().len()).unwrap_or(0);
                if total_files > 0 {
                    let vp = visible.max(1);
                    if ratio >= 1.0 {
                        self.history_selected_file_idx = (self.history_details_scroll + vp.saturating_sub(1)).min(total_files.saturating_sub(1));
                    } else if ratio <= 0.0 {
                        self.history_selected_file_idx = self.history_details_scroll.min(total_files.saturating_sub(1));
                    } else if self.history_selected_file_idx < self.history_details_scroll {
                        self.history_selected_file_idx = self.history_details_scroll;
                    } else if self.history_selected_file_idx >= self.history_details_scroll + vp {
                        self.history_selected_file_idx = (self.history_details_scroll + vp).saturating_sub(1).min(total_files.saturating_sub(1));
                    }
                }
            }
            ScrollbarTarget::Files => {
                let vp = self.file_viewport_height.max(1);
                Self::scroll_list_jump(&mut self.file_selected_idx, &mut self.file_scroll_offset, total, vp, ratio);
            }
            ScrollbarTarget::Filters => {
                let vp = self.filter_viewport_height.max(1);
                Self::scroll_list_jump(&mut self.selected_filter_idx, &mut self.filter_scroll_offset, total, vp, ratio);
            }
            ScrollbarTarget::DryRun => {
                self.dry_run_scroll = ((ratio * max_scroll as f64).round() as usize).min(max_scroll);
            }
        }
    }

}
