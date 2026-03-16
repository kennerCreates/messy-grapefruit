use egui::{Color32, Painter, Pos2, Stroke};

use crate::engine::transform;
use crate::math;
use crate::model::project::{Palette, Theme};
use crate::model::sprite::{PathVertex, StrokeElement, Sprite};
use crate::model::vec2::Vec2;
use crate::state::editor::{HandleKind, ViewportState};
use crate::theme;

/// Render all visible elements in the sprite.
pub fn render_elements(
    painter: &Painter,
    viewport: &ViewportState,
    sprite: &Sprite,
    palette: &Palette,
    canvas_rect: egui::Rect,
) {
    let canvas_center = canvas_rect.center();

    for layer in &sprite.layers {
        if !layer.visible {
            continue;
        }
        for element in &layer.elements {
            let color = palette.get_color(element.stroke_color_index).to_color32();
            render_uniform_stroke(painter, element, color, viewport, canvas_center);
        }
    }
}

/// Render a stroke with uniform width, applying element transform.
fn render_uniform_stroke(
    painter: &Painter,
    element: &StrokeElement,
    color: Color32,
    viewport: &ViewportState,
    canvas_center: Pos2,
) {
    let stroke = Stroke::new(element.stroke_width * viewport.zoom, color);
    render_element_path(painter, element, stroke, viewport, canvas_center);
}

/// Render an element's path with a given stroke, applying element transforms.
fn render_element_path(
    painter: &Painter,
    element: &StrokeElement,
    stroke: Stroke,
    viewport: &ViewportState,
    canvas_center: Pos2,
) {
    if transform::has_transform(element) {
        let verts = transform::transformed_vertices(element);
        if element.curve_mode {
            render_curve_path(painter, &verts, element.closed, stroke, viewport, canvas_center);
        } else {
            render_rounded_path(painter, &verts, element.closed, stroke, viewport, canvas_center);
        }
    } else if element.curve_mode {
        render_curve_path(painter, &element.vertices, element.closed, stroke, viewport, canvas_center);
    } else {
        render_rounded_path(painter, &element.vertices, element.closed, stroke, viewport, canvas_center);
    }
}

/// Render a path using cubic bezier segments through vertex positions (curve mode).
fn render_curve_path(
    painter: &Painter,
    verts: &[PathVertex],
    closed: bool,
    stroke: Stroke,
    viewport: &ViewportState,
    canvas_center: Pos2,
) {
    for i in 0..verts.len().saturating_sub(1) {
        render_bezier_segment(painter, &verts[i], &verts[i + 1], stroke, viewport, canvas_center);
    }
    if closed && verts.len() >= 2 {
        let last = verts.len() - 1;
        render_bezier_segment(painter, &verts[last], &verts[0], stroke, viewport, canvas_center);
    }
}

/// Render a path with Figma-style corner fillets (straight mode with radius).
/// Flattens fillet arcs into a single continuous polyline for proper line joins.
fn render_rounded_path(
    painter: &Painter,
    verts: &[PathVertex],
    closed: bool,
    stroke: Stroke,
    viewport: &ViewportState,
    canvas_center: Pos2,
) {
    let n = verts.len();
    if n < 2 {
        return;
    }

    // Adaptive tolerance: ~0.5 screen pixels regardless of zoom
    let tolerance = (0.5 / viewport.zoom).max(0.01);
    let mut world_points: Vec<Vec2> = Vec::new();

    for v in verts {
        if let (Some(t1), Some(t2)) = (v.cp1, v.cp2) {
            // Flatten the fillet arc into polyline points
            let (arc_cp1, arc_cp2) = math::fillet_arc_control_points(t1, t2, v.pos);
            let mut arc = Vec::new();
            math::flatten_cubic_bezier(t1, arc_cp1, arc_cp2, t2, tolerance, &mut arc);
            world_points.extend_from_slice(&arc);
        } else {
            world_points.push(v.pos);
        }
    }

    if world_points.len() < 2 {
        return;
    }

    let screen_points: Vec<Pos2> = world_points
        .iter()
        .map(|p| viewport.world_to_screen(*p, canvas_center))
        .collect();

    painter.add(egui::Shape::Path(egui::epaint::PathShape {
        points: screen_points,
        closed,
        fill: Color32::TRANSPARENT,
        stroke: stroke.into(),
    }));
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
                render_element_path(painter, element, stroke, viewport, canvas_center);
                return;
            }
        }
    }
}

/// Render selection highlight for all selected elements.
pub fn render_selection_highlights(
    painter: &Painter,
    sprite: &Sprite,
    selected_ids: &[String],
    viewport: &ViewportState,
    canvas_rect: egui::Rect,
    theme_mode: Theme,
) {
    if selected_ids.is_empty() {
        return;
    }
    let canvas_center = canvas_rect.center();
    let highlight_color = theme::selection_highlight_color(theme_mode);

    for layer in &sprite.layers {
        for element in &layer.elements {
            if selected_ids.iter().any(|id| id == &element.id) {
                let stroke = Stroke::new((element.stroke_width + 6.0) * viewport.zoom, highlight_color);
                render_element_path(painter, element, stroke, viewport, canvas_center);
            }
        }
    }
}

/// Render the line tool preview (in-progress stroke + rubber band to cursor).
#[allow(clippy::too_many_arguments)]
pub fn render_line_tool_preview(
    painter: &Painter,
    vertices: &[PathVertex],
    snap_pos: Vec2,
    palette: &Palette,
    viewport: &ViewportState,
    canvas_rect: egui::Rect,
    color_index: u8,
    stroke_width: f32,
    theme_mode: Theme,
    merge_target: Option<Vec2>,
    curve_mode: bool,
) {
    let canvas_center = canvas_rect.center();
    let color = palette.get_color(color_index).to_color32();
    let stroke = Stroke::new(stroke_width * viewport.zoom, color);

    // Draw committed segments using the appropriate rendering path
    if curve_mode {
        render_curve_path(painter, vertices, false, stroke, viewport, canvas_center);
    } else {
        render_rounded_path(painter, vertices, false, stroke, viewport, canvas_center);
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

pub fn draw_dashed_line(
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

/// Compute screen-space handle positions for the 8 scale handles + rotation handle.
/// Returns (handle_kind, screen_pos) pairs.
pub fn compute_handle_positions(
    bounds_min: Vec2,
    bounds_max: Vec2,
    viewport: &ViewportState,
    canvas_center: Pos2,
) -> Vec<(HandleKind, Pos2)> {
    let mid_x = (bounds_min.x + bounds_max.x) * 0.5;
    let mid_y = (bounds_min.y + bounds_max.y) * 0.5;

    let world_points: [(HandleKind, Vec2); 8] = [
        (HandleKind::ScaleNW, Vec2::new(bounds_min.x, bounds_min.y)),
        (HandleKind::ScaleN,  Vec2::new(mid_x, bounds_min.y)),
        (HandleKind::ScaleNE, Vec2::new(bounds_max.x, bounds_min.y)),
        (HandleKind::ScaleE,  Vec2::new(bounds_max.x, mid_y)),
        (HandleKind::ScaleSE, Vec2::new(bounds_max.x, bounds_max.y)),
        (HandleKind::ScaleS,  Vec2::new(mid_x, bounds_max.y)),
        (HandleKind::ScaleSW, Vec2::new(bounds_min.x, bounds_max.y)),
        (HandleKind::ScaleW,  Vec2::new(bounds_min.x, mid_y)),
    ];

    let mut handles: Vec<(HandleKind, Pos2)> = world_points
        .iter()
        .map(|(kind, wp)| (*kind, viewport.world_to_screen(*wp, canvas_center)))
        .collect();

    // Rotation handle: 24px above top-center in screen space
    let top_center = viewport.world_to_screen(Vec2::new(mid_x, bounds_min.y), canvas_center);
    handles.push((HandleKind::Rotate, Pos2::new(top_center.x, top_center.y - 24.0)));

    handles
}

/// Render transform handles for the current selection.
pub fn render_transform_handles(
    painter: &Painter,
    sprite: &Sprite,
    selected_ids: &[String],
    viewport: &ViewportState,
    canvas_rect: egui::Rect,
    theme_mode: Theme,
) {
    if selected_ids.is_empty() {
        return;
    }
    let bounds = match transform::selection_bounds(sprite, selected_ids) {
        Some(b) => b,
        None => return,
    };
    let canvas_center = canvas_rect.center();
    let handle_color = theme::handle_color(theme_mode);
    let handles = compute_handle_positions(bounds.0, bounds.1, viewport, canvas_center);

    // Draw bounding box outline (dashed)
    let tl = viewport.world_to_screen(bounds.0, canvas_center);
    let tr = viewport.world_to_screen(Vec2::new(bounds.1.x, bounds.0.y), canvas_center);
    let br = viewport.world_to_screen(bounds.1, canvas_center);
    let bl = viewport.world_to_screen(Vec2::new(bounds.0.x, bounds.1.y), canvas_center);
    let box_stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(
        handle_color.r(), handle_color.g(), handle_color.b(), 100,
    ));
    draw_dashed_line(painter, tl, tr, box_stroke, 4.0, 3.0);
    draw_dashed_line(painter, tr, br, box_stroke, 4.0, 3.0);
    draw_dashed_line(painter, br, bl, box_stroke, 4.0, 3.0);
    draw_dashed_line(painter, bl, tl, box_stroke, 4.0, 3.0);

    // Draw rotation arm (line from top-center to rotation handle)
    let top_center = viewport.world_to_screen(
        Vec2::new((bounds.0.x + bounds.1.x) * 0.5, bounds.0.y),
        canvas_center,
    );
    let rot_handle_pos = handles.iter().find(|(k, _)| *k == HandleKind::Rotate).map(|(_, p)| *p);
    if let Some(rot_pos) = rot_handle_pos {
        painter.line_segment([top_center, rot_pos], box_stroke);
        // Rotation handle: circle
        painter.circle_filled(rot_pos, 4.0, handle_color);
        painter.circle_stroke(rot_pos, 4.0, Stroke::new(1.0, handle_color));
    }

    // Draw scale handles: small filled squares
    let half = 3.5;
    for (kind, pos) in &handles {
        if *kind == HandleKind::Rotate {
            continue; // already drawn above
        }
        let rect = egui::Rect::from_center_size(*pos, egui::Vec2::splat(half * 2.0));
        painter.rect_filled(rect, 0.0, handle_color);
    }
}

/// Hit-test transform handles. Returns the handle kind if the screen position is within
/// `radius` pixels of any handle.
pub fn hit_test_handles(
    screen_pos: Pos2,
    sprite: &Sprite,
    selected_ids: &[String],
    viewport: &ViewportState,
    canvas_rect: egui::Rect,
    radius: f32,
) -> Option<HandleKind> {
    if selected_ids.is_empty() {
        return None;
    }
    let bounds = transform::selection_bounds(sprite, selected_ids)?;
    let canvas_center = canvas_rect.center();
    let handles = compute_handle_positions(bounds.0, bounds.1, viewport, canvas_center);

    let radius_sq = radius * radius;
    for (kind, pos) in &handles {
        let dx = screen_pos.x - pos.x;
        let dy = screen_pos.y - pos.y;
        if dx * dx + dy * dy <= radius_sq {
            return Some(*kind);
        }
    }
    None
}

/// Get the cursor icon for a given handle kind.
pub fn cursor_for_handle(handle: HandleKind) -> egui::CursorIcon {
    match handle {
        HandleKind::ScaleN | HandleKind::ScaleS => egui::CursorIcon::ResizeVertical,
        HandleKind::ScaleE | HandleKind::ScaleW => egui::CursorIcon::ResizeHorizontal,
        HandleKind::ScaleNW | HandleKind::ScaleSE => egui::CursorIcon::ResizeNwSe,
        HandleKind::ScaleNE | HandleKind::ScaleSW => egui::CursorIcon::ResizeNeSw,
        HandleKind::Rotate => egui::CursorIcon::Alias,
    }
}
