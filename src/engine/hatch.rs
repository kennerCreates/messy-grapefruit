use crate::model::project::{HatchPattern, PatternType};
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

/// Generate full hatch fill data for an element given a pattern.
pub fn generate_element_hatch(
    element: &StrokeElement,
    pattern: &HatchPattern,
) -> Vec<HatchRenderData> {
    let polygon = build_element_polygon(element);
    if polygon.len() < 3 {
        return Vec::new();
    }

    match pattern.pattern_type {
        PatternType::Lines => generate_lines_pattern(&polygon, pattern),
        PatternType::CrossHatch => generate_cross_hatch_pattern(&polygon, pattern),
        PatternType::Brick => generate_brick_pattern(&polygon, pattern),
    }
}

fn generate_lines_pattern(polygon: &[Vec2], pattern: &HatchPattern) -> Vec<HatchRenderData> {
    let mut result = Vec::new();
    for hatch_layer in &pattern.layers {
        let line_pairs = generate_hatch_lines(polygon, hatch_layer.angle, hatch_layer.spacing, hatch_layer.offset);
        let segments: Vec<Vec<Vec2>> = line_pairs.into_iter().map(|(a, b)| vec![a, b]).collect();
        result.push(HatchRenderData { segments });
    }
    result
}

fn generate_cross_hatch_pattern(polygon: &[Vec2], pattern: &HatchPattern) -> Vec<HatchRenderData> {
    let layer = match pattern.layers.first() {
        Some(l) => l,
        None => return Vec::new(),
    };
    let mut result = Vec::new();

    // First direction
    let lines1 = generate_hatch_lines(polygon, layer.angle, layer.spacing, layer.offset);
    let segs1: Vec<Vec<Vec2>> = lines1.into_iter().map(|(a, b)| vec![a, b]).collect();
    result.push(HatchRenderData { segments: segs1 });

    // Second direction: perpendicular, or vertical (90°) in iso mode
    let perp_angle = if pattern.iso_mode { 90.0 } else { (layer.angle + 90.0) % 180.0 };
    let lines2 = generate_hatch_lines(polygon, perp_angle, layer.spacing, layer.offset);
    let segs2: Vec<Vec<Vec2>> = lines2.into_iter().map(|(a, b)| vec![a, b]).collect();
    result.push(HatchRenderData { segments: segs2 });

    result
}

/// Generate a brick tiling pattern: course lines with staggered joints.
fn generate_brick_pattern(polygon: &[Vec2], pattern: &HatchPattern) -> Vec<HatchRenderData> {
    let layer = match pattern.layers.first() {
        Some(l) => l,
        None => return Vec::new(),
    };
    let row_height = layer.spacing;
    let brick_width = pattern.brick_width;
    let angle = layer.angle;

    if row_height < 0.1 || brick_width < 0.1 {
        return Vec::new();
    }

    let mut all_segments = Vec::new();

    // 1. Course lines at the pattern angle
    let h_lines = generate_hatch_lines(polygon, angle, row_height, layer.offset);
    for (a, b) in &h_lines {
        all_segments.push(vec![*a, *b]);
    }

    // 2. Joint segments between courses
    if pattern.iso_mode {
        // Iso mode: joints are truly vertical (90°) lines clipped between adjacent courses.
        // For each course gap, find where vertical lines at brick_width intervals intersect
        // the two adjacent course lines.
        generate_iso_brick_joints(
            polygon, &h_lines, angle, row_height, brick_width, layer.offset, &mut all_segments,
        );
    } else {
        // Normal mode: joints are perpendicular to course direction
        generate_rotated_brick_joints(
            polygon, angle, row_height, brick_width, layer.offset, &mut all_segments,
        );
    }

    vec![HatchRenderData { segments: all_segments }]
}

/// Normal brick joints: perpendicular to course angle, in rotated space.
fn generate_rotated_brick_joints(
    polygon: &[Vec2],
    angle: f32,
    row_height: f32,
    brick_width: f32,
    offset: f32,
    segments: &mut Vec<Vec<Vec2>>,
) {
    let angle_rad = angle.to_radians();
    let cos_a = angle_rad.cos();
    let sin_a = angle_rad.sin();

    // Rotate polygon into course-aligned space
    let rotated: Vec<Vec2> = polygon
        .iter()
        .map(|p| Vec2::new(p.x * cos_a + p.y * sin_a, -p.x * sin_a + p.y * cos_a))
        .collect();

    let (mut min_x, mut max_x, mut min_y, mut max_y) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    for p in &rotated {
        min_x = min_x.min(p.x);
        max_x = max_x.max(p.x);
        min_y = min_y.min(p.y);
        max_y = max_y.max(p.y);
    }

    let start_row = ((min_y + offset) / row_height).floor() as i32;
    let end_row = ((max_y + offset) / row_height).ceil() as i32;

    for row in start_row..end_row {
        let y_top = row as f32 * row_height - offset;
        let y_bot = y_top + row_height;
        let x_offset = if row % 2 != 0 { brick_width * 0.5 } else { 0.0 };

        let start_col = ((min_x - x_offset) / brick_width).floor() as i32;
        let end_col = ((max_x - x_offset) / brick_width).ceil() as i32;

        for col in start_col..=end_col {
            let x = col as f32 * brick_width + x_offset;
            // Rotate back to world space
            let top_world = Vec2::new(x * cos_a - y_top * sin_a, x * sin_a + y_top * cos_a);
            let bot_world = Vec2::new(x * cos_a - y_bot * sin_a, x * sin_a + y_bot * cos_a);

            for (ca, cb) in clip_segment_to_polygon(top_world, bot_world, polygon) {
                segments.push(vec![ca, cb]);
            }
        }
    }
}

/// Iso brick joints: truly vertical (90°) lines spanning between adjacent course lines.
/// For angled courses, each vertical joint intersects the upper and lower course line
/// at different x positions, creating the proper isometric brick look.
fn generate_iso_brick_joints(
    polygon: &[Vec2],
    course_lines: &[(Vec2, Vec2)],
    angle: f32,
    row_height: f32,
    brick_width: f32,
    offset: f32,
    segments: &mut Vec<Vec<Vec2>>,
) {
    if course_lines.is_empty() {
        return;
    }

    // We need the y-positions of course lines in world space.
    // Course lines are at angle `angle` with spacing `row_height`.
    // A point on course row `r` satisfies: x*sin(a) - y*cos(a) = -r*row_height + offset
    // (perpendicular distance from origin along the normal to the angle direction)
    let angle_rad = angle.to_radians();
    let cos_a = angle_rad.cos();
    let sin_a = angle_rad.sin();

    // Polygon AABB for x range
    let (mut min_x, mut max_x) = (f32::MAX, f32::MIN);
    let (mut min_proj, mut max_proj) = (f32::MAX, f32::MIN);
    for p in polygon {
        min_x = min_x.min(p.x);
        max_x = max_x.max(p.x);
        // Project onto the normal (perpendicular to course direction)
        let proj = -p.x * sin_a + p.y * cos_a;
        min_proj = min_proj.min(proj);
        max_proj = max_proj.max(proj);
    }

    // For each pair of adjacent course rows, generate vertical joints
    let start_row = ((min_proj + offset) / row_height).floor() as i32;
    let end_row = ((max_proj + offset) / row_height).ceil() as i32;

    for row in start_row..end_row {
        let proj_top = row as f32 * row_height - offset;
        let proj_bot = proj_top + row_height;

        // Stagger
        let x_off = if row % 2 != 0 { brick_width * 0.5 } else { 0.0 };

        let start_col = ((min_x - x_off) / brick_width).floor() as i32;
        let end_col = ((max_x - x_off) / brick_width).ceil() as i32;

        for col in start_col..=end_col {
            let x = col as f32 * brick_width + x_off;

            // Where does the vertical line x intersect course at proj_top?
            // -x*sin_a + y*cos_a = proj_top => y = (proj_top + x*sin_a) / cos_a
            // For cos_a ≈ 0 (angle ≈ 90°), courses are nearly vertical — joints degenerate
            if cos_a.abs() < 0.01 {
                continue;
            }
            let y_top = (proj_top + x * sin_a) / cos_a;
            let y_bot = (proj_bot + x * sin_a) / cos_a;

            let top_world = Vec2::new(x, y_top);
            let bot_world = Vec2::new(x, y_bot);

            for (ca, cb) in clip_segment_to_polygon(top_world, bot_world, polygon) {
                segments.push(vec![ca, cb]);
            }
        }
    }
}

/// Clip a line segment to a polygon, returning the visible sub-segments.
/// Uses parametric intersection: find all t values where the segment crosses
/// polygon edges, sort them, then keep sub-segments whose midpoints are inside.
fn clip_segment_to_polygon(a: Vec2, b: Vec2, polygon: &[Vec2]) -> Vec<(Vec2, Vec2)> {
    let n = polygon.len();
    if n < 3 {
        return Vec::new();
    }

    let dir = b - a;
    let len_sq = dir.dot(dir);
    if len_sq < 1e-10 {
        return Vec::new();
    }

    // Collect parametric t values where segment intersects polygon edges
    let mut ts = vec![0.0_f32, 1.0];
    for i in 0..n {
        let p0 = polygon[i];
        let p1 = polygon[(i + 1) % n];
        let edge = p1 - p0;

        // Solve: a + t*dir = p0 + s*edge
        let denom = dir.x * edge.y - dir.y * edge.x;
        if denom.abs() < 1e-10 {
            continue; // parallel
        }
        let dp = p0 - a;
        let t = (dp.x * edge.y - dp.y * edge.x) / denom;
        let s = (dp.x * dir.y - dp.y * dir.x) / denom;

        if t > -0.001 && t < 1.001 && s > -0.001 && s < 1.001 {
            ts.push(t.clamp(0.0, 1.0));
        }
    }

    ts.sort_by(|a, b| a.partial_cmp(b).unwrap());
    ts.dedup_by(|a, b| (*a - *b).abs() < 1e-6);

    let mut result = Vec::new();
    for i in 0..ts.len() - 1 {
        let t0 = ts[i];
        let t1 = ts[i + 1];
        if (t1 - t0) < 1e-6 {
            continue;
        }
        let mid_t = (t0 + t1) * 0.5;
        let mid = a.lerp(b, mid_t);
        if point_in_polygon(mid, polygon) {
            result.push((a.lerp(b, t0), a.lerp(b, t1)));
        }
    }
    result
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
