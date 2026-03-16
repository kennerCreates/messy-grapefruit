use crate::engine::hit_test;
use crate::model::project::{Project, Theme};
use crate::model::sprite::Sprite;
use crate::state::editor::EditorState;

use super::canvas::HIT_TEST_THRESHOLD;

/// Handle eyedropper tool input: click to sample stroke color, shift+click for fill color.
#[allow(clippy::too_many_arguments)]
pub(super) fn handle_eyedropper_tool(
    response: &egui::Response,
    painter: &egui::Painter,
    editor: &mut EditorState,
    sprite: &Sprite,
    project: &Project,
    canvas_rect: egui::Rect,
    _theme_mode: Theme,
) {
    let canvas_center = canvas_rect.center();
    let threshold = HIT_TEST_THRESHOLD / editor.viewport.zoom;

    // Hover: show color swatch tooltip
    if let Some(hover_pos) = response.hover_pos() {
        let world = editor.viewport.screen_to_world(hover_pos, canvas_center);
        let hit = hit_test::hit_test_eyedropper(
            world,
            sprite,
            threshold,
            editor.layer.solo_layer_id.as_deref(),
        );

        if let Some((_, stroke_idx, _)) = &hit {
            // Show a small color swatch tooltip near cursor
            let color = project.palette.get_color(*stroke_idx).to_color32();
            let swatch_rect = egui::Rect::from_min_size(
                egui::pos2(hover_pos.x + 16.0, hover_pos.y - 8.0),
                egui::vec2(16.0, 16.0),
            );
            painter.rect_filled(swatch_rect, 2.0, color);
            painter.rect_stroke(
                swatch_rect,
                2.0,
                egui::Stroke::new(1.0, egui::Color32::WHITE),
                egui::StrokeKind::Outside,
            );
        }

        editor.hover_element_id = hit.map(|(id, _, _)| id);
    } else {
        editor.hover_element_id = None;
    }

    // Click: sample color
    if response.clicked()
        && let Some(click_pos) = response.interact_pointer_pos()
    {
        let world = editor.viewport.screen_to_world(click_pos, canvas_center);
        let shift = response.ctx.input(|i| i.modifiers.shift);

        let hit = hit_test::hit_test_eyedropper(
            world,
            sprite,
            threshold,
            editor.layer.solo_layer_id.as_deref(),
        );

        match hit {
            Some((_, stroke_idx, fill_idx)) => {
                if shift {
                    editor.brush.fill_color_index = fill_idx;
                    editor.track_recent_color(fill_idx);
                } else {
                    editor.brush.color_index = stroke_idx;
                    editor.track_recent_color(stroke_idx);
                }
            }
            None => {
                // Sample background color
                if shift {
                    editor.brush.fill_color_index = sprite.background_color_index;
                } else {
                    editor.brush.color_index = sprite.background_color_index;
                }
                editor.track_recent_color(sprite.background_color_index);
            }
        }

        // Return to previous tool if this was a temporary eyedropper (Alt+click)
        if let Some(return_tool) = editor.eyedropper_return_tool.take() {
            editor.tool = return_tool;
        }
    }
}
