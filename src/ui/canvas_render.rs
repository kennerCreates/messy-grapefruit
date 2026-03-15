use egui::{Color32, Painter, Pos2, Stroke};

use crate::math;
use crate::model::project::{Palette, Theme};
use crate::model::sprite::{PathVertex, StrokeElement, Sprite};
use crate::model::vec2::Vec2;
use crate::state::editor::ViewportState;
use crate::theme;

/// Render all visible elements in the sprite.
pub fn render_elements(
    painter: &Painter,
    viewport: &ViewportState,
    sprite: &Sprite,
    palette: &Palette,
    canvas_rect: egui::Rect,
    taper_enabled: bool,
    _theme_mode: Theme,
) {
    let canvas_center = canvas_rect.center();

    for layer in &sprite.layers {
        if !layer.visible {
            continue;
        }
        for element in &layer.elements {
            let color = palette.get_color(element.stroke_color_index).to_color32();
            let should_taper = match element.taper_override {
                Some(v) => v,
                None => taper_enabled,
            };

            if should_taper && !element.closed && element.vertices.len() >= 2 {
                render_tapered_stroke(painter, element, color, viewport, canvas_center);
            } else {
                render_uniform_stroke(painter, element, color, viewport, canvas_center);
            }
        }
    }
}

/// Render a stroke with uniform width.
fn render_uniform_stroke(
    painter: &Painter,
    element: &StrokeElement,
    color: Color32,
    viewport: &ViewportState,
    canvas_center: Pos2,
) {
    let stroke = Stroke::new(element.stroke_width * viewport.zoom, color);
    let verts = &element.vertices;

    for i in 0..verts.len().saturating_sub(1) {
        render_bezier_segment(painter, &verts[i], &verts[i + 1], stroke, viewport, canvas_center);
    }

    if element.closed && verts.len() >= 2 {
        let last = verts.len() - 1;
        render_bezier_segment(painter, &verts[last], &verts[0], stroke, viewport, canvas_center);
    }
}

fn render_bezier_segment(
    painter: &Painter,
    v0: &PathVertex,
    v1: &PathVertex,
    stroke: Stroke,
    viewport: &ViewportState,
    canvas_center: Pos2,
) {
    let (p0, cp1, cp2, p3) = math::segment_bezier_points(v0, v1);
    let s0 = viewport.world_to_screen(p0, canvas_center);
    let s1 = viewport.world_to_screen(cp1, canvas_center);
    let s2 = viewport.world_to_screen(cp2, canvas_center);
    let s3 = viewport.world_to_screen(p3, canvas_center);

    painter.add(egui::Shape::CubicBezier(egui::epaint::CubicBezierShape {
        points: [s0, s1, s2, s3],
        closed: false,
        fill: Color32::TRANSPARENT,
        stroke: stroke.into(),
    }));
}

/// Render a stroke with parabolic taper (zero width at endpoints, full at center).
fn render_tapered_stroke(
    painter: &Painter,
    element: &StrokeElement,
    color: Color32,
    viewport: &ViewportState,
    canvas_center: Pos2,
) {
    // Flatten all segments into a single polyline
    let mut points = Vec::new();
    let verts = &element.vertices;

    for i in 0..verts.len().saturating_sub(1) {
        let (p0, cp1, cp2, p3) = math::segment_bezier_points(&verts[i], &verts[i + 1]);
        let start_idx = points.len();
        math::flatten_cubic_bezier(p0, cp1, cp2, p3, 0.5, &mut points);
        // Remove duplicate start point for subsequent segments
        if start_idx > 0 && points.len() > start_idx {
            points.remove(start_idx);
        }
    }

    if points.len() < 2 {
        return;
    }

    // Compute cumulative arc lengths
    let arc_lengths = math::cumulative_arc_lengths(&points);
    let total_length = *arc_lengths.last().unwrap_or(&0.0);
    if total_length < 0.001 {
        return;
    }

    // Draw each polyline segment with varying width
    for i in 0..points.len() - 1 {
        let t = (arc_lengths[i] + arc_lengths[i + 1]) / (2.0 * total_length);
        let width = element.stroke_width * (1.0 - (2.0 * t - 1.0).powi(2));
        let screen_width = (width * viewport.zoom).max(0.5);

        let s0 = viewport.world_to_screen(points[i], canvas_center);
        let s1 = viewport.world_to_screen(points[i + 1], canvas_center);

        if width > 0.01 {
            painter.line_segment([s0, s1], Stroke::new(screen_width, color));
        }
    }
}

/// Render hover highlight for an element.
pub fn render_hover_highlight(
    painter: &Painter,
    sprite: &Sprite,
    element_id: &str,
    viewport: &ViewportState,
    canvas_rect: egui::Rect,
    theme_mode: Theme,
) {
    let canvas_center = canvas_rect.center();
    let highlight_color = theme::hover_highlight_color(theme_mode);

    for layer in &sprite.layers {
        for element in &layer.elements {
            if element.id == element_id {
                let stroke = Stroke::new((element.stroke_width + 4.0) * viewport.zoom, highlight_color);
                for i in 0..element.vertices.len().saturating_sub(1) {
                    render_bezier_segment(
                        painter,
                        &element.vertices[i],
                        &element.vertices[i + 1],
                        stroke,
                        viewport,
                        canvas_center,
                    );
                }
                return;
            }
        }
    }
}

/// Render the line tool preview (in-progress stroke + rubber band to cursor).
#[allow(clippy::too_many_arguments)]
pub fn render_line_tool_preview(
    painter: &Painter,
    vertices: &[PathVertex],
    _cursor_world: Vec2,
    snap_pos: Vec2,
    palette: &Palette,
    viewport: &ViewportState,
    canvas_rect: egui::Rect,
    color_index: u8,
    stroke_width: f32,
    theme_mode: Theme,
    merge_target: Option<Vec2>,
) {
    let canvas_center = canvas_rect.center();
    let color = palette.get_color(color_index).to_color32();
    let stroke = Stroke::new(stroke_width * viewport.zoom, color);

    // Draw committed segments
    for i in 0..vertices.len().saturating_sub(1) {
        render_bezier_segment(painter, &vertices[i], &vertices[i + 1], stroke, viewport, canvas_center);
    }

    // Draw rubber band preview to cursor
    if let Some(last) = vertices.last() {
        let preview_color = theme::rubber_band_color(theme_mode);
        let preview_stroke = Stroke::new(stroke_width * viewport.zoom, preview_color);

        let s_last = viewport.world_to_screen(last.pos, canvas_center);
        let s_cursor = viewport.world_to_screen(snap_pos, canvas_center);
        painter.line_segment([s_last, s_cursor], preview_stroke);
    }

    // Draw vertex dots
    for v in vertices {
        let screen = viewport.world_to_screen(v.pos, canvas_center);
        painter.circle_filled(screen, 3.0, color);
    }

    // Draw snap cursor
    let snap_screen = viewport.world_to_screen(snap_pos, canvas_center);
    painter.circle_stroke(snap_screen, 4.0, Stroke::new(1.0, color));

    // Draw merge target indicator
    if let Some(merge_pos) = merge_target {
        let merge_screen = viewport.world_to_screen(merge_pos, canvas_center);
        let merge_color = theme::merge_preview_color(theme_mode);
        painter.circle_stroke(merge_screen, 8.0, Stroke::new(2.0, merge_color));
        painter.circle_stroke(merge_screen, 4.0, Stroke::new(2.0, merge_color));
    }
}

/// Render the canvas boundary (dashed rectangle).
pub fn render_canvas_boundary(
    painter: &Painter,
    viewport: &ViewportState,
    canvas_width: u32,
    canvas_height: u32,
    canvas_rect: egui::Rect,
    theme_mode: Theme,
) {
    let canvas_center = canvas_rect.center();
    let color = theme::canvas_boundary_color(theme_mode);
    let stroke = Stroke::new(1.0, color);

    let tl = viewport.world_to_screen(Vec2::ZERO, canvas_center);
    let tr = viewport.world_to_screen(Vec2::new(canvas_width as f32, 0.0), canvas_center);
    let br = viewport.world_to_screen(Vec2::new(canvas_width as f32, canvas_height as f32), canvas_center);
    let bl = viewport.world_to_screen(Vec2::new(0.0, canvas_height as f32), canvas_center);

    // Draw dashed lines (series of short segments)
    draw_dashed_line(painter, tl, tr, stroke, 6.0, 4.0);
    draw_dashed_line(painter, tr, br, stroke, 6.0, 4.0);
    draw_dashed_line(painter, br, bl, stroke, 6.0, 4.0);
    draw_dashed_line(painter, bl, tl, stroke, 6.0, 4.0);
}

fn draw_dashed_line(
    painter: &Painter,
    from: Pos2,
    to: Pos2,
    stroke: Stroke,
    dash_len: f32,
    gap_len: f32,
) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let total_len = (dx * dx + dy * dy).sqrt();
    if total_len < 0.001 {
        return;
    }
    let dir_x = dx / total_len;
    let dir_y = dy / total_len;
    let cycle = dash_len + gap_len;

    let mut dist = 0.0;
    while dist < total_len {
        let end_dist = (dist + dash_len).min(total_len);
        let p1 = Pos2::new(from.x + dir_x * dist, from.y + dir_y * dist);
        let p2 = Pos2::new(from.x + dir_x * end_dist, from.y + dir_y * end_dist);
        painter.line_segment([p1, p2], stroke);
        dist += cycle;
    }
}
