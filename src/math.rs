use crate::model::Vec2;

/// Convert Catmull-Rom spline points to cubic bezier control points.
/// Given 4 sequential points (p_prev, p_start, p_end, p_next),
/// returns (cp1, cp2) for the cubic bezier from p_start to p_end.
pub fn catmull_rom_to_cubic_bezier(
    p_prev: Vec2,
    p_start: Vec2,
    p_end: Vec2,
    p_next: Vec2,
) -> (Vec2, Vec2) {
    let cp1 = Vec2 {
        x: p_start.x + (p_end.x - p_prev.x) / 6.0,
        y: p_start.y + (p_end.y - p_prev.y) / 6.0,
    };
    let cp2 = Vec2 {
        x: p_end.x - (p_next.x - p_start.x) / 6.0,
        y: p_end.y - (p_next.y - p_start.y) / 6.0,
    };
    (cp1, cp2)
}

/// Evaluate cubic bezier at t (0..1)
pub fn cubic_bezier_eval(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
    let u = 1.0 - t;
    let u2 = u * u;
    let u3 = u2 * u;
    let t2 = t * t;
    let t3 = t2 * t;
    Vec2 {
        x: u3 * p0.x + 3.0 * u2 * t * p1.x + 3.0 * u * t2 * p2.x + t3 * p3.x,
        y: u3 * p0.y + 3.0 * u2 * t * p1.y + 3.0 * u * t2 * p2.y + t3 * p3.y,
    }
}

/// De Casteljau subdivision: split at t, return two sets of 4 control points.
pub fn cubic_bezier_split(
    p0: Vec2,
    p1: Vec2,
    p2: Vec2,
    p3: Vec2,
    t: f32,
) -> ([Vec2; 4], [Vec2; 4]) {
    let p01 = p0.lerp(p1, t);
    let p12 = p1.lerp(p2, t);
    let p23 = p2.lerp(p3, t);

    let p012 = p01.lerp(p12, t);
    let p123 = p12.lerp(p23, t);

    let p0123 = p012.lerp(p123, t);

    ([p0, p01, p012, p0123], [p0123, p123, p23, p3])
}

/// Flatten a cubic bezier into a polyline with given tolerance.
///
/// Uses recursive subdivision -- if the curve is close enough to a line
/// (max deviation < tolerance), return endpoints, otherwise split and recurse.
pub fn cubic_bezier_flatten(
    p0: Vec2,
    p1: Vec2,
    p2: Vec2,
    p3: Vec2,
    tolerance: f32,
) -> Vec<Vec2> {
    let mut result = vec![p0];
    flatten_recursive(p0, p1, p2, p3, tolerance, &mut result);
    result
}

fn flatten_recursive(
    p0: Vec2,
    p1: Vec2,
    p2: Vec2,
    p3: Vec2,
    tolerance: f32,
    result: &mut Vec<Vec2>,
) {
    // Check if the control points deviate from the straight line p0->p3
    // by more than the tolerance. We measure the max distance from cp1 and cp2
    // to the line segment p0->p3.
    if is_flat_enough(p0, p1, p2, p3, tolerance) {
        result.push(p3);
    } else {
        let (left, right) = cubic_bezier_split(p0, p1, p2, p3, 0.5);
        flatten_recursive(left[0], left[1], left[2], left[3], tolerance, result);
        flatten_recursive(right[0], right[1], right[2], right[3], tolerance, result);
    }
}

/// Returns true if the cubic bezier defined by (p0, p1, p2, p3) is flat enough
/// that it can be approximated by the line segment p0->p3 within the given tolerance.
fn is_flat_enough(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, tolerance: f32) -> bool {
    // Use the quick check: max distance of control points from the chord p0-p3.
    // For a line from p0 to p3, the distance of a point q is:
    //   |cross(p3 - p0, q - p0)| / |p3 - p0|
    let dx = p3.x - p0.x;
    let dy = p3.y - p0.y;
    let len_sq = dx * dx + dy * dy;

    if len_sq < 1e-10 {
        // Degenerate segment: just check distance from p0 to control points.
        let d1 = (p1 - p0).length();
        let d2 = (p2 - p0).length();
        return d1 <= tolerance && d2 <= tolerance;
    }

    let inv_len = 1.0 / len_sq.sqrt();

    // Distance of p1 from chord
    let cross1 = (p1.x - p0.x) * dy - (p1.y - p0.y) * dx;
    let dist1 = cross1.abs() * inv_len;

    // Distance of p2 from chord
    let cross2 = (p2.x - p0.x) * dy - (p2.y - p0.y) * dx;
    let dist2 = cross2.abs() * inv_len;

    dist1 <= tolerance && dist2 <= tolerance
}

/// Recompute control points for all vertices using Catmull-Rom auto-curve.
/// Uses duplicated-endpoint phantom points at path ends.
pub fn recompute_auto_curves(vertices: &mut [crate::model::PathVertex]) {
    let n = vertices.len();
    if n < 2 {
        // No curves to compute with fewer than 2 vertices.
        for v in vertices.iter_mut() {
            v.cp1 = None;
            v.cp2 = None;
        }
        return;
    }

    // Collect positions so we can compute without borrow conflicts.
    let positions: Vec<Vec2> = vertices.iter().map(|v| v.pos).collect();

    for i in 0..(n - 1) {
        // For the segment from vertex i to vertex i+1, we need four points:
        //   p_prev (i-1), p_start (i), p_end (i+1), p_next (i+2)
        // With phantom points: P[-1] = P[0], P[n] = P[n-1]
        let p_prev = if i == 0 { positions[0] } else { positions[i - 1] };
        let p_start = positions[i];
        let p_end = positions[i + 1];
        let p_next = if i + 2 >= n {
            positions[n - 1]
        } else {
            positions[i + 2]
        };

        let (cp1, cp2) = catmull_rom_to_cubic_bezier(p_prev, p_start, p_end, p_next);

        // cp1 is the outgoing control point for vertex i
        // cp2 is the incoming control point for vertex i+1
        vertices[i].cp1 = Some(cp1);
        vertices[i + 1].cp2 = Some(cp2);
    }

    // The first vertex has no incoming control point.
    vertices[0].cp2 = None;
    // The last vertex has no outgoing control point.
    vertices[n - 1].cp1 = None;
}
