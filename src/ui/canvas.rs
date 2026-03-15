use crate::engine::hit_test;
use crate::model::project::Project;
use crate::model::sprite::{Sprite, StrokeElement};
use crate::model::vec2::Vec2;
use crate::state::editor::EditorState;
use crate::theme;

use super::{canvas_input, canvas_render, grid};

pub enum CanvasAction {
    CommitStroke(StrokeElement),
    MergeStroke {
        merged_element: StrokeElement,
        replace_element_id: String,
    },
}

pub fn show_canvas(
    ui: &mut egui::Ui,
    editor: &mut EditorState,
    sprite: &Sprite,
    project: &Project,
    active_layer_idx: usize,
) -> Vec<CanvasAction> {
    let mut actions = Vec::new();
    let theme_mode = project.editor_preferences.theme;

    // Allocate the full available space
    let (response, mut painter) = ui.allocate_painter(
        ui.available_size(),
        egui::Sense::click_and_drag(),
    );
    let canvas_rect = response.rect;

    // Fill canvas background
    let bg = theme::canvas_bg_color(theme_mode);
    painter.rect_filled(canvas_rect, 0.0, bg);

    // Clip to canvas
    painter.set_clip_rect(canvas_rect);

    // Handle viewport input (pan, zoom, flip)
    canvas_input::handle_viewport_input(&response, editor, canvas_rect, ui);

    // Handle F key or toolbar button = zoom to fit
    if (ui.input(|i| i.key_pressed(egui::Key::F)) && !ui.input(|i| i.modifiers.ctrl))
        || editor.zoom_to_fit_requested
    {
        editor.zoom_to_fit_requested = false;
        zoom_to_fit(editor, sprite, canvas_rect);
    }

    // Render grid
    grid::render_grid(
        &painter,
        &editor.viewport,
        &project.editor_preferences,
        canvas_rect,
        theme_mode,
    );

    // Render canvas boundary
    canvas_render::render_canvas_boundary(
        &painter,
        &editor.viewport,
        sprite.canvas_width,
        sprite.canvas_height,
        canvas_rect,
        theme_mode,
    );

    // Render all sprite elements
    canvas_render::render_elements(
        &painter,
        &editor.viewport,
        sprite,
        &project.palette,
        canvas_rect,
        project.stroke_taper,
        theme_mode,
    );

    // Hit testing for hover highlight
    if !editor.line_tool.is_drawing {
        if let Some(hover_pos) = response.hover_pos() {
            let world_pos = editor.viewport.screen_to_world(hover_pos, canvas_rect.center());
            let threshold = 8.0 / editor.viewport.zoom;
            editor.hover_element_id = hit_test::hit_test_elements(world_pos, sprite, threshold);
        } else {
            editor.hover_element_id = None;
        }
    }

    // Render hover highlight
    if let Some(ref hover_id) = editor.hover_element_id
        && !editor.line_tool.is_drawing
    {
        canvas_render::render_hover_highlight(
            &painter,
            sprite,
            hover_id,
            &editor.viewport,
            canvas_rect,
            theme_mode,
        );
    }

    // Handle line tool input
    let (line_action, merge_target) = canvas_input::handle_line_tool_input(
        &response,
        editor,
        sprite,
        project,
        canvas_rect,
        active_layer_idx,
    );
    if let Some(action) = line_action {
        actions.push(action);
    }

    // Render line tool preview
    if editor.line_tool.is_drawing && !editor.line_tool.vertices.is_empty() {
        let snap_pos = canvas_input::get_snap_pos(
            editor,
            project,
            canvas_rect,
            response.hover_pos(),
        );
        let cursor_world = response
            .hover_pos()
            .map(|p| editor.viewport.screen_to_world(p, canvas_rect.center()))
            .unwrap_or(snap_pos);

        canvas_render::render_line_tool_preview(
            &painter,
            &editor.line_tool.vertices,
            cursor_world,
            snap_pos,
            &project.palette,
            &editor.viewport,
            canvas_rect,
            editor.active_color_index,
            editor.active_stroke_width,
            theme_mode,
            merge_target,
        );
    }

    actions
}

fn zoom_to_fit(editor: &mut EditorState, sprite: &Sprite, canvas_rect: egui::Rect) {
    // Compute bounding box of all visible elements
    let mut min = Vec2::new(f32::MAX, f32::MAX);
    let mut max = Vec2::new(f32::MIN, f32::MIN);
    let mut has_content = false;

    for layer in &sprite.layers {
        if !layer.visible {
            continue;
        }
        for element in &layer.elements {
            for vertex in &element.vertices {
                min = min.min(vertex.pos);
                max = max.max(vertex.pos);
                has_content = true;
            }
        }
    }

    // Fall back to canvas boundary
    if !has_content {
        min = Vec2::ZERO;
        max = Vec2::new(sprite.canvas_width as f32, sprite.canvas_height as f32);
    }

    editor.viewport.zoom_to_fit(min, max, canvas_rect.size());
}
