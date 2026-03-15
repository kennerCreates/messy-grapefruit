use crate::math;
use crate::model::sprite::Sprite;
use crate::model::vec2::Vec2;

/// Find the topmost visible, unlocked element under the cursor.
/// Returns the element ID if found.
pub fn hit_test_elements(
    world_pos: Vec2,
    sprite: &Sprite,
    threshold: f32,
) -> Option<String> {
    let mut polyline = Vec::new(); // reused across all segments

    // Iterate layers top-to-bottom (last = topmost)
    for layer in sprite.layers.iter().rev() {
        if !layer.visible || layer.locked {
            continue;
        }
        // Iterate elements in reverse (last drawn = topmost)
        for element in layer.elements.iter().rev() {
            if element.vertices.len() < 2 {
                continue;
            }
            let hit_threshold = threshold + element.stroke_width / 2.0;
            for i in 0..element.vertices.len() - 1 {
                let (p0, cp1, cp2, p3) =
                    math::segment_bezier_points(&element.vertices[i], &element.vertices[i + 1]);
                if point_to_bezier_distance(world_pos, p0, cp1, cp2, p3, &mut polyline)
                    <= hit_threshold
                {
                    return Some(element.id.clone());
                }
            }
            // For closed paths, also check the closing segment
            if element.closed && element.vertices.len() >= 2 {
                let last = element.vertices.len() - 1;
                let (p0, cp1, cp2, p3) =
                    math::segment_bezier_points(&element.vertices[last], &element.vertices[0]);
                if point_to_bezier_distance(world_pos, p0, cp1, cp2, p3, &mut polyline)
                    <= hit_threshold
                {
                    return Some(element.id.clone());
                }
            }
        }
    }
    None
}

/// Approximate distance from a point to a cubic bezier curve.
/// Reuses the provided buffer to avoid per-call allocation.
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
