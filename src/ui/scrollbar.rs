/// Snapshot of a scrollable list's position at the moment of a render or interaction.
///
/// All three fields are captured together so callers never mix up `pos` and `visible`
/// (a common mistake when passing them as separate `usize` arguments).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollState {
    pub total: usize,
    pub pos: usize,
    pub visible: usize,
}

/// Coordonnées en cellules de terminal pour une scrollbar : colonne (`scroll_x`),
/// ligne de la flèche haute (`top_y`) et ligne de la flèche basse (`bot_y`).
/// La piste occupe les lignes `top_y + 1 .. bot_y` (exclues).
///
/// Les champs nommés évitent les inversions silencieuses `top_y`/`bot_y`
/// qu'entraînerait un passage positionnel en `u16`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollbarTrackBounds {
    pub scroll_x: u16,
    pub top_y: u16,
    pub bot_y: u16,
}

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
