mod action;
mod clipboard;
mod engine;
mod io;
mod math;
mod model;
mod state;
mod theme;
mod ui;

use eframe::egui;
use model::project::Project;
use model::sprite::{Sprite, StrokeElement};
use state::editor::EditorState;
use state::history::History;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("Messy Grapefruit — Sprite Editor"),
        ..Default::default()
    };
    eframe::run_native(
        "Messy Grapefruit",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

pub struct App {
    pub project: Project,
    pub sprite: Sprite,
    pub editor: EditorState,
    pub history: History,
    pub sprite_path: Option<std::path::PathBuf>,
    /// Internal clipboard fallback (in case system clipboard fails).
    pub internal_clipboard: Option<Vec<StrokeElement>>,
}

impl App {
    fn new(cc: &eframe::CreationContext) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);

        // Load Courier Prime font
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "courier_prime".to_owned(),
            std::sync::Arc::new(egui::FontData::from_static(
                include_bytes!("../assets/fonts/Courier_Prime/CourierPrime-Regular.ttf"),
            )),
        );
        fonts.font_data.insert(
            "courier_prime_bold".to_owned(),
            std::sync::Arc::new(egui::FontData::from_static(
                include_bytes!("../assets/fonts/Courier_Prime/CourierPrime-Bold.ttf"),
            )),
        );
        fonts.families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .insert(0, "courier_prime".to_owned());
        fonts.families
            .get_mut(&egui::FontFamily::Monospace)
            .unwrap()
            .insert(0, "courier_prime".to_owned());
        cc.egui_ctx.set_fonts(fonts);

        let project = Project::new("Untitled Project");
        theme::apply_theme(&cc.egui_ctx, &project);
        let sprite = Sprite::new("Untitled", 256, 256);
        let mut editor = EditorState::default();
        editor.layer.set_active_by_idx(0, &sprite);
        Self {
            project,
            sprite,
            editor,
            history: History::new(200),
            sprite_path: None,
            internal_clipboard: None,
        }
    }
}

impl App {
    fn dispatch_action(&mut self, action: action::AppAction) {
        let before = self.sprite.clone();
        let layer_idx = self.editor.layer.resolve_active_idx(&self.sprite);

        match action {
            action::AppAction::CommitStroke(element) => {
                self.sprite.layers[layer_idx].elements.push(element);
                self.history.push("Draw stroke".into(), before, self.sprite.clone());
            }
            action::AppAction::MergeStroke { merged_element, replace_element_id } => {
                let layer = &mut self.sprite.layers[layer_idx];
                layer.elements.retain(|e| e.id != replace_element_id);
                layer.elements.push(merged_element);
                self.history.push("Merge stroke".into(), before, self.sprite.clone());
            }
            action::AppAction::SetFillColor { element_id, fill_color_index } => {
                for layer in &mut self.sprite.layers {
                    for elem in &mut layer.elements {
                        if elem.id == element_id {
                            elem.fill_color_index = fill_color_index;
                        }
                    }
                }
                self.history.push("Set fill color".into(), before, self.sprite.clone());
            }
            action::AppAction::SetBackgroundColor { background_color_index } => {
                self.sprite.background_color_index = background_color_index;
                self.history.push("Set background color".into(), before, self.sprite.clone());
            }
            action::AppAction::AddPaletteColor(color) => {
                if self.project.palette.colors.len() < 256 {
                    self.project.palette.colors.push(color);
                }
                // Project-level, no sprite undo
            }
            action::AppAction::DeletePaletteColor(index) => {
                if index == 0 || index as usize >= self.project.palette.colors.len() {
                    return;
                }
                self.project.palette.colors.remove(index as usize);
                // Remap all sprite color indices
                for layer in &mut self.sprite.layers {
                    for elem in &mut layer.elements {
                        elem.stroke_color_index = remap_color_index(elem.stroke_color_index, index);
                        elem.fill_color_index = remap_color_index(elem.fill_color_index, index);
                    }
                }
                self.sprite.background_color_index = remap_color_index(self.sprite.background_color_index, index);
                self.history.push("Delete palette color".into(), before, self.sprite.clone());
            }
            action::AppAction::EditPaletteColor { index, color } => {
                if let Some(c) = self.project.palette.colors.get_mut(index as usize) {
                    *c = color;
                }
                // Project-level, no sprite undo
            }
            action::AppAction::ImportPalette(colors) => {
                self.project.palette.colors = colors;
                // Ensure index 0 is transparent
                if self.project.palette.colors.is_empty()
                    || self.project.palette.colors[0].a != 0
                {
                    self.project.palette.colors.insert(
                        0,
                        model::project::PaletteColor::transparent(),
                    );
                }
                // Truncate to 256
                self.project.palette.colors.truncate(256);
                // Auto-pick theme colors from the new palette
                let (dark, light) = model::project::auto_pick_theme_colors(&self.project.palette);
                self.project.editor_preferences.dark_theme_colors = dark;
                self.project.editor_preferences.light_theme_colors = light;
                // Project-level, no sprite undo
            }
        }
    }
}

/// Remap a color index after a palette color has been deleted.
/// If the index equals the deleted index, it becomes 0 (transparent).
/// If the index is above the deleted index, it decrements by 1.
fn remap_color_index(index: u8, deleted: u8) -> u8 {
    if index == deleted {
        0
    } else if index > deleted {
        index - 1
    } else {
        index
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        theme::apply_theme(ctx, &self.project);

        // Handle undo/redo and copy/paste globally
        let (undo, redo, copy, paste, cut) = ctx.input(|i| {
            (
                i.modifiers.ctrl && i.key_pressed(egui::Key::Z) && !i.modifiers.shift,
                i.modifiers.ctrl
                    && (i.key_pressed(egui::Key::Y)
                        || (i.key_pressed(egui::Key::Z) && i.modifiers.shift)),
                i.modifiers.ctrl && i.key_pressed(egui::Key::C),
                i.modifiers.ctrl && i.key_pressed(egui::Key::V),
                i.modifiers.ctrl && i.key_pressed(egui::Key::X),
            )
        });

        if undo {
            self.editor.clear_vertex_selection();
            self.history.undo(&mut self.sprite);
            self.editor.layer.validate(&self.sprite);
        }
        if redo {
            self.editor.clear_vertex_selection();
            self.history.redo(&mut self.sprite);
            self.editor.layer.validate(&self.sprite);
        }
        if cut {
            clipboard::cut(&mut self.editor, &mut self.sprite, &mut self.history, &mut self.internal_clipboard);
        } else if copy {
            clipboard::copy_selected(&self.editor, &self.sprite, &mut self.internal_clipboard);
        }
        if paste {
            clipboard::paste(&mut self.editor, &mut self.sprite, &mut self.history, &self.internal_clipboard);
        }

        let panel_bg = theme::floating_panel_color(self.project.editor_preferences.theme);
        let floating_frame = egui::Frame::NONE
            .fill(panel_bg)
            .corner_radius(8.0)
            .inner_margin(10.0);

        // Canvas fills entire window (no frame)
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let actions = ui::canvas::show_canvas(
                    ui,
                    &mut self.editor,
                    &mut self.sprite,
                    &self.project,
                    &mut self.history,
                );
                for action in actions {
                    self.dispatch_action(action);
                }
            });

        // Floating toolbar (centered at top)
        egui::Window::new("toolbar")
            .title_bar(false)
            .resizable(false)
            .movable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_TOP, [0.0, 8.0])
            .frame(floating_frame)
            .show(ctx, |ui| {
                ui::toolbar::show_toolbar(
                    ui,
                    &mut self.editor,
                    &mut self.project,
                    &mut self.sprite,
                    &mut self.history,
                    &mut self.sprite_path,
                );
            });

        // Floating sidebar (right side) — width depends on collapsed/expanded
        let sidebar_width = if self.editor.sidebar_expanded { 220.0 } else { 64.0 };
        let sidebar_resp = egui::Window::new("sidebar")
            .title_bar(false)
            .resizable(false)
            .movable(false)
            .collapsible(false)
            .anchor(egui::Align2::RIGHT_TOP, [-8.0, 48.0])
            .frame(floating_frame)
            .min_width(sidebar_width)
            .max_width(sidebar_width)
            .show(ctx, |ui| {
                ui::sidebar::show_sidebar(
                    ui,
                    &mut self.editor,
                    &mut self.sprite,
                    &mut self.project,
                    &mut self.history,
                )
            });
        if let Some(resp) = sidebar_resp
            && let Some(actions) = resp.inner
        {
            for action in actions {
                self.dispatch_action(action);
            }
        }

        // Floating status bar (bottom center)
        egui::Window::new("status_bar")
            .title_bar(false)
            .resizable(false)
            .movable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -8.0])
            .frame(floating_frame)
            .show(ctx, |ui| {
                ui::status_bar::show_status_bar(ui, &self.editor, &self.sprite, &self.project);
            });
    }
}
