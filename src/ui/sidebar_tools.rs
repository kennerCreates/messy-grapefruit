use crate::model::project::Project;
use crate::model::sprite::Sprite;
use crate::state::editor::EditorState;
use crate::state::history::History;
use crate::theme;

use super::icons;
use super::sidebar_palette::{render_color_palette, render_color_swatch};

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
        for &w in &[1.0_f32, 2.0, 4.0] {
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
        for &w in &[1.0_f32, 2.0, 4.0] {
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
