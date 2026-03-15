use crate::model::sprite::PathVertex;
use crate::model::vec2::Vec2;

/// Convert Catmull-Rom spline segment to cubic bezier control points.
/// Given four points (p0, p1, p2, p3), returns (cp1, cp2) for the cubic bezier
/// from p1 to p2.
pub fn catmull_rom_to_cubic(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2) -> (Vec2, Vec2) {
    let cp1 = p1 + (p2 - p0) / 6.0;
    let cp2 = p2 - (p3 - p1) / 6.0;
    (cp1, cp2)
}

/// Recompute auto-curve control points for a vertex sequence.
///
/// - `curve_mode`: if true, uses full Catmull-Rom tangents; if false, only applies min radius.
/// - `min_corner_radius`: minimum tangent length at each vertex, ensuring corners are never
///   sharper than this radius. When 0 and not in curve mode, CPs are cleared (fully sharp).
pub fn recompute_auto_curves(
    vertices: &mut [PathVertex],
    closed: bool,
    curve_mode: bool,
    min_corner_radius: f32,
) {
    let n = vertices.len();
    if n < 2 {
        return;
    }

    let positions: Vec<Vec2> = vertices.iter().map(|v| v.pos).collect();

    for i in 0..n {
        let (p_prev, p_next) = if closed {
            (positions[(i + n - 1) % n], positions[(i + 1) % n])
        } else {
            let p_prev = if i > 0 { positions[i - 1] } else { positions[0] };
            let p_next = if i + 1 < n { positions[i + 1] } else { positions[n - 1] };
            (p_prev, p_next)
        };

        let direction = p_next - p_prev;
        let dir_len = direction.length();

        if dir_len < 0.001 {
            vertices[i].cp1 = None;
            vertices[i].cp2 = None;
            continue;
        }

        // In curve mode: auto tangent length (Catmull-Rom); otherwise 0
        let base_length = if curve_mode { dir_len / 6.0 } else { 0.0 };
        let tangent_length = base_length.max(min_corner_radius);

        if tangent_length < 0.001 {
            vertices[i].cp1 = None;
            vertices[i].cp2 = None;
        } else {
            let tangent = direction * (tangent_length / dir_len);
            vertices[i].cp1 = Some(positions[i] - tangent);
            vertices[i].cp2 = Some(positions[i] + tangent);
        }
    }
}

/// Get the four bezier points for the segment between two adjacent vertices.
/// Returns (start, cp1, cp2, end).
pub fn segment_bezier_points(v0: &PathVertex, v1: &PathVertex) -> (Vec2, Vec2, Vec2, Vec2) {
    let p0 = v0.pos;
    let p3 = v1.pos;
    // cp2 of v0 is the outgoing control point
    let cp1 = v0.cp2.unwrap_or(p0);
    // cp1 of v1 is the incoming control point
    let cp2 = v1.cp1.unwrap_or(p3);
    (p0, cp1, cp2, p3)
}

/// Evaluate a cubic bezier at parameter t.
pub fn cubic_bezier_eval(p0: Vec2, cp1: Vec2, cp2: Vec2, p3: Vec2, t: f32) -> Vec2 {
    let t2 = t * t;
    let t3 = t2 * t;
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let mt3 = mt2 * mt;
    p0 * mt3 + cp1 * (3.0 * mt2 * t) + cp2 * (3.0 * mt * t2) + p3 * t3
}

/// Flatten a cubic bezier curve into a polyline using recursive subdivision.
pub fn flatten_cubic_bezier(
    p0: Vec2,
    cp1: Vec2,
    cp2: Vec2,
    p3: Vec2,
    tolerance: f32,
    output: &mut Vec<Vec2>,
) {
    flatten_recursive(p0, cp1, cp2, p3, tolerance * tolerance, 0, output);
    output.push(p3);
}

fn flatten_recursive(
    p0: Vec2,
    cp1: Vec2,
    cp2: Vec2,
    p3: Vec2,
    tol_sq: f32,
    depth: u32,
    output: &mut Vec<Vec2>,
) {
    if depth > 10 {
        output.push(p0);
        return;
    }

    // Check if the curve is flat enough by measuring control point deviation
    let d1 = point_to_line_distance_sq(cp1, p0, p3);
    let d2 = point_to_line_distance_sq(cp2, p0, p3);

    if d1 <= tol_sq && d2 <= tol_sq {
        output.push(p0);
        return;
    }

    // De Casteljau split at t=0.5
    let (left, right) = de_casteljau_split(p0, cp1, cp2, p3, 0.5);
    flatten_recursive(left.0, left.1, left.2, left.3, tol_sq, depth + 1, output);
    flatten_recursive(right.0, right.1, right.2, right.3, tol_sq, depth + 1, output);
}

fn point_to_line_distance_sq(point: Vec2, line_a: Vec2, line_b: Vec2) -> f32 {
    let ab = line_b - line_a;
    let ap = point - line_a;
    let ab_len_sq = ab.length_sq();
    if ab_len_sq < 1e-10 {
        return ap.length_sq();
    }
    let cross = ab.x * ap.y - ab.y * ap.x;
    (cross * cross) / ab_len_sq
}

/// Split a cubic bezier at parameter t using De Casteljau's algorithm.
/// Returns ((p0, cp1, cp2, p3), (p0, cp1, cp2, p3)) for left and right halves.
#[allow(clippy::type_complexity)]
pub fn de_casteljau_split(
    p0: Vec2,
    cp1: Vec2,
    cp2: Vec2,
    p3: Vec2,
    t: f32,
) -> ((Vec2, Vec2, Vec2, Vec2), (Vec2, Vec2, Vec2, Vec2)) {
    let p01 = p0.lerp(cp1, t);
    let p12 = cp1.lerp(cp2, t);
    let p23 = cp2.lerp(p3, t);
    let p012 = p01.lerp(p12, t);
    let p123 = p12.lerp(p23, t);
    let p0123 = p012.lerp(p123, t);
    ((p0, p01, p012, p0123), (p0123, p123, p23, p3))
}

/// Approximate the arc length of a cubic bezier curve by sampling.
pub fn approximate_bezier_length(p0: Vec2, cp1: Vec2, cp2: Vec2, p3: Vec2, steps: usize) -> f32 {
    let mut length = 0.0;
    let mut prev = p0;
    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        let curr = cubic_bezier_eval(p0, cp1, cp2, p3, t);
        length += prev.distance(curr);
        prev = curr;
    }
    length
}

/// Compute cumulative arc lengths for a flattened polyline.
/// Returns a vector of cumulative distances, starting with 0.0.
pub fn cumulative_arc_lengths(points: &[Vec2]) -> Vec<f32> {
    let mut lengths = Vec::with_capacity(points.len());
    lengths.push(0.0);
    for i in 1..points.len() {
        let prev = lengths[i - 1];
        lengths.push(prev + points[i - 1].distance(points[i]));
    }
    lengths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bezier_eval_endpoints() {
        let p0 = Vec2::new(0.0, 0.0);
        let cp1 = Vec2::new(1.0, 2.0);
        let cp2 = Vec2::new(3.0, 2.0);
        let p3 = Vec2::new(4.0, 0.0);

        let start = cubic_bezier_eval(p0, cp1, cp2, p3, 0.0);
        assert!((start.x - p0.x).abs() < 1e-6);
        assert!((start.y - p0.y).abs() < 1e-6);

        let end = cubic_bezier_eval(p0, cp1, cp2, p3, 1.0);
        assert!((end.x - p3.x).abs() < 1e-6);
        assert!((end.y - p3.y).abs() < 1e-6);
    }

    #[test]
    fn test_de_casteljau_split_midpoint() {
        let p0 = Vec2::new(0.0, 0.0);
        let cp1 = Vec2::new(1.0, 2.0);
        let cp2 = Vec2::new(3.0, 2.0);
        let p3 = Vec2::new(4.0, 0.0);

        let (left, right) = de_casteljau_split(p0, cp1, cp2, p3, 0.5);

        // Left curve starts at p0
        assert!((left.0.x - p0.x).abs() < 1e-6);
        // Right curve ends at p3
        assert!((right.3.x - p3.x).abs() < 1e-6);
        // They share the midpoint
        assert!((left.3.x - right.0.x).abs() < 1e-6);
        assert!((left.3.y - right.0.y).abs() < 1e-6);
    }

    #[test]
    fn test_flatten_produces_points() {
        let p0 = Vec2::new(0.0, 0.0);
        let cp1 = Vec2::new(1.0, 2.0);
        let cp2 = Vec2::new(3.0, 2.0);
        let p3 = Vec2::new(4.0, 0.0);

        let mut points = Vec::new();
        flatten_cubic_bezier(p0, cp1, cp2, p3, 0.5, &mut points);
        assert!(points.len() >= 2);
        // First point should be p0
        assert!((points[0].x - p0.x).abs() < 1e-6);
        // Last point should be p3
        let last = points.last().unwrap();
        assert!((last.x - p3.x).abs() < 1e-6);
    }

    #[test]
    fn test_catmull_rom_straight_line() {
        // For a straight line, control points should stay on the line
        let p0 = Vec2::new(0.0, 0.0);
        let p1 = Vec2::new(1.0, 0.0);
        let p2 = Vec2::new(2.0, 0.0);
        let p3 = Vec2::new(3.0, 0.0);
        let (cp1, cp2) = catmull_rom_to_cubic(p0, p1, p2, p3);
        // For a straight line, y-components of control points should be 0
        assert!(cp1.y.abs() < 1e-6);
        assert!(cp2.y.abs() < 1e-6);
    }

    #[test]
    fn test_recompute_auto_curves_open() {
        let mut vertices = vec![
            PathVertex::new(Vec2::new(0.0, 0.0)),
            PathVertex::new(Vec2::new(10.0, 20.0)),
            PathVertex::new(Vec2::new(20.0, 0.0)),
        ];
        recompute_auto_curves(&mut vertices, false, true, 0.0);
        // All vertices should have control points
        for v in &vertices {
            assert!(v.cp1.is_some());
            assert!(v.cp2.is_some());
        }
    }

    #[test]
    fn test_recompute_auto_curves_closed() {
        let mut vertices = vec![
            PathVertex::new(Vec2::new(0.0, 0.0)),
            PathVertex::new(Vec2::new(10.0, 0.0)),
            PathVertex::new(Vec2::new(5.0, 10.0)),
        ];
        recompute_auto_curves(&mut vertices, true, true, 0.0);
        for v in &vertices {
            assert!(v.cp1.is_some());
            assert!(v.cp2.is_some());
        }
    }

    #[test]
    fn test_approximate_length_straight_line() {
        let p0 = Vec2::new(0.0, 0.0);
        let p3 = Vec2::new(10.0, 0.0);
        // Straight line: control points on the line
        let cp1 = Vec2::new(3.33, 0.0);
        let cp2 = Vec2::new(6.67, 0.0);
        let len = approximate_bezier_length(p0, cp1, cp2, p3, 100);
        assert!((len - 10.0).abs() < 0.1);
    }
}
