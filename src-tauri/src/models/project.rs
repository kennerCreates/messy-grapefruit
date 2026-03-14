use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PaletteColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Palette {
    pub name: String,
    pub colors: Vec<PaletteColor>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ExportMode {
    Bone,
    Spritesheet,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ExportLayout {
    Row,
    Column,
    Grid,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ExportSettings {
    pub mode: ExportMode,
    pub fps: u32,
    pub layout: ExportLayout,
    pub trim: bool,
    pub padding: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Dark,
    Light,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum GridMode {
    Standard,
    Isometric,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EditorPreferences {
    pub theme: Theme,
    pub grid_size: f64,
    pub grid_mode: GridMode,
    pub show_grid: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSpriteRef {
    pub id: String,
    pub file_path: String,
    pub position: Vec2,
    pub rotation: f64,
    pub z_order: i32,
    pub selected_animation_id: Option<String>,
    pub selected_skin_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub name: String,
    pub format_version: String,
    pub export_dir: Option<String>,
    pub palette: Palette,
    pub sprites: Vec<ProjectSpriteRef>,
    pub export_settings: ExportSettings,
    pub editor_preferences: EditorPreferences,
}
