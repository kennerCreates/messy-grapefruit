use crate::model::sprite::{
    AnimatableProperty, AnimationSequence, EasingCurve, EasingPreset, PropertyTrack, Sprite,
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

/// Interpolate a property track at the given time.
/// Returns None if the track has no keyframes.
pub fn interpolate_track(track: &PropertyTrack, time: f32) -> Option<f64> {
    if track.keyframes.is_empty() {
        return None;
    }

    // If before or at first keyframe
    if time <= track.keyframes[0].time {
        return Some(track.keyframes[0].value);
    }

    // If after or at last keyframe
    let last = track.keyframes.last().unwrap();
    if time >= last.time {
        return Some(last.value);
    }

    // Find the two surrounding keyframes
    let mut prev_idx = 0;
    for (i, kf) in track.keyframes.iter().enumerate() {
        if kf.time <= time {
            prev_idx = i;
        } else {
            break;
        }
    }
    let next_idx = prev_idx + 1;
    if next_idx >= track.keyframes.len() {
        return Some(track.keyframes[prev_idx].value);
    }

    let prev_kf = &track.keyframes[prev_idx];
    let next_kf = &track.keyframes[next_idx];

    // Check for step interpolation (color index, visibility)
    if track.property.uses_step_interpolation() || prev_kf.easing.preset == EasingPreset::Step {
        // Hold-previous: keep the previous keyframe value until the next keyframe
        return Some(prev_kf.value);
    }

    // Compute linear t
    let duration = next_kf.time - prev_kf.time;
    if duration <= 0.0 {
        return Some(prev_kf.value);
    }
    let linear_t = (time - prev_kf.time) / duration;

    // Apply easing
    let eased_t = apply_easing(linear_t, &prev_kf.easing);

    // Interpolate value
    let result = prev_kf.value + (next_kf.value - prev_kf.value) * eased_t as f64;
    Some(result)
}

/// Evaluate all tracks of an animation sequence at the given time.
/// Returns a list of (property, element_id, layer_id, value) tuples.
pub fn evaluate_animation(
    sequence: &AnimationSequence,
    time: f32,
) -> Vec<(AnimatableProperty, String, String, f64)> {
    let mut results = Vec::new();

    // Handle looping
    let effective_time = if sequence.looping && sequence.duration > 0.0 {
        time.rem_euclid(sequence.duration)
    } else {
        time.min(sequence.duration)
    };

    for track in &sequence.tracks {
        if let Some(value) = interpolate_track(track, effective_time) {
            results.push((
                track.property.clone(),
                track.element_id.clone(),
                track.layer_id.clone(),
                value,
            ));
        }
    }

    results
}

/// Apply evaluated animation values to a sprite (modifying it in place).
/// This creates a temporary modified copy of the sprite for rendering.
pub fn apply_animation_to_sprite(sprite: &mut Sprite, sequence: &AnimationSequence, time: f32) {
    let values = evaluate_animation(sequence, time);

    for (property, element_id, _layer_id, value) in &values {
        match property {
            AnimatableProperty::IKTargetX => {
                // Find IK target element across all layers
                for layer in &mut sprite.layers {
                    if let Some(ik_target) = layer.ik_targets.iter_mut().find(|t| t.id == *element_id) {
                        ik_target.position.x = *value as f32;
                    }
                }
            }
            AnimatableProperty::IKTargetY => {
                for layer in &mut sprite.layers {
                    if let Some(ik_target) = layer.ik_targets.iter_mut().find(|t| t.id == *element_id) {
                        ik_target.position.y = *value as f32;
                    }
                }
            }
            AnimatableProperty::IKMix => {
                // IK mix is handled during IK solving, not applied to sprite elements directly
                // The value is read from the animation tracks when solving IK chains
            }
            _ => {
                // Find the element across all layers
                for layer in &mut sprite.layers {
                    if let Some(element) = layer.elements.iter_mut().find(|e| e.id == *element_id) {
                        match property {
                            AnimatableProperty::PositionX => {
                                element.position.x = *value as f32;
                            }
                            AnimatableProperty::PositionY => {
                                element.position.y = *value as f32;
                            }
                            AnimatableProperty::Rotation => {
                                element.rotation = *value as f32;
                            }
                            AnimatableProperty::ScaleX => {
                                element.scale.x = *value as f32;
                            }
                            AnimatableProperty::ScaleY => {
                                element.scale.y = *value as f32;
                            }
                            AnimatableProperty::StrokeColorIndex => {
                                element.stroke_color_index = *value as usize;
                            }
                            AnimatableProperty::FillColorIndex => {
                                element.fill_color_index = *value as usize;
                            }
                            AnimatableProperty::VertexX(vertex_id) => {
                                if let Some(vertex) =
                                    element.vertices.iter_mut().find(|v| v.id == *vertex_id)
                                {
                                    vertex.pos.x = *value as f32;
                                }
                            }
                            AnimatableProperty::VertexY(vertex_id) => {
                                if let Some(vertex) =
                                    element.vertices.iter_mut().find(|v| v.id == *vertex_id)
                                {
                                    vertex.pos.y = *value as f32;
                                }
                            }
                            AnimatableProperty::Visible => {
                                // value > 0.5 = visible, else hidden
                                // Handled by the renderer checking the animation state.
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}

/// Check if an element is visible at the given time according to animation.
/// Returns true if visible (default if no visibility track exists).
pub fn is_element_visible(
    sequence: &AnimationSequence,
    element_id: &str,
    time: f32,
) -> bool {
    // Handle looping
    let effective_time = if sequence.looping && sequence.duration > 0.0 {
        time.rem_euclid(sequence.duration)
    } else {
        time.min(sequence.duration)
    };

    for track in &sequence.tracks {
        if track.element_id == element_id && track.property == AnimatableProperty::Visible
            && let Some(value) = interpolate_track(track, effective_time) {
                return value > 0.5;
            }
    }
    // Default: visible
    true
}

/// Get the previous keyframe time across all tracks in a sequence.
pub fn prev_keyframe_time(sequence: &AnimationSequence, current_time: f32) -> Option<f32> {
    let effective_time = if sequence.looping && sequence.duration > 0.0 {
        current_time % sequence.duration
    } else {
        current_time
    };

    let epsilon = 0.001;
    let mut best: Option<f32> = None;
    for track in &sequence.tracks {
        for kf in &track.keyframes {
            if kf.time < effective_time - epsilon
                && (best.is_none() || kf.time > best.unwrap()) {
                    best = Some(kf.time);
                }
        }
    }
    best
}

/// Get the next keyframe time across all tracks in a sequence.
pub fn next_keyframe_time(sequence: &AnimationSequence, current_time: f32) -> Option<f32> {
    let effective_time = if sequence.looping && sequence.duration > 0.0 {
        current_time % sequence.duration
    } else {
        current_time
    };

    let epsilon = 0.001;
    let mut best: Option<f32> = None;
    for track in &sequence.tracks {
        for kf in &track.keyframes {
            if kf.time > effective_time + epsilon
                && (best.is_none() || kf.time < best.unwrap()) {
                    best = Some(kf.time);
                }
        }
    }
    best
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

/// Collect IK mix values from animation tracks at a given time.
/// Returns a list of (ik_chain_id, mix_value) tuples.
pub fn collect_ik_mix_values(
    sequence: &AnimationSequence,
    time: f32,
) -> Vec<(String, f32)> {
    let effective_time = if sequence.looping && sequence.duration > 0.0 {
        time.rem_euclid(sequence.duration)
    } else {
        time.min(sequence.duration)
    };

    let mut mix_values = Vec::new();

    for track in &sequence.tracks {
        if track.property == AnimatableProperty::IKMix
            && let Some(value) = interpolate_track(track, effective_time) {
                // For IKMix tracks, element_id stores the IK chain ID
                mix_values.push((track.element_id.clone(), value as f32));
            }
    }

    mix_values
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

    // Step 1: Evaluate FK from keyframes
    apply_animation_to_sprite(&mut animated, sequence, time);

    // Step 2: Initial socket chain walk is done implicitly by resolve_socket_transform
    // which is called during rendering and IK solving.

    // Step 3: Solve IK chains (blended with FK via per-chain mix)
    if !sequence.ik_chains.is_empty() {
        let ik_targets = collect_ik_target_positions(&animated);
        let ik_mix = collect_ik_mix_values(sequence, time);
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
