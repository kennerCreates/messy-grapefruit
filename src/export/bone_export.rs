//! Bone animation export: rasterize each element as a separate PNG,
//! pack into a texture atlas, and export animation data as RON.
//!
//! This is the primary export path. Each skin produces its own atlas;
//! all skins share the same animation RON data.

use crate::export::ron_meta::{
    AtlasRegion, BoneAnimationData, PartDefinition, SkinManifestEntry,
};
use crate::export::{ExportError, ensure_dir};
use crate::model::project::Palette;
use crate::model::sprite::{Skin, Sprite};

use std::path::Path;

/// Result of a bone export operation.
pub struct BoneExportResult {
    /// Paths to generated atlas PNGs (one per skin + default)
    pub atlas_paths: Vec<std::path::PathBuf>,
    /// Path to the generated RON animation data file
    pub ron_path: std::path::PathBuf,
    /// Summary string for the export dialog
    #[allow(dead_code)]
    pub summary: String,
}

/// Rasterized element part (before atlas packing).
struct RasterizedPart {
    element_id: String,
    element_name: String,
    layer_id: String,
    png_bytes: Vec<u8>,
    width: u32,
    height: u32,
    /// Offset in world space (top-left of bounding box)
    offset_x: f32,
    offset_y: f32,
    /// Element origin (pivot) in world space
    origin_x: f32,
    origin_y: f32,
    /// Element position
    position_x: f32,
    position_y: f32,
    /// Element rotation (rest pose)
    rotation: f32,
    /// Element scale (rest pose)
    scale_x: f32,
    scale_y: f32,
    /// Socket parent reference
    socket_parent: Option<(String, String)>,
    /// Z-order (layer index * 1000 + element index)
    z_order: usize,
}

/// Export bone animation data for a sprite.
///
/// Produces:
/// - `{sprite_name}_default.png` (atlas for default skin)
/// - `{sprite_name}_{skin_name}.png` (atlas per skin)
/// - `{sprite_name}.ron` (animation data, shared)
///
/// Returns the export result with paths and summary.
pub fn export_bone_animation(
    sprite: &Sprite,
    palette: &Palette,
    output_dir: &Path,
    padding: u32,
) -> Result<BoneExportResult, ExportError> {
    ensure_dir(output_dir)?;

    let safe_name = sanitize_filename(&sprite.name);

    // 1. Rasterize all elements for the default (no-skin) atlas
    let default_parts = rasterize_all_elements(sprite, palette, None)?;

    // 2. Pack default atlas and get part definitions
    let (default_atlas_bytes, part_defs) = pack_atlas(&default_parts, padding)?;

    // Write default atlas
    let default_atlas_filename = format!("{}_default.png", safe_name);
    let default_atlas_path = output_dir.join(&default_atlas_filename);
    std::fs::write(&default_atlas_path, &default_atlas_bytes)?;

    let mut atlas_paths = vec![default_atlas_path.clone()];

    // 3. Process each skin
    let mut skin_entries = vec![SkinManifestEntry {
        name: "default".to_string(),
        atlas_file: default_atlas_filename,
    }];

    for skin in &sprite.skins {
        let skin_parts = rasterize_all_elements(sprite, palette, Some(skin))?;
        let (skin_atlas_bytes, _skin_part_defs) = pack_atlas(&skin_parts, padding)?;

        let skin_filename = format!("{}_{}.png", safe_name, sanitize_filename(&skin.name));
        let skin_atlas_path = output_dir.join(&skin_filename);
        std::fs::write(&skin_atlas_path, &skin_atlas_bytes)?;

        atlas_paths.push(skin_atlas_path);

        skin_entries.push(SkinManifestEntry {
            name: skin.name.clone(),
            atlas_file: skin_filename,
        });
    }

    // 4. Build and write RON animation data (shared across all skins)
    let ron_data = crate::export::ron_meta::build_bone_animation_data(
        sprite,
        part_defs,
        skin_entries,
    );
    let ron_string = crate::export::ron_meta::to_ron_string(&ron_data)?;
    let ron_path = output_dir.join(format!("{}.ron", safe_name));
    std::fs::write(&ron_path, &ron_string)?;

    // Build summary
    let summary = build_summary(&ron_data, &atlas_paths);

    Ok(BoneExportResult {
        atlas_paths,
        ron_path,
        summary,
    })
}

/// Preview bone export without writing files.
/// Returns the RON data and default atlas bytes for the preview dialog.
pub fn preview_bone_export(
    sprite: &Sprite,
    palette: &Palette,
    padding: u32,
) -> Result<(BoneAnimationData, Vec<u8>), ExportError> {
    let default_parts = rasterize_all_elements(sprite, palette, None)?;
    let (atlas_bytes, part_defs) = pack_atlas(&default_parts, padding)?;

    let skin_entries = vec![SkinManifestEntry {
        name: "default".to_string(),
        atlas_file: format!("{}_default.png", sanitize_filename(&sprite.name)),
    }];

    for skin in &sprite.skins {
        // Include skin names in manifest but don't rasterize for preview
        let _ = skin; // Just acknowledge it exists
    }

    let mut all_skin_entries = skin_entries;
    for skin in &sprite.skins {
        all_skin_entries.push(SkinManifestEntry {
            name: skin.name.clone(),
            atlas_file: format!(
                "{}_{}.png",
                sanitize_filename(&sprite.name),
                sanitize_filename(&skin.name)
            ),
        });
    }

    let ron_data = crate::export::ron_meta::build_bone_animation_data(
        sprite,
        part_defs,
        all_skin_entries,
    );

    Ok((ron_data, atlas_bytes))
}

// ---- Internal helpers ----

/// Rasterize all visible elements of a sprite to individual PNGs.
fn rasterize_all_elements(
    sprite: &Sprite,
    palette: &Palette,
    skin: Option<&Skin>,
) -> Result<Vec<RasterizedPart>, ExportError> {
    let mut parts = Vec::new();

    for (layer_idx, layer) in sprite.layers.iter().enumerate() {
        if !layer.visible {
            continue;
        }

        for (elem_idx, element) in layer.elements.iter().enumerate() {
            if element.vertices.is_empty() {
                continue;
            }

            let result = crate::export::rasterize::element_to_png(element, palette, skin);
            match result {
                Ok((png_bytes, w, h, off_x, off_y)) => {
                    parts.push(RasterizedPart {
                        element_id: element.id.clone(),
                        element_name: element
                            .name
                            .clone()
                            .unwrap_or_else(|| format!("part_{}", parts.len())),
                        layer_id: layer.id.clone(),
                        png_bytes,
                        width: w,
                        height: h,
                        offset_x: off_x,
                        offset_y: off_y,
                        origin_x: element.origin.x,
                        origin_y: element.origin.y,
                        position_x: element.position.x,
                        position_y: element.position.y,
                        rotation: element.rotation,
                        scale_x: element.scale.x,
                        scale_y: element.scale.y,
                        socket_parent: layer.socket.as_ref().map(|s| {
                            (s.parent_element_id.clone(), s.parent_vertex_id.clone())
                        }),
                        z_order: layer_idx * 1000 + elem_idx,
                    });
                }
                Err(e) => {
                    // Log warning but continue exporting other elements
                    eprintln!(
                        "Warning: Failed to rasterize element '{}': {}",
                        element.id, e
                    );
                }
            }
        }
    }

    Ok(parts)
}

/// Pack rasterized parts into a single texture atlas.
///
/// Uses a simple shelf-packing algorithm: parts are sorted by height (tallest first),
/// then placed left-to-right in rows.
///
/// Returns (atlas_png_bytes, part_definitions_with_atlas_regions).
fn pack_atlas(
    parts: &[RasterizedPart],
    padding: u32,
) -> Result<(Vec<u8>, Vec<PartDefinition>), ExportError> {
    if parts.is_empty() {
        // Return a 1x1 transparent PNG
        let img = image::RgbaImage::new(1, 1);
        let mut buf = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut buf);
        image::ImageEncoder::write_image(
            encoder,
            img.as_raw(),
            1,
            1,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| ExportError::Rasterize(format!("PNG encode error: {}", e)))?;
        return Ok((buf, Vec::new()));
    }

    // Sort by height (descending) for better packing
    let mut indexed: Vec<(usize, &RasterizedPart)> = parts.iter().enumerate().collect();
    indexed.sort_by(|a, b| b.1.height.cmp(&a.1.height));

    // Estimate atlas size
    let total_area: u64 = parts
        .iter()
        .map(|p| (p.width as u64 + padding as u64) * (p.height as u64 + padding as u64))
        .sum();
    let side = (total_area as f64).sqrt().ceil() as u32;
    let mut atlas_width = next_power_of_two(side.max(64));
    let mut atlas_height = atlas_width;

    // Shelf packing
    let mut placements: Vec<(usize, u32, u32)>; // (original_index, x, y)

    loop {
        placements = Vec::with_capacity(parts.len());
        let mut shelf_x = padding;
        let mut shelf_y = padding;
        let mut shelf_height = 0u32;
        let mut fits = true;

        for &(orig_idx, part) in &indexed {
            let pw = part.width + padding;
            let ph = part.height + padding;

            if shelf_x + pw > atlas_width {
                // Move to next shelf
                shelf_y += shelf_height + padding;
                shelf_x = padding;
                shelf_height = 0;
            }

            if shelf_y + ph > atlas_height {
                fits = false;
                break;
            }

            placements.push((orig_idx, shelf_x, shelf_y));
            shelf_x += pw;
            shelf_height = shelf_height.max(ph);
        }

        if fits {
            break;
        }

        // Double the size and retry
        if atlas_width <= atlas_height {
            atlas_width *= 2;
        } else {
            atlas_height *= 2;
        }

        // Safety: cap at 8192
        if atlas_width > 8192 || atlas_height > 8192 {
            return Err(ExportError::Rasterize(
                "Atlas too large (>8192px). Reduce element count or size.".to_string(),
            ));
        }
    }

    // Create the atlas image
    let mut atlas = image::RgbaImage::new(atlas_width, atlas_height);

    // Place each part and build part definitions
    let mut part_defs = vec![None; parts.len()];

    for (orig_idx, x, y) in &placements {
        let part = &parts[*orig_idx];

        // Decode the part's PNG into an RGBA image
        let part_img =
            image::load_from_memory_with_format(&part.png_bytes, image::ImageFormat::Png)
                .map_err(|e| {
                    ExportError::Rasterize(format!("Failed to decode part PNG: {}", e))
                })?
                .to_rgba8();

        // Copy pixels into the atlas
        for py in 0..part.height.min(part_img.height()) {
            for px in 0..part.width.min(part_img.width()) {
                let ax = x + px;
                let ay = y + py;
                if ax < atlas_width && ay < atlas_height {
                    atlas.put_pixel(ax, ay, *part_img.get_pixel(px, py));
                }
            }
        }

        // Build part definition
        part_defs[*orig_idx] = Some(PartDefinition {
            element_id: part.element_id.clone(),
            name: part.element_name.clone(),
            layer_id: part.layer_id.clone(),
            atlas_region: AtlasRegion {
                x: *x,
                y: *y,
                width: part.width,
                height: part.height,
            },
            origin: (
                part.origin_x - part.offset_x,
                part.origin_y - part.offset_y,
            ),
            position: (part.position_x, part.position_y),
            rotation: part.rotation,
            scale: (part.scale_x, part.scale_y),
            socket_parent: part.socket_parent.clone(),
            z_order: part.z_order,
        });
    }

    let part_defs: Vec<PartDefinition> = part_defs.into_iter().flatten().collect();

    // Encode atlas to PNG
    let mut atlas_bytes = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut atlas_bytes);
    image::ImageEncoder::write_image(
        encoder,
        atlas.as_raw(),
        atlas_width,
        atlas_height,
        image::ExtendedColorType::Rgba8,
    )
    .map_err(|e| ExportError::Rasterize(format!("Failed to encode atlas PNG: {}", e)))?;

    Ok((atlas_bytes, part_defs))
}

/// Round up to the next power of two.
fn next_power_of_two(n: u32) -> u32 {
    let mut v = n.max(1) - 1;
    v |= v >> 1;
    v |= v >> 2;
    v |= v >> 4;
    v |= v >> 8;
    v |= v >> 16;
    v + 1
}

/// Sanitize a string for use as a filename.
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Build a human-readable summary of the export.
fn build_summary(data: &BoneAnimationData, atlas_paths: &[std::path::PathBuf]) -> String {
    let mut summary = String::new();

    summary.push_str(&format!("Sprite: {}\n", data.name));
    summary.push_str(&format!(
        "Canvas: {}x{}\n",
        data.canvas_width, data.canvas_height
    ));
    summary.push_str(&format!("Parts: {}\n", data.parts.len()));
    summary.push_str(&format!("Animations: {}\n", data.animations.len()));

    for anim in &data.animations {
        summary.push_str(&format!(
            "  - {} ({:.1}s, {} tracks, {})\n",
            anim.name,
            anim.duration,
            anim.tracks.len(),
            if anim.looping { "looping" } else { "once" }
        ));
    }

    summary.push_str(&format!("IK Chains: {}\n", data.ik_chains.len()));
    summary.push_str(&format!(
        "Layers with dynamics: {}\n",
        data.layer_dynamics.len()
    ));
    summary.push_str(&format!("Skins: {}\n", data.skins.len()));

    for skin in &data.skins {
        summary.push_str(&format!("  - {} -> {}\n", skin.name, skin.atlas_file));
    }

    summary.push_str(&format!("\nAtlas files: {}\n", atlas_paths.len()));
    for path in atlas_paths {
        if let Some(name) = path.file_name() {
            summary.push_str(&format!("  - {}\n", name.to_string_lossy()));
        }
    }

    summary
}
