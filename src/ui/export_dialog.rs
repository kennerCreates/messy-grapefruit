//! Export preview dialog: shows atlas preview + RON metadata summary,
//! allows adjusting settings before confirming export.

use crate::model::project::ExportSettings;

/// State for the export dialog.
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct ExportDialogState {
    /// Whether the dialog is open.
    pub open: bool,
    /// Export settings being edited.
    pub settings: ExportSettings,
    /// Preview summary text (generated from last preview).
    pub summary: String,
    /// Auto-export toggle.
    pub auto_export_enabled: bool,
    /// Watcher active status.
    pub watcher_active: bool,
    /// Last export status message.
    pub last_export_status: Option<String>,
    /// Last time auto-export was triggered (to avoid re-exporting every frame).
    pub last_auto_export_time: Option<std::time::Instant>,
}


/// Actions returned from the export dialog.
pub enum ExportDialogAction {
    /// User confirmed export with current settings.
    ConfirmExport,
    /// User requested a preview refresh.
    RefreshPreview,
    /// User toggled auto-export.
    ToggleAutoExport(bool),
    /// User toggled the file watcher.
    ToggleWatcher(bool),
    /// Dialog was closed.
    Close,
}

/// Draw the export preview dialog.
/// Returns a list of actions to process.
pub fn draw_export_dialog(
    ctx: &egui::Context,
    state: &mut ExportDialogState,
) -> Vec<ExportDialogAction> {
    let mut actions = Vec::new();

    if !state.open {
        return actions;
    }

    let mut is_open = state.open;

    egui::Window::new("Export")
        .open(&mut is_open)
        .resizable(true)
        .default_width(500.0)
        .default_height(600.0)
        .show(ctx, |ui| {
            // Export mode selector
            ui.horizontal(|ui| {
                ui.label("Mode:");
                egui::ComboBox::from_id_salt("export_mode")
                    .selected_text(&state.settings.mode)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut state.settings.mode,
                            "bone".to_string(),
                            "Bone (Runtime Animation)",
                        );
                        ui.selectable_value(
                            &mut state.settings.mode,
                            "spritesheet".to_string(),
                            "Spritesheet",
                        );
                    });
            });

            ui.separator();

            // Settings
            ui.heading("Settings");

            ui.horizontal(|ui| {
                ui.label("FPS:");
                let mut fps = state.settings.fps as f32;
                ui.add(egui::Slider::new(&mut fps, 1.0..=60.0).integer());
                state.settings.fps = fps as u32;
            });

            ui.horizontal(|ui| {
                ui.label("Padding:");
                let mut padding = state.settings.padding as f32;
                ui.add(egui::Slider::new(&mut padding, 0.0..=8.0).integer());
                state.settings.padding = padding as u32;
            });

            ui.horizontal(|ui| {
                ui.label("Trim:");
                ui.checkbox(&mut state.settings.trim, "Trim transparent borders");
            });

            if state.settings.mode == "spritesheet" {
                ui.horizontal(|ui| {
                    ui.label("Layout:");
                    egui::ComboBox::from_id_salt("export_layout")
                        .selected_text(&state.settings.layout)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut state.settings.layout,
                                "row".to_string(),
                                "Row",
                            );
                            ui.selectable_value(
                                &mut state.settings.layout,
                                "column".to_string(),
                                "Column",
                            );
                            ui.selectable_value(
                                &mut state.settings.layout,
                                "grid".to_string(),
                                "Grid",
                            );
                        });
                });
            }

            ui.separator();

            // Auto-export and watcher toggles
            ui.heading("Automation");

            let mut auto = state.auto_export_enabled;
            if ui.checkbox(&mut auto, "Auto-export on save").changed() {
                actions.push(ExportDialogAction::ToggleAutoExport(auto));
            }
            state.auto_export_enabled = auto;

            let mut watcher = state.watcher_active;
            if ui
                .checkbox(&mut watcher, "File watcher (re-export on .sprite change)")
                .changed()
            {
                actions.push(ExportDialogAction::ToggleWatcher(watcher));
            }
            state.watcher_active = watcher;

            ui.separator();

            // Preview summary
            ui.heading("Preview");

            if ui.button("Refresh Preview").clicked() {
                actions.push(ExportDialogAction::RefreshPreview);
            }

            if !state.summary.is_empty() {
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        ui.monospace(&state.summary);
                    });
            }

            // Last export status
            if let Some(ref status) = state.last_export_status {
                ui.separator();
                ui.label(status);
            }

            ui.separator();

            // Action buttons
            ui.horizontal(|ui| {
                if ui.button("Export").clicked() {
                    actions.push(ExportDialogAction::ConfirmExport);
                }
                if ui.button("Close").clicked() {
                    actions.push(ExportDialogAction::Close);
                }
            });
        });

    if !is_open {
        state.open = false;
        actions.push(ExportDialogAction::Close);
    }

    actions
}
