use crate::model::sprite::{
    AnimationSequence, EasingCurve, EasingPreset, ElementPose, Sprite,
};
use crate::model::Vec2;

/// Evaluate a cubic bezier easing curve at parameter t (0..1).
/// control_points = [x1, y1, x2, y2] defining the cubic bezier from (0,0) to (1,1).
/// Returns the eased value (y) for the given progress (t maps to x).
fn cubic_bezier_easing(t: f32, cp: &[f32; 4]) -> f32 {
    let x1 = cp[0];
    let y1 = cp[1];
    let x2 = cp[2];
    let y2 = cp[3];

    // Newton-Raphson to solve for the bezier parameter that gives x = t
    // The bezier x(s) = 3*(1-s)^2*s*x1 + 3*(1-s)*s^2*x2 + s^3
    let mut s = t; // initial guess
    for _ in 0..8 {
        let s2 = s * s;
        let s3 = s2 * s;
        let os = 1.0 - s;
        let os2 = os * os;

        let x = 3.0 * os2 * s * x1 + 3.0 * os * s2 * x2 + s3;
        let dx = 3.0 * os2 * x1 + 6.0 * os * s * (x2 - x1) + 3.0 * s2 * (1.0 - x2);

        if dx.abs() < 1e-7 {
            break;
        }
        s -= (x - t) / dx;
        s = s.clamp(0.0, 1.0);
    }

    // Evaluate y at the found parameter
    let os = 1.0 - s;
    let os2 = os * os;
    let s2 = s * s;
    let s3 = s2 * s;
    3.0 * os2 * s * y1 + 3.0 * os * s2 * y2 + s3
}

/// Apply an easing function to a linear t value (0..1).
pub fn apply_easing(t: f32, easing: &EasingCurve) -> f32 {
    let t = t.clamp(0.0, 1.0);
    match easing.preset {
        EasingPreset::Linear => t,
        EasingPreset::EaseIn => cubic_bezier_easing(t, &[0.42, 0.0, 1.0, 1.0]),
        EasingPreset::EaseOut => cubic_bezier_easing(t, &[0.0, 0.0, 0.58, 1.0]),
        EasingPreset::EaseInOut => cubic_bezier_easing(t, &[0.42, 0.0, 0.58, 1.0]),
        EasingPreset::Bounce => bounce_ease_out(t),
        EasingPreset::Elastic => elastic_ease_out(t),
        EasingPreset::Step => 0.0, // Step returns 0 until t=1 where it jumps — but we handle step separately
        EasingPreset::Custom => cubic_bezier_easing(t, &easing.control_points),
    }
}

fn bounce_ease_out(t: f32) -> f32 {
    if t < 1.0 / 2.75 {
        7.5625 * t * t
    } else if t < 2.0 / 2.75 {
        let t = t - 1.5 / 2.75;
        7.5625 * t * t + 0.75
    } else if t < 2.5 / 2.75 {
        let t = t - 2.25 / 2.75;
        7.5625 * t * t + 0.9375
    } else {
        let t = t - 2.625 / 2.75;
        7.5625 * t * t + 0.984375
    }
}

fn elastic_ease_out(t: f32) -> f32 {
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    let p = 0.3;
    let s = p / 4.0;
    (2.0f32.powf(-10.0 * t)) * ((t - s) * std::f32::consts::TAU / p).sin() + 1.0
}

/// Check if an element is visible at the given time according to animation.
/// Returns true if visible (default if no pose keyframes exist).
pub fn is_element_visible(
    sequence: &AnimationSequence,
    element_id: &str,
    time: f32,
) -> bool {
    is_element_visible_pose(sequence, element_id, time)
}

/// Collect IK target positions from a sprite (after FK animation has been applied).
/// Returns a list of (ik_target_element_id, world_position) tuples.
pub fn collect_ik_target_positions(sprite: &Sprite) -> Vec<(String, crate::model::Vec2)> {
    let mut targets = Vec::new();
    for layer in &sprite.layers {
        for ik_target in &layer.ik_targets {
            targets.push((ik_target.id.clone(), ik_target.position));
        }
    }
    targets
}

/// Create an animated sprite copy for rendering at a given time.
/// This clones the sprite, applies FK animation values, then solves IK chains,
/// applies constraints, procedural modifiers, and physics.
///
/// Full evaluation pipeline:
/// 1. FK from keyframes
/// 2. Initial socket chain walk (implicit)
/// 3. IK chains (blended with FK)
/// 4. Constraints: look-at, volume preservation
/// 5. Procedural modifiers: sine/noise
/// 6. Physics: spring dynamics
/// 7. Final socket chain walk (implicit in rendering)
pub fn create_animated_sprite(
    sprite: &Sprite,
    sequence: &AnimationSequence,
    time: f32,
) -> Sprite {
    create_animated_sprite_with_physics(sprite, sequence, time, None)
}

/// Create an animated sprite with optional physics state.
/// If `physics_state` is provided, spring dynamics are applied.
/// Physics only runs during playback (not scrubbing).
pub fn create_animated_sprite_with_physics(
    sprite: &Sprite,
    sequence: &AnimationSequence,
    time: f32,
    mut physics_state: Option<&mut crate::engine::physics::PhysicsState>,
) -> Sprite {
    let mut animated = sprite.clone();

    // Step 1: Evaluate FK from pose keyframes
    if let Some(element_poses) = evaluate_pose_animation(sequence, time) {
        apply_pose_to_sprite(&mut animated, &element_poses);
    }

    // Step 2: Initial socket chain walk is done implicitly by resolve_socket_transform
    // which is called during rendering and IK solving.

    // Step 3: Solve IK chains (blended with FK via per-chain mix)
    if !sequence.ik_chains.is_empty() {
        let ik_targets = collect_ik_target_positions(&animated);
        let ik_mix = collect_ik_mix_values_pose(sequence, time);
        let ik_results = crate::engine::ik::solve_ik_chains(
            &animated,
            &sequence.ik_chains,
            &ik_targets,
            &ik_mix,
        );

        // Apply IK results: offset layer elements to match solved world positions
        for (layer_id, new_world_pos) in &ik_results {
            let st = crate::engine::socket::resolve_socket_transform(&animated, layer_id);
            if let Some(layer) = animated.layers.iter_mut().find(|l| l.id == *layer_id)
                && let Some(first_elem) = layer.elements.first_mut() {
                    // The IK result is a world position for the layer origin.
                    // Subtract the socket transform to get the local position offset.
                    first_elem.position.x = new_world_pos.x - st.position.x - first_elem.origin.x;
                    first_elem.position.y = new_world_pos.y - st.position.y - first_elem.origin.y;
                }
        }
    }

    // Step 4: Apply constraints (look-at, volume preservation)
    apply_constraints(&mut animated, time, physics_state.as_deref_mut());

    // Step 5: Apply procedural modifiers
    apply_procedural_modifiers(&mut animated, time);

    // Step 6: Apply physics simulation
    if let Some(phys_state) = physics_state {
        apply_physics(&mut animated, phys_state, time, 1.0 / 60.0);
    }

    // Step 7: Final socket chain walk is done implicitly during rendering

    animated
}

/// Apply constraints: look-at and volume preservation (Step 4).
fn apply_constraints(
    sprite: &mut Sprite,
    _time: f32,
    mut physics_state: Option<&mut crate::engine::physics::PhysicsState>,
) {
    use crate::engine::constraints;

    // Collect look-at target positions first (read-only pass)
    let mut look_at_targets: Vec<(usize, Vec2, Vec2)> = Vec::new(); // (layer_idx, origin, target_pos)
    {
        for (layer_idx, layer) in sprite.layers.iter().enumerate() {
            if let Some(ref look_at) = layer.constraints.look_at {
                if look_at.target_element_id.is_empty() {
                    continue;
                }
                // Find the target position
                let target_pos = find_element_world_pos(sprite, &look_at.target_element_id, look_at.target_vertex_id.as_deref());
                // Find the layer's own origin position
                let layer_origin = if let Some(first_elem) = layer.elements.first() {
                    let st = crate::engine::socket::resolve_socket_transform(sprite, &layer.id);
                    Vec2::new(
                        st.position.x + first_elem.position.x + first_elem.origin.x,
                        st.position.y + first_elem.position.y + first_elem.origin.y,
                    )
                } else {
                    Vec2::ZERO
                };
                if let Some(target_pos) = target_pos {
                    look_at_targets.push((layer_idx, layer_origin, target_pos));
                }
            }
        }
    }

    // Apply look-at constraints
    for (layer_idx, origin, target_pos) in look_at_targets {
        let layer = &sprite.layers[layer_idx];
        let look_at = layer.constraints.look_at.as_ref().unwrap();
        let current_rotation = layer
            .elements
            .first()
            .map(|e| e.rotation)
            .unwrap_or(0.0);

        let mut desired_angle = constraints::look_at_solve(
            origin,
            target_pos,
            current_rotation,
            look_at.rest_angle,
            look_at.min_angle,
            look_at.max_angle,
            look_at.mix,
        );

        // Apply spring smoothing if configured
        if let Some(ref smooth) = look_at.smooth
            && let Some(ref mut phys_state) = physics_state.as_deref_mut() {
                let layer_id = sprite.layers[layer_idx].id.clone();
                let angular_state = phys_state
                    .look_at_springs
                    .entry(layer_id)
                    .or_insert_with(|| crate::engine::physics::AngularSpringState {
                        angle: current_rotation,
                        velocity: 0.0,
                    });

                *angular_state = crate::engine::physics::step_angular_spring(
                    *angular_state,
                    desired_angle,
                    smooth.frequency,
                    smooth.damping,
                    1.0 / 60.0,
                );
                desired_angle = angular_state.angle;
            }

        // Apply to all elements on this layer
        for elem in &mut sprite.layers[layer_idx].elements {
            elem.rotation = desired_angle;
        }
    }

    // Apply volume preservation
    for layer in &mut sprite.layers {
        if layer.constraints.volume_preserve {
            for elem in &mut layer.elements {
                let (new_sx, new_sy) = constraints::volume_preserve(elem.scale.x, elem.scale.y);
                elem.scale.x = new_sx;
                elem.scale.y = new_sy;
            }
        }
    }
}

/// Apply procedural modifiers to the sprite (Step 5).
fn apply_procedural_modifiers(sprite: &mut Sprite, time: f32) {
    use crate::engine::constraints::{self, ProceduralTarget};

    for layer in &mut sprite.layers {
        if layer.constraints.procedural.is_empty() {
            continue;
        }

        // Collect modifiers to apply (to avoid borrow issues)
        let modifiers: Vec<_> = layer.constraints.procedural.clone();

        for modifier in &modifiers {
            let value = constraints::evaluate_procedural(modifier, time);
            let target = ProceduralTarget::from_str(&modifier.property);

            if let Some(target) = target {
                for elem in &mut layer.elements {
                    match target {
                        ProceduralTarget::PositionX => {
                            elem.position.x =
                                constraints::apply_procedural_value(elem.position.x, value, modifier.blend);
                        }
                        ProceduralTarget::PositionY => {
                            elem.position.y =
                                constraints::apply_procedural_value(elem.position.y, value, modifier.blend);
                        }
                        ProceduralTarget::Rotation => {
                            elem.rotation =
                                constraints::apply_procedural_value(elem.rotation, value, modifier.blend);
                        }
                        ProceduralTarget::ScaleX => {
                            elem.scale.x =
                                constraints::apply_procedural_value(elem.scale.x, value, modifier.blend);
                        }
                        ProceduralTarget::ScaleY => {
                            elem.scale.y =
                                constraints::apply_procedural_value(elem.scale.y, value, modifier.blend);
                        }
                    }
                }
            }
        }
    }
}

/// Apply physics simulation: spring dynamics chase post-modifier values (Step 6).
fn apply_physics(
    sprite: &mut Sprite,
    phys_state: &mut crate::engine::physics::PhysicsState,
    time: f32,
    dt: f32,
) {
    use crate::engine::physics;

    // First pass: collect physics info (read-only)
    struct PhysicsLayerInfo {
        layer_idx: usize,
        layer_id: String,
        constraint: crate::model::sprite::PhysicsConstraint,
        target_world_pos: Vec2,
    }

    let mut physics_layers: Vec<PhysicsLayerInfo> = Vec::new();

    for (idx, layer) in sprite.layers.iter().enumerate() {
        let physics_constraint = match &layer.constraints.physics {
            Some(c) if c.mix > 0.0 => c.clone(),
            _ => continue,
        };

        // Compute the target position in world space
        let target_world_pos = if let Some(first_elem) = layer.elements.first() {
            let st = crate::engine::socket::resolve_socket_transform(sprite, &layer.id);
            Vec2::new(
                st.position.x + first_elem.position.x,
                st.position.y + first_elem.position.y,
            )
        } else {
            continue;
        };

        physics_layers.push(PhysicsLayerInfo {
            layer_idx: idx,
            layer_id: layer.id.clone(),
            constraint: physics_constraint,
            target_world_pos,
        });
    }

    // Second pass: simulate and apply (write)
    for info in &physics_layers {
        // Initialize spring state if not present
        let spring_state = phys_state
            .springs
            .entry(info.layer_id.clone())
            .or_insert_with(|| physics::SpringState {
                position: info.target_world_pos,
                velocity: Vec2::ZERO,
            });

        // Apply external forces (gravity, wind)
        *spring_state =
            physics::apply_external_forces(*spring_state, &info.constraint, time, dt);

        // Step the spring
        *spring_state = physics::step_spring(
            *spring_state,
            info.target_world_pos,
            info.constraint.frequency,
            info.constraint.damping,
            dt,
        );

        // Convert the spring position back to a local-space offset
        let delta = Vec2::new(
            spring_state.position.x - info.target_world_pos.x,
            spring_state.position.y - info.target_world_pos.y,
        );

        // Apply the delta to element positions (blended by mix)
        let mix = info.constraint.mix;
        for elem in &mut sprite.layers[info.layer_idx].elements {
            elem.position.x += delta.x * mix;
            elem.position.y += delta.y * mix;
        }
    }
}

// === Pose-based animation functions ===

/// Evaluate pose-based animation at a given time.
/// Finds the surrounding pose keyframes and interpolates between them.
pub fn evaluate_pose_animation(
    sequence: &AnimationSequence,
    time: f32,
) -> Option<Vec<ElementPose>> {
    let poses = &sequence.pose_keyframes;
    if poses.is_empty() {
        return None;
    }

    // Handle looping
    let time = if sequence.looping && sequence.duration > 0.0 {
        time.rem_euclid(sequence.duration)
    } else {
        time
    };

    // Before first pose: use first pose
    if time <= poses[0].time || poses.len() == 1 {
        return Some(poses[0].element_poses.clone());
    }

    // After last pose: use last pose
    if time >= poses[poses.len() - 1].time {
        return Some(poses[poses.len() - 1].element_poses.clone());
    }

    // Find surrounding poses
    for i in 0..poses.len() - 1 {
        if time >= poses[i].time && time <= poses[i + 1].time {
            let duration = poses[i + 1].time - poses[i].time;
            if duration < 1e-6 {
                return Some(poses[i + 1].element_poses.clone());
            }
            let t = (time - poses[i].time) / duration;
            let eased_t = apply_easing(t, &poses[i + 1].easing);
            return Some(interpolate_poses(&poses[i], &poses[i + 1], eased_t));
        }
    }

    Some(poses.last().unwrap().element_poses.clone())
}

/// Interpolate between two poses at a given t (0..1, already eased).
fn interpolate_poses(
    pose_a: &crate::model::sprite::PoseKeyframe,
    pose_b: &crate::model::sprite::PoseKeyframe,
    t: f32,
) -> Vec<ElementPose> {
    let mut result = Vec::new();

    // Build a lookup for pose_b elements by element_id
    let b_map: std::collections::HashMap<&str, &ElementPose> = pose_b
        .element_poses
        .iter()
        .map(|ep| (ep.element_id.as_str(), ep))
        .collect();

    // Interpolate elements present in pose_a
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();

    for ep_a in &pose_a.element_poses {
        seen.insert(ep_a.element_id.as_str());
        if let Some(ep_b) = b_map.get(ep_a.element_id.as_str()) {
            result.push(lerp_element_pose(ep_a, ep_b, t));
        } else {
            // Element only in pose A — use as-is
            result.push(ep_a.clone());
        }
    }

    // Add elements only in pose B
    for ep_b in &pose_b.element_poses {
        if !seen.contains(ep_b.element_id.as_str()) {
            result.push(ep_b.clone());
        }
    }

    result
}

/// Lerp between two ElementPoses.
fn lerp_element_pose(a: &ElementPose, b: &ElementPose, t: f32) -> ElementPose {
    // Vertex positions: match by vertex ID, lerp
    let mut vertex_positions = Vec::new();
    let b_verts: std::collections::HashMap<&str, Vec2> = b
        .vertex_positions
        .iter()
        .map(|(id, pos)| (id.as_str(), *pos))
        .collect();
    let mut seen_verts: std::collections::HashSet<&str> = std::collections::HashSet::new();

    for (id, pos_a) in &a.vertex_positions {
        seen_verts.insert(id.as_str());
        if let Some(pos_b) = b_verts.get(id.as_str()) {
            vertex_positions.push((
                id.clone(),
                Vec2::new(
                    pos_a.x + (pos_b.x - pos_a.x) * t,
                    pos_a.y + (pos_b.y - pos_a.y) * t,
                ),
            ));
        } else {
            vertex_positions.push((id.clone(), *pos_a));
        }
    }
    for (id, pos_b) in &b.vertex_positions {
        if !seen_verts.contains(id.as_str()) {
            vertex_positions.push((id.clone(), *pos_b));
        }
    }

    // Shortest-path angle lerp for rotation
    let mut angle_diff = b.rotation - a.rotation;
    while angle_diff > std::f32::consts::PI {
        angle_diff -= std::f32::consts::TAU;
    }
    while angle_diff < -std::f32::consts::PI {
        angle_diff += std::f32::consts::TAU;
    }

    ElementPose {
        element_id: a.element_id.clone(),
        layer_id: a.layer_id.clone(),
        position: Vec2::new(
            a.position.x + (b.position.x - a.position.x) * t,
            a.position.y + (b.position.y - a.position.y) * t,
        ),
        rotation: a.rotation + angle_diff * t,
        scale: Vec2::new(
            a.scale.x + (b.scale.x - a.scale.x) * t,
            a.scale.y + (b.scale.y - a.scale.y) * t,
        ),
        // Step interpolation for discrete properties
        visible: if t < 1.0 { a.visible } else { b.visible },
        stroke_color_index: if t < 1.0 {
            a.stroke_color_index
        } else {
            b.stroke_color_index
        },
        fill_color_index: if t < 1.0 {
            a.fill_color_index
        } else {
            b.fill_color_index
        },
        vertex_positions,
    }
}

/// Apply interpolated element poses to a sprite.
pub fn apply_pose_to_sprite(sprite: &mut Sprite, element_poses: &[ElementPose]) {
    for ep in element_poses {
        // Find the element in the sprite and apply the pose
        for layer in &mut sprite.layers {
            if layer.id != ep.layer_id {
                continue;
            }

            // Apply to stroke elements
            if let Some(elem) = layer.elements.iter_mut().find(|e| e.id == ep.element_id) {
                elem.position = ep.position;
                elem.rotation = ep.rotation;
                elem.scale = ep.scale;
                elem.stroke_color_index = ep.stroke_color_index;
                elem.fill_color_index = ep.fill_color_index;

                // Apply vertex positions
                for (vid, vpos) in &ep.vertex_positions {
                    if let Some(vertex) = elem.vertices.iter_mut().find(|v| v.id == *vid) {
                        vertex.pos = *vpos;
                    }
                }
            }

            // Apply to IK targets
            if let Some(target) = layer.ik_targets.iter_mut().find(|t| t.id == ep.element_id) {
                target.position = ep.position;
            }

            break;
        }
    }
}

/// Check element visibility from pose-based keyframes.
pub fn is_element_visible_pose(
    sequence: &AnimationSequence,
    element_id: &str,
    time: f32,
) -> bool {
    if sequence.pose_keyframes.is_empty() {
        return true;
    }

    let time = if sequence.looping && sequence.duration > 0.0 {
        time.rem_euclid(sequence.duration)
    } else {
        time
    };

    // Find the pose at or before this time (step interpolation for visibility)
    let mut last_visible = true;
    for pose in &sequence.pose_keyframes {
        if pose.time > time {
            break;
        }
        if let Some(ep) = pose.element_poses.iter().find(|ep| ep.element_id == element_id) {
            last_visible = ep.visible;
        }
    }
    last_visible
}

/// Get previous pose keyframe time.
pub fn prev_pose_time(sequence: &AnimationSequence, current_time: f32) -> Option<f32> {
    let epsilon = 0.001;
    sequence
        .pose_keyframes
        .iter()
        .filter(|p| p.time < current_time - epsilon)
        .map(|p| p.time)
        .next_back()
}

/// Get next pose keyframe time.
pub fn next_pose_time(sequence: &AnimationSequence, current_time: f32) -> Option<f32> {
    let epsilon = 0.001;
    sequence
        .pose_keyframes
        .iter()
        .find(|p| p.time > current_time + epsilon)
        .map(|p| p.time)
}

/// Collect IK mix values from pose-based keyframes (interpolated).
pub fn collect_ik_mix_values_pose(
    sequence: &AnimationSequence,
    time: f32,
) -> Vec<(String, f32)> {
    let poses = &sequence.pose_keyframes;
    if poses.is_empty() {
        return Vec::new();
    }

    let time = if sequence.looping && sequence.duration > 0.0 {
        time.rem_euclid(sequence.duration)
    } else {
        time
    };

    // Before first or single pose
    if time <= poses[0].time || poses.len() == 1 {
        return poses[0].ik_mix_values.clone();
    }

    // After last
    if time >= poses[poses.len() - 1].time {
        return poses[poses.len() - 1].ik_mix_values.clone();
    }

    // Find surrounding poses and interpolate
    for i in 0..poses.len() - 1 {
        if time >= poses[i].time && time <= poses[i + 1].time {
            let duration = poses[i + 1].time - poses[i].time;
            if duration < 1e-6 {
                return poses[i + 1].ik_mix_values.clone();
            }
            let t = (time - poses[i].time) / duration;
            let eased_t = apply_easing(t, &poses[i + 1].easing);

            // Lerp IK mix values by chain ID
            let mut result = poses[i].ik_mix_values.clone();
            let b_map: std::collections::HashMap<&str, f32> = poses[i + 1]
                .ik_mix_values
                .iter()
                .map(|(id, v)| (id.as_str(), *v))
                .collect();
            for (chain_id, mix_a) in &mut result {
                if let Some(&mix_b) = b_map.get(chain_id.as_str()) {
                    *mix_a += (mix_b - *mix_a) * eased_t;
                }
            }
            return result;
        }
    }

    poses.last().unwrap().ik_mix_values.clone()
}

/// Find the world-space position of an element (or a specific vertex on it).
fn find_element_world_pos(sprite: &Sprite, element_id: &str, vertex_id: Option<&str>) -> Option<Vec2> {
    for layer in &sprite.layers {
        for element in &layer.elements {
            if element.id == element_id {
                let st = crate::engine::socket::resolve_socket_transform(sprite, &layer.id);
                if let Some(vid) = vertex_id {
                    // Find specific vertex
                    for vertex in &element.vertices {
                        if vertex.id == vid {
                            return Some(Vec2::new(
                                st.position.x + element.position.x + vertex.pos.x,
                                st.position.y + element.position.y + vertex.pos.y,
                            ));
                        }
                    }
                    // Vertex ID was specified but not found -- don't silently
                    // fall back to element origin
                    return None;
                }
                // No vertex requested -- return element origin position
                return Some(Vec2::new(
                    st.position.x + element.position.x + element.origin.x,
                    st.position.y + element.position.y + element.origin.y,
                ));
            }
        }
        // Check IK targets too
        for ik_target in &layer.ik_targets {
            if ik_target.id == element_id {
                return Some(ik_target.position);
            }
        }
    }
    None
}
