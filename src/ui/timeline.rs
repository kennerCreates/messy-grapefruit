use crate::engine::animation;
use crate::model::sprite::{
    AnimatableProperty, AnimationSequence, EasingCurve, EasingPreset,
    Sprite,
};
use crate::state::editor::AnimationState;

/// Actions the timeline can request
pub enum TimelineAction {
    /// Select an animation sequence by ID
    SelectSequence(Option<String>),
    /// Create a new animation sequence
    CreateSequence,
    /// Delete a sequence by ID
    DeleteSequence(String),
    /// Rename a sequence
    RenameSequence(String, String),
    /// Set the playhead time
    SetTime(f32),
    /// Toggle play/pause
    TogglePlayback,
    /// Jump to the start (frame 0)
    JumpToStart,
    /// Jump to previous keyframe
    SkipBackward,
    /// Jump to next keyframe
    SkipForward,
    /// Toggle looping
    ToggleLoop,
    /// Toggle onion skinning
    ToggleOnionSkinning,
    /// Set onion before count
    SetOnionBefore(usize),
    /// Set onion after count
    SetOnionAfter(usize),
    /// Add a keyframe to a track
    AddKeyframe {
        track_index: usize,
        time: f32,
        value: f64,
        easing: EasingCurve,
    },
    /// Remove a keyframe by track index and keyframe ID
    RemoveKeyframe {
        track_index: usize,
        keyframe_id: String,
    },
    /// Select a track for curve editing
    SelectTrack(Option<usize>),
    /// Select a keyframe for curve editing
    SelectKeyframe(Option<String>),
    /// Update the easing on a keyframe
    UpdateKeyframeEasing {
        track_index: usize,
        keyframe_id: String,
        easing: EasingCurve,
    },
    /// Set the current easing preset for new keyframes
    SetCurrentEasing(EasingPreset),
    /// Toggle the curve editor sub-panel
    ToggleCurveEditor,
    /// Set the sequence duration
    SetDuration(f32),
    /// Add a property track to the current sequence
    AddTrack {
        property: AnimatableProperty,
        element_id: String,
        layer_id: String,
    },
    /// Remove a track from the current sequence
    RemoveTrack(usize),
}

/// Draw the timeline panel at the bottom of the screen.
pub fn draw_timeline(
    ctx: &egui::Context,
    sprite: &Sprite,
    anim_state: &AnimationState,
) -> Vec<TimelineAction> {
    let mut actions = Vec::new();

    if !anim_state.timeline_visible {
        // Draw a minimal collapsed bar
        egui::TopBottomPanel::bottom("timeline_collapsed")
            .exact_height(24.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Timeline");
                });
            });
        return actions;
    }

    egui::TopBottomPanel::bottom("timeline")
        .min_height(160.0)
        .max_height(400.0)
        .resizable(true)
        .show(ctx, |ui| {
            // === Sequence tabs row ===
            draw_sequence_tabs(ui, sprite, anim_state, &mut actions);

            ui.separator();

            // === Player controls row ===
            draw_player_controls(ui, sprite, anim_state, &mut actions);

            ui.separator();

            // === Timeline tracks area ===
            let selected_seq = anim_state.selected_sequence_id.as_ref().and_then(|id| {
                sprite.animations.iter().find(|a| a.id == *id)
            });

            if let Some(seq) = selected_seq {
                draw_timeline_tracks(ui, seq, anim_state, &mut actions);

                // === Curve editor (if open) ===
                if anim_state.curve_editor_open {
                    ui.separator();
                    draw_curve_editor(ui, seq, anim_state, &mut actions);
                }
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("No animation selected. Click '+' to create one.");
                });
            }
        });

    actions
}

fn draw_sequence_tabs(
    ui: &mut egui::Ui,
    sprite: &Sprite,
    anim_state: &AnimationState,
    actions: &mut Vec<TimelineAction>,
) {
    ui.horizontal(|ui| {
        ui.label("Sequences:");

        // "None" tab for rest pose
        let is_none = anim_state.selected_sequence_id.is_none();
        if ui.selectable_label(is_none, "Rest Pose").clicked() {
            actions.push(TimelineAction::SelectSequence(None));
        }

        for seq in &sprite.animations {
            let is_selected = anim_state
                .selected_sequence_id
                .as_ref()
                .map(|id| id == &seq.id)
                .unwrap_or(false);

            // Check if we're renaming this sequence
            if anim_state.renaming_sequence_id.as_ref() == Some(&seq.id) {
                // Show text edit for rename -- handled via action
                let response = ui.selectable_label(is_selected, &seq.name);
                if response.clicked() {
                    actions.push(TimelineAction::SelectSequence(Some(seq.id.clone())));
                }
            } else {
                let response = ui.selectable_label(is_selected, &seq.name);
                if response.clicked() {
                    actions.push(TimelineAction::SelectSequence(Some(seq.id.clone())));
                }
                // Right-click context menu for rename/delete
                response.context_menu(|ui| {
                    if ui.button("Rename").clicked() {
                        actions.push(TimelineAction::RenameSequence(
                            seq.id.clone(),
                            seq.name.clone(),
                        ));
                        ui.close_menu();
                    }
                    if ui.button("Delete").clicked() {
                        actions.push(TimelineAction::DeleteSequence(seq.id.clone()));
                        ui.close_menu();
                    }
                });
            }
        }

        // + button to create new sequence
        if ui.button("+").on_hover_text("New animation sequence").clicked() {
            actions.push(TimelineAction::CreateSequence);
        }
    });
}

fn draw_player_controls(
    ui: &mut egui::Ui,
    sprite: &Sprite,
    anim_state: &AnimationState,
    actions: &mut Vec<TimelineAction>,
) {
    let selected_seq = anim_state.selected_sequence_id.as_ref().and_then(|id| {
        sprite.animations.iter().find(|a| a.id == *id)
    });

    ui.horizontal(|ui| {
        // Play/Pause button
        let play_label = if anim_state.playing {
            "\u{23F8} Pause"
        } else {
            "\u{25B6} Play"
        };
        let play_enabled = selected_seq.is_some();
        if ui
            .add_enabled(play_enabled, egui::Button::new(play_label))
            .clicked()
        {
            actions.push(TimelineAction::TogglePlayback);
        }

        // Start Over (jump to frame 0)
        if ui
            .add_enabled(play_enabled, egui::Button::new("\u{23EE} Start"))
            .on_hover_text("Jump to frame 0")
            .clicked()
        {
            actions.push(TimelineAction::JumpToStart);
        }

        // Skip backward (previous keyframe)
        if ui
            .add_enabled(play_enabled, egui::Button::new("\u{23EA} Prev"))
            .on_hover_text("Previous keyframe")
            .clicked()
        {
            actions.push(TimelineAction::SkipBackward);
        }

        // Skip forward (next keyframe)
        if ui
            .add_enabled(play_enabled, egui::Button::new("\u{23E9} Next"))
            .on_hover_text("Next keyframe")
            .clicked()
        {
            actions.push(TimelineAction::SkipForward);
        }

        ui.separator();

        // Loop toggle
        let loop_label = if anim_state.looping {
            "\u{1F501} Loop"
        } else {
            "\u{27A1} Once"
        };
        if ui
            .add_enabled(play_enabled, egui::Button::new(loop_label))
            .clicked()
        {
            actions.push(TimelineAction::ToggleLoop);
        }

        ui.separator();

        // Onion skinning toggle
        let onion_label = if anim_state.onion_skinning {
            "\u{1F9C5} Onion: ON"
        } else {
            "\u{1F9C5} Onion: OFF"
        };
        if ui.button(onion_label).clicked() {
            actions.push(TimelineAction::ToggleOnionSkinning);
        }

        if anim_state.onion_skinning {
            let mut before = anim_state.onion_before as f32;
            if ui
                .add(egui::DragValue::new(&mut before).range(0..=5).prefix("B:"))
                .changed()
            {
                actions.push(TimelineAction::SetOnionBefore(before as usize));
            }
            let mut after = anim_state.onion_after as f32;
            if ui
                .add(egui::DragValue::new(&mut after).range(0..=5).prefix("A:"))
                .changed()
            {
                actions.push(TimelineAction::SetOnionAfter(after as usize));
            }
        }

        ui.separator();

        // Time display / scrub
        if let Some(seq) = selected_seq {
            let mut time = anim_state.current_time;
            let frame = (time * 60.0).round() as i32;
            ui.label(format!("Frame: {} | Time: {:.2}s", frame, time));

            let slider_resp = ui.add(
                egui::Slider::new(&mut time, 0.0..=seq.duration)
                    .text("t")
                    .show_value(false),
            );
            if slider_resp.changed() {
                actions.push(TimelineAction::SetTime(time));
            }

            ui.separator();

            // Duration
            let mut dur = seq.duration;
            if ui
                .add(
                    egui::DragValue::new(&mut dur)
                        .range(0.1..=300.0)
                        .speed(0.05)
                        .prefix("Dur: ")
                        .suffix("s"),
                )
                .changed()
            {
                actions.push(TimelineAction::SetDuration(dur));
            }
        } else {
            ui.label("Time: 0.00s (Rest Pose)");
        }

        ui.separator();

        // Easing preset selector
        let easing_label = match anim_state.current_easing {
            EasingPreset::Linear => "Linear",
            EasingPreset::EaseIn => "Ease In",
            EasingPreset::EaseOut => "Ease Out",
            EasingPreset::EaseInOut => "Ease In/Out",
            EasingPreset::Bounce => "Bounce",
            EasingPreset::Elastic => "Elastic",
            EasingPreset::Step => "Step",
            EasingPreset::Custom => "Custom",
        };
        egui::ComboBox::from_id_salt("easing_preset")
            .selected_text(easing_label)
            .show_ui(ui, |ui| {
                let presets = [
                    (EasingPreset::Linear, "Linear"),
                    (EasingPreset::EaseIn, "Ease In"),
                    (EasingPreset::EaseOut, "Ease Out"),
                    (EasingPreset::EaseInOut, "Ease In/Out"),
                    (EasingPreset::Bounce, "Bounce"),
                    (EasingPreset::Elastic, "Elastic"),
                    (EasingPreset::Step, "Step"),
                ];
                for (preset, name) in &presets {
                    if ui
                        .selectable_label(anim_state.current_easing == *preset, *name)
                        .clicked()
                    {
                        actions.push(TimelineAction::SetCurrentEasing(*preset));
                    }
                }
            });

        // Curve editor toggle
        if ui
            .button(if anim_state.curve_editor_open {
                "Curves \u{25BC}"
            } else {
                "Curves \u{25B6}"
            })
            .clicked()
        {
            actions.push(TimelineAction::ToggleCurveEditor);
        }
    });
}

fn draw_timeline_tracks(
    ui: &mut egui::Ui,
    seq: &AnimationSequence,
    anim_state: &AnimationState,
    actions: &mut Vec<TimelineAction>,
) {
    let available_width = ui.available_width();
    let track_height = 24.0;
    let label_width = 160.0;
    let timeline_width = (available_width - label_width - 20.0).max(100.0);
    let duration = seq.duration.max(f32::EPSILON);

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .max_height(200.0)
        .show(ui, |ui| {
            // Draw time ruler
            let (ruler_rect, _) =
                ui.allocate_exact_size(egui::vec2(available_width, 20.0), egui::Sense::hover());

            let ruler_timeline_left = ruler_rect.min.x + label_width;
            let ruler_timeline_right = ruler_timeline_left + timeline_width;
            let painter = ui.painter();

            // Ruler background
            painter.rect_filled(
                ruler_rect,
                0.0,
                egui::Color32::from_rgba_unmultiplied(40, 40, 50, 200),
            );

            // Time markers
            let seconds = duration.ceil() as i32;
            for s in 0..=seconds {
                let t = s as f32;
                if t > duration {
                    break;
                }
                let x = ruler_timeline_left + (t / duration) * timeline_width;
                painter.line_segment(
                    [
                        egui::pos2(x, ruler_rect.min.y),
                        egui::pos2(x, ruler_rect.max.y),
                    ],
                    egui::Stroke::new(1.0, egui::Color32::from_gray(100)),
                );
                painter.text(
                    egui::pos2(x + 2.0, ruler_rect.min.y + 2.0),
                    egui::Align2::LEFT_TOP,
                    format!("{}s", s),
                    egui::FontId::proportional(10.0),
                    egui::Color32::from_gray(180),
                );
            }

            // Playhead on ruler
            let playhead_x =
                ruler_timeline_left + (anim_state.current_time / duration) * timeline_width;
            let playhead_x = playhead_x.clamp(ruler_timeline_left, ruler_timeline_right);
            painter.line_segment(
                [
                    egui::pos2(playhead_x, ruler_rect.min.y),
                    egui::pos2(playhead_x, ruler_rect.max.y),
                ],
                egui::Stroke::new(2.0, egui::Color32::from_rgb(0xee, 0x86, 0x95)),
            );

            // Ruler click to scrub
            let ruler_response = ui.allocate_rect(
                egui::Rect::from_min_size(
                    egui::pos2(ruler_timeline_left, ruler_rect.min.y),
                    egui::vec2(timeline_width, 20.0),
                ),
                egui::Sense::click_and_drag(),
            );
            if (ruler_response.clicked() || ruler_response.dragged())
                && let Some(pos) = ruler_response.hover_pos().or(ruler_response.interact_pointer_pos()) {
                    let t = ((pos.x - ruler_timeline_left) / timeline_width).clamp(0.0, 1.0)
                        * duration;
                    actions.push(TimelineAction::SetTime(t));
                }

            // Draw each track
            for (track_idx, track) in seq.tracks.iter().enumerate() {
                let is_selected = anim_state.selected_track_index == Some(track_idx);

                ui.horizontal(|ui| {
                    // Track label area
                    let _label_response = ui.allocate_ui_with_layout(
                        egui::vec2(label_width, track_height),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            let label = format!(
                                "{} [{}]",
                                track.property.display_name(),
                                &track.element_id[..6.min(track.element_id.len())]
                            );
                            let response = ui.selectable_label(is_selected, label);
                            if response.clicked() {
                                actions.push(TimelineAction::SelectTrack(Some(track_idx)));
                            }
                            // Right-click to remove track
                            response.context_menu(|ui| {
                                if ui.button("Remove Track").clicked() {
                                    actions.push(TimelineAction::RemoveTrack(track_idx));
                                    ui.close_menu();
                                }
                            });
                        },
                    );

                    // Timeline area for this track
                    let (track_rect, track_response) = ui.allocate_exact_size(
                        egui::vec2(timeline_width, track_height),
                        egui::Sense::click(),
                    );

                    let painter = ui.painter();

                    // Track background
                    let bg_color = if is_selected {
                        egui::Color32::from_rgba_unmultiplied(50, 60, 80, 200)
                    } else {
                        egui::Color32::from_rgba_unmultiplied(35, 40, 55, 200)
                    };
                    painter.rect_filled(track_rect, 0.0, bg_color);

                    // Draw keyframe diamonds
                    for kf in &track.keyframes {
                        let kf_x = track_rect.min.x + (kf.time / duration) * timeline_width;
                        let kf_y = track_rect.center().y;

                        let is_kf_selected = anim_state
                            .selected_keyframe_id
                            .as_ref()
                            .map(|id| id == &kf.id)
                            .unwrap_or(false);

                        let kf_color = if is_kf_selected {
                            egui::Color32::from_rgb(0xff, 0xff, 0x00)
                        } else {
                            egui::Color32::from_rgb(0xee, 0x86, 0x95)
                        };

                        // Diamond shape
                        let size = 5.0;
                        let points = vec![
                            egui::pos2(kf_x, kf_y - size),
                            egui::pos2(kf_x + size, kf_y),
                            egui::pos2(kf_x, kf_y + size),
                            egui::pos2(kf_x - size, kf_y),
                        ];
                        painter.add(egui::epaint::PathShape::convex_polygon(
                            points,
                            kf_color,
                            egui::Stroke::NONE,
                        ));
                    }

                    // Playhead line
                    let ph_x = track_rect.min.x
                        + (anim_state.current_time / duration) * timeline_width;
                    let ph_x = ph_x.clamp(track_rect.min.x, track_rect.max.x);
                    painter.line_segment(
                        [
                            egui::pos2(ph_x, track_rect.min.y),
                            egui::pos2(ph_x, track_rect.max.y),
                        ],
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(0xee, 0x86, 0x95)),
                    );

                    // Click on timeline to add keyframe or select existing one
                    if track_response.clicked()
                        && let Some(pos) = track_response.interact_pointer_pos() {
                            let click_t =
                                ((pos.x - track_rect.min.x) / timeline_width).clamp(0.0, 1.0)
                                    * duration;

                            // Check if clicking near an existing keyframe
                            let near_kf = track.keyframes.iter().find(|kf| {
                                let kf_x =
                                    track_rect.min.x + (kf.time / duration) * timeline_width;
                                (pos.x - kf_x).abs() < 8.0
                            });

                            if let Some(kf) = near_kf {
                                actions
                                    .push(TimelineAction::SelectKeyframe(Some(kf.id.clone())));
                                actions.push(TimelineAction::SelectTrack(Some(track_idx)));
                            } else {
                                // Add a keyframe at this time
                                // Get current interpolated value or default to 0
                                let value =
                                    animation::interpolate_track(track, click_t).unwrap_or(0.0);
                                let easing = easing_from_preset(anim_state.current_easing);
                                actions.push(TimelineAction::AddKeyframe {
                                    track_index: track_idx,
                                    time: click_t,
                                    value,
                                    easing,
                                });
                            }
                        }
                });
            }

            // Add track button
            if seq.tracks.is_empty() {
                ui.label("No tracks yet. Use the 'Add Track' section below.");
            }

            // Add track UI: show available properties for selected elements
            ui.add_space(4.0);
            draw_add_track_ui(ui, anim_state, actions);
        });
}

fn draw_add_track_ui(
    ui: &mut egui::Ui,
    _anim_state: &AnimationState,
    actions: &mut Vec<TimelineAction>,
) {
    ui.horizontal(|ui| {
        ui.label("Quick add track:");
        // These create tracks for a placeholder element. In a real implementation,
        // we'd use the currently selected element. For now, provide a simple interface.
        // The user needs to have an element selected to add a track.
        // We use placeholder element/layer IDs that will be resolved by main.rs
        // using the current selection.
        if ui.button("+ pos.x").clicked() {
            actions.push(TimelineAction::AddTrack {
                property: AnimatableProperty::PositionX,
                element_id: String::new(), // Will be filled from selection
                layer_id: String::new(),
            });
        }
        if ui.button("+ pos.y").clicked() {
            actions.push(TimelineAction::AddTrack {
                property: AnimatableProperty::PositionY,
                element_id: String::new(),
                layer_id: String::new(),
            });
        }
        if ui.button("+ rotation").clicked() {
            actions.push(TimelineAction::AddTrack {
                property: AnimatableProperty::Rotation,
                element_id: String::new(),
                layer_id: String::new(),
            });
        }
        if ui.button("+ scale.x").clicked() {
            actions.push(TimelineAction::AddTrack {
                property: AnimatableProperty::ScaleX,
                element_id: String::new(),
                layer_id: String::new(),
            });
        }
        if ui.button("+ scale.y").clicked() {
            actions.push(TimelineAction::AddTrack {
                property: AnimatableProperty::ScaleY,
                element_id: String::new(),
                layer_id: String::new(),
            });
        }
        if ui.button("+ visible").clicked() {
            actions.push(TimelineAction::AddTrack {
                property: AnimatableProperty::Visible,
                element_id: String::new(),
                layer_id: String::new(),
            });
        }
        if ui.button("+ ik.target.x").clicked() {
            actions.push(TimelineAction::AddTrack {
                property: AnimatableProperty::IKTargetX,
                element_id: String::new(), // Will be resolved to IK target element ID
                layer_id: String::new(),
            });
        }
        if ui.button("+ ik.target.y").clicked() {
            actions.push(TimelineAction::AddTrack {
                property: AnimatableProperty::IKTargetY,
                element_id: String::new(),
                layer_id: String::new(),
            });
        }
        if ui.button("+ ik.mix").clicked() {
            actions.push(TimelineAction::AddTrack {
                property: AnimatableProperty::IKMix,
                element_id: String::new(), // Will be resolved to IK chain ID
                layer_id: String::new(),
            });
        }
    });
}

fn draw_curve_editor(
    ui: &mut egui::Ui,
    seq: &AnimationSequence,
    anim_state: &AnimationState,
    actions: &mut Vec<TimelineAction>,
) {
    ui.label("Curve Editor");

    let Some(track_idx) = anim_state.selected_track_index else {
        ui.label("Select a track to edit curves.");
        return;
    };

    let Some(track) = seq.tracks.get(track_idx) else {
        ui.label("Invalid track selection.");
        return;
    };

    if track.keyframes.len() < 2 {
        ui.label("Need at least 2 keyframes to show curves.");
        return;
    }

    // Draw the curve visualization
    let curve_height = 80.0;
    let curve_width = ui.available_width().min(400.0);
    let (curve_rect, _response) =
        ui.allocate_exact_size(egui::vec2(curve_width, curve_height), egui::Sense::click());

    let painter = ui.painter();

    // Background
    painter.rect_filled(
        curve_rect,
        2.0,
        egui::Color32::from_rgba_unmultiplied(30, 30, 40, 220),
    );

    // Find value range
    let min_val = track
        .keyframes
        .iter()
        .map(|k| k.value)
        .fold(f64::MAX, f64::min);
    let max_val = track
        .keyframes
        .iter()
        .map(|k| k.value)
        .fold(f64::MIN, f64::max);
    let val_range = (max_val - min_val).max(0.001);
    let val_padding = val_range * 0.1;
    let min_val = min_val - val_padding;
    let max_val = max_val + val_padding;
    let val_range = max_val - min_val;

    let time_range = seq.duration;

    // Map value to screen y
    let val_to_y = |v: f64| -> f32 {
        let normalized = ((v - min_val) / val_range) as f32;
        curve_rect.max.y - normalized * curve_height
    };

    // Map time to screen x
    let time_to_x = |t: f32| -> f32 { curve_rect.min.x + (t / time_range) * curve_width };

    // Draw the interpolated curve
    let steps = 100;
    let mut prev_point: Option<egui::Pos2> = None;
    for i in 0..=steps {
        let t = (i as f32 / steps as f32) * time_range;
        if let Some(val) = animation::interpolate_track(track, t) {
            let x = time_to_x(t);
            let y = val_to_y(val);
            let point = egui::pos2(x, y);

            if let Some(prev) = prev_point {
                painter.line_segment(
                    [prev, point],
                    egui::Stroke::new(1.5, egui::Color32::from_rgb(0x4a, 0x7a, 0x96)),
                );
            }
            prev_point = Some(point);
        }
    }

    // Draw keyframe points
    for kf in &track.keyframes {
        let x = time_to_x(kf.time);
        let y = val_to_y(kf.value);

        let is_selected = anim_state
            .selected_keyframe_id
            .as_ref()
            .map(|id| id == &kf.id)
            .unwrap_or(false);

        let color = if is_selected {
            egui::Color32::from_rgb(0xff, 0xff, 0x00)
        } else {
            egui::Color32::from_rgb(0xee, 0x86, 0x95)
        };

        painter.circle_filled(egui::pos2(x, y), 4.0, color);
    }

    // Playhead on curve
    let ph_x = time_to_x(anim_state.current_time.min(time_range));
    painter.line_segment(
        [
            egui::pos2(ph_x, curve_rect.min.y),
            egui::pos2(ph_x, curve_rect.max.y),
        ],
        egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(0xee, 0x86, 0x95, 128)),
    );

    // Selected keyframe editing
    if let Some(ref kf_id) = anim_state.selected_keyframe_id
        && let Some(kf) = track.keyframes.iter().find(|k| &k.id == kf_id) {
            ui.horizontal(|ui| {
                ui.label(format!(
                    "Keyframe at {:.2}s = {:.2}",
                    kf.time, kf.value
                ));

                let easing_name = match kf.easing.preset {
                    EasingPreset::Linear => "Linear",
                    EasingPreset::EaseIn => "Ease In",
                    EasingPreset::EaseOut => "Ease Out",
                    EasingPreset::EaseInOut => "Ease In/Out",
                    EasingPreset::Bounce => "Bounce",
                    EasingPreset::Elastic => "Elastic",
                    EasingPreset::Step => "Step",
                    EasingPreset::Custom => "Custom",
                };

                egui::ComboBox::from_id_salt("kf_easing")
                    .selected_text(easing_name)
                    .show_ui(ui, |ui| {
                        let presets = [
                            (EasingPreset::Linear, "Linear"),
                            (EasingPreset::EaseIn, "Ease In"),
                            (EasingPreset::EaseOut, "Ease Out"),
                            (EasingPreset::EaseInOut, "Ease In/Out"),
                            (EasingPreset::Bounce, "Bounce"),
                            (EasingPreset::Elastic, "Elastic"),
                            (EasingPreset::Step, "Step"),
                        ];
                        for (preset, name) in &presets {
                            if ui
                                .selectable_label(kf.easing.preset == *preset, *name)
                                .clicked()
                            {
                                actions.push(TimelineAction::UpdateKeyframeEasing {
                                    track_index: track_idx,
                                    keyframe_id: kf_id.clone(),
                                    easing: easing_from_preset(*preset),
                                });
                            }
                        }
                    });

                if ui.button("Delete KF").clicked() {
                    actions.push(TimelineAction::RemoveKeyframe {
                        track_index: track_idx,
                        keyframe_id: kf_id.clone(),
                    });
                    actions.push(TimelineAction::SelectKeyframe(None));
                }
            });
        }
}

/// Create an EasingCurve from a preset.
pub fn easing_from_preset(preset: EasingPreset) -> EasingCurve {
    match preset {
        EasingPreset::Linear => EasingCurve::linear(),
        EasingPreset::EaseIn => EasingCurve::ease_in(),
        EasingPreset::EaseOut => EasingCurve::ease_out(),
        EasingPreset::EaseInOut => EasingCurve::ease_in_out(),
        EasingPreset::Bounce => EasingCurve::bounce(),
        EasingPreset::Elastic => EasingCurve::elastic(),
        EasingPreset::Step => EasingCurve::step(),
        EasingPreset::Custom => EasingCurve::default(),
    }
}
