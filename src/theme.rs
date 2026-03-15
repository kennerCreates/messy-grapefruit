use egui::Color32;

use crate::model::project::Theme;

/// Semantic color roles derived from the 5-color palette.
///
/// Dark mode ordered darkest→lightest: bg, panels, accent, secondary, text
/// Light mode ordered lightest→darkest for same roles.
///
/// Role mapping:
///   panel_bg  — panels, toolbar/sidebar boxes, grid lines (darkest / lightest)
///   canvas_bg — canvas background (2nd darkest / 2nd lightest)
///   mid       — grid dots, icon hover background (middle)
///   selected  — selected/active icon highlight (2nd lightest / 2nd darkest)
///   icon_text — icons, text, foreground (lightest / darkest)
pub struct ThemeColors {
    pub panel_bg: Color32,
    pub canvas_bg: Color32,
    pub mid: Color32,
    pub selected: Color32,
    pub icon_text: Color32,
}

pub fn theme_colors(theme: Theme) -> ThemeColors {
    match theme {
        Theme::Dark => ThemeColors {
            panel_bg:  Color32::from_rgb(0x29, 0x28, 0x31), // #292831
            canvas_bg: Color32::from_rgb(0x33, 0x3f, 0x58), // #333f58
            mid:       Color32::from_rgb(0x4a, 0x7a, 0x96), // #4a7a96
            selected:  Color32::from_rgb(0xee, 0x86, 0x95), // #ee8695
            icon_text: Color32::from_rgb(0xfb, 0xbb, 0xad), // #fbbbad
        },
        Theme::Light => ThemeColors {
            panel_bg:  Color32::from_rgb(0xff, 0xec, 0xd6), // #ffecd6
            canvas_bg: Color32::from_rgb(0xff, 0xb8, 0x73), // #ffb873
            mid:       Color32::from_rgb(0xcb, 0x76, 0x5c), // #cb765c
            selected:  Color32::from_rgb(0x7a, 0x4a, 0x5a), // #7a4a5a
            icon_text: Color32::from_rgb(0x25, 0x21, 0x3e), // #25213e
        },
    }
}

pub fn apply_theme(ctx: &egui::Context, theme: Theme) {
    let tc = theme_colors(theme);
    let mut visuals = match theme {
        Theme::Dark => egui::Visuals::dark(),
        Theme::Light => egui::Visuals::light(),
    };

    // Panel/window backgrounds = darkest (panel_bg)
    visuals.panel_fill = tc.panel_bg;
    visuals.window_fill = tc.panel_bg;
    visuals.extreme_bg_color = tc.panel_bg;
    visuals.faint_bg_color = tc.panel_bg;

    // Window border/stroke uses panel_bg for seamless look
    visuals.window_stroke = egui::Stroke::new(1.0, tc.canvas_bg);

    // Text/icons = lightest (icon_text)
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, tc.icon_text);
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, tc.icon_text);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, tc.icon_text);
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, tc.icon_text);

    // Icon background seamless to panel when not toggled/selected
    visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
    visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;

    // Non-interactive widget bg = panel_bg (seamless)
    visuals.widgets.noninteractive.bg_fill = tc.panel_bg;
    visuals.widgets.noninteractive.weak_bg_fill = tc.panel_bg;

    // Hover = selected (2nd lightest / 2nd darkest)
    visuals.widgets.hovered.bg_fill = tc.selected;
    visuals.widgets.hovered.weak_bg_fill = tc.selected;

    // Active/pressed = middle color
    visuals.widgets.active.bg_fill = tc.mid;
    visuals.widgets.active.weak_bg_fill = tc.mid;

    // Selection/toggle highlight = middle color
    visuals.selection.bg_fill = tc.mid;
    visuals.selection.stroke = egui::Stroke::new(1.0, tc.mid);

    visuals.hyperlink_color = tc.mid;

    // Separator color = subtle
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(0.5, tc.canvas_bg);

    // Slider rail = mid color so it's visible against panel_bg
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, tc.mid);

    ctx.set_visuals(visuals);
}

// --- Canvas colors ---

pub fn canvas_bg_color(theme: Theme) -> Color32 {
    theme_colors(theme).canvas_bg
}

// --- Grid colors ---

/// Grid dots = middle color (solid, no alpha)
pub fn grid_dot_color(theme: Theme) -> Color32 {
    theme_colors(theme).mid
}

/// Grid lines = darkest/panel_bg color (solid, no alpha)
pub fn grid_line_color(theme: Theme) -> Color32 {
    theme_colors(theme).panel_bg
}

// --- Canvas overlay colors ---

pub fn canvas_boundary_color(theme: Theme) -> Color32 {
    let tc = theme_colors(theme);
    Color32::from_rgba_unmultiplied(tc.mid.r(), tc.mid.g(), tc.mid.b(), 150)
}

pub fn hover_highlight_color(theme: Theme) -> Color32 {
    let tc = theme_colors(theme);
    Color32::from_rgba_unmultiplied(tc.mid.r(), tc.mid.g(), tc.mid.b(), 128)
}

pub fn merge_preview_color(theme: Theme) -> Color32 {
    match theme {
        Theme::Dark => Color32::from_rgba_unmultiplied(0x80, 0xff, 0x80, 180),
        Theme::Light => Color32::from_rgba_unmultiplied(0x00, 0x80, 0x00, 180),
    }
}

pub fn rubber_band_color(theme: Theme) -> Color32 {
    let tc = theme_colors(theme);
    Color32::from_rgba_unmultiplied(tc.icon_text.r(), tc.icon_text.g(), tc.icon_text.b(), 100)
}

pub fn floating_panel_color(theme: Theme) -> Color32 {
    let tc = theme_colors(theme);
    Color32::from_rgba_unmultiplied(tc.panel_bg.r(), tc.panel_bg.g(), tc.panel_bg.b(), 240)
}

/// The selected/active highlight color (for toggled buttons, active tools).
pub fn selected_color(theme: Theme) -> Color32 {
    theme_colors(theme).selected
}
