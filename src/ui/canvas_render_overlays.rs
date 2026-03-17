use egui::{Color32, Painter, Pos2, Stroke};

use crate::engine::animation::CanvasAnimState;
use crate::engine::transform;
use crate::model::project::Theme;
use crate::model::sprite::StrokeElement;
use crate::model::vec2::Vec2;
use crate::state::editor::{HandleKind, VertexHover, ViewportState};
use crate::theme;

/// Canvas boundary dashed line pattern.
const BOUNDARY_DASH: f32 = 6.0;
const BOUNDARY_GAP: f32 = 4.0;
/// Selection bounding box dashed line pattern.
const SELECTION_BOX_DASH: f32 = 4.0;
const SELECTION_BOX_GAP: f32 = 3.0;
/// Vertex dot rendering radii (screen pixels).
const VERTEX_DOT_RADIUS: f32 = 4.0;
const VERTEX_SELECTED_RADIUS: f32 = 5.5;
const CP_HANDLE_RADIUS: f32 = 3.5;
/// Screen-pixel offset from top-center to rotation handle.
const ROTATION_HANDLE_OFFSET: f32 = 24.0;
/// Half-size of scale handle squares (screen pixels).
const SCALE_HANDLE_HALF_SIZE: f32 = 3.5;
/// Radius of the rotation handle circle (screen pixels).
const ROTATION_HANDLE_RADIUS: f32 = 4.0;
/// Hit radius for vertex/handle picking (screen pixels).
pub const VERTEX_HIT_RADIUS: f32 = 8.0;

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
    let br = viewport.world_to_screen(
        Vec2::new(canvas_width as f32, canvas_height as f32), canvas_center,
    );
    let bl = viewport.world_to_screen(Vec2::new(0.0, canvas_height as f32), canvas_center);

    draw_dashed_line(painter, tl, tr, stroke, BOUNDARY_DASH, BOUNDARY_GAP);
    draw_dashed_line(painter, tr, br, stroke, BOUNDARY_DASH, BOUNDARY_GAP);
    draw_dashed_line(painter, br, bl, stroke, BOUNDARY_DASH, BOUNDARY_GAP);
    draw_dashed_line(painter, bl, tl, stroke, BOUNDARY_DASH, BOUNDARY_GAP);
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

    let top_center = viewport.world_to_screen(Vec2::new(mid_x, bounds_min.y), canvas_center);
    handles.push((HandleKind::Rotate, Pos2::new(top_center.x, top_center.y - ROTATION_HANDLE_OFFSET)));

    handles
}

/// Render transform handles for the current selection.
pub fn render_transform_handles(
    painter: &Painter,
    sprite: &crate::model::sprite::Sprite,
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

    let tl = viewport.world_to_screen(bounds.0, canvas_center);
    let tr = viewport.world_to_screen(Vec2::new(bounds.1.x, bounds.0.y), canvas_center);
    let br = viewport.world_to_screen(bounds.1, canvas_center);
    let bl = viewport.world_to_screen(Vec2::new(bounds.0.x, bounds.1.y), canvas_center);
    let box_stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(
        handle_color.r(), handle_color.g(), handle_color.b(), 100,
    ));
    draw_dashed_line(painter, tl, tr, box_stroke, SELECTION_BOX_DASH, SELECTION_BOX_GAP);
    draw_dashed_line(painter, tr, br, box_stroke, SELECTION_BOX_DASH, SELECTION_BOX_GAP);
    draw_dashed_line(painter, br, bl, box_stroke, SELECTION_BOX_DASH, SELECTION_BOX_GAP);
    draw_dashed_line(painter, bl, tl, box_stroke, SELECTION_BOX_DASH, SELECTION_BOX_GAP);

    let top_center = viewport.world_to_screen(
        Vec2::new((bounds.0.x + bounds.1.x) * 0.5, bounds.0.y),
        canvas_center,
    );
    let rot_handle_pos = handles.iter().find(|(k, _)| *k == HandleKind::Rotate).map(|(_, p)| *p);
    if let Some(rot_pos) = rot_handle_pos {
        painter.line_segment([top_center, rot_pos], box_stroke);
        painter.circle_filled(rot_pos, ROTATION_HANDLE_RADIUS, handle_color);
        painter.circle_stroke(rot_pos, ROTATION_HANDLE_RADIUS, Stroke::new(1.0, handle_color));
    }

    for (kind, pos) in &handles {
        if *kind == HandleKind::Rotate {
            continue;
        }
        let rect = egui::Rect::from_center_size(*pos, egui::Vec2::splat(SCALE_HANDLE_HALF_SIZE * 2.0));
        painter.rect_filled(rect, 0.0, handle_color);
    }
}

/// Hit-test transform handles. Returns the handle kind if the screen position is within
/// `radius` pixels of any handle.
pub fn hit_test_handles(
    screen_pos: Pos2,
    sprite: &crate::model::sprite::Sprite,
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

/// Render vertex dots for the selected element in vertex-edit mode.
pub fn render_vertex_dots(
    painter: &Painter,
    element: &StrokeElement,
    selected_vertex_id: Option<&str>,
    hover_vertex: Option<&VertexHover>,
    viewport: &ViewportState,
    canvas_center: Pos2,
    theme_mode: Theme,
) {
    let tc = theme::theme_colors(theme_mode);

    for v in &element.vertices {
        let world = transform::vertex_world_pos(v, element);
        let screen = viewport.world_to_screen(world, canvas_center);

        let is_selected = selected_vertex_id == Some(v.id.as_str());
        let is_hovered = matches!(hover_vertex, Some(VertexHover::Vertex { vertex_id }) if vertex_id == &v.id);

        let (radius, color) = if is_selected {
            (VERTEX_SELECTED_RADIUS, tc.selected)
        } else if is_hovered {
            (VERTEX_DOT_RADIUS + 1.0, tc.icon_text)
        } else {
            (VERTEX_DOT_RADIUS, tc.mid)
        };

        painter.circle_filled(screen, radius, color);
        painter.circle_stroke(screen, radius, Stroke::new(1.0, Color32::BLACK));
    }
}

/// Render control point handles (tangent lines + handle dots) for the selected vertex.
pub fn render_cp_handles(
    painter: &Painter,
    element: &StrokeElement,
    vertex_id: &str,
    hover_vertex: Option<&VertexHover>,
    viewport: &ViewportState,
    canvas_center: Pos2,
    theme_mode: Theme,
) {
    let tc = theme::theme_colors(theme_mode);
    let Some(vertex) = element.vertices.iter().find(|v| v.id == vertex_id) else { return };

    let v_world = transform::vertex_world_pos(vertex, element);
    let v_screen = viewport.world_to_screen(v_world, canvas_center);

    let handle_line_stroke = Stroke::new(1.0, tc.mid);

    for (cp_opt, is_cp1) in [(vertex.cp1, true), (vertex.cp2, false)] {
        if let Some(cp) = cp_opt {
            let cp_world = transform::transform_point(
                cp, element.origin, element.position, element.rotation, element.scale,
            );
            let cp_screen = viewport.world_to_screen(cp_world, canvas_center);

            painter.line_segment([v_screen, cp_screen], handle_line_stroke);

            let is_hovered = matches!(hover_vertex,
                Some(VertexHover::Handle { vertex_id: vid, is_cp1: c }) if vid == vertex_id && *c == is_cp1);
            let radius = if is_hovered { CP_HANDLE_RADIUS + 1.0 } else { CP_HANDLE_RADIUS };
            let color = if is_hovered { tc.icon_text } else { tc.mid };

            painter.circle_filled(cp_screen, radius, color);
            painter.circle_stroke(cp_screen, radius, Stroke::new(1.0, Color32::BLACK));
        }
    }
}

/// Render a vertex snap indicator (blue diamond ring) at the snap target.
pub fn render_vertex_snap_indicator(
    painter: &Painter,
    viewport: &ViewportState,
    world_pos: Vec2,
    canvas_rect: egui::Rect,
    theme_mode: Theme,
) {
    let canvas_center = canvas_rect.center();
    let screen = viewport.world_to_screen(world_pos, canvas_center);
    let color = theme::vertex_snap_color(theme_mode);

    let size = 8.0;
    let points = [
        Pos2::new(screen.x, screen.y - size),
        Pos2::new(screen.x + size, screen.y),
        Pos2::new(screen.x, screen.y + size),
        Pos2::new(screen.x - size, screen.y),
    ];
    painter.add(egui::Shape::convex_polygon(
        points.to_vec(),
        Color32::TRANSPARENT,
        Stroke::new(2.0, color),
    ));
    painter.circle_filled(screen, 3.0, color);
}

/// Render symmetry axis line(s) on the canvas.
pub fn render_symmetry_axis(
    painter: &Painter,
    viewport: &ViewportState,
    symmetry: &crate::state::editor::SymmetryState,
    sprite: &crate::model::sprite::Sprite,
    canvas_rect: egui::Rect,
    theme_mode: Theme,
) {
    let canvas_center = canvas_rect.center();
    let color = theme::symmetry_axis_color(theme_mode);
    let stroke = Stroke::new(1.5, color);
    let cw = sprite.canvas_width as f32;
    let ch = sprite.canvas_height as f32;

    match symmetry.axis {
        crate::state::editor::SymmetryAxis::Vertical | crate::state::editor::SymmetryAxis::Both => {
            let x = symmetry.axis_position.x;
            let top = viewport.world_to_screen(Vec2::new(x, -ch), canvas_center);
            let bot = viewport.world_to_screen(Vec2::new(x, ch * 2.0), canvas_center);
            draw_dashed_line(painter, top, bot, stroke, 8.0, 4.0);
        }
        _ => {}
    }
    match symmetry.axis {
        crate::state::editor::SymmetryAxis::Horizontal | crate::state::editor::SymmetryAxis::Both => {
            let y = symmetry.axis_position.y;
            let left = viewport.world_to_screen(Vec2::new(-cw, y), canvas_center);
            let right = viewport.world_to_screen(Vec2::new(cw * 2.0, y), canvas_center);
            draw_dashed_line(painter, left, right, stroke, 8.0, 4.0);
        }
        _ => {}
    }
}

/// Render symmetry ghost preview of in-progress line tool stroke.
#[allow(clippy::too_many_arguments)]
pub fn render_symmetry_ghost(
    painter: &Painter,
    vertices: &[crate::model::sprite::PathVertex],
    cursor_snap_pos: Vec2,
    symmetry: &crate::state::editor::SymmetryState,
    viewport: &ViewportState,
    canvas_rect: egui::Rect,
    stroke_width: f32,
    theme_mode: Theme,
) {
    use crate::engine::symmetry as sym;
    use crate::state::editor::SymmetryAxis;

    let canvas_center = canvas_rect.center();
    let ghost_color = theme::symmetry_ghost_color(theme_mode);
    let ghost_stroke = Stroke::new(stroke_width * viewport.zoom, ghost_color);

    let axes = match symmetry.axis {
        SymmetryAxis::Vertical => vec![SymmetryAxis::Vertical],
        SymmetryAxis::Horizontal => vec![SymmetryAxis::Horizontal],
        SymmetryAxis::Both => vec![SymmetryAxis::Vertical, SymmetryAxis::Horizontal, SymmetryAxis::Both],
    };

    for axis in axes {
        let mut screen_points: Vec<Pos2> = Vec::new();
        for v in vertices {
            let mirrored = sym::mirror_point(v.pos, axis, &symmetry.axis_position);
            screen_points.push(viewport.world_to_screen(mirrored, canvas_center));
        }
        let mirrored_cursor = sym::mirror_point(cursor_snap_pos, axis, &symmetry.axis_position);
        screen_points.push(viewport.world_to_screen(mirrored_cursor, canvas_center));

        if screen_points.len() >= 2 {
            for i in 0..screen_points.len() - 1 {
                painter.line_segment([screen_points[i], screen_points[i + 1]], ghost_stroke);
            }
        }
    }
}

/// Render reference images behind all layers.
pub fn render_reference_images(
    painter: &Painter,
    viewport: &ViewportState,
    sprite: &crate::model::sprite::Sprite,
    ref_textures: &std::collections::HashMap<String, egui::TextureHandle>,
    canvas_rect: egui::Rect,
    selected_ref_id: Option<&str>,
    theme_mode: Theme,
) {
    let canvas_center = canvas_rect.center();

    for ref_img in &sprite.reference_images {
        if !ref_img.visible {
            continue;
        }
        let tex = match ref_textures.get(&ref_img.id) {
            Some(t) => t,
            None => continue,
        };
        let (w, h) = ref_img.image_size.unwrap_or((
            tex.size()[0] as u32,
            tex.size()[1] as u32,
        ));

        let world_min = ref_img.position;
        let world_max = Vec2::new(
            ref_img.position.x + w as f32 * ref_img.scale,
            ref_img.position.y + h as f32 * ref_img.scale,
        );

        let screen_min = viewport.world_to_screen(world_min, canvas_center);
        let screen_max = viewport.world_to_screen(world_max, canvas_center);
        let rect = egui::Rect::from_two_pos(screen_min, screen_max);

        let alpha = (ref_img.opacity * 255.0) as u8;
        let tint = Color32::from_rgba_unmultiplied(255, 255, 255, alpha);

        painter.image(
            tex.id(),
            rect,
            egui::Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            tint,
        );

        if selected_ref_id == Some(ref_img.id.as_str()) {
            let border_color = theme::selection_highlight_color(theme_mode);
            draw_dashed_line(painter, rect.left_top(), rect.right_top(), Stroke::new(1.0, border_color), 4.0, 3.0);
            draw_dashed_line(painter, rect.right_top(), rect.right_bottom(), Stroke::new(1.0, border_color), 4.0, 3.0);
            draw_dashed_line(painter, rect.right_bottom(), rect.left_bottom(), Stroke::new(1.0, border_color), 4.0, 3.0);
            draw_dashed_line(painter, rect.left_bottom(), rect.left_top(), Stroke::new(1.0, border_color), 4.0, 3.0);
        }
    }
}

/// Render the canvas state indicator: a thin colored line along the top of the canvas panel.
///
/// - `Rest` → no border
/// - `OnKeyframe` → green (3px)
/// - `Interpolated` → orange (3px)
pub fn render_canvas_state_border(
    painter: &Painter,
    canvas_rect: egui::Rect,
    state: &CanvasAnimState,
) {
    let color = match state {
        CanvasAnimState::Rest => return,
        CanvasAnimState::OnKeyframe(_) => Color32::from_rgb(80, 200, 100),
        CanvasAnimState::Interpolated => Color32::from_rgb(220, 140, 60),
    };
    let y = canvas_rect.top() + 1.5;
    painter.line_segment(
        [
            Pos2::new(canvas_rect.left(), y),
            Pos2::new(canvas_rect.right(), y),
        ],
        Stroke::new(3.0, color),
    );
}
