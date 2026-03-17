use crate::action::AppAction;
use crate::model::project::{HatchLayer, Project};
use crate::model::sprite::Sprite;
use crate::state::editor::EditorState;
use crate::theme;

use super::icons;

/// Isometric angle presets for hatch layers (degrees).
/// Descending edge (top-left to bottom-right): direction (2,1), slope 0.5, angle atan(0.5).
/// Ascending edge (top-right to bottom-left): direction (-2,1), slope -0.5, angle 180 - atan(0.5).
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

    let pattern = match project.hatch_patterns.iter_mut().find(|p| p.id == pattern_id) {
        Some(p) => p,
        None => {
            ui.label("Pattern not found");
            editor.selected_hatch_pattern_id = None;
            return;
        }
    };

    ui.separator();
    ui.label("Hatch Editor");
    ui.add_space(4.0);

    // Pattern name
    theme::with_input_style(ui, project.editor_preferences.theme, |ui| {
        ui.horizontal(|ui| {
            ui.label("Name");
            ui.text_edit_singleline(&mut pattern.name);
        });
    });

    ui.add_space(2.0);
    ui.label("Uses element stroke color & width");
    ui.add_space(4.0);

    // Layers
    let mut layer_to_remove: Option<usize> = None;
    let num_layers = pattern.layers.len();

    for (i, layer) in pattern.layers.iter_mut().enumerate() {
        ui.group(|ui| {
            ui.label(format!("Layer {}", i + 1));

            theme::with_input_style(ui, project.editor_preferences.theme, |ui| {
                // Angle with iso snap presets
                ui.horizontal(|ui| {
                    ui.label("Angle");
                    ui.add(egui::Slider::new(&mut layer.angle, 0.0..=180.0).suffix("°").fixed_decimals(1));
                });
                // Angle snap presets
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
                ui.horizontal(|ui| {
                    ui.label("Spacing");
                    ui.add(egui::Slider::new(&mut layer.spacing, 1.0..=64.0).fixed_decimals(1));
                });
                ui.horizontal(|ui| {
                    ui.label("Offset");
                    ui.add(egui::Slider::new(&mut layer.offset, 0.0..=32.0).fixed_decimals(1));
                });
            });

            if num_layers > 1 {
                if ui.add(icons::small_icon_button(icons::hatch_remove_layer(), ui))
                    .on_hover_text("Remove Layer")
                    .clicked()
                {
                    layer_to_remove = Some(i);
                }
            }
        });
    }

    if let Some(idx) = layer_to_remove {
        if let Some(p) = project.hatch_patterns.iter_mut().find(|p| p.id == pattern_id) {
            p.layers.remove(idx);
        }
    }

    ui.horizontal(|ui| {
        if ui.add(icons::small_icon_button(icons::hatch_add_layer(), ui))
            .on_hover_text("Add Layer")
            .clicked()
        {
            if let Some(p) = project.hatch_patterns.iter_mut().find(|p| p.id == pattern_id) {
                let angle = if p.layers.is_empty() { 45.0 } else { p.layers.last().unwrap().angle + 90.0 };
                p.layers.push(HatchLayer {
                    angle: angle % 180.0,
                    spacing: 8.0,
                    offset: 0.0,
                });
            }
        }
    });

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
