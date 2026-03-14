use crate::state::editor::EditorState;

/// Draw the bottom status bar.
pub fn draw_status_bar(ctx: &egui::Context, editor_state: &EditorState, grid_size: f32) {
    egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            // Cursor world position
            if let Some(pos) = editor_state.cursor_world_pos {
                ui.label(format!("X: {:.1}  Y: {:.1}", pos.x, pos.y));
            } else {
                ui.label("X: --  Y: --");
            }

            ui.separator();

            // Zoom level
            ui.label(format!("Zoom: {:.0}%", editor_state.viewport.zoom * 100.0));

            ui.separator();

            // Active tool
            ui.label(format!("Tool: {}", editor_state.active_tool.name()));

            // Curve/straight mode for line tool
            if editor_state.active_tool == crate::state::editor::ToolKind::Line {
                let mode = if editor_state.curve_mode {
                    "Curve"
                } else {
                    "Straight"
                };
                ui.label(format!("({})", mode));
            }

            ui.separator();

            // Grid size
            ui.label(format!("Grid: {:.0}", grid_size));

            // Drawing state
            if editor_state.line_tool_state.active_element_id.is_some() {
                ui.separator();
                ui.label("Drawing...");
            }

            // Merge preview indicator
            if let Some(ref merge) = editor_state.merge_preview {
                ui.separator();
                if merge.same_element {
                    ui.label("Close path");
                } else {
                    ui.label("Merge");
                }
            }

            // Animation info
            if editor_state.animation.selected_sequence_id.is_some() {
                ui.separator();
                let frame = (editor_state.animation.current_time * 60.0).round() as i32;
                ui.label(format!("Frame: {}", frame));
                if editor_state.animation.playing {
                    ui.label("Playing");
                }
            }
        });
    });
}
