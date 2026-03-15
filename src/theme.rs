use egui::Color32;

use crate::model::project::Theme;

pub struct ThemeColors {
    pub bg: Color32,
    pub panels: Color32,
    pub accent: Color32,
    pub secondary: Color32,
    pub text: Color32,
}

pub fn theme_colors(theme: Theme) -> ThemeColors {
    match theme {
        Theme::Dark => ThemeColors {
            bg: Color32::from_rgb(0x29, 0x28, 0x31),
            panels: Color32::from_rgb(0x33, 0x3f, 0x58),
            accent: Color32::from_rgb(0x4a, 0x7a, 0x96),
            secondary: Color32::from_rgb(0xee, 0x86, 0x95),
            text: Color32::from_rgb(0xfb, 0xbb, 0xad),
        },
        Theme::Light => ThemeColors {
            bg: Color32::from_rgb(0xff, 0xec, 0xd6),
            panels: Color32::from_rgb(0xff, 0xb8, 0x73),
            accent: Color32::from_rgb(0xcb, 0x76, 0x5c),
            secondary: Color32::from_rgb(0x7a, 0x4a, 0x5a),
            text: Color32::from_rgb(0x25, 0x21, 0x3e),
        },
    }
}

pub fn apply_theme(ctx: &egui::Context, theme: Theme) {
    let tc = theme_colors(theme);
    let mut visuals = match theme {
        Theme::Dark => egui::Visuals::dark(),
        Theme::Light => egui::Visuals::light(),
    };

    visuals.panel_fill = tc.panels;
    visuals.window_fill = tc.panels;
    visuals.extreme_bg_color = tc.bg;
    visuals.faint_bg_color = tc.bg;

    visuals.widgets.noninteractive.fg_stroke.color = tc.text;
    visuals.widgets.inactive.fg_stroke.color = tc.text;
    visuals.widgets.hovered.fg_stroke.color = tc.text;
    visuals.widgets.active.fg_stroke.color = tc.text;

    visuals.selection.bg_fill = tc.accent;
    visuals.hyperlink_color = tc.accent;
    visuals.widgets.hovered.bg_fill = tc.accent;

    ctx.set_visuals(visuals);
}

pub fn canvas_bg_color(theme: Theme) -> Color32 {
    match theme {
        Theme::Dark => Color32::from_rgb(0x22, 0x21, 0x2a),
        Theme::Light => Color32::from_rgb(0xff, 0xf5, 0xe6),
    }
}

pub fn grid_dot_color(theme: Theme) -> Color32 {
    match theme {
        Theme::Dark => Color32::from_rgba_unmultiplied(0xfb, 0xbb, 0xad, 80),
        Theme::Light => Color32::from_rgba_unmultiplied(0x25, 0x21, 0x3e, 80),
    }
}

pub fn grid_line_color(theme: Theme) -> Color32 {
    match theme {
        Theme::Dark => Color32::from_rgba_unmultiplied(0xfb, 0xbb, 0xad, 30),
        Theme::Light => Color32::from_rgba_unmultiplied(0x25, 0x21, 0x3e, 30),
    }
}

pub fn canvas_boundary_color(theme: Theme) -> Color32 {
    match theme {
        Theme::Dark => Color32::from_rgba_unmultiplied(0x4a, 0x7a, 0x96, 150),
        Theme::Light => Color32::from_rgba_unmultiplied(0xcb, 0x76, 0x5c, 150),
    }
}

pub fn hover_highlight_color(theme: Theme) -> Color32 {
    match theme {
        Theme::Dark => Color32::from_rgba_unmultiplied(0x4a, 0x7a, 0x96, 128),
        Theme::Light => Color32::from_rgba_unmultiplied(0xcb, 0x76, 0x5c, 128),
    }
}

pub fn merge_preview_color(theme: Theme) -> Color32 {
    match theme {
        Theme::Dark => Color32::from_rgba_unmultiplied(0x80, 0xff, 0x80, 180),
        Theme::Light => Color32::from_rgba_unmultiplied(0x00, 0x80, 0x00, 180),
    }
}

pub fn rubber_band_color(theme: Theme) -> Color32 {
    match theme {
        Theme::Dark => Color32::from_rgba_unmultiplied(0xfb, 0xbb, 0xad, 100),
        Theme::Light => Color32::from_rgba_unmultiplied(0x25, 0x21, 0x3e, 100),
    }
}
