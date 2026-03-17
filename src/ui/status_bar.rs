use crate::engine::animation::{canvas_state, CanvasAnimState};
use crate::model::project::Project;
use crate::model::sprite::Sprite;
use crate::state::editor::EditorState;

use super::icons;

pub fn show_status_bar(ui: &mut egui::Ui, editor: &EditorState, sprite: &Sprite, project: &Project) {
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

        // Canvas size right-aligned
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(format!(
                "{}x{} px",
                sprite.canvas_width, sprite.canvas_height
            ));
        });
    });
}
