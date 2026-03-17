use crate::action::AppAction;
use crate::io;
use crate::model::project::{auto_pick_theme_colors, Project, Theme, ThemeColorIndices};
use crate::model::sprite::Sprite;
use crate::state::editor::{EditorState, ToolKind};
use crate::state::history::History;
use crate::theme;

use super::icons;
use super::sidebar_layers;
use super::sidebar_palette::{render_color_swatch, render_palette_panel};
use super::sidebar_hatch;
use super::sidebar_tools;

pub fn show_sidebar(
    ui: &mut egui::Ui,
    editor: &mut EditorState,
    sprite: &mut Sprite,
    project: &mut Project,
    history: &mut History,
) -> Vec<AppAction> {
    let mut actions = Vec::new();
    if editor.sidebar_expanded {
        show_expanded(ui, editor, sprite, project, history, &mut actions);
    } else {
        show_collapsed(ui, editor, sprite, project);
    }
    actions
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
            ui.label(format!("{}", editor.brush.stroke_width as u32));
        });

        ui.add_space(4.0);

        // Corner radius: icon + read-only value
        ui.horizontal(|ui| {
            ui.add(icons::small_icon(icons::prop_radius(), ui));
            ui.label(format!("{}", project.min_corner_radius as u32));
        });

        ui.add_space(4.0);

        // Active color swatch only (no palette)
        let color = project.palette.get_color(editor.brush.color_index);
        render_color_swatch(ui, color, 20.0, project.editor_preferences.theme);

        ui.add_space(4.0);
    } else if matches!(editor.tool, ToolKind::Fill) {
        // Fill color swatch
        let color = project.palette.get_color(editor.brush.fill_color_index);
        render_color_swatch(ui, color, 20.0, project.editor_preferences.theme);
        ui.add_space(4.0);
    } else if matches!(editor.tool, ToolKind::Eyedropper) {
        // Stroke color swatch
        let color = project.palette.get_color(editor.brush.color_index);
        render_color_swatch(ui, color, 20.0, project.editor_preferences.theme);
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
            render_color_swatch(ui, color, 20.0, project.editor_preferences.theme);

            ui.add_space(4.0);
        }
    }

    // Layer list (simplified, no visibility/lock toggles)
    let sel_color = theme::selected_color(project.editor_preferences.theme);
    let active_idx = editor.layer.resolve_active_idx(sprite);
    let solo = editor.layer.solo_layer_id.clone();
    let layer_count = sprite.layers.len();
    for display_idx in 0..layer_count {
        let layer_idx = layer_count - 1 - display_idx;
        let is_active = layer_idx == active_idx;
        let is_dimmed = solo.as_deref().is_some_and(|sid| sid != sprite.layers[layer_idx].id);

        let name_text = if is_dimmed {
            egui::RichText::new(&sprite.layers[layer_idx].name)
                .color(ui.visuals().weak_text_color())
        } else {
            egui::RichText::new(&sprite.layers[layer_idx].name)
        };

        let label = egui::SelectableLabel::new(is_active, name_text);
        let resp = ui.add(label);
        if is_active {
            let rect = resp.rect;
            ui.painter().line_segment(
                [rect.left_bottom(), rect.right_bottom()],
                egui::Stroke::new(2.0, sel_color),
            );
        }
        if resp.clicked() {
            editor.layer.set_active_by_idx(layer_idx, sprite);
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
    actions: &mut Vec<AppAction>,
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
            io::save_app_defaults(project);
        }
        if ui
            .add(icons::sidebar_toggle_button(icons::theme_light(), ui).selected(!is_dark))
            .on_hover_text("Light Theme")
            .clicked()
        {
            project.editor_preferences.theme = Theme::Light;
            io::save_app_defaults(project);
        }

        // Settings toggle (same row as theme buttons)
        if ui
            .add(icons::sidebar_toggle_button(icons::settings(), ui).selected(editor.theme_settings_open))
            .on_hover_text("Theme Settings")
            .clicked()
        {
            editor.theme_settings_open = !editor.theme_settings_open;
            if !editor.theme_settings_open {
                editor.theme_role_picker = None;
            }
        }
    });

    if editor.theme_settings_open {
        show_theme_color_settings(ui, editor, project);
    }

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(10.0);

    // Tool-specific options
    match editor.tool {
        ToolKind::Line => {
            sidebar_tools::show_line_tool_options(ui, editor, sprite, project);
        }
        ToolKind::Select => {
            sidebar_tools::show_select_tool_options(ui, editor, sprite, project, history);
        }
        ToolKind::Fill => {
            sidebar_tools::show_fill_tool_options(ui, editor, project, actions);
        }
        ToolKind::Eyedropper => {
            sidebar_tools::show_eyedropper_tool_options(ui, editor, project);
        }
        ToolKind::Eraser => {
            // Minimal — eraser has no configurable options
        }
    }

    // Hatch pattern editor (when open and fill tool is active)
    if editor.hatch_editor_open && editor.tool == ToolKind::Fill {
        sidebar_hatch::show_hatch_editor(ui, editor, project, sprite, actions);
    }

    if matches!(editor.tool, ToolKind::Eyedropper) {
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        // Palette management
        render_palette_panel(
            ui,
            editor,
            &project.palette,
            project.editor_preferences.theme,
            actions,
        );
    }

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(10.0);

    // Layer list
    ui.label("Layers");
    ui.add_space(4.0);
    sidebar_layers::show_layer_list(ui, sprite, editor, project, history);

    // Reference images panel
    super::sidebar_reference::show_reference_images(
        ui, editor, sprite,
        project.editor_preferences.theme,
        actions,
    );
}

/// Show theme color role settings: 5 swatches per theme mode, clickable to reassign.
fn show_theme_color_settings(
    ui: &mut egui::Ui,
    editor: &mut EditorState,
    project: &mut Project,
) {
    let theme = project.editor_preferences.theme;
    let indices = match theme {
        Theme::Dark => &mut project.editor_preferences.dark_theme_colors,
        Theme::Light => &mut project.editor_preferences.light_theme_colors,
    };

    ui.add_space(4.0);

    // Show 5 role swatches in a row
    ui.horizontal_wrapped(|ui| {
        for role in 0..5 {
            let palette_idx = indices.get(role);
            let color = project.palette.get_color(palette_idx);
            let size = egui::Vec2::splat(20.0);
            let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
            let c32 = color.to_color32();
            ui.painter().rect_filled(rect, 2.0, c32);

            // Highlight if this role's picker is open
            if editor.theme_role_picker == Some(role) {
                ui.painter().rect_stroke(
                    rect,
                    2.0,
                    egui::Stroke::new(2.0, egui::Color32::WHITE),
                    egui::StrokeKind::Outside,
                );
            }

            if response.clicked() {
                editor.theme_role_picker = if editor.theme_role_picker == Some(role) {
                    None
                } else {
                    Some(role)
                };
            }
            response.on_hover_text(ThemeColorIndices::ROLE_NAMES[role]);
        }
    });

    // If a role picker is open, show palette grid for selection
    if let Some(role) = editor.theme_role_picker {
        ui.add_space(4.0);
        ui.label(format!("Pick: {}", ThemeColorIndices::ROLE_NAMES[role]));
        let current_idx = indices.get(role);
        ui.horizontal_wrapped(|ui| {
            for (i, pc) in project.palette.colors.iter().enumerate() {
                if i == 0 { continue; } // skip transparent
                let size = egui::Vec2::splat(14.0);
                let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
                let c32 = pc.to_color32();
                ui.painter().rect_filled(rect, 1.0, c32);
                if current_idx == i as u8 {
                    ui.painter().rect_stroke(
                        rect,
                        1.0,
                        egui::Stroke::new(2.0, egui::Color32::WHITE),
                        egui::StrokeKind::Outside,
                    );
                }
                if response.clicked() {
                    // Need to re-borrow indices mutably
                    match theme {
                        Theme::Dark => project.editor_preferences.dark_theme_colors.set(role, i as u8),
                        Theme::Light => project.editor_preferences.light_theme_colors.set(role, i as u8),
                    }
                    editor.theme_role_picker = None;
                    io::save_app_defaults(project);
                }
                if response.hovered() {
                    response.on_hover_text(format!("Color {i}"));
                }
            }
        });
    }

    ui.add_space(4.0);

    // Auto-pick button
    if ui.button("Auto").on_hover_text("Auto-pick theme colors from palette").clicked() {
        let (dark, light) = auto_pick_theme_colors(&project.palette);
        project.editor_preferences.dark_theme_colors = dark;
        project.editor_preferences.light_theme_colors = light;
        editor.theme_role_picker = None;
        io::save_app_defaults(project);
    }
}
