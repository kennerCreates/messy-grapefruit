use crate::model::sprite::{
    AnimationSequence, EasingCurve, EasingPreset, Sprite,
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
    /// Jump to previous pose
    SkipBackward,
    /// Jump to next pose
    SkipForward,
    /// Toggle looping
    ToggleLoop,
    /// Toggle onion skinning
    ToggleOnionSkinning,
    /// Set onion before count
    SetOnionBefore(usize),
    /// Set onion after count
    SetOnionAfter(usize),
    /// Set the current easing preset for new poses
    SetCurrentEasing(EasingPreset),
    /// Set the sequence duration
    SetDuration(f32),
    /// Insert a pose keyframe at the given time (snapshots current sprite state)
    InsertPose { time: f32, easing: EasingCurve },
    /// Delete a pose keyframe by ID
    DeletePose(String),
    /// Select a pose keyframe by ID
    SelectPose(Option<String>),
    /// Update the easing on a pose keyframe
    UpdatePoseEasing { pose_id: String, easing: EasingCurve },
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
        .min_height(120.0)
        .max_height(300.0)
        .resizable(true)
        .show(ctx, |ui| {
            // === Sequence tabs row ===
            draw_sequence_tabs(ui, sprite, anim_state, &mut actions);

            ui.separator();

            // === Player controls row ===
            draw_player_controls(ui, sprite, anim_state, &mut actions);

            ui.separator();

            // === Pose timeline area ===
            let selected_seq = anim_state.selected_sequence_id.as_ref().and_then(|id| {
                sprite.animations.iter().find(|a| a.id == *id)
            });

            if let Some(seq) = selected_seq {
                draw_pose_timeline(ui, seq, anim_state, &mut actions);
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

        // Skip backward (previous pose)
        if ui
            .add_enabled(play_enabled, egui::Button::new("\u{23EA} Prev"))
            .on_hover_text("Previous pose")
            .clicked()
        {
            actions.push(TimelineAction::SkipBackward);
        }

        // Skip forward (next pose)
        if ui
            .add_enabled(play_enabled, egui::Button::new("\u{23E9} Next"))
            .on_hover_text("Next pose")
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
    });
}

fn draw_pose_timeline(
    ui: &mut egui::Ui,
    seq: &AnimationSequence,
    anim_state: &AnimationState,
    actions: &mut Vec<TimelineAction>,
) {
    let available_width = ui.available_width();
    let track_height = 30.0;
    let label_width = 100.0;
    let timeline_width = (available_width - label_width - 20.0).max(100.0);
    let duration = seq.duration.max(f32::EPSILON);

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

    // Single pose track row
    ui.horizontal(|ui| {
        // Label area
        ui.allocate_ui_with_layout(
            egui::vec2(label_width, track_height),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.label("Poses");
            },
        );

        // Timeline area
        let (track_rect, track_response) = ui.allocate_exact_size(
            egui::vec2(timeline_width, track_height),
            egui::Sense::click(),
        );

        let painter = ui.painter();

        // Track background
        painter.rect_filled(
            track_rect,
            0.0,
            egui::Color32::from_rgba_unmultiplied(35, 40, 55, 200),
        );

        // Draw pose keyframe diamonds
        for pose in &seq.pose_keyframes {
            let kf_x = track_rect.min.x + (pose.time / duration) * timeline_width;
            let kf_y = track_rect.center().y;

            let is_selected = anim_state
                .selected_pose_id
                .as_ref()
                .map(|id| id == &pose.id)
                .unwrap_or(false);

            let kf_color = if is_selected {
                egui::Color32::from_rgb(0xff, 0xff, 0x00)
            } else {
                egui::Color32::from_rgb(0xee, 0x86, 0x95)
            };

            // Diamond shape
            let size = 6.0;
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

        // Click on timeline: select existing pose or no-op
        if track_response.clicked()
            && let Some(pos) = track_response.interact_pointer_pos() {
                // Check if clicking near an existing pose
                let near_pose = seq.pose_keyframes.iter().find(|p| {
                    let p_x = track_rect.min.x + (p.time / duration) * timeline_width;
                    (pos.x - p_x).abs() < 10.0
                });

                if let Some(pose) = near_pose {
                    actions.push(TimelineAction::SelectPose(Some(pose.id.clone())));
                    actions.push(TimelineAction::SetTime(pose.time));
                } else {
                    actions.push(TimelineAction::SelectPose(None));
                }
            }

        // Right-click on pose for context menu
        track_response.context_menu(|ui| {
            if let Some(pos) = ui.ctx().pointer_latest_pos() {
                let near_pose = seq.pose_keyframes.iter().find(|p| {
                    let p_x = track_rect.min.x + (p.time / duration) * timeline_width;
                    (pos.x - p_x).abs() < 10.0
                });

                if let Some(pose) = near_pose {
                    ui.label(format!("Pose at {:.2}s", pose.time));
                    ui.separator();

                    // Easing selector
                    let easing_name = match pose.easing.preset {
                        EasingPreset::Linear => "Linear",
                        EasingPreset::EaseIn => "Ease In",
                        EasingPreset::EaseOut => "Ease Out",
                        EasingPreset::EaseInOut => "Ease In/Out",
                        EasingPreset::Bounce => "Bounce",
                        EasingPreset::Elastic => "Elastic",
                        EasingPreset::Step => "Step",
                        EasingPreset::Custom => "Custom",
                    };
                    ui.label(format!("Easing: {}", easing_name));

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
                        if ui.button(*name).clicked() {
                            actions.push(TimelineAction::UpdatePoseEasing {
                                pose_id: pose.id.clone(),
                                easing: easing_from_preset(*preset),
                            });
                            ui.close_menu();
                        }
                    }

                    ui.separator();
                    if ui.button("Delete Pose").clicked() {
                        actions.push(TimelineAction::DeletePose(pose.id.clone()));
                        ui.close_menu();
                    }
                }
            }
        });
    });

    // Insert pose button
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui.button("+ Insert Pose").on_hover_text("Capture current sprite state as a pose keyframe at the current time").clicked() {
            let easing = easing_from_preset(anim_state.current_easing);
            actions.push(TimelineAction::InsertPose {
                time: anim_state.current_time,
                easing,
            });
        }

        // Show selected pose info
        if let Some(ref pose_id) = anim_state.selected_pose_id
            && let Some(pose) = seq.pose_keyframes.iter().find(|p| &p.id == pose_id)
        {
            ui.separator();
            ui.label(format!("Selected: pose at {:.2}s ({} elements)",
                pose.time, pose.element_poses.len()));

            if ui.button("Delete").clicked() {
                actions.push(TimelineAction::DeletePose(pose_id.clone()));
            }
        }
    });
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
