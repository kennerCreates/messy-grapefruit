use serde::{Deserialize, Serialize};

use super::project::Vec2;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PathVertex {
    pub id: String,
    pub pos: Vec2,
    pub cp1: Option<Vec2>,
    pub cp2: Option<Vec2>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Element {
    #[serde(rename_all = "camelCase")]
    StrokeElement {
        id: String,
        name: Option<String>,
        vertices: Vec<PathVertex>,
        closed: bool,
        stroke_width: f64,
        stroke_color_index: usize,
        fill_color_index: usize,
        position: Vec2,
        rotation: f64,
        scale: Vec2,
        origin: Vec2,
        visible: bool,
    },
    #[serde(rename_all = "camelCase")]
    IKTargetElement {
        id: String,
        name: Option<String>,
        position: Vec2,
        ik_chain_id: String,
        visible: bool,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SpringParams {
    pub frequency: f64,
    pub damping: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LookAtConstraint {
    pub target_element_id: String,
    pub target_vertex_id: Option<String>,
    pub rest_angle: f64,
    pub min_angle: f64,
    pub max_angle: f64,
    pub mix: f64,
    pub smooth: Option<SpringParams>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GravityParams {
    pub angle: f64,
    pub strength: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WindParams {
    pub strength: f64,
    pub frequency: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PhysicsConstraint {
    pub frequency: f64,
    pub damping: f64,
    pub mix: f64,
    pub gravity: Option<GravityParams>,
    pub wind: Option<WindParams>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Waveform {
    Sine,
    Noise,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum BlendMode {
    Additive,
    Multiplicative,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProceduralModifier {
    pub property: String,
    pub waveform: Waveform,
    pub amplitude: f64,
    pub frequency: f64,
    pub phase: f64,
    pub blend: BlendMode,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LayerConstraints {
    pub volume_preserve: Option<bool>,
    pub look_at: Option<LookAtConstraint>,
    pub physics: Option<PhysicsConstraint>,
    pub procedural: Option<Vec<ProceduralModifier>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Socket {
    pub parent_element_id: String,
    pub parent_vertex_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Layer {
    pub id: String,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub elements: Vec<Element>,
    pub socket: Option<Socket>,
    pub constraints: Option<LayerConstraints>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SkinOverride {
    pub element_id: String,
    pub stroke_color_index: Option<usize>,
    pub fill_color_index: Option<usize>,
    pub stroke_width: Option<f64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Skin {
    pub id: String,
    pub name: String,
    pub overrides: Vec<SkinOverride>,
}

use super::animation::AnimationSequence;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Sprite {
    pub id: String,
    pub name: String,
    pub format_version: String,
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub background_color_index: usize,
    pub layers: Vec<Layer>,
    pub skins: Vec<Skin>,
    pub animations: Vec<AnimationSequence>,
}
