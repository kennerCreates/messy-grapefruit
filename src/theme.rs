use crate::model::project::Theme;

/// Dark mode: Twilight 5
const DARK_BG: egui::Color32 = egui::Color32::from_rgb(0x29, 0x28, 0x31);
const DARK_PANELS: egui::Color32 = egui::Color32::from_rgb(0x33, 0x3f, 0x58);
const DARK_ACCENT: egui::Color32 = egui::Color32::from_rgb(0x4a, 0x7a, 0x96);
const DARK_SECONDARY: egui::Color32 = egui::Color32::from_rgb(0xee, 0x86, 0x95);
const DARK_TEXT: egui::Color32 = egui::Color32::from_rgb(0xfb, 0xbb, 0xad);

/// Light mode: Golden Sunset
const LIGHT_BG: egui::Color32 = egui::Color32::from_rgb(0xff, 0xec, 0xd6);
const LIGHT_PANELS: egui::Color32 = egui::Color32::from_rgb(0xff, 0xb8, 0x73);
const LIGHT_ACCENT: egui::Color32 = egui::Color32::from_rgb(0xcb, 0x76, 0x5c);
const LIGHT_SECONDARY: egui::Color32 = egui::Color32::from_rgb(0x7a, 0x4a, 0x5a);
const LIGHT_TEXT: egui::Color32 = egui::Color32::from_rgb(0x25, 0x21, 0x3e);

pub fn apply_theme(ctx: &egui::Context, theme: Theme) {
    let mut visuals = match theme {
        Theme::Dark => egui::Visuals::dark(),
        Theme::Light => egui::Visuals::light(),
    };

    let (bg, panels, accent, text) = match theme {
        Theme::Dark => (DARK_BG, DARK_PANELS, DARK_ACCENT, DARK_TEXT),
        Theme::Light => (LIGHT_BG, LIGHT_PANELS, LIGHT_ACCENT, LIGHT_TEXT),
    };

    visuals.panel_fill = panels;
    visuals.window_fill = panels;
    visuals.extreme_bg_color = bg;
    visuals.faint_bg_color = bg;

    visuals.override_text_color = Some(text);

    // Widget styling
    visuals.widgets.noninteractive.bg_fill = panels;
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, text);

    visuals.widgets.inactive.bg_fill = bg;
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, text);

    visuals.widgets.hovered.bg_fill = accent;
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, text);

    visuals.widgets.active.bg_fill = accent;
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, text);

    visuals.selection.bg_fill = accent;
    visuals.selection.stroke = egui::Stroke::new(1.0, text);

    ctx.set_visuals(visuals);
}

pub fn accent_color(theme: Theme) -> egui::Color32 {
    match theme {
        Theme::Dark => DARK_ACCENT,
        Theme::Light => LIGHT_ACCENT,
    }
}

pub fn secondary_color(theme: Theme) -> egui::Color32 {
    match theme {
        Theme::Dark => DARK_SECONDARY,
        Theme::Light => LIGHT_SECONDARY,
    }
}

pub fn grid_color(theme: Theme) -> egui::Color32 {
    match theme {
        Theme::Dark => egui::Color32::from_rgba_unmultiplied(0xfb, 0xbb, 0xad, 40),
        Theme::Light => egui::Color32::from_rgba_unmultiplied(0x25, 0x21, 0x3e, 40),
    }
}

pub fn canvas_bg_color(theme: Theme) -> egui::Color32 {
    match theme {
        Theme::Dark => DARK_BG,
        Theme::Light => LIGHT_BG,
    }
}

pub fn text_color(theme: Theme) -> egui::Color32 {
    match theme {
        Theme::Dark => DARK_TEXT,
        Theme::Light => LIGHT_TEXT,
    }
}
