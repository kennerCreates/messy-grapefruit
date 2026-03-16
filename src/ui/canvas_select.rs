use crate::engine::{hit_test, snap, transform};
use crate::math;
use crate::model::sprite::{Sprite, StrokeElement};
use crate::model::project::Project;
use crate::model::vec2::Vec2;
use crate::state::editor::{EditorState, HandleKind, SelectDragKind, VertexHover};
use crate::state::history::History;
use crate::theme;

use super::{canvas_render, canvas_transform};

/// Screen-pixel hit radius for transform handles.
const HANDLE_HIT_RADIUS: f32 = 7.0;
/// Rotation snap increment (15 degrees).
const ROTATION_SNAP_STEP: f32 = std::f32::consts::PI / 12.0;
/// Minimum delta to trigger a snapped move (avoids micro-jitter).
const SNAP_EPSILON: f32 = 0.001;
/// Marquee dashed line pattern.
const MARQUEE_DASH: f32 = 4.0;
const MARQUEE_GAP: f32 = 3.0;

/// Select tool: orchestrates hover, drag, click, keyboard, and rendering.
#[allow(clippy::too_many_arguments)]
pub(super) fn handle_select_tool(
    response: &egui::Response,
    painter: &egui::Painter,
    editor: &mut EditorState,
    sprite: &mut Sprite,
    project: &Project,
    canvas_rect: egui::Rect,
    theme_mode: crate::model::project::Theme,
    history: &mut History,
) {
    let canvas_center = canvas_rect.center();
    let threshold = super::canvas::HIT_TEST_THRESHOLD / editor.viewport.zoom;
    let handle_radius = HANDLE_HIT_RADIUS;

    handle_select_hover(response, editor, sprite, canvas_rect, canvas_center, threshold, handle_radius);
    handle_select_drag_start(response, editor, sprite, history, canvas_rect, canvas_center, threshold, handle_radius);
    handle_select_drag_update(response, editor, sprite, project, canvas_center, canvas_rect);
    handle_select_drag_end(response, editor, sprite, project, history, canvas_center, canvas_rect);
    handle_select_click(response, editor, sprite, canvas_center, threshold);
    handle_select_double_click(response, editor, sprite, canvas_center, threshold);
    handle_select_keyboard(response, editor, sprite, project, history);
    render_select_overlays(response, painter, editor, sprite, canvas_rect, theme_mode);
}

/// Returns true when exactly one element is selected (vertex-edit sub-mode).
fn is_vertex_edit_mode(editor: &EditorState) -> bool {
    editor.selection.selected_ids.len() == 1
}

/// Find the selected element by ID (immutable).
fn find_selected_element<'a>(sprite: &'a Sprite, id: &str) -> Option<&'a StrokeElement> {
    sprite.layers.iter()
        .flat_map(|l| &l.elements)
        .find(|e| e.id == id)
}

/// Hover hit-testing: update cursor and hover_element_id when not dragging.
fn handle_select_hover(
    response: &egui::Response,
    editor: &mut EditorState,
    sprite: &Sprite,
    canvas_rect: egui::Rect,
    canvas_center: egui::Pos2,
    threshold: f32,
    handle_radius: f32,
) {
    if editor.select_drag.is_some() {
        return;
    }
    editor.hover_vertex = None;

    if let Some(hover_pos) = response.hover_pos() {
        // In vertex edit mode, check vertex/handle hits first
        if is_vertex_edit_mode(editor) {
            let element_id = &editor.selection.selected_ids[0];
            if let Some(element) = find_selected_element(sprite, element_id) {
                // Check CP handles first (only if a vertex is selected and element is curve_mode)
                if let Some(ref sel_vid) = editor.selected_vertex_id
                    && let Some((vid, is_cp1)) = hit_test::hit_test_handle(
                        hover_pos, element, sel_vid, &editor.viewport,
                        canvas_center, canvas_render::VERTEX_HIT_RADIUS,
                    )
                {
                    editor.hover_vertex = Some(VertexHover::Handle { vertex_id: vid, is_cp1 });
                    response.ctx.set_cursor_icon(egui::CursorIcon::Grab);
                    editor.hover_element_id = None;
                    return;
                }
                // Check vertex dots
                if let Some(vid) = hit_test::hit_test_vertex(
                    hover_pos, element, &editor.viewport,
                    canvas_center, canvas_render::VERTEX_HIT_RADIUS,
                ) {
                    editor.hover_vertex = Some(VertexHover::Vertex { vertex_id: vid });
                    response.ctx.set_cursor_icon(egui::CursorIcon::Grab);
                    editor.hover_element_id = None;
                    return;
                }
            }
        }

        // Standard transform handle hit test
        let handle_hit = canvas_render::hit_test_handles(
            hover_pos, sprite, &editor.selection.selected_ids,
            &editor.viewport, canvas_rect, handle_radius,
        );
        if let Some(handle) = handle_hit {
            response.ctx.set_cursor_icon(canvas_render::cursor_for_handle(handle));
            editor.hover_element_id = None;
        } else {
            let world_pos = editor.viewport.screen_to_world(hover_pos, canvas_center);
            editor.hover_element_id = hit_test::hit_test_elements(world_pos, sprite, threshold, editor.layer.solo_layer_id.as_deref());
            if editor.hover_element_id.is_some() {
                response.ctx.set_cursor_icon(egui::CursorIcon::Grab);
            } else {
                response.ctx.set_cursor_icon(egui::CursorIcon::Default);
            }
        }
    } else {
        editor.hover_element_id = None;
    }
}

/// Begin a drag operation: handle drag, element move, or marquee.
#[allow(clippy::too_many_arguments)]
fn handle_select_drag_start(
    response: &egui::Response,
    editor: &mut EditorState,
    sprite: &mut Sprite,
    history: &mut History,
    canvas_rect: egui::Rect,
    canvas_center: egui::Pos2,
    threshold: f32,
    handle_radius: f32,
) {
    if !response.drag_started_by(egui::PointerButton::Primary) {
        return;
    }
    let Some(start_screen) = response.interact_pointer_pos() else { return };
    let start_world = editor.viewport.screen_to_world(start_screen, canvas_center);

    // In vertex edit mode, check for vertex/handle drag first
    if is_vertex_edit_mode(editor) {
        let element_id = editor.selection.selected_ids[0].clone();
        if let Some(element) = find_selected_element(sprite, &element_id) {
            // Check CP handle drag (only if a vertex is selected)
            if let Some(ref sel_vid) = editor.selected_vertex_id.clone()
                && let Some((vid, is_cp1)) = hit_test::hit_test_handle(
                    start_screen, element, sel_vid, &editor.viewport,
                    canvas_center, canvas_render::VERTEX_HIT_RADIUS,
                )
            {
                let vertex = element.vertices.iter().find(|v| v.id == vid).unwrap();
                let initial_local_pos = if is_cp1 {
                    vertex.cp1.unwrap_or(vertex.pos)
                } else {
                    vertex.cp2.unwrap_or(vertex.pos)
                };
                history.begin_drag("Move handle".into(), sprite.clone());
                editor.select_drag = Some(SelectDragKind::HandleMove {
                    element_id,
                    vertex_id: vid,
                    is_cp1,
                    start_world,
                    initial_local_pos,
                });
                return;
            }
            // Check vertex drag
            if let Some(vid) = hit_test::hit_test_vertex(
                start_screen, element, &editor.viewport,
                canvas_center, canvas_render::VERTEX_HIT_RADIUS,
            ) {
                let vertex = element.vertices.iter().find(|v| v.id == vid).unwrap();
                let initial_local_pos = vertex.pos;
                editor.selected_vertex_id = Some(vid.clone());
                history.begin_drag("Move vertex".into(), sprite.clone());
                editor.select_drag = Some(SelectDragKind::VertexMove {
                    element_id,
                    vertex_id: vid,
                    start_world,
                    initial_local_pos,
                });
                return;
            }
        }
    }

    let handle_hit = canvas_render::hit_test_handles(
        start_screen, sprite, &editor.selection.selected_ids,
        &editor.viewport, canvas_rect, handle_radius,
    );

    if let Some(handle) = handle_hit {
        history.begin_drag(
            if handle == HandleKind::Rotate { "Rotate elements" } else { "Scale elements" }.into(),
            sprite.clone(),
        );

        if handle == HandleKind::Rotate {
            if let Some((bmin, bmax)) = transform::selection_bounds(sprite, &editor.selection.selected_ids) {
                let pivot = (bmin + bmax) * 0.5;
                let start_angle = (start_world.y - pivot.y).atan2(start_world.x - pivot.x);
                let initial_rotations: Vec<_> = canvas_transform::collect_selected_field(sprite, &editor.selection.selected_ids, |e| (e.id.clone(), e.rotation));
                let initial_positions: Vec<_> = canvas_transform::collect_selected_field(sprite, &editor.selection.selected_ids, |e| (e.id.clone(), e.position));
                editor.select_drag = Some(SelectDragKind::Rotate {
                    pivot,
                    start_angle,
                    initial_rotations,
                    initial_positions,
                });
            }
        } else if let Some((bmin, bmax)) = transform::selection_bounds(sprite, &editor.selection.selected_ids) {
            let anchor = canvas_transform::scale_anchor(handle, bmin, bmax);
            let initial_scales: Vec<_> = canvas_transform::collect_selected_field(sprite, &editor.selection.selected_ids, |e| (e.id.clone(), e.scale));
            let initial_positions: Vec<_> = canvas_transform::collect_selected_field(sprite, &editor.selection.selected_ids, |e| (e.id.clone(), e.position));
            editor.select_drag = Some(SelectDragKind::Scale {
                handle,
                initial_bounds: (bmin, bmax),
                initial_scales,
                initial_positions,
                anchor,
            });
        }
    } else {
        let hit = hit_test::hit_test_elements(start_world, sprite, threshold, editor.layer.solo_layer_id.as_deref());
        if let Some(hit_id) = hit {
            if !editor.selection.is_selected(&hit_id) {
                let shift = response.ctx.input(|i| i.modifiers.shift);
                if shift {
                    editor.selection.toggle(&hit_id);
                } else {
                    editor.selection.select_single(hit_id);
                }
            }
            history.begin_drag("Move elements".into(), sprite.clone());
            editor.select_drag = Some(SelectDragKind::Move {
                start_world,
                last_snapped_delta: Vec2::ZERO,
            });
        } else {
            editor.select_drag = Some(SelectDragKind::Marquee {
                start_screen,
                start_world,
            });
        }
    }
}

/// Update an in-progress drag: apply move/scale/rotate transforms.
fn handle_select_drag_update(
    response: &egui::Response,
    editor: &mut EditorState,
    sprite: &mut Sprite,
    project: &Project,
    canvas_center: egui::Pos2,
    _canvas_rect: egui::Rect,
) {
    if !response.dragged_by(egui::PointerButton::Primary) {
        return;
    }
    let Some(current_screen) = response.interact_pointer_pos() else { return };
    let current_world = editor.viewport.screen_to_world(current_screen, canvas_center);

    match &mut editor.select_drag {
        Some(SelectDragKind::Move { start_world, last_snapped_delta }) => {
            response.ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
            let raw_delta = current_world - *start_world;
            let snapped_delta = snap::snap_to_grid(
                raw_delta,
                project.editor_preferences.grid_size,
                project.editor_preferences.grid_mode,
            );

            if (snapped_delta.x - last_snapped_delta.x).abs() > SNAP_EPSILON
                || (snapped_delta.y - last_snapped_delta.y).abs() > SNAP_EPSILON
            {
                let adjust = snapped_delta - *last_snapped_delta;
                let selected = editor.selection.selected_ids.clone();
                for layer in sprite.layers.iter_mut() {
                    for element in layer.elements.iter_mut() {
                        if selected.iter().any(|id| id == &element.id) {
                            element.position += adjust;
                        }
                    }
                }
                *last_snapped_delta = snapped_delta;
            }
        }
        Some(SelectDragKind::Scale { handle, initial_bounds, initial_scales, initial_positions, anchor }) => {
            response.ctx.set_cursor_icon(canvas_render::cursor_for_handle(*handle));
            let (bmin, bmax) = *initial_bounds;
            let (sx, sy) = canvas_transform::compute_scale_factors(*handle, current_world, *anchor, bmin, bmax);

            let selected = editor.selection.selected_ids.clone();
            for layer in sprite.layers.iter_mut() {
                for element in layer.elements.iter_mut() {
                    if let Some(idx) = selected.iter().position(|id| id == &element.id) {
                        let (_, init_scale) = initial_scales[idx];
                        let (_, init_pos) = initial_positions[idx];
                        element.scale = Vec2::new(init_scale.x * sx, init_scale.y * sy);
                        let offset_from_anchor = init_pos - *anchor;
                        element.position = *anchor + Vec2::new(offset_from_anchor.x * sx, offset_from_anchor.y * sy);
                    }
                }
            }
        }
        Some(SelectDragKind::Rotate { pivot, start_angle, initial_rotations, initial_positions }) => {
            response.ctx.set_cursor_icon(egui::CursorIcon::Alias);
            let current_angle = (current_world.y - pivot.y).atan2(current_world.x - pivot.x);
            let mut delta_angle = current_angle - *start_angle;

            if response.ctx.input(|i| i.modifiers.shift) {
                delta_angle = (delta_angle / ROTATION_SNAP_STEP).round() * ROTATION_SNAP_STEP;
            }

            let (sin, cos) = delta_angle.sin_cos();
            let selected = editor.selection.selected_ids.clone();
            for layer in sprite.layers.iter_mut() {
                for element in layer.elements.iter_mut() {
                    if let Some(idx) = selected.iter().position(|id| id == &element.id) {
                        let (_, init_rot) = initial_rotations[idx];
                        let (_, init_pos) = initial_positions[idx];
                        element.rotation = init_rot + delta_angle;
                        let offset = init_pos - *pivot;
                        let rotated = Vec2::new(
                            offset.x * cos - offset.y * sin,
                            offset.x * sin + offset.y * cos,
                        );
                        element.position = *pivot + rotated;
                    }
                }
            }
        }
        Some(SelectDragKind::VertexMove { element_id, vertex_id, start_world, initial_local_pos }) => {
            response.ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
            let element_id = element_id.clone();
            let vertex_id = vertex_id.clone();
            let start_world = *start_world;
            let initial_local_pos = *initial_local_pos;

            if let Some(element) = find_selected_element(sprite, &element_id) {
                let initial_world = transform::transform_point(
                    initial_local_pos, element.origin, element.position, element.rotation, element.scale,
                );
                let delta = current_world - start_world;
                let target_world = initial_world + delta;
                let snapped = snap::snap_to_grid(
                    target_world,
                    project.editor_preferences.grid_size,
                    project.editor_preferences.grid_mode,
                );
                let origin = element.origin;
                let position = element.position;
                let rotation = element.rotation;
                let scale = element.scale;
                let closed = element.closed;
                let curve_mode = element.curve_mode;
                let new_local = transform::inverse_transform_point(
                    snapped, origin, position, rotation, scale,
                );

                if let Some((elem, idx)) = transform::find_element_vertex_mut(sprite, &element_id, &vertex_id) {
                    elem.vertices[idx].pos = new_local;
                    math::recompute_auto_curves(
                        &mut elem.vertices, closed, curve_mode,
                        project.min_corner_radius,
                    );
                }
            }
        }
        Some(SelectDragKind::HandleMove { element_id, vertex_id, is_cp1, start_world, initial_local_pos }) => {
            response.ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
            let element_id = element_id.clone();
            let vertex_id = vertex_id.clone();
            let is_cp1 = *is_cp1;
            let start_world = *start_world;
            let initial_local_pos = *initial_local_pos;

            if let Some(element) = find_selected_element(sprite, &element_id) {
                let initial_world = transform::transform_point(
                    initial_local_pos, element.origin, element.position, element.rotation, element.scale,
                );
                let delta = current_world - start_world;
                let target_world = initial_world + delta;
                // No grid snap for handle moves
                let origin = element.origin;
                let position = element.position;
                let rotation = element.rotation;
                let scale = element.scale;
                let closed = element.closed;
                let curve_mode = element.curve_mode;
                let new_local = transform::inverse_transform_point(
                    target_world, origin, position, rotation, scale,
                );

                if let Some((elem, idx)) = transform::find_element_vertex_mut(sprite, &element_id, &vertex_id) {
                    if is_cp1 {
                        elem.vertices[idx].cp1 = Some(new_local);
                    } else {
                        elem.vertices[idx].cp2 = Some(new_local);
                    }
                    elem.vertices[idx].manual_handles = true;
                    math::recompute_auto_curves(
                        &mut elem.vertices, closed, curve_mode,
                        project.min_corner_radius,
                    );
                }
            }
        }
        Some(SelectDragKind::Marquee { .. }) => {
            response.ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
        }
        _ => {}
    }
}

/// Finalize a drag: commit undo, resolve marquee selection.
#[allow(clippy::too_many_arguments)]
fn handle_select_drag_end(
    response: &egui::Response,
    editor: &mut EditorState,
    sprite: &mut Sprite,
    project: &Project,
    history: &mut History,
    canvas_center: egui::Pos2,
    _canvas_rect: egui::Rect,
) {
    if !response.drag_stopped_by(egui::PointerButton::Primary) {
        return;
    }
    match editor.select_drag.take() {
        Some(SelectDragKind::Move { .. }) => {
            history.end_drag(sprite.clone());
        }
        Some(SelectDragKind::Scale { .. })
        | Some(SelectDragKind::Rotate { .. }) => {
            canvas_transform::bake_and_snap_selected(sprite, &editor.selection.selected_ids, project);
            history.end_drag(sprite.clone());
        }
        Some(SelectDragKind::Marquee { start_world, .. }) => {
            if let Some(end_screen) = response.interact_pointer_pos() {
                let end_world = editor.viewport.screen_to_world(end_screen, canvas_center);
                let rect_min = start_world.min(end_world);
                let rect_max = start_world.max(end_world);
                let ids = transform::elements_in_rect(sprite, rect_min, rect_max, editor.layer.solo_layer_id.as_deref());
                let shift = response.ctx.input(|i| i.modifiers.shift);
                if shift {
                    for id in ids {
                        if !editor.selection.is_selected(&id) {
                            editor.selection.selected_ids.push(id);
                        }
                    }
                } else {
                    editor.selection.select_all(ids);
                }
            }
        }
        Some(SelectDragKind::VertexMove { .. })
        | Some(SelectDragKind::HandleMove { .. }) => {
            history.end_drag(sprite.clone());
        }
        None => {}
    }
}

/// Click-to-select, alt-click stack popup, right-click clear.
fn handle_select_click(
    response: &egui::Response,
    editor: &mut EditorState,
    sprite: &Sprite,
    canvas_center: egui::Pos2,
    threshold: f32,
) {
    if response.clicked() && editor.select_drag.is_none() {
        let shift = response.ctx.input(|i| i.modifiers.shift);
        let alt = response.ctx.input(|i| i.modifiers.alt);

        // In vertex edit mode, check vertex click first
        if is_vertex_edit_mode(editor) && !alt {
            let mut vertex_hit = false;
            if let Some(click_pos) = response.interact_pointer_pos()
                && let Some(element) = find_selected_element(sprite, &editor.selection.selected_ids[0])
                && let Some(vid) = hit_test::hit_test_vertex(
                    click_pos, element, &editor.viewport,
                    canvas_center, canvas_render::VERTEX_HIT_RADIUS,
                )
            {
                editor.selected_vertex_id = Some(vid);
                vertex_hit = true;
            }
            if vertex_hit {
                return;
            }
            // No vertex hit — clear vertex selection, fall through to element handling
            editor.selected_vertex_id = None;
        }

        if alt {
            if let Some(click_pos) = response.interact_pointer_pos() {
                let world_pos = editor.viewport.screen_to_world(click_pos, canvas_center);
                let all_hits = hit_test::hit_test_all_elements(world_pos, sprite, threshold, editor.layer.solo_layer_id.as_deref());
                if all_hits.len() >= 2 {
                    let entries: Vec<_> = all_hits.into_iter().map(|(id, name, color_idx)| {
                        crate::state::editor::StackEntry { element_id: id, display_name: name, stroke_color_index: color_idx }
                    }).collect();
                    editor.selection_stack_popup = Some(crate::state::editor::SelectionStackPopup {
                        screen_pos: click_pos,
                        entries,
                    });
                } else if let Some((id, _, _)) = all_hits.into_iter().next() {
                    editor.clear_vertex_selection();
                    editor.selection.select_single(id);
                }
            }
        } else if let Some(hover_id) = editor.hover_element_id.clone() {
            editor.selection_stack_popup = None;
            editor.clear_vertex_selection();
            if shift {
                editor.selection.toggle(&hover_id);
            } else {
                editor.selection.select_single(hover_id);
            }
        } else if !shift {
            editor.clear_vertex_selection();
            editor.selection.clear();
            editor.selection_stack_popup = None;
        }
    }

    if response.secondary_clicked() {
        editor.selection.clear();
        editor.selection_stack_popup = None;
    }
}

/// Double-click on canvas: element → solo its layer, background → clear solo.
fn handle_select_double_click(
    response: &egui::Response,
    editor: &mut EditorState,
    sprite: &Sprite,
    canvas_center: egui::Pos2,
    threshold: f32,
) {
    if !response.double_clicked() {
        return;
    }
    if let Some(click_pos) = response.interact_pointer_pos() {
        let world_pos = editor.viewport.screen_to_world(click_pos, canvas_center);
        // Hit test without solo filtering so we can solo any visible layer
        if let Some(hit_id) = hit_test::hit_test_elements(world_pos, sprite, threshold, None) {
            // Find which layer contains this element
            if let Some(layer) = sprite.layers.iter().find(|l| l.elements.iter().any(|e| e.id == hit_id)) {
                if editor.layer.solo_layer_id.as_deref() == Some(&layer.id) {
                    // Already soloed this layer — clear solo
                    editor.layer.solo_layer_id = None;
                } else {
                    editor.layer.solo_layer_id = Some(layer.id.clone());
                }
            }
        } else {
            // Double-click on empty canvas — clear solo
            editor.layer.solo_layer_id = None;
        }
    }
}

/// Keyboard shortcuts: Escape, Ctrl+A, Delete, C toggle curve mode.
fn handle_select_keyboard(
    response: &egui::Response,
    editor: &mut EditorState,
    sprite: &mut Sprite,
    project: &Project,
    history: &mut History,
) {
    if response.ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        if editor.selected_vertex_id.is_some() {
            // First Escape: just clear vertex selection
            editor.clear_vertex_selection();
        } else {
            editor.selection.clear();
            editor.selection_stack_popup = None;
        }
    }

    if response.ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::A)) {
        let solo = editor.layer.solo_layer_id.as_deref();
        let mut ids = Vec::new();
        for layer in &sprite.layers {
            if !layer.visible || layer.locked {
                continue;
            }
            if let Some(solo_id) = solo
                && layer.id != solo_id {
                    continue;
                }
            for element in &layer.elements {
                ids.push(element.id.clone());
            }
        }
        editor.clear_vertex_selection();
        editor.selection.select_all(ids);
    }

    let text_has_focus = response.ctx.wants_keyboard_input();
    let delete_pressed = !text_has_focus && response.ctx.input(|i| {
        i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)
    });
    if delete_pressed {
        // Vertex delete mode: delete the selected vertex
        if let Some(ref vid) = editor.selected_vertex_id.clone() {
            if is_vertex_edit_mode(editor) {
                let element_id = editor.selection.selected_ids[0].clone();
                let before = sprite.clone();
                let mut should_delete_element = false;
                for layer in sprite.layers.iter_mut() {
                    for element in layer.elements.iter_mut() {
                        if element.id == element_id {
                            element.vertices.retain(|v| v.id != *vid);
                            if element.vertices.len() < 2 {
                                should_delete_element = true;
                            } else {
                                math::recompute_auto_curves(
                                    &mut element.vertices,
                                    element.closed,
                                    element.curve_mode,
                                    project.min_corner_radius,
                                );
                            }
                        }
                    }
                }
                if should_delete_element {
                    for layer in sprite.layers.iter_mut() {
                        layer.elements.retain(|e| e.id != element_id);
                    }
                    editor.selection.clear();
                }
                editor.clear_vertex_selection();
                history.push("Delete vertex".into(), before, sprite.clone());
            }
        } else if !editor.selection.is_empty() {
            let before = sprite.clone();
            let selected = editor.selection.selected_ids.clone();
            for layer in sprite.layers.iter_mut() {
                layer.elements.retain(|e| !selected.iter().any(|id| id == &e.id));
            }
            history.push("Delete elements".into(), before, sprite.clone());
            editor.selection.clear();
        }
    }

    // R key: reset manual handles on selected vertex
    if !text_has_focus && response.ctx.input(|i| i.key_pressed(egui::Key::R) && !i.modifiers.ctrl)
        && let Some(ref vid) = editor.selected_vertex_id.clone()
        && is_vertex_edit_mode(editor)
    {
        let element_id = editor.selection.selected_ids[0].clone();
        let before = sprite.clone();
        if let Some((elem, idx)) = transform::find_element_vertex_mut(sprite, &element_id, vid) {
            elem.vertices[idx].manual_handles = false;
            let closed = elem.closed;
            let curve_mode = elem.curve_mode;
            math::recompute_auto_curves(
                &mut elem.vertices, closed, curve_mode,
                project.min_corner_radius,
            );
            history.push("Reset handles".into(), before, sprite.clone());
        }
    }

    if !text_has_focus && response.ctx.input(|i| i.key_pressed(egui::Key::C) && !i.modifiers.ctrl) && !editor.selection.is_empty() {
        let before = sprite.clone();
        let selected = editor.selection.selected_ids.clone();
        let any_curved = sprite.layers.iter()
            .flat_map(|l| &l.elements)
            .filter(|e| selected.iter().any(|id| id == &e.id))
            .any(|e| e.curve_mode);
        let target_mode = !any_curved;
        for layer in sprite.layers.iter_mut() {
            for element in layer.elements.iter_mut() {
                if selected.iter().any(|id| id == &element.id) {
                    element.curve_mode = target_mode;
                    math::recompute_auto_curves(
                        &mut element.vertices,
                        element.closed,
                        element.curve_mode,
                        project.min_corner_radius,
                    );
                }
            }
        }
        history.push("Toggle curve mode".into(), before, sprite.clone());
    }
}

/// Render selection highlights, handles, hover highlight, and marquee.
fn render_select_overlays(
    response: &egui::Response,
    painter: &egui::Painter,
    editor: &EditorState,
    sprite: &Sprite,
    canvas_rect: egui::Rect,
    theme_mode: crate::model::project::Theme,
) {
    canvas_render::render_selection_highlights(
        painter,
        sprite,
        &editor.selection.selected_ids,
        &editor.viewport,
        canvas_rect,
        theme_mode,
    );

    let in_vertex_mode = is_vertex_edit_mode(editor);

    // Show transform handles only when not in vertex mode or no vertex is selected
    let show_handles = !in_vertex_mode || editor.selected_vertex_id.is_none();
    let show_handles = show_handles && matches!(&editor.select_drag, None | Some(SelectDragKind::Scale { .. }) | Some(SelectDragKind::Rotate { .. }));
    if show_handles {
        canvas_render::render_transform_handles(
            painter,
            sprite,
            &editor.selection.selected_ids,
            &editor.viewport,
            canvas_rect,
            theme_mode,
        );
    }

    // Vertex edit mode overlays
    if in_vertex_mode {
        let element_id = &editor.selection.selected_ids[0];
        if let Some(element) = find_selected_element(sprite, element_id) {
            let canvas_center = canvas_rect.center();
            canvas_render::render_vertex_dots(
                painter,
                element,
                editor.selected_vertex_id.as_deref(),
                editor.hover_vertex.as_ref(),
                &editor.viewport,
                canvas_center,
                theme_mode,
            );
            if let Some(ref sel_vid) = editor.selected_vertex_id
                && element.curve_mode
            {
                canvas_render::render_cp_handles(
                    painter,
                    element,
                    sel_vid,
                    editor.hover_vertex.as_ref(),
                    &editor.viewport,
                    canvas_center,
                    theme_mode,
                );
            }
        }
    }

    if editor.select_drag.is_none()
        && let Some(ref hover_id) = editor.hover_element_id
            && !editor.selection.is_selected(hover_id) {
                canvas_render::render_hover_highlight(
                    painter,
                    sprite,
                    hover_id,
                    &editor.viewport,
                    canvas_rect,
                    theme_mode,
                );
            }

    if let Some(SelectDragKind::Marquee { start_screen, .. }) = &editor.select_drag
        && let Some(current_screen) = response.interact_pointer_pos() {
            let marquee_color = theme::marquee_color(theme_mode);
            let stroke = egui::Stroke::new(1.0, marquee_color);
            let min_p = egui::Pos2::new(start_screen.x.min(current_screen.x), start_screen.y.min(current_screen.y));
            let max_p = egui::Pos2::new(start_screen.x.max(current_screen.x), start_screen.y.max(current_screen.y));
            canvas_render::draw_dashed_line(painter, min_p, egui::Pos2::new(max_p.x, min_p.y), stroke, MARQUEE_DASH, MARQUEE_GAP);
            canvas_render::draw_dashed_line(painter, egui::Pos2::new(max_p.x, min_p.y), max_p, stroke, MARQUEE_DASH, MARQUEE_GAP);
            canvas_render::draw_dashed_line(painter, max_p, egui::Pos2::new(min_p.x, max_p.y), stroke, MARQUEE_DASH, MARQUEE_GAP);
            canvas_render::draw_dashed_line(painter, egui::Pos2::new(min_p.x, max_p.y), min_p, stroke, MARQUEE_DASH, MARQUEE_GAP);
        }
}
