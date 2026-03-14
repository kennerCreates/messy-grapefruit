use crate::model::Vec2;
use crate::model::sprite::PathVertex;

/// Convert Catmull-Rom control points to cubic bezier control points.
/// Given four Catmull-Rom points (p0, p1, p2, p3), returns the two
/// cubic bezier control points (cp1, cp2) for the segment from p1 to p2.
pub fn catmull_rom_to_cubic(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2) -> (Vec2, Vec2) {
    let cp1 = Vec2 {
        x: p1.x + (p2.x - p0.x) / 6.0,
        y: p1.y + (p2.y - p0.y) / 6.0,
    };
    let cp2 = Vec2 {
        x: p2.x - (p3.x - p1.x) / 6.0,
        y: p2.y - (p3.y - p1.y) / 6.0,
    };
    (cp1, cp2)
}

/// Evaluate a cubic bezier at parameter t.
/// p0 = start, p1 = control1, p2 = control2, p3 = end.
#[allow(dead_code)]
pub fn cubic_bezier_eval(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
    let u = 1.0 - t;
    let tt = t * t;
    let uu = u * u;
    let uuu = uu * u;
    let ttt = tt * t;

    Vec2 {
        x: uuu * p0.x + 3.0 * uu * t * p1.x + 3.0 * u * tt * p2.x + ttt * p3.x,
        y: uuu * p0.y + 3.0 * uu * t * p1.y + 3.0 * u * tt * p2.y + ttt * p3.y,
    }
}

/// De Casteljau split: splits a cubic bezier at parameter t into two cubic beziers.
/// Returns ((left_p0, left_p1, left_p2, left_p3), (right_p0, right_p1, right_p2, right_p3)).
pub fn de_casteljau_split(
    p0: Vec2,
    p1: Vec2,
    p2: Vec2,
    p3: Vec2,
    t: f32,
) -> ((Vec2, Vec2, Vec2, Vec2), (Vec2, Vec2, Vec2, Vec2)) {
    let q0 = p0.lerp(p1, t);
    let q1 = p1.lerp(p2, t);
    let q2 = p2.lerp(p3, t);

    let r0 = q0.lerp(q1, t);
    let r1 = q1.lerp(q2, t);

    let s = r0.lerp(r1, t);

    ((p0, q0, r0, s), (s, r1, q2, p3))
}

/// Flatten a cubic bezier curve into a polyline with the given tolerance.
pub fn flatten_cubic_bezier(
    p0: Vec2,
    p1: Vec2,
    p2: Vec2,
    p3: Vec2,
    tolerance: f32,
    output: &mut Vec<Vec2>,
) {
    flatten_recursive(p0, p1, p2, p3, tolerance, 0, output);
    output.push(p3);
}

fn flatten_recursive(
    p0: Vec2,
    p1: Vec2,
    p2: Vec2,
    p3: Vec2,
    tolerance: f32,
    depth: u32,
    output: &mut Vec<Vec2>,
) {
    if depth > 10 {
        output.push(p0);
        return;
    }

    // Check if the curve is flat enough by testing control point deviation
    let d1 = point_to_line_distance(p1, p0, p3);
    let d2 = point_to_line_distance(p2, p0, p3);

    if d1 + d2 <= tolerance {
        output.push(p0);
        return;
    }

    let (left, right) = de_casteljau_split(p0, p1, p2, p3, 0.5);
    flatten_recursive(left.0, left.1, left.2, left.3, tolerance, depth + 1, output);
    flatten_recursive(right.0, right.1, right.2, right.3, tolerance, depth + 1, output);
}

fn point_to_line_distance(point: Vec2, line_start: Vec2, line_end: Vec2) -> f32 {
    let dx = line_end.x - line_start.x;
    let dy = line_end.y - line_start.y;
    let len_sq = dx * dx + dy * dy;

    if len_sq < 1e-10 {
        return point.distance(line_start);
    }

    let cross = (point.x - line_start.x) * dy - (point.y - line_start.y) * dx;
    cross.abs() / len_sq.sqrt()
}

/// Auto-generate cp1/cp2 for all vertices using Catmull-Rom with
/// duplicated-endpoint phantom points (zero curvature at path ends).
pub fn recompute_auto_curves(vertices: &mut [PathVertex]) {
    let n = vertices.len();
    if n < 2 {
        // Clear any existing control points on single vertices
        for v in vertices.iter_mut() {
            v.cp1 = None;
            v.cp2 = None;
        }
        return;
    }

    // Build the Catmull-Rom point list with duplicated endpoints
    let positions: Vec<Vec2> = vertices.iter().map(|v| v.pos).collect();

    for i in 0..n {
        // For Catmull-Rom, we need four points: p_{i-1}, p_i, p_{i+1}, p_{i+2}
        // Duplicated-endpoint phantom: clamp indices to [0, n-1]
        let p0 = positions[if i == 0 { 0 } else { i - 1 }];
        let p1 = positions[i];
        let p2 = positions[if i + 1 < n { i + 1 } else { n - 1 }];
        let p3 = positions[if i + 2 < n { i + 2 } else { n - 1 }];

        let (cp1, cp2) = catmull_rom_to_cubic(p0, p1, p2, p3);

        // cp1 is the outgoing control point for vertex i (stored as cp2 on vertex i)
        // cp2 is the incoming control point for vertex i+1 (stored as cp1 on vertex i+1)
        // But for the Catmull-Rom approach, we compute per-segment.
        // We store: vertex[i].cp2 = outgoing handle, vertex[i].cp1 = incoming handle.

        // For segment i -> i+1: cp1 belongs as outgoing from vertex i, cp2 as incoming to vertex i+1
        if i + 1 < n {
            vertices[i].cp2 = Some(cp1);
            vertices[i + 1].cp1 = Some(cp2);
        }
    }

    // First vertex has no incoming handle
    vertices[0].cp1 = None;
    // Last vertex has no outgoing handle
    vertices[n - 1].cp2 = None;
}

/// Get the bezier points for a segment between two vertices.
/// Returns (start, cp1, cp2, end) for cubic bezier rendering.
pub fn segment_bezier_points(v0: &PathVertex, v1: &PathVertex) -> (Vec2, Vec2, Vec2, Vec2) {
    let start = v0.pos;
    let end = v1.pos;
    let cp1 = v0.cp2.unwrap_or(start);
    let cp2 = v1.cp1.unwrap_or(end);
    (start, cp1, cp2, end)
}
