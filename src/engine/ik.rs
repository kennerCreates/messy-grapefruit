use crate::model::sprite::{AngleConstraint, IKChain, SolverType, Sprite};
use crate::model::Vec2;

/// Result of a 2-bone IK solve: new positions for the mid and tip joints.
#[derive(Debug, Clone, Copy)]
pub struct TwoBoneSolution {
    pub mid_pos: Vec2,
    pub tip_pos: Vec2,
}

/// Solve a 2-bone IK chain analytically using the law of cosines.
///
/// - `root`: position of the root joint (fixed)
/// - `bone1_len`: length of the first bone (root to mid)
/// - `bone2_len`: length of the second bone (mid to tip)
/// - `target`: desired position for the tip
/// - `bend_direction`: +1 or -1 to flip the elbow/knee bend side
/// - `constraints`: optional per-joint angle constraints (min, max) in radians
///
/// Returns the solved mid and tip positions.
pub fn two_bone_solve(
    root: Vec2,
    bone1_len: f32,
    bone2_len: f32,
    target: Vec2,
    bend_direction: i8,
    constraints: &[AngleConstraint],
) -> TwoBoneSolution {
    let to_target = target - root;
    let dist = to_target.length();

    // Clamp distance to reachable range
    let max_reach = bone1_len + bone2_len;
    let min_reach = (bone1_len - bone2_len).abs();

    if dist < 1e-6 {
        // Target is at root -- just extend along +X
        let mid = Vec2::new(root.x + bone1_len, root.y);
        let tip = Vec2::new(root.x + bone1_len + bone2_len, root.y);
        return TwoBoneSolution { mid_pos: mid, tip_pos: tip };
    }

    let clamped_dist = dist.clamp(min_reach + 1e-6, max_reach - 1e-6);

    // Angle from root to target
    let angle_to_target = to_target.y.atan2(to_target.x);

    // Law of cosines: find the angle at root between bone1 and the line to target
    // cos(A) = (b1^2 + d^2 - b2^2) / (2 * b1 * d)
    let cos_a = (bone1_len * bone1_len + clamped_dist * clamped_dist - bone2_len * bone2_len)
        / (2.0 * bone1_len * clamped_dist);
    let cos_a = cos_a.clamp(-1.0, 1.0);
    let angle_a = cos_a.acos();

    // Apply bend direction as a sign flip on the offset angle
    let bend_sign = if bend_direction >= 0 { 1.0f32 } else { -1.0f32 };
    let bone1_angle = angle_to_target + bend_sign * angle_a;

    // Compute mid position
    let mid = Vec2::new(
        root.x + bone1_len * bone1_angle.cos(),
        root.y + bone1_len * bone1_angle.sin(),
    );

    // Apply angle constraint on the first joint (bone1 angle) if present
    let mid = if let Some(constraint) = constraints.first() {
        let constrained_angle = clamp_angle(bone1_angle, constraint.min, constraint.max);
        Vec2::new(
            root.x + bone1_len * constrained_angle.cos(),
            root.y + bone1_len * constrained_angle.sin(),
        )
    } else {
        mid
    };

    // Compute tip position: direction from mid toward target, length = bone2_len
    let mid_to_target = target - mid;
    let mid_to_target_dist = mid_to_target.length();

    let tip = if mid_to_target_dist < 1e-6 {
        // Mid is at target; extend in bone1 direction
        let dir_angle = bone1_angle;
        Vec2::new(
            mid.x + bone2_len * dir_angle.cos(),
            mid.y + bone2_len * dir_angle.sin(),
        )
    } else {
        let bone2_angle = mid_to_target.y.atan2(mid_to_target.x);

        // Apply angle constraint on the second joint (bone2 angle relative to bone1) if present
        let bone2_angle = if constraints.len() > 1 {
            let relative_angle = normalize_angle(bone2_angle - bone1_angle);
            let constrained = clamp_angle(relative_angle, constraints[1].min, constraints[1].max);
            bone1_angle + constrained
        } else {
            bone2_angle
        };

        Vec2::new(
            mid.x + bone2_len * bone2_angle.cos(),
            mid.y + bone2_len * bone2_angle.sin(),
        )
    };

    TwoBoneSolution { mid_pos: mid, tip_pos: tip }
}

/// FABRIK (Forward And Backward Reaching Inverse Kinematics) solver.
///
/// - `joint_positions`: current positions of all joints (root to tip, at least 2)
/// - `target`: desired position for the last joint (tip)
/// - `iterations`: number of forward-backward iterations (3-10 recommended)
/// - `tolerance`: distance threshold to consider the solution converged
///
/// Returns the new joint positions.
pub fn fabrik_solve(
    joint_positions: &[Vec2],
    target: Vec2,
    iterations: usize,
    tolerance: f32,
) -> Vec<Vec2> {
    let n = joint_positions.len();
    if n < 2 {
        return joint_positions.to_vec();
    }

    // Compute bone lengths
    let mut bone_lengths = Vec::with_capacity(n - 1);
    for i in 0..n - 1 {
        bone_lengths.push(joint_positions[i].distance(joint_positions[i + 1]));
    }

    let total_length: f32 = bone_lengths.iter().sum();
    let root = joint_positions[0];

    let mut joints = joint_positions.to_vec();

    // Check if target is reachable
    let root_to_target = root.distance(target);
    if root_to_target > total_length {
        // Target is unreachable: stretch toward it
        let dir = (target - root).normalized();
        // If dir is zero (target == root), add tiny perturbation
        let dir = if dir.length() < 1e-6 {
            Vec2::new(1e-4, 0.0)
        } else {
            dir
        };
        let mut current = root;
        joints[0] = root;
        for i in 0..n - 1 {
            current = current + dir * bone_lengths[i];
            joints[i + 1] = current;
        }
        return joints;
    }

    for _iter in 0..iterations {
        // Check if close enough
        let tip_dist = joints[n - 1].distance(target);
        if tip_dist < tolerance {
            break;
        }

        // Forward reaching: start from tip, move toward target
        joints[n - 1] = target;
        for i in (0..n - 1).rev() {
            let dir = joints[i] - joints[i + 1];
            let dir_len = dir.length();
            let dir = if dir_len < 1e-6 {
                // Add tiny perturbation to avoid collinear deadlock
                Vec2::new(1e-4, 1e-4)
            } else {
                dir * (1.0 / dir_len)
            };
            joints[i] = joints[i + 1] + dir * bone_lengths[i];
        }

        // Backward reaching: start from root, move back
        joints[0] = root;
        for i in 0..n - 1 {
            let dir = joints[i + 1] - joints[i];
            let dir_len = dir.length();
            let dir = if dir_len < 1e-6 {
                // Add tiny perturbation to avoid collinear deadlock
                Vec2::new(1e-4, 1e-4)
            } else {
                dir * (1.0 / dir_len)
            };
            joints[i + 1] = joints[i] + dir * bone_lengths[i];
        }
    }

    joints
}

/// Normalize angle to [-PI, PI]
fn normalize_angle(angle: f32) -> f32 {
    let mut a = angle % std::f32::consts::TAU;
    if a > std::f32::consts::PI {
        a -= std::f32::consts::TAU;
    } else if a < -std::f32::consts::PI {
        a += std::f32::consts::TAU;
    }
    a
}

/// Clamp an angle between min and max (in radians).
fn clamp_angle(angle: f32, min: f32, max: f32) -> f32 {
    let a = normalize_angle(angle);
    a.clamp(min, max)
}

/// Resolve IK chains for a sprite at a given animation time.
/// This modifies layer positions to reflect the IK solution.
///
/// Called after FK evaluation and initial socket walk, before constraints/physics.
/// Returns a map of layer_id -> world-space position offset from IK solving.
pub fn solve_ik_chains(
    sprite: &Sprite,
    chains: &[IKChain],
    ik_target_positions: &[(String, Vec2)], // (ik_target_element_id, world_position)
    ik_mix_values: &[(String, f32)],        // (ik_chain_id, mix_value)
) -> Vec<(String, Vec2)> {
    // Returns: Vec<(layer_id, new_world_position)>
    let mut results = Vec::new();

    for chain in chains {
        if chain.layer_ids.is_empty() {
            continue;
        }

        // Get the mix value (from animation or default)
        let mix = ik_mix_values
            .iter()
            .find(|(id, _)| id == &chain.id)
            .map(|(_, m)| *m)
            .unwrap_or(chain.mix);

        if mix < 1e-6 {
            continue; // Pure FK, skip IK solving
        }

        // Find the target position
        let target_pos = ik_target_positions
            .iter()
            .find(|(id, _)| id == &chain.target_element_id)
            .map(|(_, pos)| *pos);

        let Some(target) = target_pos else {
            continue; // No target found
        };

        // Gather joint world positions from the sprite's current state
        let mut joint_positions = Vec::new();
        for layer_id in &chain.layer_ids {
            if let Some(layer) = sprite.layers.iter().find(|l| l.id == *layer_id) {
                // Use the socket transform to get world position
                let transform = crate::engine::socket::resolve_socket_transform(sprite, &layer.id);
                // The layer's origin in world space is the socket transform position
                // plus the layer's own element positions
                let layer_origin = if let Some(first_elem) = layer.elements.first() {
                    Vec2::new(
                        transform.position.x + first_elem.origin.x,
                        transform.position.y + first_elem.origin.y,
                    )
                } else {
                    transform.position
                };
                joint_positions.push((layer_id.clone(), layer_origin));
            }
        }

        if joint_positions.len() < 2 {
            continue;
        }

        let positions: Vec<Vec2> = joint_positions.iter().map(|(_, p)| *p).collect();

        // Solve based on solver type
        let solved_positions = match chain.solver {
            SolverType::TwoBone => {
                if positions.len() >= 2 {
                    // For 2-bone: positions[0] = root, positions[1] = mid, target = tip
                    let root = positions[0];
                    let bone1_len = if positions.len() > 1 {
                        root.distance(positions[1])
                    } else {
                        0.0
                    };
                    let bone2_len = if positions.len() > 2 {
                        positions[1].distance(positions[2])
                    } else if positions.len() > 1 {
                        positions[1].distance(target)
                    } else {
                        0.0
                    };

                    let solution = two_bone_solve(
                        root,
                        bone1_len,
                        bone2_len,
                        target,
                        chain.bend_direction,
                        &chain.angle_constraints,
                    );

                    let mut solved = vec![root, solution.mid_pos, solution.tip_pos];
                    // Truncate or extend to match joint count
                    solved.truncate(positions.len());
                    while solved.len() < positions.len() {
                        solved.push(*solved.last().unwrap_or(&target));
                    }
                    solved
                } else {
                    positions.clone()
                }
            }
            SolverType::Fabrik => {
                fabrik_solve(&positions, target, 8, 0.01)
            }
        };

        // Blend FK and IK positions based on mix
        for (i, ((layer_id, fk_pos), ik_pos)) in joint_positions
            .iter()
            .zip(solved_positions.iter())
            .enumerate()
        {
            // Skip the root joint (index 0) -- it stays fixed
            if i == 0 {
                continue;
            }
            let blended = fk_pos.lerp(*ik_pos, mix);
            results.push((layer_id.clone(), blended));
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_two_bone_straight_reach() {
        // Root at origin, bones of length 100 each, target straight ahead
        let solution = two_bone_solve(
            Vec2::ZERO,
            100.0,
            100.0,
            Vec2::new(200.0, 0.0),
            1,
            &[],
        );

        // Should be fully extended along X axis
        assert!((solution.mid_pos.x - 100.0).abs() < 1.0);
        assert!(solution.mid_pos.y.abs() < 1.0);
        assert!((solution.tip_pos.x - 200.0).abs() < 1.0);
        assert!(solution.tip_pos.y.abs() < 1.0);
    }

    #[test]
    fn test_two_bone_bend_direction() {
        // Target at a reachable distance that requires bending
        let target = Vec2::new(100.0, 0.0);

        let solution_pos = two_bone_solve(
            Vec2::ZERO,
            100.0,
            100.0,
            target,
            1,
            &[],
        );
        let solution_neg = two_bone_solve(
            Vec2::ZERO,
            100.0,
            100.0,
            target,
            -1,
            &[],
        );

        // Mid positions should be on opposite sides of the line to target
        // With positive bend, mid should be above (y > 0) or below depending on convention
        // They should be mirror images across the X axis
        assert!((solution_pos.mid_pos.y + solution_neg.mid_pos.y).abs() < 1.0,
            "Mid positions should be symmetric: pos.y={}, neg.y={}",
            solution_pos.mid_pos.y, solution_neg.mid_pos.y);

        // Both tips should reach the target
        assert!(solution_pos.tip_pos.distance(target) < 2.0);
        assert!(solution_neg.tip_pos.distance(target) < 2.0);
    }

    #[test]
    fn test_two_bone_unreachable() {
        // Target is beyond max reach
        let solution = two_bone_solve(
            Vec2::ZERO,
            50.0,
            50.0,
            Vec2::new(200.0, 0.0),
            1,
            &[],
        );

        // Should extend toward target as far as possible
        // Mid should be along the direction to target
        assert!(solution.mid_pos.x > 0.0);
        assert!(solution.mid_pos.y.abs() < 2.0);
    }

    #[test]
    fn test_two_bone_with_angle_constraint() {
        let target = Vec2::new(100.0, 0.0);
        let constraints = vec![
            AngleConstraint {
                layer_id: "root".to_string(),
                min: -0.5,
                max: 0.5,
            },
        ];

        let solution = two_bone_solve(
            Vec2::ZERO,
            100.0,
            100.0,
            target,
            1,
            &constraints,
        );

        // The bone1 angle should be within constraint bounds
        let bone1_angle = (solution.mid_pos.y).atan2(solution.mid_pos.x);
        assert!(bone1_angle >= -0.5 - 0.01 && bone1_angle <= 0.5 + 0.01,
            "Bone1 angle {} should be in [-0.5, 0.5]", bone1_angle);
    }

    #[test]
    fn test_fabrik_basic_convergence() {
        let joints = vec![
            Vec2::ZERO,
            Vec2::new(100.0, 0.0),
            Vec2::new(200.0, 0.0),
        ];
        let target = Vec2::new(150.0, 100.0);

        let result = fabrik_solve(&joints, target, 10, 0.01);

        // Root should stay at origin
        assert!((result[0].x).abs() < 0.01);
        assert!((result[0].y).abs() < 0.01);

        // Tip should be close to target
        let tip_dist = result[2].distance(target);
        assert!(tip_dist < 1.0, "Tip distance to target: {}", tip_dist);

        // Bone lengths should be preserved
        let bone1 = result[0].distance(result[1]);
        let bone2 = result[1].distance(result[2]);
        assert!((bone1 - 100.0).abs() < 1.0, "Bone 1 length: {}", bone1);
        assert!((bone2 - 100.0).abs() < 1.0, "Bone 2 length: {}", bone2);
    }

    #[test]
    fn test_fabrik_unreachable_target() {
        let joints = vec![
            Vec2::ZERO,
            Vec2::new(50.0, 0.0),
            Vec2::new(100.0, 0.0),
        ];
        let target = Vec2::new(500.0, 0.0); // Way beyond reach

        let result = fabrik_solve(&joints, target, 10, 0.01);

        // Root stays at origin
        assert!((result[0].x).abs() < 0.01);

        // All joints should be stretched toward target
        assert!(result[1].x > 0.0);
        assert!(result[2].x > result[1].x);

        // Total chain length should equal sum of bone lengths
        let total = result[0].distance(result[1]) + result[1].distance(result[2]);
        assert!((total - 100.0).abs() < 1.0, "Total length: {}", total);
    }

    #[test]
    fn test_fabrik_collinear_perturbation() {
        // Joints perfectly collinear, target behind them -- tests perturbation
        let joints = vec![
            Vec2::ZERO,
            Vec2::new(100.0, 0.0),
            Vec2::new(200.0, 0.0),
        ];
        let target = Vec2::new(-50.0, 0.0);

        let result = fabrik_solve(&joints, target, 10, 0.01);

        // Root should stay at origin
        assert!((result[0].x).abs() < 0.01);
        assert!((result[0].y).abs() < 0.01);

        // Bone lengths should be preserved
        let bone1 = result[0].distance(result[1]);
        let bone2 = result[1].distance(result[2]);
        assert!((bone1 - 100.0).abs() < 1.0, "Bone 1 length: {}", bone1);
        assert!((bone2 - 100.0).abs() < 1.0, "Bone 2 length: {}", bone2);
    }

    #[test]
    fn test_fabrik_long_chain() {
        // 5-joint chain (4 bones), like a tail/tentacle
        let joints = vec![
            Vec2::ZERO,
            Vec2::new(30.0, 0.0),
            Vec2::new(60.0, 0.0),
            Vec2::new(90.0, 0.0),
            Vec2::new(120.0, 0.0),
        ];
        let target = Vec2::new(60.0, 80.0);

        let result = fabrik_solve(&joints, target, 10, 0.01);

        // Root stays fixed
        assert!((result[0].x).abs() < 0.01);

        // Tip should be close to target (if reachable)
        let tip_dist = result[4].distance(target);
        assert!(tip_dist < 2.0, "Tip distance to target: {}", tip_dist);

        // All bone lengths should be approximately preserved
        for i in 0..4 {
            let bone_len = result[i].distance(result[i + 1]);
            assert!(
                (bone_len - 30.0).abs() < 1.0,
                "Bone {} length: {} (expected ~30.0)",
                i,
                bone_len
            );
        }
    }

    #[test]
    fn test_normalize_angle() {
        assert!((normalize_angle(0.0) - 0.0).abs() < 0.001);
        assert!((normalize_angle(std::f32::consts::PI) - std::f32::consts::PI).abs() < 0.001);
        assert!((normalize_angle(std::f32::consts::TAU) - 0.0).abs() < 0.001);
        assert!((normalize_angle(-std::f32::consts::TAU) - 0.0).abs() < 0.001);
        // 3*PI = PI (mod TAU), which normalizes to PI (at the boundary, PI is valid)
        let norm_3pi = normalize_angle(3.0 * std::f32::consts::PI);
        assert!(norm_3pi.abs() - std::f32::consts::PI < 0.01,
            "3*PI normalized to {}, expected +/-PI", norm_3pi);
    }

    #[test]
    fn test_two_bone_target_at_origin() {
        // Edge case: target at root
        let solution = two_bone_solve(
            Vec2::ZERO,
            100.0,
            100.0,
            Vec2::ZERO,
            1,
            &[],
        );

        // Should produce valid (non-NaN) positions
        assert!(!solution.mid_pos.x.is_nan());
        assert!(!solution.mid_pos.y.is_nan());
        assert!(!solution.tip_pos.x.is_nan());
        assert!(!solution.tip_pos.y.is_nan());
    }
}
