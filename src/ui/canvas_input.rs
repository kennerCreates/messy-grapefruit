use crate::action::AppAction;
use crate::engine::merge;
use crate::engine::snap;
use crate::math;
use crate::model::project::Project;
use crate::model::sprite::{PathVertex, Sprite, StrokeElement};
use crate::model::vec2::Vec2;
use crate::state::editor::EditorState;

/// Handle viewport input (pan, zoom, flip, zoom-to-fit).
pub fn handle_viewport_input(
    editor: &mut EditorState,
    project: &Project,
    sprite: &Sprite,
    canvas_rect: egui::Rect,
    ui: &egui::Ui,
) {
    let canvas_center = canvas_rect.center();

    // Scroll wheel = zoom centered on cursor
    let scroll = ui.input(|i| i.smooth_scroll_delta.y);
    if scroll != 0.0 {
        let factor = if scroll > 0.0 { 1.1 } else { 1.0 / 1.1 };
        if let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) {
            editor.viewport.zoom_at(pointer, factor, canvas_center);
        }
    }

    // Middle-click drag = pan
    if ui.input(|i| i.pointer.middle_down()) {
        let delta = ui.input(|i| i.pointer.delta());
        if delta.length() > 0.0 {
            let mut world_delta = Vec2::new(delta.x, delta.y) / editor.viewport.zoom;
            if editor.viewport.flipped {
                world_delta.x = -world_delta.x;
            }
            editor.viewport.offset += world_delta;
        }
    }

    // H key = canvas flip around sprite center
    if ui.input(|i| i.key_pressed(egui::Key::H)) && !ui.input(|i| i.modifiers.ctrl) {
        let cx = sprite.canvas_width as f32 / 2.0;
        editor.viewport.flipped = !editor.viewport.flipped;
        editor.viewport.offset.x = -(editor.viewport.offset.x + 2.0 * cx);
    }

    // C key = toggle curve/straight mode (line tool only)
    if ui.input(|i| i.key_pressed(egui::Key::C)) && !ui.input(|i| i.modifiers.ctrl)
        && matches!(editor.tool, crate::state::editor::ToolKind::Line)
    {
        editor.line_tool.curve_mode = !editor.line_tool.curve_mode;
        // Recompute control points on in-progress vertices when mode changes
        if editor.line_tool.is_drawing {
            crate::math::recompute_auto_curves(
                &mut editor.line_tool.vertices,
                false,
                editor.line_tool.curve_mode,
                project.min_corner_radius,
            );
        }
    }

    // V key = switch to select tool
    if ui.input(|i| i.key_pressed(egui::Key::V)) && !ui.input(|i| i.modifiers.ctrl) {
        editor.clear_vertex_selection();
        editor.tool = crate::state::editor::ToolKind::Select;
    }

    // L key = switch to line tool
    if ui.input(|i| i.key_pressed(egui::Key::L)) && !ui.input(|i| i.modifiers.ctrl) {
        editor.clear_vertex_selection();
        editor.tool = crate::state::editor::ToolKind::Line;
    }

    // G key = switch to fill tool
    if ui.input(|i| i.key_pressed(egui::Key::G)) && !ui.input(|i| i.modifiers.ctrl) {
        editor.clear_vertex_selection();
        editor.tool = crate::state::editor::ToolKind::Fill;
    }

    // I key = switch to eyedropper tool
    if ui.input(|i| i.key_pressed(egui::Key::I)) && !ui.input(|i| i.modifiers.ctrl) {
        editor.clear_vertex_selection();
        editor.eyedropper_return_tool = None; // explicit switch, not temporary
        editor.tool = crate::state::editor::ToolKind::Eyedropper;
    }

    // Alt+click = temporary eyedropper (from Line and Fill tools only)
    if ui.input(|i| i.modifiers.alt && i.pointer.primary_pressed())
        && matches!(editor.tool, crate::state::editor::ToolKind::Line | crate::state::editor::ToolKind::Fill)
    {
        editor.eyedropper_return_tool = Some(editor.tool);
        editor.tool = crate::state::editor::ToolKind::Eyedropper;
    }
}

/// Handle line tool input. Returns an action if a stroke was committed.
pub fn handle_line_tool_input(
    response: &egui::Response,
    editor: &mut EditorState,
    sprite: &Sprite,
    project: &Project,
    canvas_rect: egui::Rect,
) -> (Option<AppAction>, Option<Vec2>) {
    let canvas_center = canvas_rect.center();
    let mut merge_target_pos: Option<Vec2> = None;

    // Get cursor world position
    let cursor_screen = response.hover_pos().unwrap_or(canvas_rect.center());
    let cursor_world = editor.viewport.screen_to_world(cursor_screen, canvas_center);
    let snap_pos = snap::snap_to_grid(
        cursor_world,
        project.editor_preferences.grid_size,
        project.editor_preferences.grid_mode,
    );

    // Check for merge target
    let layer = sprite.layers.get(editor.layer.resolve_active_idx(sprite));
    if let Some(layer) = layer {
        let threshold = project.editor_preferences.grid_size as f32;
        if let Some(target) = merge::find_merge_target(snap_pos, layer, None, threshold) {
            merge_target_pos = Some(target.position);
        }
    }

    // Escape = cancel drawing
    let escape_pressed = response.ctx.input(|i| i.key_pressed(egui::Key::Escape));
    if escape_pressed && editor.line_tool.is_drawing {
        editor.line_tool.clear();
        return (None, merge_target_pos);
    }

    // Right-click = commit stroke (if enough vertices)
    if response.secondary_clicked() {
        if editor.line_tool.vertices.len() >= 2 {
            let action = commit_stroke(editor, sprite, project);
            return (Some(action), merge_target_pos);
        } else {
            editor.line_tool.clear();
            return (None, merge_target_pos);
        }
    }

    // Check if active layer is locked — prevent drawing
    if let Some(layer) = sprite.layers.get(editor.layer.resolve_active_idx(sprite))
        && layer.locked
    {
        return (None, merge_target_pos);
    }

    // Left click = place vertex or finish
    if response.clicked() {
        let is_double_click = response.double_clicked();

        if is_double_click && editor.line_tool.vertices.len() >= 2 {
            // Double-click finishes the stroke
            let action = commit_stroke(editor, sprite, project);
            return (Some(action), merge_target_pos);
        }

        // Place a vertex
        let vertex = PathVertex::new(snap_pos);
        editor.line_tool.vertices.push(vertex);
        editor.line_tool.is_drawing = true;

        // Recompute auto-curves (applies curve mode + min corner radius)
        math::recompute_auto_curves(
            &mut editor.line_tool.vertices,
            false,
            editor.line_tool.curve_mode,
            project.min_corner_radius,
        );
    }

    // Enter key also finishes the stroke
    let enter_pressed = response.ctx.input(|i| i.key_pressed(egui::Key::Enter));
    if enter_pressed && editor.line_tool.vertices.len() >= 2 {
        let action = commit_stroke(editor, sprite, project);
        return (Some(action), merge_target_pos);
    }

    (None, merge_target_pos)
}

/// Commit the current line tool stroke as a StrokeElement.
fn commit_stroke(
    editor: &mut EditorState,
    sprite: &Sprite,
    project: &Project,
) -> AppAction {
    let mut vertices = std::mem::take(&mut editor.line_tool.vertices);
    editor.line_tool.is_drawing = false;

    let threshold = project.editor_preferences.grid_size as f32;

    // If first and last vertices coincide, close the path instead of overlapping
    if vertices.len() >= 3 && vertices[0].pos.distance(vertices[vertices.len() - 1].pos) < threshold
    {
        vertices.pop();
        math::recompute_auto_curves(
            &mut vertices,
            true,
            editor.line_tool.curve_mode,
            project.min_corner_radius,
        );
        let mut element =
            StrokeElement::new(vertices, editor.brush.stroke_width, editor.brush.color_index, editor.line_tool.curve_mode);
        element.closed = true;
        return AppAction::CommitStroke(element);
    }

    // Check for merge at start and end
    let layer = sprite.layers.get(editor.layer.resolve_active_idx(sprite));

    if let Some(layer) = layer {
        // Check if start vertex merges with an existing element
        let start_pos = vertices[0].pos;
        let end_pos = vertices[vertices.len() - 1].pos;

        if let Some(target) = merge::find_merge_target(start_pos, layer, None, threshold) {
            // Merge at start
            if let Some(existing) = layer.elements.iter().find(|e| e.id == target.element_id) {
                let merged = merge::merge_elements(
                    existing,
                    target.end,
                    &vertices,
                    merge::VertexEnd::Start,
                    editor.brush.stroke_width,
                    editor.brush.color_index,
                    editor.line_tool.curve_mode,
                    project.min_corner_radius,
                );
                return AppAction::MergeStroke {
                    merged_element: merged,
                    replace_element_id: target.element_id,
                };
            }
        }

        if let Some(target) = merge::find_merge_target(end_pos, layer, None, threshold)
            && let Some(existing) = layer.elements.iter().find(|e| e.id == target.element_id)
        {
            let merged = merge::merge_elements(
                existing,
                target.end,
                &vertices,
                merge::VertexEnd::End,
                editor.brush.stroke_width,
                editor.brush.color_index,
                editor.line_tool.curve_mode,
                project.min_corner_radius,
            );
            return AppAction::MergeStroke {
                merged_element: merged,
                replace_element_id: target.element_id,
            };
        }
    }

    // No merge — create new element
    let element = StrokeElement::new(vertices, editor.brush.stroke_width, editor.brush.color_index, editor.line_tool.curve_mode);
    AppAction::CommitStroke(element)
}

/// Get the current snap position for the cursor.
pub fn get_snap_pos(
    editor: &EditorState,
    project: &Project,
    canvas_rect: egui::Rect,
    hover_pos: Option<egui::Pos2>,
) -> Vec2 {
    let canvas_center = canvas_rect.center();
    let cursor_screen = hover_pos.unwrap_or(canvas_rect.center());
    let cursor_world = editor.viewport.screen_to_world(cursor_screen, canvas_center);
    snap::snap_to_grid(
        cursor_world,
        project.editor_preferences.grid_size,
        project.editor_preferences.grid_mode,
    )
}
