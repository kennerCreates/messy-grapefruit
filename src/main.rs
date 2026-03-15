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
    pub active_layer_idx: usize,
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
            active_layer_idx: 0,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        theme::apply_theme(ctx, self.project.editor_preferences.theme);

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui::toolbar::show_toolbar(
                ui,
                &mut self.editor,
                &mut self.project,
                &mut self.sprite,
                &mut self.history,
                &mut self.sprite_path,
                &mut self.active_layer_idx,
            );
        });

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui::status_bar::show_status_bar(ui, &self.editor, &self.sprite, &self.project);
        });

        egui::SidePanel::right("sidebar")
            .default_width(220.0)
            .show(ctx, |ui| {
                ui::sidebar::show_sidebar(
                    ui,
                    &mut self.editor,
                    &mut self.sprite,
                    &self.project,
                    &mut self.active_layer_idx,
                );
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let actions = ui::canvas::show_canvas(
                ui,
                &mut self.editor,
                &self.sprite,
                &self.project,
                self.active_layer_idx,
            );

            for action in actions {
                match action {
                    ui::canvas::CanvasAction::CommitStroke(element) => {
                        let before = self.sprite.clone();
                        let layer_idx = self.active_layer_idx.min(self.sprite.layers.len().saturating_sub(1));
                        self.sprite.layers[layer_idx].elements.push(element);
                        self.history.push("Draw stroke".into(), before, self.sprite.clone());
                    }
                    ui::canvas::CanvasAction::MergeStroke { merged_element, replace_element_id } => {
                        let before = self.sprite.clone();
                        let layer_idx = self.active_layer_idx.min(self.sprite.layers.len().saturating_sub(1));
                        let layer = &mut self.sprite.layers[layer_idx];
                        layer.elements.retain(|e| e.id != replace_element_id);
                        layer.elements.push(merged_element);
                        self.history.push("Merge stroke".into(), before, self.sprite.clone());
                    }
                }
            }

            // Handle undo/redo
            ui.input(|i| {
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
        });
    }
}
