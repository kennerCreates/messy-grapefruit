use serde::{Deserialize, Serialize};

use super::vec2::Vec2;

fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathVertex {
    pub id: String,
    pub pos: Vec2,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cp1: Option<Vec2>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cp2: Option<Vec2>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub manual_handles: bool,
}

impl PathVertex {
    pub fn new(pos: Vec2) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            pos,
            cp1: None,
            cp2: None,
            manual_handles: false,
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
    #[serde(default)]
    pub curve_mode: bool,
    pub stroke_width: f32,
    pub stroke_color_index: u8,
    pub fill_color_index: u8,
    pub position: Vec2,
    pub rotation: f32,
    pub scale: Vec2,
    pub origin: Vec2,
}

impl StrokeElement {
    pub fn new(vertices: Vec<PathVertex>, stroke_width: f32, stroke_color_index: u8, curve_mode: bool) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: None,
            vertices,
            closed: false,
            curve_mode,
            stroke_width,
            stroke_color_index,
            fill_color_index: 0,
            position: Vec2::ZERO,
            rotation: 0.0,
            scale: Vec2::ONE,
            origin: Vec2::ZERO,
        }
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerGroup {
    pub id: String,
    pub name: String,
    pub collapsed: bool,
    pub visible: bool,
    pub locked: bool,
}

impl LayerGroup {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            collapsed: false,
            visible: true,
            locked: false,
        }
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
}

impl Layer {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            visible: true,
            locked: false,
            elements: Vec::new(),
            group_id: None,
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
    #[serde(default)]
    pub layer_groups: Vec<LayerGroup>,
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
            layer_groups: Vec::new(),
        }
    }

    /// Find layer index by ID. Returns None if not found.
    pub fn layer_idx_by_id(&self, id: &str) -> Option<usize> {
        self.layers.iter().position(|l| l.id == id)
    }

    /// Get indices of layers belonging to a group, in order.
    pub fn layers_in_group(&self, group_id: &str) -> Vec<usize> {
        self.layers.iter().enumerate()
            .filter(|(_, l)| l.group_id.as_deref() == Some(group_id))
            .map(|(i, _)| i)
            .collect()
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
            false,
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
            false,
        ));
        assert_eq!(sprite.element_count(), 1);
        assert_eq!(sprite.vertex_count(), 2);
    }

    #[test]
    fn test_layer_group_serde_round_trip() {
        let mut sprite = Sprite::new("Test", 256, 256);
        let group = LayerGroup::new("Body Parts");
        let group_id = group.id.clone();
        sprite.layer_groups.push(group);
        sprite.layers[0].group_id = Some(group_id.clone());

        let json = serde_json::to_string_pretty(&sprite).unwrap();
        let sprite2: Sprite = serde_json::from_str(&json).unwrap();

        assert_eq!(sprite2.layer_groups.len(), 1);
        assert_eq!(sprite2.layer_groups[0].name, "Body Parts");
        assert!(sprite2.layer_groups[0].visible);
        assert!(!sprite2.layer_groups[0].locked);
        assert!(!sprite2.layer_groups[0].collapsed);
        assert_eq!(sprite2.layers[0].group_id.as_deref(), Some(group_id.as_str()));
    }

    #[test]
    fn test_backward_compat_no_groups() {
        // JSON from before groups were added (no groupId, no layerGroups)
        let json = r#"{
            "id": "test-id",
            "name": "OldSprite",
            "formatVersion": 1,
            "canvasWidth": 128,
            "canvasHeight": 128,
            "backgroundColorIndex": 0,
            "layers": [{
                "id": "layer-id",
                "name": "Layer 1",
                "visible": true,
                "locked": false,
                "elements": []
            }]
        }"#;
        let sprite: Sprite = serde_json::from_str(json).unwrap();
        assert_eq!(sprite.layer_groups.len(), 0);
        assert!(sprite.layers[0].group_id.is_none());
    }

    #[test]
    fn test_layers_in_group() {
        let mut sprite = Sprite::new("Test", 256, 256);
        let group = LayerGroup::new("Group");
        let gid = group.id.clone();
        sprite.layer_groups.push(group);

        sprite.layers[0].group_id = Some(gid.clone());
        let mut l2 = Layer::new("Layer 2");
        l2.group_id = Some(gid.clone());
        sprite.layers.push(l2);
        sprite.layers.push(Layer::new("Layer 3")); // ungrouped

        assert_eq!(sprite.layers_in_group(&gid), vec![0, 1]);
    }

    #[test]
    fn test_layer_idx_by_id() {
        let sprite = Sprite::new("Test", 256, 256);
        let id = sprite.layers[0].id.clone();
        assert_eq!(sprite.layer_idx_by_id(&id), Some(0));
        assert_eq!(sprite.layer_idx_by_id("nonexistent"), None);
    }
}
