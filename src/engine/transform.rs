use crate::model::sprite::{PathVertex, Sprite, StrokeElement};
use crate::model::vec2::Vec2;

/// Apply element transform to a local vertex position.
/// Transform order: (pos - origin) * scale → rotate → + origin + position
pub fn transform_point(pos: Vec2, origin: Vec2, position: Vec2, rotation: f32, scale: Vec2) -> Vec2 {
    let centered = pos - origin;
    let scaled = Vec2::new(centered.x * scale.x, centered.y * scale.y);
    let (sin, cos) = rotation.sin_cos();
    let rotated = Vec2::new(
        scaled.x * cos - scaled.y * sin,
        scaled.x * sin + scaled.y * cos,
    );
    rotated + origin + position
}

/// Inverse of transform_point — converts world position to element-local position.
pub fn inverse_transform_point(
    world_pos: Vec2,
    origin: Vec2,
    position: Vec2,
    rotation: f32,
    scale: Vec2,
) -> Vec2 {
    let translated = world_pos - origin - position;
    let (sin, cos) = rotation.sin_cos();
    let unrotated = Vec2::new(
        translated.x * cos + translated.y * sin,
        -translated.x * sin + translated.y * cos,
    );
    let unscaled = Vec2::new(
        if scale.x.abs() > 1e-6 { unrotated.x / scale.x } else { unrotated.x },
        if scale.y.abs() > 1e-6 { unrotated.y / scale.y } else { unrotated.y },
    );
    unscaled + origin
}

/// Get the world-space position of a vertex using the element's transform.
pub fn vertex_world_pos(vertex: &PathVertex, element: &StrokeElement) -> Vec2 {
    transform_point(vertex.pos, element.origin, element.position, element.rotation, element.scale)
}

/// Transform a control point from local to world space, if present.
pub fn cp_world_pos(cp: Option<Vec2>, element: &StrokeElement) -> Option<Vec2> {
    cp.map(|p| transform_point(p, element.origin, element.position, element.rotation, element.scale))
}

/// Create a transformed copy of vertices for rendering.
pub fn transformed_vertices(element: &StrokeElement) -> Vec<PathVertex> {
    element.vertices.iter().map(|v| PathVertex {
        id: v.id.clone(),
        pos: vertex_world_pos(v, element),
        cp1: cp_world_pos(v.cp1, element),
        cp2: cp_world_pos(v.cp2, element),
        manual_handles: v.manual_handles,
    }).collect()
}

/// Check if an element has a non-identity transform.
pub fn has_transform(element: &StrokeElement) -> bool {
    element.position.x != 0.0
        || element.position.y != 0.0
        || element.rotation != 0.0
        || element.scale.x != 1.0
        || element.scale.y != 1.0
}

/// Compute the AABB of an element's transformed vertices.
pub fn element_bounds(element: &StrokeElement) -> Option<(Vec2, Vec2)> {
    if element.vertices.is_empty() {
        return None;
    }
    let mut min = Vec2::new(f32::MAX, f32::MAX);
    let mut max = Vec2::new(f32::MIN, f32::MIN);
    for v in &element.vertices {
        let world = vertex_world_pos(v, element);
        min = min.min(world);
        max = max.max(world);
    }
    Some((min, max))
}

/// Compute the combined AABB of selected elements.
pub fn selection_bounds(sprite: &Sprite, selected_ids: &[String]) -> Option<(Vec2, Vec2)> {
    let mut min = Vec2::new(f32::MAX, f32::MAX);
    let mut max = Vec2::new(f32::MIN, f32::MIN);
    let mut found = false;

    for layer in &sprite.layers {
        for element in &layer.elements {
            if selected_ids.iter().any(|id| id == &element.id)
                && let Some((emin, emax)) = element_bounds(element) {
                    min = min.min(emin);
                    max = max.max(emax);
                    found = true;
                }
        }
    }

    if found { Some((min, max)) } else { None }
}

/// Find all visible, unlocked elements whose AABB intersects the given rect.
pub fn elements_in_rect(sprite: &Sprite, rect_min: Vec2, rect_max: Vec2) -> Vec<String> {
    let mut result = Vec::new();
    for layer in &sprite.layers {
        if !layer.visible || layer.locked {
            continue;
        }
        for element in &layer.elements {
            if let Some((emin, emax)) = element_bounds(element) {
                // AABB intersection test
                if emin.x <= rect_max.x && emax.x >= rect_min.x
                    && emin.y <= rect_max.y && emax.y >= rect_min.y
                {
                    result.push(element.id.clone());
                }
            }
        }
    }
    result
}

/// Apply a mutation to all elements whose ID is in `selected_ids`.
pub fn for_selected_elements_mut(
    sprite: &mut Sprite,
    selected_ids: &[String],
    mut f: impl FnMut(&mut StrokeElement),
) {
    for layer in sprite.layers.iter_mut() {
        for element in layer.elements.iter_mut() {
            if selected_ids.iter().any(|id| id == &element.id) {
                f(element);
            }
        }
    }
}

/// Find a mutable reference to an element and vertex index by their IDs.
pub fn find_element_vertex_mut<'a>(
    sprite: &'a mut Sprite,
    element_id: &str,
    vertex_id: &str,
) -> Option<(&'a mut StrokeElement, usize)> {
    for layer in sprite.layers.iter_mut() {
        for element in layer.elements.iter_mut() {
            if element.id == element_id {
                if let Some(idx) = element.vertices.iter().position(|v| v.id == vertex_id) {
                    return Some((element, idx));
                }
                return None;
            }
        }
    }
    None
}

/// Recompute auto-curves for all elements in the sprite.
pub fn recompute_all_curves(sprite: &mut Sprite, min_corner_radius: f32) {
    for layer in &mut sprite.layers {
        for element in &mut layer.elements {
            crate::math::recompute_auto_curves(
                &mut element.vertices,
                element.closed,
                element.curve_mode,
                min_corner_radius,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_transform_round_trip() {
        let pos = Vec2::new(10.0, 20.0);
        let origin = Vec2::ZERO;
        let position = Vec2::ZERO;
        let rotation = 0.0;
        let scale = Vec2::ONE;

        let world = transform_point(pos, origin, position, rotation, scale);
        assert!((world.x - pos.x).abs() < 1e-4);
        assert!((world.y - pos.y).abs() < 1e-4);

        let back = inverse_transform_point(world, origin, position, rotation, scale);
        assert!((back.x - pos.x).abs() < 1e-4);
        assert!((back.y - pos.y).abs() < 1e-4);
    }

    #[test]
    fn test_translation_transform() {
        let pos = Vec2::new(5.0, 10.0);
        let origin = Vec2::ZERO;
        let position = Vec2::new(100.0, 200.0);
        let rotation = 0.0;
        let scale = Vec2::ONE;

        let world = transform_point(pos, origin, position, rotation, scale);
        assert!((world.x - 105.0).abs() < 1e-4);
        assert!((world.y - 210.0).abs() < 1e-4);

        let back = inverse_transform_point(world, origin, position, rotation, scale);
        assert!((back.x - pos.x).abs() < 1e-4);
        assert!((back.y - pos.y).abs() < 1e-4);
    }

    #[test]
    fn test_rotation_transform_round_trip() {
        let pos = Vec2::new(10.0, 0.0);
        let origin = Vec2::ZERO;
        let position = Vec2::ZERO;
        let rotation = std::f32::consts::FRAC_PI_2; // 90 degrees
        let scale = Vec2::ONE;

        let world = transform_point(pos, origin, position, rotation, scale);
        // 90° rotation: (10, 0) → (0, 10)
        assert!((world.x - 0.0).abs() < 1e-4);
        assert!((world.y - 10.0).abs() < 1e-4);

        let back = inverse_transform_point(world, origin, position, rotation, scale);
        assert!((back.x - pos.x).abs() < 1e-4);
        assert!((back.y - pos.y).abs() < 1e-4);
    }

    #[test]
    fn test_scale_transform_round_trip() {
        let pos = Vec2::new(5.0, 10.0);
        let origin = Vec2::ZERO;
        let position = Vec2::ZERO;
        let rotation = 0.0;
        let scale = Vec2::new(2.0, 3.0);

        let world = transform_point(pos, origin, position, rotation, scale);
        assert!((world.x - 10.0).abs() < 1e-4);
        assert!((world.y - 30.0).abs() < 1e-4);

        let back = inverse_transform_point(world, origin, position, rotation, scale);
        assert!((back.x - pos.x).abs() < 1e-4);
        assert!((back.y - pos.y).abs() < 1e-4);
    }

    #[test]
    fn test_combined_transform_round_trip() {
        let pos = Vec2::new(5.0, 10.0);
        let origin = Vec2::new(3.0, 3.0);
        let position = Vec2::new(50.0, 60.0);
        let rotation = 1.2;
        let scale = Vec2::new(2.0, 0.5);

        let world = transform_point(pos, origin, position, rotation, scale);
        let back = inverse_transform_point(world, origin, position, rotation, scale);
        assert!((back.x - pos.x).abs() < 1e-3);
        assert!((back.y - pos.y).abs() < 1e-3);
    }
}
