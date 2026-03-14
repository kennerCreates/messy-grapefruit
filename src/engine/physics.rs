use crate::model::Vec2;
use crate::model::sprite::PhysicsConstraint;
use std::collections::HashMap;

/// State for a single spring simulation (position + velocity in world space).
#[derive(Debug, Clone, Copy)]
pub struct SpringState {
    pub position: Vec2,
    pub velocity: Vec2,
}

impl Default for SpringState {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
        }
    }
}

/// Collection of spring states keyed by layer ID.
#[derive(Debug, Clone, Default)]
pub struct PhysicsState {
    pub springs: HashMap<String, SpringState>,
    /// Spring state for look-at angular smoothing, keyed by layer ID.
    pub look_at_springs: HashMap<String, AngularSpringState>,
}

impl PhysicsState {
    pub fn new() -> Self {
        Self {
            springs: HashMap::new(),
            look_at_springs: HashMap::new(),
        }
    }

    /// Reset all spring states (called on animation restart).
    pub fn reset(&mut self) {
        self.springs.clear();
        self.look_at_springs.clear();
    }
}

/// State for angular spring (look-at smoothing).
#[derive(Debug, Clone, Copy)]
pub struct AngularSpringState {
    pub angle: f32,
    pub velocity: f32,
}

impl Default for AngularSpringState {
    fn default() -> Self {
        Self {
            angle: 0.0,
            velocity: 0.0,
        }
    }
}

/// Step a 2D spring simulation using semi-implicit Euler.
///
/// - `state`: current position and velocity
/// - `target`: target position the spring chases
/// - `frequency`: spring natural frequency in Hz
/// - `damping`: damping ratio (0 = no damping, 1 = critical, >1 = overdamped)
/// - `dt`: time step in seconds
///
/// Returns the new spring state.
pub fn step_spring(
    state: SpringState,
    target: Vec2,
    frequency: f32,
    damping: f32,
    dt: f32,
) -> SpringState {
    // Convert frequency to angular frequency
    let omega = 2.0 * std::f32::consts::PI * frequency;
    let omega_sq = omega * omega;
    let damping_force = 2.0 * damping * omega;

    // Compute spring force: F = -omega^2 * (pos - target) - 2*damping*omega * velocity
    let displacement = state.position - target;

    let accel = Vec2::new(
        -omega_sq * displacement.x - damping_force * state.velocity.x,
        -omega_sq * displacement.y - damping_force * state.velocity.y,
    );

    // Semi-implicit Euler: velocity first, then position
    let new_velocity = Vec2::new(
        state.velocity.x + accel.x * dt,
        state.velocity.y + accel.y * dt,
    );
    let new_position = Vec2::new(
        state.position.x + new_velocity.x * dt,
        state.position.y + new_velocity.y * dt,
    );

    SpringState {
        position: new_position,
        velocity: new_velocity,
    }
}

/// Step a 1D angular spring using semi-implicit Euler.
/// Handles angle wrapping correctly.
pub fn step_angular_spring(
    state: AngularSpringState,
    target_angle: f32,
    frequency: f32,
    damping: f32,
    dt: f32,
) -> AngularSpringState {
    let omega = 2.0 * std::f32::consts::PI * frequency;
    let omega_sq = omega * omega;
    let damping_force = 2.0 * damping * omega;

    // Use shortest angular difference
    let diff = wrap_angle(target_angle - state.angle);

    let accel = omega_sq * diff - damping_force * state.velocity;

    // Semi-implicit Euler
    let new_velocity = state.velocity + accel * dt;
    let new_angle = state.angle + new_velocity * dt;

    AngularSpringState {
        angle: wrap_angle(new_angle),
        velocity: new_velocity,
    }
}

/// Apply gravity and wind forces to a spring state.
/// Returns the adjusted state after applying external forces for one timestep.
pub fn apply_external_forces(
    state: SpringState,
    constraint: &PhysicsConstraint,
    time: f32,
    dt: f32,
) -> SpringState {
    let mut velocity = state.velocity;

    // Gravity: constant force in a given direction
    if let Some(ref gravity) = constraint.gravity {
        if gravity.strength > 0.0 {
            let angle_rad = gravity.angle.to_radians();
            let gx = angle_rad.cos() * gravity.strength;
            let gy = angle_rad.sin() * gravity.strength;
            velocity.x += gx * dt;
            velocity.y += gy * dt;
        }
    }

    // Wind: sinusoidal force (horizontal)
    if let Some(ref wind) = constraint.wind {
        if wind.strength > 0.0 {
            let wind_force = wind.strength
                * (time * wind.frequency * 2.0 * std::f32::consts::PI).sin();
            velocity.x += wind_force * dt;
        }
    }

    SpringState {
        position: state.position,
        velocity,
    }
}

/// Wrap an angle to the range [-PI, PI].
pub fn wrap_angle(angle: f32) -> f32 {
    let mut a = angle % std::f32::consts::TAU;
    if a > std::f32::consts::PI {
        a -= std::f32::consts::TAU;
    } else if a < -std::f32::consts::PI {
        a += std::f32::consts::TAU;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spring_converges_to_target() {
        let target = Vec2::new(100.0, 50.0);
        let mut state = SpringState {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
        };

        // Run for 5 seconds at 60fps
        let dt = 1.0 / 60.0;
        for _ in 0..300 {
            state = step_spring(state, target, 2.0, 0.7, dt);
        }

        // Should have converged close to target
        assert!(
            (state.position.x - target.x).abs() < 1.0,
            "x={}, expected ~{}",
            state.position.x,
            target.x
        );
        assert!(
            (state.position.y - target.y).abs() < 1.0,
            "y={}, expected ~{}",
            state.position.y,
            target.y
        );
        // Velocity should be near zero
        assert!(
            state.velocity.length() < 1.0,
            "velocity={:?}",
            state.velocity
        );
    }

    #[test]
    fn test_spring_underdamped_overshoots() {
        let target = Vec2::new(100.0, 0.0);
        let mut state = SpringState {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
        };
        let dt = 1.0 / 60.0;

        let mut max_x = 0.0f32;
        for _ in 0..300 {
            state = step_spring(state, target, 3.0, 0.3, dt);
            max_x = max_x.max(state.position.x);
        }

        // With low damping, should overshoot the target
        assert!(
            max_x > target.x,
            "max_x={}, expected overshoot past {}",
            max_x,
            target.x
        );
    }

    #[test]
    fn test_spring_critically_damped_no_oscillation() {
        let target = Vec2::new(100.0, 0.0);
        let mut state = SpringState {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
        };
        let dt = 1.0 / 60.0;

        // With critical damping (1.0) or higher, should not significantly overshoot
        let mut max_overshoot = 0.0f32;
        for _ in 0..600 {
            state = step_spring(state, target, 2.0, 1.0, dt);
            let overshoot = state.position.x - target.x;
            max_overshoot = max_overshoot.max(overshoot);
        }

        // Critical damping allows very small overshoot due to discrete integration
        assert!(
            max_overshoot < 5.0,
            "max overshoot={}, expected near 0",
            max_overshoot
        );
    }

    #[test]
    fn test_angular_spring_wraps_correctly() {
        // Start near PI, target near -PI (should take the short path)
        let mut state = AngularSpringState {
            angle: 3.0, // near PI
            velocity: 0.0,
        };
        let target = -3.0; // near -PI

        let dt = 1.0 / 60.0;
        for _ in 0..300 {
            state = step_angular_spring(state, target, 2.0, 0.7, dt);
        }

        // Should have converged close to target
        let diff = wrap_angle(state.angle - target).abs();
        assert!(diff < 0.1, "angle={}, target={}, diff={}", state.angle, target, diff);
    }

    #[test]
    fn test_wrap_angle() {
        assert!((wrap_angle(0.0) - 0.0).abs() < 0.001);
        assert!((wrap_angle(std::f32::consts::PI) - std::f32::consts::PI).abs() < 0.001);
        assert!((wrap_angle(std::f32::consts::TAU) - 0.0).abs() < 0.001);
        assert!((wrap_angle(-std::f32::consts::TAU) - 0.0).abs() < 0.001);

        // 3*PI should wrap to approximately -PI or PI
        let w = wrap_angle(3.0 * std::f32::consts::PI);
        assert!(w.abs() - std::f32::consts::PI < 0.01, "wrapped 3*PI = {}", w);
    }

    #[test]
    fn test_external_forces_gravity() {
        let state = SpringState {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
        };
        let constraint = PhysicsConstraint {
            frequency: 2.0,
            damping: 0.5,
            mix: 1.0,
            gravity: Some(crate::model::sprite::GravityForce {
                angle: 270.0, // straight down in screen coords
                strength: 100.0,
            }),
            wind: None,
        };

        let result = apply_external_forces(state, &constraint, 0.0, 1.0 / 60.0);

        // Gravity at 270 degrees: cos(270deg) ~= 0, sin(270deg) ~= -1
        // So force should be mostly in the -Y direction
        assert!(
            result.velocity.y < -0.1,
            "expected negative y velocity from gravity, got {}",
            result.velocity.y
        );
    }
}
