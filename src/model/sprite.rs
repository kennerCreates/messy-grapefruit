use serde::{Deserialize, Serialize};

use super::vec2::Vec2;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathVertex {
    pub id: String,
    pub pos: Vec2,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cp1: Option<Vec2>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cp2: Option<Vec2>,
}

impl PathVertex {
    pub fn new(pos: Vec2) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            pos,
            cp1: None,
            cp2: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrokeElement {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub vertices: Vec<PathVertex>,
    pub closed: bool,
    pub stroke_width: f32,
    pub stroke_color_index: u8,
    pub fill_color_index: u8,
    pub position: Vec2,
    pub rotation: f32,
    pub scale: Vec2,
    pub origin: Vec2,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub taper_override: Option<bool>,
}

impl StrokeElement {
    pub fn new(vertices: Vec<PathVertex>, stroke_width: f32, stroke_color_index: u8) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: None,
            vertices,
            closed: false,
            stroke_width,
            stroke_color_index,
            fill_color_index: 0,
            position: Vec2::ZERO,
            rotation: 0.0,
            scale: Vec2::ONE,
            origin: Vec2::ZERO,
            taper_override: None,
        }
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Layer {
    pub id: String,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub elements: Vec<StrokeElement>,
}

impl Layer {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            visible: true,
            locked: false,
            elements: Vec::new(),
        }
    }

    pub fn element_count(&self) -> usize {
        self.elements.len()
    }

    pub fn vertex_count(&self) -> usize {
        self.elements.iter().map(|e| e.vertex_count()).sum()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sprite {
    pub id: String,
    pub name: String,
    pub format_version: u32,
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub background_color_index: u8,
    pub layers: Vec<Layer>,
}

impl Sprite {
    pub fn new(name: impl Into<String>, canvas_width: u32, canvas_height: u32) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            format_version: 1,
            canvas_width,
            canvas_height,
            background_color_index: 0,
            layers: vec![Layer::new("Layer 1")],
        }
    }

    pub fn element_count(&self) -> usize {
        self.layers.iter().map(|l| l.element_count()).sum()
    }

    pub fn vertex_count(&self) -> usize {
        self.layers.iter().map(|l| l.vertex_count()).sum()
    }

    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sprite_new_has_one_layer() {
        let sprite = Sprite::new("Test", 256, 256);
        assert_eq!(sprite.layers.len(), 1);
        assert_eq!(sprite.layers[0].name, "Layer 1");
        assert!(sprite.layers[0].visible);
        assert!(!sprite.layers[0].locked);
    }

    #[test]
    fn test_sprite_serde_round_trip() {
        let mut sprite = Sprite::new("TestSprite", 128, 128);
        let elem = StrokeElement::new(
            vec![
                PathVertex::new(Vec2::new(0.0, 0.0)),
                PathVertex::new(Vec2::new(10.0, 20.0)),
                PathVertex::new(Vec2::new(30.0, 10.0)),
            ],
            2.0,
            1,
        );
        sprite.layers[0].elements.push(elem);

        let json = serde_json::to_string_pretty(&sprite).unwrap();
        let sprite2: Sprite = serde_json::from_str(&json).unwrap();

        assert_eq!(sprite2.name, "TestSprite");
        assert_eq!(sprite2.canvas_width, 128);
        assert_eq!(sprite2.layers.len(), 1);
        assert_eq!(sprite2.layers[0].elements.len(), 1);
        assert_eq!(sprite2.layers[0].elements[0].vertices.len(), 3);
        assert_eq!(sprite2.layers[0].elements[0].stroke_width, 2.0);
    }

    #[test]
    fn test_metrics() {
        let mut sprite = Sprite::new("Test", 256, 256);
        assert_eq!(sprite.element_count(), 0);
        assert_eq!(sprite.vertex_count(), 0);
        assert_eq!(sprite.layer_count(), 1);

        sprite.layers[0].elements.push(StrokeElement::new(
            vec![PathVertex::new(Vec2::ZERO), PathVertex::new(Vec2::ONE)],
            1.0,
            1,
        ));
        assert_eq!(sprite.element_count(), 1);
        assert_eq!(sprite.vertex_count(), 2);
    }
}
