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
    Off,
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
        // Downgraded 32 by Polyphorge — https://lospec.com/palette-list/downgraded-32
        Self {
            name: "Downgraded 32".to_string(),
            colors: vec![
                PaletteColor::transparent(),           //  0: transparent
                PaletteColor::new(0x7b, 0x33, 0x4c),  //  1: #7b334c
                PaletteColor::new(0xa1, 0x4d, 0x55),  //  2: #a14d55
                PaletteColor::new(0xc7, 0x73, 0x69),  //  3: #c77369
                PaletteColor::new(0xe3, 0xa0, 0x84),  //  4: #e3a084
                PaletteColor::new(0xf2, 0xcb, 0x9b),  //  5: #f2cb9b
                PaletteColor::new(0xd3, 0x7b, 0x86),  //  6: #d37b86
                PaletteColor::new(0xaf, 0x5d, 0x8b),  //  7: #af5d8b
                PaletteColor::new(0x80, 0x40, 0x85),  //  8: #804085
                PaletteColor::new(0x5b, 0x33, 0x74),  //  9: #5b3374
                PaletteColor::new(0x41, 0x20, 0x51),  // 10: #412051
                PaletteColor::new(0x5c, 0x48, 0x6a),  // 11: #5c486a
                PaletteColor::new(0x88, 0x7d, 0x8d),  // 12: #887d8d
                PaletteColor::new(0xb8, 0xb4, 0xb2),  // 13: #b8b4b2
                PaletteColor::new(0xdc, 0xda, 0xc9),  // 14: #dcdac9
                PaletteColor::new(0xff, 0xff, 0xe0),  // 15: #ffffe0
                PaletteColor::new(0xb6, 0xf5, 0xdb),  // 16: #b6f5db
                PaletteColor::new(0x89, 0xd9, 0xd9),  // 17: #89d9d9
                PaletteColor::new(0x72, 0xb6, 0xcf),  // 18: #72b6cf
                PaletteColor::new(0x5c, 0x8b, 0xa8),  // 19: #5c8ba8
                PaletteColor::new(0x4e, 0x66, 0x79),  // 20: #4e6679
                PaletteColor::new(0x46, 0x49, 0x69),  // 21: #464969
                PaletteColor::new(0x44, 0x35, 0x5d),  // 22: #44355d
                PaletteColor::new(0x3d, 0x00, 0x3d),  // 23: #3d003d
                PaletteColor::new(0x62, 0x17, 0x48),  // 24: #621748
                PaletteColor::new(0x94, 0x2c, 0x4b),  // 25: #942c4b
                PaletteColor::new(0xc7, 0x42, 0x4f),  // 26: #c7424f
                PaletteColor::new(0xe0, 0x6b, 0x51),  // 27: #e06b51
                PaletteColor::new(0xf2, 0xa5, 0x61),  // 28: #f2a561
                PaletteColor::new(0xfc, 0xef, 0x8d),  // 29: #fcef8d
                PaletteColor::new(0xb1, 0xd4, 0x80),  // 30: #b1d480
                PaletteColor::new(0x80, 0xb8, 0x78),  // 31: #80b878
                PaletteColor::new(0x65, 0x8d, 0x78),  // 32: #658d78
            ],
        }
    }

    pub fn get_color(&self, index: u8) -> PaletteColor {
        self.colors.get(index as usize).copied().unwrap_or(PaletteColor::transparent())
    }
}

/// Palette indices for the 5 semantic theme color roles.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ThemeColorIndices {
    pub panel_bg: u8,
    pub canvas_bg: u8,
    pub mid: u8,
    pub selected: u8,
    pub icon_text: u8,
}

impl ThemeColorIndices {
    /// Default dark theme indices for the Downgraded 32 palette.
    pub fn default_dark() -> Self {
        Self { panel_bg: 23, canvas_bg: 10, mid: 19, selected: 27, icon_text: 5 }
    }

    /// Default light theme indices for the Downgraded 32 palette.
    pub fn default_light() -> Self {
        Self { panel_bg: 15, canvas_bg: 5, mid: 31, selected: 1, icon_text: 23 }
    }

    /// Returns the 5 role names in order.
    pub const ROLE_NAMES: [&'static str; 5] = ["Panel", "Canvas", "Accent", "Highlight", "Text"];

    /// Get the index for a role by position (0..5).
    pub fn get(&self, role: usize) -> u8 {
        match role {
            0 => self.panel_bg,
            1 => self.canvas_bg,
            2 => self.mid,
            3 => self.selected,
            4 => self.icon_text,
            _ => 0,
        }
    }

    /// Set the index for a role by position (0..5).
    pub fn set(&mut self, role: usize, index: u8) {
        match role {
            0 => self.panel_bg = index,
            1 => self.canvas_bg = index,
            2 => self.mid = index,
            3 => self.selected = index,
            4 => self.icon_text = index,
            _ => {}
        }
    }
}

/// Auto-pick theme color indices from a palette by sorting colors by luminance.
pub fn auto_pick_theme_colors(palette: &Palette) -> (ThemeColorIndices, ThemeColorIndices) {
    let mut indexed_lum: Vec<(u8, f32)> = palette
        .colors
        .iter()
        .enumerate()
        .skip(1) // skip transparent
        .filter(|(_, c)| c.a > 0)
        .map(|(i, c)| {
            let lum = 0.299 * c.r as f32 + 0.587 * c.g as f32 + 0.114 * c.b as f32;
            (i as u8, lum)
        })
        .collect();
    indexed_lum.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    let n = indexed_lum.len();
    if n < 5 {
        let fb = indexed_lum.first().map(|x| x.0).unwrap_or(1);
        let indices = ThemeColorIndices { panel_bg: fb, canvas_bg: fb, mid: fb, selected: fb, icon_text: fb };
        return (indices, indices);
    }

    let pick = |fraction: f32| -> u8 {
        let idx = ((n - 1) as f32 * fraction).round() as usize;
        indexed_lum[idx].0
    };

    let dark = ThemeColorIndices {
        panel_bg: pick(0.0),
        canvas_bg: pick(0.15),
        mid: pick(0.5),
        selected: pick(0.75),
        icon_text: pick(0.9),
    };
    let light = ThemeColorIndices {
        panel_bg: pick(1.0),
        canvas_bg: pick(0.85),
        mid: pick(0.5),
        selected: pick(0.25),
        icon_text: pick(0.1),
    };

    (dark, light)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorPreferences {
    pub theme: Theme,
    pub grid_size: u32,
    pub grid_mode: GridMode,
    pub show_dots: bool,
    #[serde(default = "ThemeColorIndices::default_dark")]
    pub dark_theme_colors: ThemeColorIndices,
    #[serde(default = "ThemeColorIndices::default_light")]
    pub light_theme_colors: ThemeColorIndices,
}

impl Default for EditorPreferences {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            grid_size: 8,
            grid_mode: GridMode::Off,
            show_dots: true,
            dark_theme_colors: ThemeColorIndices::default_dark(),
            light_theme_colors: ThemeColorIndices::default_light(),
        }
    }
}

/// A single hatch layer: one set of parallel lines at a given angle.
/// Color and stroke width come from the element's stroke properties, not the pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HatchLayer {
    pub angle: f32,
    pub spacing: f32,
    #[serde(default)]
    pub offset: f32,
}

/// A hatch pattern: one or more layers of parallel lines.
/// Multi-layer patterns produce cross-hatch effects.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HatchPattern {
    pub id: String,
    pub name: String,
    pub layers: Vec<HatchLayer>,
}

impl HatchPattern {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            layers: vec![HatchLayer {
                angle: 45.0,
                spacing: 8.0,
                offset: 0.0,
            }],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub name: String,
    pub format_version: u32,
    pub palette: Palette,
    pub min_corner_radius: f32,
    pub editor_preferences: EditorPreferences,
    #[serde(default)]
    pub hatch_patterns: Vec<HatchPattern>,
}

impl Project {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            format_version: 1,
            palette: Palette::default_palette(),
            min_corner_radius: 4.0,
            editor_preferences: EditorPreferences::default(),
            hatch_patterns: Vec::new(),
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
        assert_eq!(project2.palette.colors.len(), 33);
        assert_eq!(project2.editor_preferences.grid_size, 8);
        assert!(project2.hatch_patterns.is_empty());
    }

    #[test]
    fn test_backward_compat_no_hatch_patterns() {
        let json = r#"{
            "name": "OldProject",
            "formatVersion": 1,
            "palette": {"name": "Test", "colors": [{"r":0,"g":0,"b":0,"a":0}]},
            "minCornerRadius": 4.0,
            "editorPreferences": {"theme": "dark", "gridSize": 8, "gridMode": "off", "showDots": true}
        }"#;
        let project: Project = serde_json::from_str(json).unwrap();
        assert!(project.hatch_patterns.is_empty());
    }

    #[test]
    fn test_hatch_pattern_serde_round_trip() {
        let mut project = Project::new("Test");
        let mut pattern = HatchPattern::new("Cross-hatch");
        pattern.layers.push(HatchLayer {
            angle: 135.0,
            spacing: 8.0,
            offset: 0.0,
        });
        project.hatch_patterns.push(pattern);

        let json = serde_json::to_string(&project).unwrap();
        let project2: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(project2.hatch_patterns.len(), 1);
        assert_eq!(project2.hatch_patterns[0].name, "Cross-hatch");
        assert_eq!(project2.hatch_patterns[0].layers.len(), 2);
        assert_eq!(project2.hatch_patterns[0].layers[1].angle, 135.0);
    }
}
