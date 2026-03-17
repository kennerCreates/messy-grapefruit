use crate::model::sprite::PathVertex;
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

/// Mirror a single vertex (creates a new vertex with new UUID).
pub fn mirror_vertex(v: &PathVertex, axis: SymmetryAxis, axis_pos: &Vec2) -> PathVertex {
    let mut mirrored = PathVertex::new(mirror_point(v.pos, axis, axis_pos));
    // Swap cp1/cp2 to maintain proper path direction when vertices are reversed
    mirrored.cp1 = mirror_cp(&v.cp2, axis, axis_pos);
    mirrored.cp2 = mirror_cp(&v.cp1, axis, axis_pos);
    mirrored.manual_handles = v.manual_handles;
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
}
