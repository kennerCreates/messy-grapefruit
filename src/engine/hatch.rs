use crate::model::project::HatchPattern;
use crate::model::sprite::StrokeElement;
use crate::model::vec2::Vec2;

/// Output data for rendering one hatch layer.
/// Color and stroke width come from the element's stroke properties.
pub struct HatchRenderData {
    /// Each entry is a polyline segment (2+ points for straight, more for warped).
    pub segments: Vec<Vec<Vec2>>,
}

/// Generate clipped hatch line segments for a single hatch layer within a polygon.
///
/// Algorithm (rotation-based scanline clipping):
/// 1. Rotate polygon by -angle so hatch lines become horizontal
/// 2. Compute AABB of rotated polygon
/// 3. Generate horizontal scanlines spaced by `spacing`
/// 4. For each scanline: intersect with polygon edges, sort, pair into segments
/// 5. Rotate segments back by +angle
pub fn generate_hatch_lines(
    polygon: &[Vec2],
    angle_deg: f32,
    spacing: f32,
    offset: f32,
) -> Vec<(Vec2, Vec2)> {
    if polygon.len() < 3 || spacing < 0.1 {
        return Vec::new();
    }

    let angle_rad = angle_deg.to_radians();
    let cos_a = angle_rad.cos();
    let sin_a = angle_rad.sin();

    // Rotate polygon by -angle (so hatch lines become horizontal)
    let rotated: Vec<Vec2> = polygon
        .iter()
        .map(|p| Vec2::new(p.x * cos_a + p.y * sin_a, -p.x * sin_a + p.y * cos_a))
        .collect();

    // Compute AABB of rotated polygon
    let mut min_y = f32::MAX;
    let mut max_y = f32::MIN;
    for p in &rotated {
        min_y = min_y.min(p.y);
        max_y = max_y.max(p.y);
    }

    // Generate scanlines
    let start_y = ((min_y + offset) / spacing).ceil() * spacing - offset;
    let mut segments = Vec::new();
    let n = rotated.len();

    let mut y = start_y;
    while y <= max_y {
        // Find all intersections of horizontal line y with polygon edges
        let mut intersections = Vec::new();
        for i in 0..n {
            let p0 = rotated[i];
            let p1 = rotated[(i + 1) % n];

            // Skip horizontal edges
            if (p0.y - p1.y).abs() < 1e-6 {
                continue;
            }

            // Check if scanline crosses this edge
            if (y < p0.y.min(p1.y)) || (y >= p0.y.max(p1.y)) {
                continue;
            }

            // Compute x-intersection
            let t = (y - p0.y) / (p1.y - p0.y);
            let x = p0.x + t * (p1.x - p0.x);
            intersections.push(x);
        }

        intersections.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // Pair up intersections (inside/outside alternation)
        let mut i = 0;
        while i + 1 < intersections.len() {
            let x0 = intersections[i];
            let x1 = intersections[i + 1];
            if (x1 - x0).abs() > 0.01 {
                // Rotate back by +angle to get world-space coordinates
                let wx0 = x0 * cos_a - y * sin_a;
                let wy0 = x0 * sin_a + y * cos_a;
                let wx1 = x1 * cos_a - y * sin_a;
                let wy1 = x1 * sin_a + y * cos_a;
                segments.push((Vec2::new(wx0, wy0), Vec2::new(wx1, wy1)));
            }
            i += 2;
        }

        y += spacing;
    }

    segments
}

/// Build a polygon from an element's vertices (flattened to polyline).
pub fn build_element_polygon(element: &StrokeElement) -> Vec<Vec2> {
    use crate::engine::transform;
    use crate::math;

    let verts = if transform::has_transform(element) {
        transform::transformed_vertices(element)
    } else {
        element.vertices.clone()
    };

    if verts.len() < 3 || !element.closed {
        return Vec::new();
    }

    let mut polygon = Vec::new();
    let tolerance = 0.5; // world-space tolerance for flattening

    if element.curve_mode {
        let seg_count = verts.len(); // closed
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

/// Test if a point is inside a polygon using ray casting.
fn point_in_polygon(point: Vec2, polygon: &[Vec2]) -> bool {
    let n = polygon.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let pi = polygon[i];
        let pj = polygon[j];
        if (pi.y > point.y) != (pj.y > point.y)
            && point.x < (pj.x - pi.x) * (point.y - pi.y) / (pj.y - pi.y) + pi.x
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Check if a line segment midpoint falls inside any mask polygon.
fn segment_is_masked(a: Vec2, b: Vec2, masks: &[Vec<Vec2>]) -> bool {
    if masks.is_empty() {
        return false;
    }
    let mid = a.lerp(b, 0.5);
    masks.iter().any(|mask| point_in_polygon(mid, mask))
}

/// Generate full hatch fill data for an element given a pattern.
pub fn generate_element_hatch(
    element: &StrokeElement,
    pattern: &HatchPattern,
) -> Vec<HatchRenderData> {
    let polygon = build_element_polygon(element);
    if polygon.len() < 3 {
        return Vec::new();
    }

    let mut result = Vec::new();

    for hatch_layer in &pattern.layers {
        let line_pairs = generate_hatch_lines(
            &polygon,
            hatch_layer.angle,
            hatch_layer.spacing,
            hatch_layer.offset,
        );

        // Filter out segments whose midpoints fall inside mask polygons
        let segments: Vec<Vec<Vec2>> = line_pairs
            .into_iter()
            .filter(|(a, b)| !segment_is_masked(*a, *b, &element.hatch_masks))
            .map(|(a, b)| vec![a, b])
            .collect();

        result.push(HatchRenderData { segments });
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square_polygon() -> Vec<Vec2> {
        vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(100.0, 0.0),
            Vec2::new(100.0, 100.0),
            Vec2::new(0.0, 100.0),
        ]
    }

    #[test]
    fn test_horizontal_hatch_on_square() {
        let poly = square_polygon();
        let lines = generate_hatch_lines(&poly, 0.0, 10.0, 0.0);
        // Should generate lines at y = 10, 20, ..., 90 (9 interior lines)
        // May include boundary lines depending on float precision
        assert!(
            lines.len() >= 9 && lines.len() <= 11,
            "Expected ~9-10 horizontal lines, got {}",
            lines.len()
        );
        // Each line should span from x~0 to x~100
        for (a, b) in &lines {
            assert!(a.x < 1.0, "Start x should be near 0, got {}", a.x);
            assert!(b.x > 99.0, "End x should be near 100, got {}", b.x);
        }
    }

    #[test]
    fn test_45_degree_hatch_on_square() {
        let poly = square_polygon();
        let lines = generate_hatch_lines(&poly, 45.0, 10.0, 0.0);
        assert!(!lines.is_empty(), "Should produce diagonal hatch lines");
    }

    #[test]
    fn test_cross_hatch_two_layers() {
        let poly = square_polygon();
        let lines_45 = generate_hatch_lines(&poly, 45.0, 10.0, 0.0);
        let lines_135 = generate_hatch_lines(&poly, 135.0, 10.0, 0.0);
        assert!(!lines_45.is_empty());
        assert!(!lines_135.is_empty());
    }

    #[test]
    fn test_empty_polygon() {
        let lines = generate_hatch_lines(&[], 0.0, 10.0, 0.0);
        assert!(lines.is_empty());
    }

    #[test]
    fn test_degenerate_polygon() {
        // Collinear points
        let poly = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(50.0, 0.0),
            Vec2::new(100.0, 0.0),
        ];
        let lines = generate_hatch_lines(&poly, 0.0, 10.0, 0.0);
        assert!(lines.is_empty(), "Degenerate polygon should produce no hatch lines");
    }

    #[test]
    fn test_spacing_too_small() {
        let poly = square_polygon();
        let lines = generate_hatch_lines(&poly, 0.0, 0.05, 0.0);
        assert!(lines.is_empty(), "Spacing below 0.1 should produce no lines");
    }

}
