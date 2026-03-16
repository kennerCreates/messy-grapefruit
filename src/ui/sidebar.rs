use crate::model::project::{Project, Theme};
use crate::model::sprite::Sprite;
use crate::state::editor::{EditorState, ToolKind};
use crate::state::history::History;
use crate::theme;

use super::icons;

pub fn show_sidebar(
    ui: &mut egui::Ui,
    editor: &mut EditorState,
    sprite: &mut Sprite,
    project: &mut Project,
    history: &mut History,
) {
    if editor.sidebar_expanded {
        show_expanded(ui, editor, sprite, project, history);
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
    ui.set_min_width(ui.available_width());

    ui.horizontal(|ui| {
        // Expand toggle (kept small)
        if ui
            .add(icons::sidebar_toggle_button(icons::sidebar_expand(), ui))
            .on_hover_text("Expand Sidebar")
            .clicked()
        {
            editor.sidebar_expanded = true;
        }

        // Theme toggle (flip-flop, small)
        let is_dark = project.editor_preferences.theme == Theme::Dark;
        let theme_icon = if is_dark { icons::theme_dark() } else { icons::theme_light() };
        if ui
            .add(icons::sidebar_toggle_button(theme_icon, ui))
            .on_hover_text(if is_dark { "Switch to Light" } else { "Switch to Dark" })
            .clicked()
        {
            project.editor_preferences.theme = if is_dark { Theme::Light } else { Theme::Dark };
        }
    });

    ui.add_space(4.0);

    if matches!(editor.tool, ToolKind::Line) {
        // Curve/straight toggle
        let mut mode_changed = false;
        if editor.line_tool.curve_mode {
            if ui
                .add(icons::icon_button(icons::mode_curve(), ui))
                .on_hover_text("Curve Mode (C)")
                .clicked()
            {
                editor.line_tool.curve_mode = false;
                mode_changed = true;
            }
        } else if ui
            .add(icons::icon_button(icons::mode_straight(), ui))
            .on_hover_text("Straight Mode (C)")
            .clicked()
        {
            editor.line_tool.curve_mode = true;
            mode_changed = true;
        }
        if mode_changed && editor.line_tool.is_drawing {
            crate::math::recompute_auto_curves(
                &mut editor.line_tool.vertices,
                false,
                editor.line_tool.curve_mode,
                project.min_corner_radius,
            );
        }

        ui.add_space(4.0);

        // Stroke width: show current value only
        ui.horizontal(|ui| {
            ui.add(icons::small_icon(icons::prop_width(), ui));
            ui.label(format!("{}", editor.active_stroke_width as u32));
        });

        ui.add_space(4.0);

        // Corner radius: icon + read-only value
        ui.horizontal(|ui| {
            ui.add(icons::small_icon(icons::prop_radius(), ui));
            ui.label(format!("{}", project.min_corner_radius as u32));
        });

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
    } else if matches!(editor.tool, ToolKind::Select) && !editor.selection.is_empty() {
        // Collapsed select tool: show selected element properties (compact)
        let selected = &editor.selection.selected_ids;
        let first_elem = sprite.layers.iter().flat_map(|l| &l.elements)
            .find(|e| selected.iter().any(|id| id == &e.id));

        if let Some(elem) = first_elem {
            // Position
            ui.horizontal(|ui| {
                ui.add(icons::small_icon(icons::prop_position(), ui));
                ui.label(format!("{:02},{:02}", elem.position.x as i32, elem.position.y as i32));
            });

            // Rotation
            ui.horizontal(|ui| {
                ui.add(icons::small_icon(icons::prop_rotation(), ui));
                ui.label(format!("{}°", elem.rotation.to_degrees() as i32));
            });

            // Scale
            ui.horizontal(|ui| {
                ui.add(icons::small_icon(icons::prop_scale(), ui));
                ui.label(format!("{},{}", elem.scale.x as i32, elem.scale.y as i32));
            });

            // Stroke width
            ui.horizontal(|ui| {
                ui.add(icons::small_icon(icons::prop_width(), ui));
                ui.label(format!("{}", elem.stroke_width as u32));
            });

            // Color swatch
            let color = project.palette.get_color(elem.stroke_color_index);
            let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(20.0), egui::Sense::hover());
            let c32 = color.to_color32();
            if c32.a() == 0 {
                draw_checkerboard(ui, rect);
            } else {
                ui.painter().rect_filled(rect, 2.0, c32);
            }
            let sel_color = theme::selected_color(project.editor_preferences.theme);
            ui.painter().rect_stroke(rect, 2.0, egui::Stroke::new(1.0, sel_color), egui::StrokeKind::Outside);

            ui.add_space(4.0);
        }
    }

    // Layer list
    let sel_color = theme::selected_color(project.editor_preferences.theme);
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
    history: &mut History,
) {
    ui.spacing_mut().item_spacing.y = 6.0;
    // Force the layout to use full available width
    ui.set_min_width(ui.available_width());

    ui.horizontal(|ui| {
        // Collapse toggle (kept small)
        if ui
            .add(icons::sidebar_toggle_button(icons::sidebar_collapse(), ui))
            .on_hover_text("Collapse Sidebar")
            .clicked()
        {
            editor.sidebar_expanded = false;
        }

        // Theme toggle (small, same line)
        let is_dark = project.editor_preferences.theme == Theme::Dark;
        if ui
            .add(icons::sidebar_toggle_button(icons::theme_dark(), ui).selected(is_dark))
            .on_hover_text("Dark Theme")
            .clicked()
        {
            project.editor_preferences.theme = Theme::Dark;
        }
        if ui
            .add(icons::sidebar_toggle_button(icons::theme_light(), ui).selected(!is_dark))
            .on_hover_text("Light Theme")
            .clicked()
        {
            project.editor_preferences.theme = Theme::Light;
        }
    });

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(10.0);

    // Tool-specific options
    match editor.tool {
        ToolKind::Line => {
            show_line_tool_options(ui, editor, sprite, project);
        }
        ToolKind::Select => {
            show_select_tool_options(ui, editor, sprite, project, history);
        }
    }

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
    // Stroke width: 3 toggle buttons
    ui.horizontal(|ui| {
        ui.add(icons::small_icon(icons::prop_width(), ui));
        ui.label("Width");
        for &w in &[1.0_f32, 2.0, 4.0] {
            let selected = (editor.active_stroke_width - w).abs() < 0.01;
            if ui.selectable_label(selected, format!("{}", w as u32)).clicked() {
                editor.active_stroke_width = w;
            }
        }
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
    let radius_changed = theme::with_input_style(ui, project.editor_preferences.theme, |ui| {
        ui.horizontal(|ui| {
            ui.add(icons::small_icon(icons::prop_radius(), ui));
            ui.label("Radius");
            ui.add(
                egui::Slider::new(&mut project.min_corner_radius, 0.0..=32.0).fixed_decimals(1),
            )
            .changed()
        })
        .inner
    });

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
        let mut mode_changed = false;
        if ui
            .add(icons::icon_button(icons::mode_curve(), ui).selected(editor.line_tool.curve_mode))
            .on_hover_text("Curve Mode (C)")
            .clicked()
            && !editor.line_tool.curve_mode
        {
            editor.line_tool.curve_mode = true;
            mode_changed = true;
        }
        if ui
            .add(icons::icon_button(icons::mode_straight(), ui).selected(!editor.line_tool.curve_mode))
            .on_hover_text("Straight Mode")
            .clicked()
            && editor.line_tool.curve_mode
        {
            editor.line_tool.curve_mode = false;
            mode_changed = true;
        }
        if mode_changed && editor.line_tool.is_drawing {
            crate::math::recompute_auto_curves(
                &mut editor.line_tool.vertices,
                false,
                editor.line_tool.curve_mode,
                project.min_corner_radius,
            );
        }
    });
}

fn show_select_tool_options(
    ui: &mut egui::Ui,
    editor: &mut EditorState,
    sprite: &mut Sprite,
    project: &mut Project,
    history: &mut History,
) {
    if editor.selection.is_empty() {
        ui.label("Select Tool");
        ui.add_space(4.0);
        ui.label("Click to select elements");
        return;
    }

    // Find the first selected element to show properties
    // (for multi-select, show shared properties or first element's values)
    let selected = editor.selection.selected_ids.clone();
    let count = selected.len();

    ui.label(if count == 1 { "Element" } else { "Selection" });
    ui.add_space(4.0);

    // Gather current values from first selected element
    let first_elem = sprite.layers.iter().flat_map(|l| &l.elements)
        .find(|e| selected.iter().any(|id| id == &e.id));

    let (mut pos_x, mut pos_y, mut rot_deg, mut scale_x, mut scale_y, mut stroke_w, mut color_idx, mut is_curve) =
        match first_elem {
            Some(e) => (
                e.position.x, e.position.y,
                e.rotation.to_degrees(), e.scale.x, e.scale.y,
                e.stroke_width, e.stroke_color_index, e.curve_mode,
            ),
            None => return,
        };

    // Track which property changed (for separate undo descriptions)
    let mut change_desc: Option<&str> = None;

    theme::with_input_style(ui, project.editor_preferences.theme, |ui| {
        // Position
        ui.horizontal(|ui| {
            ui.add(icons::small_icon(icons::prop_position(), ui));
            ui.label("X");
            if ui.add(egui::DragValue::new(&mut pos_x).speed(0.5).fixed_decimals(1)).changed() {
                change_desc = Some("Edit position");
            }
            ui.label("Y");
            if ui.add(egui::DragValue::new(&mut pos_y).speed(0.5).fixed_decimals(1)).changed() {
                change_desc = Some("Edit position");
            }
        });

        // Rotation
        ui.horizontal(|ui| {
            ui.add(icons::small_icon(icons::prop_rotation(), ui));
            if ui.add(egui::DragValue::new(&mut rot_deg).speed(1.0).suffix("°").fixed_decimals(1)).changed() {
                change_desc = Some("Edit rotation");
            }
        });

        // Scale
        ui.horizontal(|ui| {
            ui.add(icons::small_icon(icons::prop_scale(), ui));
            ui.label("X");
            if ui.add(egui::DragValue::new(&mut scale_x).speed(0.01).fixed_decimals(2)).changed() {
                change_desc = Some("Edit scale");
            }
            ui.label("Y");
            if ui.add(egui::DragValue::new(&mut scale_y).speed(0.01).fixed_decimals(2)).changed() {
                change_desc = Some("Edit scale");
            }
        });
    });

    // Stroke width: 3 toggle buttons
    ui.horizontal(|ui| {
        ui.add(icons::small_icon(icons::prop_width(), ui));
        ui.label("Width");
        for &w in &[1.0_f32, 2.0, 4.0] {
            let selected = (stroke_w - w).abs() < 0.01;
            if ui.selectable_label(selected, format!("{}", w as u32)).clicked() {
                stroke_w = w;
                change_desc = Some("Edit stroke width");
            }
        }
    });

    ui.add_space(4.0);

    // Color picker (same mini palette as line tool)
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
            if color_idx == i as u8 {
                let sel_color = theme::selected_color(project.editor_preferences.theme);
                ui.painter().rect_stroke(rect, 1.0, egui::Stroke::new(2.0, sel_color), egui::StrokeKind::Outside);
            }
            if response.clicked() {
                color_idx = i as u8;
                change_desc = Some("Edit color");
            }
        }
    });

    ui.add_space(4.0);

    // Curve/straight toggle — sets all selected elements to the same mode
    ui.horizontal(|ui| {
        if ui.add(icons::icon_button(icons::mode_curve(), ui).selected(is_curve)).on_hover_text("Curve Mode (C)").clicked()
            && !is_curve
        {
            is_curve = true;
            change_desc = Some("Edit curve mode");
        }
        if ui.add(icons::icon_button(icons::mode_straight(), ui).selected(!is_curve)).on_hover_text("Straight Mode").clicked()
            && is_curve
        {
            is_curve = false;
            change_desc = Some("Edit curve mode");
        }
    });

    // Apply changes — coalesced undo per property type
    if let Some(desc) = change_desc {
        let before = sprite.clone();
        let new_rot = rot_deg.to_radians();
        for layer in sprite.layers.iter_mut() {
            for element in layer.elements.iter_mut() {
                if selected.iter().any(|id| id == &element.id) {
                    element.position.x = pos_x;
                    element.position.y = pos_y;
                    element.rotation = new_rot;
                    element.scale.x = scale_x;
                    element.scale.y = scale_y;
                    element.stroke_width = stroke_w;
                    element.stroke_color_index = color_idx;
                    if element.curve_mode != is_curve {
                        element.curve_mode = is_curve;
                        crate::math::recompute_auto_curves(
                            &mut element.vertices,
                            element.closed,
                            element.curve_mode,
                            project.min_corner_radius,
                        );
                    }
                }
            }
        }
        history.push_coalesced(desc.into(), before, sprite.clone());
    }
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
