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
use model::sprite::Sprite;
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

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        theme::apply_theme(ctx, self.project.editor_preferences.theme);

        // Handle undo/redo globally (works regardless of focused panel)
        ctx.input(|i| {
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Z) && !i.modifiers.shift {
                self.history.undo(&mut self.sprite);
            }
            if i.modifiers.ctrl
                && (i.key_pressed(egui::Key::Y)
                    || (i.key_pressed(egui::Key::Z) && i.modifiers.shift))
            {
                self.history.redo(&mut self.sprite);
            }
        });

        let panel_bg = theme::floating_panel_color(self.project.editor_preferences.theme);
        let floating_frame = egui::Frame::NONE
            .fill(panel_bg)
            .corner_radius(8.0)
            .inner_margin(6.0);

        // Canvas fills entire window (no frame)
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let actions = ui::canvas::show_canvas(
                    ui,
                    &mut self.editor,
                    &self.sprite,
                    &self.project,
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

        // Floating sidebar (right side)
        egui::Window::new("sidebar")
            .title_bar(false)
            .resizable(false)
            .movable(false)
            .collapsible(false)
            .anchor(egui::Align2::RIGHT_TOP, [-8.0, 48.0])
            .frame(floating_frame)
            .min_width(180.0)
            .max_width(180.0)
            .show(ctx, |ui| {
                ui::sidebar::show_sidebar(
                    ui,
                    &mut self.editor,
                    &mut self.sprite,
                    &mut self.project,
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
