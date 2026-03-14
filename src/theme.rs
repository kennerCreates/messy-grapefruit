use eframe::egui;
use egui::Color32;

use crate::model::Theme;

pub fn apply_theme(ctx: &egui::Context, theme: &Theme) {
    let mut visuals = match theme {
        Theme::Dark => egui::Visuals::dark(),
        Theme::Light => egui::Visuals::light(),
    };

    match theme {
        Theme::Dark => {
            // Twilight 5 palette
            visuals.panel_fill = Color32::from_rgb(0x33, 0x3f, 0x58);
            visuals.window_fill = Color32::from_rgb(0x29, 0x28, 0x31);
            visuals.extreme_bg_color = Color32::from_rgb(0x29, 0x28, 0x31);
            visuals.override_text_color = Some(Color32::from_rgb(0xfb, 0xbb, 0xad));
            visuals.selection.bg_fill = Color32::from_rgb(0x4a, 0x7a, 0x96);
            visuals.widgets.active.bg_fill = Color32::from_rgb(0x4a, 0x7a, 0x96);
            visuals.widgets.hovered.bg_fill = Color32::from_rgb(0x4a, 0x7a, 0x96);
            visuals.widgets.inactive.bg_fill = Color32::from_rgb(0x33, 0x3f, 0x58);
            visuals.hyperlink_color = Color32::from_rgb(0xee, 0x86, 0x95);
        }
        Theme::Light => {
            // Golden Sunset palette
            visuals.panel_fill = Color32::from_rgb(0x7a, 0x4a, 0x5a);
            visuals.window_fill = Color32::from_rgb(0x25, 0x21, 0x3e);
            visuals.extreme_bg_color = Color32::from_rgb(0x25, 0x21, 0x3e);
            visuals.override_text_color = Some(Color32::from_rgb(0xff, 0xec, 0xd6));
            visuals.selection.bg_fill = Color32::from_rgb(0xcb, 0x76, 0x5c);
            visuals.widgets.active.bg_fill = Color32::from_rgb(0xcb, 0x76, 0x5c);
            visuals.widgets.hovered.bg_fill = Color32::from_rgb(0xcb, 0x76, 0x5c);
            visuals.widgets.inactive.bg_fill = Color32::from_rgb(0x7a, 0x4a, 0x5a);
            visuals.hyperlink_color = Color32::from_rgb(0xff, 0xb8, 0x73);
        }
    }

    ctx.set_visuals(visuals);
}

/// Get the accent color for the current theme
pub fn accent_color(theme: &Theme) -> Color32 {
    match theme {
        Theme::Dark => Color32::from_rgb(0x4a, 0x7a, 0x96),
        Theme::Light => Color32::from_rgb(0xcb, 0x76, 0x5c),
    }
}

/// Get the secondary/highlight color for the current theme
pub fn secondary_color(theme: &Theme) -> Color32 {
    match theme {
        Theme::Dark => Color32::from_rgb(0xee, 0x86, 0x95),
        Theme::Light => Color32::from_rgb(0xff, 0xb8, 0x73),
    }
}

/// Get the grid dot color for the current theme
pub fn grid_color(theme: &Theme) -> Color32 {
    match theme {
        Theme::Dark => Color32::from_rgba_premultiplied(0xfb, 0xbb, 0xad, 40),
        Theme::Light => Color32::from_rgba_premultiplied(0xff, 0xec, 0xd6, 40),
    }
}

/// Get the canvas background color for the current theme (used when no palette bg)
pub fn canvas_bg_color(theme: &Theme) -> Color32 {
    match theme {
        Theme::Dark => Color32::from_rgb(0x29, 0x28, 0x31),
        Theme::Light => Color32::from_rgb(0x25, 0x21, 0x3e),
    }
}
