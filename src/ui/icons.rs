use egui::{Image, ImageSource};

const ICON_SIZE: f32 = 16.0;
const SMALL_ICON_SIZE: f32 = 12.0;

// Action icons
pub fn undo() -> ImageSource<'static> { egui::include_image!("../../assets/icons/action_undo.svg") }
pub fn redo() -> ImageSource<'static> { egui::include_image!("../../assets/icons/action_redo.svg") }

// Tool icons
pub fn tool_line() -> ImageSource<'static> { egui::include_image!("../../assets/icons/tool_line.svg") }

// Grid icons
pub fn grid_dots() -> ImageSource<'static> { egui::include_image!("../../assets/icons/grid_dots.svg") }
pub fn grid_lines() -> ImageSource<'static> { egui::include_image!("../../assets/icons/grid_lines.svg") }
pub fn grid_iso() -> ImageSource<'static> { egui::include_image!("../../assets/icons/grid_iso.svg") }

// Stroke icons
pub fn stroke_taper() -> ImageSource<'static> { egui::include_image!("../../assets/icons/stroke_taper.svg") }

// Theme icons
pub fn theme_dark() -> ImageSource<'static> { egui::include_image!("../../assets/icons/theme_dark.svg") }
pub fn theme_light() -> ImageSource<'static> { egui::include_image!("../../assets/icons/theme_light.svg") }

// View icons
pub fn view_flip() -> ImageSource<'static> { egui::include_image!("../../assets/icons/view_flip.svg") }
pub fn view_zoom_fit() -> ImageSource<'static> { egui::include_image!("../../assets/icons/view_zoom_fit.svg") }

// Layer icons
pub fn layer_visible() -> ImageSource<'static> { egui::include_image!("../../assets/icons/layer_visible.svg") }
pub fn layer_hidden() -> ImageSource<'static> { egui::include_image!("../../assets/icons/layer_hidden.svg") }
pub fn layer_locked() -> ImageSource<'static> { egui::include_image!("../../assets/icons/layer_locked.svg") }
pub fn layer_unlocked() -> ImageSource<'static> { egui::include_image!("../../assets/icons/layer_unlocked.svg") }
pub fn layer_add() -> ImageSource<'static> { egui::include_image!("../../assets/icons/layer_add.svg") }

// Mode icons
pub fn mode_curve() -> ImageSource<'static> { egui::include_image!("../../assets/icons/mode_curve.svg") }
pub fn mode_straight() -> ImageSource<'static> { egui::include_image!("../../assets/icons/mode_straight.svg") }

// Metric icons
pub fn metric_element() -> ImageSource<'static> { egui::include_image!("../../assets/icons/metric_element.svg") }
pub fn metric_vertex() -> ImageSource<'static> { egui::include_image!("../../assets/icons/metric_vertex.svg") }
pub fn metric_layer() -> ImageSource<'static> { egui::include_image!("../../assets/icons/metric_layer.svg") }
pub fn metric_animation() -> ImageSource<'static> { egui::include_image!("../../assets/icons/metric_animation.svg") }

// Helpers

/// Create a toolbar-sized icon button (16x16).
pub fn icon_button(source: ImageSource<'static>) -> egui::Button<'static> {
    egui::Button::image(
        Image::new(source).fit_to_exact_size(egui::Vec2::splat(ICON_SIZE))
    )
}

/// Create a small icon image (12x12) for status bar.
pub fn small_icon(source: ImageSource<'static>) -> Image<'static> {
    Image::new(source).fit_to_exact_size(egui::Vec2::splat(SMALL_ICON_SIZE))
}
