use crate::model::project::Palette;

/// Find palette colors related to the anchor color by hue proximity.
/// Returns up to `max_results` indices sorted by lightness (highlight → shadow).
pub fn find_color_ramp(palette: &Palette, anchor_index: u8, max_results: usize) -> Vec<u8> {
    let anchor = palette.get_color(anchor_index);
    if anchor.a == 0 {
        return vec![];
    }

    let (anchor_h, anchor_s, _) = rgb_to_hsl(anchor.r, anchor.g, anchor.b);

    // Skip very desaturated colors (grays) — use a lightness-only ramp
    let hue_tolerance = if anchor_s < 0.1 { 360.0 } else { 35.0 };

    let mut candidates: Vec<(u8, f32)> = Vec::new(); // (index, lightness)

    for (i, pc) in palette.colors.iter().enumerate() {
        let idx = i as u8;
        if idx == 0 || pc.a == 0 {
            continue; // skip transparent
        }
        let (h, s, l) = rgb_to_hsl(pc.r, pc.g, pc.b);

        // For desaturated anchor, accept any low-saturation colors
        if anchor_s < 0.1 {
            if s < 0.15 {
                candidates.push((idx, l));
            }
            continue;
        }

        // Hue distance (circular)
        let hue_dist = hue_distance(anchor_h, h);
        if hue_dist <= hue_tolerance && s > 0.05 {
            candidates.push((idx, l));
        }
    }

    // Sort by lightness (light → dark)
    candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Take up to max_results
    candidates.into_iter().take(max_results).map(|(idx, _)| idx).collect()
}

/// Convert RGB (0-255) to HSL (h: 0-360, s: 0-1, l: 0-1).
fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let rf = r as f32 / 255.0;
    let gf = g as f32 / 255.0;
    let bf = b as f32 / 255.0;

    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let delta = max - min;

    let l = (max + min) / 2.0;

    if delta < 1e-6 {
        return (0.0, 0.0, l); // achromatic
    }

    let s = if l < 0.5 {
        delta / (max + min)
    } else {
        delta / (2.0 - max - min)
    };

    let h = if (max - rf).abs() < 1e-6 {
        ((gf - bf) / delta) % 6.0
    } else if (max - gf).abs() < 1e-6 {
        (bf - rf) / delta + 2.0
    } else {
        (rf - gf) / delta + 4.0
    };

    let h = (h * 60.0 + 360.0) % 360.0;
    (h, s, l)
}

/// Circular hue distance in degrees (0-180).
fn hue_distance(h1: f32, h2: f32) -> f32 {
    let diff = (h1 - h2).abs();
    diff.min(360.0 - diff)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_to_hsl_red() {
        let (h, s, l) = rgb_to_hsl(255, 0, 0);
        assert!((h - 0.0).abs() < 1.0);
        assert!((s - 1.0).abs() < 0.01);
        assert!((l - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_rgb_to_hsl_white() {
        let (_, s, l) = rgb_to_hsl(255, 255, 255);
        assert!(s < 0.01);
        assert!((l - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_hue_distance() {
        assert!((hue_distance(10.0, 350.0) - 20.0).abs() < 0.01);
        assert!((hue_distance(180.0, 0.0) - 180.0).abs() < 0.01);
    }

    #[test]
    fn test_find_ramp_empty_palette() {
        let palette = crate::model::project::Palette::default_palette();
        let ramp = find_color_ramp(&palette, 0, 5);
        assert!(ramp.is_empty()); // transparent anchor = no ramp
    }
}
