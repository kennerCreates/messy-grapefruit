use std::cell::RefCell;

use egui::Color32;

use crate::model::project::{Palette, Project, Theme, ThemeColorIndices};

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
#[derive(Clone, Copy)]
pub struct ThemeColors {
    pub panel_bg: Color32,
    pub canvas_bg: Color32,
    pub mid: Color32,
    pub selected: Color32,
    pub icon_text: Color32,
}

// Hardcoded fallback colors (Downgraded 32 palette).
const DEFAULT_DARK: ThemeColors = ThemeColors {
    panel_bg:  Color32::from_rgb(0x3d, 0x00, 0x3d),
    canvas_bg: Color32::from_rgb(0x41, 0x20, 0x51),
    mid:       Color32::from_rgb(0x5c, 0x8b, 0xa8),
    selected:  Color32::from_rgb(0xe0, 0x6b, 0x51),
    icon_text: Color32::from_rgb(0xf2, 0xcb, 0x9b),
};

const DEFAULT_LIGHT: ThemeColors = ThemeColors {
    panel_bg:  Color32::from_rgb(0xff, 0xff, 0xe0),
    canvas_bg: Color32::from_rgb(0xf2, 0xcb, 0x9b),
    mid:       Color32::from_rgb(0x80, 0xb8, 0x78),
    selected:  Color32::from_rgb(0x7b, 0x33, 0x4c),
    icon_text: Color32::from_rgb(0x3d, 0x00, 0x3d),
};

thread_local! {
    static ACTIVE_DARK: RefCell<ThemeColors> = const { RefCell::new(DEFAULT_DARK) };
    static ACTIVE_LIGHT: RefCell<ThemeColors> = const { RefCell::new(DEFAULT_LIGHT) };
}

/// Resolve theme colors from palette indices.
fn resolve_from_palette(palette: &Palette, indices: &ThemeColorIndices) -> ThemeColors {
    ThemeColors {
        panel_bg:  palette.get_color(indices.panel_bg).to_color32(),
        canvas_bg: palette.get_color(indices.canvas_bg).to_color32(),
        mid:       palette.get_color(indices.mid).to_color32(),
        selected:  palette.get_color(indices.selected).to_color32(),
        icon_text: palette.get_color(indices.icon_text).to_color32(),
    }
}

/// Update the cached theme colors from the project's palette and indices.
/// Called once per frame from apply_theme.
fn cache_theme_colors(project: &Project) {
    let dark = resolve_from_palette(&project.palette, &project.editor_preferences.dark_theme_colors);
    let light = resolve_from_palette(&project.palette, &project.editor_preferences.light_theme_colors);
    ACTIVE_DARK.with(|c| *c.borrow_mut() = dark);
    ACTIVE_LIGHT.with(|c| *c.borrow_mut() = light);
}

/// Get the current theme colors (reads from per-frame cache).
pub fn theme_colors(theme: Theme) -> ThemeColors {
    match theme {
        Theme::Dark => ACTIVE_DARK.with(|c| *c.borrow()),
        Theme::Light => ACTIVE_LIGHT.with(|c| *c.borrow()),
    }
}

pub fn apply_theme(ctx: &egui::Context, project: &Project) {
    cache_theme_colors(project);
    let theme = project.editor_preferences.theme;
    let tc = theme_colors(theme);
    let mut visuals = match theme {
        Theme::Dark => egui::Visuals::dark(),
        Theme::Light => egui::Visuals::light(),
    };

    // Panel/window backgrounds = 2nd darkest (canvas_bg) for toolbar panels
    visuals.panel_fill = tc.canvas_bg;
    visuals.window_fill = tc.canvas_bg;
    // Text input backgrounds: darkest color so they look "sunken" / distinct
    visuals.extreme_bg_color = tc.panel_bg;
    visuals.faint_bg_color = tc.canvas_bg;

    // Window border/stroke subtle
    visuals.window_stroke = egui::Stroke::new(1.0, tc.panel_bg);

    // Text/icons = lightest (icon_text)
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, tc.icon_text);
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, tc.icon_text);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, tc.icon_text);
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, tc.icon_text);

    // Slider rail + text input bg = darkest so they're visible against panel
    visuals.widgets.inactive.bg_fill = tc.panel_bg;
    // Icon buttons: transparent so they blend with panel
    visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;

    // Non-interactive widget bg = canvas_bg (seamless with panels)
    visuals.widgets.noninteractive.bg_fill = tc.canvas_bg;
    visuals.widgets.noninteractive.weak_bg_fill = tc.canvas_bg;

    // Hover = selected (2nd lightest / 2nd darkest)
    visuals.widgets.hovered.bg_fill = tc.selected;
    visuals.widgets.hovered.weak_bg_fill = tc.selected;

    // Active/pressed = middle color
    visuals.widgets.active.bg_fill = tc.mid;
    visuals.widgets.active.weak_bg_fill = tc.mid;

    // Selection/toggle highlight = middle color
    visuals.selection.bg_fill = tc.mid;
    visuals.selection.stroke = egui::Stroke::new(1.0, tc.icon_text);

    visuals.hyperlink_color = tc.mid;

    // No border on inactive widgets (keeps buttons clean)
    visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
    // Visible border on hover/active for all widgets
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, tc.mid);
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, tc.icon_text);

    // Separator color = darkest, subtle against panel bg
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(0.5, tc.panel_bg);

    ctx.set_visuals(visuals);
}

// --- Canvas colors ---

/// Canvas background = darkest (panel_bg)
pub fn canvas_bg_color(theme: Theme) -> Color32 {
    theme_colors(theme).panel_bg
}

// --- Grid colors ---

/// Grid dots = 2nd darkest (canvas_bg)
pub fn grid_dot_color(theme: Theme) -> Color32 {
    theme_colors(theme).canvas_bg
}

/// Grid lines = 2nd darkest (canvas_bg)
pub fn grid_line_color(theme: Theme) -> Color32 {
    theme_colors(theme).canvas_bg
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
        Theme::Dark => Color32::from_rgba_unmultiplied(0xb1, 0xd4, 0x80, 180),  // #b1d480
        Theme::Light => Color32::from_rgba_unmultiplied(0x65, 0x8d, 0x78, 180), // #658d78
    }
}

pub fn rubber_band_color(theme: Theme) -> Color32 {
    let tc = theme_colors(theme);
    Color32::from_rgba_unmultiplied(tc.icon_text.r(), tc.icon_text.g(), tc.icon_text.b(), 100)
}

/// Floating panels (toolbar, sidebar, status bar) = 2nd darkest (canvas_bg)
pub fn floating_panel_color(theme: Theme) -> Color32 {
    let tc = theme_colors(theme);
    Color32::from_rgba_unmultiplied(tc.canvas_bg.r(), tc.canvas_bg.g(), tc.canvas_bg.b(), 240)
}

/// The selected/active highlight color (for toggled buttons, active tools).
pub fn selected_color(theme: Theme) -> Color32 {
    theme_colors(theme).selected
}

/// Selection highlight outline on canvas (for selected elements).
pub fn selection_highlight_color(theme: Theme) -> Color32 {
    let tc = theme_colors(theme);
    Color32::from_rgba_unmultiplied(tc.selected.r(), tc.selected.g(), tc.selected.b(), 160)
}

/// Marquee selection rectangle color.
pub fn marquee_color(theme: Theme) -> Color32 {
    let tc = theme_colors(theme);
    Color32::from_rgba_unmultiplied(tc.mid.r(), tc.mid.g(), tc.mid.b(), 100)
}

/// Transform handle color.
pub fn handle_color(theme: Theme) -> Color32 {
    theme_colors(theme).icon_text
}

/// Vertex snap indicator color (blue tint, distinct from green merge).
pub fn vertex_snap_color(theme: Theme) -> Color32 {
    match theme {
        Theme::Dark => Color32::from_rgba_unmultiplied(0x64, 0xb5, 0xf6, 200),  // light blue
        Theme::Light => Color32::from_rgba_unmultiplied(0x1e, 0x88, 0xe5, 200), // blue
    }
}

/// Eraser hover highlight color (red tint for deletion preview).
pub fn eraser_highlight_color(theme: Theme) -> Color32 {
    match theme {
        Theme::Dark => Color32::from_rgba_unmultiplied(0xef, 0x53, 0x50, 200),  // red
        Theme::Light => Color32::from_rgba_unmultiplied(0xc6, 0x28, 0x28, 200), // dark red
    }
}

/// Symmetry axis line color.
pub fn symmetry_axis_color(theme: Theme) -> Color32 {
    match theme {
        Theme::Dark => Color32::from_rgba_unmultiplied(0xce, 0x93, 0xd8, 140),  // light purple
        Theme::Light => Color32::from_rgba_unmultiplied(0x8e, 0x24, 0xaa, 140), // purple
    }
}

/// Symmetry ghost preview color.
pub fn symmetry_ghost_color(theme: Theme) -> Color32 {
    let tc = theme_colors(theme);
    Color32::from_rgba_unmultiplied(tc.icon_text.r(), tc.icon_text.g(), tc.icon_text.b(), 70)
}

/// Origin crosshair color.
#[allow(dead_code)] // Phase 3+: origin point handle rendering
pub fn origin_color(theme: Theme) -> Color32 {
    let tc = theme_colors(theme);
    Color32::from_rgba_unmultiplied(tc.selected.r(), tc.selected.g(), tc.selected.b(), 200)
}

pub fn flow_curve_color(theme: Theme) -> Color32 {
    match theme {
        Theme::Dark => Color32::from_rgba_unmultiplied(100, 200, 255, 180),
        Theme::Light => Color32::from_rgba_unmultiplied(30, 120, 200, 180),
    }
}

/// Apply a visible border to text inputs (DragValue, Slider, TextEdit) within the closure.
/// Buttons remain borderless because they use weak_bg_fill (transparent).
pub fn with_input_style<R>(ui: &mut egui::Ui, theme: Theme, f: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let tc = theme_colors(theme);
    let prev = ui.visuals().widgets.inactive.bg_stroke;
    ui.visuals_mut().widgets.inactive.bg_stroke = egui::Stroke::new(1.0, tc.mid);
    let result = f(ui);
    ui.visuals_mut().widgets.inactive.bg_stroke = prev;
    result
}
