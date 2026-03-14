//! SVG generation from sprite data.
//!
//! Converts a Sprite (optionally at a given animation time, with a skin applied)
//! into an SVG string suitable for rasterization via resvg.

use crate::model::project::Palette;
use crate::model::sprite::{Skin, Sprite, StrokeElement};
use crate::model::Vec2;

/// Generate an SVG string for the entire sprite at a given state.
///
/// - `sprite`: the sprite (already animated if needed -- caller should pass the
///   result of `create_animated_sprite` for a specific time).
/// - `palette`: the project palette for color lookups.
/// - `skin`: optional skin to apply visual overrides.
///
/// The SVG viewport matches `canvas_width` x `canvas_height`.
pub fn sprite_to_svg(sprite: &Sprite, palette: &Palette, skin: Option<&Skin>) -> String {
    let w = sprite.canvas_width;
    let h = sprite.canvas_height;

    let mut svg = String::with_capacity(4096);
    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">"#,
        w, h, w, h
    ));
    svg.push('\n');

    // Background fill
    let bg_color = resolve_palette_color(sprite.background_color_index, palette);
    if let Some(bg) = bg_color {
        svg.push_str(&format!(
            r#"  <rect x="0" y="0" width="{}" height="{}" fill="{}"/>"#,
            w, h, bg
        ));
        svg.push('\n');
    }

    // Render layers bottom-to-top
    for layer in &sprite.layers {
        if !layer.visible {
            continue;
        }

        // Compute the socket transform offset for this layer
        let socket_tf =
            crate::engine::socket::resolve_socket_transform(sprite, &layer.id);

        for element in &layer.elements {
            let resolved = resolve_element_visuals(element, skin);
            element_to_svg_fragment(
                element,
                &resolved,
                palette,
                socket_tf.position,
                socket_tf.rotation,
                &mut svg,
            );
        }
    }

    svg.push_str("</svg>\n");
    svg
}

/// Generate an SVG string for a single element in isolation.
/// Used by bone export to rasterize individual body parts.
///
/// The SVG viewport is sized to exactly fit the element's bounding box plus
/// the stroke width, so each element PNG is tightly cropped.
///
/// Returns (svg_string, width, height, offset_x, offset_y) where offset is the
/// top-left corner of the bounding box in world space.
pub fn element_to_svg(
    element: &StrokeElement,
    palette: &Palette,
    skin: Option<&Skin>,
) -> (String, u32, u32, f32, f32) {
    let resolved = resolve_element_visuals(element, skin);

    // Compute the element's bounding box
    let (min, max) = element_bounding_box(element);
    let stroke_w = resolved.stroke_width;
    let padding = stroke_w.ceil() + 2.0; // extra padding for anti-aliasing

    let x0 = min.x - padding;
    let y0 = min.y - padding;
    let svg_w = (max.x - min.x + 2.0 * padding).ceil().max(1.0);
    let svg_h = (max.y - min.y + 2.0 * padding).ceil().max(1.0);

    let w = svg_w as u32;
    let h = svg_h as u32;

    let mut svg = String::with_capacity(2048);
    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="{} {} {} {}">"#,
        w, h, x0, y0, svg_w, svg_h
    ));
    svg.push('\n');

    // Render the element at its own position (no socket transform offset)
    element_to_svg_fragment(
        element,
        &resolved,
        palette,
        Vec2::ZERO,
        0.0,
        &mut svg,
    );

    svg.push_str("</svg>\n");
    (svg, w, h, x0, y0)
}

// ---- Internal helpers ----

/// Resolved visual properties for an element (with skin overrides applied).
struct ResolvedVisuals {
    stroke_color_index: usize,
    fill_color_index: usize,
    stroke_width: f32,
}

fn resolve_element_visuals(element: &StrokeElement, skin: Option<&Skin>) -> ResolvedVisuals {
    let mut vis = ResolvedVisuals {
        stroke_color_index: element.stroke_color_index,
        fill_color_index: element.fill_color_index,
        stroke_width: element.stroke_width,
    };

    if let Some(skin) = skin
        && let Some(ov) = skin.overrides.iter().find(|o| o.element_id == element.id) {
            if let Some(sci) = ov.stroke_color_index {
                vis.stroke_color_index = sci;
            }
            if let Some(fci) = ov.fill_color_index {
                vis.fill_color_index = fci;
            }
            if let Some(sw) = ov.stroke_width {
                vis.stroke_width = sw;
            }
        }

    vis
}

/// Resolve a palette index to a CSS-compatible color string.
/// Returns None for index 0 (transparent) or out-of-range indices.
fn resolve_palette_color(index: usize, palette: &Palette) -> Option<String> {
    if index == 0 {
        return None;
    }
    palette.colors.get(index).map(|c| {
        let hex = c.hex.trim_start_matches('#');
        if hex.len() == 8 {
            // RGBA hex -- convert to CSS rgba
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
            let a = u8::from_str_radix(&hex[6..8], 16).unwrap_or(255);
            if a == 255 {
                format!("#{}", &hex[0..6])
            } else {
                format!(
                    "rgba({},{},{},{})",
                    r,
                    g,
                    b,
                    a as f32 / 255.0
                )
            }
        } else {
            format!("#{}", hex)
        }
    })
}

/// Write an SVG `<path>` or `<g>` fragment for a single element.
fn element_to_svg_fragment(
    element: &StrokeElement,
    visuals: &ResolvedVisuals,
    palette: &Palette,
    socket_offset: Vec2,
    socket_rotation: f32,
    out: &mut String,
) {
    if element.vertices.is_empty() {
        return;
    }

    // Build the path data string
    let d = build_path_data(element);
    if d.is_empty() {
        return;
    }

    // Resolve colors
    let stroke_color = resolve_palette_color(visuals.stroke_color_index, palette);
    let fill_color = resolve_palette_color(visuals.fill_color_index, palette);

    let stroke_attr = match &stroke_color {
        Some(c) => format!(r#" stroke="{}" stroke-width="{:.2}" stroke-linecap="round" stroke-linejoin="round""#, c, visuals.stroke_width),
        None => r#" stroke="none""#.to_string(),
    };
    let fill_attr = match &fill_color {
        Some(c) if element.closed => format!(r#" fill="{}""#, c),
        _ => r#" fill="none""#.to_string(),
    };

    // Build transform attribute
    let transform = build_transform(element, socket_offset, socket_rotation);
    let transform_attr = if transform.is_empty() {
        String::new()
    } else {
        format!(r#" transform="{}""#, transform)
    };

    out.push_str(&format!(
        r#"  <path d="{}"{}{}{}/>"#,
        d, stroke_attr, fill_attr, transform_attr
    ));
    out.push('\n');
}

/// Build SVG path data (the `d` attribute) from an element's vertices.
fn build_path_data(element: &StrokeElement) -> String {
    let verts = &element.vertices;
    if verts.is_empty() {
        return String::new();
    }

    let mut d = String::with_capacity(verts.len() * 40);

    // Move to first vertex
    d.push_str(&format!("M{:.2},{:.2}", verts[0].pos.x, verts[0].pos.y));

    // Draw segments
    for i in 0..verts.len() - 1 {
        let v0 = &verts[i];
        let v1 = &verts[i + 1];

        let cp1 = v0.cp2.unwrap_or(v0.pos);
        let cp2 = v1.cp1.unwrap_or(v1.pos);

        // Check if this segment is a straight line (control points at endpoints)
        let is_straight = (cp1.x - v0.pos.x).abs() < 0.01
            && (cp1.y - v0.pos.y).abs() < 0.01
            && (cp2.x - v1.pos.x).abs() < 0.01
            && (cp2.y - v1.pos.y).abs() < 0.01;

        if is_straight {
            d.push_str(&format!(" L{:.2},{:.2}", v1.pos.x, v1.pos.y));
        } else {
            d.push_str(&format!(
                " C{:.2},{:.2} {:.2},{:.2} {:.2},{:.2}",
                cp1.x, cp1.y, cp2.x, cp2.y, v1.pos.x, v1.pos.y
            ));
        }
    }

    if element.closed && verts.len() > 1 {
        // Close with a curve from last to first
        let last = verts.last().unwrap();
        let first = &verts[0];
        let cp1 = last.cp2.unwrap_or(last.pos);
        let cp2 = first.cp1.unwrap_or(first.pos);
        let is_straight = (cp1.x - last.pos.x).abs() < 0.01
            && (cp1.y - last.pos.y).abs() < 0.01
            && (cp2.x - first.pos.x).abs() < 0.01
            && (cp2.y - first.pos.y).abs() < 0.01;

        if is_straight {
            d.push_str(" Z");
        } else {
            d.push_str(&format!(
                " C{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} Z",
                cp1.x, cp1.y, cp2.x, cp2.y, first.pos.x, first.pos.y
            ));
        }
    }

    d
}

/// Build SVG transform string for element position, rotation, scale, and origin.
fn build_transform(
    element: &StrokeElement,
    socket_offset: Vec2,
    socket_rotation: f32,
) -> String {
    let mut parts = Vec::new();

    // Apply socket offset
    let total_x = element.position.x + socket_offset.x;
    let total_y = element.position.y + socket_offset.y;
    let total_rotation = element.rotation + socket_rotation;

    if total_x.abs() > 0.001 || total_y.abs() > 0.001 {
        parts.push(format!("translate({:.2},{:.2})", total_x, total_y));
    }

    // Rotation around origin
    if total_rotation.abs() > 0.001 {
        let deg = total_rotation.to_degrees();
        parts.push(format!(
            "rotate({:.2},{:.2},{:.2})",
            deg, element.origin.x, element.origin.y
        ));
    }

    // Scale around origin
    if (element.scale.x - 1.0).abs() > 0.001 || (element.scale.y - 1.0).abs() > 0.001 {
        // translate to origin, scale, translate back
        parts.push(format!(
            "translate({:.2},{:.2})",
            element.origin.x, element.origin.y
        ));
        parts.push(format!(
            "scale({:.4},{:.4})",
            element.scale.x, element.scale.y
        ));
        parts.push(format!(
            "translate({:.2},{:.2})",
            -element.origin.x, -element.origin.y
        ));
    }

    parts.join(" ")
}

/// Compute the axis-aligned bounding box of an element's vertices
/// (in element-local space, before position/rotation/scale transforms).
fn element_bounding_box(element: &StrokeElement) -> (Vec2, Vec2) {
    if element.vertices.is_empty() {
        return (Vec2::ZERO, Vec2::ZERO);
    }

    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;

    for v in &element.vertices {
        // Include vertex position
        update_bounds(v.pos, &mut min_x, &mut min_y, &mut max_x, &mut max_y);

        // Include control points
        if let Some(cp) = v.cp1 {
            update_bounds(cp, &mut min_x, &mut min_y, &mut max_x, &mut max_y);
        }
        if let Some(cp) = v.cp2 {
            update_bounds(cp, &mut min_x, &mut min_y, &mut max_x, &mut max_y);
        }
    }

    // Offset by element position
    min_x += element.position.x;
    min_y += element.position.y;
    max_x += element.position.x;
    max_y += element.position.y;

    (Vec2::new(min_x, min_y), Vec2::new(max_x, max_y))
}

fn update_bounds(p: Vec2, min_x: &mut f32, min_y: &mut f32, max_x: &mut f32, max_y: &mut f32) {
    if p.x < *min_x {
        *min_x = p.x;
    }
    if p.y < *min_y {
        *min_y = p.y;
    }
    if p.x > *max_x {
        *max_x = p.x;
    }
    if p.y > *max_y {
        *max_y = p.y;
    }
}
