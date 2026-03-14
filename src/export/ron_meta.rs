//! Bevy-compatible RON metadata generation for bone animation export.

use serde::{Deserialize, Serialize};

/// Top-level bone animation data exported as RON for a single sprite.
/// The Bevy runtime reads this to assemble parts and evaluate animation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoneAnimationData {
    /// Sprite name
    pub name: String,
    /// Canvas dimensions (reference resolution)
    pub canvas_width: u32,
    pub canvas_height: u32,
    /// Per-element part definitions (texture regions, origins, sockets)
    pub parts: Vec<PartDefinition>,
    /// Animation sequences
    pub animations: Vec<AnimationExport>,
    /// IK chain definitions (shared across animations)
    pub ik_chains: Vec<IKChainExport>,
    /// Layer constraint/dynamics definitions
    pub layer_dynamics: Vec<LayerDynamicsExport>,
    /// Skin manifest: maps skin names to atlas file references
    pub skins: Vec<SkinManifestEntry>,
}

/// A single body part (element) in the texture atlas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartDefinition {
    /// Element ID for runtime reference
    pub element_id: String,
    /// Human-readable name
    pub name: String,
    /// Layer ID this element belongs to
    pub layer_id: String,
    /// Region in the texture atlas (x, y, width, height in pixels)
    pub atlas_region: AtlasRegion,
    /// Origin point (pivot) relative to the part's top-left corner
    pub origin: (f32, f32),
    /// Position in the sprite's coordinate system
    pub position: (f32, f32),
    /// Default rotation (rest pose)
    pub rotation: f32,
    /// Default scale (rest pose)
    pub scale: (f32, f32),
    /// Socket parent reference: (parent_element_id, parent_vertex_id) if socketed
    pub socket_parent: Option<(String, String)>,
    /// Z-order index (rendering order)
    pub z_order: usize,
}

/// Region within a texture atlas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Exported animation sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationExport {
    pub id: String,
    pub name: String,
    pub duration: f32,
    pub looping: bool,
    pub tracks: Vec<TrackExport>,
}

/// Exported property track with keyframes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackExport {
    pub property: String,
    pub element_id: String,
    pub layer_id: String,
    pub keyframes: Vec<KeyframeExport>,
}

/// Exported keyframe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyframeExport {
    pub time: f32,
    pub value: f64,
    pub easing: EasingExport,
}

/// Exported easing curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EasingExport {
    pub preset: String,
    pub control_points: [f32; 4],
}

/// Exported IK chain definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IKChainExport {
    pub id: String,
    pub name: String,
    pub layer_ids: Vec<String>,
    pub target_element_id: String,
    pub mix: f32,
    pub bend_direction: i8,
    pub solver: String,
    pub angle_constraints: Vec<AngleConstraintExport>,
}

/// Exported angle constraint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AngleConstraintExport {
    pub layer_id: String,
    pub min: f32,
    pub max: f32,
}

/// Exported layer dynamics (physics, constraints, procedural modifiers).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerDynamicsExport {
    pub layer_id: String,
    pub layer_name: String,
    pub volume_preserve: bool,
    pub look_at: Option<LookAtExport>,
    pub physics: Option<PhysicsExport>,
    pub procedural: Vec<ProceduralExport>,
}

/// Exported look-at constraint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookAtExport {
    pub target_element_id: String,
    pub target_vertex_id: Option<String>,
    pub rest_angle: f32,
    pub min_angle: f32,
    pub max_angle: f32,
    pub mix: f32,
    pub smooth: Option<SpringSmoothingExport>,
}

/// Exported spring smoothing params.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpringSmoothingExport {
    pub frequency: f32,
    pub damping: f32,
}

/// Exported physics constraint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsExport {
    pub frequency: f32,
    pub damping: f32,
    pub mix: f32,
    pub gravity: Option<GravityExport>,
    pub wind: Option<WindExport>,
}

/// Exported gravity force.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GravityExport {
    pub angle: f32,
    pub strength: f32,
}

/// Exported wind force.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindExport {
    pub strength: f32,
    pub frequency: f32,
}

/// Exported procedural modifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProceduralExport {
    pub property: String,
    pub waveform: String,
    pub amplitude: f32,
    pub frequency: f32,
    pub phase: f32,
    pub blend: String,
}

/// Skin manifest entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkinManifestEntry {
    pub name: String,
    pub atlas_file: String,
}

/// Convert the model types into the export RON format.
pub fn build_bone_animation_data(
    sprite: &crate::model::sprite::Sprite,
    parts: Vec<PartDefinition>,
    skin_entries: Vec<SkinManifestEntry>,
) -> BoneAnimationData {
    let animations = sprite
        .animations
        .iter()
        .map(|seq| AnimationExport {
            id: seq.id.clone(),
            name: seq.name.clone(),
            duration: seq.duration,
            looping: seq.looping,
            tracks: seq
                .tracks
                .iter()
                .map(|track| TrackExport {
                    property: format!("{:?}", track.property),
                    element_id: track.element_id.clone(),
                    layer_id: track.layer_id.clone(),
                    keyframes: track
                        .keyframes
                        .iter()
                        .map(|kf| KeyframeExport {
                            time: kf.time,
                            value: kf.value,
                            easing: EasingExport {
                                preset: format!("{:?}", kf.easing.preset),
                                control_points: kf.easing.control_points,
                            },
                        })
                        .collect(),
                })
                .collect(),
        })
        .collect();

    // Collect all IK chains from all animations (deduplicated by ID)
    let mut ik_chains_map = std::collections::HashMap::new();
    for seq in &sprite.animations {
        for chain in &seq.ik_chains {
            ik_chains_map.entry(chain.id.clone()).or_insert_with(|| {
                IKChainExport {
                    id: chain.id.clone(),
                    name: chain.name.clone(),
                    layer_ids: chain.layer_ids.clone(),
                    target_element_id: chain.target_element_id.clone(),
                    mix: chain.mix,
                    bend_direction: chain.bend_direction,
                    solver: format!("{:?}", chain.solver),
                    angle_constraints: chain
                        .angle_constraints
                        .iter()
                        .map(|ac| AngleConstraintExport {
                            layer_id: ac.layer_id.clone(),
                            min: ac.min,
                            max: ac.max,
                        })
                        .collect(),
                }
            });
        }
    }
    let ik_chains: Vec<IKChainExport> = ik_chains_map.into_values().collect();

    // Layer dynamics
    let layer_dynamics = sprite
        .layers
        .iter()
        .filter(|l| {
            l.constraints.volume_preserve
                || l.constraints.look_at.is_some()
                || l.constraints.physics.is_some()
                || !l.constraints.procedural.is_empty()
        })
        .map(|layer| LayerDynamicsExport {
            layer_id: layer.id.clone(),
            layer_name: layer.name.clone(),
            volume_preserve: layer.constraints.volume_preserve,
            look_at: layer.constraints.look_at.as_ref().map(|la| LookAtExport {
                target_element_id: la.target_element_id.clone(),
                target_vertex_id: la.target_vertex_id.clone(),
                rest_angle: la.rest_angle,
                min_angle: la.min_angle,
                max_angle: la.max_angle,
                mix: la.mix,
                smooth: la.smooth.as_ref().map(|s| SpringSmoothingExport {
                    frequency: s.frequency,
                    damping: s.damping,
                }),
            }),
            physics: layer.constraints.physics.as_ref().map(|p| PhysicsExport {
                frequency: p.frequency,
                damping: p.damping,
                mix: p.mix,
                gravity: p.gravity.as_ref().map(|g| GravityExport {
                    angle: g.angle,
                    strength: g.strength,
                }),
                wind: p.wind.as_ref().map(|w| WindExport {
                    strength: w.strength,
                    frequency: w.frequency,
                }),
            }),
            procedural: layer
                .constraints
                .procedural
                .iter()
                .map(|pm| ProceduralExport {
                    property: pm.property.clone(),
                    waveform: format!("{:?}", pm.waveform),
                    amplitude: pm.amplitude,
                    frequency: pm.frequency,
                    phase: pm.phase,
                    blend: format!("{:?}", pm.blend),
                })
                .collect(),
        })
        .collect();

    BoneAnimationData {
        name: sprite.name.clone(),
        canvas_width: sprite.canvas_width,
        canvas_height: sprite.canvas_height,
        parts,
        animations,
        ik_chains,
        layer_dynamics,
        skins: skin_entries,
    }
}

/// Serialize bone animation data to a RON string.
pub fn to_ron_string(data: &BoneAnimationData) -> Result<String, crate::export::ExportError> {
    let config = ron::ser::PrettyConfig::new()
        .depth_limit(10)
        .struct_names(true)
        .enumerate_arrays(false);

    ron::ser::to_string_pretty(data, config)
        .map_err(|e| crate::export::ExportError::Ron(format!("Failed to serialize RON: {}", e)))
}
