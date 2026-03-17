use crate::action::AppAction;
use crate::model::project::Project;
use crate::model::sprite::{GradientAlignment, GradientStop, SpreadMethod, Sprite};
use crate::state::editor::{EditorState, FillMode};
use crate::state::history::History;
use crate::theme;

use super::gradient_bar;
use super::icons;
use super::sidebar_palette::{render_color_palette, render_color_swatch};

pub(super) fn show_fill_tool_options(
    ui: &mut egui::Ui,
    editor: &mut EditorState,
    project: &mut Project,
    actions: &mut Vec<AppAction>,
) {
    // ── Fill section (collapsible) ──
    egui::CollapsingHeader::new("Fill")
        .default_open(true)
        .show(ui, |ui| {
            // Fill mode toggles (flat / linear gradient / radial gradient)
            ui.horizontal(|ui| {
                let modes = [
                    (FillMode::Flat, icons::fill_flat(), "Flat Fill"),
                    (FillMode::LinearGradient, icons::fill_linear(), "Linear Gradient"),
                    (FillMode::RadialGradient, icons::fill_radial(), "Radial Gradient"),
                ];
                for (mode, icon, tooltip) in modes {
                    let selected = editor.brush.fill_mode == mode;
                    if ui.add(icons::small_icon_button(icon, ui).selected(selected))
                        .on_hover_text(tooltip)
                        .clicked()
                    {
                        editor.brush.fill_mode = mode;
                    }
                }
            });

            ui.add_space(4.0);

            match editor.brush.fill_mode {
                FillMode::Flat => {
                    ui.horizontal(|ui| {
                        ui.label("Fill");
                        let color = project.palette.get_color(editor.brush.fill_color_index);
                        render_color_swatch(ui, color, 20.0, project.editor_preferences.theme);
                        ui.label(format!("idx {}", editor.brush.fill_color_index));
                    });

                    if let Some(new_idx) = render_color_palette(
                        ui,
                        &project.palette.colors,
                        editor.brush.fill_color_index,
                        project.editor_preferences.theme,
                    ) {
                        editor.brush.fill_color_index = new_idx;
                        editor.track_recent_color(new_idx);
                    }
                }
                FillMode::LinearGradient | FillMode::RadialGradient => {
                    render_gradient_controls(ui, editor, project, actions);
                }
            }
        });

    // ── Hatch section (collapsible) ──
    egui::CollapsingHeader::new("Hatch")
        .default_open(false)
        .show(ui, |ui| {
            render_hatch_picker(ui, editor, project, actions);
        });

    ui.add_space(4.0);
    ui.label("Click closed shape to apply");
}

fn render_gradient_controls(
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
            // Previous/next stop buttons
            if ui.small_button("<").clicked() && sel_idx > 0 {
                editor.brush.selected_stop_index = Some(sel_idx - 1);
            }
            ui.label(format!("{}/{}", sel_idx + 1, stop_count));
            if ui.small_button(">").clicked() && sel_idx + 1 < stop_count {
                editor.brush.selected_stop_index = Some(sel_idx + 1);
            }

            // Location drag value (as percentage)
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
                // Add a stop at the midpoint of the current segment
                let new_pos = if sel_idx + 1 < editor.brush.gradient_stops.len() {
                    (editor.brush.gradient_stops[sel_idx].position
                        + editor.brush.gradient_stops[sel_idx + 1].position) / 2.0
                } else {
                    (editor.brush.gradient_stops[sel_idx].position + 1.0) / 2.0
                };
                let new_stop = GradientStop { position: new_pos, color_index: current_color_idx };
                editor.brush.gradient_stops.push(new_stop);
                editor.brush.gradient_stops.sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap());
                // Update midpoints to match new stop count
                editor.brush.gradient_midpoints.resize(
                    editor.brush.gradient_stops.len().saturating_sub(1), 0.5,
                );
                // Select the new stop
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
        // Free angle input (degrees)
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

fn render_hatch_picker(
    ui: &mut egui::Ui,
    editor: &mut EditorState,
    project: &mut Project,
    actions: &mut Vec<AppAction>,
) {
    // Apply toggle
    ui.checkbox(&mut editor.brush.hatch_apply_enabled, "Apply on click");

    ui.add_space(4.0);

    if project.hatch_patterns.is_empty() {
        ui.label("No patterns yet");
    } else {
        let mut selected_idx = project.hatch_patterns.iter().position(|p| {
            editor.selected_hatch_pattern_id.as_deref() == Some(p.id.as_str())
        });
        for (i, pattern) in project.hatch_patterns.iter().enumerate() {
            let is_selected = selected_idx == Some(i);
            if ui.selectable_label(is_selected, &pattern.name).clicked() {
                editor.selected_hatch_pattern_id = Some(pattern.id.clone());
                selected_idx = Some(i);
            }
        }
    }

    ui.add_space(4.0);

    ui.horizontal(|ui| {
        if ui.button("+ New").clicked() {
            let pattern = crate::model::project::HatchPattern::new("Hatch");
            editor.selected_hatch_pattern_id = Some(pattern.id.clone());
            actions.push(AppAction::AddHatchPattern(pattern));
        }
        if ui.button("Edit").clicked() {
            editor.hatch_editor_open = !editor.hatch_editor_open;
        }
    });

    ui.add_space(4.0);

    if ui.button("Remove Hatch from Selected").clicked() {
        for id in &editor.selection.selected_ids {
            actions.push(AppAction::ClearHatchFill { element_id: id.clone() });
        }
    }
}

pub(super) fn show_eyedropper_tool_options(
    ui: &mut egui::Ui,
    editor: &mut EditorState,
    project: &mut Project,
) {
    ui.label("Eyedropper");
    ui.add_space(4.0);

    // Show current stroke and fill colors in an aligned grid
    egui::Grid::new("eyedropper_colors").show(ui, |ui| {
        ui.label("Stroke");
        let color = project.palette.get_color(editor.brush.color_index);
        render_color_swatch(ui, color, 20.0, project.editor_preferences.theme);
        ui.label(format!("idx {}", editor.brush.color_index));
        ui.end_row();

        ui.label("Fill");
        let color = project.palette.get_color(editor.brush.fill_color_index);
        render_color_swatch(ui, color, 20.0, project.editor_preferences.theme);
        ui.label(format!("idx {}", editor.brush.fill_color_index));
        ui.end_row();
    });

    ui.add_space(8.0);
    ui.label("Click = stroke color");
    ui.label("Shift+Click = fill color");
}

pub(super) fn show_line_tool_options(
    ui: &mut egui::Ui,
    editor: &mut EditorState,
    sprite: &mut Sprite,
    project: &mut Project,
) {
    // Stroke width: 3 toggle buttons
    ui.horizontal(|ui| {
        ui.add(icons::small_icon(icons::prop_width(), ui));
        ui.label("Width");
        for &w in &[2.0_f32, 4.0, 8.0] {
            let selected = (editor.brush.stroke_width - w).abs() < 0.01;
            if ui.selectable_label(selected, format!("{}", w as u32)).clicked() {
                editor.brush.stroke_width = w;
            }
        }
    });

    ui.add_space(4.0);

    // Active color
    ui.horizontal(|ui| {
        ui.label("Color");
        let color = project.palette.get_color(editor.brush.color_index);
        render_color_swatch(ui, color, 20.0, project.editor_preferences.theme);
        ui.label(format!("idx {}", editor.brush.color_index));
    });

    // Color palette mini-picker
    if let Some(new_idx) = render_color_palette(ui, &project.palette.colors, editor.brush.color_index, project.editor_preferences.theme) {
        editor.brush.color_index = new_idx;
    }

    ui.add_space(4.0);

    // Min corner radius
    let radius_changed = theme::with_input_style(ui, project.editor_preferences.theme, |ui| {
        ui.horizontal(|ui| {
            ui.add(icons::small_icon(icons::prop_radius(), ui));
            ui.label("Radius");
            ui.add(
                egui::Slider::new(&mut project.min_corner_radius, 0.0..=32.0).fixed_decimals(1),
            )
            .changed()
        })
        .inner
    });

    if radius_changed {
        crate::engine::transform::recompute_all_curves(sprite, project.min_corner_radius);
    }

    ui.add_space(4.0);

    // Curve/straight toggle
    ui.horizontal(|ui| {
        let mut mode_changed = false;
        if ui
            .add(icons::icon_button(icons::mode_curve(), ui).selected(editor.line_tool.curve_mode))
            .on_hover_text("Curve Mode (C)")
            .clicked()
            && !editor.line_tool.curve_mode
        {
            editor.line_tool.curve_mode = true;
            mode_changed = true;
        }
        if ui
            .add(icons::icon_button(icons::mode_straight(), ui).selected(!editor.line_tool.curve_mode))
            .on_hover_text("Straight Mode")
            .clicked()
            && editor.line_tool.curve_mode
        {
            editor.line_tool.curve_mode = false;
            mode_changed = true;
        }
        if mode_changed && editor.line_tool.is_drawing {
            crate::math::recompute_auto_curves(
                &mut editor.line_tool.vertices,
                false,
                editor.line_tool.curve_mode,
                project.min_corner_radius,
            );
        }
    });
}

pub(super) fn show_select_tool_options(
    ui: &mut egui::Ui,
    editor: &mut EditorState,
    sprite: &mut Sprite,
    project: &mut Project,
    history: &mut History,
) {
    if editor.selection.is_empty() {
        ui.label("Select Tool");
        ui.add_space(4.0);
        ui.label("Click to select elements");
        return;
    }

    // Find the first selected element to show properties
    // (for multi-select, show shared properties or first element's values)
    let selected = editor.selection.selected_ids.clone();
    let count = selected.len();

    ui.label(if count == 1 { "Element" } else { "Selection" });
    ui.add_space(4.0);

    // Gather current values from first selected element
    let first_elem = sprite.layers.iter().flat_map(|l| &l.elements)
        .find(|e| selected.iter().any(|id| id == &e.id));

    let (mut pos_x, mut pos_y, mut rot_deg, mut scale_x, mut scale_y, mut stroke_w, mut color_idx, mut is_curve) =
        match first_elem {
            Some(e) => (
                e.position.x, e.position.y,
                e.rotation.to_degrees(), e.scale.x, e.scale.y,
                e.stroke_width, e.stroke_color_index, e.curve_mode,
            ),
            None => return,
        };

    // Track which property changed (for separate undo descriptions)
    let mut change_desc: Option<&str> = None;

    theme::with_input_style(ui, project.editor_preferences.theme, |ui| {
        // Position
        ui.horizontal(|ui| {
            ui.add(icons::small_icon(icons::prop_position(), ui));
            ui.label("X");
            if ui.add(egui::DragValue::new(&mut pos_x).speed(0.5).fixed_decimals(1)).changed() {
                change_desc = Some("Edit position");
            }
            ui.label("Y");
            if ui.add(egui::DragValue::new(&mut pos_y).speed(0.5).fixed_decimals(1)).changed() {
                change_desc = Some("Edit position");
            }
        });

        // Rotation
        ui.horizontal(|ui| {
            ui.add(icons::small_icon(icons::prop_rotation(), ui));
            if ui.add(egui::DragValue::new(&mut rot_deg).speed(1.0).suffix("°").fixed_decimals(1)).changed() {
                change_desc = Some("Edit rotation");
            }
        });

        // Scale
        ui.horizontal(|ui| {
            ui.add(icons::small_icon(icons::prop_scale(), ui));
            ui.label("X");
            if ui.add(egui::DragValue::new(&mut scale_x).speed(0.01).fixed_decimals(2)).changed() {
                change_desc = Some("Edit scale");
            }
            ui.label("Y");
            if ui.add(egui::DragValue::new(&mut scale_y).speed(0.01).fixed_decimals(2)).changed() {
                change_desc = Some("Edit scale");
            }
        });
    });

    // Stroke width: 3 toggle buttons
    ui.horizontal(|ui| {
        ui.add(icons::small_icon(icons::prop_width(), ui));
        ui.label("Width");
        for &w in &[2.0_f32, 4.0, 8.0] {
            let selected = (stroke_w - w).abs() < 0.01;
            if ui.selectable_label(selected, format!("{}", w as u32)).clicked() {
                stroke_w = w;
                change_desc = Some("Edit stroke width");
            }
        }
    });

    ui.add_space(4.0);

    // Color picker (same mini palette as line tool)
    if let Some(new_idx) = render_color_palette(ui, &project.palette.colors, color_idx, project.editor_preferences.theme) {
        color_idx = new_idx;
        change_desc = Some("Edit color");
    }

    ui.add_space(4.0);

    // Curve/straight toggle — sets all selected elements to the same mode
    ui.horizontal(|ui| {
        if ui.add(icons::icon_button(icons::mode_curve(), ui).selected(is_curve)).on_hover_text("Curve Mode (C)").clicked()
            && !is_curve
        {
            is_curve = true;
            change_desc = Some("Edit curve mode");
        }
        if ui.add(icons::icon_button(icons::mode_straight(), ui).selected(!is_curve)).on_hover_text("Straight Mode").clicked()
            && is_curve
        {
            is_curve = false;
            change_desc = Some("Edit curve mode");
        }
    });

    // Apply changes — coalesced undo per property type
    if let Some(desc) = change_desc {
        let before = sprite.clone();
        let new_rot = rot_deg.to_radians();
        let min_radius = project.min_corner_radius;
        crate::engine::transform::for_selected_elements_mut(sprite, &selected, |element| {
            element.position.x = pos_x;
            element.position.y = pos_y;
            element.rotation = new_rot;
            element.scale.x = scale_x;
            element.scale.y = scale_y;
            element.stroke_width = stroke_w;
            element.stroke_color_index = color_idx;
            if element.curve_mode != is_curve {
                element.curve_mode = is_curve;
                crate::math::recompute_auto_curves(
                    &mut element.vertices,
                    element.closed,
                    element.curve_mode,
                    min_radius,
                );
            }
        });
        history.push_coalesced(desc.into(), before, sprite.clone());
    }
}
