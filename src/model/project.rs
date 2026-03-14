use serde::{Deserialize, Serialize};

use super::Vec2;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Project {
    pub name: String,
    pub format_version: u32,
    pub export_dir: Option<String>,
    pub palette: Palette,
    pub sprites: Vec<ProjectSpriteRef>,
    pub editor_preferences: EditorPreferences,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Palette {
    pub name: String,
    pub colors: Vec<PaletteColor>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PaletteColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProjectSpriteRef {
    pub id: String,
    pub file_path: String,
    pub position: Vec2,
    pub rotation: f32,
    pub z_order: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EditorPreferences {
    pub theme: Theme,
    pub grid_size: f32,
    pub grid_mode: GridMode,
    pub show_grid: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Theme {
    Dark,
    Light,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum GridMode {
    Standard,
    Isometric,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            name: "Untitled Project".to_string(),
            format_version: 1,
            export_dir: None,
            palette: Palette {
                name: "Default".to_string(),
                colors: vec![PaletteColor { r: 0, g: 0, b: 0, a: 0 }],
            },
            sprites: Vec::new(),
            editor_preferences: EditorPreferences::default(),
        }
    }
}

impl Default for PaletteColor {
    fn default() -> Self {
        Self { r: 0, g: 0, b: 0, a: 0 }
    }
}

impl Default for EditorPreferences {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            grid_size: 16.0,
            grid_mode: GridMode::default(),
            show_grid: true,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme::Dark
    }
}

impl Default for GridMode {
    fn default() -> Self {
        GridMode::Standard
    }
}

impl PaletteColor {
    pub fn to_color32(&self) -> egui::Color32 {
        egui::Color32::from_rgba_unmultiplied(self.r, self.g, self.b, self.a)
    }

    pub fn from_hex(hex: &str) -> Result<Self, String> {
        let hex = hex.trim_start_matches('#');
        match hex.len() {
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16)
                    .map_err(|e| format!("Invalid red component: {}", e))?;
                let g = u8::from_str_radix(&hex[2..4], 16)
                    .map_err(|e| format!("Invalid green component: {}", e))?;
                let b = u8::from_str_radix(&hex[4..6], 16)
                    .map_err(|e| format!("Invalid blue component: {}", e))?;
                Ok(Self { r, g, b, a: 255 })
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16)
                    .map_err(|e| format!("Invalid red component: {}", e))?;
                let g = u8::from_str_radix(&hex[2..4], 16)
                    .map_err(|e| format!("Invalid green component: {}", e))?;
                let b = u8::from_str_radix(&hex[4..6], 16)
                    .map_err(|e| format!("Invalid blue component: {}", e))?;
                let a = u8::from_str_radix(&hex[6..8], 16)
                    .map_err(|e| format!("Invalid alpha component: {}", e))?;
                Ok(Self { r, g, b, a })
            }
            _ => Err(format!(
                "Invalid hex color '{}': expected 6 or 8 hex digits",
                hex
            )),
        }
    }
}

impl Project {
    pub fn new() -> Self {
        Self::default()
    }
}
