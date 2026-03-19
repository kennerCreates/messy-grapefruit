use std::borrow::Cow;
use std::collections::HashMap;

use crate::model::animation::{AnimationSequence, EasingCurve, ElementPose, PoseKeyframe, VertexPoseEntry};
use crate::model::sprite::{Sprite, StrokeElement};
use crate::model::vec2::Vec2;

/// Canvas animation state — used for the colored border and status bar dot.
#[derive(Debug, Clone, PartialEq)]
pub enum CanvasAnimState {
    /// No animation selected, or playhead at frame 0 with no sequence.
    Rest,
    /// Playhead is exactly on a keyframe.
    OnKeyframe(String),
    /// Playhead is between keyframes (interpolated state).
    Interpolated,
}

// ── Easing ────────────────────────────────────────────────────────────────────

/// Evaluate a CSS-style cubic bezier easing curve.
/// The curve passes through (0,0) and (1,1) with user controls at (x1,y1) and (x2,y2).
/// Input `t` is the raw linear blend parameter [0,1]; returns the eased value [0,1].
pub fn eval_cubic_bezier_easing(curve: &EasingCurve, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let [x1, y1, x2, y2] = curve.control_points;

    // For linear, skip the expensive solve
    if (x1 - y1).abs() < 1e-5 && (x2 - y2).abs() < 1e-5 {
        return t;
    }

    // Cubic bezier: B(s) = 3s(1-s)²P + 3s²(1-s)Q + s³
    // where P=(x1,y1), Q=(x2,y2), endpoints fixed at (0,0) and (1,1)
    let bx = |s: f32| -> f32 {
        let mt = 1.0 - s;
        3.0 * mt * mt * s * x1 + 3.0 * mt * s * s * x2 + s * s * s
    };
    let bx_deriv = |s: f32| -> f32 {
        let mt = 1.0 - s;
        3.0 * (mt * mt * x1 + 2.0 * mt * s * (x2 - x1) + s * s * (1.0 - x2))
    };
    let by = |s: f32| -> f32 {
        let mt = 1.0 - s;
        3.0 * mt * mt * s * y1 + 3.0 * mt * s * s * y2 + s * s * s
    };

    // Newton-Raphson: find s such that Bx(s) = t
    let mut s = t; // good initial guess
    for _ in 0..5 {
        let dx = bx(s) - t;
        let d = bx_deriv(s);
        if d.abs() < 1e-6 { break; }
        s -= dx / d;
        s = s.clamp(0.0, 1.0);
    }

    by(s)
}

// ── Interpolation helpers ─────────────────────────────────────────────────────

fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn lerp_vec2(a: Vec2, b: Vec2, t: f32) -> Vec2 {
    Vec2::new(lerp_f32(a.x, b.x, t), lerp_f32(a.y, b.y, t))
}

/// Shortest-path angle lerp, wrapping through ±π.
fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
    use std::f32::consts::TAU;
    let mut delta = b - a;
    // Normalize delta to [-π, π]
    delta = ((delta % TAU) + TAU) % TAU;
    if delta > std::f32::consts::PI { delta -= TAU; }
    a + delta * t
}

/// Interpolate two ElementPoses using the given easing curve.
fn interpolate_element_pose(a: &ElementPose, b: &ElementPose, raw_t: f32, easing: &EasingCurve) -> ElementPose {
    let t = eval_cubic_bezier_easing(easing, raw_t);

    // Step interpolation for integers: use b's value at/after midpoint
    let stroke_color_index = if t >= 0.5 { b.stroke_color_index } else { a.stroke_color_index };
    let fill_color_index = if t >= 0.5 { b.fill_color_index } else { a.fill_color_index };
    let visible = if t >= 0.5 { b.visible } else { a.visible };

    // Interpolate vertex positions by stable ID
    let mut vertex_positions: Vec<VertexPoseEntry> = Vec::new();
    for vb in &b.vertex_positions {
        if let Some(va) = a.vertex_positions.iter().find(|v| v.vertex_id == vb.vertex_id) {
            vertex_positions.push(VertexPoseEntry {
                vertex_id: vb.vertex_id.clone(),
                pos: lerp_vec2(va.pos, vb.pos, t),
            });
        } else {
            // Vertex exists in b but not a (added after keyframe a) — use b position
            vertex_positions.push(VertexPoseEntry {
                vertex_id: vb.vertex_id.clone(),
                pos: vb.pos,
            });
        }
    }

    ElementPose {
        element_id: a.element_id.clone(),
        layer_id: a.layer_id.clone(),
        position: lerp_vec2(a.position, b.position, t),
        rotation: lerp_angle(a.rotation, b.rotation, t),
        scale: lerp_vec2(a.scale, b.scale, t),
        visible,
        stroke_color_index,
        fill_color_index,
        vertex_positions,
    }
}

// ── Core evaluation ───────────────────────────────────────────────────────────

/// Evaluate the animation at `time_secs`, returning a map of element_id → interpolated pose.
///
/// Uses sparse per-element search:
/// - For each element that appears in any keyframe, find the surrounding keyframes.
/// - If both before/after found: interpolate.
/// - If only before: hold at that value.
/// - If only after: interpolate from rest pose (caller handles rest pose by absence from returned map).
///   Actually we return only elements where we have at least one keyframe — elements with no
///   keyframe are absent from the map and the caller renders them at rest pose.
pub fn evaluate_pose(sprite: &Sprite, sequence: &AnimationSequence, time_secs: f32) -> HashMap<String, ElementPose> {
    let mut result: HashMap<String, ElementPose> = HashMap::new();

    if sequence.pose_keyframes.is_empty() {
        return result;
    }

    // Collect all unique element IDs that appear in any keyframe
    let mut element_ids: Vec<String> = Vec::new();
    for kf in &sequence.pose_keyframes {
        for ep in &kf.element_poses {
            if !element_ids.contains(&ep.element_id) {
                element_ids.push(ep.element_id.clone());
            }
        }
    }

    // keyframes are assumed sorted by time_secs
    let keyframes = &sequence.pose_keyframes;

    for element_id in &element_ids {
        // Find prev keyframe (latest at or before time) that contains this element
        let prev = keyframes.iter()
            .filter(|kf| kf.time_secs <= time_secs)
            .filter_map(|kf| kf.element_poses.iter().find(|ep| &ep.element_id == element_id).map(|ep| (kf, ep)))
            .last();

        // Find next keyframe (earliest after time) that contains this element
        let next = keyframes.iter()
            .filter(|kf| kf.time_secs > time_secs)
            .find_map(|kf| kf.element_poses.iter().find(|ep| &ep.element_id == element_id).map(|ep| (kf, ep)));

        match (prev, next) {
            (Some((kf_a, ep_a)), Some((kf_b, ep_b))) => {
                // Interpolate between prev and next
                let span = kf_b.time_secs - kf_a.time_secs;
                let raw_t = if span > 1e-6 {
                    ((time_secs - kf_a.time_secs) / span).clamp(0.0, 1.0)
                } else {
                    1.0
                };
                result.insert(element_id.clone(), interpolate_element_pose(ep_a, ep_b, raw_t, &kf_b.easing));
            }
            (Some((_kf_a, ep_a)), None) => {
                // Past the last keyframe for this element — hold at last value
                result.insert(element_id.clone(), ep_a.clone());
            }
            (None, Some((kf_b, ep_b))) => {
                // Before the first keyframe for this element — interpolate from rest pose.
                // Get rest pose values from the sprite directly.
                if let Some(rest) = rest_pose_for_element(sprite, element_id) {
                    let first_time = kf_b.time_secs;
                    let raw_t = if first_time > 1e-6 {
                        (time_secs / first_time).clamp(0.0, 1.0)
                    } else {
                        1.0
                    };
                    result.insert(element_id.clone(), interpolate_element_pose(&rest, ep_b, raw_t, &kf_b.easing));
                } else {
                    // Element not in sprite anymore — skip
                }
            }
            (None, None) => {
                // No keyframe for this element — should not happen since we collected from keyframes
            }
        }
    }

    result
}

/// Build an ElementPose from the sprite's current (rest) state for a given element.
fn rest_pose_for_element(sprite: &Sprite, element_id: &str) -> Option<ElementPose> {
    for layer in &sprite.layers {
        for elem in &layer.elements {
            if elem.id == element_id {
                return Some(element_to_pose(elem, &layer.id));
            }
        }
    }
    None
}

fn element_to_pose(elem: &StrokeElement, layer_id: &str) -> ElementPose {
    ElementPose {
        element_id: elem.id.clone(),
        layer_id: layer_id.to_string(),
        position: elem.position,
        rotation: elem.rotation,
        scale: elem.scale,
        visible: true,
        stroke_color_index: elem.stroke_color_index,
        fill_color_index: elem.fill_color_index,
        vertex_positions: elem.vertices.iter().map(|v| VertexPoseEntry {
            vertex_id: v.id.clone(),
            pos: v.pos,
        }).collect(),
    }
}

// ── capture_pose ──────────────────────────────────────────────────────────────

/// Capture the current sprite state as a PoseKeyframe.
///
/// - `selected_ids`: if `Some`, only captures elements whose IDs are in the list.
///   If `None`, captures all visible elements.
pub fn capture_pose(
    sprite: &Sprite,
    time_secs: f32,
    easing_preset: &str,
    selected_ids: Option<&[String]>,
) -> PoseKeyframe {
    let mut element_poses = Vec::new();

    for layer in &sprite.layers {
        if !layer.visible { continue; }
        for elem in &layer.elements {
            let should_capture = match selected_ids {
                Some(ids) => ids.contains(&elem.id),
                None => true,
            };
            if should_capture {
                element_poses.push(element_to_pose(elem, &layer.id));
            }
        }
    }

    PoseKeyframe {
        id: uuid::Uuid::new_v4().to_string(),
        time_secs,
        easing: EasingCurve::from_preset(easing_preset),
        element_poses,
    }
}

// ── auto_key_capture ─────────────────────────────────────────────────────────

/// If auto-key is enabled and a sequence is selected, capture/merge a keyframe
/// at the current playhead time for the given element IDs.
///
/// Call this **before** `history.end_drag()` so the undo snapshot includes both
/// the transform change and the auto-key'd keyframe.
pub fn auto_key_capture(
    timeline: &mut crate::state::editor::TimelineState,
    sprite: &mut Sprite,
    element_ids: &[String],
) {
    if !timeline.auto_key { return; }
    let Some(ref seq_id) = timeline.selected_sequence_id else { return; };
    if element_ids.is_empty() { return; }

    let time = timeline.playhead_time;

    // Capture poses for the affected elements from the current sprite state
    let captured = capture_pose(sprite, time, "ease-in-out", Some(element_ids));

    let Some(seq) = sprite.animations.iter_mut().find(|s| &s.id == seq_id) else { return; };

    // Merge into existing keyframe at this time, or create a new one
    if let Some(existing) = seq.pose_keyframes.iter_mut().find(|kf| (kf.time_secs - time).abs() < 0.001) {
        // Merge: update existing element poses, add new ones
        for new_pose in &captured.element_poses {
            if let Some(ep) = existing.element_poses.iter_mut().find(|ep| ep.element_id == new_pose.element_id) {
                *ep = new_pose.clone();
            } else {
                existing.element_poses.push(new_pose.clone());
            }
        }
    } else {
        let kf_id = captured.id.clone();
        seq.pose_keyframes.push(captured);
        seq.pose_keyframes.sort_by(|a, b| a.time_secs.partial_cmp(&b.time_secs).unwrap());
        if time > seq.duration_secs {
            seq.duration_secs = time;
        }
        timeline.selected_keyframe_id = Some(kf_id);
    }
}

// ── build_evaluated_sprite ────────────────────────────────────────────────────

/// Build a sprite view with poses applied.
///
/// Returns `Cow::Borrowed(sprite)` when no poses are provided (zero cost — the common
/// non-animation path). Returns `Cow::Owned(...)` with overridden element values
/// when animation is playing.
pub fn build_evaluated_sprite<'a>(
    sprite: &'a Sprite,
    poses: Option<&HashMap<String, ElementPose>>,
) -> Cow<'a, Sprite> {
    let poses = match poses {
        Some(p) if !p.is_empty() => p,
        _ => return Cow::Borrowed(sprite),
    };

    let mut owned = sprite.clone();
    for layer in &mut owned.layers {
        for elem in &mut layer.elements {
            if let Some(pose) = poses.get(&elem.id) {
                elem.position = pose.position;
                elem.rotation = pose.rotation;
                elem.scale = pose.scale;
                // Apply vertex positions
                for v in &mut elem.vertices {
                    if let Some(vp) = pose.vertex_positions.iter().find(|vp| vp.vertex_id == v.id) {
                        v.pos = vp.pos;
                    }
                }
                // Step-interpolated properties
                elem.stroke_color_index = pose.stroke_color_index;
                elem.fill_color_index = pose.fill_color_index;
                // Visibility: hide the element by removing it is too destructive;
                // instead we keep it but rely on render_elements respecting a
                // future visibility flag. For now mark it via a sentinel approach:
                // the layer visibility is not touched — only per-element visibility
                // will be handled when the model adds a visible field to StrokeElement.
            }
        }
    }
    Cow::Owned(owned)
}

// ── canvas_state ──────────────────────────────────────────────────────────────

/// Determine the canvas animation state for the colored border / status bar dot.
pub fn canvas_state(
    sequence: Option<&AnimationSequence>,
    playhead_time: f32,
) -> CanvasAnimState {
    let sequence = match sequence {
        Some(s) => s,
        None => return CanvasAnimState::Rest,
    };

    if sequence.pose_keyframes.is_empty() {
        return CanvasAnimState::Rest;
    }

    // Check if playhead is exactly on a keyframe (within 1ms)
    for kf in &sequence.pose_keyframes {
        if (kf.time_secs - playhead_time).abs() < 0.001 {
            return CanvasAnimState::OnKeyframe(kf.id.clone());
        }
    }

    CanvasAnimState::Interpolated
}

// ── Phase 8: apply_evaluated_to_sprite ────────────────────────────────────────

/// Apply evaluated animation poses to the sprite data in-place.
/// This overwrites element positions/rotations/scales/vertices/colors with the
/// interpolated values, so that canvas drags start from the visual position.
pub fn apply_evaluated_to_sprite(sprite: &mut Sprite, poses: &HashMap<String, ElementPose>) {
    if poses.is_empty() { return; }
    for layer in &mut sprite.layers {
        for elem in &mut layer.elements {
            if let Some(pose) = poses.get(&elem.id) {
                elem.position = pose.position;
                elem.rotation = pose.rotation;
                elem.scale = pose.scale;
                elem.stroke_color_index = pose.stroke_color_index;
                elem.fill_color_index = pose.fill_color_index;
                for v in &mut elem.vertices {
                    if let Some(vp) = pose.vertex_positions.iter().find(|vp| vp.vertex_id == v.id) {
                        v.pos = vp.pos;
                    }
                }
            }
        }
    }
}

// ── Phase 8: mirror_element_poses ────────────────────────────────────────────

/// Mirror a set of element poses horizontally around the canvas center.
/// Used for walk cycle mirroring: flips positions and negates rotations.
pub fn mirror_element_poses(poses: &[ElementPose], canvas_width: f32) -> Vec<ElementPose> {
    let center_x = canvas_width / 2.0;
    poses.iter().map(|pose| {
        let mut mirrored = pose.clone();
        // Mirror position around canvas center
        mirrored.position.x = 2.0 * center_x - pose.position.x;
        // Negate rotation
        mirrored.rotation = -pose.rotation;
        // Mirror vertex positions around canvas center
        for vp in &mut mirrored.vertex_positions {
            vp.pos.x = 2.0 * center_x - vp.pos.x;
        }
        mirrored
    }).collect()
}

// ── Phase 8: onion skin ghost computation ────────────────────────────────────

/// A ghost frame for onion skin rendering.
pub struct OnionGhost {
    /// Evaluated poses for this ghost frame.
    pub poses: HashMap<String, ElementPose>,
    /// Tint color with alpha baked in.
    pub tint: egui::Color32,
}

/// Compute onion skin ghost frames for rendering.
pub fn compute_onion_skin_ghosts(
    sprite: &Sprite,
    sequence: &AnimationSequence,
    playhead_time: f32,
    mode: crate::state::editor::OnionSkinMode,
    prev_count: u8,
    next_count: u8,
    prev_color: [u8; 3],
    next_color: [u8; 3],
    base_opacity: f32,
) -> Vec<OnionGhost> {
    use crate::state::editor::OnionSkinMode;
    let mut ghosts = Vec::new();

    let keyframe_times: Vec<f32> = sequence.pose_keyframes.iter().map(|kf| kf.time_secs).collect();

    // Keyframe mode: ghosts at adjacent keyframe times
    if mode == OnionSkinMode::Keyframe || mode == OnionSkinMode::Both {
        // Previous keyframes
        let prev_kfs: Vec<f32> = keyframe_times.iter()
            .filter(|&&t| t < playhead_time - 0.001)
            .copied()
            .collect();
        for (i, &t) in prev_kfs.iter().rev().take(prev_count as usize).enumerate() {
            let alpha = (base_opacity * 0.7_f32.powi(i as i32) * 255.0) as u8;
            let poses = evaluate_pose(sprite, sequence, t);
            ghosts.push(OnionGhost {
                poses,
                tint: egui::Color32::from_rgba_unmultiplied(prev_color[0], prev_color[1], prev_color[2], alpha),
            });
        }
        // Next keyframes
        let next_kfs: Vec<f32> = keyframe_times.iter()
            .filter(|&&t| t > playhead_time + 0.001)
            .copied()
            .collect();
        for (i, &t) in next_kfs.iter().take(next_count as usize).enumerate() {
            let alpha = (base_opacity * 0.7_f32.powi(i as i32) * 255.0) as u8;
            let poses = evaluate_pose(sprite, sequence, t);
            ghosts.push(OnionGhost {
                poses,
                tint: egui::Color32::from_rgba_unmultiplied(next_color[0], next_color[1], next_color[2], alpha),
            });
        }
    }

    // Frame mode: ghosts at fixed time offsets
    if mode == OnionSkinMode::Frame || mode == OnionSkinMode::Both {
        let step = (sequence.duration_secs / 10.0).max(0.05);
        for i in 1..=prev_count {
            let t = playhead_time - step * i as f32;
            if t >= 0.0 {
                let alpha = (base_opacity * 0.7_f32.powi((i - 1) as i32) * 255.0) as u8;
                let poses = evaluate_pose(sprite, sequence, t);
                ghosts.push(OnionGhost {
                    poses,
                    tint: egui::Color32::from_rgba_unmultiplied(prev_color[0], prev_color[1], prev_color[2], alpha),
                });
            }
        }
        for i in 1..=next_count {
            let t = playhead_time + step * i as f32;
            if t <= sequence.duration_secs {
                let alpha = (base_opacity * 0.7_f32.powi((i - 1) as i32) * 255.0) as u8;
                let poses = evaluate_pose(sprite, sequence, t);
                ghosts.push(OnionGhost {
                    poses,
                    tint: egui::Color32::from_rgba_unmultiplied(next_color[0], next_color[1], next_color[2], alpha),
                });
            }
        }
    }

    ghosts
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::animation::EasingCurve;

    #[test]
    fn linear_easing_midpoint() {
        let curve = EasingCurve::from_preset("linear");
        let result = eval_cubic_bezier_easing(&curve, 0.5);
        assert!((result - 0.5).abs() < 0.01, "linear(0.5) = {result}");
    }

    #[test]
    fn easing_endpoints() {
        let curve = EasingCurve::from_preset("ease-in-out");
        assert!((eval_cubic_bezier_easing(&curve, 0.0)).abs() < 1e-4);
        assert!((eval_cubic_bezier_easing(&curve, 1.0) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn angle_lerp_shortest_path() {
        use std::f32::consts::PI;
        // Should go from 170° to -170° via 180° (delta = -20°, not +340°)
        let a = 170.0_f32.to_radians();
        let b = -170.0_f32.to_radians();
        let mid = lerp_angle(a, b, 0.5);
        // Should be near ±180°
        assert!(mid.abs() > PI * 0.9, "angle lerp shortest path failed: {mid}");
    }

    #[test]
    fn step_interpolation_snaps() {
        let pose_a = ElementPose {
            element_id: "a".into(), layer_id: "l".into(),
            position: Vec2::ZERO, rotation: 0.0, scale: Vec2::ONE,
            visible: true, stroke_color_index: 1, fill_color_index: 2,
            vertex_positions: vec![],
        };
        let pose_b = ElementPose {
            stroke_color_index: 5, fill_color_index: 6,
            ..pose_a.clone()
        };
        let curve = EasingCurve::from_preset("linear");
        let at_49 = interpolate_element_pose(&pose_a, &pose_b, 0.49, &curve);
        let at_51 = interpolate_element_pose(&pose_a, &pose_b, 0.51, &curve);
        assert_eq!(at_49.stroke_color_index, 1);
        assert_eq!(at_51.stroke_color_index, 5);
    }

    #[test]
    fn canvas_state_variants() {
        let mut seq = AnimationSequence::new("test");
        assert_eq!(canvas_state(Some(&seq), 0.0), CanvasAnimState::Rest);

        seq.pose_keyframes.push(PoseKeyframe {
            id: "kf1".into(), time_secs: 0.0,
            easing: EasingCurve::linear(), element_poses: vec![],
        });
        seq.pose_keyframes.push(PoseKeyframe {
            id: "kf2".into(), time_secs: 1.0,
            easing: EasingCurve::linear(), element_poses: vec![],
        });

        assert!(matches!(canvas_state(Some(&seq), 0.0), CanvasAnimState::OnKeyframe(_)));
        assert_eq!(canvas_state(Some(&seq), 0.5), CanvasAnimState::Interpolated);
        assert_eq!(canvas_state(None, 0.5), CanvasAnimState::Rest);
    }
}
