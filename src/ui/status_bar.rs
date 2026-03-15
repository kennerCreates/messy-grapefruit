use crate::model::project::Project;
use crate::model::sprite::Sprite;
use crate::state::editor::EditorState;

use super::icons;

pub fn show_status_bar(ui: &mut egui::Ui, editor: &EditorState, sprite: &Sprite, project: &Project) {
    ui.horizontal(|ui| {
        // Tool icon
        ui.add(icons::small_icon(icons::tool_line()));

        ui.separator();

        // Curve/straight mode icon
        if editor.line_tool.curve_mode {
            ui.add(icons::small_icon(icons::mode_curve()));
        } else {
            ui.add(icons::small_icon(icons::mode_straight()));
        }

        ui.separator();

        // Flip indicator
        if editor.viewport.flipped {
            ui.add(icons::small_icon(icons::view_flip()));
            ui.colored_label(egui::Color32::YELLOW, "FLIPPED");
            ui.separator();
        }

        // Grid mode icon
        match project.editor_preferences.grid_mode {
            crate::model::project::GridMode::Straight => {
                ui.add(icons::small_icon(icons::grid_dots()));
            }
            crate::model::project::GridMode::Isometric => {
                ui.add(icons::small_icon(icons::grid_iso()));
            }
        };

        ui.separator();

        // Sprite metrics (right-aligned)
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(format!(
                "{}x{} px",
                sprite.canvas_width, sprite.canvas_height
            ));
            ui.separator();
            ui.add(icons::small_icon(icons::metric_animation()));
            ui.label("0");
            ui.separator();
            ui.add(icons::small_icon(icons::metric_layer()));
            ui.label(format!("{}", sprite.layer_count()));
            ui.separator();
            ui.add(icons::small_icon(icons::metric_vertex()));
            ui.label(format!("{}", sprite.vertex_count()));
            ui.separator();
            ui.add(icons::small_icon(icons::metric_element()));
            ui.label(format!("{}", sprite.element_count()));
        });
    });
}
