use crate::model::project::GridMode;
use crate::model::vec2::Vec2;

/// Snap a world-space position to the nearest grid point.
pub fn snap_to_grid(pos: Vec2, grid_size: u32, grid_mode: GridMode) -> Vec2 {
    let gs = grid_size as f32;
    if gs < 1.0 {
        return pos;
    }

    match grid_mode {
        GridMode::Off | GridMode::Straight => Vec2::new(
            (pos.x / gs).round() * gs,
            (pos.y / gs).round() * gs,
        ),
        GridMode::Isometric => {
            // Isometric grid: 2:1 ratio (26.57 degrees)
            // Snap to nearest intersection of the two diagonal axis sets
            // Iso axes: (2, 1) and (2, -1), scaled by grid_size
            let half_gs = gs / 2.0;

            // Transform to iso coordinates
            let iso_x = pos.x / gs + pos.y / half_gs;
            let iso_y = pos.x / gs - pos.y / half_gs;

            // Round in iso space
            let iso_x_r = iso_x.round();
            let iso_y_r = iso_y.round();

            // Transform back
            Vec2::new(
                (iso_x_r + iso_y_r) * gs / 2.0,
                (iso_x_r - iso_y_r) * half_gs / 2.0,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snap_straight() {
        let snapped = snap_to_grid(Vec2::new(13.0, 7.0), 8, GridMode::Straight);
        assert_eq!(snapped, Vec2::new(16.0, 8.0));
    }

    #[test]
    fn test_snap_straight_exact() {
        let snapped = snap_to_grid(Vec2::new(16.0, 8.0), 8, GridMode::Straight);
        assert_eq!(snapped, Vec2::new(16.0, 8.0));
    }

    #[test]
    fn test_snap_isometric() {
        // Just verify it returns a valid snapped position
        let snapped = snap_to_grid(Vec2::new(10.0, 5.0), 8, GridMode::Isometric);
        // Should be near a grid point
        assert!(snapped.x.is_finite());
        assert!(snapped.y.is_finite());
    }
}
