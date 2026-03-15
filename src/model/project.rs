use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GridMode {
    #[default]
    Straight,
    Isometric,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaletteColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl PaletteColor {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn transparent() -> Self {
        Self { r: 0, g: 0, b: 0, a: 0 }
    }

    pub fn to_color32(self) -> egui::Color32 {
        egui::Color32::from_rgba_premultiplied(self.r, self.g, self.b, self.a)
    }

    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Self::new(r, g, b))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Palette {
    pub name: String,
    pub colors: Vec<PaletteColor>,
}

impl Palette {
    pub fn default_palette() -> Self {
        Self {
            name: "Default".to_string(),
            colors: vec![
                PaletteColor::transparent(),       // 0: transparent
                PaletteColor::new(0, 0, 0),        // 1: black
                PaletteColor::new(255, 255, 255),   // 2: white
                PaletteColor::new(255, 0, 0),       // 3: red
                PaletteColor::new(0, 255, 0),       // 4: green
                PaletteColor::new(0, 0, 255),       // 5: blue
                PaletteColor::new(255, 255, 0),     // 6: yellow
                PaletteColor::new(255, 128, 0),     // 7: orange
                PaletteColor::new(128, 0, 255),     // 8: purple
                PaletteColor::new(128, 128, 128),   // 9: gray
            ],
        }
    }

    pub fn get_color(&self, index: u8) -> PaletteColor {
        self.colors.get(index as usize).copied().unwrap_or(PaletteColor::transparent())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorPreferences {
    pub theme: Theme,
    pub grid_size: u32,
    pub grid_mode: GridMode,
    pub show_dots: bool,
    pub show_lines: bool,
}

impl Default for EditorPreferences {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            grid_size: 8,
            grid_mode: GridMode::Straight,
            show_dots: true,
            show_lines: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub name: String,
    pub format_version: u32,
    pub palette: Palette,
    pub stroke_taper: bool,
    pub editor_preferences: EditorPreferences,
}

impl Project {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            format_version: 1,
            palette: Palette::default_palette(),
            stroke_taper: true,
            editor_preferences: EditorPreferences::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_palette_index_0_transparent() {
        let palette = Palette::default_palette();
        let c = palette.get_color(0);
        assert_eq!(c.a, 0);
    }

    #[test]
    fn test_palette_out_of_range_returns_transparent() {
        let palette = Palette::default_palette();
        let c = palette.get_color(255);
        assert_eq!(c.a, 0);
    }

    #[test]
    fn test_palette_color_from_hex() {
        let c = PaletteColor::from_hex("#ff8040").unwrap();
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 128);
        assert_eq!(c.b, 64);
        assert_eq!(c.a, 255);
    }

    #[test]
    fn test_project_serde_round_trip() {
        let project = Project::new("TestProject");
        let json = serde_json::to_string_pretty(&project).unwrap();
        let project2: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(project2.name, "TestProject");
        assert!(project2.stroke_taper);
        assert_eq!(project2.palette.colors.len(), 10);
        assert_eq!(project2.editor_preferences.grid_size, 8);
    }
}
