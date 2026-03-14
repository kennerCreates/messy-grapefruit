use crate::model::sprite::Sprite;
use crate::model::Vec2;
use std::collections::HashSet;

/// Accumulated world-space transform from the socket chain.
#[derive(Debug, Clone, Copy)]
pub struct SocketTransform {
    /// Accumulated world-space position offset from the socket chain.
    pub position: Vec2,
    /// Accumulated rotation (radians) from the socket chain.
    pub rotation: f32,
}

impl Default for SocketTransform {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            rotation: 0.0,
        }
    }
}

/// Check if assigning `child_layer_id` to be socketed to an element on
/// `parent_layer_id` would create a circular reference.
/// Returns true if a cycle would be created (i.e. the assignment should be rejected).
pub fn would_create_cycle(
    sprite: &Sprite,
    child_layer_id: &str,
    parent_element_id: &str,
) -> bool {
    // Find which layer owns the parent element
    let parent_layer_id = match find_layer_for_element(sprite, parent_element_id) {
        Some(id) => id,
        None => return false, // Parent element doesn't exist -- no cycle
    };

    // If the parent layer IS the child layer, that's a self-reference cycle
    if parent_layer_id == child_layer_id {
        return true;
    }

    // Walk up the socket chain from parent_layer_id and check if we ever reach child_layer_id
    let mut visited = HashSet::new();
    visited.insert(child_layer_id.to_string());
    let mut current_layer_id = parent_layer_id;

    loop {
        if visited.contains(current_layer_id) {
            return true; // Cycle detected
        }
        visited.insert(current_layer_id.to_string());

        // Find this layer's socket parent
        let layer = sprite.layers.iter().find(|l| l.id == current_layer_id);
        let Some(layer) = layer else {
            return false; // Layer not found, no cycle
        };

        match &layer.socket {
            Some(socket) => {
                // Find the layer that owns socket.parent_element_id
                match find_layer_for_element(sprite, &socket.parent_element_id) {
                    Some(next_layer_id) => {
                        current_layer_id = next_layer_id;
                    }
                    None => return false, // Broken socket reference, no cycle
                }
            }
            None => return false, // Reached a root layer, no cycle
        }
    }
}

/// Find the layer ID that contains the given element ID.
fn find_layer_for_element<'a>(sprite: &'a Sprite, element_id: &str) -> Option<&'a str> {
    for layer in &sprite.layers {
        for element in &layer.elements {
            if element.id == element_id {
                return Some(&layer.id);
            }
        }
    }
    None
}

/// Resolve the socket chain transform for a given layer.
/// Walks from the layer up to the root, accumulating position and rotation.
/// Returns the accumulated world-space transform.
pub fn resolve_socket_transform(sprite: &Sprite, layer_id: &str) -> SocketTransform {
    let mut transform = SocketTransform::default();
    let mut current_layer_id = layer_id.to_string();
    let mut visited = HashSet::new();

    loop {
        if visited.contains(&current_layer_id) {
            // Cycle detected (should not happen if cycle detection is correct at assignment time)
            break;
        }
        visited.insert(current_layer_id.clone());

        let layer = sprite.layers.iter().find(|l| l.id == current_layer_id);
        let Some(layer) = layer else { break };

        let Some(ref socket) = layer.socket else {
            break; // Root layer reached
        };

        // Find the parent vertex position
        let parent_vertex_pos = find_vertex_position(sprite, &socket.parent_element_id, &socket.parent_vertex_id);
        let Some(vertex_pos) = parent_vertex_pos else {
            break; // Broken reference
        };

        // Find the parent element rotation
        let parent_rotation = find_element_rotation(sprite, &socket.parent_element_id);

        // Accumulate: rotate the current accumulated offset by the parent's rotation,
        // then add the parent vertex position.
        // We build from leaf-to-root, then the final accumulated transform
        // gives us the world offset to apply.
        let cos_r = parent_rotation.cos();
        let sin_r = parent_rotation.sin();
        let rotated_x = transform.position.x * cos_r - transform.position.y * sin_r;
        let rotated_y = transform.position.x * sin_r + transform.position.y * cos_r;

        transform.position = Vec2::new(
            vertex_pos.x + rotated_x,
            vertex_pos.y + rotated_y,
        );
        transform.rotation += parent_rotation;

        // Move up to the parent layer
        match find_layer_for_element(sprite, &socket.parent_element_id) {
            Some(parent_layer_id) => {
                current_layer_id = parent_layer_id.to_string();
            }
            None => break,
        }
    }

    transform
}

/// Find the world position of a specific vertex on a specific element.
fn find_vertex_position(sprite: &Sprite, element_id: &str, vertex_id: &str) -> Option<Vec2> {
    for layer in &sprite.layers {
        for element in &layer.elements {
            if element.id == element_id {
                for vertex in &element.vertices {
                    if vertex.id == vertex_id {
                        // The vertex position is in element-local space.
                        // Apply element's position offset.
                        return Some(Vec2::new(
                            vertex.pos.x + element.position.x,
                            vertex.pos.y + element.position.y,
                        ));
                    }
                }
                return None;
            }
        }
    }
    None
}

/// Find the rotation of a specific element.
fn find_element_rotation(sprite: &Sprite, element_id: &str) -> f32 {
    for layer in &sprite.layers {
        for element in &layer.elements {
            if element.id == element_id {
                return element.rotation;
            }
        }
    }
    0.0
}

/// Find all layer IDs that are socketed to a specific vertex.
/// Used to warn before vertex deletion.
pub fn find_child_layers_for_vertex(sprite: &Sprite, element_id: &str, vertex_id: &str) -> Vec<String> {
    let mut children = Vec::new();
    for layer in &sprite.layers {
        if let Some(ref socket) = layer.socket {
            if socket.parent_element_id == element_id && socket.parent_vertex_id == vertex_id {
                children.push(layer.id.clone());
            }
        }
    }
    children
}

/// Find all layer IDs that are socketed to any vertex on a specific element.
pub fn find_child_layers_for_element(sprite: &Sprite, element_id: &str) -> Vec<String> {
    let mut children = Vec::new();
    for layer in &sprite.layers {
        if let Some(ref socket) = layer.socket {
            if socket.parent_element_id == element_id {
                children.push(layer.id.clone());
            }
        }
    }
    children
}

/// Detach a layer from its socket parent, snapping it to its current world-space position.
/// This preserves the visual position of the layer after detaching.
pub fn detach_layer_to_world_space(sprite: &mut Sprite, layer_id: &str) {
    // First, resolve the current world-space transform before detaching
    let world_transform = resolve_socket_transform(sprite, layer_id);

    // Find the layer and clear its socket
    if let Some(layer) = sprite.layers.iter_mut().find(|l| l.id == layer_id) {
        layer.socket = None;

        // Offset all element positions by the accumulated socket transform
        // so the layer stays visually in the same place
        for element in &mut layer.elements {
            let cos_r = world_transform.rotation.cos();
            let sin_r = world_transform.rotation.sin();

            // Rotate each vertex by the accumulated rotation and add position offset
            for vertex in &mut element.vertices {
                let rx = vertex.pos.x * cos_r - vertex.pos.y * sin_r;
                let ry = vertex.pos.x * sin_r + vertex.pos.y * cos_r;
                vertex.pos = Vec2::new(rx + world_transform.position.x, ry + world_transform.position.y);

                if let Some(ref mut cp1) = vertex.cp1 {
                    let rx = cp1.x * cos_r - cp1.y * sin_r;
                    let ry = cp1.x * sin_r + cp1.y * cos_r;
                    *cp1 = Vec2::new(rx + world_transform.position.x, ry + world_transform.position.y);
                }
                if let Some(ref mut cp2) = vertex.cp2 {
                    let rx = cp2.x * cos_r - cp2.y * sin_r;
                    let ry = cp2.x * sin_r + cp2.y * cos_r;
                    *cp2 = Vec2::new(rx + world_transform.position.x, ry + world_transform.position.y);
                }
            }

            // Also apply to element position and origin
            let px = element.position.x * cos_r - element.position.y * sin_r;
            let py = element.position.x * sin_r + element.position.y * cos_r;
            element.position = Vec2::new(px + world_transform.position.x, py + world_transform.position.y);

            element.rotation += world_transform.rotation;

            let ox = element.origin.x * cos_r - element.origin.y * sin_r;
            let oy = element.origin.x * sin_r + element.origin.y * cos_r;
            element.origin = Vec2::new(ox + world_transform.position.x, oy + world_transform.position.y);
        }
    }
}

/// Get a list of all vertices across all layers and elements, suitable for
/// populating a socket assignment picker.
/// Returns: Vec<(layer_id, layer_name, element_id, element_name, vertex_id, vertex_pos)>
pub fn get_all_socket_targets(
    sprite: &Sprite,
    exclude_layer_id: &str,
) -> Vec<(String, String, String, String, String, Vec2)> {
    let mut targets = Vec::new();
    for layer in &sprite.layers {
        if layer.id == exclude_layer_id {
            continue; // Can't socket to yourself
        }
        for element in &layer.elements {
            let elem_name = element.name.clone().unwrap_or_else(|| {
                format!("Element {}", &element.id[..6.min(element.id.len())])
            });
            for vertex in &element.vertices {
                targets.push((
                    layer.id.clone(),
                    layer.name.clone(),
                    element.id.clone(),
                    elem_name.clone(),
                    vertex.id.clone(),
                    vertex.pos,
                ));
            }
        }
    }
    targets
}
