use crate::engine::animation::{canvas_state, CanvasAnimState};
use crate::model::project::Project;
use crate::model::sprite::Sprite;
use crate::state::editor::EditorState;

use super::icons;

/// Available cube counts for canvas sizing.
const CUBE_COUNTS: &[u32] = &[1, 2, 3, 4, 5, 6, 7, 8, 12, 16, 20, 24, 28, 32];

/// Base grid size for canvas sizing — always 8px regardless of current grid setting.
const BASE_GS: u32 = 8;

/// Max stroke width — used as padding so edge strokes aren't clipped.
const MAX_STROKE: u32 = 8;

/// Iso-aligned canvas width options.
/// One cube width = 2·√3·BASE_GS, plus stroke padding.
fn iso_widths() -> Vec<u32> {
    let gs_f = BASE_GS as f32;
    let sqrt3 = 3.0_f32.sqrt();
    CUBE_COUNTS.iter().map(|&n| {
        (n as f32 * 2.0 * sqrt3 * gs_f).round() as u32 + MAX_STROKE
    }).collect()
}

/// Iso-aligned canvas height options.
/// One cube height = 4·BASE_GS (top peak to bottom vertex), plus stroke padding.
fn iso_heights() -> Vec<u32> {
    CUBE_COUNTS.iter().map(|&n| n * 4 * BASE_GS + MAX_STROKE).collect()
}

pub fn show_status_bar(ui: &mut egui::Ui, editor: &EditorState, sprite: &mut Sprite, project: &mut Project) {
    ui.horizontal(|ui| {
        // Canvas animation state dot (colored circle)
        let anim_state = canvas_state(
            editor.timeline.selected_sequence_id.as_ref()
                .and_then(|id| sprite.animations.iter().find(|s| &s.id == id)),
            editor.timeline.playhead_time,
        );
        let (dot_color, dot_tooltip) = match &anim_state {
            CanvasAnimState::Rest => (None, "Rest pose"),
            CanvasAnimState::OnKeyframe(_) => (Some(egui::Color32::from_rgb(80, 200, 100)), "On keyframe"),
            CanvasAnimState::Interpolated => (Some(egui::Color32::from_rgb(220, 140, 60)), "Interpolated"),
        };
        if let Some(color) = dot_color {
            let (rect, response) = ui.allocate_exact_size(egui::Vec2::splat(12.0), egui::Sense::hover());
            ui.painter().circle_filled(rect.center(), 5.0, color);
            response.on_hover_text(dot_tooltip);
            ui.separator();
        }

        // Flip indicator (always show icon; tint when active)
        if editor.viewport.flipped {
            let tint = crate::theme::theme_colors(project.editor_preferences.theme).icon_text;
            ui.add(icons::small_icon_tinted(icons::view_flip(), tint, ui));
            ui.separator();
        }

        // Symmetry axis indicator
        if editor.symmetry.active {
            let sym_icon = match editor.symmetry.axis {
                crate::state::editor::SymmetryAxis::Vertical => icons::symmetry_vertical(),
                crate::state::editor::SymmetryAxis::Horizontal => icons::symmetry_horizontal(),
                crate::state::editor::SymmetryAxis::Both => icons::symmetry_both(),
            };
            let tint = crate::theme::symmetry_axis_color(project.editor_preferences.theme);
            ui.add(icons::small_icon_tinted(sym_icon, tint, ui));
            ui.separator();
        }

        // Sprite metrics: icon then count, left-to-right
        ui.add(icons::small_icon(icons::metric_element(), ui));
        ui.label(format!("{}", sprite.element_count()));
        ui.separator();

        ui.add(icons::small_icon(icons::metric_vertex(), ui));
        ui.label(format!("{}", sprite.vertex_count()));
        ui.separator();

        ui.add(icons::small_icon(icons::metric_layer(), ui));
        ui.label(format!("{}", sprite.layer_count()));
        ui.separator();

        ui.add(icons::small_icon(icons::metric_animation(), ui));
        ui.label(format!("{}", sprite.animation_count()));

        // Canvas size + grid offset, right-aligned
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Canvas size dropdowns (iso-aligned, always based on 8px grid)
            let widths = iso_widths();
            let heights = iso_heights();

            let old_w = sprite.canvas_width;
            let old_h = sprite.canvas_height;

            ui.label("px");
            egui::ComboBox::from_id_salt("canvas_h")
                .width(60.0)
                .selected_text(format!("{}", sprite.canvas_height))
                .show_ui(ui, |ui| {
                    for (i, &s) in heights.iter().enumerate() {
                        let n = CUBE_COUNTS[i];
                        let label = format!("{s}px  {n} cube{}", if n > 1 { "s" } else { "" });
                        ui.selectable_value(&mut sprite.canvas_height, s, label);
                    }
                });
            ui.label("\u{00d7}");
            egui::ComboBox::from_id_salt("canvas_w")
                .width(60.0)
                .selected_text(format!("{}", sprite.canvas_width))
                .show_ui(ui, |ui| {
                    for (i, &s) in widths.iter().enumerate() {
                        let n = CUBE_COUNTS[i];
                        let label = format!("{s}px  {n} cube{}", if n > 1 { "s" } else { "" });
                        ui.selectable_value(&mut sprite.canvas_width, s, label);
                    }
                });

            // When canvas size changes, re-center the grid on the canvas
            if sprite.canvas_width != old_w || sprite.canvas_height != old_h {
                let new_center = (sprite.canvas_width as f32 / 2.0, sprite.canvas_height as f32 / 2.0);
                let old_offset = project.editor_preferences.grid_offset;
                let dx = new_center.0 - old_offset.0;
                let dy = new_center.1 - old_offset.1;
                project.editor_preferences.grid_offset = new_center;
                let d = crate::model::vec2::Vec2::new(dx, dy);
                for layer in &mut sprite.layers {
                    for elem in &mut layer.elements {
                        for v in &mut elem.vertices {
                            v.pos = v.pos + d;
                            if let Some(ref mut cp) = v.cp1 { *cp = *cp + d; }
                            if let Some(ref mut cp) = v.cp2 { *cp = *cp + d; }
                        }
                        elem.origin = elem.origin + d;
                    }
                }
            }

            ui.separator();

            // Grid offset nudge (steps of 2) — shifts grid + all artwork
            let mut delta = (0.0_f32, 0.0_f32);
            if ui.small_button("v").on_hover_text("Shift grid down").clicked() {
                delta.1 = 2.0;
            }
            if ui.small_button("^").on_hover_text("Shift grid up").clicked() {
                delta.1 = -2.0;
            }
            if ui.small_button(">").on_hover_text("Shift grid right").clicked() {
                delta.0 = 2.0;
            }
            if ui.small_button("<").on_hover_text("Shift grid left").clicked() {
                delta.0 = -2.0;
            }
            let offset = &project.editor_preferences.grid_offset;
            if (offset.0.abs() > 0.01 || offset.1.abs() > 0.01)
                && ui.small_button("X").on_hover_text("Reset grid offset").clicked()
            {
                delta = (-offset.0, -offset.1);
            }
            if delta.0.abs() > 0.001 || delta.1.abs() > 0.001 {
                project.editor_preferences.grid_offset.0 += delta.0;
                project.editor_preferences.grid_offset.1 += delta.1;
                let d = crate::model::vec2::Vec2::new(delta.0, delta.1);
                for layer in &mut sprite.layers {
                    for elem in &mut layer.elements {
                        for v in &mut elem.vertices {
                            v.pos = v.pos + d;
                            if let Some(ref mut cp) = v.cp1 {
                                *cp = *cp + d;
                            }
                            if let Some(ref mut cp) = v.cp2 {
                                *cp = *cp + d;
                            }
                        }
                        elem.origin = elem.origin + d;
                    }
                }
            }
        });
    });
}
