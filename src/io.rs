use crate::model::project::Project;
use crate::model::sprite::Sprite;
use std::path::Path;

#[derive(Debug)]
pub enum IoError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Network(String),
}

impl std::fmt::Display for IoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IoError::Io(e) => write!(f, "IO error: {}", e),
            IoError::Json(e) => write!(f, "JSON error: {}", e),
            IoError::Network(e) => write!(f, "Network error: {}", e),
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

/// Save a sprite to a .sprite JSON file
pub fn save_sprite(sprite: &Sprite, path: &Path) -> Result<(), IoError> {
    let json = serde_json::to_string_pretty(sprite)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Load a sprite from a .sprite JSON file
pub fn load_sprite(path: &Path) -> Result<Sprite, IoError> {
    let json = std::fs::read_to_string(path)?;
    let sprite: Sprite = serde_json::from_str(&json)?;
    Ok(sprite)
}

/// Save a project to a .spriteproj JSON file
pub fn save_project(project: &Project, path: &Path) -> Result<(), IoError> {
    let json = serde_json::to_string_pretty(project)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Load a project from a .spriteproj JSON file
pub fn load_project(path: &Path) -> Result<Project, IoError> {
    let json = std::fs::read_to_string(path)?;
    let project: Project = serde_json::from_str(&json)?;
    Ok(project)
}

/// Fetch a palette from Lospec by slug.
/// Returns a list of hex color strings (e.g., ["ff0000", "00ff00", ...])
pub fn fetch_lospec_palette(slug: &str) -> Result<Vec<String>, IoError> {
    let url = format!("https://lospec.com/palette-list/{}.json", slug);

    let response = reqwest::blocking::get(&url)
        .map_err(|e| IoError::Network(e.to_string()))?;

    if !response.status().is_success() {
        return Err(IoError::Network(format!(
            "HTTP {} for palette '{}'",
            response.status(),
            slug
        )));
    }

    let body: serde_json::Value = response
        .json()
        .map_err(|e| IoError::Network(e.to_string()))?;

    let colors = body
        .get("colors")
        .and_then(|c| c.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| {
                    let hex = s.to_lowercase();
                    // Lospec returns 6-char hex; append "ff" for full opacity
                    if hex.len() == 6 { format!("{}ff", hex) } else { hex }
                }))
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    Ok(colors)
}
