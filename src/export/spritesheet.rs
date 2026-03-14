//! Spritesheet export: step through animation frames, rasterize each,
//! pack into an atlas, and write atlas PNG + TextureAtlasLayout RON.

use crate::export::{ExportError, ensure_dir};
use crate::model::project::{ExportSettings, LayoutMode, Palette};
use crate::model::sprite::{AnimationSequence, Skin, Sprite};

use std::path::Path;

/// Result of a spritesheet export operation.
#[allow(dead_code)]
pub struct SpritesheetExportResult {
    pub atlas_path: std::path::PathBuf,
    pub ron_path: std::path::PathBuf,
    pub summary: String,
}

/// Export a spritesheet for a sprite + animation sequence.
///
/// Steps through the animation at the configured FPS, evaluates the full
/// pipeline per frame, generates SVG, rasterizes, and packs into an atlas.
pub fn export_spritesheet(
    sprite: &Sprite,
    sequence: &AnimationSequence,
    palette: &Palette,
    skin: Option<&Skin>,
    settings: &ExportSettings,
    output_dir: &Path,
) -> Result<SpritesheetExportResult, ExportError> {
    ensure_dir(output_dir)?;

    let fps = settings.fps.max(1);
    let duration = sequence.duration;
    let frame_count = ((duration * fps as f32).ceil() as usize).max(1);
    let padding = settings.padding;

    // Step 1: Generate all frames with physics baking
    // Physics runs at 60 FPS internally; we sample at the configured export FPS
    let mut frames: Vec<Vec<u8>> = Vec::with_capacity(frame_count);
    let canvas_w = sprite.canvas_width;
    let canvas_h = sprite.canvas_height;

    let physics_fps: f32 = 60.0;
    let physics_dt = 1.0 / physics_fps;
    let mut physics_state = crate::engine::physics::PhysicsState::new();
    let total_physics_steps = (duration * physics_fps).ceil() as usize;

    // Pre-compute which physics step corresponds to each export frame
    let mut frame_at_step: Vec<Option<usize>> = vec![None; total_physics_steps + 1];
    for frame_idx in 0..frame_count {
        let frame_time = frame_idx as f32 / fps as f32;
        let step = (frame_time * physics_fps).round() as usize;
        let step = step.min(total_physics_steps);
        frame_at_step[step] = Some(frame_idx);
    }

    // Step sequentially from frame 0 to bake physics
    let mut frame_results: Vec<Option<Vec<u8>>> = vec![None; frame_count];
    for (step, frame_idx_opt) in frame_at_step.iter().enumerate() {
        let time = step as f32 * physics_dt;

        // Check if we need to capture this frame
        if let Some(&frame_idx) = frame_idx_opt.as_ref() {
            let animated = crate::engine::animation::create_animated_sprite_with_physics(
                sprite, sequence, time, Some(&mut physics_state),
            );
            let svg_string =
                crate::export::svg_gen::sprite_to_svg(&animated, palette, skin);
            let png_bytes = crate::export::rasterize::svg_to_png(&svg_string, canvas_w, canvas_h)?;
            frame_results[frame_idx] = Some(png_bytes);
        } else {
            // Still step physics even if we don't capture
            let _ = crate::engine::animation::create_animated_sprite_with_physics(
                sprite, sequence, time, Some(&mut physics_state),
            );
        }
    }

    for result in frame_results {
        frames.push(result.unwrap_or_else(|| {
            // Fallback: generate without physics (shouldn't happen)
            let animated = crate::engine::animation::create_animated_sprite(sprite, sequence, 0.0);
            let svg_string = crate::export::svg_gen::sprite_to_svg(&animated, palette, skin);
            crate::export::rasterize::svg_to_png(&svg_string, canvas_w, canvas_h)
                .unwrap_or_default()
        }));
    }

    // Step 2: Decode all frames to RGBA images
    let mut images: Vec<image::RgbaImage> = Vec::with_capacity(frames.len());
    for png_bytes in &frames {
        let img = image::load_from_memory_with_format(png_bytes, image::ImageFormat::Png)
            .map_err(|e| ExportError::Rasterize(format!("Failed to decode frame PNG: {}", e)))?
            .to_rgba8();
        images.push(img);
    }

    // Step 3: Uniform trim (if enabled)
    let (trim_x, trim_y, trim_w, trim_h) = if settings.trim {
        compute_uniform_trim(&images, canvas_w, canvas_h)
    } else {
        (0, 0, canvas_w, canvas_h)
    };

    // Crop frames to the uniform trim bounds
    let mut cropped: Vec<image::RgbaImage> = Vec::with_capacity(images.len());
    for img in &images {
        let sub = image::imageops::crop_imm(img, trim_x, trim_y, trim_w, trim_h).to_image();
        cropped.push(sub);
    }

    let tile_w = trim_w;
    let tile_h = trim_h;

    // Step 4: Compute layout
    let (columns, rows) = compute_layout(settings.layout, frame_count);

    // Step 5: Pack atlas
    let atlas_w = columns as u32 * (tile_w + padding) + padding;
    let atlas_h = rows as u32 * (tile_h + padding) + padding;

    let mut atlas = image::RgbaImage::new(atlas_w, atlas_h);

    for (idx, frame_img) in cropped.iter().enumerate() {
        let col = (idx % columns) as u32;
        let row = (idx / columns) as u32;
        let x = padding + col * (tile_w + padding);
        let y = padding + row * (tile_h + padding);

        for py in 0..tile_h.min(frame_img.height()) {
            for px in 0..tile_w.min(frame_img.width()) {
                let ax = x + px;
                let ay = y + py;
                if ax < atlas_w && ay < atlas_h {
                    atlas.put_pixel(ax, ay, *frame_img.get_pixel(px, py));
                }
            }
        }
    }

    // Step 6: Encode atlas PNG
    let mut atlas_bytes = Vec::new();
    {
        let encoder = image::codecs::png::PngEncoder::new(&mut atlas_bytes);
        image::ImageEncoder::write_image(
            encoder,
            atlas.as_raw(),
            atlas_w,
            atlas_h,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| ExportError::Rasterize(format!("Failed to encode atlas PNG: {}", e)))?;
    }

    // Step 7: Write files
    let safe_name = sanitize_filename(&sprite.name);
    let safe_anim = sanitize_filename(&sequence.name);
    let atlas_filename = format!("{}_{}_atlas.png", safe_name, safe_anim);
    let ron_filename = format!("{}_{}_atlas.ron", safe_name, safe_anim);

    let atlas_path = output_dir.join(&atlas_filename);
    let ron_path = output_dir.join(&ron_filename);

    std::fs::write(&atlas_path, &atlas_bytes)?;

    // Step 8: Generate RON metadata (TextureAtlasLayout-compatible)
    let ron_content = generate_texture_atlas_ron(
        tile_w, tile_h, columns as u32, rows as u32, padding, padding,
    );
    std::fs::write(&ron_path, &ron_content)?;

    let summary = format!(
        "Spritesheet exported:\n\
         Animation: {} ({:.1}s)\n\
         Frames: {}\n\
         Tile size: {}x{}\n\
         Layout: {}x{} ({})\n\
         Atlas size: {}x{}\n\
         FPS: {}\n\
         Padding: {}px\n\
         Trim: {}\n\
         Atlas: {}\n\
         RON: {}",
        sequence.name,
        duration,
        frame_count,
        tile_w,
        tile_h,
        columns,
        rows,
        settings.layout,
        atlas_w,
        atlas_h,
        fps,
        padding,
        if settings.trim { "uniform" } else { "none" },
        atlas_path.display(),
        ron_path.display(),
    );

    Ok(SpritesheetExportResult {
        atlas_path,
        ron_path,
        summary,
    })
}

/// Compute the uniform trim bounds across all frames.
/// Returns (x, y, w, h) of the smallest bounding box that contains
/// all non-transparent pixels across all frames.
fn compute_uniform_trim(
    images: &[image::RgbaImage],
    canvas_w: u32,
    canvas_h: u32,
) -> (u32, u32, u32, u32) {
    let mut global_min_x = canvas_w;
    let mut global_min_y = canvas_h;
    let mut global_max_x = 0u32;
    let mut global_max_y = 0u32;

    for img in images {
        for y in 0..img.height() {
            for x in 0..img.width() {
                let pixel = img.get_pixel(x, y);
                if pixel[3] > 0 {
                    global_min_x = global_min_x.min(x);
                    global_min_y = global_min_y.min(y);
                    global_max_x = global_max_x.max(x);
                    global_max_y = global_max_y.max(y);
                }
            }
        }
    }

    if global_max_x < global_min_x || global_max_y < global_min_y {
        // All frames are fully transparent
        return (0, 0, canvas_w, canvas_h);
    }

    let w = (global_max_x - global_min_x + 1).max(1);
    let h = (global_max_y - global_min_y + 1).max(1);

    (global_min_x, global_min_y, w, h)
}

/// Compute the number of columns and rows based on layout mode.
fn compute_layout(layout: LayoutMode, frame_count: usize) -> (usize, usize) {
    match layout {
        LayoutMode::Row => (frame_count, 1),
        LayoutMode::Column => (1, frame_count),
        LayoutMode::Grid => {
            // Grid: try to be as square as possible
            let cols = (frame_count as f32).sqrt().ceil() as usize;
            let rows = ((frame_count as f32) / cols as f32).ceil() as usize;
            (cols.max(1), rows.max(1))
        }
    }
}

/// Generate RON metadata compatible with Bevy's TextureAtlasLayout::from_grid().
fn generate_texture_atlas_ron(
    tile_w: u32,
    tile_h: u32,
    columns: u32,
    rows: u32,
    padding_x: u32,
    padding_y: u32,
) -> String {
    format!(
        "// TextureAtlasLayout::from_grid() parameters\n\
         // Usage: TextureAtlasLayout::from_grid(\n\
         //     UVec2::new({tile_w}, {tile_h}),  // tile_size\n\
         //     {columns},                         // columns\n\
         //     {rows},                            // rows\n\
         //     Some(UVec2::new({padding_x}, {padding_y})),  // padding\n\
         //     Some(UVec2::new({padding_x}, {padding_y})),  // offset\n\
         // )\n\
         (\n\
             tile_size: ({tile_w}, {tile_h}),\n\
             columns: {columns},\n\
             rows: {rows},\n\
             padding: ({padding_x}, {padding_y}),\n\
             offset: ({padding_x}, {padding_y}),\n\
         )\n"
    )
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

/// Generate a preview summary for the spritesheet export.
pub fn preview_spritesheet(
    sprite: &Sprite,
    sequence: &AnimationSequence,
    settings: &ExportSettings,
) -> String {
    let fps = settings.fps.max(1);
    let frame_count = ((sequence.duration * fps as f32).ceil() as usize).max(1);
    let (columns, rows) = compute_layout(settings.layout, frame_count);

    let tile_w = sprite.canvas_width;
    let tile_h = sprite.canvas_height;
    let atlas_w = columns as u32 * (tile_w + settings.padding) + settings.padding;
    let atlas_h = rows as u32 * (tile_h + settings.padding) + settings.padding;

    format!(
        "Spritesheet preview:\n\
         Animation: {} ({:.1}s)\n\
         Frames: {}\n\
         Tile size: {}x{} (before trim)\n\
         Layout: {}x{} ({})\n\
         Estimated atlas: {}x{}\n\
         FPS: {}\n\
         Padding: {}px\n\
         Trim: {}",
        sequence.name,
        sequence.duration,
        frame_count,
        tile_w,
        tile_h,
        columns,
        rows,
        settings.layout,
        atlas_w,
        atlas_h,
        fps,
        settings.padding,
        if settings.trim { "uniform" } else { "none" },
    )
}
