use crate::model::Vec2;
use crate::model::sprite::{Layer, StrokeElement};
use crate::state::editor::MergeTarget;

/// Snap distance threshold for auto-merge detection (in world units)
const MERGE_THRESHOLD: f32 = 0.5;

/// Find existing vertices on the same layer that are within snap distance
/// of the given position. Returns a MergeTarget if found.
pub fn find_merge_target(
    pos: Vec2,
    layer: &Layer,
    current_element_id: Option<&str>,
    snap_distance: f32,
) -> Option<MergeTarget> {
    let threshold = snap_distance.max(MERGE_THRESHOLD);
    let mut best: Option<(f32, MergeTarget)> = None;

    for element in &layer.elements {
        for vertex in &element.vertices {
            let dist = pos.distance(vertex.pos);
            if dist <= threshold {
                let same_element = current_element_id
                    .map(|id| id == element.id)
                    .unwrap_or(false);

                let candidate = MergeTarget {
                    element_id: element.id.clone(),
                    vertex_id: vertex.id.clone(),
                    position: vertex.pos,
                    same_element,
                };

                if best.as_ref().is_none_or(|(d, _)| dist < *d) {
                    best = Some((dist, candidate));
                }
            }
        }
    }

    best.map(|(_, target)| target)
}

/// Execute a cross-element merge: fuse two elements into one.
/// The target element absorbs the source element's vertices.
/// Returns true if the merge was performed.
pub fn merge_elements(
    layer: &mut Layer,
    target_element_id: &str,
    target_vertex_id: &str,
    source_element_id: &str,
    source_is_start: bool,
) -> bool {
    // Find indices
    let target_idx = layer
        .elements
        .iter()
        .position(|e| e.id == target_element_id);
    let source_idx = layer
        .elements
        .iter()
        .position(|e| e.id == source_element_id);

    let (target_idx, source_idx) = match (target_idx, source_idx) {
        (Some(t), Some(s)) if t != s => (t, s),
        _ => return false,
    };

    // Remove the source element
    let source_element = layer.elements.remove(if source_idx > target_idx {
        source_idx
    } else {
        // Adjust target index since source was before it
        source_idx
    });

    let target_idx = if source_idx < target_idx {
        target_idx - 1
    } else {
        target_idx
    };

    let target_element = &mut layer.elements[target_idx];

    // Determine where in the target to attach
    let target_is_end = target_element
        .vertices
        .last()
        .map(|v| v.id == target_vertex_id)
        .unwrap_or(false);

    // Append or prepend the source vertices (skip the overlapping vertex)
    let mut source_verts = source_element.vertices;
    if source_is_start && !source_verts.is_empty() {
        source_verts.remove(0); // Remove the overlapping first vertex
    } else if !source_is_start && !source_verts.is_empty() {
        source_verts.pop(); // Remove the overlapping last vertex
        source_verts.reverse();
    }

    if target_is_end {
        target_element.vertices.extend(source_verts);
    } else {
        // Prepend
        source_verts.reverse();
        for v in source_verts {
            target_element.vertices.insert(0, v);
        }
    }

    true
}

/// Close an element's path (same-element merge).
pub fn close_element(element: &mut StrokeElement) {
    element.closed = true;
}
