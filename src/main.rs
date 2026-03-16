mod action;
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
use model::vec2::Vec2;
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
        theme::apply_theme(&cc.egui_ctx, project.editor_preferences.theme);
        Self {
            project,
            sprite: Sprite::new("Untitled", 256, 256),
            editor: EditorState::default(),
            history: History::new(200),
            sprite_path: None,
            internal_clipboard: None,
        }
    }
}

impl App {
    fn dispatch_action(&mut self, action: action::AppAction) {
        let before = self.sprite.clone();
        let layer_idx = self.editor.active_layer_idx.min(self.sprite.layers.len().saturating_sub(1));

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
        }
    }
}

/// Clipboard JSON wrapper for cross-sprite copy/paste.
#[derive(serde::Serialize, serde::Deserialize)]
struct ClipboardData {
    messy_grapefruit_clipboard: bool,
    elements: Vec<StrokeElement>,
}

impl App {
    fn copy_selected(&mut self) {
        if self.editor.selection.is_empty() {
            return;
        }
        let mut elements = Vec::new();
        for layer in &self.sprite.layers {
            for element in &layer.elements {
                if self.editor.selection.is_selected(&element.id) {
                    elements.push(element.clone());
                }
            }
        }
        if elements.is_empty() {
            return;
        }

        // Always store in internal clipboard
        self.internal_clipboard = Some(elements.clone());

        // Also try system clipboard
        let data = ClipboardData {
            messy_grapefruit_clipboard: true,
            elements,
        };
        if let Ok(json) = serde_json::to_string(&data)
            && let Ok(mut clipboard) = arboard::Clipboard::new()
        {
            let _ = clipboard.set_text(json);
        }
    }

    fn paste_from_clipboard(&mut self) {
        // Try system clipboard first
        let elements = if let Ok(mut clipboard) = arboard::Clipboard::new()
            && let Ok(json) = clipboard.get_text()
            && let Ok(data) = serde_json::from_str::<ClipboardData>(&json)
            && data.messy_grapefruit_clipboard
            && !data.elements.is_empty()
        {
            data.elements
        } else if let Some(elements) = &self.internal_clipboard {
            // Fall back to internal clipboard
            elements.clone()
        } else {
            return;
        };

        let before = self.sprite.clone();
        let layer_idx = self.editor.active_layer_idx.min(self.sprite.layers.len().saturating_sub(1));
        let mut new_ids = Vec::new();

        for mut element in elements {
            // Assign new UUIDs
            element.id = uuid::Uuid::new_v4().to_string();
            for v in &mut element.vertices {
                v.id = uuid::Uuid::new_v4().to_string();
            }
            // Offset position
            element.position += Vec2::new(10.0, 10.0);
            new_ids.push(element.id.clone());
            self.sprite.layers[layer_idx].elements.push(element);
        }

        self.history.push("Paste elements".into(), before, self.sprite.clone());
        self.editor.selection.select_all(new_ids);
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        theme::apply_theme(ctx, self.project.editor_preferences.theme);

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
            self.history.undo(&mut self.sprite);
        }
        if redo {
            self.history.redo(&mut self.sprite);
        }
        if copy || cut {
            self.copy_selected();
        }
        if cut && !self.editor.selection.is_empty() {
            let before = self.sprite.clone();
            let selected = self.editor.selection.selected_ids.clone();
            for layer in self.sprite.layers.iter_mut() {
                layer.elements.retain(|e| !selected.iter().any(|id| id == &e.id));
            }
            self.history.push("Cut elements".into(), before, self.sprite.clone());
            self.editor.selection.clear();
        }
        if paste {
            self.paste_from_clipboard();
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
        egui::Window::new("sidebar")
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
                );
            });

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
