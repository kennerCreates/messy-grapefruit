use std::fs;
use std::path::Path;
use uuid::Uuid;

use crate::models::project::{
    EditorPreferences, ExportLayout, ExportMode, ExportSettings, GridMode, Palette, PaletteColor,
    Project, Theme,
};
use crate::models::sprite::{Layer, Sprite};

#[tauri::command]
pub fn new_project(name: String, dir: String) -> Result<Project, String> {
    let project = Project {
        name,
        format_version: "1.0".to_string(),
        export_dir: None,
        palette: Palette {
            name: "Default".to_string(),
            colors: vec![PaletteColor {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            }],
        },
        sprites: vec![],
        export_settings: ExportSettings {
            mode: ExportMode::Bone,
            fps: 12,
            layout: ExportLayout::Grid,
            trim: true,
            padding: 1,
        },
        editor_preferences: EditorPreferences {
            theme: Theme::Dark,
            grid_size: 16.0,
            grid_mode: GridMode::Standard,
            show_grid: true,
        },
    };

    let dir_path = Path::new(&dir);
    fs::create_dir_all(dir_path).map_err(|e| format!("Failed to create directory: {}", e))?;

    let file_path = dir_path.join(format!("{}.spriteproj", &project.name));
    let json =
        serde_json::to_string_pretty(&project).map_err(|e| format!("Serialization error: {}", e))?;
    fs::write(&file_path, json).map_err(|e| format!("Failed to write project file: {}", e))?;

    Ok(project)
}

#[tauri::command]
pub fn open_project(path: String) -> Result<Project, String> {
    let contents =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read project file: {}", e))?;
    let project: Project =
        serde_json::from_str(&contents).map_err(|e| format!("Failed to parse project file: {}", e))?;
    Ok(project)
}

#[tauri::command]
pub fn save_project(project: Project, path: String) -> Result<(), String> {
    let json =
        serde_json::to_string_pretty(&project).map_err(|e| format!("Serialization error: {}", e))?;
    fs::write(&path, json).map_err(|e| format!("Failed to write project file: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn new_sprite(name: String, width: u32, height: u32) -> Result<Sprite, String> {
    let sprite = Sprite {
        id: Uuid::new_v4().to_string(),
        name,
        format_version: "1.0".to_string(),
        canvas_width: width,
        canvas_height: height,
        background_color_index: 0,
        layers: vec![Layer {
            id: Uuid::new_v4().to_string(),
            name: "Layer 1".to_string(),
            visible: true,
            locked: false,
            elements: vec![],
            socket: None,
            constraints: None,
        }],
        skins: vec![],
        animations: vec![],
    };

    Ok(sprite)
}

#[tauri::command]
pub fn open_sprite(path: String) -> Result<Sprite, String> {
    let contents =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read sprite file: {}", e))?;
    let sprite: Sprite =
        serde_json::from_str(&contents).map_err(|e| format!("Failed to parse sprite file: {}", e))?;
    Ok(sprite)
}

#[tauri::command]
pub fn save_sprite(sprite: Sprite, path: String) -> Result<(), String> {
    let json =
        serde_json::to_string_pretty(&sprite).map_err(|e| format!("Serialization error: {}", e))?;
    fs::write(&path, json).map_err(|e| format!("Failed to write sprite file: {}", e))?;
    Ok(())
}
