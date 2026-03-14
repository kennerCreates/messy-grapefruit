use std::path::Path;
use std::io;
use crate::model::{Project, Sprite, PaletteColor};

/// Save a sprite to a .sprite JSON file
pub fn save_sprite(sprite: &Sprite, path: &Path) -> Result<(), io::Error> {
    let json = serde_json::to_string_pretty(sprite)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, json)
}

/// Load a sprite from a .sprite JSON file
pub fn load_sprite(path: &Path) -> Result<Sprite, io::Error> {
    let json = std::fs::read_to_string(path)?;
    serde_json::from_str(&json)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Save a project to a .spriteproj JSON file
pub fn save_project(project: &Project, path: &Path) -> Result<(), io::Error> {
    let json = serde_json::to_string_pretty(project)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, json)
}

/// Load a project from a .spriteproj JSON file
pub fn load_project(path: &Path) -> Result<Project, io::Error> {
    let json = std::fs::read_to_string(path)?;
    serde_json::from_str(&json)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Fetch a palette from Lospec by slug.
/// Endpoint: GET https://lospec.com/palette-list/{slug}.json
pub fn fetch_lospec_palette(slug: &str) -> Result<(String, Vec<PaletteColor>), Box<dyn std::error::Error>> {
    let url = format!("https://lospec.com/palette-list/{}.json", slug);
    let resp: serde_json::Value = reqwest::blocking::get(&url)?.json()?;
    let name = resp["name"].as_str().unwrap_or("Imported").to_string();
    let colors_hex = resp["colors"].as_array().ok_or("no colors field in response")?;
    let mut colors = vec![PaletteColor { r: 0, g: 0, b: 0, a: 0 }]; // Index 0 = transparent
    for hex in colors_hex {
        let h = hex.as_str().ok_or("color is not a string")?;
        // Lospec colors are hex without #
        let h = h.trim_start_matches('#');
        if h.len() < 6 {
            continue;
        }
        let r = u8::from_str_radix(&h[0..2], 16)?;
        let g = u8::from_str_radix(&h[2..4], 16)?;
        let b = u8::from_str_radix(&h[4..6], 16)?;
        colors.push(PaletteColor { r, g, b, a: 255 });
    }
    Ok((name, colors))
}
