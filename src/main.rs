mod action;
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
            action::AppAction::MergeSymmetricStrokes(entries) => {
                let layer = &mut self.sprite.layers[layer_idx];
                for entry in entries {
                    layer.elements.retain(|e| e.id != entry.replace_element_id);
                    layer.elements.push(entry.merged_element);
                }
                self.history.push("Merge symmetric strokes".into(), before, self.sprite.clone());
            }
            action::AppAction::CommitSymmetricStrokes(elements) => {
                for elem in elements {
                    self.sprite.layers[layer_idx].elements.push(elem);
                }
                self.history.push("Draw symmetric strokes".into(), before, self.sprite.clone());
            }
            action::AppAction::SetFillColor { element_id, fill_color_index } => {
                for layer in &mut self.sprite.layers {
                    for elem in &mut layer.elements {
                        if elem.id == element_id {
                            elem.fill_color_index = fill_color_index;
                            // Flat fill replaces gradient fill
                            elem.gradient_fill = None;
                        }
                    }
                }
                self.history.push("Set fill color".into(), before, self.sprite.clone());
            }
            action::AppAction::SetBackgroundColor { background_color_index } => {
                self.sprite.background_color_index = background_color_index;
                self.history.push("Set background color".into(), before, self.sprite.clone());
            }
            action::AppAction::EraseVertex { element_id, vertex_id } => {
                if let Some((layer_idx, elem_idx)) = find_element_location(&self.sprite, &element_id) {
                    let element = &self.sprite.layers[layer_idx].elements[elem_idx];
                    let result = engine::eraser::erase_vertex(element, &vertex_id, self.project.min_corner_radius);
                    self.sprite.layers[layer_idx].elements.remove(elem_idx);
                    for (i, new_elem) in result.new_elements.into_iter().enumerate() {
                        self.sprite.layers[layer_idx].elements.insert(elem_idx + i, new_elem);
                    }
                    self.history.push("Erase vertex".into(), before, self.sprite.clone());
                }
            }
            action::AppAction::EraseSegment { element_id, segment_index } => {
                if let Some((layer_idx, elem_idx)) = find_element_location(&self.sprite, &element_id) {
                    let element = &self.sprite.layers[layer_idx].elements[elem_idx];
                    let result = engine::eraser::erase_segment(element, segment_index, self.project.min_corner_radius);
                    self.sprite.layers[layer_idx].elements.remove(elem_idx);
                    for (i, new_elem) in result.new_elements.into_iter().enumerate() {
                        self.sprite.layers[layer_idx].elements.insert(elem_idx + i, new_elem);
                    }
                    self.history.push("Erase segment".into(), before, self.sprite.clone());
                }
            }
            action::AppAction::AddPaletteColor(color) => {
                if self.project.palette.colors.len() < 256 {
                    self.project.palette.colors.push(color);
                    io::save_app_defaults(&self.project);
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
                        if let Some(ref mut grad) = elem.gradient_fill {
                            grad.color_index_start = remap_color_index(grad.color_index_start, index);
                            grad.color_index_end = remap_color_index(grad.color_index_end, index);
                        }
                    }
                }
                self.sprite.background_color_index = remap_color_index(self.sprite.background_color_index, index);
                self.history.push("Delete palette color".into(), before, self.sprite.clone());
                io::save_app_defaults(&self.project);
            }
            action::AppAction::EditPaletteColor { index, color } => {
                if let Some(c) = self.project.palette.colors.get_mut(index as usize) {
                    *c = color;
                }
                io::save_app_defaults(&self.project);
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
                // Save as application defaults
                io::save_app_defaults(&self.project);
                // Project-level, no sprite undo
            }
            action::AppAction::AddReferenceImage(ref_image) => {
                self.sprite.reference_images.push(ref_image);
                self.history.push("Add reference image".into(), before, self.sprite.clone());
            }
            action::AppAction::RemoveReferenceImage(id) => {
                self.sprite.reference_images.retain(|r| r.id != id);
                self.ref_image_textures.remove(&id);
                self.history.push("Remove reference image".into(), before, self.sprite.clone());
            }

            // ── Phase 6: Gradient & Hatch Fills ─────────────────────────

            action::AppAction::SetGradientFill { element_id, gradient_fill } => {
                for layer in &mut self.sprite.layers {
                    for elem in &mut layer.elements {
                        if elem.id == element_id {
                            elem.gradient_fill = Some(gradient_fill.clone());
                        }
                    }
                }
                self.history.push("Set gradient fill".into(), before, self.sprite.clone());
            }
            action::AppAction::ClearGradientFill { element_id } => {
                for layer in &mut self.sprite.layers {
                    for elem in &mut layer.elements {
                        if elem.id == element_id {
                            elem.gradient_fill = None;
                        }
                    }
                }
                self.history.push("Clear gradient fill".into(), before, self.sprite.clone());
            }
            action::AppAction::SetHatchFill { element_id, hatch_fill_id } => {
                for layer in &mut self.sprite.layers {
                    for elem in &mut layer.elements {
                        if elem.id == element_id {
                            elem.hatch_fill_id = Some(hatch_fill_id.clone());
                        }
                    }
                }
                self.history.push("Set hatch fill".into(), before, self.sprite.clone());
            }
            action::AppAction::ClearHatchFill { element_id } => {
                for layer in &mut self.sprite.layers {
                    for elem in &mut layer.elements {
                        if elem.id == element_id {
                            elem.hatch_fill_id = None;
                        }
                    }
                }
                self.history.push("Clear hatch fill".into(), before, self.sprite.clone());
            }
            action::AppAction::SetFlowCurve { element_id, flow_curve } => {
                for layer in &mut self.sprite.layers {
                    for elem in &mut layer.elements {
                        if elem.id == element_id {
                            elem.hatch_flow_curve = Some(flow_curve.clone());
                        }
                    }
                }
                self.history.push("Set flow curve".into(), before, self.sprite.clone());
            }
            action::AppAction::ClearFlowCurve { element_id } => {
                for layer in &mut self.sprite.layers {
                    for elem in &mut layer.elements {
                        if elem.id == element_id {
                            elem.hatch_flow_curve = None;
                        }
                    }
                }
                self.history.push("Clear flow curve".into(), before, self.sprite.clone());
            }
            action::AppAction::AddHatchPattern(pattern) => {
                self.project.hatch_patterns.push(pattern);
                // Project-level, no sprite undo
            }
            action::AppAction::UpdateHatchPattern(pattern) => {
                if let Some(p) = self.project.hatch_patterns.iter_mut().find(|p| p.id == pattern.id) {
                    *p = pattern;
                }
                // Project-level, no sprite undo
            }
            action::AppAction::DeleteHatchPattern(id) => {
                self.project.hatch_patterns.retain(|p| p.id != id);
                // Clear references on all elements
                for layer in &mut self.sprite.layers {
                    for elem in &mut layer.elements {
                        if elem.hatch_fill_id.as_deref() == Some(id.as_str()) {
                            elem.hatch_fill_id = None;
                            elem.hatch_flow_curve = None;
                        }
                    }
                }
                self.history.push("Delete hatch pattern".into(), before, self.sprite.clone());
            }
            action::AppAction::ImportHatchPatterns(patterns) => {
                for pattern in patterns {
                    // Skip duplicates by name
                    if !self.project.hatch_patterns.iter().any(|p| p.name == pattern.name) {
                        self.project.hatch_patterns.push(pattern);
                    }
                }
                // Project-level, no sprite undo
            }
            action::AppAction::AddHatchMask { element_id, mask_polygon } => {
                for layer in &mut self.sprite.layers {
                    for elem in &mut layer.elements {
                        if elem.id == element_id {
                            elem.hatch_masks.push(mask_polygon.clone());
                        }
                    }
                }
                self.history.push("Add hatch mask".into(), before, self.sprite.clone());
            }
            action::AppAction::ClearHatchMasks { element_id } => {
                for layer in &mut self.sprite.layers {
                    for elem in &mut layer.elements {
                        if elem.id == element_id {
                            elem.hatch_masks.clear();
                        }
                    }
                }
                self.history.push("Clear hatch masks".into(), before, self.sprite.clone());
            }
        }
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

/// Find the layer index and element index of an element by ID.
fn find_element_location(sprite: &Sprite, element_id: &str) -> Option<(usize, usize)> {
    for (li, layer) in sprite.layers.iter().enumerate() {
        for (ei, elem) in layer.elements.iter().enumerate() {
            if elem.id == element_id {
                return Some((li, ei));
            }
        }
    }
    None
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

        // Sync reference image textures
        self.sync_ref_image_textures(ctx);

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
