use crate::action::AppAction;
use crate::engine::{hit_test, transform};
use crate::model::sprite::Sprite;
use crate::model::project::Project;
use crate::model::vec2::Vec2;
use crate::state::editor::{EditorState, ToolKind};
use crate::state::history::History;
use crate::theme;

use super::{canvas_eraser, canvas_eyedropper, canvas_fill, canvas_input, canvas_refimage, canvas_render, canvas_select, grid};

/// Base hit-test threshold in world units (divided by zoom at use site).
pub(super) const HIT_TEST_THRESHOLD: f32 = 8.0;

pub fn show_canvas(
    ui: &mut egui::Ui,
    editor: &mut EditorState,
    sprite: &mut Sprite,
    project: &Project,
    history: &mut History,
    ref_image_textures: &std::collections::HashMap<String, egui::TextureHandle>,
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
    canvas_input::handle_viewport_input(editor, project, sprite, canvas_rect, ui);

    // Handle F key or toolbar button = zoom to fit
    if (!ui.ctx().wants_keyboard_input() && ui.input(|i| i.key_pressed(egui::Key::F)) && !ui.input(|i| i.modifiers.ctrl))
        || editor.zoom_to_fit_requested
    {
        editor.zoom_to_fit_requested = false;
        zoom_to_fit(editor, sprite, canvas_rect);
    }

    // --- Shared: render grid, boundary, reference images, elements ---
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

    canvas_render::render_background(
        &painter,
        &editor.viewport,
        sprite,
        &project.palette,
        canvas_rect,
    );

    // Reference images render behind all layers
    canvas_render::render_reference_images(
        &painter,
        &editor.viewport,
        sprite,
        ref_image_textures,
        canvas_rect,
        editor.selected_ref_image_id.as_deref(),
        theme_mode,
    );

    canvas_render::render_elements(
        &painter,
        &editor.viewport,
        sprite,
        &project.palette,
        canvas_rect,
        editor.layer.solo_layer_id.as_deref(),
        &project.hatch_patterns,
    );

    // --- Flow curve rendering (if editing a hatched element with flow curve) ---
    if let Some(ref fc_state) = editor.editing_flow_curve {
        // Find the element being edited
        if let Some(element) = sprite.layers.iter().flat_map(|l| &l.elements)
            .find(|e| e.id == fc_state.element_id)
        {
            canvas_render::render_flow_curve(
                &painter,
                element,
                &editor.viewport,
                canvas_rect,
                theme_mode,
                fc_state.dragging_cp_index,
            );
        }
    }

    // --- Symmetry axis rendering (always, if active) ---
    if editor.symmetry.active {
        canvas_render::render_symmetry_axis(
            &painter,
            &editor.viewport,
            &editor.symmetry,
            sprite,
            canvas_rect,
            theme_mode,
        );
    }

    // --- Reference image interaction (move/resize) — runs before tools ---
    let ref_image_consumed = canvas_refimage::handle_ref_image_input(
        &response,
        editor,
        sprite,
        history,
        canvas_rect,
        &mut actions,
    );

    // Render resize handle on selected ref image
    canvas_refimage::render_ref_image_handles(
        &painter,
        editor,
        sprite,
        canvas_rect,
        theme_mode,
    );

    // --- Flow curve interaction (drag control points) ---
    let flow_curve_consumed = handle_flow_curve_input(
        &response, editor, sprite, canvas_rect, &mut actions,
    );

    // --- Tool-specific: input, hit testing, preview ---
    if !ref_image_consumed && !flow_curve_consumed { match editor.tool {
        ToolKind::Select => {
            canvas_select::handle_select_tool(
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
        ToolKind::Fill => {
            canvas_fill::handle_fill_tool(
                &response,
                &painter,
                editor,
                sprite,
                canvas_rect,
                theme_mode,
                &mut actions,
            );
        }
        ToolKind::Eyedropper => {
            canvas_eyedropper::handle_eyedropper_tool(
                &response,
                &painter,
                editor,
                sprite,
                project,
                canvas_rect,
                theme_mode,
            );
        }
        ToolKind::Eraser => {
            canvas_eraser::handle_eraser_tool(
                &response,
                &painter,
                editor,
                sprite,
                canvas_rect,
                theme_mode,
                &mut actions,
            );
        }
    } }

    // --- Vertex snap indicator ---
    if let Some(snap_target) = editor.snap_vertex_target {
        canvas_render::render_vertex_snap_indicator(
            &painter,
            &editor.viewport,
            snap_target,
            canvas_rect,
            theme_mode,
        );
    }

    // --- Selection stack popup (Alt+click) ---
    render_selection_stack_popup(ui, editor, project);

    actions
}

/// Handle flow curve control point dragging.
/// Returns true if the interaction was consumed (to prevent tool dispatch).
fn handle_flow_curve_input(
    response: &egui::Response,
    editor: &mut EditorState,
    sprite: &mut Sprite,
    canvas_rect: egui::Rect,
    actions: &mut Vec<AppAction>,
) -> bool {
    // Only active when editing a flow curve
    let fc_state = match &editor.editing_flow_curve {
        Some(s) => s.clone(),
        None => return false,
    };

    let canvas_center = canvas_rect.center();

    // Find the element
    let element = match sprite.layers.iter().flat_map(|l| &l.elements)
        .find(|e| e.id == fc_state.element_id)
    {
        Some(e) => e,
        None => {
            editor.editing_flow_curve = None;
            return false;
        }
    };

    // Start drag: click on a control point
    if response.drag_started() {
        if let Some(pointer_pos) = response.interact_pointer_pos() {
            if let Some(cp_idx) = canvas_render::hit_test_flow_curve_cp(
                pointer_pos, element, &editor.viewport, canvas_center, 10.0,
            ) {
                let cps = &element.hatch_flow_curve.as_ref().unwrap().control_points;
                editor.editing_flow_curve = Some(crate::state::editor::FlowCurveEditState {
                    element_id: fc_state.element_id.clone(),
                    dragging_cp_index: Some(cp_idx),
                    drag_start_world: editor.viewport.screen_to_world(pointer_pos, canvas_center),
                    initial_cp_pos: cps[cp_idx],
                });
                return true;
            }
        }
    }

    // Continue drag
    if response.dragged() {
        if let Some(cp_idx) = fc_state.dragging_cp_index {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                let current_world = editor.viewport.screen_to_world(pointer_pos, canvas_center);
                let delta = current_world - fc_state.drag_start_world;
                let new_pos = fc_state.initial_cp_pos + delta;

                // Update the control point directly on the sprite for live preview
                for layer in &mut sprite.layers {
                    for elem in &mut layer.elements {
                        if elem.id == fc_state.element_id {
                            if let Some(ref mut fc) = elem.hatch_flow_curve {
                                if cp_idx < fc.control_points.len() {
                                    fc.control_points[cp_idx] = new_pos;
                                }
                            }
                        }
                    }
                }
                return true;
            }
        }
    }

    // End drag: commit the change via action for undo tracking
    if response.drag_stopped() {
        if fc_state.dragging_cp_index.is_some() {
            // Read the current flow curve state and commit it
            if let Some(element) = sprite.layers.iter().flat_map(|l| &l.elements)
                .find(|e| e.id == fc_state.element_id)
            {
                if let Some(ref fc) = element.hatch_flow_curve {
                    actions.push(AppAction::SetFlowCurve {
                        element_id: fc_state.element_id.clone(),
                        flow_curve: fc.clone(),
                    });
                }
            }
            editor.editing_flow_curve = Some(crate::state::editor::FlowCurveEditState {
                element_id: fc_state.element_id.clone(),
                dragging_cp_index: None,
                drag_start_world: Vec2::ZERO,
                initial_cp_pos: Vec2::ZERO,
            });
            return true;
        }
    }

    false
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
            let threshold = HIT_TEST_THRESHOLD / editor.viewport.zoom;
            editor.hover_element_id = hit_test::hit_test_elements(world_pos, sprite, threshold, editor.layer.solo_layer_id.as_deref());
        } else {
            editor.hover_element_id = None;
        }
    }

    // Set line tool cursor
    if response.hover_pos().is_some() {
        if editor.hover_element_id.is_some() && !editor.line_tool.is_drawing {
            response.ctx.set_cursor_icon(egui::CursorIcon::Grab);
        } else {
            response.ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
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

    // Handle line tool input (returns Vec<AppAction> now for symmetry support)
    let (line_actions, merge_target) = canvas_input::handle_line_tool_input(
        response,
        editor,
        sprite,
        project,
        canvas_rect,
    );
    actions.extend(line_actions);

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
            editor.brush.color_index,
            editor.brush.stroke_width,
            theme_mode,
            merge_target,
            editor.line_tool.curve_mode,
        );

        // Render symmetry ghost preview
        if editor.symmetry.active {
            canvas_render::render_symmetry_ghost(
                painter,
                &editor.line_tool.vertices,
                snap_pos,
                &editor.symmetry,
                &editor.viewport,
                canvas_rect,
                editor.brush.stroke_width,
                theme_mode,
            );
        }
    }
}

fn zoom_to_fit(editor: &mut EditorState, sprite: &Sprite, canvas_rect: egui::Rect) {
    // Shrink effective size to account for floating UI panels overlapping the canvas:
    // toolbar (~48px top), status bar (~40px bottom), sidebar (right)
    let sidebar_w = if editor.sidebar_expanded { 220.0 } else { 64.0 };
    let inset = egui::Vec2::new(sidebar_w + 16.0, 88.0);
    let effective_size = (canvas_rect.size() - inset).max(egui::Vec2::splat(100.0));

    // If elements are selected, frame the selection instead of all content
    if !editor.selection.is_empty()
        && let Some((sel_min, sel_max)) = transform::selection_bounds(sprite, &editor.selection.selected_ids) {
            editor.viewport.zoom_to_fit(sel_min, sel_max, effective_size);
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

    editor.viewport.zoom_to_fit(min, max, effective_size);
}
