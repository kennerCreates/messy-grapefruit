use crate::engine::transform;
use crate::model::project::GridMode;
use crate::model::sprite::Sprite;
use crate::model::vec2::Vec2;

/// Snap to the nearest existing vertex on any visible, unlocked layer.
/// Returns the world-space position and vertex ID if within threshold.
pub fn snap_to_vertex(
    pos: Vec2,
    sprite: &Sprite,
    threshold_world: f32,
    exclude_element_id: Option<&str>,
    solo_layer_id: Option<&str>,
) -> Option<(Vec2, String)> {
    let mut best: Option<(f32, Vec2, String)> = None;

    for layer in &sprite.layers {
        if !layer.visible || layer.locked {
            continue;
        }
        if let Some(solo_id) = solo_layer_id
            && layer.id != solo_id
        {
            continue;
        }
        for element in &layer.elements {
            if let Some(exclude_id) = exclude_element_id
                && element.id == exclude_id
            {
                continue;
            }
            for vertex in &element.vertices {
                let world_pos = transform::vertex_world_pos(vertex, element);
                let dist = pos.distance(world_pos);
                if dist < threshold_world
                    && (best.is_none() || dist < best.as_ref().unwrap().0)
                {
                    best = Some((dist, world_pos, vertex.id.clone()));
                }
            }
        }
    }

    best.map(|(_, pos, id)| (pos, id))
}

/// Snap a world-space position to the nearest grid point.
pub fn snap_to_grid(pos: Vec2, grid_size: u32, grid_mode: GridMode) -> Vec2 {
    let gs = grid_size as f32;
    if gs < 1.0 {
        return pos;
    }

    // True isometric diamond lattice (30° angles).
    // Lattice basis: u = (√3·gs, gs), v = (√3·gs, -gs).
    // Transform to lattice coordinates, round, transform back.
    let _ = grid_mode;
    let sqrt3 = 3.0_f32.sqrt();
    let ux = sqrt3 * gs;
    // s = dot(pos, u) / dot(u, u), t = dot(pos, v) / dot(v, v)
    // u = (ux, gs), v = (ux, -gs), dot(u,u) = dot(v,v) = 3gs² + gs² = 4gs²
    let denom = 2.0 * (ux * ux + gs * gs); // = 2 * 4gs² = 8gs²
    let s = (ux * pos.x + gs * pos.y) / denom;
    let t = (ux * pos.x - gs * pos.y) / denom;
    let sr = s.round();
    let tr = t.round();
    Vec2::new(
        ux * (sr + tr),
        gs * (sr - tr),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snap_origin() {
        // Near origin should snap to (0, 0)
        let snapped = snap_to_grid(Vec2::new(1.0, 1.0), 8, GridMode::Straight);
        assert!(snapped.x.abs() < 0.01);
        assert!(snapped.y.abs() < 0.01);
    }

    #[test]
    fn test_snap_exact_lattice_point() {
        // True iso lattice: u = (√3·8, 8). Point (s=1, t=0) = (√3·8, 8).
        let gs = 8.0_f32;
        let ux = 3.0_f32.sqrt() * gs;
        let point = Vec2::new(ux, gs);
        let snapped = snap_to_grid(point, 8, GridMode::Isometric);
        assert!((snapped.x - ux).abs() < 0.01);
        assert!((snapped.y - gs).abs() < 0.01);
    }

    #[test]
    fn test_snap_near_lattice_point() {
        // Slightly off from origin should snap back to origin
        let snapped = snap_to_grid(Vec2::new(2.0, 1.5), 8, GridMode::Isometric);
        assert!(snapped.x.abs() < 0.01);
        assert!(snapped.y.abs() < 0.01);
    }

    #[test]
    fn test_snap_symmetry() {
        // Points (s=1,t=0) and (s=0,t=1) should be symmetric about x-axis
        let gs = 8.0_f32;
        let ux = 3.0_f32.sqrt() * gs;
        let p1 = snap_to_grid(Vec2::new(ux + 0.5, gs + 0.5), 8, GridMode::Off);
        let p2 = snap_to_grid(Vec2::new(ux + 0.5, -gs + 0.5), 8, GridMode::Off);
        assert!((p1.x - p2.x).abs() < 0.01);
        assert!((p1.y + p2.y).abs() < 0.01);
    }
}
