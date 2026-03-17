use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};

use crate::action::AppAction;
use crate::engine::hit_test;
use crate::model::project::Theme;
use crate::model::sprite::{GradientFill, Sprite};
use crate::model::vec2::Vec2;
use crate::state::editor::{EditorState, FillMode, GradientDragKind, GradientDragState};
use crate::theme;

use super::canvas::HIT_TEST_THRESHOLD;

/// Preset snap angles: cardinal, 45° diagonals, and isometric perpendiculars.
const SNAP_ANGLES: &[f32] = &[
    0.0,                          // Horizontal (right)
    FRAC_PI_4,                    // 45°
    FRAC_PI_2,                    // Vertical (down)
    3.0 * FRAC_PI_4,             // 135°
    PI,                           // 180° (left)
    -3.0 * FRAC_PI_4,           // -135°
    -FRAC_PI_2,                  // -90° (up)
    -FRAC_PI_4,                  // -45°
];

/// Snap threshold in radians (~6°).
const SNAP_THRESHOLD: f32 = 0.105;

/// Snap an angle to the nearest preset (including iso angles computed at runtime).
fn snap_angle(angle: f32) -> f32 {
    let iso_asc = 2.0_f32.atan();    // ~63.43°
    let iso_desc = -(2.0_f32.atan()); // ~-63.43°

    let mut best = angle;
    let mut best_diff = f32::MAX;

    for &preset in SNAP_ANGLES.iter().chain(&[iso_asc, iso_desc]) {
        // Normalize angle difference to [-PI, PI]
        let diff = ((angle - preset) + PI).rem_euclid(2.0 * PI) - PI;
        if diff.abs() < best_diff {
            best_diff = diff.abs();
            best = preset;
        }
    }

    if best_diff < SNAP_THRESHOLD { best } else { angle }
}

/// Handle fill tool input: click for flat fill, drag for gradient placement.
#[allow(clippy::too_many_arguments)]
pub(super) fn handle_fill_tool(
    response: &egui::Response,
    painter: &egui::Painter,
    editor: &mut EditorState,
    sprite: &Sprite,
    canvas_rect: egui::Rect,
    theme_mode: Theme,
    actions: &mut Vec<AppAction>,
) {
    let canvas_center = canvas_rect.center();
    let threshold = HIT_TEST_THRESHOLD / editor.viewport.zoom;

    // Hover: update hover element for highlight feedback
    if let Some(hover_pos) = response.hover_pos() {
        let world = editor.viewport.screen_to_world(hover_pos, canvas_center);
        let hit = hit_test::hit_test_fill(
            world,
            sprite,
            threshold,
            editor.layer.solo_layer_id.as_deref(),
        );
        editor.hover_element_id = hit.map(|(id, _)| id);
    } else {
        editor.hover_element_id = None;
    }

    let is_gradient_mode = matches!(
        editor.brush.fill_mode,
        FillMode::LinearGradient | FillMode::RadialGradient
    );

    // === Gradient drag interaction ===
    if is_gradient_mode {
        // Drag start: begin defining gradient line on a closed element
        if response.drag_started_by(egui::PointerButton::Primary)
            && let Some(pos) = response.interact_pointer_pos()
            && canvas_rect.contains(pos)
        {
            let world = editor.viewport.screen_to_world(pos, canvas_center);
            let hit = hit_test::hit_test_fill(
                world, sprite, threshold,
                editor.layer.solo_layer_id.as_deref(),
            );
            if let Some((element_id, true)) = hit {
                let alt_held = response.ctx.input(|i| i.modifiers.alt);
                editor.gradient_drag = Some(GradientDragState {
                    kind: GradientDragKind::DefineLine {
                        element_id,
                        start_world: world,
                    },
                    snap_active: !alt_held,
                });
            }
        }

        // Drag update: render preview line
        if response.dragged_by(egui::PointerButton::Primary)
            && let Some(pos) = response.interact_pointer_pos()
        {
            let alt_held = response.ctx.input(|i| i.modifiers.alt);
            if let Some(ref mut drag) = editor.gradient_drag {
                drag.snap_active = !alt_held;
                let GradientDragKind::DefineLine { ref start_world, .. } = drag.kind;

                let end_world = editor.viewport.screen_to_world(pos, canvas_center);
                let raw_angle = (end_world.y - start_world.y)
                    .atan2(end_world.x - start_world.x);
                let display_angle = if drag.snap_active {
                    snap_angle(raw_angle)
                } else {
                    raw_angle
                };

                let dist = start_world.distance(end_world);
                let snapped_end = Vec2::new(
                    start_world.x + dist * display_angle.cos(),
                    start_world.y + dist * display_angle.sin(),
                );

                let screen_start = editor.viewport.world_to_screen(*start_world, canvas_center);
                let screen_end = editor.viewport.world_to_screen(snapped_end, canvas_center);
                let line_color = theme::theme_colors(theme_mode).selected;

                draw_dashed_line(painter, screen_start, screen_end, line_color, 2.0, 6.0, 4.0);
                painter.circle_filled(screen_start, 4.0, line_color);
                painter.circle_filled(screen_end, 4.0, line_color);
            }
        }

        // Drag end: apply gradient
        if response.drag_stopped_by(egui::PointerButton::Primary)
            && let Some(drag) = editor.gradient_drag.take()
        {
            let GradientDragKind::DefineLine { element_id, start_world } = drag.kind;
            if let Some(pos) = response.interact_pointer_pos() {
                let end_world = editor.viewport.screen_to_world(pos, canvas_center);
                let raw_angle = (end_world.y - start_world.y)
                    .atan2(end_world.x - start_world.x);
                let final_angle = if drag.snap_active {
                    snap_angle(raw_angle)
                } else {
                    raw_angle
                };

                let gradient_fill = match editor.brush.fill_mode {
                    FillMode::LinearGradient => {
                        let mut gf = GradientFill::linear(
                            editor.brush.gradient_stops.clone(),
                            final_angle,
                        );
                        gf.spread = editor.brush.gradient_spread;
                        gf.midpoints = editor.brush.gradient_midpoints.clone();
                        gf
                    }
                    FillMode::RadialGradient => {
                        let mut gf = GradientFill::radial(
                            editor.brush.gradient_stops.clone(),
                            editor.brush.radial_center,
                            editor.brush.radial_radius,
                        );
                        gf.spread = editor.brush.gradient_spread;
                        gf.midpoints = editor.brush.gradient_midpoints.clone();
                        gf.focal_offset = Some(editor.brush.radial_focal_offset);
                        gf
                    }
                    _ => return,
                };

                actions.push(AppAction::SetGradientFill {
                    element_id: element_id.clone(),
                    gradient_fill,
                });

                if editor.brush.hatch_apply_enabled
                    && let Some(ref pattern_id) = editor.selected_hatch_pattern_id
                {
                    actions.push(AppAction::SetHatchFill {
                        element_id,
                        hatch_fill_id: pattern_id.clone(),
                    });
                }
            }
        }

        // Click (no drag) still applies gradient with current brush angle
        if response.clicked()
            && let Some(click_pos) = response.interact_pointer_pos()
            && canvas_rect.contains(click_pos)
            && editor.gradient_drag.is_none()
        {
            let world = editor.viewport.screen_to_world(click_pos, canvas_center);
            let hit = hit_test::hit_test_fill(
                world, sprite, threshold,
                editor.layer.solo_layer_id.as_deref(),
            );
            if let Some((element_id, true)) = hit {
                let gradient_fill = match editor.brush.fill_mode {
                    FillMode::LinearGradient => {
                        let mut gf = GradientFill::linear(
                            editor.brush.gradient_stops.clone(),
                            editor.brush.gradient_angle,
                        );
                        gf.spread = editor.brush.gradient_spread;
                        gf.midpoints = editor.brush.gradient_midpoints.clone();
                        gf
                    }
                    FillMode::RadialGradient => {
                        let mut gf = GradientFill::radial(
                            editor.brush.gradient_stops.clone(),
                            editor.brush.radial_center,
                            editor.brush.radial_radius,
                        );
                        gf.spread = editor.brush.gradient_spread;
                        gf.midpoints = editor.brush.gradient_midpoints.clone();
                        gf.focal_offset = Some(editor.brush.radial_focal_offset);
                        gf
                    }
                    _ => return,
                };

                actions.push(AppAction::SetGradientFill {
                    element_id: element_id.clone(),
                    gradient_fill,
                });

                if editor.brush.hatch_apply_enabled
                    && let Some(ref pattern_id) = editor.selected_hatch_pattern_id
                {
                    actions.push(AppAction::SetHatchFill {
                        element_id,
                        hatch_fill_id: pattern_id.clone(),
                    });
                }
            }
        }

        return;
    }

    // === Flat fill (click only) ===
    if response.clicked()
        && let Some(click_pos) = response.interact_pointer_pos()
        && canvas_rect.contains(click_pos)
    {
        let world = editor.viewport.screen_to_world(click_pos, canvas_center);
        let hit = hit_test::hit_test_fill(
            world, sprite, threshold,
            editor.layer.solo_layer_id.as_deref(),
        );

        match hit {
            Some((element_id, true)) => {
                actions.push(AppAction::SetFillColor {
                    element_id: element_id.clone(),
                    fill_color_index: editor.brush.fill_color_index,
                });
                editor.track_recent_color(editor.brush.fill_color_index);

                if editor.brush.hatch_apply_enabled
                    && let Some(ref pattern_id) = editor.selected_hatch_pattern_id
                {
                    actions.push(AppAction::SetHatchFill {
                        element_id,
                        hatch_fill_id: pattern_id.clone(),
                    });
                }
            }
            Some((_, false)) => {}
            None => {
                actions.push(AppAction::SetBackgroundColor {
                    background_color_index: editor.brush.fill_color_index,
                });
                editor.track_recent_color(editor.brush.fill_color_index);
            }
        }
    }
}

/// Draw a dashed line between two screen-space points.
fn draw_dashed_line(
    painter: &egui::Painter,
    from: egui::Pos2,
    to: egui::Pos2,
    color: egui::Color32,
    width: f32,
    dash: f32,
    gap: f32,
) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 0.1 { return; }
    let ux = dx / len;
    let uy = dy / len;
    let mut d = 0.0;
    let stroke = egui::Stroke::new(width, color);
    while d < len {
        let seg_end = (d + dash).min(len);
        let p0 = egui::Pos2::new(from.x + ux * d, from.y + uy * d);
        let p1 = egui::Pos2::new(from.x + ux * seg_end, from.y + uy * seg_end);
        painter.line_segment([p0, p1], stroke);
        d = seg_end + gap;
    }
}
