use std::path::Path;

use crate::model::project::{PaletteColor, Project};
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
