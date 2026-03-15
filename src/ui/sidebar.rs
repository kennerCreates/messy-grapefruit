use crate::model::project::Project;
use crate::model::sprite::Sprite;
use crate::state::editor::EditorState;

use super::icons;

pub fn show_sidebar(
    ui: &mut egui::Ui,
    editor: &mut EditorState,
    sprite: &mut Sprite,
    project: &Project,
    active_layer_idx: &mut usize,
) {
    // Top zone: tool options
    ui.heading("Tool Options");
    ui.separator();

    match editor.tool {
        crate::state::editor::ToolKind::Line => {
            show_line_tool_options(ui, editor, project);
        }
    }

    ui.add_space(16.0);
    ui.separator();

    // Bottom zone: layers
    ui.heading("Layers");
    ui.separator();

    show_layer_list(ui, sprite, active_layer_idx);
}

fn show_line_tool_options(ui: &mut egui::Ui, editor: &mut EditorState, project: &Project) {
    // Stroke width
    ui.horizontal(|ui| {
        ui.label("Width:");
        ui.add(egui::Slider::new(&mut editor.active_stroke_width, 1.0..=32.0).fixed_decimals(1));
    });

    // Active color
    ui.horizontal(|ui| {
        ui.label("Color:");
        let color = project.palette.get_color(editor.active_color_index);
        let (rect, response) = ui.allocate_exact_size(egui::Vec2::splat(20.0), egui::Sense::click());
        ui.painter().rect_filled(rect, 2.0, color.to_color32());
        ui.painter().rect_stroke(rect, 2.0, egui::Stroke::new(1.0, egui::Color32::GRAY), egui::StrokeKind::Outside);

        if response.clicked() {
            // TODO: open palette picker popup
        }
        ui.label(format!("idx {}", editor.active_color_index));
    });

    // Color palette mini-picker
    ui.horizontal_wrapped(|ui| {
        for (i, pc) in project.palette.colors.iter().enumerate() {
            let size = egui::Vec2::splat(16.0);
            let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
            let c32 = pc.to_color32();
            if c32.a() == 0 {
                // Draw checkerboard for transparent
                ui.painter().rect_filled(rect, 1.0, egui::Color32::WHITE);
                let half = rect.size() / 2.0;
                ui.painter().rect_filled(
                    egui::Rect::from_min_size(rect.min, egui::Vec2::new(half.x, half.y)),
                    0.0,
                    egui::Color32::LIGHT_GRAY,
                );
                ui.painter().rect_filled(
                    egui::Rect::from_min_size(
                        rect.min + egui::Vec2::new(half.x, half.y),
                        egui::Vec2::new(half.x, half.y),
                    ),
                    0.0,
                    egui::Color32::LIGHT_GRAY,
                );
            } else {
                ui.painter().rect_filled(rect, 1.0, c32);
            }

            if editor.active_color_index == i as u8 {
                ui.painter().rect_stroke(rect, 1.0, egui::Stroke::new(2.0, egui::Color32::WHITE), egui::StrokeKind::Outside);
            }

            if response.clicked() {
                editor.active_color_index = i as u8;
            }
            if response.hovered() {
                response.on_hover_text(format!("Color {i}"));
            }
        }
    });

    // Curve/straight toggle
    ui.horizontal(|ui| {
        if ui
            .add(icons::icon_button(icons::mode_curve()).selected(editor.line_tool.curve_mode))
            .on_hover_text("Curve Mode (C)")
            .clicked()
        {
            editor.line_tool.curve_mode = true;
        }
        if ui
            .add(icons::icon_button(icons::mode_straight()).selected(!editor.line_tool.curve_mode))
            .on_hover_text("Straight Mode")
            .clicked()
        {
            editor.line_tool.curve_mode = false;
        }
    });
}

fn show_layer_list(ui: &mut egui::Ui, sprite: &mut Sprite, active_layer_idx: &mut usize) {
    // Add layer button
    if ui
        .add(icons::icon_button(icons::layer_add()))
        .on_hover_text("Add Layer")
        .clicked()
    {
        let n = sprite.layers.len() + 1;
        sprite.layers.push(crate::model::sprite::Layer::new(format!("Layer {n}")));
    }

    ui.add_space(4.0);

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
                .add(icons::icon_button(vis_icon))
                .on_hover_text(if sprite.layers[layer_idx].visible { "Hide Layer" } else { "Show Layer" })
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
                .add(icons::icon_button(lock_icon))
                .on_hover_text(if sprite.layers[layer_idx].locked { "Unlock Layer" } else { "Lock Layer" })
                .clicked()
            {
                sprite.layers[layer_idx].locked = !sprite.layers[layer_idx].locked;
            }

            // Layer name (clickable to select)
            let label = egui::SelectableLabel::new(
                is_active,
                &sprite.layers[layer_idx].name,
            );
            if ui.add(label).clicked() {
                *active_layer_idx = layer_idx;
            }
        });
    }
}
