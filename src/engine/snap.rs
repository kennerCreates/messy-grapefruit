use crate::model::project::GridMode;
use crate::model::vec2::Vec2;

/// Snap a world-space position to the nearest grid point.
pub fn snap_to_grid(pos: Vec2, grid_size: u32, grid_mode: GridMode) -> Vec2 {
    let gs = grid_size as f32;
    if gs < 1.0 {
        return pos;
    }

    // Always snap to isometric diamond lattice points.
    // Lattice basis: u = (2gs, gs), v = (2gs, -gs).
    // Transform to lattice coordinates, round, transform back.
    let _ = grid_mode;
    let s = (pos.x + 2.0 * pos.y) / (4.0 * gs);
    let t = (pos.x - 2.0 * pos.y) / (4.0 * gs);
    let sr = s.round();
    let tr = t.round();
    Vec2::new(
        2.0 * gs * (sr + tr),
        gs * (sr - tr),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snap_origin() {
        let snapped = snap_to_grid(Vec2::new(1.0, 1.0), 8, GridMode::Straight);
        assert_eq!(snapped, Vec2::new(0.0, 0.0));
    }

    #[test]
    fn test_snap_exact_lattice_point() {
        // (16, 8) is a lattice point (even row offset by 2gs)
        let snapped = snap_to_grid(Vec2::new(16.0, 8.0), 8, GridMode::Straight);
        assert_eq!(snapped, Vec2::new(16.0, 8.0));
    }

    #[test]
    fn test_snap_isometric_lattice() {
        // Near (32, 0) which is a lattice point (even row, x=4gs)
        let snapped = snap_to_grid(Vec2::new(30.0, 1.0), 8, GridMode::Isometric);
        assert_eq!(snapped, Vec2::new(32.0, 0.0));
    }

    #[test]
    fn test_snap_odd_row() {
        // (16, 8) is a lattice point on odd row (gy=1, gx=2 → 2*8=16)
        let snapped = snap_to_grid(Vec2::new(17.0, 7.0), 8, GridMode::Off);
        assert_eq!(snapped, Vec2::new(16.0, 8.0));
    }
}
