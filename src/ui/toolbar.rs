use crate::model::sprite::Skin;
use crate::state::editor::ToolKind;

/// Skin info passed to the toolbar for rendering the dropdown
pub struct ToolbarSkinInfo {
    pub active_skin_id: Option<String>,
    pub skins: Vec<Skin>,
}

/// Draw the top toolbar with tool buttons.
/// Returns the newly selected tool if it changed, otherwise None.
pub fn draw_toolbar(
    ctx: &egui::Context,
    active_tool: ToolKind,
    curve_mode: bool,
    skin_info: Option<&ToolbarSkinInfo>,
) -> Option<ToolAction> {
    let mut action = None;

    egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.heading("Sprite Tool");
            ui.separator();

            let tools = [
                (ToolKind::Line, "\u{270F} Line", "1"),
                (ToolKind::Select, "\u{25CB} Select", "2"),
                (ToolKind::Fill, "\u{25A0} Fill", "3"),
                (ToolKind::Eraser, "\u{2716} Eraser", "4"),
            ];

            for (tool, label, shortcut) in &tools {
                let is_active = active_tool == *tool;
                let button = egui::Button::new(format!("{} [{}]", label, shortcut))
                    .selected(is_active);

                if ui.add(button).clicked() {
                    action = Some(ToolAction::SelectTool(*tool));
                }
            }

            ui.separator();

            // Curve mode toggle (only relevant for line tool)
            if active_tool == ToolKind::Line {
                let mode_label = if curve_mode { "Curve" } else { "Straight" };
                if ui
                    .add(egui::Button::new(format!("\u{27B0} {} [C]", mode_label)))
                    .clicked()
                {
                    action = Some(ToolAction::ToggleCurveMode);
                }
            }

            // Export button
            ui.separator();
            if ui.button("\u{1F4E6} Export").clicked() {
                action = Some(ToolAction::OpenExportDialog);
            }

            // Skin selector dropdown (right side of toolbar)
            if let Some(skin_info) = skin_info {
                ui.separator();

                let current_label = match &skin_info.active_skin_id {
                    None => "Skin: Default".to_string(),
                    Some(id) => skin_info
                        .skins
                        .iter()
                        .find(|s| s.id == *id)
                        .map(|s| format!("Skin: {}", s.name))
                        .unwrap_or_else(|| "Skin: Unknown".to_string()),
                };

                egui::ComboBox::from_id_salt("toolbar_skin_selector")
                    .selected_text(&current_label)
                    .show_ui(ui, |ui| {
                        let is_default = skin_info.active_skin_id.is_none();
                        if ui.selectable_label(is_default, "Default (Base)").clicked() {
                            action = Some(ToolAction::SetActiveSkin(None));
                        }
                        for skin in &skin_info.skins {
                            let is_selected = skin_info.active_skin_id.as_ref() == Some(&skin.id);
                            if ui.selectable_label(is_selected, &skin.name).clicked() {
                                action = Some(ToolAction::SetActiveSkin(Some(skin.id.clone())));
                            }
                        }
                    });
            }
        });
    });

    action
}

pub enum ToolAction {
    SelectTool(ToolKind),
    ToggleCurveMode,
    SetActiveSkin(Option<String>),
    OpenExportDialog,
}
