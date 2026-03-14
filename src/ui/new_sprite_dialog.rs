//! New sprite dialog: name field, canvas size presets, freeform input.

/// State for the new sprite dialog.
#[derive(Debug, Clone)]
pub struct NewSpriteDialogState {
    /// Whether the dialog is open.
    pub open: bool,
    /// Sprite name.
    pub name: String,
    /// Canvas width.
    pub width: u32,
    /// Canvas height.
    pub height: u32,
    /// Currently selected preset index (None = freeform).
    pub selected_preset: Option<usize>,
}

impl Default for NewSpriteDialogState {
    fn default() -> Self {
        Self {
            open: false,
            name: "New Sprite".to_string(),
            width: 256,
            height: 256,
            selected_preset: Some(2), // 256x256 default
        }
    }
}

impl NewSpriteDialogState {
    pub fn reset(&mut self) {
        self.name = "New Sprite".to_string();
        self.width = 256;
        self.height = 256;
        self.selected_preset = Some(2);
    }
}

/// Actions the new sprite dialog can return.
pub enum NewSpriteDialogAction {
    /// User confirmed creation.
    Create { name: String, width: u32, height: u32 },
    /// User cancelled / closed.
    Close,
}

/// Canvas size presets.
const PRESETS: &[(u32, u32, &str)] = &[
    (64, 64, "64 x 64"),
    (128, 128, "128 x 128"),
    (256, 256, "256 x 256"),
    (512, 512, "512 x 512"),
];

/// Draw the new sprite dialog.
pub fn draw_new_sprite_dialog(
    ctx: &egui::Context,
    state: &mut NewSpriteDialogState,
) -> Vec<NewSpriteDialogAction> {
    let mut actions = Vec::new();

    if !state.open {
        return actions;
    }

    let mut is_open = state.open;

    egui::Window::new("New Sprite")
        .open(&mut is_open)
        .resizable(false)
        .collapsible(false)
        .default_width(300.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                // Name field
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut state.name);
                });

                ui.add_space(8.0);

                // Size presets
                ui.label("Canvas Size:");
                ui.horizontal(|ui| {
                    for (idx, (w, h, label)) in PRESETS.iter().enumerate() {
                        let is_selected = state.selected_preset == Some(idx);
                        if ui.selectable_label(is_selected, *label).clicked() {
                            state.selected_preset = Some(idx);
                            state.width = *w;
                            state.height = *h;
                        }
                    }
                    let is_freeform = state.selected_preset.is_none();
                    if ui.selectable_label(is_freeform, "Custom").clicked() {
                        state.selected_preset = None;
                    }
                });

                ui.add_space(4.0);

                // Freeform input
                let freeform = state.selected_preset.is_none();
                ui.horizontal(|ui| {
                    ui.label("Width:");
                    let mut w = state.width as f32;
                    let resp = ui.add_enabled(
                        freeform,
                        egui::DragValue::new(&mut w)
                            .range(1..=4096)
                            .speed(1.0),
                    );
                    if resp.changed() {
                        state.width = w as u32;
                    }

                    ui.label("Height:");
                    let mut h = state.height as f32;
                    let resp = ui.add_enabled(
                        freeform,
                        egui::DragValue::new(&mut h)
                            .range(1..=4096)
                            .speed(1.0),
                    );
                    if resp.changed() {
                        state.height = h as u32;
                    }
                });

                ui.add_space(12.0);

                // Buttons
                ui.horizontal(|ui| {
                    let name_valid = !state.name.trim().is_empty();
                    let size_valid = state.width > 0 && state.height > 0;

                    if ui
                        .add_enabled(name_valid && size_valid, egui::Button::new("Create"))
                        .clicked()
                    {
                        actions.push(NewSpriteDialogAction::Create {
                            name: state.name.trim().to_string(),
                            width: state.width,
                            height: state.height,
                        });
                    }
                    if ui.button("Cancel").clicked() {
                        actions.push(NewSpriteDialogAction::Close);
                    }
                });
            });
        });

    if !is_open {
        state.open = false;
        actions.push(NewSpriteDialogAction::Close);
    }

    actions
}
