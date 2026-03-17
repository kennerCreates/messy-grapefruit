use crate::engine::transform;
use crate::math;
use crate::model::sprite::{Layer, PathVertex, Sprite, StrokeElement};
use crate::model::vec2::Vec2;

#[allow(dead_code)] // vertex_id used in auto-merge target identification
pub struct MergeTarget {
    pub element_id: String,
    pub vertex_id: String,
    pub position: Vec2,
    pub end: VertexEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexEnd {
    Start,
    End,
}

/// Check if a position is near the start or end vertex of any element on the layer.
/// `exclude_element_id` is used to skip the element currently being drawn.
pub fn find_merge_target(
    pos: Vec2,
    layer: &Layer,
    exclude_element_id: Option<&str>,
    threshold: f32,
) -> Option<MergeTarget> {
    for element in &layer.elements {
        if let Some(exclude) = exclude_element_id
            && element.id == exclude
        {
            continue;
        }
        if element.vertices.is_empty() {
            continue;
        }

        // Check start vertex
        let first = &element.vertices[0];
        if first.pos.distance(pos) <= threshold {
            return Some(MergeTarget {
                element_id: element.id.clone(),
                vertex_id: first.id.clone(),
                position: first.pos,
                end: VertexEnd::Start,
            });
        }

        // Check end vertex
        let last = &element.vertices[element.vertices.len() - 1];
        if last.pos.distance(pos) <= threshold {
            return Some(MergeTarget {
                element_id: element.id.clone(),
                vertex_id: last.id.clone(),
                position: last.pos,
                end: VertexEnd::End,
            });
        }
    }
    None
}

/// Merge a new stroke into an existing element by connecting at endpoints.
/// Returns the merged element.
#[allow(clippy::too_many_arguments)]
pub fn merge_elements(
    existing: &StrokeElement,
    existing_end: VertexEnd,
    new_vertices: &[PathVertex],
    new_end: VertexEnd,
    stroke_width: f32,
    stroke_color_index: u8,
    curve_mode: bool,
    min_corner_radius: f32,
) -> StrokeElement {
    let mut merged_verts: Vec<PathVertex> = Vec::new();

    match (existing_end, new_end) {
        (VertexEnd::End, VertexEnd::Start) => {
            // existing [...] -> new [...]  (most common: drew from existing endpoint)
            merged_verts.extend(existing.vertices.iter().cloned());
            // Skip the first new vertex since it overlaps with existing's last
            merged_verts.extend(new_vertices.iter().skip(1).cloned());
        }
        (VertexEnd::Start, VertexEnd::End) => {
            // new [...] -> existing [...]
            merged_verts.extend(new_vertices.iter().rev().skip(1).cloned());
            merged_verts.extend(existing.vertices.iter().cloned());
        }
        (VertexEnd::End, VertexEnd::End) => {
            // existing [...] <- new [...]  (new stroke ends at existing's end)
            merged_verts.extend(existing.vertices.iter().cloned());
            merged_verts.extend(new_vertices.iter().rev().skip(1).cloned());
        }
        (VertexEnd::Start, VertexEnd::Start) => {
            // new [...] -> existing [...]  (new stroke starts at existing's start)
            merged_verts.extend(new_vertices.iter().rev().skip(1).cloned());
            merged_verts.extend(existing.vertices.iter().cloned());
        }
    }

    // Recompute auto-curves for the merged vertices
    math::recompute_auto_curves(&mut merged_verts, false, curve_mode, min_corner_radius);

    StrokeElement {
        id: existing.id.clone(),
        name: existing.name.clone(),
        vertices: merged_verts,
        closed: false,
        curve_mode,
        stroke_width,
        stroke_color_index,
        fill_color_index: existing.fill_color_index,
        position: existing.position,
        rotation: existing.rotation,
        scale: existing.scale,
        origin: existing.origin,
    }
}

/// Find a merge target endpoint on any visible layer, checking world-space positions.
/// `exclude_element_id` skips the element being dragged.
/// `solo_layer_id` restricts search to a single layer if set.
pub fn find_endpoint_target_world(
    world_pos: Vec2,
    sprite: &Sprite,
    exclude_element_id: &str,
    threshold: f32,
    solo_layer_id: Option<&str>,
) -> Option<MergeTarget> {
    for layer in &sprite.layers {
        if !layer.visible || layer.locked {
            continue;
        }
        if let Some(solo) = solo_layer_id
            && layer.id != solo
        {
            continue;
        }
        for element in &layer.elements {
            if element.id == exclude_element_id || element.closed || element.vertices.is_empty() {
                continue;
            }

            let first = &element.vertices[0];
            let first_world = transform::transform_point(
                first.pos, element.origin, element.position, element.rotation, element.scale,
            );
            if first_world.distance(world_pos) <= threshold {
                return Some(MergeTarget {
                    element_id: element.id.clone(),
                    vertex_id: first.id.clone(),
                    position: first_world,
                    end: VertexEnd::Start,
                });
            }

            let last = &element.vertices[element.vertices.len() - 1];
            let last_world = transform::transform_point(
                last.pos, element.origin, element.position, element.rotation, element.scale,
            );
            if last_world.distance(world_pos) <= threshold {
                return Some(MergeTarget {
                    element_id: element.id.clone(),
                    vertex_id: last.id.clone(),
                    position: last_world,
                    end: VertexEnd::End,
                });
            }
        }
    }
    None
}

/// Join two existing elements at their endpoints. The source element's vertices are
/// appended/prepended to the target, and the source element is removed.
/// Returns the joined element (retaining the target's ID).
pub fn join_elements(
    target: &StrokeElement,
    target_end: VertexEnd,
    source: &StrokeElement,
    source_end: VertexEnd,
    min_corner_radius: f32,
) -> StrokeElement {
    // Convert source vertices into target's local space.
    // For each source vertex, transform to world, then inverse-transform into target space.
    let source_local: Vec<PathVertex> = source.vertices.iter().map(|v| {
        let world = transform::transform_point(
            v.pos, source.origin, source.position, source.rotation, source.scale,
        );
        let local = transform::inverse_transform_point(
            world, target.origin, target.position, target.rotation, target.scale,
        );
        let mut pv = PathVertex::new(local);
        // Also transform control points
        if let Some(cp1) = v.cp1 {
            let cp1_world = transform::transform_point(
                cp1, source.origin, source.position, source.rotation, source.scale,
            );
            pv.cp1 = Some(transform::inverse_transform_point(
                cp1_world, target.origin, target.position, target.rotation, target.scale,
            ));
        }
        if let Some(cp2) = v.cp2 {
            let cp2_world = transform::transform_point(
                cp2, source.origin, source.position, source.rotation, source.scale,
            );
            pv.cp2 = Some(transform::inverse_transform_point(
                cp2_world, target.origin, target.position, target.rotation, target.scale,
            ));
        }
        pv.manual_handles = v.manual_handles;
        pv
    }).collect();

    let mut merged_verts: Vec<PathVertex> = Vec::new();

    match (target_end, source_end) {
        (VertexEnd::End, VertexEnd::Start) => {
            merged_verts.extend(target.vertices.iter().cloned());
            merged_verts.extend(source_local.into_iter().skip(1));
        }
        (VertexEnd::End, VertexEnd::End) => {
            merged_verts.extend(target.vertices.iter().cloned());
            merged_verts.extend(source_local.into_iter().rev().skip(1));
        }
        (VertexEnd::Start, VertexEnd::End) => {
            merged_verts.extend(source_local.into_iter().rev().skip(1));
            merged_verts.extend(target.vertices.iter().cloned());
        }
        (VertexEnd::Start, VertexEnd::Start) => {
            merged_verts.extend(source_local.into_iter().skip(1));
            merged_verts.extend(target.vertices.iter().cloned());
        }
    }

    let curve_mode = target.curve_mode;
    math::recompute_auto_curves(&mut merged_verts, false, curve_mode, min_corner_radius);

    StrokeElement {
        id: target.id.clone(),
        name: target.name.clone(),
        vertices: merged_verts,
        closed: false,
        curve_mode,
        stroke_width: target.stroke_width,
        stroke_color_index: target.stroke_color_index,
        fill_color_index: target.fill_color_index,
        position: target.position,
        rotation: target.rotation,
        scale: target.scale,
        origin: target.origin,
    }
}
