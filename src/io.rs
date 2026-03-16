use std::path::{Path, PathBuf};

use crate::model::project::{EditorPreferences, Palette, PaletteColor, Project};
use crate::model::sprite::Sprite;

#[derive(Debug)]
pub enum IoError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Network(String),
}

impl std::fmt::Display for IoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IoError::Io(e) => write!(f, "IO error: {e}"),
            IoError::Json(e) => write!(f, "JSON error: {e}"),
            IoError::Network(e) => write!(f, "Network error: {e}"),
        }
    }
}

impl From<std::io::Error> for IoError {
    fn from(e: std::io::Error) -> Self {
        IoError::Io(e)
    }
}

impl From<serde_json::Error> for IoError {
    fn from(e: serde_json::Error) -> Self {
        IoError::Json(e)
    }
}

pub fn save_sprite(sprite: &Sprite, path: &Path) -> Result<(), IoError> {
    let json = serde_json::to_string_pretty(sprite)?;
    std::fs::write(path, json)?;
    Ok(())
}

pub fn load_sprite(path: &Path) -> Result<Sprite, IoError> {
    let data = std::fs::read_to_string(path)?;
    let sprite = serde_json::from_str(&data)?;
    Ok(sprite)
}

#[allow(dead_code)] // Phase 15: project management
pub fn save_project(project: &Project, path: &Path) -> Result<(), IoError> {
    let json = serde_json::to_string_pretty(project)?;
    std::fs::write(path, json)?;
    Ok(())
}

#[allow(dead_code)] // Phase 15: project management
pub fn load_project(path: &Path) -> Result<Project, IoError> {
    let data = std::fs::read_to_string(path)?;
    let project = serde_json::from_str(&data)?;
    Ok(project)
}

// ── App defaults persistence ──────────────────────────────────────────

/// Saved app defaults: palette + editor preferences.
/// Persisted to `<config_dir>/messy-grapefruit/defaults.json`.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppDefaults {
    pub palette: Palette,
    pub editor_preferences: EditorPreferences,
}

fn defaults_path() -> Option<PathBuf> {
    let dir = dirs::config_dir()?.join("messy-grapefruit");
    Some(dir.join("defaults.json"))
}

/// Save palette + editor preferences as application defaults.
pub fn save_app_defaults(project: &Project) {
    let Some(path) = defaults_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let defaults = AppDefaults {
        palette: project.palette.clone(),
        editor_preferences: project.editor_preferences.clone(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&defaults) {
        let _ = std::fs::write(&path, json);
    }
}

/// Load saved app defaults, if they exist.
pub fn load_app_defaults() -> Option<AppDefaults> {
    let path = defaults_path()?;
    let data = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

/// Fetch a palette from Lospec by slug (e.g., "endesga-32").
/// Returns the colors parsed from the API JSON response.
/// Uses blocking HTTP — UI will freeze briefly during the fetch.
pub fn fetch_lospec_palette(slug: &str) -> Result<Vec<PaletteColor>, IoError> {
    #[derive(serde::Deserialize)]
    struct LospecResponse {
        colors: Vec<String>,
    }

    let url = format!("https://lospec.com/palette-list/{slug}.json");
    let resp = reqwest::blocking::get(&url)
        .map_err(|e| IoError::Network(e.to_string()))?;
    let data: LospecResponse = resp
        .json()
        .map_err(|e| IoError::Network(e.to_string()))?;
    let colors: Vec<PaletteColor> = data
        .colors
        .iter()
        .filter_map(|hex| PaletteColor::from_hex(hex))
        .collect();
    Ok(colors)
}
