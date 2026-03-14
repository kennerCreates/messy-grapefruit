use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub name: String,
    pub format_version: u32,
    pub export_dir: String,
    pub palette: Palette,
    pub sprites: Vec<ProjectSpriteRef>,
    pub export_settings: ExportSettings,
    pub editor_preferences: EditorPreferences,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            name: "Untitled Project".to_string(),
            format_version: 1,
            export_dir: "export".to_string(),
            palette: Palette::default(),
            sprites: Vec::new(),
            export_settings: ExportSettings::default(),
            editor_preferences: EditorPreferences::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Palette {
    pub name: String,
    pub colors: Vec<PaletteColor>,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            colors: vec![
                PaletteColor {
                    hex: "00000000".to_string(),
                    name: Some("Transparent".to_string()),
                },
                PaletteColor {
                    hex: "000000ff".to_string(),
                    name: Some("Black".to_string()),
                },
                PaletteColor {
                    hex: "ffffffff".to_string(),
                    name: Some("White".to_string()),
                },
                PaletteColor {
                    hex: "ee8695ff".to_string(),
                    name: Some("Rose".to_string()),
                },
                PaletteColor {
                    hex: "4a7a96ff".to_string(),
                    name: Some("Teal".to_string()),
                },
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaletteColor {
    pub hex: String,
    pub name: Option<String>,
}

impl PaletteColor {
    pub fn to_color32(&self) -> egui::Color32 {
        parse_hex_color(&self.hex)
    }
}

pub fn parse_hex_color(hex: &str) -> egui::Color32 {
    let hex = hex.trim_start_matches('#');
    match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
            egui::Color32::from_rgb(r, g, b)
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
            let a = u8::from_str_radix(&hex[6..8], 16).unwrap_or(255);
            egui::Color32::from_rgba_unmultiplied(r, g, b, a)
        }
        _ => egui::Color32::TRANSPARENT,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSpriteRef {
    pub id: String,
    pub file_path: String,
    pub position: super::vec2::Vec2,
    pub rotation: f32,
    pub z_order: i32,
    pub selected_animation_id: Option<String>,
    pub selected_skin_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorPreferences {
    pub theme: Theme,
    pub grid_size: f32,
    pub grid_mode: GridMode,
    pub show_grid: bool,
}

impl Default for EditorPreferences {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            grid_size: 8.0,
            grid_mode: GridMode::Standard,
            show_grid: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GridMode {
    Standard,
    Isometric,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportSettings {
    pub mode: String,
    pub fps: u32,
    pub layout: String,
    pub trim: bool,
    pub padding: u32,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            mode: "bone".to_string(),
            fps: 12,
            layout: "grid".to_string(),
            trim: true,
            padding: 1,
        }
    }
}
