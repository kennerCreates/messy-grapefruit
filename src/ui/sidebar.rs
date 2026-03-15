use crate::model::project::{GridMode, Project, Theme};
use crate::model::sprite::Sprite;
use crate::state::editor::EditorState;
use crate::theme;

use super::icons;

pub fn show_sidebar(
    ui: &mut egui::Ui,
    editor: &mut EditorState,
    sprite: &mut Sprite,
    project: &mut Project,
) {
    if editor.sidebar_expanded {
        show_expanded(ui, editor, sprite, project);
    } else {
        show_collapsed(ui, editor, sprite, project);
    }
}

/// Collapsed sidebar — narrow strip with essential controls stacked vertically.
fn show_collapsed(
    ui: &mut egui::Ui,
    editor: &mut EditorState,
    sprite: &mut Sprite,
    project: &mut Project,
) {
    ui.spacing_mut().item_spacing.y = 6.0;

    // Expand toggle (kept small)
    if ui
        .add(icons::sidebar_toggle_button(icons::sidebar_expand(), ui))
        .on_hover_text("Expand Sidebar")
        .clicked()
    {
        editor.sidebar_expanded = true;
    }

    ui.add_space(6.0);

    // Theme toggle (flip-flop)
    let is_dark = project.editor_preferences.theme == Theme::Dark;
    let theme_icon = if is_dark { icons::theme_dark() } else { icons::theme_light() };
    if ui
        .add(icons::icon_button(theme_icon, ui))
        .on_hover_text(if is_dark { "Switch to Light" } else { "Switch to Dark" })
        .clicked()
    {
        project.editor_preferences.theme = if is_dark { Theme::Light } else { Theme::Dark };
    }

    ui.add_space(4.0);

    // Curve/straight toggle
    if editor.line_tool.curve_mode {
        if ui
            .add(icons::icon_button(icons::mode_curve(), ui))
            .on_hover_text("Curve Mode (C)")
            .clicked()
        {
            editor.line_tool.curve_mode = false;
        }
    } else if ui
        .add(icons::icon_button(icons::mode_straight(), ui))
        .on_hover_text("Straight Mode (C)")
        .clicked()
    {
        editor.line_tool.curve_mode = true;
    }

    ui.add_space(4.0);

    // Stroke width: icon + drag value on same line
    ui.horizontal(|ui| {
        ui.add(icons::small_icon(icons::prop_width(), ui));
        ui.add(
            egui::DragValue::new(&mut editor.active_stroke_width)
                .range(1.0..=32.0)
                .speed(0.1)
                .fixed_decimals(1),
        );
    });

    ui.add_space(4.0);

    // Corner radius: icon + drag value on same line
    let radius_changed = ui
        .horizontal(|ui| {
            ui.add(icons::small_icon(icons::prop_radius(), ui));
            ui.add(
                egui::DragValue::new(&mut project.min_corner_radius)
                    .range(0.0..=32.0)
                    .speed(0.1)
                    .fixed_decimals(1),
            )
            .changed()
        })
        .inner;

    if radius_changed {
        for layer in &mut sprite.layers {
            for element in &mut layer.elements {
                crate::math::recompute_auto_curves(
                    &mut element.vertices,
                    element.closed,
                    element.curve_mode,
                    project.min_corner_radius,
                );
            }
        }
    }

    ui.add_space(4.0);

    // Active color swatch only (no palette)
    let color = project.palette.get_color(editor.active_color_index);
    let (rect, _response) = ui.allocate_exact_size(egui::Vec2::splat(20.0), egui::Sense::click());
    let c32 = color.to_color32();
    if c32.a() == 0 {
        draw_checkerboard(ui, rect);
    } else {
        ui.painter().rect_filled(rect, 2.0, c32);
    }
    let sel_color = theme::selected_color(project.editor_preferences.theme);
    ui.painter().rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(1.0, sel_color),
        egui::StrokeKind::Outside,
    );

    ui.add_space(4.0);

    // Grid icons (vertical, no size dropdown)
    if ui
        .add(icons::icon_button(icons::grid_dots(), ui).selected(project.editor_preferences.show_dots))
        .on_hover_text("Grid Dots")
        .clicked()
    {
        project.editor_preferences.show_dots = !project.editor_preferences.show_dots;
    }

    let is_straight = project.editor_preferences.grid_mode == GridMode::Straight;
    if ui
        .add(icons::icon_button(icons::grid_lines(), ui).selected(is_straight))
        .on_hover_text("Straight Grid")
        .clicked()
    {
        project.editor_preferences.grid_mode = if is_straight {
            GridMode::Off
        } else {
            GridMode::Straight
        };
    }

    let is_iso = project.editor_preferences.grid_mode == GridMode::Isometric;
    if ui
        .add(icons::icon_button(icons::grid_iso(), ui).selected(is_iso))
        .on_hover_text("Isometric Grid")
        .clicked()
    {
        project.editor_preferences.grid_mode = if is_iso {
            GridMode::Off
        } else {
            GridMode::Isometric
        };
    }

    ui.add_space(4.0);

    // Layer list (select only, no lock/visible toggles)
    let layer_count = sprite.layers.len();
    for display_idx in 0..layer_count {
        let layer_idx = layer_count - 1 - display_idx;
        let is_active = layer_idx == editor.active_layer_idx;

        let label = egui::SelectableLabel::new(is_active, &sprite.layers[layer_idx].name);
        let resp = ui.add(label);
        if is_active {
            let rect = resp.rect;
            ui.painter().line_segment(
                [rect.left_bottom(), rect.right_bottom()],
                egui::Stroke::new(2.0, sel_color),
            );
        }
        if resp.clicked() {
            editor.active_layer_idx = layer_idx;
        }
    }
}

/// Expanded sidebar — full settings panel.
fn show_expanded(
    ui: &mut egui::Ui,
    editor: &mut EditorState,
    sprite: &mut Sprite,
    project: &mut Project,
) {
    ui.spacing_mut().item_spacing.y = 6.0;

    // Collapse toggle (kept small)
    if ui
        .add(icons::sidebar_toggle_button(icons::sidebar_collapse(), ui))
        .on_hover_text("Collapse Sidebar")
        .clicked()
    {
        editor.sidebar_expanded = false;
    }

    ui.add_space(8.0);

    // Theme toggle
    ui.horizontal(|ui| {
        let is_dark = project.editor_preferences.theme == Theme::Dark;
        if ui
            .add(icons::icon_button(icons::theme_dark(), ui).selected(is_dark))
            .on_hover_text("Dark Theme")
            .clicked()
        {
            project.editor_preferences.theme = Theme::Dark;
        }
        if ui
            .add(icons::icon_button(icons::theme_light(), ui).selected(!is_dark))
            .on_hover_text("Light Theme")
            .clicked()
        {
            project.editor_preferences.theme = Theme::Light;
        }
    });

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(10.0);

    // Tool options
    show_line_tool_options(ui, editor, sprite, project);

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(10.0);

    // Grid controls
    ui.label("Grid");
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        if ui
            .add(icons::icon_button(icons::grid_dots(), ui).selected(project.editor_preferences.show_dots))
            .on_hover_text("Grid Dots")
            .clicked()
        {
            project.editor_preferences.show_dots = !project.editor_preferences.show_dots;
        }
    });

    ui.horizontal(|ui| {
        let is_straight = project.editor_preferences.grid_mode == GridMode::Straight;
        if ui
            .add(icons::icon_button(icons::grid_lines(), ui).selected(is_straight))
            .on_hover_text("Straight Grid")
            .clicked()
        {
            project.editor_preferences.grid_mode = if is_straight {
                GridMode::Off
            } else {
                GridMode::Straight
            };
        }

        let is_iso = project.editor_preferences.grid_mode == GridMode::Isometric;
        if ui
            .add(icons::icon_button(icons::grid_iso(), ui).selected(is_iso))
            .on_hover_text("Isometric Grid")
            .clicked()
        {
            project.editor_preferences.grid_mode = if is_iso {
                GridMode::Off
            } else {
                GridMode::Isometric
            };
        }
    });

    ui.horizontal(|ui| {
        ui.label("Size");
        let grid_sizes: &[u32] = &[1, 2, 4, 8, 16, 32, 64];
        egui::ComboBox::from_id_salt("grid_size_sb")
            .selected_text(format!("{}", project.editor_preferences.grid_size))
            .width(50.0)
            .show_ui(ui, |ui| {
                for &size in grid_sizes {
                    ui.selectable_value(
                        &mut project.editor_preferences.grid_size,
                        size,
                        format!("{size}"),
                    );
                }
            });
    });

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(10.0);

    // Layer list
    ui.label("Layers");
    ui.add_space(4.0);
    show_layer_list(ui, sprite, &mut editor.active_layer_idx, project.editor_preferences.theme);
}

fn show_line_tool_options(
    ui: &mut egui::Ui,
    editor: &mut EditorState,
    sprite: &mut Sprite,
    project: &mut Project,
) {
    // Stroke width
    ui.horizontal(|ui| {
        ui.add(icons::small_icon(icons::prop_width(), ui));
        ui.label("Width");
        ui.add(egui::Slider::new(&mut editor.active_stroke_width, 1.0..=32.0).fixed_decimals(1));
    });

    ui.add_space(4.0);

    // Active color
    ui.horizontal(|ui| {
        ui.label("Color");
        let color = project.palette.get_color(editor.active_color_index);
        let (rect, response) =
            ui.allocate_exact_size(egui::Vec2::splat(20.0), egui::Sense::click());
        ui.painter().rect_filled(rect, 2.0, color.to_color32());
        ui.painter().rect_stroke(
            rect,
            2.0,
            egui::Stroke::new(1.0, egui::Color32::GRAY),
            egui::StrokeKind::Outside,
        );
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
                draw_checkerboard(ui, rect);
            } else {
                ui.painter().rect_filled(rect, 1.0, c32);
            }
            if editor.active_color_index == i as u8 {
                let sel_color = theme::selected_color(project.editor_preferences.theme);
                ui.painter().rect_stroke(
                    rect,
                    1.0,
                    egui::Stroke::new(2.0, sel_color),
                    egui::StrokeKind::Outside,
                );
            }
            if response.clicked() {
                editor.active_color_index = i as u8;
            }
            if response.hovered() {
                response.on_hover_text(format!("Color {i}"));
            }
        }
    });

    ui.add_space(4.0);

    // Min corner radius
    let radius_changed = ui
        .horizontal(|ui| {
            ui.add(icons::small_icon(icons::prop_radius(), ui));
            ui.label("Radius");
            ui.add(
                egui::Slider::new(&mut project.min_corner_radius, 0.0..=32.0).fixed_decimals(1),
            )
            .changed()
        })
        .inner;

    if radius_changed {
        for layer in &mut sprite.layers {
            for element in &mut layer.elements {
                crate::math::recompute_auto_curves(
                    &mut element.vertices,
                    element.closed,
                    element.curve_mode,
                    project.min_corner_radius,
                );
            }
        }
    }

    ui.add_space(4.0);

    // Curve/straight toggle
    ui.horizontal(|ui| {
        if ui
            .add(icons::icon_button(icons::mode_curve(), ui).selected(editor.line_tool.curve_mode))
            .on_hover_text("Curve Mode (C)")
            .clicked()
        {
            editor.line_tool.curve_mode = true;
        }
        if ui
            .add(icons::icon_button(icons::mode_straight(), ui).selected(!editor.line_tool.curve_mode))
            .on_hover_text("Straight Mode")
            .clicked()
        {
            editor.line_tool.curve_mode = false;
        }
    });
}

fn show_layer_list(
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

fn draw_checkerboard(ui: &egui::Ui, rect: egui::Rect) {
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
}
