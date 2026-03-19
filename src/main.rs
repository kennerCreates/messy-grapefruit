mod action;
mod action_handler;
mod clipboard;
mod engine;
mod io;
mod math;
mod model;
mod state;
mod theme;
mod ui;

use std::collections::HashMap;

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
    /// Cached textures for reference images, keyed by reference image ID.
    pub ref_image_textures: HashMap<String, egui::TextureHandle>,
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

        let mut project = Project::new("Untitled Project");
        // Load saved app defaults (palette + theme) if available
        if let Some(defaults) = io::load_app_defaults() {
            project.palette = defaults.palette;
            project.editor_preferences = defaults.editor_preferences;
            project.hatch_patterns = defaults.hatch_patterns;
        }
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
            ref_image_textures: HashMap::new(),
        }
    }
}

impl App {
    fn dispatch_action(&mut self, action: action::AppAction) {
        action_handler::dispatch(self, action);
    }

    /// Load reference image textures that are missing from the cache.
    fn sync_ref_image_textures(&mut self, ctx: &egui::Context) {
        let base_path = self.sprite_path.as_ref().and_then(|p| p.parent().map(|p| p.to_path_buf()));

        for ref_img in &mut self.sprite.reference_images {
            if self.ref_image_textures.contains_key(&ref_img.id) {
                continue;
            }
            // Resolve absolute path
            let path = if let Some(base) = &base_path {
                base.join(&ref_img.path)
            } else {
                std::path::PathBuf::from(&ref_img.path)
            };
            if let Ok((tex, w, h)) = io::load_image_texture(ctx, &path) {
                ref_img.image_size = Some((w, h));
                self.ref_image_textures.insert(ref_img.id.clone(), tex);
            }
        }

        // Remove textures for deleted reference images
        let valid_ids: Vec<String> = self.sprite.reference_images.iter().map(|r| r.id.clone()).collect();
        self.ref_image_textures.retain(|id, _| valid_ids.contains(id));
    }
}


impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        theme::apply_theme(ctx, &self.project);

        // Sync reference image textures
        self.sync_ref_image_textures(ctx);

        // Animation playback tick
        if self.editor.playback.playing {
            let now = ctx.input(|i| i.time);
            if let Some(last) = self.editor.playback.last_frame_time {
                let delta = (now - last) as f32 * self.editor.playback.speed;
                self.editor.timeline.playhead_time += delta;
                // Handle end of animation
                if let Some(seq_id) = self.editor.timeline.selected_sequence_id.clone() {
                    if let Some(seq) = self.sprite.animations.iter().find(|s| s.id == seq_id) {
                        if self.editor.timeline.playhead_time >= seq.duration_secs {
                            if self.editor.playback.loop_mode && seq.looping {
                                if seq.duration_secs > 0.0 {
                                    self.editor.timeline.playhead_time %= seq.duration_secs;
                                } else {
                                    self.editor.timeline.playhead_time = 0.0;
                                }
                            } else {
                                self.editor.timeline.playhead_time = seq.duration_secs;
                                self.editor.playback.playing = false;
                                self.editor.playback.last_frame_time = None;
                            }
                        }
                    }
                }
            }
            if self.editor.playback.playing {
                self.editor.playback.last_frame_time = Some(now);
            }
            ctx.request_repaint();
        }

        // Handle drag-and-drop for reference images
        let dropped_files: Vec<_> = ctx.input(|i| i.raw.dropped_files.clone());
        for file in dropped_files {
            if let Some(path) = &file.path {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                if matches!(ext.as_str(), "png" | "jpg" | "jpeg") {
                    let path_str = path.to_string_lossy().to_string();
                    let ref_image = model::sprite::ReferenceImage::new(path_str);
                    self.dispatch_action(action::AppAction::AddReferenceImage(ref_image));
                }
            }
        }

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
            // If a tool is actively in use, cancel it instead of undoing
            if self.editor.line_tool.is_drawing {
                self.editor.line_tool.clear();
            } else if self.editor.select_drag.is_some() {
                self.editor.select_drag = None;
                self.history.cancel_drag(&mut self.sprite);
            } else if self.editor.dragging_ref_image.is_some() {
                self.editor.dragging_ref_image = None;
                self.history.cancel_drag(&mut self.sprite);
            } else {
                self.editor.clear_vertex_selection();
                self.history.undo(&mut self.sprite);
                self.editor.layer.validate(&self.sprite);
            }
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
                    &self.ref_image_textures,
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
                    &mut self.ref_image_textures,
                );
            });

        // Floating sidebar (right side) — width depends on collapsed/expanded
        let sidebar_width = if self.editor.ui.sidebar_expanded { 220.0 } else { 64.0 };
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
            .min_width(560.0)
            .frame(floating_frame)
            .show(ctx, |ui| {
                ui::status_bar::show_status_bar(ui, &self.editor, &mut self.sprite, &mut self.project);
            });

        // Floating timeline (bottom center, above status bar)
        if self.editor.timeline.is_timeline_visible {
            const TIMELINE_HEIGHT: f32 = 160.0;
            const STATUS_BAR_HEIGHT: f32 = 36.0;
            let timeline_resp = egui::Window::new("timeline")
                .title_bar(false)
                .resizable(false)
                .movable(false)
                .collapsible(false)
                .anchor(
                    egui::Align2::CENTER_BOTTOM,
                    [0.0, -(STATUS_BAR_HEIGHT + TIMELINE_HEIGHT + 12.0)],
                )
                .frame(floating_frame)
                .min_width(600.0)
                .show(ctx, |ui| {
                    ui::timeline::show_timeline(
                        ui,
                        &mut self.editor,
                        &mut self.sprite,
                        &self.project,
                    )
                });
            if let Some(resp) = timeline_resp
                && let Some(actions) = resp.inner
            {
                for action in actions {
                    self.dispatch_action(action);
                }
            }
        }
    }
}
