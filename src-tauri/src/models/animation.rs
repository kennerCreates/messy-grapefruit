use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EasingCurve {
    pub preset: String,
    pub control_points: [f64; 4],
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Keyframe {
    pub id: String,
    pub time: f64,
    pub value: f64,
    pub easing: EasingCurve,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PropertyTrack {
    pub property: String,
    pub element_id: String,
    pub layer_id: String,
    pub keyframes: Vec<Keyframe>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AngleConstraint {
    pub layer_id: String,
    pub min: f64,
    pub max: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IKChain {
    pub id: String,
    pub name: String,
    pub layer_ids: Vec<String>,
    pub target_element_id: String,
    pub mix: f64,
    pub bend_direction: i8,
    pub solver: String,
    pub angle_constraints: Option<Vec<AngleConstraint>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AnimationSequence {
    pub id: String,
    pub name: String,
    pub duration: f64,
    pub looping: bool,
    pub tracks: Vec<PropertyTrack>,
    pub ik_chains: Vec<IKChain>,
}
