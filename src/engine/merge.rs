use crate::math;
use crate::model::sprite::{Layer, PathVertex, StrokeElement};
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
