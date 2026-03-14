use crate::models::project::PaletteColor;
use serde::Deserialize;

#[derive(Deserialize)]
struct LospecResponse {
    name: String,
    colors: Vec<String>, // hex color strings like "1a1c2c"
}

#[tauri::command]
pub fn fetch_lospec_palette(slug: String) -> Result<(String, Vec<PaletteColor>), String> {
    let url = format!("https://lospec.com/palette-list/{}.json", slug);
    let resp = reqwest::blocking::get(&url)
        .map_err(|e| format!("Failed to fetch palette: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Palette '{}' not found (HTTP {})", slug, resp.status()));
    }

    let data: LospecResponse = resp.json()
        .map_err(|e| format!("Failed to parse palette: {}", e))?;

    // Always include transparent as index 0
    let mut colors = vec![PaletteColor { r: 0, g: 0, b: 0, a: 0 }];

    for hex in &data.colors {
        let hex = hex.trim_start_matches('#');
        if hex.len() >= 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
            colors.push(PaletteColor { r, g, b, a: 255 });
        }
    }

    Ok((data.name, colors))
}
