use serde::{Deserialize, Serialize};

use super::vec2::Vec2;

fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GradientType {
    Linear,
    Radial,
}

/// Preset gradient direction aligned to grid axes.
/// The isometric grid uses a 2:1 diamond lattice (slopes +/-0.5),
/// so iso angles are atan(0.5) ~ 26.57 deg and pi - atan(0.5) ~ 153.43 deg.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GradientAlignment {
    Horizontal,
    Vertical,
    IsoDescending,
    IsoAscending,
}

impl GradientAlignment {
    pub fn to_radians(self) -> f32 {
        match self {
            Self::Horizontal => 0.0,
            Self::Vertical => std::f32::consts::FRAC_PI_2,
            Self::IsoDescending => 0.5_f32.atan(),                          // ~26.57°
            Self::IsoAscending => 0.5_f32.atan() + std::f32::consts::FRAC_PI_2, // ~116.57° (90° from descending)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GradientFill {
    pub gradient_type: GradientType,
    pub color_index_start: u8,
    pub color_index_end: u8,
    pub alignment: GradientAlignment,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub center: Option<Vec2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius: Option<f32>,
    /// Sharpness of the gradient transition. 1.0 = linear (default),
    /// <1.0 = softer start, >1.0 = sharper/more abrupt transition.
    #[serde(default = "default_sharpness")]
    pub sharpness: f32,
}

fn default_sharpness() -> f32 { 1.0 }

/// Bezier control points for a hatch flow curve.
/// Stored as a sequence of Vec2 positions forming cubic bezier segments
/// (groups of 4: anchor, cp1, cp2, anchor).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowCurve {
    pub control_points: Vec<Vec2>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gradient_fill: Option<GradientFill>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hatch_fill_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hatch_flow_curve: Option<FlowCurve>,
    /// Mask regions where hatch lines are suppressed.
    /// Each mask is a polygon (list of Vec2 points) in local element coordinates.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hatch_masks: Vec<Vec<Vec2>>,
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
            gradient_fill: None,
            hatch_fill_id: None,
            hatch_flow_curve: None,
            hatch_masks: Vec::new(),
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

/// A reference image overlay (not exported, editing aid only).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceImage {
    pub id: String,
    /// Path relative to the sprite file directory.
    pub path: String,
    pub position: Vec2,
    #[serde(default = "default_ref_opacity")]
    pub opacity: f32,
    #[serde(default)]
    pub locked: bool,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default = "default_one")]
    pub scale: f32,
    /// Loaded image dimensions (width, height) in pixels. Not serialized.
    #[serde(skip)]
    pub image_size: Option<(u32, u32)>,
}

fn default_ref_opacity() -> f32 { 0.3 }
fn default_true() -> bool { true }
fn default_one() -> f32 { 1.0 }

impl ReferenceImage {
    pub fn new(path: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            path,
            position: Vec2::ZERO,
            opacity: 0.3,
            locked: false,
            visible: true,
            scale: 1.0,
            image_size: None,
        }
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
    #[serde(default)]
    pub reference_images: Vec<ReferenceImage>,
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
            reference_images: Vec::new(),
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

    #[test]
    fn test_backward_compat_no_fill_fields() {
        // JSON from before gradient/hatch fills were added
        let json = r#"{
            "id": "elem-id",
            "vertices": [],
            "closed": true,
            "curveMode": false,
            "strokeWidth": 2.0,
            "strokeColorIndex": 1,
            "fillColorIndex": 5,
            "position": {"x": 0, "y": 0},
            "rotation": 0,
            "scale": {"x": 1, "y": 1},
            "origin": {"x": 0, "y": 0}
        }"#;
        let elem: StrokeElement = serde_json::from_str(json).unwrap();
        assert!(elem.gradient_fill.is_none());
        assert!(elem.hatch_fill_id.is_none());
        assert!(elem.hatch_flow_curve.is_none());
        assert_eq!(elem.fill_color_index, 5);
    }

    #[test]
    fn test_gradient_fill_serde_round_trip() {
        let mut elem = StrokeElement::new(
            vec![PathVertex::new(Vec2::ZERO)],
            2.0, 1, false,
        );
        elem.gradient_fill = Some(GradientFill {
            gradient_type: GradientType::Linear,
            color_index_start: 3,
            color_index_end: 7,
            alignment: GradientAlignment::IsoDescending,
            center: None,
            radius: None,
            sharpness: 1.0,
        });

        let json = serde_json::to_string(&elem).unwrap();
        let elem2: StrokeElement = serde_json::from_str(&json).unwrap();
        let grad = elem2.gradient_fill.unwrap();
        assert_eq!(grad.gradient_type, GradientType::Linear);
        assert_eq!(grad.color_index_start, 3);
        assert_eq!(grad.color_index_end, 7);
        assert_eq!(grad.alignment, GradientAlignment::IsoDescending);
    }

    #[test]
    fn test_gradient_alignment_angles() {
        assert!((GradientAlignment::Horizontal.to_radians() - 0.0).abs() < 1e-6);
        assert!((GradientAlignment::Vertical.to_radians() - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
        // IsoDescending: atan(0.5) ≈ 0.4636 rad
        assert!((GradientAlignment::IsoDescending.to_radians() - 0.4636).abs() < 0.001);
        // IsoAscending: atan(0.5) + pi/2 ≈ 2.0344 rad ≈ 116.57°
        let expected = 0.5_f32.atan() + std::f32::consts::FRAC_PI_2;
        assert!((GradientAlignment::IsoAscending.to_radians() - expected).abs() < 0.001);
        // Verify 90° apart
        let diff = GradientAlignment::IsoAscending.to_radians() - GradientAlignment::IsoDescending.to_radians();
        assert!((diff - std::f32::consts::FRAC_PI_2).abs() < 0.001);
    }
}
