use crate::engine::snap::adaptive_grid_size;
use crate::model::project::{GridMode, Theme};
use crate::model::Vec2;
use crate::state::editor::ViewportState;
use crate::theme;

/// Render the dot grid on the canvas.
pub fn draw_grid(
    painter: &egui::Painter,
    viewport: &ViewportState,
    canvas_rect: egui::Rect,
    canvas_center: Vec2,
    base_grid_size: f32,
    current_theme: Theme,
    grid_mode: GridMode,
) -> f32 {
    let grid_size = adaptive_grid_size(base_grid_size, viewport.zoom);
    let dot_color = theme::grid_color(current_theme);
    let dot_radius = 1.5;

    // Compute visible world-space bounds
    let top_left = viewport.screen_to_world(
        Vec2::new(canvas_rect.min.x, canvas_rect.min.y),
        canvas_center,
    );
    let bottom_right = viewport.screen_to_world(
        Vec2::new(canvas_rect.max.x, canvas_rect.max.y),
        canvas_center,
    );

    match grid_mode {
        GridMode::Standard => {
            draw_standard_grid(
                painter, viewport, canvas_rect, canvas_center,
                grid_size, dot_color, dot_radius, top_left, bottom_right,
            );
        }
        GridMode::Isometric => {
            draw_isometric_grid(
                painter, viewport, canvas_rect, canvas_center,
                grid_size, dot_color, dot_radius, top_left, bottom_right,
            );
        }
    }

    grid_size
}

fn draw_standard_grid(
    painter: &egui::Painter,
    viewport: &ViewportState,
    canvas_rect: egui::Rect,
    canvas_center: Vec2,
    grid_size: f32,
    dot_color: egui::Color32,
    dot_radius: f32,
    top_left: Vec2,
    bottom_right: Vec2,
) {
    let start_x = (top_left.x / grid_size).floor() as i32;
    let end_x = (bottom_right.x / grid_size).ceil() as i32;
    let start_y = (top_left.y / grid_size).floor() as i32;
    let end_y = (bottom_right.y / grid_size).ceil() as i32;

    // Limit the number of dots to prevent performance issues
    let max_dots = 10000;
    let total = (end_x - start_x + 1) as usize * (end_y - start_y + 1) as usize;
    if total > max_dots {
        return;
    }

    for gx in start_x..=end_x {
        for gy in start_y..=end_y {
            let world_pos = Vec2::new(gx as f32 * grid_size, gy as f32 * grid_size);
            let screen_pos = viewport.world_to_screen(world_pos, canvas_center);

            if canvas_rect.contains(egui::pos2(screen_pos.x, screen_pos.y)) {
                painter.circle_filled(
                    egui::pos2(screen_pos.x, screen_pos.y),
                    dot_radius,
                    dot_color,
                );
            }
        }
    }
}

fn draw_isometric_grid(
    painter: &egui::Painter,
    viewport: &ViewportState,
    canvas_rect: egui::Rect,
    canvas_center: Vec2,
    grid_size: f32,
    dot_color: egui::Color32,
    dot_radius: f32,
    top_left: Vec2,
    bottom_right: Vec2,
) {
    // 2:1 isometric grid
    // horizontal spacing = grid_size, vertical spacing = grid_size / 2
    let half_w = grid_size;
    let half_h = grid_size / 2.0;

    // Compute the range of iso coordinates to cover
    let start_x = (top_left.x / half_w).floor() as i32 - 1;
    let end_x = (bottom_right.x / half_w).ceil() as i32 + 1;
    let start_y = (top_left.y / half_h).floor() as i32 - 1;
    let end_y = (bottom_right.y / half_h).ceil() as i32 + 1;

    let max_dots = 10000;
    let total = (end_x - start_x + 1) as usize * (end_y - start_y + 1) as usize;
    if total > max_dots {
        return;
    }

    // Draw diamond grid: points where (col + row) is even
    for row in start_y..=end_y {
        for col in start_x..=end_x {
            // Only draw where col + row is even (creates the diamond pattern)
            if (col + row) % 2 != 0 {
                continue;
            }
            let world_pos = Vec2::new(col as f32 * half_w, row as f32 * half_h);
            let screen_pos = viewport.world_to_screen(world_pos, canvas_center);

            if canvas_rect.contains(egui::pos2(screen_pos.x, screen_pos.y)) {
                painter.circle_filled(
                    egui::pos2(screen_pos.x, screen_pos.y),
                    dot_radius,
                    dot_color,
                );
            }
        }
    }
}
