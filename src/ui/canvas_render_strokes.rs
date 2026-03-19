use egui::{Color32, Painter, Pos2, Stroke};

use crate::engine::transform;
use crate::math;
use crate::model::project::{HatchPattern, Palette, Theme};
use crate::model::sprite::{GradientType, PathVertex, SpreadMethod, StrokeElement, Sprite};
use crate::model::vec2::Vec2;
use crate::state::editor::ViewportState;
use crate::theme;

/// Curve flattening: target screen-pixel tolerance.
const FLATTEN_TOLERANCE_PX: f32 = 0.5;
/// Minimum world-space flattening tolerance (prevents infinite subdivision).
const FLATTEN_MIN_TOLERANCE: f32 = 0.01;
/// Extra stroke width added for hover highlight (world units).
const HOVER_HIGHLIGHT_EXTRA: f32 = 2.0;
/// Extra stroke width added for selection highlight (world units).
const SELECTION_HIGHLIGHT_EXTRA: f32 = 2.0;

/// Fill rendering info passed through the render pipeline.
#[derive(Clone)]
#[allow(dead_code)]
enum FillInfo {
    Flat(Color32),
    Gradient {
        /// Color stops sorted by position, pre-resolved from palette.
        stops: Vec<(f32, Color32)>,
        /// Midpoint values between each adjacent stop pair (0.0-1.0).
        midpoints: Vec<f32>,
        gradient_type: GradientType,
        angle_rad: f32,
        /// Element AABB in screen space (min, max).
        bounds_min: Pos2,
        bounds_max: Pos2,
        /// Radial center, normalized 0..1 within element AABB.
        center: (f32, f32),
        /// Radial radius, normalized 0..1 of the AABB max dimension.
        radius: f32,
        /// Spread method (pad, reflect, repeat).
        spread: SpreadMethod,
        /// Radial focal point offset (normalized 0..1 within AABB).
        focal_offset: (f32, f32),
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

/// Apply spread method to raw t value, returning a value suitable for stop lookup.
fn apply_spread(t: f32, spread: SpreadMethod) -> f32 {
    match spread {
        SpreadMethod::Pad => t.clamp(0.0, 1.0),
        SpreadMethod::Repeat => {
            let t = t.rem_euclid(1.0);
            if t < 0.0 { t + 1.0 } else { t }
        }
        SpreadMethod::Reflect => {
            let cycle = t.rem_euclid(2.0);
            if cycle > 1.0 { 2.0 - cycle } else { cycle }
        }
    }
}

/// Sample color from multi-stop gradient at parameter t (0..1).
fn sample_gradient(t: f32, stops: &[(f32, Color32)], midpoints: &[f32]) -> Color32 {
    if stops.is_empty() {
        return Color32::TRANSPARENT;
    }
    if stops.len() == 1 || t <= stops[0].0 {
        return stops[0].1;
    }
    if t >= stops[stops.len() - 1].0 {
        return stops[stops.len() - 1].1;
    }

    let mut i = 0;
    while i + 1 < stops.len() && stops[i + 1].0 < t {
        i += 1;
    }
    if i + 1 >= stops.len() {
        return stops[stops.len() - 1].1;
    }

    let (pos_a, col_a) = stops[i];
    let (pos_b, col_b) = stops[i + 1];
    let seg_len = pos_b - pos_a;
    if seg_len < 0.0001 {
        return col_a;
    }

    let local_t = ((t - pos_a) / seg_len).clamp(0.0, 1.0);
    let m = midpoints.get(i).copied().unwrap_or(0.5).clamp(0.01, 0.99);
    let adjusted = if local_t <= m {
        0.5 * (local_t / m)
    } else {
        0.5 + 0.5 * ((local_t - m) / (1.0 - m))
    };

    lerp_color(col_a, col_b, adjusted)
}

fn gradient_color_at(pos: Pos2, fill: &FillInfo) -> Color32 {
    match fill {
        FillInfo::Flat(c) => *c,
        FillInfo::Gradient {
            stops, midpoints, gradient_type, angle_rad,
            bounds_min, bounds_max, center: _, radius, spread,
            focal_offset,
        } => {
            if stops.is_empty() {
                return Color32::TRANSPARENT;
            }

            let raw_t = match gradient_type {
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
                        return stops[0].1;
                    }
                    let proj = pos.x * dir_x + pos.y * dir_y;
                    (proj - proj_min) / extent
                }
                GradientType::Radial => {
                    let fx = bounds_min.x + focal_offset.0 * (bounds_max.x - bounds_min.x);
                    let fy = bounds_min.y + focal_offset.1 * (bounds_max.y - bounds_min.y);
                    let dx = bounds_max.x - bounds_min.x;
                    let dy = bounds_max.y - bounds_min.y;
                    let max_dist = radius * (dx.max(dy) * 0.5);
                    if max_dist < 0.001 {
                        return stops[0].1;
                    }
                    let dist = ((pos.x - fx).powi(2) + (pos.y - fy).powi(2)).sqrt();
                    dist / max_dist
                }
            };

            let t = apply_spread(raw_t, *spread);
            sample_gradient(t, stops, midpoints)
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

    if let Some(ref grad) = element.gradient_fill
        && element.closed && grad.stops.len() >= 2
    {
        let (bmin, bmax) = element_screen_bounds(element, viewport, canvas_center);
        let stops: Vec<(f32, Color32)> = grad.stops.iter()
            .map(|s| (s.position, apply_dim(palette.get_color(s.color_index).to_color32())))
            .collect();
        let center = grad.center.map(|c| (c.x, c.y)).unwrap_or((0.5, 0.5));
        let radius = grad.radius.unwrap_or(0.5);
        let focal_offset = grad.focal_offset.map(|f| (f.x, f.y)).unwrap_or(center);
        return FillInfo::Gradient {
            stops,
            midpoints: grad.midpoints.clone(),
            gradient_type: grad.gradient_type,
            angle_rad: grad.angle_rad,
            bounds_min: bmin,
            bounds_max: bmax,
            center,
            radius,
            spread: grad.spread,
            focal_offset,
        };
    }

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
        return;
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

/// Render a sprite as a monochrome onion skin ghost.
/// All strokes are drawn in `tint` color (which has alpha baked in).
/// Fills and hatching are skipped for clarity.
pub fn render_onion_ghost(
    painter: &Painter,
    viewport: &ViewportState,
    sprite: &Sprite,
    canvas_rect: egui::Rect,
    tint: Color32,
) {
    let canvas_center = canvas_rect.center();

    for layer in &sprite.layers {
        if !layer.visible {
            continue;
        }
        for element in &layer.elements {
            let stroke = Stroke::new(element.stroke_width * viewport.zoom, tint);
            let fill_info = FillInfo::Flat(Color32::TRANSPARENT);
            render_element_path(painter, element, stroke, &fill_info, viewport, canvas_center);
        }
    }
}

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

    let tolerance = (FLATTEN_TOLERANCE_PX / viewport.zoom).max(FLATTEN_MIN_TOLERANCE);
    let mut world_points: Vec<Vec2> = Vec::new();

    for v in verts {
        if let (Some(t1), Some(t2)) = (v.cp1, v.cp2) {
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

/// Subdivide a triangle into a grid and add to the mesh with per-vertex gradient colors.
fn subdivide_triangle_gradient(
    mesh: &mut egui::Mesh,
    p0: Pos2,
    p1: Pos2,
    p2: Pos2,
    subdivisions: u32,
    fill_info: &FillInfo,
) {
    let n = subdivisions;
    let base_idx = mesh.vertices.len() as u32;

    for i in 0..=n {
        for j in 0..=(n - i) {
            let u = i as f32 / n as f32;
            let v = j as f32 / n as f32;
            let w = 1.0 - u - v;
            let pt = Pos2::new(
                p0.x * w + p1.x * u + p2.x * v,
                p0.y * w + p1.y * u + p2.y * v,
            );
            mesh.vertices.push(egui::epaint::Vertex {
                pos: pt,
                uv: egui::epaint::WHITE_UV,
                color: gradient_color_at(pt, fill_info),
            });
        }
    }

    let row_start = |i: u32| -> u32 {
        let mut s = 0u32;
        for k in 0..i {
            s += n - k + 1;
        }
        s
    };

    for i in 0..n {
        let row_len = n - i + 1;
        let r0 = row_start(i);
        let r1 = row_start(i + 1);
        for j in 0..(row_len - 1) {
            mesh.indices.push(base_idx + r0 + j);
            mesh.indices.push(base_idx + r1 + j);
            mesh.indices.push(base_idx + r0 + j + 1);

            if j + 1 < (n - i) {
                mesh.indices.push(base_idx + r1 + j);
                mesh.indices.push(base_idx + r1 + j + 1);
                mesh.indices.push(base_idx + r0 + j + 1);
            }
        }
    }
}

fn render_filled_path(
    painter: &Painter,
    screen_points: &[Pos2],
    closed: bool,
    stroke: Stroke,
    fill_info: &FillInfo,
) {
    let has_fill = closed && screen_points.len() >= 3
        && !matches!(fill_info, FillInfo::Flat(c) if *c == Color32::TRANSPARENT);

    if has_fill {
        let mut deduped: Vec<Pos2> = Vec::with_capacity(screen_points.len());
        for &pt in screen_points {
            if deduped.last().is_none_or(|last: &Pos2| {
                (last.x - pt.x).abs() > 0.1 || (last.y - pt.y).abs() > 0.1
            }) {
                deduped.push(pt);
            }
        }
        if deduped.len() > 2 {
            let first = deduped[0];
            let last = *deduped.last().unwrap();
            if (first.x - last.x).abs() < 0.1 && (first.y - last.y).abs() < 0.1 {
                deduped.pop();
            }
        }

        let coords: Vec<f64> = deduped.iter().flat_map(|p| [p.x as f64, p.y as f64]).collect();
        let indices = earcutr::earcut(&coords, &[], 2).unwrap_or_default();
        if !indices.is_empty() {
            let is_gradient = matches!(fill_info, FillInfo::Gradient { .. });

            if is_gradient {
                let subdivisions = 8_u32;
                let mut mesh = egui::Mesh::default();
                for tri in indices.chunks(3) {
                    let (i0, i1, i2) = (tri[0], tri[1], tri[2]);
                    let p0 = deduped[i0];
                    let p1 = deduped[i1];
                    let p2 = deduped[i2];
                    subdivide_triangle_gradient(&mut mesh, p0, p1, p2, subdivisions, fill_info);
                }
                painter.add(egui::Shape::mesh(mesh));
            } else {
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
    }

    painter.add(egui::Shape::Path(egui::epaint::PathShape {
        points: screen_points.to_vec(),
        closed,
        fill: Color32::TRANSPARENT,
        stroke: stroke.into(),
    }));

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
    let hatch_data = crate::engine::hatch::generate_element_hatch(element, pattern);
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
                painter.circle_filled(screen_points[0], cap_radius, color);
                painter.circle_filled(*screen_points.last().unwrap(), cap_radius, color);
            }
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
                let stroke = Stroke::new(
                    (element.stroke_width + HOVER_HIGHLIGHT_EXTRA) * viewport.zoom,
                    highlight_color,
                );
                render_element_path(
                    painter, element, stroke,
                    &FillInfo::Flat(Color32::TRANSPARENT), viewport, canvas_center,
                );
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
                let stroke = Stroke::new(
                    (element.stroke_width + SELECTION_HIGHLIGHT_EXTRA) * viewport.zoom,
                    highlight_color,
                );
                render_element_path(
                    painter, element, stroke,
                    &FillInfo::Flat(Color32::TRANSPARENT), viewport, canvas_center,
                );
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
    curve_mode: bool,
) {
    let canvas_center = canvas_rect.center();
    let color = palette.get_color(color_index).to_color32();
    let stroke = Stroke::new(stroke_width * viewport.zoom, color);

    if curve_mode {
        render_curve_path(painter, vertices, false, stroke, &FillInfo::Flat(Color32::TRANSPARENT), viewport, canvas_center);
    } else {
        render_rounded_path(painter, vertices, false, stroke, &FillInfo::Flat(Color32::TRANSPARENT), viewport, canvas_center);
    }

    if let Some(last) = vertices.last() {
        let preview_color = theme::rubber_band_color(theme_mode);
        let preview_stroke = Stroke::new(stroke_width * viewport.zoom, preview_color);

        let s_last = viewport.world_to_screen(last.pos, canvas_center);
        let s_cursor = viewport.world_to_screen(snap_pos, canvas_center);
        painter.line_segment([s_last, s_cursor], preview_stroke);
    }

    for v in vertices {
        let screen = viewport.world_to_screen(v.pos, canvas_center);
        painter.circle_filled(screen, 3.0, color);
    }

    let snap_screen = viewport.world_to_screen(snap_pos, canvas_center);
    painter.circle_stroke(snap_screen, 4.0, Stroke::new(1.0, color));
}
