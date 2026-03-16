use egui::{Color32, Image, ImageSource};

const ICON_SIZE: f32 = 32.0;
const SMALL_ICON_SIZE: f32 = 24.0;
const SIDEBAR_TOGGLE_SIZE: f32 = 16.0;

// Action icons
pub fn action_new() -> ImageSource<'static> { egui::include_image!("../../assets/icons/action_new.svg") }
pub fn action_load() -> ImageSource<'static> { egui::include_image!("../../assets/icons/action_load.svg") }
pub fn action_save() -> ImageSource<'static> { egui::include_image!("../../assets/icons/action_save.svg") }
pub fn action_save_as() -> ImageSource<'static> { egui::include_image!("../../assets/icons/action_save_as.svg") }
pub fn undo() -> ImageSource<'static> { egui::include_image!("../../assets/icons/action_undo.svg") }
pub fn redo() -> ImageSource<'static> { egui::include_image!("../../assets/icons/action_redo.svg") }

// Tool icons
pub fn tool_select() -> ImageSource<'static> { egui::include_image!("../../assets/icons/tool_select.svg") }
pub fn tool_line() -> ImageSource<'static> { egui::include_image!("../../assets/icons/tool_line.svg") }

// Grid icons
pub fn grid_dots() -> ImageSource<'static> { egui::include_image!("../../assets/icons/grid_dots.svg") }
pub fn grid_lines() -> ImageSource<'static> { egui::include_image!("../../assets/icons/grid_lines.svg") }
pub fn grid_iso() -> ImageSource<'static> { egui::include_image!("../../assets/icons/grid_iso.svg") }

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

// Sidebar icons
pub fn sidebar_expand() -> ImageSource<'static> { egui::include_image!("../../assets/icons/sidebar_expand.svg") }
pub fn sidebar_collapse() -> ImageSource<'static> { egui::include_image!("../../assets/icons/sidebar_collapse.svg") }

// Property icons
pub fn prop_width() -> ImageSource<'static> { egui::include_image!("../../assets/icons/prop_width.svg") }
pub fn prop_radius() -> ImageSource<'static> { egui::include_image!("../../assets/icons/prop_radius.svg") }
pub fn prop_position() -> ImageSource<'static> { egui::include_image!("../../assets/icons/prop_position.svg") }
pub fn prop_rotation() -> ImageSource<'static> { egui::include_image!("../../assets/icons/prop_rotation.svg") }
pub fn prop_scale() -> ImageSource<'static> { egui::include_image!("../../assets/icons/prop_scale.svg") }

// Metric icons
pub fn metric_element() -> ImageSource<'static> { egui::include_image!("../../assets/icons/metric_element.svg") }
pub fn metric_vertex() -> ImageSource<'static> { egui::include_image!("../../assets/icons/metric_vertex.svg") }
pub fn metric_layer() -> ImageSource<'static> { egui::include_image!("../../assets/icons/metric_layer.svg") }
pub fn metric_animation() -> ImageSource<'static> { egui::include_image!("../../assets/icons/metric_animation.svg") }

// Helpers

/// Get the icon tint color from the current UI visuals.
pub fn tint(ui: &egui::Ui) -> Color32 {
    ui.visuals().widgets.inactive.fg_stroke.color
}

/// Create a toolbar-sized icon button (16x16), tinted to match theme.
pub fn icon_button(source: ImageSource<'static>, ui: &egui::Ui) -> egui::Button<'static> {
    let tint = tint(ui);
    egui::Button::image(
        Image::new(source).fit_to_exact_size(egui::Vec2::splat(ICON_SIZE)).tint(tint)
    )
}

/// Create a sidebar toggle button (16x16), kept smaller than other icons.
pub fn sidebar_toggle_button(source: ImageSource<'static>, ui: &egui::Ui) -> egui::Button<'static> {
    let tint = tint(ui);
    egui::Button::image(
        Image::new(source).fit_to_exact_size(egui::Vec2::splat(SIDEBAR_TOGGLE_SIZE)).tint(tint)
    )
}

/// Create a small icon button (16x16) for layer controls, tinted to match theme.
pub fn small_icon_button(source: ImageSource<'static>, ui: &egui::Ui) -> egui::Button<'static> {
    let tint = tint(ui);
    egui::Button::image(
        Image::new(source).fit_to_exact_size(egui::Vec2::splat(SIDEBAR_TOGGLE_SIZE)).tint(tint)
    )
}

/// Create a small icon image (24x24) for status bar, tinted to match theme.
pub fn small_icon(source: ImageSource<'static>, ui: &egui::Ui) -> Image<'static> {
    let tint = tint(ui);
    Image::new(source).fit_to_exact_size(egui::Vec2::splat(SMALL_ICON_SIZE)).tint(tint)
}
