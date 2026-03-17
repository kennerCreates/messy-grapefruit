use crate::action::AppAction;
use crate::model::project::Theme;
use crate::model::sprite::Sprite;
use crate::state::editor::EditorState;
use crate::theme;

use super::icons;

/// Show the reference images panel in the sidebar.
pub fn show_reference_images(
    ui: &mut egui::Ui,
    editor: &mut EditorState,
    sprite: &mut Sprite,
    theme_mode: Theme,
    actions: &mut Vec<AppAction>,
) {
    if sprite.reference_images.is_empty() {
        return;
    }

    ui.separator();
    ui.label("Reference Images");

    let mut to_remove: Option<String> = None;

    for ref_img in &mut sprite.reference_images {
        let is_selected = editor.selected_ref_image_id.as_deref() == Some(&ref_img.id);

        ui.horizontal(|ui| {
            // Visibility toggle
            let vis_icon = if ref_img.visible { icons::layer_visible() } else { icons::layer_hidden() };
            if ui.add(icons::small_icon_button(vis_icon, ui)).on_hover_text("Visibility").clicked() {
                ref_img.visible = !ref_img.visible;
            }

            // Lock toggle
            let lock_icon = if ref_img.locked { icons::layer_locked() } else { icons::layer_unlocked() };
            if ui.add(icons::small_icon_button(lock_icon, ui)).on_hover_text("Lock").clicked() {
                ref_img.locked = !ref_img.locked;
            }

            // Filename label (selectable)
            let name = std::path::Path::new(&ref_img.path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("image");
            if ui.selectable_label(is_selected, name).clicked() {
                editor.selected_ref_image_id = Some(ref_img.id.clone());
            }

            // Delete button
            if ui.add(icons::small_icon_button(icons::layer_remove(), ui)).on_hover_text("Remove").clicked() {
                to_remove = Some(ref_img.id.clone());
            }
        });

        // Opacity slider for selected ref image
        if is_selected {
            ui.horizontal(|ui| {
                ui.label("Opacity");
                theme::with_input_style(ui, theme_mode, |ui| {
                    let mut pct = (ref_img.opacity * 100.0) as i32;
                    if ui.add(egui::Slider::new(&mut pct, 0..=100).suffix("%")).changed() {
                        ref_img.opacity = pct as f32 / 100.0;
                    }
                });
            });
        }
    }

    if let Some(id) = to_remove {
        actions.push(AppAction::RemoveReferenceImage(id));
    }
}
