use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Vec2;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Sprite {
    pub id: String,
    pub name: String,
    pub format_version: u32,
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub background_color_index: usize,
    pub layers: Vec<Layer>,
    pub skins: Vec<Skin>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Layer {
    pub id: String,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub elements: Vec<StrokeElement>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StrokeElement {
    pub id: String,
    pub name: Option<String>,
    pub vertices: Vec<PathVertex>,
    pub closed: bool,
    pub stroke_width: f32,
    pub stroke_color_index: usize,
    pub fill_color_index: usize,
    pub position: Vec2,
    pub rotation: f32,
    pub scale: Vec2,
    pub origin: Vec2,
    pub visible: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PathVertex {
    pub id: String,
    pub pos: Vec2,
    pub cp1: Option<Vec2>,
    pub cp2: Option<Vec2>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Skin {
    pub id: String,
    pub name: String,
    pub overrides: Vec<SkinOverride>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SkinOverride {
    pub element_id: String,
    pub stroke_color_index: Option<usize>,
    pub fill_color_index: Option<usize>,
    pub stroke_width: Option<f32>,
}

impl Sprite {
    pub fn new(name: &str, width: u32, height: u32) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            format_version: 1,
            canvas_width: width,
            canvas_height: height,
            background_color_index: 0,
            layers: vec![Layer::new("Layer 1")],
            skins: Vec::new(),
        }
    }
}

impl Layer {
    pub fn new(name: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            visible: true,
            locked: false,
            elements: Vec::new(),
        }
    }
}

impl StrokeElement {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: None,
            vertices: Vec::new(),
            closed: false,
            stroke_width: 2.0,
            stroke_color_index: 1,
            fill_color_index: 0,
            position: Vec2::default(),
            rotation: 0.0,
            scale: Vec2::new(1.0, 1.0),
            origin: Vec2::default(),
            visible: true,
        }
    }
}

impl PathVertex {
    pub fn new(pos: Vec2) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            pos,
            cp1: None,
            cp2: None,
        }
    }
}
