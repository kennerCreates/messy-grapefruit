use crate::math;
use crate::model::Vec2;
use crate::model::sprite::StrokeElement;

/// Result of a hit test
#[derive(Debug, Clone)]
pub struct HitResult {
    pub element_id: String,
    #[allow(dead_code)]
    pub vertex_id: Option<String>,
    pub distance: f32,
}

/// Test if a point is within threshold distance of any stroke element.
/// Returns the closest hit, if any.
pub fn hit_test_elements(
    point: Vec2,
    elements: &[StrokeElement],
    threshold: f32,
) -> Option<HitResult> {
    let mut best: Option<HitResult> = None;

    for element in elements {
        if element.vertices.len() < 2 {
            // Check single vertex
            if let Some(v) = element.vertices.first() {
                let d = point.distance(v.pos);
                if d <= threshold {
                    if best.as_ref().is_none_or(|b| d < b.distance) {
                        best = Some(HitResult {
                            element_id: element.id.clone(),
                            vertex_id: Some(v.id.clone()),
                            distance: d,
                        });
                    }
                }
            }
            continue;
        }

        // Flatten the element's bezier curves to a polyline
        let mut polyline = Vec::new();
        for i in 0..element.vertices.len() - 1 {
            let (p0, p1, p2, p3) =
                math::segment_bezier_points(&element.vertices[i], &element.vertices[i + 1]);
            math::flatten_cubic_bezier(p0, p1, p2, p3, 1.0, &mut polyline);
        }

        // Check distance from point to each line segment in the polyline
        for i in 0..polyline.len().saturating_sub(1) {
            let d = point_to_segment_distance(point, polyline[i], polyline[i + 1]);
            if d <= threshold {
                if best.as_ref().is_none_or(|b| d < b.distance) {
                    best = Some(HitResult {
                        element_id: element.id.clone(),
                        vertex_id: None,
                        distance: d,
                    });
                }
            }
        }

        // Also check proximity to vertices specifically
        for v in &element.vertices {
            let d = point.distance(v.pos);
            if d <= threshold {
                if best.as_ref().is_none_or(|b| d < b.distance) {
                    best = Some(HitResult {
                        element_id: element.id.clone(),
                        vertex_id: Some(v.id.clone()),
                        distance: d,
                    });
                }
            }
        }
    }

    best
}

/// Distance from a point to a line segment
fn point_to_segment_distance(point: Vec2, seg_start: Vec2, seg_end: Vec2) -> f32 {
    let dx = seg_end.x - seg_start.x;
    let dy = seg_end.y - seg_start.y;
    let len_sq = dx * dx + dy * dy;

    if len_sq < 1e-10 {
        return point.distance(seg_start);
    }

    let t = ((point.x - seg_start.x) * dx + (point.y - seg_start.y) * dy) / len_sq;
    let t = t.clamp(0.0, 1.0);

    let closest = Vec2 {
        x: seg_start.x + t * dx,
        y: seg_start.y + t * dy,
    };

    point.distance(closest)
}
