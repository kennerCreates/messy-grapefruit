use serde::{Deserialize, Serialize};

use super::vec2::Vec2;

// === Constraint & Dynamics Data Model (Phase 7) ===

/// Waveform type for procedural modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Waveform {
    Sine,
    Noise,
}

impl Waveform {
    pub fn export_name(&self) -> &'static str {
        match self {
            Waveform::Sine => "sine",
            Waveform::Noise => "noise",
        }
    }
}

/// Blend mode for procedural modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BlendMode {
    Additive,
    Multiplicative,
}

impl BlendMode {
    pub fn export_name(&self) -> &'static str {
        match self {
            BlendMode::Additive => "additive",
            BlendMode::Multiplicative => "multiplicative",
        }
    }
}

/// A procedural modifier that applies oscillation to an animatable property.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProceduralModifier {
    /// Which property to oscillate (e.g., "position.x", "position.y", "rotation", "scale.x", "scale.y")
    pub property: String,
    pub waveform: Waveform,
    pub amplitude: f32,
    /// Frequency in Hz
    pub frequency: f32,
    /// Phase offset in degrees
    pub phase: f32,
    pub blend: BlendMode,
}

impl Default for ProceduralModifier {
    fn default() -> Self {
        Self {
            property: "position.y".to_string(),
            waveform: Waveform::Sine,
            amplitude: 5.0,
            frequency: 1.0,
            phase: 0.0,
            blend: BlendMode::Additive,
        }
    }
}

/// Gravity force for physics constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GravityForce {
    /// Angle in degrees (270 = down in screen space)
    pub angle: f32,
    pub strength: f32,
}

impl Default for GravityForce {
    fn default() -> Self {
        Self {
            angle: 270.0,
            strength: 0.0,
        }
    }
}

/// Wind force for physics constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindForce {
    pub strength: f32,
    /// Frequency of wind oscillation in Hz
    pub frequency: f32,
}

impl Default for WindForce {
    fn default() -> Self {
        Self {
            strength: 0.0,
            frequency: 0.5,
        }
    }
}

/// Spring/jiggle physics constraint for a layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhysicsConstraint {
    /// Spring frequency in Hz (0.1-10, default 2)
    pub frequency: f32,
    /// Damping ratio (0-2, default 0.5)
    pub damping: f32,
    /// Mix (0-1)
    pub mix: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gravity: Option<GravityForce>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wind: Option<WindForce>,
}

impl Default for PhysicsConstraint {
    fn default() -> Self {
        Self {
            frequency: 2.0,
            damping: 0.5,
            mix: 1.0,
            gravity: None,
            wind: None,
        }
    }
}

/// Spring smoothing parameters for look-at constraint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpringSmoothing {
    pub frequency: f32,
    pub damping: f32,
}

impl Default for SpringSmoothing {
    fn default() -> Self {
        Self {
            frequency: 4.0,
            damping: 0.7,
        }
    }
}

/// Look-at constraint: layer rotates to face a target.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LookAtConstraint {
    pub target_element_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_vertex_id: Option<String>,
    /// Default facing direction in radians
    pub rest_angle: f32,
    /// Minimum angle limit in radians (relative to rest)
    pub min_angle: f32,
    /// Maximum angle limit in radians (relative to rest)
    pub max_angle: f32,
    /// Mix (0-1)
    pub mix: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smooth: Option<SpringSmoothing>,
}

impl Default for LookAtConstraint {
    fn default() -> Self {
        Self {
            target_element_id: String::new(),
            target_vertex_id: None,
            rest_angle: 0.0,
            min_angle: -std::f32::consts::PI,
            max_angle: std::f32::consts::PI,
            mix: 1.0,
            smooth: None,
        }
    }
}

/// Per-layer constraints and dynamics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerConstraints {
    /// When true, scale_x = 1/scale_y (volume preservation)
    #[serde(default)]
    pub volume_preserve: bool,
    /// Look-at constraint: aim at target element/vertex
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub look_at: Option<LookAtConstraint>,
    /// Spring/jiggle physics
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub physics: Option<PhysicsConstraint>,
    /// Procedural modifiers (sine/noise oscillation)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub procedural: Vec<ProceduralModifier>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sprite {
    pub id: String,
    pub name: String,
    pub format_version: u32,
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub background_color_index: usize,
    pub layers: Vec<Layer>,
    pub skins: Vec<Skin>,
    #[serde(default)]
    pub animations: Vec<AnimationSequence>,
}

impl Sprite {
    pub fn new(name: &str, width: u32, height: u32) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            format_version: 1,
            canvas_width: width,
            canvas_height: height,
            background_color_index: 0,
            layers: vec![Layer::new("Layer 1")],
            skins: Vec::new(),
            animations: Vec::new(),
        }
    }
}

/// Socket attachment: a layer can be parented to a vertex on another layer's element.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerSocket {
    pub parent_element_id: String,
    pub parent_vertex_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Layer {
    pub id: String,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub elements: Vec<StrokeElement>,
    /// IK target elements on this layer (lightweight position-only elements).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ik_targets: Vec<IKTargetElement>,
    /// If set, this layer is socketed to a vertex on another layer's element.
    /// The layer inherits position and rotation from the parent vertex.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub socket: Option<LayerSocket>,
    /// Constraints and dynamics for this layer (physics, look-at, procedural, volume preserve).
    #[serde(default, skip_serializing_if = "is_default_constraints")]
    pub constraints: LayerConstraints,
}

fn is_default_constraints(c: &LayerConstraints) -> bool {
    !c.volume_preserve
        && c.look_at.is_none()
        && c.physics.is_none()
        && c.procedural.is_empty()
}

impl Layer {
    pub fn new(name: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            visible: true,
            locked: false,
            elements: Vec::new(),
            ik_targets: Vec::new(),
            socket: None,
            constraints: LayerConstraints::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
}

impl StrokeElement {
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: None,
            vertices: Vec::new(),
            closed: false,
            stroke_width: 2.0,
            stroke_color_index: 1,
            fill_color_index: 0,
            position: Vec2::ZERO,
            rotation: 0.0,
            scale: Vec2::ONE,
            origin: Vec2::ZERO,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathVertex {
    pub id: String,
    pub pos: Vec2,
    pub cp1: Option<Vec2>,
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
pub struct Skin {
    pub id: String,
    pub name: String,
    pub overrides: Vec<SkinOverride>,
}

impl Skin {
    pub fn new(name: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            overrides: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinOverride {
    pub element_id: String,
    pub stroke_color_index: Option<usize>,
    pub fill_color_index: Option<usize>,
    pub stroke_width: Option<f32>,
}

// === IK Data Model ===

/// Solver type for an IK chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SolverType {
    TwoBone,
    Fabrik,
}

impl SolverType {
    pub fn export_name(&self) -> &'static str {
        match self {
            SolverType::TwoBone => "two-bone",
            SolverType::Fabrik => "fabrik",
        }
    }
}

/// Per-joint angle constraint for IK chains.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AngleConstraint {
    pub layer_id: String,
    /// Minimum angle in radians relative to parent bone.
    pub min: f32,
    /// Maximum angle in radians relative to parent bone.
    pub max: f32,
}

/// An IK chain definition, stored on AnimationSequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IKChain {
    pub id: String,
    pub name: String,
    /// Ordered root-to-tip layer IDs forming the socket chain.
    pub layer_ids: Vec<String>,
    /// References an IKTargetElement on the tip layer.
    pub target_element_id: String,
    /// FK/IK mix: 0 = pure FK, 1 = pure IK. Keyframeable via pose keyframes.
    pub mix: f32,
    /// Bend direction sign for 2-bone solver: +1 or -1.
    pub bend_direction: i8,
    /// Solver type: two-bone analytical or FABRIK iterative.
    pub solver: SolverType,
    /// Per-joint angle constraints (2-bone only initially).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub angle_constraints: Vec<AngleConstraint>,
}

impl IKChain {
    pub fn new(name: &str, layer_ids: Vec<String>, target_element_id: &str) -> Self {
        let solver = if layer_ids.len() <= 2 {
            SolverType::TwoBone
        } else {
            SolverType::Fabrik
        };
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            layer_ids,
            target_element_id: target_element_id.to_string(),
            mix: 1.0,
            bend_direction: 1,
            solver,
            angle_constraints: Vec::new(),
        }
    }
}

/// A lightweight IK target element. Position is world-space.
/// Lives on a layer alongside stroke elements for organization.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IKTargetElement {
    pub id: String,
    pub name: Option<String>,
    pub position: Vec2,
    pub ik_chain_id: String,
}

impl IKTargetElement {
    pub fn new(position: Vec2, ik_chain_id: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: None,
            position,
            ik_chain_id: ik_chain_id.to_string(),
        }
    }
}

/// Enum for elements that can live on a layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
#[allow(dead_code)]
pub enum Element {
    #[serde(rename = "stroke")]
    Stroke(StrokeElement),
    #[serde(rename = "ik-target")]
    IkTarget(IKTargetElement),
}

// === Animation Data Model ===

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationSequence {
    pub id: String,
    pub name: String,
    /// Duration in seconds
    pub duration: f32,
    pub looping: bool,
    /// Pose-based keyframes: each captures the full sprite state at a point in time.
    #[serde(default)]
    pub pose_keyframes: Vec<PoseKeyframe>,
    /// IK chain definitions for this animation.
    #[serde(default)]
    pub ik_chains: Vec<IKChain>,
}

impl AnimationSequence {
    pub fn new(name: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            duration: 1.0,
            looping: true,
            pose_keyframes: Vec::new(),
            ik_chains: Vec::new(),
        }
    }

    /// Auto-extend duration if a pose keyframe is placed past the current end.
    pub fn auto_extend_duration_poses(&mut self) {
        for pk in &self.pose_keyframes {
            if pk.time > self.duration {
                self.duration = pk.time;
            }
        }
    }

}

/// A pose-based keyframe: captures the full state of all elements at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PoseKeyframe {
    pub id: String,
    /// Time in seconds
    pub time: f32,
    /// Easing curve for the transition TO this pose from the previous one
    pub easing: EasingCurve,
    /// State of each element at this pose
    pub element_poses: Vec<ElementPose>,
    /// Per-IK-chain mix values at this pose (chain_id, mix 0..1)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ik_mix_values: Vec<(String, f32)>,
}

/// The state of a single element within a pose.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementPose {
    pub element_id: String,
    pub layer_id: String,
    pub position: Vec2,
    pub rotation: f32,
    pub scale: Vec2,
    pub visible: bool,
    pub stroke_color_index: usize,
    pub fill_color_index: usize,
    /// Vertex positions captured by stable vertex ID.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vertex_positions: Vec<(String, Vec2)>,
}

/// Snapshot the current sprite state into a PoseKeyframe.
pub fn snapshot_sprite_to_pose(
    sprite: &Sprite,
    time: f32,
    easing: EasingCurve,
    ik_chains: &[IKChain],
) -> PoseKeyframe {
    let mut element_poses = Vec::new();

    for layer in &sprite.layers {
        for stroke in &layer.elements {
            let vertex_positions: Vec<(String, Vec2)> = stroke
                .vertices
                .iter()
                .map(|v| (v.id.clone(), v.pos))
                .collect();

            element_poses.push(ElementPose {
                element_id: stroke.id.clone(),
                layer_id: layer.id.clone(),
                position: stroke.position,
                rotation: stroke.rotation,
                scale: stroke.scale,
                visible: true,
                stroke_color_index: stroke.stroke_color_index,
                fill_color_index: stroke.fill_color_index,
                vertex_positions,
            });
        }

        for target in &layer.ik_targets {
            element_poses.push(ElementPose {
                element_id: target.id.clone(),
                layer_id: layer.id.clone(),
                position: target.position,
                rotation: 0.0,
                scale: Vec2::new(1.0, 1.0),
                visible: true,
                stroke_color_index: 0,
                fill_color_index: 0,
                vertex_positions: Vec::new(),
            });
        }
    }

    // Collect IK mix values from chain defaults
    let ik_mix_values: Vec<(String, f32)> = ik_chains
        .iter()
        .map(|chain| (chain.id.clone(), chain.mix))
        .collect();

    PoseKeyframe {
        id: uuid::Uuid::new_v4().to_string(),
        time,
        easing,
        element_poses,
        ik_mix_values,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EasingCurve {
    pub preset: EasingPreset,
    /// Cubic bezier control points [x1, y1, x2, y2] for custom easing.
    /// Only used when preset is Custom.
    pub control_points: [f32; 4],
}

impl Default for EasingCurve {
    fn default() -> Self {
        Self {
            preset: EasingPreset::Linear,
            control_points: [0.0, 0.0, 1.0, 1.0],
        }
    }
}

impl EasingCurve {
    pub fn linear() -> Self {
        Self {
            preset: EasingPreset::Linear,
            control_points: [0.0, 0.0, 1.0, 1.0],
        }
    }

    pub fn ease_in() -> Self {
        Self {
            preset: EasingPreset::EaseIn,
            control_points: [0.42, 0.0, 1.0, 1.0],
        }
    }

    pub fn ease_out() -> Self {
        Self {
            preset: EasingPreset::EaseOut,
            control_points: [0.0, 0.0, 0.58, 1.0],
        }
    }

    pub fn ease_in_out() -> Self {
        Self {
            preset: EasingPreset::EaseInOut,
            control_points: [0.42, 0.0, 0.58, 1.0],
        }
    }

    pub fn bounce() -> Self {
        Self {
            preset: EasingPreset::Bounce,
            control_points: [0.0, 0.0, 1.0, 1.0],
        }
    }

    pub fn elastic() -> Self {
        Self {
            preset: EasingPreset::Elastic,
            control_points: [0.0, 0.0, 1.0, 1.0],
        }
    }

    pub fn step() -> Self {
        Self {
            preset: EasingPreset::Step,
            control_points: [0.0, 0.0, 1.0, 1.0],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EasingPreset {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    Bounce,
    Elastic,
    Step,
    Custom,
}

impl EasingPreset {
    pub fn export_name(&self) -> &'static str {
        match self {
            EasingPreset::Linear => "linear",
            EasingPreset::EaseIn => "ease-in",
            EasingPreset::EaseOut => "ease-out",
            EasingPreset::EaseInOut => "ease-in-out",
            EasingPreset::Bounce => "bounce",
            EasingPreset::Elastic => "elastic",
            EasingPreset::Step => "step",
            EasingPreset::Custom => "custom",
        }
    }
}

