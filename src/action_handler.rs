use crate::action::AppAction;
use crate::model::project::PaletteColor;
use crate::model::sprite::{Layer, Sprite};
use crate::App;

/// Dispatch an action, mutating sprite/project/history as needed.
pub fn dispatch(app: &mut App, action: AppAction) {
    let before = app.sprite.clone();
    let layer_idx = app.editor.layer.resolve_active_idx(&app.sprite);

    match action {
        AppAction::CommitStroke(element) => {
            let eid = element.id.clone();
            let group_id = app.sprite.layers.get(layer_idx).and_then(|l| l.group_id.clone());
            let mut new_layer = Layer::new_with_element(element);
            new_layer.group_id = group_id;
            let insert_idx = layer_idx + 1;
            app.sprite.layers.insert(insert_idx.min(app.sprite.layers.len()), new_layer.clone());
            app.editor.layer.active_layer_id = Some(new_layer.id);
            app.sprite.cleanup_empty_layers();
            crate::engine::animation::auto_key_capture(
                &mut app.editor.timeline, &mut app.sprite, &[eid],
            );
            app.history.push("Draw stroke".into(), before, app.sprite.clone());
        }
        AppAction::CommitSymmetricStrokes(elements) => {
            let eids: Vec<String> = elements.iter().map(|e| e.id.clone()).collect();
            let group_id = app.sprite.layers.get(layer_idx).and_then(|l| l.group_id.clone());
            let mut last_layer_id = None;
            for (i, elem) in elements.into_iter().enumerate() {
                let mut new_layer = Layer::new_with_element(elem);
                new_layer.group_id = group_id.clone();
                let insert_idx = (layer_idx + 1 + i).min(app.sprite.layers.len());
                last_layer_id = Some(new_layer.id.clone());
                app.sprite.layers.insert(insert_idx, new_layer);
            }
            if let Some(id) = last_layer_id {
                app.editor.layer.active_layer_id = Some(id);
            }
            app.sprite.cleanup_empty_layers();
            crate::engine::animation::auto_key_capture(
                &mut app.editor.timeline, &mut app.sprite, &eids,
            );
            app.history.push("Draw symmetric strokes".into(), before, app.sprite.clone());
        }
        AppAction::SetFillColor { element_id, fill_color_index } => {
            for layer in &mut app.sprite.layers {
                for elem in &mut layer.elements {
                    if elem.id == element_id {
                        elem.fill_color_index = fill_color_index;
                        // Flat fill replaces gradient fill
                        elem.gradient_fill = None;
                    }
                }
            }
            crate::engine::animation::auto_key_capture(
                &mut app.editor.timeline, &mut app.sprite, &[element_id],
            );
            app.history.push("Set fill color".into(), before, app.sprite.clone());
        }
        AppAction::SetBackgroundColor { background_color_index } => {
            app.sprite.background_color_index = background_color_index;
            app.history.push("Set background color".into(), before, app.sprite.clone());
        }
        AppAction::EraseVertex { element_id, vertex_id } => {
            if let Some((li, ei)) = find_element_location(&app.sprite, &element_id) {
                let element = &app.sprite.layers[li].elements[ei];
                let group_id = app.sprite.layers[li].group_id.clone();
                let result = crate::engine::eraser::erase_vertex(
                    element, &vertex_id, app.project.min_corner_radius,
                );
                app.sprite.layers[li].elements.remove(ei);
                // First result element stays in existing layer; extras get new layers
                for (i, new_elem) in result.new_elements.into_iter().enumerate() {
                    if i == 0 {
                        app.sprite.layers[li].elements.push(new_elem);
                    } else {
                        let mut new_layer = Layer::new_with_element(new_elem);
                        new_layer.group_id = group_id.clone();
                        app.sprite.layers.insert(li + i, new_layer);
                    }
                }
                app.sprite.cleanup_empty_layers();
                app.editor.layer.validate(&app.sprite);
                app.history.push("Erase vertex".into(), before, app.sprite.clone());
            }
        }
        AppAction::EraseSegment { element_id, segment_index } => {
            if let Some((li, ei)) = find_element_location(&app.sprite, &element_id) {
                let element = &app.sprite.layers[li].elements[ei];
                let group_id = app.sprite.layers[li].group_id.clone();
                let result = crate::engine::eraser::erase_segment(
                    element, segment_index, app.project.min_corner_radius,
                );
                app.sprite.layers[li].elements.remove(ei);
                for (i, new_elem) in result.new_elements.into_iter().enumerate() {
                    if i == 0 {
                        app.sprite.layers[li].elements.push(new_elem);
                    } else {
                        let mut new_layer = Layer::new_with_element(new_elem);
                        new_layer.group_id = group_id.clone();
                        app.sprite.layers.insert(li + i, new_layer);
                    }
                }
                app.sprite.cleanup_empty_layers();
                app.editor.layer.validate(&app.sprite);
                app.history.push("Erase segment".into(), before, app.sprite.clone());
            }
        }
        AppAction::AddPaletteColor(color) => {
            if app.project.palette.colors.len() < 256 {
                app.project.palette.colors.push(color);
                crate::io::save_app_defaults(&app.project);
            }
            // Project-level, no sprite undo
        }
        AppAction::DeletePaletteColor(index) => {
            if index == 0 || index as usize >= app.project.palette.colors.len() {
                return;
            }
            app.project.palette.colors.remove(index as usize);
            // Remap all sprite color indices
            for layer in &mut app.sprite.layers {
                for elem in &mut layer.elements {
                    elem.stroke_color_index = remap_color_index(elem.stroke_color_index, index);
                    elem.fill_color_index = remap_color_index(elem.fill_color_index, index);
                    if let Some(ref mut grad) = elem.gradient_fill {
                        for stop in &mut grad.stops {
                            stop.color_index = remap_color_index(stop.color_index, index);
                        }
                    }
                }
            }
            app.sprite.background_color_index =
                remap_color_index(app.sprite.background_color_index, index);
            app.history.push("Delete palette color".into(), before, app.sprite.clone());
            crate::io::save_app_defaults(&app.project);
        }
        AppAction::EditPaletteColor { index, color } => {
            if let Some(c) = app.project.palette.colors.get_mut(index as usize) {
                *c = color;
            }
            crate::io::save_app_defaults(&app.project);
            // Project-level, no sprite undo
        }
        AppAction::ImportPalette(colors) => {
            app.project.palette.colors = colors;
            // Ensure index 0 is transparent
            if app.project.palette.colors.is_empty()
                || app.project.palette.colors[0].a != 0
            {
                app.project.palette.colors.insert(0, PaletteColor::transparent());
            }
            // Truncate to 256
            app.project.palette.colors.truncate(256);
            // Auto-pick theme colors from the new palette
            let (dark, light) =
                crate::model::project::auto_pick_theme_colors(&app.project.palette);
            app.project.editor_preferences.dark_theme_colors = dark;
            app.project.editor_preferences.light_theme_colors = light;
            crate::io::save_app_defaults(&app.project);
            // Project-level, no sprite undo
        }
        AppAction::AddReferenceImage(ref_image) => {
            app.sprite.reference_images.push(ref_image);
            app.history.push("Add reference image".into(), before, app.sprite.clone());
        }
        AppAction::RemoveReferenceImage(id) => {
            app.sprite.reference_images.retain(|r| r.id != id);
            app.ref_image_textures.remove(&id);
            app.history.push("Remove reference image".into(), before, app.sprite.clone());
        }

        // ── Phase 6: Gradient & Hatch Fills ─────────────────────────

        AppAction::SetGradientFill { element_id, gradient_fill } => {
            for layer in &mut app.sprite.layers {
                for elem in &mut layer.elements {
                    if elem.id == element_id {
                        elem.gradient_fill = Some(gradient_fill.clone());
                    }
                }
            }
            app.history.push("Set gradient fill".into(), before, app.sprite.clone());
        }
        AppAction::ClearGradientFill { element_id } => {
            for layer in &mut app.sprite.layers {
                for elem in &mut layer.elements {
                    if elem.id == element_id {
                        elem.gradient_fill = None;
                    }
                }
            }
            app.history.push("Clear gradient fill".into(), before, app.sprite.clone());
        }
        AppAction::SetHatchFill { element_id, hatch_fill_id } => {
            for layer in &mut app.sprite.layers {
                for elem in &mut layer.elements {
                    if elem.id == element_id {
                        elem.hatch_fill_id = Some(hatch_fill_id.clone());
                    }
                }
            }
            app.history.push("Set hatch fill".into(), before, app.sprite.clone());
        }
        AppAction::ClearHatchFill { element_id } => {
            for layer in &mut app.sprite.layers {
                for elem in &mut layer.elements {
                    if elem.id == element_id {
                        elem.hatch_fill_id = None;
                    }
                }
            }
            app.history.push("Clear hatch fill".into(), before, app.sprite.clone());
        }
        AppAction::AddHatchPattern(pattern) => {
            app.project.hatch_patterns.push(pattern);
            crate::io::save_app_defaults(&app.project);
        }
        AppAction::UpdateHatchPattern(pattern) => {
            if let Some(p) = app.project.hatch_patterns.iter_mut().find(|p| p.id == pattern.id) {
                *p = pattern;
            }
            crate::io::save_app_defaults(&app.project);
        }
        AppAction::DeleteHatchPattern(id) => {
            app.project.hatch_patterns.retain(|p| p.id != id);
            // Clear references on all elements
            for layer in &mut app.sprite.layers {
                for elem in &mut layer.elements {
                    if elem.hatch_fill_id.as_deref() == Some(id.as_str()) {
                        elem.hatch_fill_id = None;
                    }
                }
            }
            app.history.push("Delete hatch pattern".into(), before, app.sprite.clone());
            crate::io::save_app_defaults(&app.project);
        }
        AppAction::ImportHatchPatterns(patterns) => {
            for pattern in patterns {
                // Skip duplicates by name
                if !app.project.hatch_patterns.iter().any(|p| p.name == pattern.name) {
                    app.project.hatch_patterns.push(pattern);
                }
            }
            crate::io::save_app_defaults(&app.project);
        }

        // ── Phase 7: Animation ───────────────────────────────────────────────

        AppAction::CreateSequence { name } => {
            let seq = crate::model::animation::AnimationSequence::new(name);
            let seq_id = seq.id.clone();
            app.sprite.animations.push(seq);
            app.editor.timeline.selected_sequence_id = Some(seq_id);
            app.editor.timeline.playhead_time = 0.0;
            app.editor.playback.playing = false;
            app.editor.playback.last_frame_time = None;
            app.history.push("Create animation".into(), before, app.sprite.clone());
        }
        AppAction::DeleteSequence { sequence_id } => {
            app.sprite.animations.retain(|s| s.id != sequence_id);
            if app.editor.timeline.selected_sequence_id.as_deref() == Some(&sequence_id) {
                app.editor.timeline.selected_sequence_id = app.sprite.animations.first().map(|s| s.id.clone());
                app.editor.timeline.playhead_time = 0.0;
                app.editor.playback.playing = false;
                app.editor.playback.last_frame_time = None;
            }
            app.history.push("Delete animation".into(), before, app.sprite.clone());
        }
        AppAction::RenameSequence { sequence_id, name } => {
            if let Some(seq) = app.sprite.animations.iter_mut().find(|s| s.id == sequence_id) {
                seq.name = name;
            }
            app.history.push("Rename animation".into(), before, app.sprite.clone());
        }
        AppAction::SelectSequence { sequence_id } => {
            app.editor.timeline.selected_sequence_id = sequence_id;
            app.editor.timeline.playhead_time = 0.0;
            app.editor.timeline.selected_keyframe_id = None;
            app.editor.playback.playing = false;
            app.editor.playback.last_frame_time = None;
            // Navigation — no undo
        }
        AppAction::InsertPose { sequence_id, selected_ids } => {
            let time_secs = app.editor.timeline.playhead_time;
            let selected_refs: Option<Vec<String>> = selected_ids;
            let keyframe = crate::engine::animation::capture_pose(
                &app.sprite,
                time_secs,
                "ease-in-out",
                selected_refs.as_deref(),
            );
            let kf_id = keyframe.id.clone();
            if let Some(seq) = app.sprite.animations.iter_mut().find(|s| s.id == sequence_id) {
                // Remove any existing keyframe at this exact time
                seq.pose_keyframes.retain(|kf| (kf.time_secs - time_secs).abs() >= 0.001);
                seq.pose_keyframes.push(keyframe);
                // Keep sorted by time
                seq.pose_keyframes.sort_by(|a, b| a.time_secs.partial_cmp(&b.time_secs).unwrap());
                // Auto-extend duration
                if time_secs > seq.duration_secs {
                    seq.duration_secs = time_secs;
                }
            }
            app.editor.timeline.selected_keyframe_id = Some(kf_id);
            app.history.push("Insert pose".into(), before, app.sprite.clone());
        }
        AppAction::DeleteKeyframe { sequence_id, keyframe_id } => {
            if let Some(seq) = app.sprite.animations.iter_mut().find(|s| s.id == sequence_id) {
                seq.pose_keyframes.retain(|kf| kf.id != keyframe_id);
            }
            if app.editor.timeline.selected_keyframe_id.as_deref() == Some(&keyframe_id) {
                app.editor.timeline.selected_keyframe_id = None;
            }
            app.history.push("Delete keyframe".into(), before, app.sprite.clone());
        }
        AppAction::SetPlayheadTime { time_secs } => {
            // Clamp to active sequence duration
            let max_time = app.editor.timeline.selected_sequence_id.as_ref()
                .and_then(|id| app.sprite.animations.iter().find(|s| &s.id == id))
                .map(|s| s.duration_secs)
                .unwrap_or(0.0);
            app.editor.timeline.playhead_time = time_secs.clamp(0.0, max_time.max(0.0));
            // Navigation — no undo
        }
        AppAction::SetSequenceDuration { sequence_id, duration_secs } => {
            if let Some(seq) = app.sprite.animations.iter_mut().find(|s| s.id == sequence_id) {
                seq.duration_secs = duration_secs.max(0.1);
            }
            app.history.push("Set duration".into(), before, app.sprite.clone());
        }
        AppAction::SetSequenceLooping { sequence_id, looping } => {
            if let Some(seq) = app.sprite.animations.iter_mut().find(|s| s.id == sequence_id) {
                seq.looping = looping;
            }
            app.history.push("Set looping".into(), before, app.sprite.clone());
        }
        AppAction::SetEasingCurve { sequence_id, keyframe_id, easing } => {
            if let Some(seq) = app.sprite.animations.iter_mut().find(|s| s.id == sequence_id)
                && let Some(kf) = seq.pose_keyframes.iter_mut().find(|kf| kf.id == keyframe_id)
            {
                kf.easing = easing;
            }
            app.history.push("Set easing".into(), before, app.sprite.clone());
        }

        // ── Phase 8: Animation Workflow ───────────────────────────────────────

        AppAction::AddEventMarker { sequence_id, time_secs, name } => {
            if let Some(seq) = app.sprite.animations.iter_mut().find(|s| s.id == sequence_id) {
                let marker = crate::model::animation::EventMarker {
                    id: uuid::Uuid::new_v4().to_string(),
                    time_secs,
                    name,
                };
                seq.event_markers.push(marker);
                seq.event_markers.sort_by(|a, b| a.time_secs.partial_cmp(&b.time_secs).unwrap());
            }
            app.history.push("Add event marker".into(), before, app.sprite.clone());
        }
        AppAction::DeleteEventMarker { sequence_id, marker_id } => {
            if let Some(seq) = app.sprite.animations.iter_mut().find(|s| s.id == sequence_id) {
                seq.event_markers.retain(|m| m.id != marker_id);
            }
            app.history.push("Delete event marker".into(), before, app.sprite.clone());
        }
        AppAction::RenameEventMarker { sequence_id, marker_id, name } => {
            if let Some(seq) = app.sprite.animations.iter_mut().find(|s| s.id == sequence_id)
                && let Some(m) = seq.event_markers.iter_mut().find(|m| m.id == marker_id)
            {
                m.name = name;
            }
            app.history.push("Rename event marker".into(), before, app.sprite.clone());
        }
        AppAction::MoveEventMarker { sequence_id, marker_id, time_secs } => {
            if let Some(seq) = app.sprite.animations.iter_mut().find(|s| s.id == sequence_id) {
                if let Some(m) = seq.event_markers.iter_mut().find(|m| m.id == marker_id) {
                    m.time_secs = time_secs.clamp(0.0, seq.duration_secs);
                }
                seq.event_markers.sort_by(|a, b| a.time_secs.partial_cmp(&b.time_secs).unwrap());
            }
            app.history.push("Move event marker".into(), before, app.sprite.clone());
        }
        AppAction::MoveKeyframe { sequence_id, keyframe_id, new_time } => {
            if let Some(seq) = app.sprite.animations.iter_mut().find(|s| s.id == sequence_id) {
                if let Some(kf) = seq.pose_keyframes.iter_mut().find(|kf| kf.id == keyframe_id) {
                    kf.time_secs = new_time.clamp(0.0, seq.duration_secs);
                }
                seq.pose_keyframes.sort_by(|a, b| a.time_secs.partial_cmp(&b.time_secs).unwrap());
            }
            app.history.push("Move keyframe".into(), before, app.sprite.clone());
        }
        AppAction::PastePose { sequence_id, time_secs, element_poses } => {
            if let Some(seq) = app.sprite.animations.iter_mut().find(|s| s.id == sequence_id) {
                // Remove any existing keyframe at this exact time
                seq.pose_keyframes.retain(|kf| (kf.time_secs - time_secs).abs() >= 0.001);
                let kf = crate::model::animation::PoseKeyframe {
                    id: uuid::Uuid::new_v4().to_string(),
                    time_secs,
                    easing: crate::model::animation::EasingCurve::default(),
                    element_poses,
                };
                let kf_id = kf.id.clone();
                seq.pose_keyframes.push(kf);
                seq.pose_keyframes.sort_by(|a, b| a.time_secs.partial_cmp(&b.time_secs).unwrap());
                if time_secs > seq.duration_secs {
                    seq.duration_secs = time_secs;
                }
                app.editor.timeline.selected_keyframe_id = Some(kf_id);
            }
            app.history.push("Paste pose".into(), before, app.sprite.clone());
        }
        AppAction::MirrorPose { sequence_id, keyframe_id, time_secs } => {
            if let Some(seq) = app.sprite.animations.iter_mut().find(|s| s.id == sequence_id)
                && let Some(source_kf) = seq.pose_keyframes.iter().find(|kf| kf.id == keyframe_id)
            {
                let mirrored_poses = crate::engine::animation::mirror_element_poses(
                    &source_kf.element_poses,
                    app.sprite.canvas_width as f32,
                );
                // Remove any existing keyframe at this exact time
                seq.pose_keyframes.retain(|kf| (kf.time_secs - time_secs).abs() >= 0.001);
                let kf = crate::model::animation::PoseKeyframe {
                    id: uuid::Uuid::new_v4().to_string(),
                    time_secs,
                    easing: crate::model::animation::EasingCurve::default(),
                    element_poses: mirrored_poses,
                };
                let kf_id = kf.id.clone();
                seq.pose_keyframes.push(kf);
                seq.pose_keyframes.sort_by(|a, b| a.time_secs.partial_cmp(&b.time_secs).unwrap());
                if time_secs > seq.duration_secs {
                    seq.duration_secs = time_secs;
                }
                app.editor.timeline.selected_keyframe_id = Some(kf_id);
            }
            app.history.push("Mirror pose".into(), before, app.sprite.clone());
        }
        AppAction::ApplyAnimationTemplate { sequence_id, template_name } => {
            use crate::model::animation::ANIMATION_TEMPLATES;
            if let Some(template) = ANIMATION_TEMPLATES.iter().find(|t| t.name == template_name) {
                // Capture poses first (needs immutable borrow of sprite)
                let keyframes: Vec<_> = template.keyframes.iter().map(|tkf| {
                    crate::engine::animation::capture_pose(
                        &app.sprite,
                        tkf.time_secs,
                        tkf.easing_preset,
                        None,
                    )
                }).collect();
                // Now mutate the sequence
                if let Some(seq) = app.sprite.animations.iter_mut().find(|s| s.id == sequence_id) {
                    seq.duration_secs = template.duration_secs;
                    seq.looping = template.looping;
                    seq.pose_keyframes = keyframes;
                }
            }
            app.history.push("Apply animation template".into(), before, app.sprite.clone());
        }
    }
}

/// Find the layer index and element index of an element by ID.
pub fn find_element_location(sprite: &Sprite, element_id: &str) -> Option<(usize, usize)> {
    for (li, layer) in sprite.layers.iter().enumerate() {
        for (ei, elem) in layer.elements.iter().enumerate() {
            if elem.id == element_id {
                return Some((li, ei));
            }
        }
    }
    None
}

/// Remap a color index after a palette color has been deleted.
/// If the index equals the deleted index, it becomes 0 (transparent).
/// If the index is above the deleted index, it decrements by 1.
fn remap_color_index(index: u8, deleted: u8) -> u8 {
    if index == deleted {
        0
    } else if index > deleted {
        index - 1
    } else {
        index
    }
}
