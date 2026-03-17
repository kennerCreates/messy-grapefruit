use egui::{Color32, Pos2, Rect, Sense, Shape, Stroke, Vec2};

use crate::action::AppAction;
use crate::model::project::Project;
use crate::model::sprite::Sprite;
use crate::state::editor::{EditorState, PlaybackState, TimelineState};

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
        if let Some(kf_id) = editor.timeline.selected_keyframe_id.clone() {
            if ui.small_button("✕").on_hover_text("Delete keyframe").clicked() {
                actions.push(AppAction::DeleteKeyframe {
                    sequence_id: seq_id.clone(),
                    keyframe_id: kf_id,
                });
            }
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

    // Draw keyframe diamonds
    let kf_y = axis_y + SCRUBBER_HEIGHT * 0.25;
    let keyframes: Vec<(String, f32)> = sprite.animations[seq_idx].pose_keyframes.iter()
        .map(|kf| (kf.id.clone(), kf.time_secs))
        .collect();

    for (kf_id, kf_time) in &keyframes {
        let x = time_to_x(*kf_time, duration, track_rect);
        let center = Pos2::new(x, kf_y);
        let is_selected = editor.timeline.selected_keyframe_id.as_deref() == Some(kf_id);
        let is_on_playhead = (kf_time - editor.timeline.playhead_time).abs() < 0.001;

        let fill_color = if is_selected || is_on_playhead {
            Color32::from_rgb(80, 200, 100)
        } else {
            Color32::TRANSPARENT
        };
        let outline_color = if is_selected || is_on_playhead {
            Color32::from_rgb(80, 200, 100)
        } else {
            Color32::from_gray(180)
        };

        // Diamond = rotated square
        let diamond_points = diamond(center, DIAMOND_HALF);
        painter.add(Shape::convex_polygon(diamond_points.to_vec(), fill_color, Stroke::new(1.5, outline_color)));

        // Click to select keyframe
        let kf_hit_rect = Rect::from_center_size(center, Vec2::splat(DIAMOND_HALF * 2.5));
        if scrubber_resp.clicked()
            && let Some(click_pos) = scrubber_resp.interact_pointer_pos()
            && kf_hit_rect.contains(click_pos)
        {
            editor.timeline.selected_keyframe_id = Some(kf_id.clone());
            actions.push(AppAction::SetPlayheadTime { time_secs: *kf_time });
        }
    }

    // Handle scrubber drag / click to set playhead
    if scrubber_resp.dragged() || scrubber_resp.clicked() {
        if let Some(pointer_pos) = scrubber_resp.interact_pointer_pos() {
            if pointer_pos.x >= track_rect.left() {
                let new_time = x_to_time(pointer_pos.x, duration, track_rect);
                actions.push(AppAction::SetPlayheadTime { time_secs: new_time });
            }
        }
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
