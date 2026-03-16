use crate::model::project::Project;
use crate::model::sprite::Sprite;
use crate::state::editor::EditorState;

use super::icons;

pub fn show_status_bar(ui: &mut egui::Ui, _editor: &EditorState, sprite: &Sprite, _project: &Project) {
    ui.horizontal(|ui| {
        // Flip indicator (always show icon; tint when active)
        if _editor.viewport.flipped {
            let tint = crate::theme::theme_colors(_project.editor_preferences.theme).icon_text;
            ui.add(icons::small_icon_tinted(icons::view_flip(), tint, ui));
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
        ui.label("0");

        // Canvas size right-aligned
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(format!(
                "{}x{} px",
                sprite.canvas_width, sprite.canvas_height
            ));
        });
    });
}
