use crate::action::AppAction;
use crate::model::project::{Palette, PaletteColor, Theme};
use crate::state::editor::EditorState;
use crate::theme;

use super::icons;

/// Render a color palette mini-picker grid. Returns the newly clicked color index, if any.
pub(super) fn render_color_palette(
    ui: &mut egui::Ui,
    colors: &[PaletteColor],
    selected_index: u8,
    theme: Theme,
) -> Option<u8> {
    let mut clicked = None;
    ui.horizontal_wrapped(|ui| {
        for (i, pc) in colors.iter().enumerate() {
            let size = egui::Vec2::splat(16.0);
            let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
            let c32 = pc.to_color32();
            if c32.a() == 0 {
                draw_checkerboard(ui, rect);
            } else {
                ui.painter().rect_filled(rect, 1.0, c32);
            }
            if selected_index == i as u8 {
                let sel_color = theme::selected_color(theme);
                ui.painter().rect_stroke(
                    rect,
                    1.0,
                    egui::Stroke::new(2.0, sel_color),
                    egui::StrokeKind::Outside,
                );
            }
            if response.clicked() {
                clicked = Some(i as u8);
            }
            if response.hovered() {
                response.on_hover_text(format!("Color {i}"));
            }
        }
    });
    clicked
}

/// Render the full palette management panel (expanded sidebar).
/// Includes: recent colors, palette grid, add/delete/edit controls.
pub(super) fn render_palette_panel(
    ui: &mut egui::Ui,
    editor: &mut EditorState,
    palette: &Palette,
    theme: Theme,
    actions: &mut Vec<AppAction>,
) {
    ui.label("Palette");
    ui.add_space(4.0);

    // Recent colors bar
    if !editor.recent_colors.is_empty() {
        ui.horizontal_wrapped(|ui| {
            ui.label("Recent");
            for &idx in &editor.recent_colors.clone() {
                let color = palette.get_color(idx);
                let size = egui::Vec2::splat(14.0);
                let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
                let c32 = color.to_color32();
                if c32.a() == 0 {
                    draw_checkerboard(ui, rect);
                } else {
                    ui.painter().rect_filled(rect, 1.0, c32);
                }
                if response.clicked() {
                    editor.brush.color_index = idx;
                }
                if response.hovered() {
                    response.on_hover_text(format!("Color {idx}"));
                }
            }
        });
        ui.add_space(4.0);
    }

    // Palette grid (click to select stroke color)
    if let Some(new_idx) = render_color_palette(
        ui,
        &palette.colors,
        editor.brush.color_index,
        theme,
    ) {
        editor.brush.color_index = new_idx;
        editor.track_recent_color(new_idx);
    }

    ui.add_space(4.0);

    // Palette management buttons
    ui.horizontal(|ui| {
        // Add color
        let at_limit = palette.colors.len() >= 256;
        if ui
            .add_enabled(!at_limit, icons::small_icon_button(icons::palette_add(), ui))
            .on_hover_text(if at_limit { "Palette Full (256 max)" } else { "Add Color" })
            .clicked()
        {
            actions.push(AppAction::AddPaletteColor(PaletteColor::new(255, 255, 255)));
        }

        // Delete selected color
        if ui
            .add(icons::small_icon_button(icons::palette_remove(), ui))
            .on_hover_text("Delete Selected Color")
            .clicked()
            && editor.brush.color_index != 0
            && (editor.brush.color_index as usize) < palette.colors.len()
        {
            actions.push(AppAction::DeletePaletteColor(editor.brush.color_index));
            // Move selection to previous color
            if editor.brush.color_index > 0 {
                editor.brush.color_index -= 1;
            }
        }

        // Lospec import
        if ui
            .add(icons::small_icon_button(icons::palette_import(), ui))
            .on_hover_text("Import from Lospec")
            .clicked()
        {
            editor.ui.lospec_popup_open = !editor.ui.lospec_popup_open;
            editor.ui.lospec_error = None;
        }
    });

    // Lospec import popup
    if editor.ui.lospec_popup_open {
        ui.group(|ui| {
            ui.label("Lospec Import");
            ui.horizontal(|ui| {
                ui.label("Slug:");
                ui.text_edit_singleline(&mut editor.ui.lospec_slug);
            });
            if let Some(err) = &editor.ui.lospec_error {
                ui.colored_label(egui::Color32::from_rgb(255, 100, 100), err);
            }
            ui.horizontal(|ui| {
                if ui.button("Import").clicked() && !editor.ui.lospec_slug.is_empty() {
                    match crate::io::fetch_lospec_palette(&editor.ui.lospec_slug) {
                        Ok(colors) => {
                            actions.push(AppAction::ImportPalette(colors));
                            editor.ui.lospec_popup_open = false;
                            editor.ui.lospec_error = None;
                            editor.brush.color_index = 1.min((palette.colors.len().saturating_sub(1)) as u8);
                        }
                        Err(e) => {
                            editor.ui.lospec_error = Some(e.to_string());
                        }
                    }
                }
                if ui.button("Cancel").clicked() {
                    editor.ui.lospec_popup_open = false;
                }
            });
        });
    }

    ui.add_space(4.0);

    // Color editor: show RGB sliders for the selected color
    if editor.brush.color_index != 0
        && let Some(pc) = palette.colors.get(editor.brush.color_index as usize)
    {
        let mut r = pc.r;
        let mut g = pc.g;
        let mut b = pc.b;
        let mut changed = false;

        theme::with_input_style(ui, theme, |ui| {
            ui.horizontal(|ui| {
                ui.label("R");
                if ui.add(egui::DragValue::new(&mut r).range(0..=255).speed(1.0)).changed() {
                    changed = true;
                }
                ui.label("G");
                if ui.add(egui::DragValue::new(&mut g).range(0..=255).speed(1.0)).changed() {
                    changed = true;
                }
                ui.label("B");
                if ui.add(egui::DragValue::new(&mut b).range(0..=255).speed(1.0)).changed() {
                    changed = true;
                }
            });
        });

        if changed {
            actions.push(AppAction::EditPaletteColor {
                index: editor.brush.color_index,
                color: PaletteColor::new(r, g, b),
            });
        }
    }

}

/// Render a single color swatch with selection border.
pub(super) fn render_color_swatch(
    ui: &mut egui::Ui,
    color: PaletteColor,
    size: f32,
    theme: Theme,
) {
    let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(size), egui::Sense::hover());
    let c32 = color.to_color32();
    if c32.a() == 0 {
        draw_checkerboard(ui, rect);
    } else {
        ui.painter().rect_filled(rect, 2.0, c32);
    }
    let sel_color = theme::selected_color(theme);
    ui.painter().rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(1.0, sel_color),
        egui::StrokeKind::Outside,
    );
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
