use crate::model::project::Theme;
use crate::theme;

/// Render a color palette mini-picker grid. Returns the newly clicked color index, if any.
pub(super) fn render_color_palette(
    ui: &mut egui::Ui,
    colors: &[crate::model::project::PaletteColor],
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

/// Render a single color swatch with selection border.
pub(super) fn render_color_swatch(
    ui: &mut egui::Ui,
    color: crate::model::project::PaletteColor,
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
