use crate::model::project::{Project, Theme};
use crate::model::sprite::Sprite;
use crate::state::editor::{EditorState, ToolKind};
use crate::state::history::History;
use crate::theme;

use super::icons;
use super::sidebar_layers;
use super::sidebar_palette::render_color_swatch;
use super::sidebar_tools;

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
            sidebar_tools::show_line_tool_options(ui, editor, sprite, project);
        }
        ToolKind::Select => {
            sidebar_tools::show_select_tool_options(ui, editor, sprite, project, history);
        }
    }

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(10.0);

    // Layer list
    ui.label("Layers");
    ui.add_space(4.0);
    sidebar_layers::show_layer_list(ui, sprite, &mut editor.active_layer_idx, project.editor_preferences.theme);
}
