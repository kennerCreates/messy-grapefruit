use crate::action::AppAction;
use crate::engine::hit_test;
use crate::model::project::Theme;
use crate::model::sprite::Sprite;
use crate::state::editor::{EditorState, EraserHover};
use crate::theme;

use super::canvas::HIT_TEST_THRESHOLD;
use super::canvas_render;

/// Eraser tool: hover detection, click handling, preview rendering.
#[allow(clippy::too_many_arguments)]
pub(super) fn handle_eraser_tool(
    response: &egui::Response,
    painter: &egui::Painter,
    editor: &mut EditorState,
    sprite: &Sprite,
    canvas_rect: egui::Rect,
    theme_mode: Theme,
    actions: &mut Vec<AppAction>,
) {
    let canvas_center = canvas_rect.center();
    let threshold = HIT_TEST_THRESHOLD / editor.viewport.zoom;

    // Update hover state
    if let Some(hover_pos) = response.hover_pos() {
        let world_pos = editor.viewport.screen_to_world(hover_pos, canvas_center);
        editor.eraser_hover = hit_test::hit_test_eraser(
            world_pos,
            sprite,
            threshold,
            editor.layer.solo_layer_id.as_deref(),
            &editor.viewport,
            canvas_center,
            hover_pos,
            canvas_render::VERTEX_HIT_RADIUS,
        );
    } else {
        editor.eraser_hover = None;
    }

    // Set cursor
    if response.hover_pos().is_some() {
        if editor.eraser_hover.is_some() {
            response.ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
        } else {
            response.ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
        }
    }

    // Handle click
    if response.clicked()
        && let Some(hover) = &editor.eraser_hover
    {
        match hover {
            EraserHover::Vertex { element_id, vertex_id, .. } => {
                actions.push(AppAction::EraseVertex {
                    element_id: element_id.clone(),
                    vertex_id: vertex_id.clone(),
                });
            }
            EraserHover::Segment { element_id, segment_index, .. } => {
                actions.push(AppAction::EraseSegment {
                    element_id: element_id.clone(),
                    segment_index: *segment_index,
                });
            }
        }
    }

    // Render hover preview
    if let Some(hover) = &editor.eraser_hover.clone() {
        let color = theme::eraser_highlight_color(theme_mode);
        match hover {
            EraserHover::Vertex { element_id, vertex_id, .. } => {
                // Highlight the vertex with a red dot
                for layer in &sprite.layers {
                    for element in &layer.elements {
                        if element.id == *element_id
                            && let Some(v) = element.vertices.iter().find(|v| v.id == *vertex_id)
                        {
                            let world_pos = crate::engine::transform::vertex_world_pos(v, element);
                            let screen = editor.viewport.world_to_screen(world_pos, canvas_center);
                            painter.circle_filled(screen, 6.0, color);
                        }
                    }
                }
            }
            EraserHover::Segment { element_id, segment_index, .. } => {
                // Highlight the segment with a thick red stroke
                for layer in &sprite.layers {
                    for element in &layer.elements {
                        if element.id == *element_id {
                            let n = element.vertices.len();
                            let seg = *segment_index;
                            let next = (seg + 1) % n;
                            if seg < n && next < n {
                                let v0 = &element.vertices[seg];
                                let v1 = &element.vertices[next];
                                let w0 = crate::engine::transform::vertex_world_pos(v0, element);
                                let w1 = crate::engine::transform::vertex_world_pos(v1, element);
                                let s0 = editor.viewport.world_to_screen(w0, canvas_center);
                                let s1 = editor.viewport.world_to_screen(w1, canvas_center);
                                painter.line_segment(
                                    [s0, s1],
                                    egui::Stroke::new(4.0, color),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
