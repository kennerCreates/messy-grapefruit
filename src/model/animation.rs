use serde::{Deserialize, Serialize};

use super::vec2::Vec2;

/// Easing curve for pose-to-pose interpolation.
/// Uses CSS-style cubic bezier with four control scalars (x1,y1,x2,y2).
/// The curve always passes through (0,0) and (1,1).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EasingCurve {
    /// Named preset: "linear", "ease-in", "ease-out", "ease-in-out".
    pub preset: String,
    /// Cubic bezier control points [x1, y1, x2, y2].
    pub control_points: [f32; 4],
}

impl EasingCurve {
    pub fn from_preset(preset: &str) -> Self {
        let cp = match preset {
            "ease-in"     => [0.42, 0.0, 1.0,  1.0 ],
            "ease-out"    => [0.0,  0.0, 0.58, 1.0 ],
            "ease-in-out" => [0.42, 0.0, 0.58, 1.0 ],
            _             => [0.0,  0.0, 1.0,  1.0 ], // linear
        };
        Self { preset: preset.to_string(), control_points: cp }
    }

    pub fn linear() -> Self {
        Self::from_preset("linear")
    }
}

impl Default for EasingCurve {
    fn default() -> Self {
        Self::from_preset("ease-in-out")
    }
}

/// Per-vertex position snapshot (stable vertex ID + world position).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VertexPoseEntry {
    pub vertex_id: String,
    pub pos: Vec2,
}

/// Snapshot of one element's animatable properties at a keyframe time.
/// Only present in a keyframe if the element was explicitly keyed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementPose {
    pub element_id: String,
    pub layer_id: String,
    pub position: Vec2,
    pub rotation: f32,
    pub scale: Vec2,
    pub visible: bool,
    pub stroke_color_index: u8,
    pub fill_color_index: u8,
    /// Per-vertex positions keyed by stable vertex ID.
    #[serde(default)]
    pub vertex_positions: Vec<VertexPoseEntry>,
}

/// A pose keyframe: sparse snapshot of explicitly keyed elements at a point in time.
///
/// Sparse: `element_poses` only contains elements that were keyed at this time.
/// Elements not present are evaluated via per-element search (see engine::animation).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PoseKeyframe {
    pub id: String,
    pub time_secs: f32,
    /// Easing curve controlling the transition TO this pose from the previous one.
    pub easing: EasingCurve,
    /// Sparse: only elements explicitly keyed at this time.
    #[serde(default)]
    pub element_poses: Vec<ElementPose>,
}

/// Named event marker at a specific time (for game events in Bevy).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventMarker {
    pub id: String,
    pub time_secs: f32,
    pub name: String,
}

/// An animation sequence containing sparse pose keyframes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationSequence {
    pub id: String,
    pub name: String,
    pub duration_secs: f32,
    pub looping: bool,
    /// Sorted by time_secs ascending. Maintained sorted on insert.
    #[serde(default)]
    pub pose_keyframes: Vec<PoseKeyframe>,
    #[serde(default)]
    pub event_markers: Vec<EventMarker>,
}

impl AnimationSequence {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            duration_secs: 2.0,
            looping: true,
            pose_keyframes: Vec::new(),
            event_markers: Vec::new(),
        }
    }
}
