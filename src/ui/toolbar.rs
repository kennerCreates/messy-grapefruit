use crate::model::project::{GridMode, Project};
use crate::model::sprite::Sprite;
use crate::state::editor::EditorState;
use crate::state::history::History;

use super::icons;

pub fn show_toolbar(
    ui: &mut egui::Ui,
    editor: &mut EditorState,
    project: &mut Project,
    sprite: &mut Sprite,
    history: &mut History,
    sprite_path: &mut Option<std::path::PathBuf>,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;

        // File buttons
        if ui
            .add(icons::icon_button(icons::action_new(), ui))
            .on_hover_text("New")
            .clicked()
        {
            *sprite = Sprite::new("Untitled", 256, 256);
            *history = History::new(200);
            *sprite_path = None;
            editor.layer.set_active_by_idx(0, sprite);
            editor.line_tool.clear();
            editor.zoom_to_fit_requested = true;
        }

        if ui
            .add(icons::icon_button(icons::action_load(), ui))
            .on_hover_text("Open")
            .clicked()
            && let Some(path) = rfd::FileDialog::new()
                .add_filter("Sprite", &["sprite"])
                .pick_file()
        {
            match crate::io::load_sprite(&path) {
                Ok(loaded) => {
                    *sprite = loaded;
                    *history = History::new(200);
                    *sprite_path = Some(path);
                    editor.layer.set_active_by_idx(0, sprite);
                    editor.line_tool.clear();
                    editor.zoom_to_fit_requested = true;
                }
                Err(e) => {
                    eprintln!("Failed to load sprite: {e}");
                }
            }
        }

        if ui
            .add(icons::icon_button(icons::action_save(), ui))
            .on_hover_text("Save")
            .clicked()
        {
            let path = if let Some(existing) = sprite_path.as_ref() {
                Some(existing.clone())
            } else {
                rfd::FileDialog::new()
                    .add_filter("Sprite", &["sprite"])
                    .set_file_name(format!("{}.sprite", sprite.name))
                    .save_file()
            };
            if let Some(path) = path {
                match crate::io::save_sprite(sprite, &path) {
                    Ok(()) => {
                        *sprite_path = Some(path);
                    }
                    Err(e) => {
                        eprintln!("Failed to save sprite: {e}");
                    }
                }
            }
        }

        if ui
            .add(icons::icon_button(icons::action_save_as(), ui))
            .on_hover_text("Save As")
            .clicked()
            && let Some(path) = rfd::FileDialog::new()
                .add_filter("Sprite", &["sprite"])
                .set_file_name(format!("{}.sprite", sprite.name))
                .save_file()
        {
            match crate::io::save_sprite(sprite, &path) {
                Ok(()) => {
                    *sprite_path = Some(path);
                }
                Err(e) => {
                    eprintln!("Failed to save sprite: {e}");
                }
            }
        }

        ui.separator();

        // Undo/Redo
        if ui
            .add_enabled(history.can_undo(), icons::icon_button(icons::undo(), ui))
            .on_hover_text("Undo (Ctrl+Z)")
            .clicked()
        {
            editor.clear_vertex_selection();
            history.undo(sprite);
            editor.layer.validate(sprite);
        }
        if ui
            .add_enabled(history.can_redo(), icons::icon_button(icons::redo(), ui))
            .on_hover_text("Redo (Ctrl+Y)")
            .clicked()
        {
            editor.clear_vertex_selection();
            history.redo(sprite);
            editor.layer.validate(sprite);
        }

        ui.separator();

        // Tools: Select, Line
        let is_select = matches!(editor.tool, crate::state::editor::ToolKind::Select);
        if ui
            .add(icons::icon_button(icons::tool_select(), ui).selected(is_select))
            .on_hover_text("Select Tool (V)")
            .clicked()
        {
            editor.clear_vertex_selection();
            editor.tool = crate::state::editor::ToolKind::Select;
        }

        let is_line = matches!(editor.tool, crate::state::editor::ToolKind::Line);
        if ui
            .add(icons::icon_button(icons::tool_line(), ui).selected(is_line))
            .on_hover_text("Line Tool (L)")
            .clicked()
        {
            editor.clear_vertex_selection();
            editor.tool = crate::state::editor::ToolKind::Line;
        }

        let is_fill = matches!(editor.tool, crate::state::editor::ToolKind::Fill);
        if ui
            .add(icons::icon_button(icons::tool_fill(), ui).selected(is_fill))
            .on_hover_text("Fill Tool (G)")
            .clicked()
        {
            editor.clear_vertex_selection();
            editor.tool = crate::state::editor::ToolKind::Fill;
        }

        let is_eyedropper = matches!(editor.tool, crate::state::editor::ToolKind::Eyedropper);
        if ui
            .add(icons::icon_button(icons::tool_eyedropper(), ui).selected(is_eyedropper))
            .on_hover_text("Eyedropper (I)")
            .clicked()
        {
            editor.clear_vertex_selection();
            editor.tool = crate::state::editor::ToolKind::Eyedropper;
        }

        ui.separator();

        // Grid controls
        let grid_sizes: &[u32] = &[1, 2, 4, 8, 16, 32, 64];
        egui::ComboBox::from_id_salt("grid_size")
            .selected_text(format!("{}px", project.editor_preferences.grid_size))
            .width(60.0)
            .show_ui(ui, |ui| {
                for &size in grid_sizes {
                    ui.selectable_value(
                        &mut project.editor_preferences.grid_size,
                        size,
                        format!("{size}px"),
                    );
                }
            });

        if ui
            .add(icons::icon_button(icons::grid_dots(), ui).selected(project.editor_preferences.show_dots))
            .on_hover_text("Grid Dots")
            .clicked()
        {
            project.editor_preferences.show_dots = !project.editor_preferences.show_dots;
        }

        // Three-way grid line mode: Off / Straight / Isometric (mutually exclusive)
        let is_straight = project.editor_preferences.grid_mode == GridMode::Straight;
        if ui
            .add(icons::icon_button(icons::grid_lines(), ui).selected(is_straight))
            .on_hover_text("Straight Grid Lines")
            .clicked()
        {
            project.editor_preferences.grid_mode = if is_straight {
                GridMode::Off
            } else {
                GridMode::Straight
            };
        }

        let is_iso = project.editor_preferences.grid_mode == GridMode::Isometric;
        if ui
            .add(icons::icon_button(icons::grid_iso(), ui).selected(is_iso))
            .on_hover_text("Isometric Grid Lines")
            .clicked()
        {
            project.editor_preferences.grid_mode = if is_iso {
                GridMode::Off
            } else {
                GridMode::Isometric
            };
        }

        ui.separator();

        // View: Flip + Zoom to Fit
        if ui
            .add(icons::icon_button(icons::view_flip(), ui).selected(editor.viewport.flipped))
            .on_hover_text("Flip Canvas (H)")
            .clicked()
        {
            editor.viewport.flipped = !editor.viewport.flipped;
            editor.viewport.offset.x = -editor.viewport.offset.x;
        }
        if ui
            .add(icons::icon_button(icons::view_zoom_fit(), ui))
            .on_hover_text("Zoom to Fit (F)")
            .clicked()
        {
            editor.zoom_to_fit_requested = true;
        }

    });
}
