use crate::model::project::{GridMode, Project, Theme};
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
    active_layer_idx: &mut usize,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;

        // File buttons (no icons provided — keep as text)
        if ui.button("New").clicked() {
            *sprite = Sprite::new("Untitled", 256, 256);
            *history = History::new(200);
            *sprite_path = None;
            *active_layer_idx = 0;
            editor.line_tool.clear();
        }

        if ui.button("Open").clicked()
            && let Some(path) = rfd::FileDialog::new()
                .add_filter("Sprite", &["sprite"])
                .pick_file()
        {
            match crate::io::load_sprite(&path) {
                Ok(loaded) => {
                    *sprite = loaded;
                    *history = History::new(200);
                    *sprite_path = Some(path);
                    *active_layer_idx = 0;
                    editor.line_tool.clear();
                }
                Err(e) => {
                    eprintln!("Failed to load sprite: {e}");
                }
            }
        }

        if ui.button("Save").clicked() {
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

        ui.separator();

        // Undo/Redo
        if ui
            .add_enabled(history.can_undo(), icons::icon_button(icons::undo()))
            .on_hover_text("Undo (Ctrl+Z)")
            .clicked()
        {
            history.undo(sprite);
        }
        if ui
            .add_enabled(history.can_redo(), icons::icon_button(icons::redo()))
            .on_hover_text("Redo (Ctrl+Y)")
            .clicked()
        {
            history.redo(sprite);
        }

        ui.separator();

        // Tool: Line
        let is_line = matches!(editor.tool, crate::state::editor::ToolKind::Line);
        if ui
            .add(icons::icon_button(icons::tool_line()).selected(is_line))
            .on_hover_text("Line Tool (L)")
            .clicked()
        {
            editor.tool = crate::state::editor::ToolKind::Line;
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
            .add(icons::icon_button(icons::grid_dots()).selected(project.editor_preferences.show_dots))
            .on_hover_text("Grid Dots")
            .clicked()
        {
            project.editor_preferences.show_dots = !project.editor_preferences.show_dots;
        }
        if ui
            .add(icons::icon_button(icons::grid_lines()).selected(project.editor_preferences.show_lines))
            .on_hover_text("Grid Lines")
            .clicked()
        {
            project.editor_preferences.show_lines = !project.editor_preferences.show_lines;
        }

        let is_iso = project.editor_preferences.grid_mode == GridMode::Isometric;
        if ui
            .add(icons::icon_button(icons::grid_iso()).selected(is_iso))
            .on_hover_text("Isometric Grid")
            .clicked()
        {
            project.editor_preferences.grid_mode = if is_iso {
                GridMode::Straight
            } else {
                GridMode::Isometric
            };
        }

        ui.separator();

        // Taper toggle
        if ui
            .add(icons::icon_button(icons::stroke_taper()).selected(project.stroke_taper))
            .on_hover_text("Stroke Taper")
            .clicked()
        {
            project.stroke_taper = !project.stroke_taper;
        }

        ui.separator();

        // View: Flip + Zoom to Fit
        if ui
            .add(icons::icon_button(icons::view_flip()).selected(editor.viewport.flipped))
            .on_hover_text("Flip Canvas (H)")
            .clicked()
        {
            editor.viewport.flipped = !editor.viewport.flipped;
        }
        if ui
            .add(icons::icon_button(icons::view_zoom_fit()))
            .on_hover_text("Zoom to Fit (F)")
            .clicked()
        {
            editor.zoom_to_fit_requested = true;
        }

        ui.separator();

        // Theme toggle
        let is_dark = project.editor_preferences.theme == Theme::Dark;
        let theme_icon = if is_dark { icons::theme_dark() } else { icons::theme_light() };
        if ui
            .add(icons::icon_button(theme_icon))
            .on_hover_text(if is_dark { "Switch to Light Theme" } else { "Switch to Dark Theme" })
            .clicked()
        {
            project.editor_preferences.theme = if is_dark { Theme::Light } else { Theme::Dark };
        }
    });
}
