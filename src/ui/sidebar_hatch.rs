use crate::action::AppAction;
use crate::model::project::{HatchLayer, PatternType, Project, Theme};
use crate::model::sprite::Sprite;
use crate::state::editor::EditorState;
use crate::theme;

use super::icons;

/// Isometric angle presets for hatch layers (degrees).
const ISO_ANGLE_DESC: f32 = 26.57;  // atan(0.5) in degrees
const ISO_ANGLE_ASC: f32 = 153.43;  // 180 - atan(0.5) in degrees

/// Show the hatch pattern editor panel (when hatch_editor_open is true).
pub(super) fn show_hatch_editor(
    ui: &mut egui::Ui,
    editor: &mut EditorState,
    project: &mut Project,
    _sprite: &Sprite,
    actions: &mut Vec<AppAction>,
) {
    let pattern_id = match &editor.selected_hatch_pattern_id {
        Some(id) => id.clone(),
        None => {
            ui.label("No pattern selected");
            return;
        }
    };

    let theme_mode = project.editor_preferences.theme;

    // Check pattern exists
    if !project.hatch_patterns.iter().any(|p| p.id == pattern_id) {
        ui.label("Pattern not found");
        editor.selected_hatch_pattern_id = None;
        return;
    }

    ui.separator();
    ui.label("Hatch Editor");
    ui.add_space(4.0);

    // Pattern name
    if let Some(pattern) = project.hatch_patterns.iter_mut().find(|p| p.id == pattern_id) {
        theme::with_input_style(ui, theme_mode, |ui| {
            ui.horizontal(|ui| {
                ui.label("Name");
                ui.text_edit_singleline(&mut pattern.name);
            });
        });
    }

    ui.add_space(2.0);
    ui.label("Uses element stroke color & width");
    ui.add_space(4.0);

    // Read pattern type
    let pattern_type = project.hatch_patterns.iter()
        .find(|p| p.id == pattern_id)
        .map(|p| p.pattern_type)
        .unwrap_or_default();

    // Type-specific controls
    match pattern_type {
        PatternType::Lines => {
            show_lines_editor(ui, &pattern_id, project, theme_mode);
        }
        PatternType::CrossHatch => {
            show_cross_hatch_editor(ui, &pattern_id, project, theme_mode);
        }
        PatternType::Brick => {
            show_brick_editor(ui, &pattern_id, project, theme_mode);
        }
    }

    ui.add_space(4.0);

    // Import/export
    ui.horizontal(|ui| {
        if ui.button("Import").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Hatch Patterns", &["hatchpatterns"])
                .pick_file()
            {
                match crate::io::load_hatch_patterns(&path) {
                    Ok(patterns) => {
                        actions.push(AppAction::ImportHatchPatterns(patterns));
                    }
                    Err(e) => {
                        eprintln!("Failed to import hatch patterns: {e}");
                    }
                }
            }
        }
        if ui.button("Export").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Hatch Patterns", &["hatchpatterns"])
                .set_file_name("patterns.hatchpatterns")
                .save_file()
            {
                if let Err(e) = crate::io::save_hatch_patterns(&project.hatch_patterns, &path) {
                    eprintln!("Failed to export hatch patterns: {e}");
                }
            }
        }
    });

    ui.add_space(4.0);

    // Delete pattern
    if ui.button("Delete Pattern").clicked() {
        actions.push(AppAction::DeleteHatchPattern(pattern_id.clone()));
        editor.selected_hatch_pattern_id = None;
        editor.hatch_editor_open = false;
    }
}

/// Angle controls shared by all pattern types.
fn show_angle_controls(ui: &mut egui::Ui, layer: &mut HatchLayer, theme_mode: Theme) {
    theme::with_input_style(ui, theme_mode, |ui| {
        ui.horizontal(|ui| {
            ui.label("Angle");
            ui.add(egui::Slider::new(&mut layer.angle, 0.0..=180.0).suffix("°").fixed_decimals(1));
        });
    });
    ui.horizontal(|ui| {
        for (label, angle) in [
            ("0°", 0.0),
            ("90°", 90.0),
            ("Iso↘", ISO_ANGLE_DESC),
            ("Iso↗", ISO_ANGLE_ASC),
        ] {
            let selected = (layer.angle - angle).abs() < 0.5;
            if ui.selectable_label(selected, label).clicked() {
                layer.angle = angle;
            }
        }
    });
}

/// Lines pattern editor: multi-layer with independent angles.
fn show_lines_editor(ui: &mut egui::Ui, pattern_id: &str, project: &mut Project, theme_mode: Theme) {
    let mut layer_to_remove: Option<usize> = None;

    if let Some(pattern) = project.hatch_patterns.iter_mut().find(|p| p.id == pattern_id) {
        let num_layers = pattern.layers.len();

        for (i, layer) in pattern.layers.iter_mut().enumerate() {
            ui.group(|ui| {
                ui.label(format!("Layer {}", i + 1));
                show_angle_controls(ui, layer, theme_mode);
                theme::with_input_style(ui, theme_mode, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Spacing");
                        ui.add(egui::Slider::new(&mut layer.spacing, 1.0..=64.0).fixed_decimals(1));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Offset");
                        ui.add(egui::Slider::new(&mut layer.offset, 0.0..=32.0).fixed_decimals(1));
                    });
                });

                if num_layers > 1
                    && ui.add(icons::small_icon_button(icons::hatch_remove_layer(), ui))
                        .on_hover_text("Remove Layer")
                        .clicked()
                {
                    layer_to_remove = Some(i);
                }
            });
        }
    }

    if let Some(idx) = layer_to_remove
        && let Some(p) = project.hatch_patterns.iter_mut().find(|p| p.id == pattern_id)
    {
        p.layers.remove(idx);
    }

    if ui.add(icons::small_icon_button(icons::hatch_add_layer(), ui))
        .on_hover_text("Add Layer")
        .clicked()
        && let Some(p) = project.hatch_patterns.iter_mut().find(|p| p.id == pattern_id)
    {
        let angle = if p.layers.is_empty() { 45.0 } else { p.layers.last().unwrap().angle + 90.0 };
        p.layers.push(HatchLayer {
            angle: angle % 180.0,
            spacing: 8.0,
            offset: 0.0,
        });
    }
}

/// CrossHatch editor: single angle + spacing, perpendicular auto-generated.
fn show_cross_hatch_editor(ui: &mut egui::Ui, pattern_id: &str, project: &mut Project, theme_mode: Theme) {
    if let Some(pattern) = project.hatch_patterns.iter_mut().find(|p| p.id == pattern_id) {
        if pattern.layers.is_empty() {
            pattern.layers.push(HatchLayer { angle: 45.0, spacing: 8.0, offset: 0.0 });
        }
        let layer = &mut pattern.layers[0];

        show_angle_controls(ui, layer, theme_mode);
        let cross_label = if pattern.iso_mode {
            "Cross at 90.0° (vertical)".to_string()
        } else {
            format!("Cross at {:.1}°", (layer.angle + 90.0) % 180.0)
        };
        ui.label(cross_label);
        theme::with_input_style(ui, theme_mode, |ui| {
            ui.horizontal(|ui| {
                ui.label("Spacing");
                ui.add(egui::Slider::new(&mut layer.spacing, 1.0..=64.0).fixed_decimals(1));
            });
        });
        ui.checkbox(&mut pattern.iso_mode, "Iso (vertical cross)");
    }
}

/// Brick editor: angle + row height + brick width.
fn show_brick_editor(ui: &mut egui::Ui, pattern_id: &str, project: &mut Project, theme_mode: Theme) {
    if let Some(pattern) = project.hatch_patterns.iter_mut().find(|p| p.id == pattern_id) {
        if pattern.layers.is_empty() {
            pattern.layers.push(HatchLayer { angle: 0.0, spacing: 10.0, offset: 0.0 });
        }
        let layer = &mut pattern.layers[0];

        show_angle_controls(ui, layer, theme_mode);
        theme::with_input_style(ui, theme_mode, |ui| {
            ui.horizontal(|ui| {
                ui.label("Row Height");
                ui.add(egui::Slider::new(&mut layer.spacing, 2.0..=64.0).fixed_decimals(1));
            });
            ui.horizontal(|ui| {
                ui.label("Brick Width");
                ui.add(egui::Slider::new(&mut pattern.brick_width, 4.0..=128.0).fixed_decimals(1));
            });
        });
        ui.checkbox(&mut pattern.iso_mode, "Iso (vertical joints)");
    }
}
