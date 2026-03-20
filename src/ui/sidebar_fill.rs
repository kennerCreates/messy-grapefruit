use crate::action::AppAction;
use crate::model::project::{HatchPattern, PatternType, Project, Theme};
use crate::model::sprite::{GradientFill, Sprite};
use crate::state::editor::{EditorState, FillMode};
use crate::state::history::History;
use crate::theme;

use super::icons;
use super::sidebar_gradient;
use super::sidebar_palette::{render_color_palette, render_color_swatch};

pub(super) fn show_fill_tool_options(
    ui: &mut egui::Ui,
    editor: &mut EditorState,
    project: &mut Project,
    actions: &mut Vec<AppAction>,
) {
    // ── Fill section (collapsible) ──
    let prev_indent = ui.spacing().indent;
    ui.spacing_mut().indent = 0.0;
    egui::CollapsingHeader::new("Fill")
        .default_open(true)
        .show(ui, |ui| {
            // Fill mode toggles (flat / linear gradient / radial gradient)
            ui.horizontal(|ui| {
                let modes = [
                    (FillMode::Flat, icons::fill_flat(), "Flat Fill"),
                    (FillMode::LinearGradient, icons::fill_linear(), "Linear Gradient"),
                    (FillMode::RadialGradient, icons::fill_radial(), "Radial Gradient"),
                ];
                for (mode, icon, tooltip) in modes {
                    let selected = editor.brush.fill_mode == mode;
                    if ui.add(icons::small_icon_button(icon, ui).selected(selected))
                        .on_hover_text(tooltip)
                        .clicked()
                    {
                        editor.brush.fill_mode = mode;
                    }
                }
            });

            ui.add_space(4.0);

            match editor.brush.fill_mode {
                FillMode::Flat => {
                    ui.horizontal(|ui| {
                        ui.label("Fill");
                        let color = project.palette.get_color(editor.brush.fill_color_index);
                        render_color_swatch(ui, color, 20.0, project.editor_preferences.theme);
                        ui.label(format!("idx {}", editor.brush.fill_color_index));
                    });

                    if let Some(new_idx) = render_color_palette(
                        ui,
                        &project.palette.colors,
                        editor.brush.fill_color_index,
                        project.editor_preferences.theme,
                    ) {
                        editor.brush.fill_color_index = new_idx;
                        editor.track_recent_color(new_idx);
                    }
                }
                FillMode::LinearGradient | FillMode::RadialGradient => {
                    sidebar_gradient::render_gradient_controls(ui, editor, project, actions);
                }
            }
        });

    // ── Hatch section (collapsible) ──
    egui::CollapsingHeader::new("Hatch")
        .default_open(false)
        .show(ui, |ui| {
            render_hatch_picker(ui, editor, project, actions);
        });
    ui.spacing_mut().indent = prev_indent;

    ui.add_space(4.0);
    ui.label("Click closed shape to apply");
}

pub(super) fn render_hatch_picker(
    ui: &mut egui::Ui,
    editor: &mut EditorState,
    project: &mut Project,
    actions: &mut Vec<AppAction>,
) {
    // Apply toggle
    ui.checkbox(&mut editor.brush.hatch_apply_enabled, "Apply on click");

    ui.add_space(4.0);

    if project.hatch_patterns.is_empty() {
        ui.label("No patterns yet");
    } else {
        let theme = project.editor_preferences.theme;
        ui.horizontal_wrapped(|ui| {
            for pattern in project.hatch_patterns.iter() {
                let is_selected = editor.selected_hatch_pattern_id.as_deref() == Some(pattern.id.as_str());
                let resp = paint_hatch_thumbnail(ui, pattern, is_selected, false, theme, egui::vec2(36.0, 36.0));
                if resp.on_hover_text(&pattern.name).clicked() {
                    editor.selected_hatch_pattern_id = Some(pattern.id.clone());
                }
            }
        });
    }

    ui.add_space(4.0);

    ui.horizontal(|ui| {
        if ui.button("+ Lines").clicked() {
            let pattern = HatchPattern::new("Lines");
            editor.selected_hatch_pattern_id = Some(pattern.id.clone());
            actions.push(AppAction::AddHatchPattern(pattern));
        }
        if ui.button("+ Cross").clicked() {
            let pattern = HatchPattern::new_cross_hatch("Cross-Hatch");
            editor.selected_hatch_pattern_id = Some(pattern.id.clone());
            actions.push(AppAction::AddHatchPattern(pattern));
        }
        if ui.button("+ Brick").clicked() {
            let pattern = HatchPattern::new_brick("Brick");
            editor.selected_hatch_pattern_id = Some(pattern.id.clone());
            actions.push(AppAction::AddHatchPattern(pattern));
        }
    });
    ui.horizontal(|ui| {
        if ui.button("Edit").clicked() {
            editor.ui.hatch_editor_open = !editor.ui.hatch_editor_open;
        }
    });

    ui.add_space(4.0);

    if ui.button("Remove Hatch from Selected").clicked() {
        for id in &editor.selection.selected_ids {
            actions.push(AppAction::ClearHatchFill { element_id: id.clone() });
        }
    }
}

/// Fill controls within the select tool sidebar.
#[allow(clippy::too_many_arguments)]
pub(super) fn show_select_fill_section(
    ui: &mut egui::Ui,
    editor: &mut EditorState,
    sprite: &mut Sprite,
    project: &mut Project,
    history: &mut History,
    actions: &mut Vec<AppAction>,
    selected: &[String],
) {
    let first_elem = sprite.layers.iter().flat_map(|l| &l.elements)
        .find(|e| selected.iter().any(|id| id == &e.id));
    let (mut fill_idx, has_gradient) = match first_elem {
        Some(e) => (e.fill_color_index, e.gradient_fill.is_some()),
        None => return,
    };

    if has_gradient {
        ui.label("Gradient fill active");
        if ui.button("Clear Gradient").clicked() {
            for id in selected {
                actions.push(AppAction::ClearGradientFill { element_id: id.clone() });
            }
        }
    } else {
        ui.label("Fill");
        ui.add_space(2.0);

        if let Some(new_idx) = render_color_palette(
            ui, &project.palette.colors, fill_idx, project.editor_preferences.theme,
        ) {
            fill_idx = new_idx;
            let before = sprite.clone();
            crate::engine::transform::for_selected_elements_mut(sprite, selected, |element| {
                element.fill_color_index = fill_idx;
            });
            history.push_coalesced("Edit fill color".into(), before, sprite.clone());
            editor.track_recent_color(fill_idx);
        }

        ui.add_space(2.0);

        ui.horizontal(|ui| {
            if ui.button("Linear Grad").clicked() {
                let gf = GradientFill::linear(
                    editor.brush.gradient_stops.clone(),
                    editor.brush.gradient_angle,
                );
                for id in selected {
                    actions.push(AppAction::SetGradientFill {
                        element_id: id.clone(),
                        gradient_fill: gf.clone(),
                    });
                }
            }
            if ui.button("Radial Grad").clicked() {
                let gf = GradientFill::radial(
                    editor.brush.gradient_stops.clone(),
                    editor.brush.radial_center,
                    editor.brush.radial_radius,
                );
                for id in selected {
                    actions.push(AppAction::SetGradientFill {
                        element_id: id.clone(),
                        gradient_fill: gf.clone(),
                    });
                }
            }
        });
    }
}

/// Hatch controls within the select tool sidebar.
#[allow(clippy::too_many_arguments)]
pub(super) fn show_select_hatch_section(
    ui: &mut egui::Ui,
    editor: &mut EditorState,
    sprite: &Sprite,
    project: &mut Project,
    _history: &mut History,
    actions: &mut Vec<AppAction>,
    selected: &[String],
) {
    ui.add_space(4.0);

    let first_elem = sprite.layers.iter().flat_map(|l| &l.elements)
        .find(|e| selected.iter().any(|id| id == &e.id));
    let current_hatch_id = first_elem.and_then(|e| e.hatch_fill_id.as_deref());

    ui.label("Hatch");
    ui.add_space(2.0);

    if let Some(hatch_id) = current_hatch_id {
        let pattern_name = project.hatch_patterns.iter()
            .find(|p| p.id == hatch_id)
            .map(|p| p.name.as_str())
            .unwrap_or("(unknown)");
        ui.label(format!("Pattern: {pattern_name}"));

        if ui.button("Remove Hatch").clicked() {
            for id in selected {
                actions.push(AppAction::ClearHatchFill { element_id: id.clone() });
            }
        }
    }

    ui.add_space(2.0);

    if project.hatch_patterns.is_empty() {
        ui.label("No patterns");
        ui.horizontal(|ui| {
            if ui.small_button("+ Lines").clicked() {
                let pattern = HatchPattern::new("Lines");
                editor.selected_hatch_pattern_id = Some(pattern.id.clone());
                actions.push(AppAction::AddHatchPattern(pattern));
            }
            if ui.small_button("+ Cross").clicked() {
                let pattern = HatchPattern::new_cross_hatch("Cross-Hatch");
                editor.selected_hatch_pattern_id = Some(pattern.id.clone());
                actions.push(AppAction::AddHatchPattern(pattern));
            }
            if ui.small_button("+ Brick").clicked() {
                let pattern = HatchPattern::new_brick("Brick");
                editor.selected_hatch_pattern_id = Some(pattern.id.clone());
                actions.push(AppAction::AddHatchPattern(pattern));
            }
        });
    } else {
        let theme = project.editor_preferences.theme;
        ui.horizontal_wrapped(|ui| {
            for pattern in &project.hatch_patterns {
                let is_applied = current_hatch_id == Some(pattern.id.as_str());
                let is_selected = editor.selected_hatch_pattern_id.as_deref() == Some(pattern.id.as_str());
                let resp = paint_hatch_thumbnail(ui, pattern, is_selected, is_applied, theme, egui::vec2(36.0, 36.0));
                if resp.on_hover_text(&pattern.name).clicked() {
                    editor.selected_hatch_pattern_id = Some(pattern.id.clone());
                    for id in selected {
                        actions.push(AppAction::SetHatchFill {
                            element_id: id.clone(),
                            hatch_fill_id: pattern.id.clone(),
                        });
                    }
                }
            }
        });
    }
}

// ── Hatch thumbnail rendering ──

/// Draw a small hatch pattern thumbnail preview.
pub(super) fn paint_hatch_thumbnail(
    ui: &mut egui::Ui,
    pattern: &HatchPattern,
    is_selected: bool,
    is_applied: bool,
    theme: Theme,
    size: egui::Vec2,
) -> egui::Response {
    let (response, painter) = ui.allocate_painter(size, egui::Sense::click());
    let rect = response.rect;
    let tc = crate::theme::theme_colors(theme);

    painter.rect_filled(rect, 2.0, tc.panel_bg);

    let clip = painter.with_clip_rect(painter.clip_rect().intersect(rect));
    let stroke = egui::Stroke::new(1.0, tc.icon_text);

    match pattern.pattern_type {
        PatternType::Lines => {
            let angle = pattern.layers.first().map(|l| l.angle).unwrap_or(45.0);
            draw_thumb_lines(&clip, rect, angle.to_radians(), 6.0, stroke);
        }
        PatternType::CrossHatch => {
            let angle = pattern.layers.first().map(|l| l.angle).unwrap_or(45.0);
            let angle_rad = angle.to_radians();
            draw_thumb_lines(&clip, rect, angle_rad, 6.0, stroke);
            let perp = if pattern.iso_mode {
                std::f32::consts::FRAC_PI_2
            } else {
                angle_rad + std::f32::consts::FRAC_PI_2
            };
            draw_thumb_lines(&clip, rect, perp, 6.0, stroke);
        }
        PatternType::Brick => {
            draw_thumb_bricks(&clip, rect, 8.0, 14.0, stroke);
        }
    }

    let (border_w, border_color) = if is_selected {
        (2.0, tc.selected)
    } else {
        (1.0, tc.mid)
    };
    painter.rect_stroke(rect, 2.0, egui::Stroke::new(border_w, border_color), egui::StrokeKind::Outside);

    if is_applied {
        let dot_center = rect.right_top() + egui::vec2(-5.0, 5.0);
        painter.circle_filled(dot_center, 3.0, tc.selected);
    }

    response
}

fn draw_thumb_lines(
    painter: &egui::Painter,
    rect: egui::Rect,
    angle_rad: f32,
    spacing: f32,
    stroke: egui::Stroke,
) {
    let center = rect.center();
    let cos_a = angle_rad.cos();
    let sin_a = angle_rad.sin();
    let dir = egui::vec2(cos_a, sin_a);
    let perp = egui::vec2(-sin_a, cos_a);

    let center_v = center.to_vec2();
    let corners = [
        rect.left_top().to_vec2(),
        rect.right_top().to_vec2(),
        rect.left_bottom().to_vec2(),
        rect.right_bottom().to_vec2(),
    ];

    let mut min_perp = f32::INFINITY;
    let mut max_perp = f32::NEG_INFINITY;
    let mut max_extent: f32 = 0.0;
    for c in &corners {
        let d = *c - center_v;
        let pd = d.x * perp.x + d.y * perp.y;
        let dd = (d.x * dir.x + d.y * dir.y).abs();
        min_perp = min_perp.min(pd);
        max_perp = max_perp.max(pd);
        max_extent = max_extent.max(dd);
    }

    let mut d = (min_perp / spacing).floor() * spacing;
    while d <= max_perp {
        let lc = center + perp * d;
        let p1 = lc - dir * max_extent;
        let p2 = lc + dir * max_extent;
        painter.line_segment([p1, p2], stroke);
        d += spacing;
    }
}

fn draw_thumb_bricks(
    painter: &egui::Painter,
    rect: egui::Rect,
    row_h: f32,
    brick_w: f32,
    stroke: egui::Stroke,
) {
    let left = rect.left();
    let right = rect.right();
    let top = rect.top();
    let bottom = rect.bottom();

    let mut y = top;
    let mut row = 0;
    while y <= bottom {
        painter.line_segment([egui::pos2(left, y), egui::pos2(right, y)], stroke);
        let next_y = (y + row_h).min(bottom);
        let offset = if row % 2 == 1 { brick_w / 2.0 } else { 0.0 };
        let mut x = left + offset;
        while x <= right {
            painter.line_segment([egui::pos2(x, y), egui::pos2(x, next_y)], stroke);
            x += brick_w;
        }
        y += row_h;
        row += 1;
    }
}

/// theme helper re-used from parent
#[allow(dead_code)]
fn _unused_theme_import() {
    // Silence unused import — theme is used above via crate::theme::theme_colors
    let _ = theme::theme_colors;
}
