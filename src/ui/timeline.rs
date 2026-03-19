use egui::{Color32, Pos2, Rect, Sense, Shape, Stroke, Vec2};

use crate::action::AppAction;
use crate::model::project::Project;
use crate::model::sprite::Sprite;
use crate::state::editor::{EasingPopupState, EditorState, PlaybackState, PoseClipboard, TimelineState};

/// Height of the scrubber track area.
const SCRUBBER_HEIGHT: f32 = 56.0;
/// Horizontal margin at the left of the scrubber (for time label).
const SCRUBBER_MARGIN: f32 = 40.0;
/// Half-size of a keyframe diamond (screen pixels).
const DIAMOND_HALF: f32 = 6.0;
/// Minimum sequence duration (seconds).
const MIN_DURATION: f32 = 0.1;

pub fn show_timeline(
    ui: &mut egui::Ui,
    editor: &mut EditorState,
    sprite: &mut Sprite,
    _project: &Project,
) -> Vec<AppAction> {
    let mut actions: Vec<AppAction> = Vec::new();

    // ── Row 1: Sequence tabs ──────────────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;

        let seq_count = sprite.animations.len();
        for i in 0..seq_count {
            let seq = &sprite.animations[i];
            let seq_id = seq.id.clone();
            let is_active = editor.timeline.selected_sequence_id.as_deref() == Some(&seq_id);

            let label = if editor.timeline.renaming_sequence_id.as_deref() == Some(&seq_id) {
                // Inline rename
                let mut name = seq.name.clone();
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut name)
                        .desired_width(80.0)
                        .font(egui::TextStyle::Small),
                );
                if resp.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let trimmed = name.trim().to_string();
                    let final_name = if trimmed.is_empty() { seq.name.clone() } else { trimmed };
                    actions.push(AppAction::RenameSequence { sequence_id: seq_id.clone(), name: final_name });
                    editor.timeline.renaming_sequence_id = None;
                }
                return;
            } else {
                seq.name.clone()
            };

            let tab_resp = ui.selectable_label(is_active, &label);
            if tab_resp.clicked() && !is_active {
                actions.push(AppAction::SelectSequence { sequence_id: Some(seq_id.clone()) });
            }
            // Right-click context menu
            tab_resp.context_menu(|ui| {
                if ui.button("Rename").clicked() {
                    editor.timeline.renaming_sequence_id = Some(seq_id.clone());
                    ui.close_menu();
                }
                if ui.button("Delete").clicked() {
                    actions.push(AppAction::DeleteSequence { sequence_id: seq_id.clone() });
                    ui.close_menu();
                }
            });
        }

        // "+" button to create new sequence
        if ui.small_button("+").on_hover_text("New animation sequence").clicked() {
            actions.push(AppAction::CreateSequence { name: "Animation".into() });
        }
    });

    // If no sequence is selected, show a hint and return early
    let Some(seq_id) = editor.timeline.selected_sequence_id.clone() else {
        ui.separator();
        ui.label("No animation selected. Click + to create one.");
        return actions;
    };

    let Some(seq_idx) = sprite.animations.iter().position(|s| s.id == seq_id) else {
        return actions;
    };

    let duration = sprite.animations[seq_idx].duration_secs.max(MIN_DURATION);
    let looping = sprite.animations[seq_idx].looping;

    ui.separator();

    // ── Row 2: Playback controls ──────────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;

        // |◀  Start over
        if ui.small_button("|◀").on_hover_text("Go to start").clicked() {
            actions.push(AppAction::SetPlayheadTime { time_secs: 0.0 });
        }

        // ◀◀  Previous keyframe
        if ui.small_button("◀◀").on_hover_text("Previous keyframe").clicked() {
            let t = editor.timeline.playhead_time;
            let prev_time = sprite.animations[seq_idx].pose_keyframes.iter()
                .filter(|kf| kf.time_secs < t - 0.001)
                .map(|kf| kf.time_secs)
                .fold(f32::NEG_INFINITY, f32::max);
            let target = if prev_time.is_finite() { prev_time } else { 0.0 };
            actions.push(AppAction::SetPlayheadTime { time_secs: target });
        }

        // ▶ / ⏸  Play / Pause
        let play_label = if editor.playback.playing { "⏸" } else { "▶" };
        let play_tooltip = if editor.playback.playing { "Pause" } else { "Play" };
        if ui.small_button(play_label).on_hover_text(play_tooltip).clicked() {
            toggle_playback(&mut editor.playback, &mut editor.timeline, duration);
        }

        // ▶▶  Next keyframe
        if ui.small_button("▶▶").on_hover_text("Next keyframe").clicked() {
            let t = editor.timeline.playhead_time;
            let next_time = sprite.animations[seq_idx].pose_keyframes.iter()
                .filter(|kf| kf.time_secs > t + 0.001)
                .map(|kf| kf.time_secs)
                .fold(f32::INFINITY, f32::min);
            let target = if next_time.is_finite() { next_time } else { duration };
            actions.push(AppAction::SetPlayheadTime { time_secs: target });
        }

        // 🔁  Loop toggle
        let loop_label = if looping { "🔁" } else { "→" };
        let loop_tooltip = if looping { "Looping (click to disable)" } else { "One-shot (click to loop)" };
        if ui.small_button(loop_label).on_hover_text(loop_tooltip).clicked() {
            actions.push(AppAction::SetSequenceLooping { sequence_id: seq_id.clone(), looping: !looping });
        }

        ui.separator();

        // Time readout
        let current_t = editor.timeline.playhead_time;
        ui.label(format!("{:.2}s / {:.2}s", current_t, duration));

        ui.separator();

        // Insert Pose button
        let insert_label = if editor.selection.is_empty() { "Insert Pose" } else { "Insert Pose (selected)" };
        let insert_tooltip = if editor.selection.is_empty() {
            "Capture all visible elements as a keyframe at the current time"
        } else {
            "Capture selected elements as a keyframe at the current time"
        };
        if ui.button(insert_label).on_hover_text(insert_tooltip).clicked() {
            let selected_ids = if editor.selection.is_empty() {
                None
            } else {
                Some(editor.selection.selected_ids.clone())
            };
            actions.push(AppAction::InsertPose { sequence_id: seq_id.clone(), selected_ids });
        }

        // Delete selected keyframe
        if let Some(kf_id) = editor.timeline.selected_keyframe_id.clone()
            && ui.small_button("✕").on_hover_text("Delete keyframe").clicked()
        {
            actions.push(AppAction::DeleteKeyframe {
                sequence_id: seq_id.clone(),
                keyframe_id: kf_id,
            });
        }

        ui.separator();

        // Auto-key toggle
        let auto_key_label = if editor.timeline.auto_key { "🔴" } else { "⬤" };
        let auto_key_tip = if editor.timeline.auto_key {
            "Auto-Key: ON (edits auto-create keyframes)"
        } else {
            "Auto-Key: OFF"
        };
        if ui.selectable_label(editor.timeline.auto_key, auto_key_label)
            .on_hover_text(auto_key_tip)
            .clicked()
        {
            editor.timeline.auto_key = !editor.timeline.auto_key;
        }

        // Onion skin toggle
        let onion_tip = if editor.timeline.onion_skin_enabled {
            "Onion Skin: ON"
        } else {
            "Onion Skin: OFF"
        };
        if ui.selectable_label(editor.timeline.onion_skin_enabled, "👻")
            .on_hover_text(onion_tip)
            .clicked()
        {
            editor.timeline.onion_skin_enabled = !editor.timeline.onion_skin_enabled;
        }

        // Onion skin settings gear
        if editor.timeline.onion_skin_enabled
            && ui.small_button("⚙").on_hover_text("Onion Skin Settings").clicked()
        {
            editor.timeline.onion_skin_settings_open = !editor.timeline.onion_skin_settings_open;
        }

        // Animation templates dropdown
        egui::ComboBox::from_id_salt("anim_template")
            .selected_text("Template")
            .width(80.0)
            .show_ui(ui, |ui| {
                for template in crate::model::animation::ANIMATION_TEMPLATES {
                    if ui.selectable_label(false, template.name).clicked() {
                        actions.push(AppAction::ApplyAnimationTemplate {
                            sequence_id: seq_id.clone(),
                            template_name: template.name.to_string(),
                        });
                    }
                }
            });

        ui.separator();

        // Add event marker at playhead
        if ui.small_button("🚩").on_hover_text("Add event marker at playhead").clicked() {
            actions.push(AppAction::AddEventMarker {
                sequence_id: seq_id.clone(),
                time_secs: editor.timeline.playhead_time,
                name: "Event".into(),
            });
        }
    });

    ui.separator();

    // ── Row 3: Scrubber ───────────────────────────────────────────────────────
    let (scrubber_rect, scrubber_resp) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), SCRUBBER_HEIGHT),
        Sense::click_and_drag(),
    );

    let painter = ui.painter_at(scrubber_rect);
    let track_rect = Rect::from_min_max(
        Pos2::new(scrubber_rect.left() + SCRUBBER_MARGIN, scrubber_rect.top()),
        scrubber_rect.max,
    );

    // Background
    painter.rect_filled(scrubber_rect, 4.0, Color32::from_gray(22));

    // Draw tick marks along the time axis
    let label_color = Color32::from_gray(140);
    let tick_color = Color32::from_gray(60);
    let axis_y = track_rect.top() + SCRUBBER_HEIGHT * 0.3;

    // Compute tick interval: aim for ~50px between ticks
    let track_width = track_rect.width().max(1.0);
    let seconds_per_pixel = duration / track_width;
    let target_tick_gap = 50.0; // pixels
    let raw_interval = seconds_per_pixel * target_tick_gap;
    let tick_interval = nice_interval(raw_interval);

    let mut t = 0.0_f32;
    while t <= duration + 1e-4 {
        let x = time_to_x(t, duration, track_rect);
        painter.line_segment(
            [Pos2::new(x, axis_y - 4.0), Pos2::new(x, axis_y + 4.0)],
            Stroke::new(1.0, tick_color),
        );
        // Label every other tick (or all if sparse)
        painter.text(
            Pos2::new(x, axis_y - 8.0),
            egui::Align2::CENTER_BOTTOM,
            format!("{:.2}s", t),
            egui::FontId::proportional(9.0),
            label_color,
        );
        t += tick_interval;
    }

    // Horizontal time axis line
    painter.line_segment(
        [Pos2::new(track_rect.left(), axis_y), Pos2::new(track_rect.right(), axis_y)],
        Stroke::new(1.0, Color32::from_gray(50)),
    );

    // ── Event markers (flags above the axis) ───────────────────────────────
    let marker_flag_color = Color32::from_rgb(100, 160, 255);
    let markers: Vec<(String, f32, String)> = sprite.animations[seq_idx]
        .event_markers
        .iter()
        .map(|m| (m.id.clone(), m.time_secs, m.name.clone()))
        .collect();

    let mut marker_drag_active = editor.timeline.dragging_event_marker_id.is_some();

    // Start event marker drag
    if !marker_drag_active && scrubber_resp.drag_started()
        && let Some(press_pos) = scrubber_resp.interact_pointer_pos()
    {
        for (m_id, m_time, _) in &markers {
            let x = time_to_x(*m_time, duration, track_rect);
            let hit_rect = Rect::from_center_size(
                Pos2::new(x, axis_y - 6.0),
                Vec2::new(14.0, 14.0),
            );
            if hit_rect.contains(press_pos) {
                editor.timeline.dragging_event_marker_id = Some(m_id.clone());
                marker_drag_active = true;
                break;
            }
        }
    }

    // Update event marker drag
    if marker_drag_active && scrubber_resp.dragged() {
        // Visual only — we store dragging_event_marker_id and commit on release
    }

    // End event marker drag
    if marker_drag_active && scrubber_resp.drag_stopped()
        && let Some(m_id) = editor.timeline.dragging_event_marker_id.take()
        && let Some(pointer_pos) = scrubber_resp.interact_pointer_pos()
    {
        let new_time = x_to_time(pointer_pos.x, duration, track_rect);
        actions.push(AppAction::MoveEventMarker {
            sequence_id: seq_id.clone(),
            marker_id: m_id,
            time_secs: new_time,
        });
    }

    for (m_id, m_time, m_name) in &markers {
        let display_time = if editor.timeline.dragging_event_marker_id.as_deref() == Some(m_id) {
            // While dragging, show at pointer position
            scrubber_resp
                .interact_pointer_pos()
                .map(|p| x_to_time(p.x, duration, track_rect))
                .unwrap_or(*m_time)
        } else {
            *m_time
        };
        let x = time_to_x(display_time, duration, track_rect);

        // Flag triangle
        let flag_top = Pos2::new(x, axis_y - 14.0);
        let tri = [
            flag_top,
            Pos2::new(x + 8.0, flag_top.y + 5.0),
            Pos2::new(x, flag_top.y + 10.0),
        ];
        painter.add(Shape::convex_polygon(tri.to_vec(), marker_flag_color, Stroke::NONE));
        // Vertical line down to axis
        painter.line_segment(
            [Pos2::new(x, flag_top.y + 10.0), Pos2::new(x, axis_y)],
            Stroke::new(1.0, marker_flag_color),
        );

        // Inline rename or hover tooltip
        if editor.timeline.renaming_event_marker_id.as_deref() == Some(m_id) {
            // Handled after scrubber section via egui::Area
        } else {
            // Name label near flag
            painter.text(
                Pos2::new(x + 10.0, axis_y - 10.0),
                egui::Align2::LEFT_CENTER,
                m_name,
                egui::FontId::proportional(9.0),
                marker_flag_color,
            );
        }

        // Right-click context menu on marker
        if scrubber_resp.secondary_clicked()
            && let Some(click_pos) = scrubber_resp.interact_pointer_pos()
        {
            let hit_rect = Rect::from_center_size(
                Pos2::new(x, axis_y - 6.0),
                Vec2::new(14.0, 14.0),
            );
            if hit_rect.contains(click_pos) {
                // We'll use the renaming_event_marker_id for a small popup
                editor.timeline.renaming_event_marker_id = Some(m_id.clone());
            }
        }
    }

    // Event marker rename/delete popup
    if let Some(ref rename_m_id) = editor.timeline.renaming_event_marker_id.clone() {
        if let Some(marker) = sprite.animations[seq_idx]
            .event_markers
            .iter()
            .find(|m| m.id == *rename_m_id)
        {
            let mx = time_to_x(marker.time_secs, duration, track_rect);
            let popup_pos = Pos2::new(mx, axis_y - 30.0);
            let mut close_popup = false;

            let area_resp = egui::Area::new(egui::Id::new("marker_popup"))
                .fixed_pos(popup_pos)
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        // Rename field
                        let mut name = marker.name.clone();
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut name)
                                .desired_width(100.0)
                                .font(egui::TextStyle::Small),
                        );
                        if resp.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            let trimmed = name.trim().to_string();
                            if !trimmed.is_empty() && trimmed != marker.name {
                                actions.push(AppAction::RenameEventMarker {
                                    sequence_id: seq_id.clone(),
                                    marker_id: rename_m_id.clone(),
                                    name: trimmed,
                                });
                            }
                            close_popup = true;
                        }

                        if ui.button("Delete").clicked() {
                            actions.push(AppAction::DeleteEventMarker {
                                sequence_id: seq_id.clone(),
                                marker_id: rename_m_id.clone(),
                            });
                            close_popup = true;
                        }
                    });
                });

            if close_popup || area_resp.response.clicked_elsewhere() {
                editor.timeline.renaming_event_marker_id = None;
            }
        } else {
            editor.timeline.renaming_event_marker_id = None;
        }
    }

    // Draw keyframe diamonds
    let kf_y = axis_y + SCRUBBER_HEIGHT * 0.25;
    let keyframes: Vec<(String, f32)> = sprite.animations[seq_idx].pose_keyframes.iter()
        .map(|kf| (kf.id.clone(), kf.time_secs))
        .collect();

    // Track whether a keyframe drag is consuming the scrubber interaction
    let mut kf_drag_active = editor.timeline.dragging_keyframe_id.is_some() || marker_drag_active;

    // Start keyframe drag: check if drag begins on a diamond
    if scrubber_resp.drag_started()
        && let Some(press_pos) = scrubber_resp.interact_pointer_pos()
    {
        for (kf_id, kf_time) in &keyframes {
            let x = time_to_x(*kf_time, duration, track_rect);
            let center = Pos2::new(x, kf_y);
            let kf_hit_rect = Rect::from_center_size(center, Vec2::splat(DIAMOND_HALF * 2.5));
            if kf_hit_rect.contains(press_pos) {
                editor.timeline.dragging_keyframe_id = Some(kf_id.clone());
                editor.timeline.dragging_keyframe_preview_time = Some(*kf_time);
                editor.timeline.selected_keyframe_id = Some(kf_id.clone());
                kf_drag_active = true;
                break;
            }
        }
    }

    // Update keyframe drag preview
    if kf_drag_active && scrubber_resp.dragged()
        && let Some(pointer_pos) = scrubber_resp.interact_pointer_pos()
    {
        let new_time = x_to_time(pointer_pos.x, duration, track_rect);
        editor.timeline.dragging_keyframe_preview_time = Some(new_time);
    }

    // End keyframe drag: commit the move
    if kf_drag_active && scrubber_resp.drag_stopped()
        && let (Some(kf_id), Some(new_time)) = (
            editor.timeline.dragging_keyframe_id.take(),
            editor.timeline.dragging_keyframe_preview_time.take(),
        )
    {
        actions.push(AppAction::MoveKeyframe {
            sequence_id: seq_id.clone(),
            keyframe_id: kf_id,
            new_time,
        });
    }

    for (kf_id, kf_time) in &keyframes {
        // Use preview time if this keyframe is being dragged
        let display_time = if editor.timeline.dragging_keyframe_id.as_deref() == Some(kf_id) {
            editor.timeline.dragging_keyframe_preview_time.unwrap_or(*kf_time)
        } else {
            *kf_time
        };
        let x = time_to_x(display_time, duration, track_rect);
        let center = Pos2::new(x, kf_y);
        let is_selected = editor.timeline.selected_keyframe_id.as_deref() == Some(kf_id);
        let is_on_playhead = (display_time - editor.timeline.playhead_time).abs() < 0.001;
        let is_dragging = editor.timeline.dragging_keyframe_id.as_deref() == Some(kf_id);

        let fill_color = if is_dragging {
            Color32::from_rgb(255, 200, 60)
        } else if is_selected || is_on_playhead {
            Color32::from_rgb(80, 200, 100)
        } else {
            Color32::TRANSPARENT
        };
        let outline_color = if is_dragging {
            Color32::from_rgb(255, 200, 60)
        } else if is_selected || is_on_playhead {
            Color32::from_rgb(80, 200, 100)
        } else {
            Color32::from_gray(180)
        };

        // Diamond = rotated square
        let diamond_points = diamond(center, DIAMOND_HALF);
        painter.add(Shape::convex_polygon(diamond_points.to_vec(), fill_color, Stroke::new(1.5, outline_color)));

        // Click to select keyframe (only when not dragging)
        if !kf_drag_active {
            let kf_hit_rect = Rect::from_center_size(center, Vec2::splat(DIAMOND_HALF * 2.5));
            if scrubber_resp.clicked()
                && let Some(click_pos) = scrubber_resp.interact_pointer_pos()
                && kf_hit_rect.contains(click_pos)
            {
                editor.timeline.selected_keyframe_id = Some(kf_id.clone());
                actions.push(AppAction::SetPlayheadTime { time_secs: *kf_time });
            }
        }

        // Right-click to open context menu
        if scrubber_resp.secondary_clicked()
            && let Some(click_pos) = scrubber_resp.interact_pointer_pos()
        {
            let kf_hit_rect = Rect::from_center_size(center, Vec2::splat(DIAMOND_HALF * 2.5));
            if kf_hit_rect.contains(click_pos) {
                editor.timeline.context_menu_keyframe_id = Some(kf_id.clone());
                editor.timeline.context_menu_screen_pos = Some(click_pos);
            }
        }
    }

    // Handle scrubber drag / click to set playhead (only when not dragging a keyframe)
    if !kf_drag_active
        && (scrubber_resp.dragged() || scrubber_resp.clicked())
        && let Some(pointer_pos) = scrubber_resp.interact_pointer_pos()
        && pointer_pos.x >= track_rect.left()
    {
        let new_time = x_to_time(pointer_pos.x, duration, track_rect);
        actions.push(AppAction::SetPlayheadTime { time_secs: new_time });
    }

    // Playhead line
    let ph_x = time_to_x(editor.timeline.playhead_time, duration, track_rect);
    let playhead_color = Color32::from_rgb(220, 80, 60);
    // Triangle handle at top
    let tri_top = Pos2::new(ph_x, scrubber_rect.top() + 2.0);
    let tri = [
        tri_top,
        Pos2::new(ph_x - 5.0, tri_top.y + 8.0),
        Pos2::new(ph_x + 5.0, tri_top.y + 8.0),
    ];
    painter.add(Shape::convex_polygon(tri.to_vec(), playhead_color, Stroke::NONE));
    painter.line_segment(
        [Pos2::new(ph_x, tri_top.y + 8.0), Pos2::new(ph_x, scrubber_rect.bottom())],
        Stroke::new(1.5, playhead_color),
    );

    // Time label in the left margin
    painter.text(
        Pos2::new(scrubber_rect.left() + SCRUBBER_MARGIN * 0.5, axis_y),
        egui::Align2::CENTER_CENTER,
        format!("{:.1}", editor.timeline.playhead_time),
        egui::FontId::proportional(10.0),
        Color32::from_gray(180),
    );

    // ── Keyframe context menu popup ──────────────────────────────────────────
    if let Some(ctx_kf_id) = editor.timeline.context_menu_keyframe_id.clone() {
        let popup_pos = editor.timeline.context_menu_screen_pos.unwrap_or_default();
        let mut close_menu = false;

        let area_resp = egui::Area::new(egui::Id::new("kf_context_menu"))
            .fixed_pos(popup_pos)
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    // Copy Pose
                    if ui.button("Copy Pose").clicked() {
                        if let Some(kf) = sprite.animations[seq_idx].pose_keyframes.iter()
                            .find(|kf| kf.id == ctx_kf_id)
                        {
                            editor.timeline.pose_clipboard = Some(PoseClipboard {
                                element_poses: kf.element_poses.clone(),
                            });
                        }
                        close_menu = true;
                    }

                    // Paste Pose (only if clipboard has data)
                    let has_clipboard = editor.timeline.pose_clipboard.is_some();
                    if ui.add_enabled(has_clipboard, egui::Button::new("Paste Pose")).clicked() {
                        if let Some(ref clipboard) = editor.timeline.pose_clipboard {
                            actions.push(AppAction::PastePose {
                                sequence_id: seq_id.clone(),
                                time_secs: editor.timeline.playhead_time,
                                element_poses: clipboard.element_poses.clone(),
                            });
                        }
                        close_menu = true;
                    }

                    // Mirror Pose
                    if ui.button("Mirror Pose").clicked() {
                        actions.push(AppAction::MirrorPose {
                            sequence_id: seq_id.clone(),
                            keyframe_id: ctx_kf_id.clone(),
                            time_secs: editor.timeline.playhead_time,
                        });
                        close_menu = true;
                    }

                    ui.separator();

                    // Edit Easing
                    if ui.button("Edit Easing...").clicked() {
                        editor.timeline.easing_popup = Some(EasingPopupState {
                            keyframe_id: ctx_kf_id.clone(),
                            sequence_id: seq_id.clone(),
                            screen_pos: popup_pos,
                        });
                        close_menu = true;
                    }

                    ui.separator();

                    // Delete Keyframe
                    if ui.button("Delete Keyframe").clicked() {
                        actions.push(AppAction::DeleteKeyframe {
                            sequence_id: seq_id.clone(),
                            keyframe_id: ctx_kf_id.clone(),
                        });
                        close_menu = true;
                    }
                });
            });

        if close_menu || area_resp.response.clicked_elsewhere() {
            editor.timeline.context_menu_keyframe_id = None;
            editor.timeline.context_menu_screen_pos = None;
        }
    }

    // ── Easing curve editor popup ───────────────────────────────────────────
    if let Some(ref popup) = editor.timeline.easing_popup.clone() {
        let current_easing = sprite.animations[seq_idx]
            .pose_keyframes
            .iter()
            .find(|kf| kf.id == popup.keyframe_id)
            .map(|kf| kf.easing.clone())
            .unwrap_or_default();

        let mut close_popup = false;
        let mut new_easing: Option<crate::model::animation::EasingCurve> = None;

        egui::Window::new("Easing Curve")
            .fixed_pos(popup.screen_pos + egui::Vec2::new(0.0, 20.0))
            .fixed_size(egui::Vec2::new(200.0, 200.0))
            .collapsible(false)
            .title_bar(true)
            .show(ui.ctx(), |ui| {
                // Preset buttons
                ui.horizontal(|ui| {
                    for preset in &["linear", "ease-in", "ease-out", "ease-in-out"] {
                        let is_active = current_easing.preset == *preset;
                        if ui.selectable_label(is_active, *preset).clicked() {
                            new_easing = Some(crate::model::animation::EasingCurve::from_preset(preset));
                        }
                    }
                });

                ui.separator();

                // Visual bezier curve preview
                let cp = current_easing.control_points;
                let (curve_resp, curve_painter) = ui.allocate_painter(
                    egui::Vec2::new(180.0, 120.0),
                    egui::Sense::click_and_drag(),
                );
                let rect = curve_resp.rect;

                // Background
                curve_painter.rect_filled(rect, 2.0, Color32::from_gray(30));

                // Grid + diagonal reference (linear)
                let grid_color = Color32::from_gray(50);
                for i in 1..4 {
                    let f = i as f32 / 4.0;
                    let x = rect.left() + f * rect.width();
                    let y = rect.bottom() - f * rect.height();
                    curve_painter.line_segment(
                        [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                        Stroke::new(0.5, grid_color),
                    );
                    curve_painter.line_segment(
                        [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                        Stroke::new(0.5, grid_color),
                    );
                }
                // Diagonal (linear reference)
                curve_painter.line_segment(
                    [
                        Pos2::new(rect.left(), rect.bottom()),
                        Pos2::new(rect.right(), rect.top()),
                    ],
                    Stroke::new(0.5, Color32::from_gray(70)),
                );

                // Draw the cubic bezier curve by sampling
                let curve_color = Color32::from_rgb(100, 200, 255);
                let steps = 40;
                let mut points = Vec::with_capacity(steps + 1);
                for i in 0..=steps {
                    let t = i as f32 / steps as f32;
                    // Evaluate cubic bezier (CSS-style: P0=(0,0) P1=(x1,y1) P2=(x2,y2) P3=(1,1))
                    let x = 3.0 * (1.0 - t).powi(2) * t * cp[0]
                        + 3.0 * (1.0 - t) * t.powi(2) * cp[2]
                        + t.powi(3);
                    let y = 3.0 * (1.0 - t).powi(2) * t * cp[1]
                        + 3.0 * (1.0 - t) * t.powi(2) * cp[3]
                        + t.powi(3);
                    let screen_x = rect.left() + x * rect.width();
                    let screen_y = rect.bottom() - y * rect.height();
                    points.push(Pos2::new(screen_x, screen_y));
                }
                for pair in points.windows(2) {
                    curve_painter.line_segment([pair[0], pair[1]], Stroke::new(2.0, curve_color));
                }

                // Control point handles
                let p1_screen = Pos2::new(
                    rect.left() + cp[0] * rect.width(),
                    rect.bottom() - cp[1] * rect.height(),
                );
                let p2_screen = Pos2::new(
                    rect.left() + cp[2] * rect.width(),
                    rect.bottom() - cp[3] * rect.height(),
                );

                // Lines from corners to control points
                curve_painter.line_segment(
                    [Pos2::new(rect.left(), rect.bottom()), p1_screen],
                    Stroke::new(1.0, Color32::from_gray(100)),
                );
                curve_painter.line_segment(
                    [Pos2::new(rect.right(), rect.top()), p2_screen],
                    Stroke::new(1.0, Color32::from_gray(100)),
                );

                // Handle circles
                let handle_radius = 5.0;
                curve_painter.circle_filled(p1_screen, handle_radius, Color32::from_rgb(255, 120, 80));
                curve_painter.circle_filled(p2_screen, handle_radius, Color32::from_rgb(80, 200, 255));

                // Handle dragging
                if curve_resp.dragged()
                    && let Some(pointer) = curve_resp.interact_pointer_pos()
                {
                    let norm_x = ((pointer.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                    let norm_y = ((rect.bottom() - pointer.y) / rect.height()).clamp(-0.5, 1.5);

                    let d1 = (pointer - p1_screen).length();
                    let d2 = (pointer - p2_screen).length();

                    let mut new_cp = cp;
                    if d1 <= d2 {
                        new_cp[0] = norm_x;
                        new_cp[1] = norm_y;
                    } else {
                        new_cp[2] = norm_x;
                        new_cp[3] = norm_y;
                    }
                    new_easing = Some(crate::model::animation::EasingCurve {
                        preset: "custom".to_string(),
                        control_points: new_cp,
                    });
                }

                // Control point values
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "P1({:.2},{:.2})  P2({:.2},{:.2})",
                        cp[0], cp[1], cp[2], cp[3]
                    ));
                });

                // Close button
                if ui.button("Close").clicked() {
                    close_popup = true;
                }
            });

        if let Some(easing) = new_easing {
            actions.push(AppAction::SetEasingCurve {
                sequence_id: popup.sequence_id.clone(),
                keyframe_id: popup.keyframe_id.clone(),
                easing,
            });
        }

        if close_popup {
            editor.timeline.easing_popup = None;
        }
    }

    // ── Onion skin settings popup ────────────────────────────────────────────
    if editor.timeline.onion_skin_settings_open {
        use crate::state::editor::OnionSkinMode;

        let mut open = true;
        egui::Window::new("Onion Skin Settings")
            .collapsible(false)
            .resizable(false)
            .fixed_size(egui::Vec2::new(220.0, 0.0))
            .open(&mut open)
            .show(ui.ctx(), |ui| {
                // Mode selector
                ui.label("Mode");
                ui.horizontal(|ui| {
                    if ui.selectable_label(
                        editor.timeline.onion_skin_mode == OnionSkinMode::Keyframe,
                        "Keyframe",
                    ).clicked() {
                        editor.timeline.onion_skin_mode = OnionSkinMode::Keyframe;
                    }
                    if ui.selectable_label(
                        editor.timeline.onion_skin_mode == OnionSkinMode::Frame,
                        "Frame",
                    ).clicked() {
                        editor.timeline.onion_skin_mode = OnionSkinMode::Frame;
                    }
                    if ui.selectable_label(
                        editor.timeline.onion_skin_mode == OnionSkinMode::Both,
                        "Both",
                    ).clicked() {
                        editor.timeline.onion_skin_mode = OnionSkinMode::Both;
                    }
                });

                ui.separator();

                // Ghost counts
                let mut prev = editor.timeline.onion_skin_prev_count as i32;
                let mut next = editor.timeline.onion_skin_next_count as i32;
                ui.add(egui::Slider::new(&mut prev, 0..=5).text("Prev frames"));
                ui.add(egui::Slider::new(&mut next, 0..=5).text("Next frames"));
                editor.timeline.onion_skin_prev_count = prev as u8;
                editor.timeline.onion_skin_next_count = next as u8;

                ui.separator();

                // Colors
                ui.horizontal(|ui| {
                    ui.label("Prev color");
                    let mut c = editor.timeline.onion_skin_prev_color;
                    let mut color = Color32::from_rgb(c[0], c[1], c[2]);
                    egui::color_picker::color_edit_button_srgba(
                        ui, &mut color, egui::color_picker::Alpha::Opaque,
                    );
                    c = [color.r(), color.g(), color.b()];
                    editor.timeline.onion_skin_prev_color = c;
                });

                ui.horizontal(|ui| {
                    ui.label("Next color");
                    let mut c = editor.timeline.onion_skin_next_color;
                    let mut color = Color32::from_rgb(c[0], c[1], c[2]);
                    egui::color_picker::color_edit_button_srgba(
                        ui, &mut color, egui::color_picker::Alpha::Opaque,
                    );
                    c = [color.r(), color.g(), color.b()];
                    editor.timeline.onion_skin_next_color = c;
                });

                ui.separator();

                // Opacity
                ui.add(
                    egui::Slider::new(&mut editor.timeline.onion_skin_opacity, 0.0..=1.0)
                        .text("Opacity"),
                );
            });

        if !open {
            editor.timeline.onion_skin_settings_open = false;
        }
    }

    actions
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn time_to_x(time: f32, duration: f32, track_rect: Rect) -> f32 {
    let frac = if duration > 0.0 { (time / duration).clamp(0.0, 1.0) } else { 0.0 };
    track_rect.left() + frac * track_rect.width()
}

fn x_to_time(x: f32, duration: f32, track_rect: Rect) -> f32 {
    let frac = ((x - track_rect.left()) / track_rect.width().max(1.0)).clamp(0.0, 1.0);
    (frac * duration).clamp(0.0, duration)
}

/// Diamond shape: 4 points around a center.
fn diamond(center: Pos2, half: f32) -> [Pos2; 4] {
    [
        Pos2::new(center.x, center.y - half), // top
        Pos2::new(center.x + half, center.y), // right
        Pos2::new(center.x, center.y + half), // bottom
        Pos2::new(center.x - half, center.y), // left
    ]
}

/// Round a tick interval to a "nice" number (0.1, 0.25, 0.5, 1.0, 2.0, etc.).
fn nice_interval(raw: f32) -> f32 {
    let candidates = [0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0];
    for &c in &candidates {
        if c >= raw { return c; }
    }
    10.0
}

/// Toggle play/pause directly (editor state mutation, no action dispatch).
fn toggle_playback(playback: &mut PlaybackState, timeline: &mut TimelineState, duration: f32) {
    if playback.playing {
        playback.playing = false;
        playback.last_frame_time = None;
    } else {
        // If at the end, rewind first
        if timeline.playhead_time >= duration - 0.001 {
            timeline.playhead_time = 0.0;
        }
        playback.playing = true;
        playback.last_frame_time = None; // will be set on first tick
    }
}
