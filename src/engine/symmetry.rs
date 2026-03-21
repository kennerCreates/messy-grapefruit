use crate::model::sprite::{GradientFill, PathVertex, StrokeElement};
use crate::model::vec2::Vec2;
use crate::state::editor::SymmetryAxis;

/// Mirror a point across the given axis.
pub fn mirror_point(pos: Vec2, axis: SymmetryAxis, axis_pos: &Vec2) -> Vec2 {
    match axis {
        SymmetryAxis::Vertical => Vec2::new(2.0 * axis_pos.x - pos.x, pos.y),
        SymmetryAxis::Horizontal => Vec2::new(pos.x, 2.0 * axis_pos.y - pos.y),
        SymmetryAxis::Both => Vec2::new(2.0 * axis_pos.x - pos.x, 2.0 * axis_pos.y - pos.y),
    }
}

/// Mirror an optional control point position.
fn mirror_cp(cp: &Option<Vec2>, axis: SymmetryAxis, axis_pos: &Vec2) -> Option<Vec2> {
    cp.map(|p| mirror_point(p, axis, axis_pos))
}

/// Mirror a gradient fill to match a mirrored element.
/// Flips the angle and spatial parameters based on the symmetry axis.
fn mirror_gradient(grad: &GradientFill, axis: SymmetryAxis) -> GradientFill {
    let mut m = grad.clone();
    match axis {
        SymmetryAxis::Vertical => {
            // Mirror across vertical axis: negate angle (flip horizontal direction)
            m.angle_rad = -grad.angle_rad;
            if let Some(ref mut c) = m.center { c.x = 1.0 - c.x; }
            if let Some(ref mut f) = m.focal_offset { f.x = 1.0 - f.x; }
            if let Some(ref mut s) = m.line_start { s.x = 1.0 - s.x; }
            if let Some(ref mut e) = m.line_end { e.x = 1.0 - e.x; }
        }
        SymmetryAxis::Horizontal => {
            // Mirror across horizontal axis: negate angle (flip vertical direction)
            m.angle_rad = -grad.angle_rad;
            if let Some(ref mut c) = m.center { c.y = 1.0 - c.y; }
            if let Some(ref mut f) = m.focal_offset { f.y = 1.0 - f.y; }
            if let Some(ref mut s) = m.line_start { s.y = 1.0 - s.y; }
            if let Some(ref mut e) = m.line_end { e.y = 1.0 - e.y; }
        }
        SymmetryAxis::Both => {
            // Mirror across both axes: rotate 180°
            m.angle_rad = grad.angle_rad + std::f32::consts::PI;
            if let Some(ref mut c) = m.center { c.x = 1.0 - c.x; c.y = 1.0 - c.y; }
            if let Some(ref mut f) = m.focal_offset { f.x = 1.0 - f.x; f.y = 1.0 - f.y; }
            if let Some(ref mut s) = m.line_start { s.x = 1.0 - s.x; s.y = 1.0 - s.y; }
            if let Some(ref mut e) = m.line_end { e.x = 1.0 - e.x; e.y = 1.0 - e.y; }
        }
    }
    m
}

/// Mirror a single vertex (creates a new vertex with new UUID).
pub fn mirror_vertex(v: &PathVertex, axis: SymmetryAxis, axis_pos: &Vec2) -> PathVertex {
    let mut mirrored = PathVertex::new(mirror_point(v.pos, axis, axis_pos));
    // Swap cp1/cp2 to maintain proper path direction when vertices are reversed
    mirrored.cp1 = mirror_cp(&v.cp2, axis, axis_pos);
    mirrored.cp2 = mirror_cp(&v.cp1, axis, axis_pos);
    mirrored.manual_handles = v.manual_handles;
    mirrored.sharp = v.sharp;
    mirrored.invert_curve = v.invert_curve;
    mirrored
}

/// Mirror a sequence of vertices. The result is reversed to maintain visual consistency
/// (mirrored strokes draw in the opposite direction).
pub fn mirror_vertices(verts: &[PathVertex], axis: SymmetryAxis, axis_pos: &Vec2) -> Vec<PathVertex> {
    let mut mirrored: Vec<PathVertex> = verts.iter()
        .map(|v| mirror_vertex(v, axis, axis_pos))
        .collect();
    mirrored.reverse();
    mirrored
}

/// Mirror vertex positions only, preserving order and control point assignment.
/// Use this when the mirrored vertices will be merged into an existing element
/// (where the merge logic handles vertex ordering and deduplication).
#[allow(dead_code)]
pub fn mirror_vertices_in_place(verts: &[PathVertex], axis: SymmetryAxis, axis_pos: &Vec2) -> Vec<PathVertex> {
    verts.iter().map(|v| {
        let mut mirrored = PathVertex::new(mirror_point(v.pos, axis, axis_pos));
        // Keep cp1/cp2 in same slots (no swap) since vertex order is preserved
        mirrored.cp1 = mirror_cp(&v.cp1, axis, axis_pos);
        mirrored.cp2 = mirror_cp(&v.cp2, axis, axis_pos);
        mirrored.manual_handles = v.manual_handles;
        mirrored
    }).collect()
}

/// Check if a vertex position is on (or very near) the symmetry axis.
fn is_on_axis(pos: Vec2, axis: SymmetryAxis, axis_pos: &Vec2, threshold: f32) -> bool {
    match axis {
        SymmetryAxis::Vertical => (pos.x - axis_pos.x).abs() < threshold,
        SymmetryAxis::Horizontal => (pos.y - axis_pos.y).abs() < threshold,
        SymmetryAxis::Both => {
            (pos.x - axis_pos.x).abs() < threshold && (pos.y - axis_pos.y).abs() < threshold
        }
    }
}

/// Snap a vertex position exactly onto the symmetry axis.
fn snap_to_axis(pos: Vec2, axis: SymmetryAxis, axis_pos: &Vec2) -> Vec2 {
    match axis {
        SymmetryAxis::Vertical => Vec2::new(axis_pos.x, pos.y),
        SymmetryAxis::Horizontal => Vec2::new(pos.x, axis_pos.y),
        SymmetryAxis::Both => *axis_pos,
    }
}

/// Result of attempting to join a stroke with its mirror.
pub enum SymmetryResult {
    /// Endpoints touched the axis — joined into a single element.
    Joined(Box<StrokeElement>),
    /// Endpoints did not touch — return separate primary + mirrored elements.
    Separate(Vec<StrokeElement>),
}

/// Try to join a primary element with its mirror across a single axis.
/// If the first or last vertex of the primary stroke is on the axis,
/// the primary and mirrored vertices are concatenated into one path.
///
/// - If only one endpoint is on axis → open joined path
/// - If both endpoints are on axis → closed joined path
pub fn try_join_symmetric(
    element: &StrokeElement,
    axis: SymmetryAxis,
    axis_pos: &Vec2,
    threshold: f32,
    min_corner_radius: f32,
) -> SymmetryResult {
    let verts = &element.vertices;
    if verts.is_empty() {
        return SymmetryResult::Separate(vec![]);
    }

    let first_on = is_on_axis(verts[0].pos, axis, axis_pos, threshold);
    let last_on = is_on_axis(verts[verts.len() - 1].pos, axis, axis_pos, threshold);

    // Already-closed paths: don't attempt join (would produce degenerate result)
    if element.closed {
        let mirrored_verts = mirror_vertices(verts, axis, axis_pos);
        let mut m = StrokeElement::new(mirrored_verts, element.stroke_width, element.stroke_color_index, element.curve_mode);
        m.closed = true;
        m.fill_color_index = element.fill_color_index;
        m.gradient_fill = element.gradient_fill.as_ref().map(|g| mirror_gradient(g, axis));
        m.hatch_fill_id = element.hatch_fill_id.clone();
        crate::math::recompute_auto_curves(&mut m.vertices, m.closed, m.curve_mode, min_corner_radius);
        return SymmetryResult::Separate(vec![m]);
    }

    if !first_on && !last_on {
        // Neither endpoint on axis — return separate mirrored copy
        let mirrored_verts = mirror_vertices(verts, axis, axis_pos);
        let mut m = StrokeElement::new(mirrored_verts, element.stroke_width, element.stroke_color_index, element.curve_mode);
        m.closed = element.closed;
        m.fill_color_index = element.fill_color_index;
        m.gradient_fill = element.gradient_fill.as_ref().map(|g| mirror_gradient(g, axis));
        m.hatch_fill_id = element.hatch_fill_id.clone();
        crate::math::recompute_auto_curves(&mut m.vertices, m.closed, m.curve_mode, min_corner_radius);
        return SymmetryResult::Separate(vec![m]);
    }

    // At least one endpoint is on the axis — join the paths.
    // Mirror vertices are already reversed by mirror_vertices().
    let mirrored = mirror_vertices(verts, axis, axis_pos);

    let mut joined = Vec::new();
    let closed;

    if last_on && first_on {
        // Both endpoints on axis → closed shape.
        // Primary: [A ... Z] where A and Z are on axis
        // Mirrored (reversed): [Z' ... A'] where Z'≈Z, A'≈A
        // Join: [A ... Z, Z'(skip, ≈Z) ... A'(skip, ≈A)] → closed
        joined.extend_from_slice(verts);
        // Snap the last primary vertex onto the axis exactly
        if let Some(last) = joined.last_mut() {
            last.pos = snap_to_axis(last.pos, axis, axis_pos);
        }
        // Append mirrored, skipping the first (≈ last primary) and last (≈ first primary)
        if mirrored.len() > 2 {
            joined.extend_from_slice(&mirrored[1..mirrored.len() - 1]);
        }
        // Snap the first vertex too
        joined[0].pos = snap_to_axis(joined[0].pos, axis, axis_pos);
        closed = true;
    } else if last_on {
        // Only last vertex on axis → open path, joined at the end.
        // Primary: [A ... Z(on axis)]
        // Mirrored (reversed): [Z' ... A']
        // Join: [A ... Z, (skip Z') ... A'] — open path
        joined.extend_from_slice(verts);
        if let Some(last) = joined.last_mut() {
            last.pos = snap_to_axis(last.pos, axis, axis_pos);
        }
        // Skip first mirrored vertex (coincides with snapped last)
        if mirrored.len() > 1 {
            joined.extend_from_slice(&mirrored[1..]);
        }
        closed = false;
    } else {
        // Only first vertex on axis → open path, joined at the start.
        // Mirrored (reversed): [Z' ... A'(on axis)]
        // Primary: [A(on axis) ... Z]
        // Join: [Z' ... A', (skip A) ... Z] — open path
        joined.extend_from_slice(&mirrored);
        if let Some(last) = joined.last_mut() {
            last.pos = snap_to_axis(last.pos, axis, axis_pos);
        }
        // Skip first primary vertex (coincides with snapped last mirrored)
        if verts.len() > 1 {
            joined.extend_from_slice(&verts[1..]);
        }
        closed = false;
    }

    let mut result = StrokeElement::new(joined, element.stroke_width, element.stroke_color_index, element.curve_mode);
    result.closed = closed;
    result.fill_color_index = element.fill_color_index;
    result.gradient_fill = element.gradient_fill.clone();
    result.hatch_fill_id = element.hatch_fill_id.clone();
    crate::math::recompute_auto_curves(&mut result.vertices, result.closed, result.curve_mode, min_corner_radius);
    SymmetryResult::Joined(Box::new(result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mirror_vertical() {
        let axis_pos = Vec2::new(100.0, 50.0);
        let p = Vec2::new(80.0, 30.0);
        let m = mirror_point(p, SymmetryAxis::Vertical, &axis_pos);
        assert!((m.x - 120.0).abs() < 1e-4);
        assert!((m.y - 30.0).abs() < 1e-4);
    }

    #[test]
    fn mirror_horizontal() {
        let axis_pos = Vec2::new(100.0, 50.0);
        let p = Vec2::new(80.0, 30.0);
        let m = mirror_point(p, SymmetryAxis::Horizontal, &axis_pos);
        assert!((m.x - 80.0).abs() < 1e-4);
        assert!((m.y - 70.0).abs() < 1e-4);
    }

    #[test]
    fn mirror_both() {
        let axis_pos = Vec2::new(100.0, 50.0);
        let p = Vec2::new(80.0, 30.0);
        let m = mirror_point(p, SymmetryAxis::Both, &axis_pos);
        assert!((m.x - 120.0).abs() < 1e-4);
        assert!((m.y - 70.0).abs() < 1e-4);
    }

    #[test]
    fn mirror_vertices_reversed() {
        let axis_pos = Vec2::new(50.0, 50.0);
        let verts = vec![
            PathVertex::new(Vec2::new(10.0, 10.0)),
            PathVertex::new(Vec2::new(20.0, 20.0)),
            PathVertex::new(Vec2::new(30.0, 30.0)),
        ];
        let mirrored = mirror_vertices(&verts, SymmetryAxis::Vertical, &axis_pos);
        assert_eq!(mirrored.len(), 3);
        // Reversed: first mirrored vertex should be from the last original
        assert!((mirrored[0].pos.x - 70.0).abs() < 1e-4); // 2*50-30=70
        assert!((mirrored[2].pos.x - 90.0).abs() < 1e-4); // 2*50-10=90
    }

    #[test]
    fn join_last_on_axis() {
        // Stroke ending on vertical axis should produce a single joined element
        let axis_pos = Vec2::new(50.0, 50.0);
        let verts = vec![
            PathVertex::new(Vec2::new(10.0, 10.0)),
            PathVertex::new(Vec2::new(30.0, 20.0)),
            PathVertex::new(Vec2::new(50.0, 30.0)), // on axis
        ];
        let elem = StrokeElement::new(verts, 2.0, 1, false);
        let result = try_join_symmetric(&elem, SymmetryAxis::Vertical, &axis_pos, 5.0, 0.0);
        match result {
            SymmetryResult::Joined(j) => {
                // 3 primary + 2 mirrored (first mirrored skipped) = 5
                assert_eq!(j.vertices.len(), 5);
                assert!(!j.closed);
                // First vertex is original start
                assert!((j.vertices[0].pos.x - 10.0).abs() < 1e-4);
                // Last vertex is mirrored start (2*50-10=90)
                assert!((j.vertices[4].pos.x - 90.0).abs() < 1e-4);
            }
            SymmetryResult::Separate(_) => panic!("Expected joined result"),
        }
    }

    #[test]
    fn join_both_on_axis_closed() {
        // Stroke with both endpoints on axis should produce a closed joined element
        let axis_pos = Vec2::new(50.0, 50.0);
        let verts = vec![
            PathVertex::new(Vec2::new(50.0, 10.0)), // on axis
            PathVertex::new(Vec2::new(30.0, 30.0)),
            PathVertex::new(Vec2::new(50.0, 50.0)), // on axis
        ];
        let elem = StrokeElement::new(verts, 2.0, 1, false);
        let result = try_join_symmetric(&elem, SymmetryAxis::Vertical, &axis_pos, 5.0, 0.0);
        match result {
            SymmetryResult::Joined(j) => {
                // 3 primary + 1 mirrored interior = 4
                assert_eq!(j.vertices.len(), 4);
                assert!(j.closed);
            }
            SymmetryResult::Separate(_) => panic!("Expected joined result"),
        }
    }

    #[test]
    fn no_join_when_off_axis() {
        // Stroke not touching axis should produce separate elements
        let axis_pos = Vec2::new(50.0, 50.0);
        let verts = vec![
            PathVertex::new(Vec2::new(10.0, 10.0)),
            PathVertex::new(Vec2::new(20.0, 20.0)),
            PathVertex::new(Vec2::new(30.0, 30.0)),
        ];
        let elem = StrokeElement::new(verts, 2.0, 1, false);
        let result = try_join_symmetric(&elem, SymmetryAxis::Vertical, &axis_pos, 5.0, 0.0);
        match result {
            SymmetryResult::Separate(mirrors) => {
                assert_eq!(mirrors.len(), 1);
            }
            SymmetryResult::Joined(_) => panic!("Expected separate result"),
        }
    }
}
