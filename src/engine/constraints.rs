use crate::engine::physics::wrap_angle;
use crate::model::Vec2;
use crate::model::sprite::{BlendMode, ProceduralModifier, Waveform};

/// Solve look-at constraint: compute the rotation angle to aim at a target.
///
/// - `origin_pos`: world-space position of the layer/element pivot
/// - `target_pos`: world-space position the layer should aim at
/// - `current_rotation`: current rotation of the layer (radians)
/// - `rest_angle`: the default facing direction (radians)
/// - `min_angle`: minimum rotation limit relative to rest (radians)
/// - `max_angle`: maximum rotation limit relative to rest (radians)
/// - `mix`: 0-1 blend factor (0 = no effect, 1 = full look-at)
///
/// Returns the new rotation angle (radians).
pub fn look_at_solve(
    origin_pos: Vec2,
    target_pos: Vec2,
    current_rotation: f32,
    rest_angle: f32,
    min_angle: f32,
    max_angle: f32,
    mix: f32,
) -> f32 {
    let diff = target_pos - origin_pos;
    let target_angle = diff.y.atan2(diff.x);

    // Compute angle relative to rest
    let relative = wrap_angle(target_angle - rest_angle);

    // Clamp to angle limits
    let clamped = relative.clamp(min_angle, max_angle);

    // Convert back to absolute angle
    let desired = rest_angle + clamped;

    // Blend with current rotation
    
    lerp_angle(current_rotation, desired, mix)
}

/// Linear interpolation between two angles, using shortest path.
fn lerp_angle(from: f32, to: f32, t: f32) -> f32 {
    let diff = wrap_angle(to - from);
    wrap_angle(from + diff * t)
}

/// Apply volume preservation: maintain original area (scale_x * scale_y = constant).
/// Uses scale_y as the driving value, adjusts scale_x to preserve the original area.
pub fn volume_preserve(scale_x: f32, scale_y: f32) -> (f32, f32) {
    let original_area = scale_x * scale_y;
    if scale_y.abs() < 1e-6 {
        // Avoid division by zero; keep scale as is
        (scale_x, scale_y)
    } else {
        (original_area / scale_y, scale_y)
    }
}

/// Evaluate a procedural modifier at a given time.
/// Returns the modifier value to apply (before blend mode consideration).
pub fn evaluate_procedural(modifier: &ProceduralModifier, time: f32) -> f32 {
    let phase_rad = modifier.phase.to_radians();
    let t = time * modifier.frequency * 2.0 * std::f32::consts::PI + phase_rad;

    let wave_value = match modifier.waveform {
        Waveform::Sine => t.sin(),
        Waveform::Noise => value_noise(time * modifier.frequency + modifier.phase / 360.0),
    };

    wave_value * modifier.amplitude
}

/// Apply a procedural modifier value to a base value according to blend mode.
pub fn apply_procedural_value(base: f32, modifier_value: f32, blend: BlendMode) -> f32 {
    match blend {
        BlendMode::Additive => base + modifier_value,
        BlendMode::Multiplicative => base * (1.0 + modifier_value),
    }
}

/// Simple value noise function. Returns values in [-1, 1].
/// Uses a hash-based approach for deterministic pseudo-random noise.
fn value_noise(t: f32) -> f32 {
    let i = t.floor() as i32;
    let f = t - t.floor(); // fractional part

    // Smoothstep interpolation factor
    let u = f * f * (3.0 - 2.0 * f);

    let n0 = hash_float(i);
    let n1 = hash_float(i + 1);

    // Interpolate
    n0 + (n1 - n0) * u
}

/// Hash an integer to a float in [-1, 1].
fn hash_float(n: i32) -> f32 {
    // Simple integer hash
    let n = (n as u32).wrapping_mul(1103515245).wrapping_add(12345);
    let n = (n >> 16) ^ n;
    let n = n.wrapping_mul(0x45d9f3b);
    let n = (n >> 16) ^ n;
    // Map to [-1, 1]
    (n as f32 / u32::MAX as f32) * 2.0 - 1.0
}

/// Match a property string to an animatable property for procedural modifiers.
/// Returns which element field to modify.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProceduralTarget {
    PositionX,
    PositionY,
    Rotation,
    ScaleX,
    ScaleY,
}

impl ProceduralTarget {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "position.x" => Some(ProceduralTarget::PositionX),
            "position.y" => Some(ProceduralTarget::PositionY),
            "rotation" => Some(ProceduralTarget::Rotation),
            "scale.x" => Some(ProceduralTarget::ScaleX),
            "scale.y" => Some(ProceduralTarget::ScaleY),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn all() -> &'static [(&'static str, ProceduralTarget)] {
        &[
            ("position.x", ProceduralTarget::PositionX),
            ("position.y", ProceduralTarget::PositionY),
            ("rotation", ProceduralTarget::Rotation),
            ("scale.x", ProceduralTarget::ScaleX),
            ("scale.y", ProceduralTarget::ScaleY),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_look_at_solve_direct() {
        // Origin at (0,0), target at (100,0) -> angle should be 0
        let angle = look_at_solve(
            Vec2::ZERO,
            Vec2::new(100.0, 0.0),
            0.0,
            0.0,
            -std::f32::consts::PI,
            std::f32::consts::PI,
            1.0,
        );
        assert!((angle - 0.0).abs() < 0.01, "angle={}", angle);
    }

    #[test]
    fn test_look_at_solve_up() {
        // Target directly below (positive Y = down in screen space)
        let angle = look_at_solve(
            Vec2::ZERO,
            Vec2::new(0.0, 100.0),
            0.0,
            0.0,
            -std::f32::consts::PI,
            std::f32::consts::PI,
            1.0,
        );
        // atan2(100, 0) = PI/2
        assert!(
            (angle - std::f32::consts::FRAC_PI_2).abs() < 0.01,
            "angle={}",
            angle
        );
    }

    #[test]
    fn test_look_at_angle_limits() {
        // Target is behind (angle PI), but limits are [-PI/4, PI/4]
        let angle = look_at_solve(
            Vec2::ZERO,
            Vec2::new(-100.0, 0.0),
            0.0,
            0.0,
            -std::f32::consts::FRAC_PI_4,
            std::f32::consts::FRAC_PI_4,
            1.0,
        );
        // Should be clamped to max limit
        // atan2(0, -100) = PI, relative to rest 0 = PI, clamped to PI/4
        // But PI wraps, so the relative angle is PI, clamped to PI/4
        assert!(
            angle.abs() <= std::f32::consts::FRAC_PI_4 + 0.01,
            "angle={}, expected within PI/4",
            angle
        );
    }

    #[test]
    fn test_look_at_mix() {
        let current = 0.0;
        let angle = look_at_solve(
            Vec2::ZERO,
            Vec2::new(0.0, 100.0),
            current,
            0.0,
            -std::f32::consts::PI,
            std::f32::consts::PI,
            0.5,
        );
        // With mix 0.5, should be halfway between current (0) and target (PI/2)
        let expected = std::f32::consts::FRAC_PI_2 * 0.5;
        assert!((angle - expected).abs() < 0.1, "angle={}, expected={}", angle, expected);
    }

    #[test]
    fn test_volume_preserve() {
        // Original area = 1.0 * 2.0 = 2.0; preserving area: sx = 2.0/2.0 = 1.0
        let (sx, sy) = volume_preserve(1.0, 2.0);
        assert!((sx - 1.0).abs() < 0.001, "sx={}", sx);
        assert!((sy - 2.0).abs() < 0.001, "sy={}", sy);
        // Verify area preserved
        assert!((sx * sy - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_volume_preserve_identity() {
        let (sx, sy) = volume_preserve(1.0, 1.0);
        assert!((sx - 1.0).abs() < 0.001, "sx={}", sx);
        assert!((sy - 1.0).abs() < 0.001, "sy={}", sy);
    }

    #[test]
    fn test_procedural_sine() {
        let modifier = ProceduralModifier {
            property: "position.y".to_string(),
            waveform: Waveform::Sine,
            amplitude: 10.0,
            frequency: 1.0,
            phase: 0.0,
            blend: BlendMode::Additive,
        };

        // At t=0, sin(0) = 0
        let val = evaluate_procedural(&modifier, 0.0);
        assert!(val.abs() < 0.01, "val at t=0 = {}", val);

        // At t=0.25 (quarter period at 1Hz), sin(PI/2) = 1
        let val = evaluate_procedural(&modifier, 0.25);
        assert!((val - 10.0).abs() < 0.01, "val at t=0.25 = {}", val);
    }

    #[test]
    fn test_procedural_noise_bounded() {
        let modifier = ProceduralModifier {
            property: "rotation".to_string(),
            waveform: Waveform::Noise,
            amplitude: 5.0,
            frequency: 2.0,
            phase: 0.0,
            blend: BlendMode::Additive,
        };

        // Value noise should produce results within amplitude
        for i in 0..100 {
            let t = i as f32 * 0.01;
            let val = evaluate_procedural(&modifier, t);
            assert!(
                val.abs() <= 5.0 + 0.01,
                "noise value out of bounds: {} at t={}",
                val,
                t
            );
        }
    }

    #[test]
    fn test_apply_procedural_additive() {
        let result = apply_procedural_value(100.0, 5.0, BlendMode::Additive);
        assert!((result - 105.0).abs() < 0.001);
    }

    #[test]
    fn test_apply_procedural_multiplicative() {
        let result = apply_procedural_value(100.0, 0.1, BlendMode::Multiplicative);
        // 100 * (1 + 0.1) = 110
        assert!((result - 110.0).abs() < 0.001);
    }

    #[test]
    fn test_wrap_angle_values() {
        assert!((wrap_angle(0.0) - 0.0).abs() < 0.001);
        assert!((wrap_angle(std::f32::consts::PI) - std::f32::consts::PI).abs() < 0.001);
        assert!((wrap_angle(-std::f32::consts::PI) + std::f32::consts::PI).abs() < 0.001);
        assert!((wrap_angle(std::f32::consts::TAU) - 0.0).abs() < 0.001);
    }
}
