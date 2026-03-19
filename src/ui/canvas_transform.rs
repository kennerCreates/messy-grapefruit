use crate::engine::{snap, transform};
use crate::math;
use crate::model::project::{GridMode, Project};
use crate::model::sprite::{Sprite, StrokeElement};
use crate::model::vec2::Vec2;
use crate::state::editor::HandleKind;

/// Collect a field from all selected elements.
pub(super) fn collect_selected_field<T>(
    sprite: &Sprite,
    selected_ids: &[String],
    f: impl Fn(&StrokeElement) -> (String, T),
) -> Vec<(String, T)> {
    let mut result = Vec::new();
    for layer in &sprite.layers {
        for element in &layer.elements {
            if selected_ids.iter().any(|id| id == &element.id) {
                result.push(f(element));
            }
        }
    }
    // Ensure ordering matches selected_ids
    let mut ordered = Vec::with_capacity(selected_ids.len());
    for id in selected_ids {
        if let Some(pos) = result.iter().position(|(eid, _)| eid == id) {
            ordered.push(result.swap_remove(pos));
        }
    }
    ordered
}

/// After scale/rotate, bake the transform into vertex positions and snap to grid.
pub(super) fn bake_and_snap_selected(sprite: &mut Sprite, selected_ids: &[String], project: &Project) {
    let grid_size = project.editor_preferences.grid_size;
    let grid_mode = project.editor_preferences.grid_mode;
    let grid_offset = project.editor_preferences.grid_offset;

    for layer in sprite.layers.iter_mut() {
        for element in layer.elements.iter_mut() {
            if selected_ids.iter().any(|id| id == &element.id) {
                bake_element_transform(element, grid_size, grid_mode, grid_offset, project.min_corner_radius);
            }
        }
    }
}

/// Bake an element's transform into its vertices, snap to grid, reset transform to identity.
fn bake_element_transform(element: &mut StrokeElement, grid_size: u32, grid_mode: GridMode, grid_offset: (f32, f32), corner_radius: f32) {
    for v in &mut element.vertices {
        v.pos = transform::transform_point(v.pos, element.origin, element.position, element.rotation, element.scale);
        v.pos = snap::snap_to_grid(v.pos, grid_size, grid_mode, grid_offset);
    }
    // Reset transform to identity
    element.position = Vec2::ZERO;
    element.rotation = 0.0;
    element.scale = Vec2::ONE;
    element.origin = Vec2::ZERO;
    // Recompute curves from snapped positions
    math::recompute_auto_curves(
        &mut element.vertices,
        element.closed,
        element.curve_mode,
        corner_radius,
    );
}

/// Compute the anchor point (opposite to the handle being dragged) on the selection AABB.
pub(super) fn scale_anchor(handle: HandleKind, bmin: Vec2, bmax: Vec2) -> Vec2 {
    let mid_x = (bmin.x + bmax.x) * 0.5;
    let mid_y = (bmin.y + bmax.y) * 0.5;
    match handle {
        HandleKind::ScaleNW => bmax,
        HandleKind::ScaleN  => Vec2::new(mid_x, bmax.y),
        HandleKind::ScaleNE => Vec2::new(bmin.x, bmax.y),
        HandleKind::ScaleE  => Vec2::new(bmin.x, mid_y),
        HandleKind::ScaleSE => bmin,
        HandleKind::ScaleS  => Vec2::new(mid_x, bmin.y),
        HandleKind::ScaleSW => Vec2::new(bmax.x, bmin.y),
        HandleKind::ScaleW  => Vec2::new(bmax.x, mid_y),
        HandleKind::Rotate  => Vec2::new(mid_x, mid_y), // not used for scale
    }
}

/// Compute scale factors (sx, sy) from handle drag.
/// Uses the handle's original position to determine the direction from anchor.
pub(super) fn compute_scale_factors(handle: HandleKind, cursor: Vec2, anchor: Vec2, bmin: Vec2, bmax: Vec2) -> (f32, f32) {
    let mid_x = (bmin.x + bmax.x) * 0.5;
    let mid_y = (bmin.y + bmax.y) * 0.5;

    // Original handle position in world space
    let handle_pos = match handle {
        HandleKind::ScaleNW => Vec2::new(bmin.x, bmin.y),
        HandleKind::ScaleN  => Vec2::new(mid_x, bmin.y),
        HandleKind::ScaleNE => Vec2::new(bmax.x, bmin.y),
        HandleKind::ScaleE  => Vec2::new(bmax.x, mid_y),
        HandleKind::ScaleSE => Vec2::new(bmax.x, bmax.y),
        HandleKind::ScaleS  => Vec2::new(mid_x, bmax.y),
        HandleKind::ScaleSW => Vec2::new(bmin.x, bmax.y),
        HandleKind::ScaleW  => Vec2::new(bmin.x, mid_y),
        HandleKind::Rotate  => return (1.0, 1.0),
    };

    let original_span = handle_pos - anchor;
    let current_span = cursor - anchor;

    let sx = if original_span.x.abs() > 0.001 { current_span.x / original_span.x } else { 1.0 };
    let sy = if original_span.y.abs() > 0.001 { current_span.y / original_span.y } else { 1.0 };

    match handle {
        HandleKind::ScaleN | HandleKind::ScaleS => (1.0, sy),
        HandleKind::ScaleE | HandleKind::ScaleW => (sx, 1.0),
        _ => (sx, sy),
    }
}
