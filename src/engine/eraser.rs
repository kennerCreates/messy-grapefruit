use crate::math;
use crate::model::sprite::StrokeElement;

/// Result of erasing a vertex or segment from an element.
pub struct SplitResult {
    /// 0, 1, or 2 elements resulting from the erase operation.
    pub new_elements: Vec<StrokeElement>,
}

/// Erase a vertex from an element. Splits the element if the vertex is interior to an open path.
pub fn erase_vertex(element: &StrokeElement, vertex_id: &str, min_corner_radius: f32) -> SplitResult {
    let idx = match element.vertices.iter().position(|v| v.id == vertex_id) {
        Some(i) => i,
        None => return SplitResult { new_elements: vec![element.clone()] },
    };

    let n = element.vertices.len();

    if element.closed {
        // Closed path: remove vertex, open the path
        if n <= 3 {
            // Removing from a triangle or less → too few vertices
            if n <= 2 {
                return SplitResult { new_elements: vec![] };
            }
            // 3 vertices → remove one, 2 remain as open path
            let mut verts: Vec<_> = element.vertices.iter()
                .enumerate()
                .filter(|(i, _)| *i != idx)
                .map(|(_, v)| v.clone())
                .collect();
            math::recompute_auto_curves(&mut verts, false, element.curve_mode, min_corner_radius);
            let mut e = make_child_element(element, verts, false);
            e.id = element.id.clone(); // keep original ID
            return SplitResult { new_elements: vec![e] };
        }

        // Reorder vertices so the gap is at the removed position, making an open path
        let mut verts = Vec::with_capacity(n - 1);
        for i in 1..n {
            let vi = (idx + i) % n;
            verts.push(element.vertices[vi].clone());
        }
        math::recompute_auto_curves(&mut verts, false, element.curve_mode, min_corner_radius);
        let mut e = make_child_element(element, verts, false);
        e.id = element.id.clone();
        SplitResult { new_elements: vec![e] }
    } else {
        // Open path
        if idx == 0 || idx == n - 1 {
            // Endpoint removal
            let mut verts: Vec<_> = element.vertices.iter()
                .enumerate()
                .filter(|(i, _)| *i != idx)
                .map(|(_, v)| v.clone())
                .collect();
            if verts.len() < 2 {
                return SplitResult { new_elements: vec![] };
            }
            math::recompute_auto_curves(&mut verts, false, element.curve_mode, min_corner_radius);
            let mut e = make_child_element(element, verts, false);
            e.id = element.id.clone();
            SplitResult { new_elements: vec![e] }
        } else {
            // Interior split: [0..idx] and [idx+1..end]
            let left_verts: Vec<_> = element.vertices[..idx].to_vec();
            let right_verts: Vec<_> = element.vertices[idx + 1..].to_vec();
            let mut results = Vec::new();

            if left_verts.len() >= 2 {
                let mut verts = left_verts;
                math::recompute_auto_curves(&mut verts, false, element.curve_mode, min_corner_radius);
                let mut e = make_child_element(element, verts, false);
                e.id = element.id.clone(); // first part keeps original ID
                results.push(e);
            }
            if right_verts.len() >= 2 {
                let mut verts = right_verts;
                math::recompute_auto_curves(&mut verts, false, element.curve_mode, min_corner_radius);
                let e = make_child_element(element, verts, false);
                // second part gets new UUID (from make_child_element)
                results.push(e);
            }
            SplitResult { new_elements: results }
        }
    }
}

/// Erase a segment between vertex at `segment_index` and the next vertex.
pub fn erase_segment(element: &StrokeElement, segment_index: usize, min_corner_radius: f32) -> SplitResult {
    let n = element.vertices.len();

    if element.closed {
        // Closed path: remove one connection → open path
        // Reorder so the gap is at the removed segment
        let mut verts = Vec::with_capacity(n);
        for i in 0..n {
            let vi = (segment_index + 1 + i) % n;
            verts.push(element.vertices[vi].clone());
        }
        if verts.len() < 2 {
            return SplitResult { new_elements: vec![] };
        }
        math::recompute_auto_curves(&mut verts, false, element.curve_mode, min_corner_radius);
        let mut e = make_child_element(element, verts, false);
        e.id = element.id.clone();
        SplitResult { new_elements: vec![e] }
    } else {
        // Open path: split into [0..=segment_index] and [segment_index+1..end]
        let left_verts: Vec<_> = element.vertices[..=segment_index].to_vec();
        let right_verts: Vec<_> = element.vertices[segment_index + 1..].to_vec();
        let mut results = Vec::new();

        if left_verts.len() >= 2 {
            let mut verts = left_verts;
            math::recompute_auto_curves(&mut verts, false, element.curve_mode, min_corner_radius);
            let mut e = make_child_element(element, verts, false);
            e.id = element.id.clone();
            results.push(e);
        }
        if right_verts.len() >= 2 {
            let mut verts = right_verts;
            math::recompute_auto_curves(&mut verts, false, element.curve_mode, min_corner_radius);
            let e = make_child_element(element, verts, false);
            results.push(e);
        }
        SplitResult { new_elements: results }
    }
}

/// Create a child element inheriting properties from the parent, with new vertices.
fn make_child_element(parent: &StrokeElement, vertices: Vec<crate::model::sprite::PathVertex>, closed: bool) -> StrokeElement {
    let mut e = StrokeElement::new(
        vertices,
        parent.stroke_width,
        parent.stroke_color_index,
        parent.curve_mode,
    );
    e.closed = closed;
    e.fill_color_index = parent.fill_color_index;
    e.position = parent.position;
    e.rotation = parent.rotation;
    e.scale = parent.scale;
    e.origin = parent.origin;
    e.gradient_fill = parent.gradient_fill.clone();
    e.hatch_fill_id = parent.hatch_fill_id.clone();
    e
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::sprite::PathVertex;
    use crate::model::vec2::Vec2;

    fn make_open_path(n: usize) -> StrokeElement {
        let mut verts = Vec::new();
        for i in 0..n {
            let mut v = PathVertex::new(Vec2::new(i as f32 * 16.0, 0.0));
            v.id = format!("v{i}");
            verts.push(v);
        }
        let mut e = StrokeElement::new(verts, 2.0, 1, false);
        e.id = "elem1".to_string();
        e
    }

    fn make_closed_path(n: usize) -> StrokeElement {
        let mut e = make_open_path(n);
        e.closed = true;
        e
    }

    #[test]
    fn erase_endpoint_open() {
        let elem = make_open_path(4);
        let result = erase_vertex(&elem, "v0", 0.0);
        assert_eq!(result.new_elements.len(), 1);
        assert_eq!(result.new_elements[0].vertices.len(), 3);
        assert_eq!(result.new_elements[0].id, "elem1");
    }

    #[test]
    fn erase_interior_open_splits() {
        let elem = make_open_path(5);
        let result = erase_vertex(&elem, "v2", 0.0);
        assert_eq!(result.new_elements.len(), 2);
        assert_eq!(result.new_elements[0].vertices.len(), 2); // [v0, v1]
        assert_eq!(result.new_elements[1].vertices.len(), 2); // [v3, v4]
        assert_eq!(result.new_elements[0].id, "elem1"); // keeps original
        assert_ne!(result.new_elements[1].id, "elem1"); // new ID
    }

    #[test]
    fn erase_vertex_closed_opens() {
        let elem = make_closed_path(4);
        let result = erase_vertex(&elem, "v1", 0.0);
        assert_eq!(result.new_elements.len(), 1);
        assert!(!result.new_elements[0].closed);
        assert_eq!(result.new_elements[0].vertices.len(), 3);
    }

    #[test]
    fn erase_segment_open_splits() {
        let elem = make_open_path(4);
        let result = erase_segment(&elem, 1, 0.0); // remove segment between v1-v2
        assert_eq!(result.new_elements.len(), 2);
        assert_eq!(result.new_elements[0].vertices.len(), 2); // [v0, v1]
        assert_eq!(result.new_elements[1].vertices.len(), 2); // [v2, v3]
    }

    #[test]
    fn erase_segment_closed_opens() {
        let elem = make_closed_path(4);
        let result = erase_segment(&elem, 2, 0.0);
        assert_eq!(result.new_elements.len(), 1);
        assert!(!result.new_elements[0].closed);
        assert_eq!(result.new_elements[0].vertices.len(), 4);
    }

    #[test]
    fn erase_vertex_too_few_deletes() {
        let elem = make_open_path(2);
        let result = erase_vertex(&elem, "v0", 0.0);
        assert_eq!(result.new_elements.len(), 0);
    }
}
