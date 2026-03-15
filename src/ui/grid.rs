use egui::{Painter, Pos2};

use crate::model::project::{EditorPreferences, GridMode};
use crate::model::vec2::Vec2;
use crate::state::editor::ViewportState;
use crate::theme;
use crate::model::project::Theme;

/// Render the grid (dots and/or lines) on the canvas.
pub fn render_grid(
    painter: &Painter,
    viewport: &ViewportState,
    prefs: &EditorPreferences,
    canvas_rect: egui::Rect,
    theme: Theme,
) {
    let canvas_center = canvas_rect.center();
    let gs = prefs.grid_size as f32;
    if gs < 1.0 {
        return;
    }

    // Calculate world-space bounds of visible area
    let top_left = viewport.screen_to_world(canvas_rect.left_top(), canvas_center);
    let bottom_right = viewport.screen_to_world(canvas_rect.right_bottom(), canvas_center);
    let world_min = top_left.min(bottom_right);
    let world_max = top_left.max(bottom_right);

    // Grid range with one extra cell of padding
    let start_x = ((world_min.x / gs).floor() as i32 - 1).max(-1000);
    let end_x = ((world_max.x / gs).ceil() as i32 + 1).min(1000);
    let start_y = ((world_min.y / gs).floor() as i32 - 1).max(-1000);
    let end_y = ((world_max.y / gs).ceil() as i32 + 1).min(1000);

    match prefs.grid_mode {
        GridMode::Off => {}
        GridMode::Straight | GridMode::Isometric => {
            render_lines(painter, viewport, prefs, canvas_center, theme, gs, start_x, end_x, start_y, end_y);
        }
    }

    if prefs.show_dots {
        render_dots(painter, viewport, canvas_rect, canvas_center, theme, gs, start_x, end_x, start_y, end_y);
    }
}

#[allow(clippy::too_many_arguments)]
fn render_dots(
    painter: &Painter,
    viewport: &ViewportState,
    canvas_rect: egui::Rect,
    canvas_center: Pos2,
    theme: Theme,
    gs: f32,
    start_x: i32, end_x: i32,
    start_y: i32, end_y: i32,
) {
    let dot_color = theme::grid_dot_color(theme);
    let screen_gs = gs * viewport.zoom;

    // Skip dots at extreme zoom-out where they'd overlap
    if screen_gs < 4.0 {
        return;
    }

    let dot_radius = (1.0_f32).max(viewport.zoom * 0.5).min(2.0);

    // Dots at isometric diamond lattice points (staggered grid).
    for gy in start_y..=end_y {
        let row_even = gy.rem_euclid(2) == 0;
        let gx_start = if row_even {
            start_x - start_x.rem_euclid(4)
        } else {
            let k = (start_x - 2).div_euclid(4);
            k * 4 + 2
        };
        let mut gx = gx_start;
        while gx <= end_x {
            let world = Vec2::new(gx as f32 * gs, gy as f32 * gs);
            let screen = viewport.world_to_screen(world, canvas_center);
            if canvas_rect.contains(screen) {
                painter.circle_filled(screen, dot_radius, dot_color);
            }
            gx += 4;
        }
    }
}

/// Draw a dashed line between two screen-space points.
fn draw_dashed_line(painter: &Painter, p1: Pos2, p2: Pos2, stroke: egui::Stroke, dash: f32, gap: f32) {
    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1.0 {
        return;
    }
    let nx = dx / len;
    let ny = dy / len;
    let cycle = dash + gap;
    let mut t = 0.0;
    while t < len {
        let t_end = (t + dash).min(len);
        let a = Pos2::new(p1.x + nx * t, p1.y + ny * t);
        let b = Pos2::new(p1.x + nx * t_end, p1.y + ny * t_end);
        painter.line_segment([a, b], stroke);
        t += cycle;
    }
}

#[allow(clippy::too_many_arguments)]
fn render_lines(
    painter: &Painter,
    viewport: &ViewportState,
    prefs: &EditorPreferences,
    canvas_center: Pos2,
    theme: Theme,
    gs: f32,
    start_x: i32, end_x: i32,
    start_y: i32, end_y: i32,
) {
    let line_color = theme::grid_line_color(theme);
    let stroke = egui::Stroke::new(0.5, line_color);
    let dash = 4.0;
    let gap = 4.0;

    match prefs.grid_mode {
        GridMode::Off => {}
        GridMode::Straight => {
            // Vertical lines
            for gx in start_x..=end_x {
                let x = gx as f32 * gs;
                let top = viewport.world_to_screen(Vec2::new(x, start_y as f32 * gs), canvas_center);
                let bot = viewport.world_to_screen(Vec2::new(x, end_y as f32 * gs), canvas_center);
                draw_dashed_line(painter, top, bot, stroke, dash, gap);
            }
            // Horizontal lines
            for gy in start_y..=end_y {
                let y = gy as f32 * gs;
                let left = viewport.world_to_screen(Vec2::new(start_x as f32 * gs, y), canvas_center);
                let right = viewport.world_to_screen(Vec2::new(end_x as f32 * gs, y), canvas_center);
                draw_dashed_line(painter, left, right, stroke, dash, gap);
            }
        }
        GridMode::Isometric => {
            let step = 4i32;

            // Slope +0.5 lines
            let k_min_pos = 2 * start_y - end_x;
            let k_max_pos = 2 * end_y - start_x;
            let k_start = k_min_pos - k_min_pos.rem_euclid(step);
            let mut k = k_start;
            while k <= k_max_pos {
                let x1 = start_x as f32 * gs;
                let x2 = end_x as f32 * gs;
                let y1 = 0.5 * x1 + k as f32 * gs / 2.0;
                let y2 = 0.5 * x2 + k as f32 * gs / 2.0;
                let s1 = viewport.world_to_screen(Vec2::new(x1, y1), canvas_center);
                let s2 = viewport.world_to_screen(Vec2::new(x2, y2), canvas_center);
                draw_dashed_line(painter, s1, s2, stroke, dash, gap);
                k += step;
            }

            // Slope -0.5 lines
            let k_min_neg = 2 * start_y + start_x;
            let k_max_neg = 2 * end_y + end_x;
            let k_start = k_min_neg - k_min_neg.rem_euclid(step);
            let mut k = k_start;
            while k <= k_max_neg {
                let x1 = start_x as f32 * gs;
                let x2 = end_x as f32 * gs;
                let y1 = -0.5 * x1 + k as f32 * gs / 2.0;
                let y2 = -0.5 * x2 + k as f32 * gs / 2.0;
                let s1 = viewport.world_to_screen(Vec2::new(x1, y1), canvas_center);
                let s2 = viewport.world_to_screen(Vec2::new(x2, y2), canvas_center);
                draw_dashed_line(painter, s1, s2, stroke, dash, gap);
                k += step;
            }
        }
    }
}
