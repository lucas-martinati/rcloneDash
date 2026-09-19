#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollbarGeometry {
    pub thumb_size: usize,
    pub available_travel: usize,
    pub thumb_start: usize,
}

impl ScrollbarGeometry {
    /// Calcule avec précision la géométrie du curseur de scrollbar
    pub fn compute(total: usize, visible: usize, track_height: usize, pos: usize) -> Self {
        if total == 0 || track_height == 0 {
            return Self {
                thumb_size: 0,
                available_travel: 0,
                thumb_start: 0,
            };
        }
        let thumb_size = (((visible as f64 / total.max(1) as f64) * track_height as f64).round() as usize)
            .max(1)
            .min(track_height);
        let available_travel = track_height.saturating_sub(thumb_size);
        let max_scroll = total.saturating_sub(visible);
        let effective_pos = pos.min(max_scroll);
        let thumb_start = if max_scroll > 0 && available_travel > 0 {
            ((effective_pos as f64 / max_scroll as f64) * available_travel as f64).round() as usize
        } else {
            0
        };
        Self {
            thumb_size,
            available_travel,
            thumb_start,
        }
    }
}
