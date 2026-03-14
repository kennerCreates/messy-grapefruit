use crate::model::Vec2;
use crate::model::project::GridMode;

/// Compute the adaptive grid size based on current zoom level.
/// Uses power-of-2 grid sizes. At higher zoom, grid is finer; at lower zoom, grid is coarser.
pub fn adaptive_grid_size(base_grid_size: f32, zoom: f32) -> f32 {
    // Target: keep grid dots at ~20-40 pixel screen spacing
    let target_screen_spacing = 32.0;
    let world_spacing = target_screen_spacing / zoom;

    // Round to nearest power of 2 that's >= base_grid_size minimum of 1
    let raw = world_spacing / base_grid_size;
    let power = raw.log2().round() as i32;
    let multiplier = 2.0f32.powi(power).max(1.0);
    base_grid_size * multiplier
}

/// Snap a world position to the nearest grid intersection.
pub fn snap_to_grid(pos: Vec2, grid_size: f32, grid_mode: GridMode) -> Vec2 {
    match grid_mode {
        GridMode::Standard => {
            Vec2 {
                x: (pos.x / grid_size).round() * grid_size,
                y: (pos.y / grid_size).round() * grid_size,
            }
        }
        GridMode::Isometric => {
            // 2:1 isometric grid (26.57 degrees)
            // Isometric grid has diamond-shaped cells
            let half_w = grid_size;
            let half_h = grid_size / 2.0;

            // Convert to isometric grid coordinates
            let iso_x = pos.x / half_w;
            let iso_y = pos.y / half_h;

            // In iso space, snap to points where (iso_x + iso_y) and (iso_x - iso_y)
            // are both even or both odd
            let col = (iso_x + iso_y).round();
            let row = (iso_x - iso_y).round();

            // Ensure col and row have the same parity
            let col = col as i32;
            let row = row as i32;
            let (col, row) = if (col + row) % 2 != 0 {
                // Adjust to nearest valid iso intersection
                let col_f = iso_x + iso_y;
                let row_f = iso_x - iso_y;
                let col_floor = col_f.floor() as i32;
                let col_ceil = col_f.ceil() as i32;
                let row_floor = row_f.floor() as i32;
                let row_ceil = row_f.ceil() as i32;

                // Try all four combinations and pick closest
                let candidates = [
                    (col_floor, row_floor),
                    (col_floor, row_ceil),
                    (col_ceil, row_floor),
                    (col_ceil, row_ceil),
                ];

                let mut best = (col, row);
                let mut best_dist = f32::MAX;
                for (c, r) in candidates {
                    if (c + r) % 2 == 0 {
                        let wx = ((c + r) as f32 / 2.0) * half_w;
                        let wy = ((c - r) as f32 / 2.0) * half_h;
                        let d = (wx - pos.x).powi(2) + (wy - pos.y).powi(2);
                        if d < best_dist {
                            best_dist = d;
                            best = (c, r);
                        }
                    }
                }
                best
            } else {
                (col, row)
            };

            Vec2 {
                x: ((col + row) as f32 / 2.0) * half_w,
                y: ((col - row) as f32 / 2.0) * half_h,
            }
        }
    }
}
