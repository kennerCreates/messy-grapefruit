use crate::action::AppAction;
use crate::model::project::Project;
use crate::model::sprite::{GradientAlignment, SpreadMethod};
use crate::state::editor::{EditorState, FillMode};
use crate::theme;

use super::gradient_bar;
use super::icons;
use super::sidebar_palette::{render_color_palette, render_color_swatch};

/// Render gradient editor controls (stops, direction, spread).
/// Called from `sidebar_fill::show_fill_tool_options` and the select tool fill section.
pub(super) fn render_gradient_controls(
    ui: &mut egui::Ui,
    editor: &mut EditorState,
    project: &mut Project,
    _actions: &mut Vec<AppAction>,
) {
    // Gradient bar widget
    gradient_bar::render_gradient_bar(ui, editor, &project.palette, project.editor_preferences.theme);

    ui.add_space(4.0);

    // Selected stop color picker
    let sel_idx = editor.brush.selected_stop_index.unwrap_or(0);
    let sel_idx = sel_idx.min(editor.brush.gradient_stops.len().saturating_sub(1));
    let current_color_idx = editor.brush.gradient_stops.get(sel_idx)
        .map(|s| s.color_index).unwrap_or(1);

    ui.horizontal(|ui| {
        ui.label(format!("Stop {}", sel_idx + 1));
        let color = project.palette.get_color(current_color_idx);
        render_color_swatch(ui, color, 16.0, project.editor_preferences.theme);
    });
    if let Some(new_idx) = render_color_palette(
        ui,
        &project.palette.colors,
        current_color_idx,
        project.editor_preferences.theme,
    ) {
        if let Some(stop) = editor.brush.gradient_stops.get_mut(sel_idx) {
            stop.color_index = new_idx;
        }
        editor.track_recent_color(new_idx);
    }

    // Stop location and navigation
    ui.add_space(2.0);
    theme::with_input_style(ui, project.editor_preferences.theme, |ui| {
        ui.horizontal(|ui| {
            let stop_count = editor.brush.gradient_stops.len();
            if ui.small_button("<").clicked() && sel_idx > 0 {
                editor.brush.selected_stop_index = Some(sel_idx - 1);
            }
            ui.label(format!("{}/{}", sel_idx + 1, stop_count));
            if ui.small_button(">").clicked() && sel_idx + 1 < stop_count {
                editor.brush.selected_stop_index = Some(sel_idx + 1);
            }

            if let Some(stop) = editor.brush.gradient_stops.get_mut(sel_idx) {
                let mut pct = stop.position * 100.0;
                ui.label("Loc");
                if ui.add(egui::DragValue::new(&mut pct)
                    .speed(1.0).range(0.0..=100.0).suffix("%").fixed_decimals(0))
                    .changed()
                {
                    stop.position = (pct / 100.0).clamp(0.0, 1.0);
                }
            }
        });

        // Add / Delete stop buttons
        ui.horizontal(|ui| {
            if ui.small_button("+ Stop").clicked() && editor.brush.gradient_stops.len() < 16 {
                let new_pos = if sel_idx + 1 < editor.brush.gradient_stops.len() {
                    (editor.brush.gradient_stops[sel_idx].position
                        + editor.brush.gradient_stops[sel_idx + 1].position) / 2.0
                } else {
                    (editor.brush.gradient_stops[sel_idx].position + 1.0) / 2.0
                };
                let new_stop = crate::model::sprite::GradientStop {
                    position: new_pos,
                    color_index: current_color_idx,
                };
                editor.brush.gradient_stops.push(new_stop);
                editor.brush.gradient_stops.sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap());
                editor.brush.gradient_midpoints.resize(
                    editor.brush.gradient_stops.len().saturating_sub(1), 0.5,
                );
                editor.brush.selected_stop_index = editor.brush.gradient_stops
                    .iter().position(|s| (s.position - new_pos).abs() < 0.001);
            }
            if ui.small_button("Delete").clicked() && editor.brush.gradient_stops.len() > 2 {
                editor.brush.gradient_stops.remove(sel_idx);
                editor.brush.gradient_midpoints.resize(
                    editor.brush.gradient_stops.len().saturating_sub(1), 0.5,
                );
                editor.brush.selected_stop_index = Some(sel_idx.min(
                    editor.brush.gradient_stops.len().saturating_sub(1),
                ));
            }
        });
    });

    ui.add_space(4.0);

    if editor.brush.fill_mode == FillMode::LinearGradient {
        // Direction presets
        ui.label("Direction");
        ui.horizontal(|ui| {
            let alignments = [
                (GradientAlignment::Horizontal, icons::grad_horizontal(), "Horizontal"),
                (GradientAlignment::Vertical, icons::grad_vertical(), "Vertical"),
                (GradientAlignment::IsoDescending, icons::grad_iso_desc(), "Iso Descending"),
                (GradientAlignment::IsoAscending, icons::grad_iso_asc(), "Iso Ascending"),
            ];
            for (align, icon, tooltip) in alignments {
                let angle = align.to_radians();
                let selected = (editor.brush.gradient_angle - angle).abs() < 0.01;
                if ui.add(icons::small_icon_button(icon, ui).selected(selected))
                    .on_hover_text(tooltip)
                    .clicked()
                {
                    editor.brush.gradient_angle = angle;
                }
            }
        });
        theme::with_input_style(ui, project.editor_preferences.theme, |ui| {
            ui.horizontal(|ui| {
                ui.label("Angle");
                let mut degrees = editor.brush.gradient_angle.to_degrees();
                if ui.add(egui::DragValue::new(&mut degrees)
                    .speed(1.0).range(-180.0..=180.0).suffix("°").fixed_decimals(1))
                    .changed()
                {
                    editor.brush.gradient_angle = degrees.to_radians();
                }
            });
        });
    } else {
        // Radial controls
        theme::with_input_style(ui, project.editor_preferences.theme, |ui| {
            ui.horizontal(|ui| {
                ui.label("Center X");
                ui.add(egui::DragValue::new(&mut editor.brush.radial_center.x)
                    .speed(0.01).range(0.0..=1.0).fixed_decimals(2));
                ui.label("Y");
                ui.add(egui::DragValue::new(&mut editor.brush.radial_center.y)
                    .speed(0.01).range(0.0..=1.0).fixed_decimals(2));
            });
            ui.horizontal(|ui| {
                ui.label("Radius");
                ui.add(egui::Slider::new(&mut editor.brush.radial_radius, 0.1..=1.0).fixed_decimals(2));
            });
            ui.horizontal(|ui| {
                ui.label("Focal X");
                ui.add(egui::DragValue::new(&mut editor.brush.radial_focal_offset.x)
                    .speed(0.01).range(0.0..=1.0).fixed_decimals(2));
                ui.label("Y");
                ui.add(egui::DragValue::new(&mut editor.brush.radial_focal_offset.y)
                    .speed(0.01).range(0.0..=1.0).fixed_decimals(2));
            });
        });
    }

    // Spread method
    ui.add_space(4.0);
    ui.label("Spread");
    ui.horizontal(|ui| {
        let methods = [
            (SpreadMethod::Pad, "Pad"),
            (SpreadMethod::Reflect, "Reflect"),
            (SpreadMethod::Repeat, "Repeat"),
        ];
        for (method, label) in methods {
            let selected = editor.brush.gradient_spread == method;
            if ui.selectable_label(selected, label).clicked() {
                editor.brush.gradient_spread = method;
            }
        }
    });
}
