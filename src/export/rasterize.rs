//! Rasterize SVG strings to PNG images using resvg.

use crate::export::ExportError;
use crate::model::project::Palette;
use crate::model::sprite::{Skin, StrokeElement};

/// Rasterize an SVG string to PNG bytes.
///
/// - `svg_string`: valid SVG document as a string
/// - `width`, `height`: desired output dimensions in pixels
///
/// Returns the PNG-encoded image bytes.
pub fn svg_to_png(svg_string: &str, width: u32, height: u32) -> Result<Vec<u8>, ExportError> {
    // Parse the SVG
    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_str(svg_string, &options)
        .map_err(|e| ExportError::Rasterize(format!("Failed to parse SVG: {}", e)))?;

    // Create a pixel buffer
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| ExportError::Rasterize("Failed to create pixel buffer".to_string()))?;

    // Render the SVG tree into the pixmap
    resvg::render(&tree, resvg::tiny_skia::Transform::default(), &mut pixmap.as_mut());

    // Encode to PNG
    let png_bytes = pixmap
        .encode_png()
        .map_err(|e| ExportError::Rasterize(format!("Failed to encode PNG: {}", e)))?;

    Ok(png_bytes)
}

/// Rasterize a single element to PNG bytes.
///
/// Generates an SVG for just this element (tightly cropped), rasterizes it,
/// and returns (png_bytes, width, height, offset_x, offset_y).
/// The offset indicates where the top-left corner of this PNG sits in
/// the element's world space.
pub fn element_to_png(
    element: &StrokeElement,
    palette: &Palette,
    skin: Option<&Skin>,
) -> Result<(Vec<u8>, u32, u32, f32, f32), ExportError> {
    let (svg_string, w, h, off_x, off_y) =
        crate::export::svg_gen::element_to_svg(element, palette, skin);

    if w == 0 || h == 0 {
        return Err(ExportError::Rasterize(
            "Element has zero-size bounding box".to_string(),
        ));
    }

    let png_bytes = svg_to_png(&svg_string, w, h)?;
    Ok((png_bytes, w, h, off_x, off_y))
}

/// Rasterize a full sprite SVG to PNG bytes at the sprite's canvas dimensions.
#[allow(dead_code)]
pub fn sprite_to_png(
    sprite: &crate::model::sprite::Sprite,
    palette: &Palette,
    skin: Option<&Skin>,
) -> Result<Vec<u8>, ExportError> {
    let svg_string = crate::export::svg_gen::sprite_to_svg(sprite, palette, skin);
    svg_to_png(&svg_string, sprite.canvas_width, sprite.canvas_height)
}
