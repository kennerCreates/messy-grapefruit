use crate::model::sprite::PathVertex;
use crate::model::vec2::Vec2;

/// Convert Catmull-Rom spline segment to cubic bezier control points.
/// Given four points (p0, p1, p2, p3), returns (cp1, cp2) for the cubic bezier
/// from p1 to p2.
#[allow(dead_code)] // Used by curve mode tests; may inline into recompute_auto_curves
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

    // If endpoints coincide on an open path, treat them like a closed path for tangent
    // computation so both endpoints get the same two-sided corner radius as interior vertices.
    let endpoints_coincide =
        !closed && n >= 3 && positions[0].distance(positions[n - 1]) < 0.5;

    for i in 0..n {
        if vertices[i].manual_handles {
            // Enforce minimum curvature radius on manual handles using the same
            // angle-based formula as straight-mode fillets: d = R / tan(θ/2).
            // θ is the angle between the two handle directions at the vertex.
            // When handles form a sharp kink (small θ), longer tangents are required.
            if curve_mode && min_corner_radius > 0.001
                && let (Some(cp1), Some(cp2)) = (vertices[i].cp1, vertices[i].cp2)
            {
                let to_cp1 = cp1 - positions[i];
                let to_cp2 = cp2 - positions[i];
                let len1 = to_cp1.length();
                let len2 = to_cp2.length();

                if len1 > 0.001 && len2 > 0.001 {
                    let cos_theta = (to_cp1.dot(to_cp2) / (len1 * len2)).clamp(-1.0, 1.0);
                    let theta = cos_theta.acos();
                    let half_theta_tan = (theta / 2.0).tan();

                    if half_theta_tan > 0.001 {
                        let min_tangent_len = min_corner_radius / half_theta_tan;
                        if len1 < min_tangent_len {
                            vertices[i].cp1 = Some(positions[i] + to_cp1 * (min_tangent_len / len1));
                        }
                        if len2 < min_tangent_len {
                            vertices[i].cp2 = Some(positions[i] + to_cp2 * (min_tangent_len / len2));
                        }
                    }
                }
            }
            continue;
        }

        let (p_prev, p_next) = if closed {
            (positions[(i + n - 1) % n], positions[(i + 1) % n])
        } else if endpoints_coincide && (i == 0 || i == n - 1) {
            // Both endpoints are at the same position — use wrapped neighbors
            (positions[n - 2], positions[1])
        } else {
            let p_prev = if i > 0 { positions[i - 1] } else { positions[0] };
            let p_next = if i + 1 < n { positions[i + 1] } else { positions[n - 1] };
            (p_prev, p_next)
        };

        if curve_mode {
            // Catmull-Rom: tangent along bisector direction (p_next - p_prev)
            let direction = p_next - p_prev;
            let dir_len = direction.length();

            if dir_len < 0.001 {
                vertices[i].cp1 = None;
                vertices[i].cp2 = None;
                continue;
            }

            let tangent_length = (dir_len / 6.0).max(min_corner_radius);
            let mut tangent = direction * (tangent_length / dir_len);
            // Invert tangent to flip curve from convex to concave
            if vertices[i].invert_curve {
                tangent = tangent * -1.0;
            }
            vertices[i].cp1 = Some(positions[i] - tangent);
            // Sharp vertex: keep cp1 (incoming handle) but clear cp2
            // so the outgoing segment renders as a straight line
            if vertices[i].sharp {
                vertices[i].cp2 = None;
            } else {
                vertices[i].cp2 = Some(positions[i] + tangent);
            }
        } else {
            // Figma-style corner rounding.
            // cp1 = tangent point on incoming edge, cp2 = tangent point on outgoing edge.
            // R is the arc radius; tangent distance d = R / tan(θ/2).
            let to_prev = p_prev - positions[i];
            let to_next = p_next - positions[i];
            let dist_prev = to_prev.length();
            let dist_next = to_next.length();

            // No corner to round at open-path endpoints or if radius is zero
            if dist_prev < 0.001 || dist_next < 0.001 || min_corner_radius < 0.001 {
                vertices[i].cp1 = None;
                vertices[i].cp2 = None;
                continue;
            }

            // Angle between edges at this vertex
            let cos_theta = to_prev.dot(to_next) / (dist_prev * dist_next);
            let cos_theta = cos_theta.clamp(-1.0, 1.0);
            let theta = cos_theta.acos();

            // For nearly straight angles (θ → π), no visible rounding needed
            let half_theta_tan = (theta / 2.0).tan();
            if half_theta_tan < 0.001 {
                vertices[i].cp1 = None;
                vertices[i].cp2 = None;
                continue;
            }

            // Tangent distance: d = R / tan(θ/2), clamped to half of shortest edge
            let d = (min_corner_radius / half_theta_tan)
                .min(dist_prev / 2.0)
                .min(dist_next / 2.0);

            vertices[i].cp1 = Some(positions[i] + to_prev * (d / dist_prev));
            vertices[i].cp2 = Some(positions[i] + to_next * (d / dist_next));
        }
    }
}

/// Compute cubic bezier control points for a fillet arc at a rounded corner.
/// `t1` = tangent point on incoming edge (vertex.cp1)
/// `t2` = tangent point on outgoing edge (vertex.cp2)
/// `v` = vertex position (the sharp corner)
/// Returns (arc_cp1, arc_cp2) for a bezier from t1 to t2 that approximates a circular arc.
pub fn fillet_arc_control_points(t1: Vec2, t2: Vec2, v: Vec2) -> (Vec2, Vec2) {
    let d1 = t1 - v;
    let d2 = t2 - v;
    let len1 = d1.length();
    let len2 = d2.length();

    if len1 < 0.001 || len2 < 0.001 {
        return (t1, t2);
    }

    // Angle between edges at V
    let cos_theta = d1.dot(d2) / (len1 * len2);
    let cos_theta = cos_theta.clamp(-1.0, 1.0);
    let theta = cos_theta.acos();

    // Arc sweep angle (how much the path turns at this corner)
    let alpha = std::f32::consts::PI - theta;

    if alpha.abs() < 0.001 {
        // Nearly straight — trivial arc
        return (t1, t2);
    }

    // Bezier approximation factor: ratio = (4/3) * tan(θ/2) * tan(α/4)
    // This places control points on the edge lines between T and V.
    let ratio = (4.0 / 3.0) * (theta / 2.0).tan() * (alpha / 4.0).tan();

    let arc_cp1 = t1 + (v - t1) * ratio;
    let arc_cp2 = t2 + (v - t2) * ratio;

    (arc_cp1, arc_cp2)
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
    fn test_fillet_arc_right_angle() {
        // 90° corner at origin, edges along +x and +y
        let v = Vec2::new(0.0, 0.0);
        let t1 = Vec2::new(5.0, 0.0); // tangent point on +x edge
        let t2 = Vec2::new(0.0, 5.0); // tangent point on +y edge
        let (cp1, cp2) = fillet_arc_control_points(t1, t2, v);
        // For 90° corner, kappa ≈ 0.5523. Control points should be ~55% from T toward V.
        // cp1 = t1 + 0.5523 * (v - t1) = (5,0) + 0.5523 * (-5,0) = (5-2.76, 0) = (2.24, 0)
        let expected_kappa = 0.5523;
        assert!((cp1.x - (5.0 * (1.0 - expected_kappa))).abs() < 0.02);
        assert!(cp1.y.abs() < 0.01);
        assert!(cp2.x.abs() < 0.01);
        assert!((cp2.y - (5.0 * (1.0 - expected_kappa))).abs() < 0.02);
    }

    #[test]
    fn test_fillet_arc_straight_line() {
        // Nearly straight (θ ≈ π): fillet should be trivial
        let v = Vec2::new(0.0, 0.0);
        let t1 = Vec2::new(-5.0, 0.0);
        let t2 = Vec2::new(5.0, 0.0);
        let (cp1, cp2) = fillet_arc_control_points(t1, t2, v);
        // Nearly no arc needed — control points should be very close to tangent points
        assert!((cp1.x - t1.x).abs() < 0.1);
        assert!((cp2.x - t2.x).abs() < 0.1);
    }

    #[test]
    fn test_recompute_straight_radius_zero_gives_sharp() {
        // With radius 0, all vertices should have no control points (sharp corners)
        let mut vertices = vec![
            PathVertex::new(Vec2::new(0.0, 0.0)),
            PathVertex::new(Vec2::new(10.0, 0.0)),
            PathVertex::new(Vec2::new(10.0, 10.0)),
            PathVertex::new(Vec2::new(0.0, 10.0)),
        ];
        recompute_auto_curves(&mut vertices, true, false, 0.0);
        for v in &vertices {
            assert!(v.cp1.is_none(), "cp1 should be None with radius 0");
            assert!(v.cp2.is_none(), "cp2 should be None with radius 0");
        }
    }

    #[test]
    fn test_recompute_straight_radius_produces_tangent_points() {
        // Square with radius 3 — each 90° corner should get tangent points
        let mut vertices = vec![
            PathVertex::new(Vec2::new(0.0, 0.0)),
            PathVertex::new(Vec2::new(10.0, 0.0)),
            PathVertex::new(Vec2::new(10.0, 10.0)),
            PathVertex::new(Vec2::new(0.0, 10.0)),
        ];
        recompute_auto_curves(&mut vertices, true, false, 3.0);
        // For 90° corner, tan(45°) = 1, so d = R/1 = R = 3
        // Vertex at (10,0): cp1 should be 3 units toward (0,0) → (7,0)
        //                   cp2 should be 3 units toward (10,10) → (10,3)
        let v1 = &vertices[1]; // (10, 0)
        let cp1 = v1.cp1.unwrap();
        let cp2 = v1.cp2.unwrap();
        assert!((cp1.x - 7.0).abs() < 0.01, "cp1.x = {} expected 7.0", cp1.x);
        assert!(cp1.y.abs() < 0.01, "cp1.y = {} expected 0.0", cp1.y);
        assert!((cp2.x - 10.0).abs() < 0.01, "cp2.x = {} expected 10.0", cp2.x);
        assert!((cp2.y - 3.0).abs() < 0.01, "cp2.y = {} expected 3.0", cp2.y);
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
