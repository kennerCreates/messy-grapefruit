use crate::model::project::Theme;
use crate::model::sprite::Sprite;
use crate::theme;

use super::icons;

pub(super) fn show_layer_list(
    ui: &mut egui::Ui,
    sprite: &mut Sprite,
    active_layer_idx: &mut usize,
    theme: Theme,
) {
    // Add layer button
    if ui
        .add(icons::small_icon_button(icons::layer_add(), ui))
        .on_hover_text("Add Layer")
        .clicked()
    {
        let n = sprite.layers.len() + 1;
        sprite
            .layers
            .push(crate::model::sprite::Layer::new(format!("Layer {n}")));
    }

    ui.add_space(4.0);

    let sel_color = theme::selected_color(theme);

    // Layer list (bottom-to-top display, since layers render bottom-to-top)
    let layer_count = sprite.layers.len();
    for display_idx in 0..layer_count {
        let layer_idx = layer_count - 1 - display_idx;
        let is_active = layer_idx == *active_layer_idx;

        ui.horizontal(|ui| {
            // Visibility toggle
            let vis_icon = if sprite.layers[layer_idx].visible {
                icons::layer_visible()
            } else {
                icons::layer_hidden()
            };
            if ui
                .add(icons::small_icon_button(vis_icon, ui))
                .on_hover_text(if sprite.layers[layer_idx].visible {
                    "Hide Layer"
                } else {
                    "Show Layer"
                })
                .clicked()
            {
                sprite.layers[layer_idx].visible = !sprite.layers[layer_idx].visible;
            }

            // Lock toggle
            let lock_icon = if sprite.layers[layer_idx].locked {
                icons::layer_locked()
            } else {
                icons::layer_unlocked()
            };
            if ui
                .add(icons::small_icon_button(lock_icon, ui))
                .on_hover_text(if sprite.layers[layer_idx].locked {
                    "Unlock Layer"
                } else {
                    "Lock Layer"
                })
                .clicked()
            {
                sprite.layers[layer_idx].locked = !sprite.layers[layer_idx].locked;
            }

            // Layer name (clickable to select)
            let label =
                egui::SelectableLabel::new(is_active, &sprite.layers[layer_idx].name);
            let resp = ui.add(label);
            if is_active {
                // Mark active layer with selection color underline
                let rect = resp.rect;
                ui.painter().line_segment(
                    [rect.left_bottom(), rect.right_bottom()],
                    egui::Stroke::new(2.0, sel_color),
                );
            }
            if resp.clicked() {
                *active_layer_idx = layer_idx;
            }
        });
    }
}
