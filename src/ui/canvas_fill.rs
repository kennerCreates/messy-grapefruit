use crate::action::AppAction;
use crate::engine::hit_test;
use crate::model::project::Theme;
use crate::model::sprite::{GradientFill, Sprite};
use crate::state::editor::{EditorState, FillMode};

use super::canvas::HIT_TEST_THRESHOLD;

/// Handle fill tool input: click closed element to fill, click empty canvas for background.
#[allow(clippy::too_many_arguments)]
pub(super) fn handle_fill_tool(
    response: &egui::Response,
    _painter: &egui::Painter,
    editor: &mut EditorState,
    sprite: &Sprite,
    canvas_rect: egui::Rect,
    _theme_mode: Theme,
    actions: &mut Vec<AppAction>,
) {
    let canvas_center = canvas_rect.center();
    let threshold = HIT_TEST_THRESHOLD / editor.viewport.zoom;

    // Hover: update hover element for highlight feedback
    if let Some(hover_pos) = response.hover_pos() {
        let world = editor.viewport.screen_to_world(hover_pos, canvas_center);
        let hit = hit_test::hit_test_fill(
            world,
            sprite,
            threshold,
            editor.layer.solo_layer_id.as_deref(),
        );
        editor.hover_element_id = hit.map(|(id, _)| id);
    } else {
        editor.hover_element_id = None;
    }

    // Click: apply fill (only inside the canvas area)
    if response.clicked()
        && let Some(click_pos) = response.interact_pointer_pos()
        && canvas_rect.contains(click_pos)
    {
        let world = editor.viewport.screen_to_world(click_pos, canvas_center);
        let hit = hit_test::hit_test_fill(
            world,
            sprite,
            threshold,
            editor.layer.solo_layer_id.as_deref(),
        );

        match hit {
            Some((element_id, true)) => {
                if editor.brush.hatch_apply_enabled {
                    // Hatch-only mode: apply hatch without changing fill
                    if let Some(ref pattern_id) = editor.selected_hatch_pattern_id {
                        actions.push(AppAction::SetHatchFill {
                            element_id,
                            hatch_fill_id: pattern_id.clone(),
                        });
                    }
                } else {
                    // Apply fill color (flat or gradient)
                    match editor.brush.fill_mode {
                        FillMode::Flat => {
                            actions.push(AppAction::SetFillColor {
                                element_id: element_id.clone(),
                                fill_color_index: editor.brush.fill_color_index,
                            });
                            editor.track_recent_color(editor.brush.fill_color_index);
                        }
                        FillMode::LinearGradient => {
                            let mut gf = GradientFill::linear(
                                editor.brush.gradient_stops.clone(),
                                editor.brush.gradient_angle,
                            );
                            gf.spread = editor.brush.gradient_spread;
                            gf.midpoints = editor.brush.gradient_midpoints.clone();
                            actions.push(AppAction::SetGradientFill {
                                element_id: element_id.clone(),
                                gradient_fill: gf,
                            });
                        }
                        FillMode::RadialGradient => {
                            let mut gf = GradientFill::radial(
                                editor.brush.gradient_stops.clone(),
                                editor.brush.radial_center,
                                editor.brush.radial_radius,
                            );
                            gf.spread = editor.brush.gradient_spread;
                            gf.midpoints = editor.brush.gradient_midpoints.clone();
                            gf.focal_offset = Some(editor.brush.radial_focal_offset);
                            actions.push(AppAction::SetGradientFill {
                                element_id: element_id.clone(),
                                gradient_fill: gf,
                            });
                        }
                    }
                }
            }
            Some((_, false)) => {
                // Open element: cannot fill (no-op)
            }
            None => {
                // Empty canvas: set background color (flat mode only)
                if editor.brush.fill_mode == FillMode::Flat {
                    actions.push(AppAction::SetBackgroundColor {
                        background_color_index: editor.brush.fill_color_index,
                    });
                    editor.track_recent_color(editor.brush.fill_color_index);
                }
            }
        }
    }
}
