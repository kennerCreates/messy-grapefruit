use crate::action::AppAction;
use crate::engine::{hit_test, snap, transform};
use crate::math;
use crate::model::sprite::StrokeElement;
use crate::model::project::Project;
use crate::model::sprite::Sprite;
use crate::model::vec2::Vec2;
use crate::state::editor::{EditorState, HandleKind, SelectDragKind, SelectionStackPopup, StackEntry, ToolKind};
use crate::state::history::History;
use crate::theme;

use super::{canvas_input, canvas_render, grid};

pub fn show_canvas(
    ui: &mut egui::Ui,
    editor: &mut EditorState,
    sprite: &mut Sprite,
    project: &Project,
    history: &mut History,
) -> Vec<AppAction> {
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

    // --- Shared: viewport input (pan, zoom, flip) ---
    canvas_input::handle_viewport_input(editor, project, canvas_rect, ui);

    // Handle F key or toolbar button = zoom to fit
    if (ui.input(|i| i.key_pressed(egui::Key::F)) && !ui.input(|i| i.modifiers.ctrl))
        || editor.zoom_to_fit_requested
    {
        editor.zoom_to_fit_requested = false;
        zoom_to_fit(editor, sprite, canvas_rect);
    }

    // --- Shared: render grid, boundary, elements ---
    grid::render_grid(
        &painter,
        &editor.viewport,
        &project.editor_preferences,
        canvas_rect,
        theme_mode,
    );

    canvas_render::render_canvas_boundary(
        &painter,
        &editor.viewport,
        sprite.canvas_width,
        sprite.canvas_height,
        canvas_rect,
        theme_mode,
    );

    canvas_render::render_elements(
        &painter,
        &editor.viewport,
        sprite,
        &project.palette,
        canvas_rect,
    );

    // --- Tool-specific: input, hit testing, preview ---
    match editor.tool {
        ToolKind::Select => {
            handle_select_tool(
                &response,
                &painter,
                editor,
                sprite,
                project,
                canvas_rect,
                theme_mode,
                history,
            );
        }
        ToolKind::Line => {
            handle_line_tool(
                &response,
                &painter,
                editor,
                sprite,
                project,
                canvas_rect,
                theme_mode,
                &mut actions,
            );
        }
    }

    // --- Selection stack popup (Alt+click) ---
    render_selection_stack_popup(ui, editor, project);

    actions
}

/// Render the selection stack popup as a floating area.
fn render_selection_stack_popup(
    ui: &mut egui::Ui,
    editor: &mut EditorState,
    project: &Project,
) {
    let popup = match &editor.selection_stack_popup {
        Some(p) => p.clone(),
        None => return,
    };

    let mut close = false;
    let mut selected_id: Option<String> = None;

    let panel_bg = theme::floating_panel_color(project.editor_preferences.theme);
    let area_resp = egui::Area::new(egui::Id::new("selection_stack_popup"))
        .fixed_pos(popup.screen_pos)
        .constrain(true)
        .show(ui.ctx(), |ui| {
            egui::Frame::NONE
                .fill(panel_bg)
                .corner_radius(6.0)
                .inner_margin(4.0)
                .show(ui, |ui| {
                    for entry in &popup.entries {
                        let color = project.palette.get_color(entry.stroke_color_index).to_color32();
                        ui.horizontal(|ui| {
                            // Color swatch
                            let (rect, _) = ui.allocate_exact_size(
                                egui::Vec2::splat(12.0),
                                egui::Sense::hover(),
                            );
                            ui.painter().rect_filled(rect, 2.0, color);

                            if ui.selectable_label(false, &entry.display_name).clicked() {
                                selected_id = Some(entry.element_id.clone());
                            }
                        });
                    }
                });
        });

    // Close popup if clicked outside
    if area_resp.response.clicked_elsewhere() {
        close = true;
    }

    if let Some(id) = selected_id {
        editor.selection.select_single(id);
        close = true;
    }
    if close {
        editor.selection_stack_popup = None;
    }
}

/// Select tool: hit testing, hover highlight, click/drag selection, move, scale, rotate, marquee.
#[allow(clippy::too_many_arguments)]
fn handle_select_tool(
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
    let threshold = 8.0 / editor.viewport.zoom;
    let handle_radius = 7.0; // screen-pixel hit radius for handles

    // Hit testing for hover (only when not dragging)
    if editor.select_drag.is_none() {
        if let Some(hover_pos) = response.hover_pos() {
            // Check handle hover first
            let handle_hit = canvas_render::hit_test_handles(
                hover_pos, sprite, &editor.selection.selected_ids,
                &editor.viewport, canvas_rect, handle_radius,
            );
            if let Some(handle) = handle_hit {
                response.ctx.set_cursor_icon(canvas_render::cursor_for_handle(handle));
                editor.hover_element_id = None;
            } else {
                let world_pos = editor.viewport.screen_to_world(hover_pos, canvas_center);
                editor.hover_element_id = hit_test::hit_test_elements(world_pos, sprite, threshold);
                if editor.hover_element_id.is_some() {
                    response.ctx.set_cursor_icon(egui::CursorIcon::Grab);
                }
            }
        } else {
            editor.hover_element_id = None;
        }
    }

    // --- Drag start ---
    if response.drag_started_by(egui::PointerButton::Primary)
        && let Some(start_screen) = response.interact_pointer_pos() {
            let start_world = editor.viewport.screen_to_world(start_screen, canvas_center);

            // Check if we're dragging a handle
            let handle_hit = canvas_render::hit_test_handles(
                start_screen, sprite, &editor.selection.selected_ids,
                &editor.viewport, canvas_rect, handle_radius,
            );

            if let Some(handle) = handle_hit {
                // Start handle drag (scale or rotate)
                history.begin_drag(
                    if handle == HandleKind::Rotate { "Rotate elements" } else { "Scale elements" }.into(),
                    sprite.clone(),
                );

                if handle == HandleKind::Rotate {
                    // Rotation drag
                    if let Some((bmin, bmax)) = transform::selection_bounds(sprite, &editor.selection.selected_ids) {
                        let pivot = (bmin + bmax) * 0.5;
                        let start_angle = (start_world.y - pivot.y).atan2(start_world.x - pivot.x);
                        let initial_rotations: Vec<_> = collect_selected_field(sprite, &editor.selection.selected_ids, |e| (e.id.clone(), e.rotation));
                        let initial_positions: Vec<_> = collect_selected_field(sprite, &editor.selection.selected_ids, |e| (e.id.clone(), e.position));
                        editor.select_drag = Some(SelectDragKind::Rotate {
                            pivot,
                            start_angle,
                            initial_rotations,
                            initial_positions,
                        });
                    }
                } else {
                    // Scale drag
                    if let Some((bmin, bmax)) = transform::selection_bounds(sprite, &editor.selection.selected_ids) {
                        let anchor = scale_anchor(handle, bmin, bmax);
                        let initial_scales: Vec<_> = collect_selected_field(sprite, &editor.selection.selected_ids, |e| (e.id.clone(), e.scale));
                        let initial_positions: Vec<_> = collect_selected_field(sprite, &editor.selection.selected_ids, |e| (e.id.clone(), e.position));
                        editor.select_drag = Some(SelectDragKind::Scale {
                            handle,
                            initial_bounds: (bmin, bmax),
                            initial_scales,
                            initial_positions,
                            anchor,
                        });
                    }
                }
            } else {
                let hit = hit_test::hit_test_elements(start_world, sprite, threshold);
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

    // --- Drag update ---
    if response.dragged_by(egui::PointerButton::Primary)
        && let Some(current_screen) = response.interact_pointer_pos() {
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

                    if (snapped_delta.x - last_snapped_delta.x).abs() > 0.001
                        || (snapped_delta.y - last_snapped_delta.y).abs() > 0.001
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
                    let (sx, sy) = compute_scale_factors(*handle, current_world, *anchor, bmin, bmax);

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

                    // Shift = 15° snap
                    if response.ctx.input(|i| i.modifiers.shift) {
                        let snap_step = std::f32::consts::PI / 12.0; // 15 degrees
                        delta_angle = (delta_angle / snap_step).round() * snap_step;
                    }

                    let (sin, cos) = delta_angle.sin_cos();
                    let selected = editor.selection.selected_ids.clone();
                    for layer in sprite.layers.iter_mut() {
                        for element in layer.elements.iter_mut() {
                            if let Some(idx) = selected.iter().position(|id| id == &element.id) {
                                let (_, init_rot) = initial_rotations[idx];
                                let (_, init_pos) = initial_positions[idx];
                                element.rotation = init_rot + delta_angle;
                                // Rotate position around pivot
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
                _ => {}
            }
        }

    // --- Drag end ---
    if response.drag_stopped_by(egui::PointerButton::Primary) {
        match editor.select_drag.take() {
            Some(SelectDragKind::Move { .. }) => {
                history.end_drag(sprite.clone());
            }
            Some(SelectDragKind::Scale { .. })
            | Some(SelectDragKind::Rotate { .. }) => {
                // Bake transform into vertices and snap to grid
                bake_and_snap_selected(sprite, &editor.selection.selected_ids, project);
                history.end_drag(sprite.clone());
            }
            Some(SelectDragKind::Marquee { start_world, .. }) => {
                if let Some(end_screen) = response.interact_pointer_pos() {
                    let end_world = editor.viewport.screen_to_world(end_screen, canvas_center);
                    let rect_min = start_world.min(end_world);
                    let rect_max = start_world.max(end_world);
                    let ids = transform::elements_in_rect(sprite, rect_min, rect_max);
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
            None => {}
        }
    }

    // --- Click-to-select (no drag happened) ---
    if response.clicked() && editor.select_drag.is_none() {
        let shift = response.ctx.input(|i| i.modifiers.shift);
        let alt = response.ctx.input(|i| i.modifiers.alt);

        if alt {
            // Alt+click: show selection stack popup if 2+ elements under cursor
            if let Some(click_pos) = response.interact_pointer_pos() {
                let world_pos = editor.viewport.screen_to_world(click_pos, canvas_center);
                let all_hits = hit_test::hit_test_all_elements(world_pos, sprite, threshold);
                if all_hits.len() >= 2 {
                    let entries: Vec<StackEntry> = all_hits.into_iter().map(|(id, name, color_idx)| {
                        StackEntry { element_id: id, display_name: name, stroke_color_index: color_idx }
                    }).collect();
                    editor.selection_stack_popup = Some(SelectionStackPopup {
                        screen_pos: click_pos,
                        entries,
                    });
                } else if let Some((id, _, _)) = all_hits.into_iter().next() {
                    editor.selection.select_single(id);
                }
            }
        } else if let Some(ref hover_id) = editor.hover_element_id {
            editor.selection_stack_popup = None;
            if shift {
                editor.selection.toggle(hover_id);
            } else {
                editor.selection.select_single(hover_id.clone());
            }
        } else if !shift {
            editor.selection.clear();
            editor.selection_stack_popup = None;
        }
    }

    // --- Right-click clears selection ---
    if response.secondary_clicked() {
        editor.selection.clear();
        editor.selection_stack_popup = None;
    }

    // --- Escape clears selection and popup ---
    if response.ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        editor.selection.clear();
        editor.selection_stack_popup = None;
    }

    // --- Ctrl+A select all ---
    if response.ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::A)) {
        let mut ids = Vec::new();
        for layer in &sprite.layers {
            if !layer.visible || layer.locked {
                continue;
            }
            for element in &layer.elements {
                ids.push(element.id.clone());
            }
        }
        editor.selection.select_all(ids);
    }

    // --- Delete selected elements ---
    let delete_pressed = response.ctx.input(|i| {
        i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)
    });
    if delete_pressed && !editor.selection.is_empty() {
        let before = sprite.clone();
        let selected = editor.selection.selected_ids.clone();
        for layer in sprite.layers.iter_mut() {
            layer.elements.retain(|e| !selected.iter().any(|id| id == &e.id));
        }
        history.push("Delete elements".into(), before, sprite.clone());
        editor.selection.clear();
    }

    // --- C key toggles curve/straight mode on selected elements ---
    // If any selected element is curved → all become straight; otherwise all become curved.
    if response.ctx.input(|i| i.key_pressed(egui::Key::C) && !i.modifiers.ctrl) && !editor.selection.is_empty() {
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

    // --- Render selection highlights ---
    canvas_render::render_selection_highlights(
        painter,
        sprite,
        &editor.selection.selected_ids,
        &editor.viewport,
        canvas_rect,
        theme_mode,
    );

    // --- Render transform handles (only when not dragging or during handle drag) ---
    let show_handles = matches!(&editor.select_drag, None | Some(SelectDragKind::Scale { .. }) | Some(SelectDragKind::Rotate { .. }));
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

    // --- Render hover highlight (only for non-selected, not during drag) ---
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

    // --- Render marquee rectangle ---
    if let Some(SelectDragKind::Marquee { start_screen, .. }) = &editor.select_drag
        && let Some(current_screen) = response.interact_pointer_pos() {
            let marquee_color = theme::marquee_color(theme_mode);
            let stroke = egui::Stroke::new(1.0, marquee_color);
            let min_p = egui::Pos2::new(start_screen.x.min(current_screen.x), start_screen.y.min(current_screen.y));
            let max_p = egui::Pos2::new(start_screen.x.max(current_screen.x), start_screen.y.max(current_screen.y));
            canvas_render::draw_dashed_line(painter, min_p, egui::Pos2::new(max_p.x, min_p.y), stroke, 4.0, 3.0);
            canvas_render::draw_dashed_line(painter, egui::Pos2::new(max_p.x, min_p.y), max_p, stroke, 4.0, 3.0);
            canvas_render::draw_dashed_line(painter, max_p, egui::Pos2::new(min_p.x, max_p.y), stroke, 4.0, 3.0);
            canvas_render::draw_dashed_line(painter, egui::Pos2::new(min_p.x, max_p.y), min_p, stroke, 4.0, 3.0);
        }
}

/// Collect a field from all selected elements.
fn collect_selected_field<T>(
    sprite: &Sprite,
    selected_ids: &[String],
    f: impl Fn(&crate::model::sprite::StrokeElement) -> (String, T),
) -> Vec<(String, T)> {
    let mut result = Vec::new();
    for layer in &sprite.layers {
        for element in &layer.elements {
            if selected_ids.iter().any(|id| id == &element.id) {
                result.push(f(element));
            }
        }
    }
    // Ensure ordering matches selected_ids
    let mut ordered = Vec::with_capacity(selected_ids.len());
    for id in selected_ids {
        if let Some(pos) = result.iter().position(|(eid, _)| eid == id) {
            ordered.push(result.swap_remove(pos));
        }
    }
    ordered
}

/// After scale/rotate, bake the transform into vertex positions and snap to grid.
fn bake_and_snap_selected(sprite: &mut Sprite, selected_ids: &[String], project: &Project) {
    let grid_size = project.editor_preferences.grid_size;
    let grid_mode = project.editor_preferences.grid_mode;

    for layer in sprite.layers.iter_mut() {
        for element in layer.elements.iter_mut() {
            if selected_ids.iter().any(|id| id == &element.id) {
                bake_element_transform(element, grid_size, grid_mode, project.min_corner_radius);
            }
        }
    }
}

/// Bake an element's transform into its vertices, snap to grid, reset transform to identity.
fn bake_element_transform(element: &mut StrokeElement, grid_size: u32, grid_mode: crate::model::project::GridMode, corner_radius: f32) {
    for v in &mut element.vertices {
        v.pos = transform::transform_point(v.pos, element.origin, element.position, element.rotation, element.scale);
        v.pos = snap::snap_to_grid(v.pos, grid_size, grid_mode);
    }
    // Reset transform to identity
    element.position = Vec2::ZERO;
    element.rotation = 0.0;
    element.scale = Vec2::ONE;
    element.origin = Vec2::ZERO;
    // Recompute curves from snapped positions
    math::recompute_auto_curves(
        &mut element.vertices,
        element.closed,
        element.curve_mode,
        corner_radius,
    );
}

/// Compute the anchor point (opposite to the handle being dragged) on the selection AABB.
fn scale_anchor(handle: HandleKind, bmin: Vec2, bmax: Vec2) -> Vec2 {
    let mid_x = (bmin.x + bmax.x) * 0.5;
    let mid_y = (bmin.y + bmax.y) * 0.5;
    match handle {
        HandleKind::ScaleNW => bmax,
        HandleKind::ScaleN  => Vec2::new(mid_x, bmax.y),
        HandleKind::ScaleNE => Vec2::new(bmin.x, bmax.y),
        HandleKind::ScaleE  => Vec2::new(bmin.x, mid_y),
        HandleKind::ScaleSE => bmin,
        HandleKind::ScaleS  => Vec2::new(mid_x, bmin.y),
        HandleKind::ScaleSW => Vec2::new(bmax.x, bmin.y),
        HandleKind::ScaleW  => Vec2::new(bmax.x, mid_y),
        HandleKind::Rotate  => Vec2::new(mid_x, mid_y), // not used for scale
    }
}

/// Compute scale factors (sx, sy) from handle drag.
/// Uses the handle's original position to determine the direction from anchor.
fn compute_scale_factors(handle: HandleKind, cursor: Vec2, anchor: Vec2, bmin: Vec2, bmax: Vec2) -> (f32, f32) {
    let mid_x = (bmin.x + bmax.x) * 0.5;
    let mid_y = (bmin.y + bmax.y) * 0.5;

    // Original handle position in world space
    let handle_pos = match handle {
        HandleKind::ScaleNW => Vec2::new(bmin.x, bmin.y),
        HandleKind::ScaleN  => Vec2::new(mid_x, bmin.y),
        HandleKind::ScaleNE => Vec2::new(bmax.x, bmin.y),
        HandleKind::ScaleE  => Vec2::new(bmax.x, mid_y),
        HandleKind::ScaleSE => Vec2::new(bmax.x, bmax.y),
        HandleKind::ScaleS  => Vec2::new(mid_x, bmax.y),
        HandleKind::ScaleSW => Vec2::new(bmin.x, bmax.y),
        HandleKind::ScaleW  => Vec2::new(bmin.x, mid_y),
        HandleKind::Rotate  => return (1.0, 1.0),
    };

    let original_span = handle_pos - anchor;
    let current_span = cursor - anchor;

    let sx = if original_span.x.abs() > 0.001 { current_span.x / original_span.x } else { 1.0 };
    let sy = if original_span.y.abs() > 0.001 { current_span.y / original_span.y } else { 1.0 };

    match handle {
        HandleKind::ScaleN | HandleKind::ScaleS => (1.0, sy),
        HandleKind::ScaleE | HandleKind::ScaleW => (sx, 1.0),
        _ => (sx, sy),
    }
}

/// Line tool: hit testing (when not drawing), hover highlight, input, preview.
#[allow(clippy::too_many_arguments)]
fn handle_line_tool(
    response: &egui::Response,
    painter: &egui::Painter,
    editor: &mut EditorState,
    sprite: &Sprite,
    project: &Project,
    canvas_rect: egui::Rect,
    theme_mode: crate::model::project::Theme,
    actions: &mut Vec<AppAction>,
) {
    // Hit testing for hover highlight (only when not mid-draw)
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
            painter,
            sprite,
            hover_id,
            &editor.viewport,
            canvas_rect,
            theme_mode,
        );
    }

    // Handle line tool input
    let (line_action, merge_target) = canvas_input::handle_line_tool_input(
        response,
        editor,
        sprite,
        project,
        canvas_rect,
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

        canvas_render::render_line_tool_preview(
            painter,
            &editor.line_tool.vertices,
            snap_pos,
            &project.palette,
            &editor.viewport,
            canvas_rect,
            editor.active_color_index,
            editor.active_stroke_width,
            theme_mode,
            merge_target,
            editor.line_tool.curve_mode,
        );
    }
}

fn zoom_to_fit(editor: &mut EditorState, sprite: &Sprite, canvas_rect: egui::Rect) {
    // If elements are selected, frame the selection instead of all content
    if !editor.selection.is_empty()
        && let Some((sel_min, sel_max)) = transform::selection_bounds(sprite, &editor.selection.selected_ids) {
            editor.viewport.zoom_to_fit(sel_min, sel_max, canvas_rect.size());
            return;
        }

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
                let world = transform::vertex_world_pos(vertex, element);
                min = min.min(world);
                max = max.max(world);
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
