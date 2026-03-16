use crate::engine::transform;
use crate::math;
use crate::model::sprite::{PathVertex, Sprite, StrokeElement};
use crate::model::vec2::Vec2;
use crate::state::editor::ViewportState;

/// Test whether a single element is hit at the given world position.
/// Returns the distance from the cursor to the element's path.
fn element_hit_distance(
    world_pos: Vec2,
    element: &crate::model::sprite::StrokeElement,
    polyline: &mut Vec<Vec2>,
) -> f32 {
    if element.vertices.len() < 2 {
        return f32::MAX;
    }

    let local_pos = if transform::has_transform(element) {
        transform::inverse_transform_point(
            world_pos,
            element.origin,
            element.position,
            element.rotation,
            element.scale,
        )
    } else {
        world_pos
    };

    if element.curve_mode {
        hit_test_curve_path(local_pos, &element.vertices, element.closed, polyline)
    } else {
        hit_test_rounded_path(local_pos, &element.vertices, element.closed, polyline)
    }
}

/// Check if a layer is interactive (visible, unlocked, and not excluded by solo).
fn is_layer_interactive(layer: &crate::model::sprite::Layer, solo_layer_id: Option<&str>) -> bool {
    if !layer.visible || layer.locked {
        return false;
    }
    if let Some(solo_id) = solo_layer_id
        && layer.id != solo_id {
            return false;
        }
    true
}

/// Find the topmost visible, unlocked element under the cursor.
/// When `solo_layer_id` is set, only the soloed layer is hit-testable.
/// Returns the element ID if found.
pub fn hit_test_elements(
    world_pos: Vec2,
    sprite: &Sprite,
    threshold: f32,
    solo_layer_id: Option<&str>,
) -> Option<String> {
    let mut polyline = Vec::new();

    for layer in sprite.layers.iter().rev() {
        if !is_layer_interactive(layer, solo_layer_id) {
            continue;
        }
        for element in layer.elements.iter().rev() {
            let hit_threshold = threshold + element.stroke_width / 2.0;
            if element_hit_distance(world_pos, element, &mut polyline) <= hit_threshold {
                return Some(element.id.clone());
            }
        }
    }
    None
}

/// Find ALL visible, unlocked elements under the cursor, ordered top-to-bottom.
/// When `solo_layer_id` is set, only the soloed layer is searched.
/// Returns (element_id, display_name, stroke_color_index) tuples.
pub fn hit_test_all_elements(
    world_pos: Vec2,
    sprite: &Sprite,
    threshold: f32,
    solo_layer_id: Option<&str>,
) -> Vec<(String, String, u8)> {
    let mut results = Vec::new();
    let mut polyline = Vec::new();

    for layer in sprite.layers.iter().rev() {
        if !is_layer_interactive(layer, solo_layer_id) {
            continue;
        }
        for element in layer.elements.iter().rev() {
            let hit_threshold = threshold + element.stroke_width / 2.0;
            if element_hit_distance(world_pos, element, &mut polyline) <= hit_threshold {
                let name = element.name.clone().unwrap_or_else(|| "Stroke".to_string());
                results.push((element.id.clone(), name, element.stroke_color_index));
            }
        }
    }
    results
}

/// Hit test a curve-mode path (bezier segments through vertex positions).
fn hit_test_curve_path(
    point: Vec2,
    verts: &[PathVertex],
    closed: bool,
    polyline: &mut Vec<Vec2>,
) -> f32 {
    let mut min_dist = f32::MAX;

    for i in 0..verts.len().saturating_sub(1) {
        let (p0, cp1, cp2, p3) = math::segment_bezier_points(&verts[i], &verts[i + 1]);
        let dist = point_to_bezier_distance(point, p0, cp1, cp2, p3, polyline);
        if dist < min_dist {
            min_dist = dist;
        }
    }
    if closed && verts.len() >= 2 {
        let last = verts.len() - 1;
        let (p0, cp1, cp2, p3) = math::segment_bezier_points(&verts[last], &verts[0]);
        let dist = point_to_bezier_distance(point, p0, cp1, cp2, p3, polyline);
        if dist < min_dist {
            min_dist = dist;
        }
    }
    min_dist
}

/// Hit test a straight-mode path (straight edges with fillet arcs at corners).
/// Mirrors the geometry built by `render_rounded_path` in canvas_render.rs.
fn hit_test_rounded_path(
    point: Vec2,
    verts: &[PathVertex],
    closed: bool,
    polyline: &mut Vec<Vec2>,
) -> f32 {
    // Build the same polyline that render_rounded_path uses
    polyline.clear();
    let tolerance = 1.0; // world-space tolerance for hit testing

    for v in verts {
        if let (Some(t1), Some(t2)) = (v.cp1, v.cp2) {
            let (arc_cp1, arc_cp2) = math::fillet_arc_control_points(t1, t2, v.pos);
            let mut arc = Vec::new();
            math::flatten_cubic_bezier(t1, arc_cp1, arc_cp2, t2, tolerance, &mut arc);
            polyline.extend_from_slice(&arc);
        } else {
            polyline.push(v.pos);
        }
    }

    if closed {
        // Close the polyline by adding the first point at the end
        if let Some(&first) = polyline.first() {
            polyline.push(first);
        }
    }

    point_to_polyline_distance(point, polyline)
}

/// Approximate distance from a point to a cubic bezier curve.
fn point_to_bezier_distance(
    point: Vec2,
    p0: Vec2,
    cp1: Vec2,
    cp2: Vec2,
    p3: Vec2,
    polyline: &mut Vec<Vec2>,
) -> f32 {
    polyline.clear();
    math::flatten_cubic_bezier(p0, cp1, cp2, p3, 1.0, polyline);
    point_to_polyline_distance(point, polyline)
}

/// Distance from a point to a polyline (series of connected line segments).
fn point_to_polyline_distance(point: Vec2, polyline: &[Vec2]) -> f32 {
    let mut min_dist = f32::MAX;
    for i in 0..polyline.len().saturating_sub(1) {
        let dist = point_to_segment_distance(point, polyline[i], polyline[i + 1]);
        if dist < min_dist {
            min_dist = dist;
        }
    }
    min_dist
}

/// Distance from a point to a line segment.
fn point_to_segment_distance(point: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let ap = point - a;
    let len_sq = ab.length_sq();
    if len_sq < 1e-10 {
        return ap.length();
    }
    let t = (ap.dot(ab) / len_sq).clamp(0.0, 1.0);
    let closest = a + ab * t;
    point.distance(closest)
}

/// Point-in-polygon test using ray-casting algorithm.
/// Counts crossings of a horizontal ray from `point` going rightward.
fn point_in_polygon(point: Vec2, polygon: &[Vec2]) -> bool {
    let n = polygon.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let yi = polygon[i].y;
        let yj = polygon[j].y;
        if (yi > point.y) != (yj > point.y) {
            let x_intersect =
                polygon[i].x + (point.y - yi) / (yj - yi) * (polygon[j].x - polygon[i].x);
            if point.x < x_intersect {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

/// Build a flattened polygon for a closed element (for point-in-polygon testing).
fn build_element_polygon(element: &StrokeElement) -> Vec<Vec2> {
    let tolerance = 1.0;
    let mut polygon = Vec::new();

    let verts = if transform::has_transform(element) {
        transform::transformed_vertices(element)
    } else {
        element.vertices.clone()
    };

    if element.curve_mode {
        let seg_count = if element.closed { verts.len() } else { verts.len().saturating_sub(1) };
        for i in 0..seg_count {
            let v0 = &verts[i];
            let v1 = &verts[(i + 1) % verts.len()];
            let (p0, cp1, cp2, p3) = math::segment_bezier_points(v0, v1);
            math::flatten_cubic_bezier(p0, cp1, cp2, p3, tolerance, &mut polygon);
        }
    } else {
        for v in &verts {
            if let (Some(t1), Some(t2)) = (v.cp1, v.cp2) {
                let (arc_cp1, arc_cp2) = math::fillet_arc_control_points(t1, t2, v.pos);
                let mut arc = Vec::new();
                math::flatten_cubic_bezier(t1, arc_cp1, arc_cp2, t2, tolerance, &mut arc);
                polygon.extend_from_slice(&arc);
            } else {
                polygon.push(v.pos);
            }
        }
    }
    polygon
}

/// Hit test for the fill tool: checks both stroke proximity AND point-in-polygon
/// for closed elements. Returns (element_id, is_closed) for the topmost hit.
pub fn hit_test_fill(
    world_pos: Vec2,
    sprite: &Sprite,
    threshold: f32,
    solo_layer_id: Option<&str>,
) -> Option<(String, bool)> {
    let mut polyline = Vec::new();

    for layer in sprite.layers.iter().rev() {
        if !is_layer_interactive(layer, solo_layer_id) {
            continue;
        }
        for element in layer.elements.iter().rev() {
            // Check stroke proximity first
            let hit_threshold = threshold + element.stroke_width / 2.0;
            if element_hit_distance(world_pos, element, &mut polyline) <= hit_threshold {
                return Some((element.id.clone(), element.closed));
            }
            // For closed elements, also check if the point is inside the fill area
            if element.closed && element.vertices.len() >= 3 {
                let polygon = build_element_polygon(element);
                // For transformed elements, we built the polygon in world space already
                if point_in_polygon(world_pos, &polygon) {
                    return Some((element.id.clone(), true));
                }
            }
        }
    }
    None
}

/// Hit test for the eyedropper tool: returns (element_id, stroke_color_index, fill_color_index)
/// for the topmost hit element. Checks stroke proximity and point-in-polygon for closed shapes.
pub fn hit_test_eyedropper(
    world_pos: Vec2,
    sprite: &Sprite,
    threshold: f32,
    solo_layer_id: Option<&str>,
) -> Option<(String, u8, u8)> {
    let mut polyline = Vec::new();

    for layer in sprite.layers.iter().rev() {
        if !is_layer_interactive(layer, solo_layer_id) {
            continue;
        }
        for element in layer.elements.iter().rev() {
            let hit_threshold = threshold + element.stroke_width / 2.0;
            if element_hit_distance(world_pos, element, &mut polyline) <= hit_threshold {
                return Some((element.id.clone(), element.stroke_color_index, element.fill_color_index));
            }
            if element.closed && element.vertices.len() >= 3 {
                let polygon = build_element_polygon(element);
                if point_in_polygon(world_pos, &polygon) {
                    return Some((element.id.clone(), element.stroke_color_index, element.fill_color_index));
                }
            }
        }
    }
    None
}

/// Hit test vertices of an element in screen space.
/// Returns the vertex ID of the first hit, checking in reverse order (top-most first).
pub fn hit_test_vertex(
    screen_pos: egui::Pos2,
    element: &StrokeElement,
    viewport: &ViewportState,
    canvas_center: egui::Pos2,
    radius_px: f32,
) -> Option<String> {
    for v in element.vertices.iter().rev() {
        let world = transform::vertex_world_pos(v, element);
        let vscreen = viewport.world_to_screen(world, canvas_center);
        let dx = screen_pos.x - vscreen.x;
        let dy = screen_pos.y - vscreen.y;
        if dx * dx + dy * dy <= radius_px * radius_px {
            return Some(v.id.clone());
        }
    }
    None
}

/// Hit test control point handles of the selected vertex in screen space.
/// Only tests cp1/cp2 of the vertex with `selected_vertex_id` when element is curve_mode.
/// Returns `(vertex_id, is_cp1)` if hit.
pub fn hit_test_handle(
    screen_pos: egui::Pos2,
    element: &StrokeElement,
    selected_vertex_id: &str,
    viewport: &ViewportState,
    canvas_center: egui::Pos2,
    radius_px: f32,
) -> Option<(String, bool)> {
    if !element.curve_mode {
        return None;
    }
    let vertex = element.vertices.iter().find(|v| v.id == selected_vertex_id)?;
    // Test cp1
    if let Some(cp1) = vertex.cp1 {
        let world = transform::transform_point(cp1, element.origin, element.position, element.rotation, element.scale);
        let hscreen = viewport.world_to_screen(world, canvas_center);
        let dx = screen_pos.x - hscreen.x;
        let dy = screen_pos.y - hscreen.y;
        if dx * dx + dy * dy <= radius_px * radius_px {
            return Some((vertex.id.clone(), true));
        }
    }
    // Test cp2
    if let Some(cp2) = vertex.cp2 {
        let world = transform::transform_point(cp2, element.origin, element.position, element.rotation, element.scale);
        let hscreen = viewport.world_to_screen(world, canvas_center);
        let dx = screen_pos.x - hscreen.x;
        let dy = screen_pos.y - hscreen.y;
        if dx * dx + dy * dy <= radius_px * radius_px {
            return Some((vertex.id.clone(), false));
        }
    }
    None
}
