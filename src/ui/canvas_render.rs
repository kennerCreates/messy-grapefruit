use egui::{Color32, Painter, Pos2, Stroke};

use crate::engine::transform;
use crate::math;
use crate::model::project::{HatchPattern, Palette, Theme};
use crate::model::sprite::{GradientType, PathVertex, StrokeElement, Sprite};
use crate::model::vec2::Vec2;
use crate::state::editor::{HandleKind, VertexHover, ViewportState};
use crate::theme;

/// Curve flattening: target screen-pixel tolerance.
const FLATTEN_TOLERANCE_PX: f32 = 0.5;
/// Minimum world-space flattening tolerance (prevents infinite subdivision).
const FLATTEN_MIN_TOLERANCE: f32 = 0.01;
/// Extra stroke width added for hover highlight (world units).
const HOVER_HIGHLIGHT_EXTRA: f32 = 4.0;
/// Extra stroke width added for selection highlight (world units).
const SELECTION_HIGHLIGHT_EXTRA: f32 = 6.0;
/// Screen-pixel offset from top-center to rotation handle.
const ROTATION_HANDLE_OFFSET: f32 = 24.0;
/// Half-size of scale handle squares (screen pixels).
const SCALE_HANDLE_HALF_SIZE: f32 = 3.5;
/// Radius of the rotation handle circle (screen pixels).
const ROTATION_HANDLE_RADIUS: f32 = 4.0;
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
/// Hit radius for vertex/handle picking (screen pixels).
pub const VERTEX_HIT_RADIUS: f32 = 8.0;

/// Fill rendering info passed through the render pipeline.
#[derive(Clone)]
enum FillInfo {
    Flat(Color32),
    Gradient {
        color_start: Color32,
        color_end: Color32,
        gradient_type: GradientType,
        angle_rad: f32,
        /// Element AABB in screen space (min, max).
        bounds_min: Pos2,
        bounds_max: Pos2,
        /// Radial center, normalized 0..1 within element AABB.
        center: (f32, f32),
        /// Radial radius, normalized 0..1 of the AABB diagonal.
        radius: f32,
        /// Sharpness: 1.0 = linear, >1 = sharper, <1 = softer.
        sharpness: f32,
    },
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let inv = 1.0 - t;
    Color32::from_rgba_unmultiplied(
        (a.r() as f32 * inv + b.r() as f32 * t) as u8,
        (a.g() as f32 * inv + b.g() as f32 * t) as u8,
        (a.b() as f32 * inv + b.b() as f32 * t) as u8,
        (a.a() as f32 * inv + b.a() as f32 * t) as u8,
    )
}

fn gradient_color_at(pos: Pos2, fill: &FillInfo) -> Color32 {
    match fill {
        FillInfo::Flat(c) => *c,
        FillInfo::Gradient {
            color_start, color_end, gradient_type, angle_rad,
            bounds_min, bounds_max, center, radius, sharpness,
        } => {
            match gradient_type {
                GradientType::Linear => {
                    let dir_x = angle_rad.cos();
                    let dir_y = angle_rad.sin();
                    let corners = [
                        (bounds_min.x, bounds_min.y),
                        (bounds_max.x, bounds_min.y),
                        (bounds_min.x, bounds_max.y),
                        (bounds_max.x, bounds_max.y),
                    ];
                    let mut proj_min = f32::MAX;
                    let mut proj_max = f32::MIN;
                    for (cx, cy) in corners {
                        let p = cx * dir_x + cy * dir_y;
                        proj_min = proj_min.min(p);
                        proj_max = proj_max.max(p);
                    }
                    let extent = proj_max - proj_min;
                    if extent < 0.001 {
                        return *color_start;
                    }
                    let proj = pos.x * dir_x + pos.y * dir_y;
                    let t = ((proj - proj_min) / extent).clamp(0.0, 1.0).powf(*sharpness);
                    lerp_color(*color_start, *color_end, t)
                }
                GradientType::Radial => {
                    let cx = bounds_min.x + center.0 * (bounds_max.x - bounds_min.x);
                    let cy = bounds_min.y + center.1 * (bounds_max.y - bounds_min.y);
                    let dx = bounds_max.x - bounds_min.x;
                    let dy = bounds_max.y - bounds_min.y;
                    let max_dist = radius * (dx.max(dy) * 0.5);
                    if max_dist < 0.001 {
                        return *color_start;
                    }
                    let dist = ((pos.x - cx).powi(2) + (pos.y - cy).powi(2)).sqrt();
                    let t = (dist / max_dist).clamp(0.0, 1.0).powf(*sharpness);
                    lerp_color(*color_start, *color_end, t)
                }
            }
        }
    }
}

/// Compute screen-space AABB for an element's vertices.
fn element_screen_bounds(
    element: &StrokeElement,
    viewport: &ViewportState,
    canvas_center: Pos2,
) -> (Pos2, Pos2) {
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;

    let verts = if transform::has_transform(element) {
        transform::transformed_vertices(element)
    } else {
        element.vertices.clone()
    };

    for v in &verts {
        let screen = viewport.world_to_screen(v.pos, canvas_center);
        min_x = min_x.min(screen.x);
        min_y = min_y.min(screen.y);
        max_x = max_x.max(screen.x);
        max_y = max_y.max(screen.y);
    }
    (Pos2::new(min_x, min_y), Pos2::new(max_x, max_y))
}

/// Resolve the FillInfo for an element based on its fill properties.
fn resolve_fill_info(
    element: &StrokeElement,
    palette: &Palette,
    viewport: &ViewportState,
    canvas_center: Pos2,
    dim_alpha: Option<f32>,
) -> FillInfo {
    let apply_dim = |c: Color32| -> Color32 {
        if let Some(alpha) = dim_alpha {
            Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (c.a() as f32 * alpha) as u8)
        } else {
            c
        }
    };

    if let Some(ref grad) = element.gradient_fill {
        if element.closed {
            let (bmin, bmax) = element_screen_bounds(element, viewport, canvas_center);
            let cs = apply_dim(palette.get_color(grad.color_index_start).to_color32());
            let ce = apply_dim(palette.get_color(grad.color_index_end).to_color32());
            let center = grad.center.map(|c| (c.x, c.y)).unwrap_or((0.5, 0.5));
            let radius = grad.radius.unwrap_or(0.5);
            return FillInfo::Gradient {
                color_start: cs,
                color_end: ce,
                gradient_type: grad.gradient_type,
                angle_rad: grad.alignment.to_radians(),
                bounds_min: bmin,
                bounds_max: bmax,
                center,
                radius,
                sharpness: grad.sharpness,
            };
        }
    }

    // Flat fill
    let fill_color = if element.closed && element.fill_color_index != 0 {
        apply_dim(palette.get_color(element.fill_color_index).to_color32())
    } else {
        Color32::TRANSPARENT
    };
    FillInfo::Flat(fill_color)
}

/// Render the sprite background color within the canvas boundary area.
pub fn render_background(
    painter: &Painter,
    viewport: &ViewportState,
    sprite: &Sprite,
    palette: &Palette,
    canvas_rect: egui::Rect,
) {
    let bg = palette.get_color(sprite.background_color_index);
    if bg.a == 0 {
        return; // transparent = no background fill
    }
    let canvas_center = canvas_rect.center();
    let tl = viewport.world_to_screen(Vec2::ZERO, canvas_center);
    let br = viewport.world_to_screen(
        Vec2::new(sprite.canvas_width as f32, sprite.canvas_height as f32),
        canvas_center,
    );
    let rect = egui::Rect::from_min_max(
        Pos2::new(tl.x.min(br.x), tl.y.min(br.y)),
        Pos2::new(tl.x.max(br.x), tl.y.max(br.y)),
    );
    painter.rect_filled(rect, 0.0, bg.to_color32());
}

/// Render all visible elements in the sprite.
/// When `solo_layer_id` is set, the soloed layer renders at full opacity,
/// while other layers are dimmed to ~15%.
pub fn render_elements(
    painter: &Painter,
    viewport: &ViewportState,
    sprite: &Sprite,
    palette: &Palette,
    canvas_rect: egui::Rect,
    solo_layer_id: Option<&str>,
    hatch_patterns: &[HatchPattern],
) {
    let canvas_center = canvas_rect.center();

    for layer in &sprite.layers {
        if !layer.visible {
            continue;
        }
        let is_dimmed = solo_layer_id.is_some_and(|sid| sid != layer.id);
        let dim_alpha = if is_dimmed { Some(0.15) } else { None };
        for element in &layer.elements {
            let mut color = palette.get_color(element.stroke_color_index).to_color32();
            if is_dimmed {
                color = Color32::from_rgba_unmultiplied(
                    color.r(), color.g(), color.b(),
                    (color.a() as f32 * 0.15) as u8,
                );
            }

            let fill_info = resolve_fill_info(element, palette, viewport, canvas_center, dim_alpha);
            render_uniform_stroke(painter, element, color, &fill_info, viewport, canvas_center);

            // Render hatch fill lines on top of the fill mesh, under the stroke
            if let Some(ref hatch_id) = element.hatch_fill_id {
                if let Some(pattern) = hatch_patterns.iter().find(|p| p.id == *hatch_id) {
                    render_hatch_fill(
                        painter, element, pattern, palette, viewport,
                        canvas_center, dim_alpha,
                    );
                }
            }
        }
    }
}

/// Render a stroke with uniform width, applying element transform.
fn render_uniform_stroke(
    painter: &Painter,
    element: &StrokeElement,
    color: Color32,
    fill_info: &FillInfo,
    viewport: &ViewportState,
    canvas_center: Pos2,
) {
    let stroke = Stroke::new(element.stroke_width * viewport.zoom, color);
    render_element_path(painter, element, stroke, fill_info, viewport, canvas_center);
}

/// Render an element's path with a given stroke, applying element transforms.
fn render_element_path(
    painter: &Painter,
    element: &StrokeElement,
    stroke: Stroke,
    fill_info: &FillInfo,
    viewport: &ViewportState,
    canvas_center: Pos2,
) {
    if transform::has_transform(element) {
        let verts = transform::transformed_vertices(element);
        if element.curve_mode {
            render_curve_path(painter, &verts, element.closed, stroke, fill_info, viewport, canvas_center);
        } else {
            render_rounded_path(painter, &verts, element.closed, stroke, fill_info, viewport, canvas_center);
        }
    } else if element.curve_mode {
        render_curve_path(painter, &element.vertices, element.closed, stroke, fill_info, viewport, canvas_center);
    } else {
        render_rounded_path(painter, &element.vertices, element.closed, stroke, fill_info, viewport, canvas_center);
    }
}

/// Render a path using cubic bezier segments through vertex positions (curve mode).
/// Flattens all segments into a single polyline so line joins are handled properly
/// (no gaps at sharp corners).
fn render_curve_path(
    painter: &Painter,
    verts: &[PathVertex],
    closed: bool,
    stroke: Stroke,
    fill_info: &FillInfo,
    viewport: &ViewportState,
    canvas_center: Pos2,
) {
    if verts.len() < 2 {
        return;
    }

    // Adaptive tolerance: ~0.5 screen pixels regardless of zoom
    let tolerance = (FLATTEN_TOLERANCE_PX / viewport.zoom).max(FLATTEN_MIN_TOLERANCE);
    let mut world_points: Vec<Vec2> = Vec::new();

    let seg_count = if closed { verts.len() } else { verts.len() - 1 };
    for i in 0..seg_count {
        let v0 = &verts[i];
        let v1 = &verts[(i + 1) % verts.len()];
        let (p0, cp1, cp2, p3) = math::segment_bezier_points(v0, v1);
        math::flatten_cubic_bezier(p0, cp1, cp2, p3, tolerance, &mut world_points);
    }

    if world_points.len() < 2 {
        return;
    }

    let screen_points: Vec<Pos2> = world_points
        .iter()
        .map(|p| viewport.world_to_screen(*p, canvas_center))
        .collect();

    render_filled_path(painter, &screen_points, closed, stroke, fill_info);
}

/// Render a path with Figma-style corner fillets (straight mode with radius).
/// Flattens fillet arcs into a single continuous polyline for proper line joins.
fn render_rounded_path(
    painter: &Painter,
    verts: &[PathVertex],
    closed: bool,
    stroke: Stroke,
    fill_info: &FillInfo,
    viewport: &ViewportState,
    canvas_center: Pos2,
) {
    let n = verts.len();
    if n < 2 {
        return;
    }

    // Adaptive tolerance: ~0.5 screen pixels regardless of zoom
    let tolerance = (FLATTEN_TOLERANCE_PX / viewport.zoom).max(FLATTEN_MIN_TOLERANCE);
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

    render_filled_path(painter, &screen_points, closed, stroke, fill_info);
}

/// Render a polygon with proper fill (using ear-clipping triangulation for concave shapes)
/// and stroke as separate shapes so the fill follows the curve geometry exactly.
fn render_filled_path(
    painter: &Painter,
    screen_points: &[Pos2],
    closed: bool,
    stroke: Stroke,
    fill_info: &FillInfo,
) {
    let has_fill = closed && screen_points.len() >= 3 && !matches!(fill_info, FillInfo::Flat(c) if *c == Color32::TRANSPARENT);

    // Render fill as a triangulated mesh (handles concave polygons correctly)
    if has_fill {
        // Deduplicate near-identical consecutive points (from curve flattening segment boundaries)
        let mut deduped: Vec<Pos2> = Vec::with_capacity(screen_points.len());
        for &pt in screen_points {
            if deduped.last().is_none_or(|last: &Pos2| {
                (last.x - pt.x).abs() > 0.1 || (last.y - pt.y).abs() > 0.1
            }) {
                deduped.push(pt);
            }
        }
        // Also check wrap-around duplicate
        if deduped.len() > 2 {
            let first = deduped[0];
            let last = *deduped.last().unwrap();
            if (first.x - last.x).abs() < 0.1 && (first.y - last.y).abs() < 0.1 {
                deduped.pop();
            }
        }

        // Triangulate using earcutr (robust MapBox earcut port)
        let coords: Vec<f64> = deduped.iter().flat_map(|p| [p.x as f64, p.y as f64]).collect();
        let indices = earcutr::earcut(&coords, &[], 2).unwrap_or_default();
        if !indices.is_empty() {
            let mut mesh = egui::Mesh::default();
            for &pt in &deduped {
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: pt,
                    uv: egui::epaint::WHITE_UV,
                    color: gradient_color_at(pt, fill_info),
                });
            }
            for idx in &indices {
                mesh.indices.push(*idx as u32);
            }
            painter.add(egui::Shape::mesh(mesh));
        }
    }

    // Render stroke separately
    painter.add(egui::Shape::Path(egui::epaint::PathShape {
        points: screen_points.to_vec(),
        closed,
        fill: Color32::TRANSPARENT,
        stroke: stroke.into(),
    }));

    // Round caps on open path endpoints
    if !closed && screen_points.len() >= 2 {
        let cap_radius = stroke.width * 0.5;
        painter.circle_filled(screen_points[0], cap_radius, stroke.color);
        painter.circle_filled(*screen_points.last().unwrap(), cap_radius, stroke.color);
    }
}


/// Render hatch fill lines for an element.
/// Uses the element's stroke color and width for all hatch lines.
fn render_hatch_fill(
    painter: &Painter,
    element: &StrokeElement,
    pattern: &HatchPattern,
    palette: &Palette,
    viewport: &ViewportState,
    canvas_center: Pos2,
    dim_alpha: Option<f32>,
) {
    let hatch_data = crate::engine::hatch::generate_element_hatch(
        element,
        pattern,
        element.hatch_flow_curve.as_ref(),
    );
    let mut color = palette.get_color(element.stroke_color_index).to_color32();
    if let Some(alpha) = dim_alpha {
        color = Color32::from_rgba_unmultiplied(
            color.r(), color.g(), color.b(),
            (color.a() as f32 * alpha) as u8,
        );
    }
    let sw = element.stroke_width * viewport.zoom;
    let stroke = Stroke::new(sw, color);
    let cap_radius = sw * 0.5;
    for layer_data in &hatch_data {
        for segment in &layer_data.segments {
            let screen_points: Vec<Pos2> = segment
                .iter()
                .map(|p| viewport.world_to_screen(*p, canvas_center))
                .collect();
            if screen_points.len() >= 2 {
                painter.add(egui::Shape::line(screen_points.clone(), stroke));
                // Round caps: filled circles at endpoints
                painter.circle_filled(screen_points[0], cap_radius, color);
                painter.circle_filled(*screen_points.last().unwrap(), cap_radius, color);
            }
        }
    }
}

/// Render a flow curve on the canvas with control point handles.
pub fn render_flow_curve(
    painter: &Painter,
    element: &StrokeElement,
    viewport: &ViewportState,
    canvas_rect: egui::Rect,
    theme_mode: Theme,
    dragging_cp: Option<usize>,
) {
    let flow_curve = match &element.hatch_flow_curve {
        Some(fc) => fc,
        None => return,
    };
    let cps = &flow_curve.control_points;
    if cps.len() < 4 {
        return;
    }

    let canvas_center = canvas_rect.center();
    let color = theme::flow_curve_color(theme_mode);

    // Flatten the bezier curve for rendering
    let (p0, cp1, cp2, p3) = (cps[0], cps[1], cps[2], cps[3]);
    let tolerance = (FLATTEN_TOLERANCE_PX / viewport.zoom).max(FLATTEN_MIN_TOLERANCE);
    let mut world_pts = Vec::new();
    math::flatten_cubic_bezier(p0, cp1, cp2, p3, tolerance, &mut world_pts);

    if world_pts.len() >= 2 {
        let screen_pts: Vec<Pos2> = world_pts
            .iter()
            .map(|p| viewport.world_to_screen(*p, canvas_center))
            .collect();
        // Draw as dashed line
        for i in 0..screen_pts.len() - 1 {
            draw_dashed_line(
                painter,
                screen_pts[i],
                screen_pts[i + 1],
                Stroke::new(1.5, color),
                4.0,
                3.0,
            );
        }
    }

    // Draw tangent lines from anchors to their control points
    let tangent_stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(
        color.r(), color.g(), color.b(), 100,
    ));
    let s0 = viewport.world_to_screen(cps[0], canvas_center);
    let s1 = viewport.world_to_screen(cps[1], canvas_center);
    let s2 = viewport.world_to_screen(cps[2], canvas_center);
    let s3 = viewport.world_to_screen(cps[3], canvas_center);
    painter.line_segment([s0, s1], tangent_stroke);
    painter.line_segment([s3, s2], tangent_stroke);

    // Draw control point handles
    for (i, cp) in cps.iter().enumerate().take(4) {
        let screen = viewport.world_to_screen(*cp, canvas_center);
        let is_anchor = i == 0 || i == 3;
        let is_dragging = dragging_cp == Some(i);
        let radius = if is_dragging { 6.0 } else if is_anchor { 5.0 } else { 4.0 };
        let fill = if is_anchor { color } else { Color32::TRANSPARENT };
        painter.circle_filled(screen, radius, fill);
        painter.circle_stroke(screen, radius, Stroke::new(1.5, color));
    }
}

/// Hit-test flow curve control points. Returns the index (0-3) if within radius.
pub fn hit_test_flow_curve_cp(
    screen_pos: Pos2,
    element: &StrokeElement,
    viewport: &ViewportState,
    canvas_center: Pos2,
    radius: f32,
) -> Option<usize> {
    let flow_curve = element.hatch_flow_curve.as_ref()?;
    let cps = &flow_curve.control_points;
    if cps.len() < 4 {
        return None;
    }
    let radius_sq = radius * radius;
    for i in 0..4 {
        let screen = viewport.world_to_screen(cps[i], canvas_center);
        let dx = screen_pos.x - screen.x;
        let dy = screen_pos.y - screen.y;
        if dx * dx + dy * dy <= radius_sq {
            return Some(i);
        }
    }
    None
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
                let stroke = Stroke::new((element.stroke_width + HOVER_HIGHLIGHT_EXTRA) * viewport.zoom, highlight_color);
                render_element_path(painter, element, stroke, &FillInfo::Flat(Color32::TRANSPARENT), viewport, canvas_center);
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
                let stroke = Stroke::new((element.stroke_width + SELECTION_HIGHLIGHT_EXTRA) * viewport.zoom, highlight_color);
                render_element_path(painter, element, stroke, &FillInfo::Flat(Color32::TRANSPARENT), viewport, canvas_center);
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
        render_curve_path(painter, vertices, false, stroke, &FillInfo::Flat(Color32::TRANSPARENT), viewport, canvas_center);
    } else {
        render_rounded_path(painter, vertices, false, stroke, &FillInfo::Flat(Color32::TRANSPARENT), viewport, canvas_center);
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
    handles.push((HandleKind::Rotate, Pos2::new(top_center.x, top_center.y - ROTATION_HANDLE_OFFSET)));

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
    draw_dashed_line(painter, tl, tr, box_stroke, SELECTION_BOX_DASH, SELECTION_BOX_GAP);
    draw_dashed_line(painter, tr, br, box_stroke, SELECTION_BOX_DASH, SELECTION_BOX_GAP);
    draw_dashed_line(painter, br, bl, box_stroke, SELECTION_BOX_DASH, SELECTION_BOX_GAP);
    draw_dashed_line(painter, bl, tl, box_stroke, SELECTION_BOX_DASH, SELECTION_BOX_GAP);

    // Draw rotation arm (line from top-center to rotation handle)
    let top_center = viewport.world_to_screen(
        Vec2::new((bounds.0.x + bounds.1.x) * 0.5, bounds.0.y),
        canvas_center,
    );
    let rot_handle_pos = handles.iter().find(|(k, _)| *k == HandleKind::Rotate).map(|(_, p)| *p);
    if let Some(rot_pos) = rot_handle_pos {
        painter.line_segment([top_center, rot_pos], box_stroke);
        // Rotation handle: circle
        painter.circle_filled(rot_pos, ROTATION_HANDLE_RADIUS, handle_color);
        painter.circle_stroke(rot_pos, ROTATION_HANDLE_RADIUS, Stroke::new(1.0, handle_color));
    }

    // Draw scale handles: small filled squares
    for (kind, pos) in &handles {
        if *kind == HandleKind::Rotate {
            continue; // already drawn above
        }
        let rect = egui::Rect::from_center_size(*pos, egui::Vec2::splat(SCALE_HANDLE_HALF_SIZE * 2.0));
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
        // Outline for visibility against any background
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

            // Tangent line from vertex to handle
            painter.line_segment([v_screen, cp_screen], handle_line_stroke);

            // Handle dot
            let is_hovered = matches!(hover_vertex, Some(VertexHover::Handle { vertex_id: vid, is_cp1: c }) if vid == vertex_id && *c == is_cp1);
            let radius = if is_hovered { CP_HANDLE_RADIUS + 1.0 } else { CP_HANDLE_RADIUS };
            let color = if is_hovered { tc.icon_text } else { tc.mid };

            painter.circle_filled(cp_screen, radius, color);
            painter.circle_stroke(cp_screen, radius, Stroke::new(1.0, Color32::BLACK));
        }
    }
}

// ── Phase 5: Snap indicator, symmetry, reference images ──────────────

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

    // Diamond shape (rotated square)
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
    // Inner dot
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
    vertices: &[PathVertex],
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

    // Build the axes to mirror across
    let axes = match symmetry.axis {
        SymmetryAxis::Vertical => vec![SymmetryAxis::Vertical],
        SymmetryAxis::Horizontal => vec![SymmetryAxis::Horizontal],
        SymmetryAxis::Both => vec![SymmetryAxis::Vertical, SymmetryAxis::Horizontal, SymmetryAxis::Both],
    };

    for axis in axes {
        // Mirror committed vertices
        let mut screen_points: Vec<Pos2> = Vec::new();
        for v in vertices {
            let mirrored = sym::mirror_point(v.pos, axis, &symmetry.axis_position);
            screen_points.push(viewport.world_to_screen(mirrored, canvas_center));
        }
        // Mirror the rubber-band cursor position
        let mirrored_cursor = sym::mirror_point(cursor_snap_pos, axis, &symmetry.axis_position);
        screen_points.push(viewport.world_to_screen(mirrored_cursor, canvas_center));

        // Draw as simple polyline (ghost doesn't need full curve rendering)
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

        // Selection border
        if selected_ref_id == Some(ref_img.id.as_str()) {
            let border_color = theme::selection_highlight_color(theme_mode);
            draw_dashed_line(painter, rect.left_top(), rect.right_top(), Stroke::new(1.0, border_color), 4.0, 3.0);
            draw_dashed_line(painter, rect.right_top(), rect.right_bottom(), Stroke::new(1.0, border_color), 4.0, 3.0);
            draw_dashed_line(painter, rect.right_bottom(), rect.left_bottom(), Stroke::new(1.0, border_color), 4.0, 3.0);
            draw_dashed_line(painter, rect.left_bottom(), rect.left_top(), Stroke::new(1.0, border_color), 4.0, 3.0);
        }
    }
}
