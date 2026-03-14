use crate::engine;
use crate::engine::animation as anim_engine;
use crate::engine::socket as socket_engine;
use crate::math;
use crate::model::project::{GridMode, Palette, Theme};
use crate::model::sprite::{PathVertex, Skin, Sprite, StrokeElement};
use crate::model::Vec2;
use crate::state::editor::{EditorState, HandleDragState, ToolKind};
use crate::state::history::{History, SnapshotCommand};
use crate::theme;
use crate::ui::grid;

/// Actions produced by canvas interaction
pub enum CanvasAction {
    SpriteChanged,
}

/// Draw the central canvas panel.
#[allow(clippy::too_many_arguments)]
pub fn draw_canvas(
    ctx: &egui::Context,
    sprite: &mut Sprite,
    editor_state: &mut EditorState,
    palette: &Palette,
    current_theme: Theme,
    grid_size_base: f32,
    history: &mut History,
    sprite_index: usize,
    grid_mode: GridMode,
    physics_state: &mut crate::engine::physics::PhysicsState,
) -> Vec<CanvasAction> {
    let mut actions = Vec::new();

    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(theme::canvas_bg_color(current_theme)))
        .show(ctx, |ui| {
            let available_rect = ui.available_rect_before_wrap();
            let canvas_center = Vec2::new(
                (available_rect.min.x + available_rect.max.x) / 2.0,
                (available_rect.min.y + available_rect.max.y) / 2.0,
            );

            // Allocate the full canvas area for input
            let response = ui.allocate_rect(available_rect, egui::Sense::click_and_drag());

            let painter = ui.painter_at(available_rect);

            // Update cursor positions
            if let Some(hover_pos) = response.hover_pos() {
                let screen_pos = Vec2::from(hover_pos);
                editor_state.cursor_screen_pos = Some(screen_pos);
                editor_state.cursor_world_pos = Some(
                    editor_state
                        .viewport
                        .screen_to_world(screen_pos, canvas_center),
                );
            } else {
                editor_state.cursor_screen_pos = None;
                editor_state.cursor_world_pos = None;
            }

            // --- Draw grid ---
            let current_grid_size = grid::draw_grid(
                &painter,
                &editor_state.viewport,
                available_rect,
                canvas_center,
                grid_size_base,
                current_theme,
                grid_mode,
            );

            // --- Draw canvas boundary ---
            draw_canvas_boundary(
                &painter,
                &editor_state.viewport,
                canvas_center,
                sprite.canvas_width,
                sprite.canvas_height,
                current_theme,
            );

            // --- Render onion skinning ghost frames (before current elements) ---
            render_onion_skins(
                &painter,
                &editor_state.viewport,
                canvas_center,
                sprite,
                palette,
                editor_state,
            );

            // --- Render all visible elements (with animation applied) ---
            {
                // If an animation sequence is selected, render the animated version
                let animated_sprite = get_animated_sprite_with_physics(sprite, editor_state, physics_state);
                let render_sprite = animated_sprite.as_ref().unwrap_or(sprite);
                let active_seq = get_active_sequence(sprite, editor_state);

                render_elements(
                    &painter,
                    &editor_state.viewport,
                    canvas_center,
                    render_sprite,
                    palette,
                    editor_state,
                    current_theme,
                    active_seq.as_ref(),
                    editor_state.animation.current_time,
                );

                // Render IK targets as crosshair icons
                render_ik_targets(
                    &painter,
                    &editor_state.viewport,
                    canvas_center,
                    render_sprite,
                    current_theme,
                );

                // Render debug overlays
                render_debug_overlays(
                    &painter,
                    &editor_state.viewport,
                    canvas_center,
                    render_sprite,
                    &editor_state.debug_overlays,
                    current_theme,
                );
            }

            // --- Handle viewport controls ---
            handle_viewport_controls(
                &response,
                editor_state,
                canvas_center,
                ctx,
            );

            // --- Handle tool interactions ---
            match editor_state.active_tool {
                ToolKind::Line => {
                    handle_line_tool(
                        &response,
                        sprite,
                        editor_state,
                        canvas_center,
                        current_grid_size,
                        &painter,
                        current_theme,
                        history,
                        sprite_index,
                        &mut actions,
                        grid_mode,
                    );
                }
                ToolKind::Select => {
                    handle_select_tool(
                        ctx,
                        &response,
                        sprite,
                        editor_state,
                        canvas_center,
                        &painter,
                        current_theme,
                        history,
                        sprite_index,
                        &mut actions,
                    );
                }
                ToolKind::Fill => {
                    handle_fill_tool(
                        &response,
                        sprite,
                        editor_state,
                        canvas_center,
                        current_grid_size,
                        history,
                        sprite_index,
                        &mut actions,
                        grid_mode,
                    );
                }
                ToolKind::Eraser => {
                    handle_eraser_tool(
                        &response,
                        sprite,
                        editor_state,
                        canvas_center,
                        current_grid_size,
                        history,
                        sprite_index,
                        &mut actions,
                        grid_mode,
                    );
                }
            }

            // --- Draw selection highlights ---
            draw_selection_highlights(
                &painter,
                sprite,
                editor_state,
                canvas_center,
            );

            // --- Draw marquee rectangle ---
            if editor_state.marquee.is_active
                && let (Some(start), Some(current)) = (
                    editor_state.marquee.start_world,
                    editor_state.marquee.current_world,
                ) {
                    let s_start = editor_state.viewport.world_to_screen(start, canvas_center);
                    let s_current = editor_state.viewport.world_to_screen(current, canvas_center);
                    let rect = egui::Rect::from_two_pos(
                        egui::pos2(s_start.x, s_start.y),
                        egui::pos2(s_current.x, s_current.y),
                    );
                    painter.rect_stroke(
                        rect,
                        0.0,
                        egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(0x4a, 0x7a, 0x96, 200)),
                        egui::epaint::StrokeKind::Outside,
                    );
                    painter.rect_filled(
                        rect,
                        0.0,
                        egui::Color32::from_rgba_unmultiplied(0x4a, 0x7a, 0x96, 30),
                    );
                }

            // --- Draw curve handles on selected vertices ---
            draw_curve_handles(
                &painter,
                sprite,
                editor_state,
                canvas_center,
                current_theme,
            );

            // --- Draw transform handles on selection bounding box ---
            if editor_state.active_tool == ToolKind::Select
                && !editor_state.selection.selected_element_ids.is_empty()
            {
                draw_transform_handles(
                    &painter,
                    sprite,
                    editor_state,
                    canvas_center,
                );
            }

            // --- Draw toast message ---
            if let Some(ref toast) = editor_state.toast {
                let elapsed = toast.created.elapsed();
                if elapsed.as_secs() < 3 {
                    let alpha = if elapsed.as_secs() >= 2 {
                        ((3.0 - elapsed.as_secs_f32()) * 255.0) as u8
                    } else {
                        255
                    };
                    let toast_rect = egui::Rect::from_min_size(
                        egui::pos2(available_rect.center().x - 100.0, available_rect.min.y + 10.0),
                        egui::vec2(200.0, 30.0),
                    );
                    painter.rect_filled(
                        toast_rect,
                        4.0,
                        egui::Color32::from_rgba_unmultiplied(40, 40, 40, alpha),
                    );
                    painter.text(
                        toast_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        &toast.text,
                        egui::FontId::default(),
                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
                    );
                    ctx.request_repaint();
                }
            }

            // Request continuous repaint while drawing, hovering, or animating
            if editor_state.line_tool_state.active_element_id.is_some()
                || editor_state.cursor_screen_pos.is_some()
                || editor_state.marquee.is_active
                || editor_state.select_drag.is_dragging
                || editor_state.animation.playing
            {
                ctx.request_repaint();
            }
        });

    actions
}

/// Get the active animation sequence if one is selected
fn get_active_sequence(
    sprite: &Sprite,
    editor_state: &EditorState,
) -> Option<crate::model::sprite::AnimationSequence> {
    editor_state
        .animation
        .selected_sequence_id
        .as_ref()
        .and_then(|id| sprite.animations.iter().find(|a| a.id == *id))
        .cloned()
}

/// Create an animated sprite for rendering if an animation is active (without physics).
#[allow(dead_code)]
fn get_animated_sprite(
    sprite: &Sprite,
    editor_state: &EditorState,
) -> Option<Sprite> {
    let seq_id = editor_state.animation.selected_sequence_id.as_ref()?;
    let seq = sprite.animations.iter().find(|a| a.id == *seq_id)?;
    let time = editor_state.animation.current_time;
    Some(anim_engine::create_animated_sprite(sprite, seq, time))
}

/// Create an animated sprite with physics simulation (for playback).
fn get_animated_sprite_with_physics(
    sprite: &Sprite,
    editor_state: &EditorState,
    physics_state: &mut crate::engine::physics::PhysicsState,
) -> Option<Sprite> {
    let seq_id = editor_state.animation.selected_sequence_id.as_ref()?;
    let seq = sprite.animations.iter().find(|a| a.id == *seq_id)?;
    let time = editor_state.animation.current_time;
    // Physics only runs during playback
    if editor_state.animation.playing {
        Some(anim_engine::create_animated_sprite_with_physics(
            sprite,
            seq,
            time,
            Some(physics_state),
        ))
    } else {
        Some(anim_engine::create_animated_sprite(sprite, seq, time))
    }
}

/// Render onion skinning ghost frames
fn render_onion_skins(
    painter: &egui::Painter,
    viewport: &crate::state::editor::ViewportState,
    canvas_center: Vec2,
    sprite: &Sprite,
    palette: &Palette,
    editor_state: &EditorState,
) {
    if !editor_state.animation.onion_skinning {
        return;
    }

    let Some(ref seq_id) = editor_state.animation.selected_sequence_id else {
        return;
    };

    let Some(seq) = sprite.animations.iter().find(|a| a.id == *seq_id) else {
        return;
    };

    let current_time = editor_state.animation.current_time;
    let step = editor_state.animation.onion_step;

    // Render ghost frames BEFORE current time (blue-tinted, fading opacity)
    for i in 1..=editor_state.animation.onion_before {
        let ghost_time = current_time - step * i as f32;
        if ghost_time < 0.0 {
            continue;
        }
        let opacity = 0.3 / i as f32; // Decreasing opacity for further frames
        let ghost_sprite = anim_engine::create_animated_sprite(sprite, seq, ghost_time);
        render_ghost_frame(
            painter,
            viewport,
            canvas_center,
            &ghost_sprite,
            palette,
            editor_state,
            opacity,
            egui::Color32::from_rgba_unmultiplied(80, 120, 200, (opacity * 255.0) as u8),
            seq,
            ghost_time,
        );
    }

    // Render ghost frames AFTER current time (red-tinted, fading opacity)
    for i in 1..=editor_state.animation.onion_after {
        let ghost_time = current_time + step * i as f32;
        if ghost_time > seq.duration {
            continue;
        }
        let opacity = 0.3 / i as f32;
        let ghost_sprite = anim_engine::create_animated_sprite(sprite, seq, ghost_time);
        render_ghost_frame(
            painter,
            viewport,
            canvas_center,
            &ghost_sprite,
            palette,
            editor_state,
            opacity,
            egui::Color32::from_rgba_unmultiplied(200, 80, 80, (opacity * 255.0) as u8),
            seq,
            ghost_time,
        );
    }
}

/// Render a single ghost frame for onion skinning
#[allow(clippy::too_many_arguments)]
fn render_ghost_frame(
    painter: &egui::Painter,
    viewport: &crate::state::editor::ViewportState,
    canvas_center: Vec2,
    ghost_sprite: &Sprite,
    _palette: &Palette,
    _editor_state: &EditorState,
    opacity: f32,
    tint: egui::Color32,
    sequence: &crate::model::sprite::AnimationSequence,
    time: f32,
) {
    let alpha = (opacity * 255.0).min(255.0) as u8;
    let ghost_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(
        tint.r(), tint.g(), tint.b(), alpha,
    ));

    for layer in &ghost_sprite.layers {
        if !layer.visible {
            continue;
        }

        // Resolve socket transform for ghost frame
        let socket_transform = socket_engine::resolve_socket_transform(ghost_sprite, &layer.id);
        let has_socket = layer.socket.is_some();

        let transform_point = |p: Vec2| -> Vec2 {
            if has_socket {
                apply_socket_transform(p, &socket_transform)
            } else {
                p
            }
        };

        for element in &layer.elements {
            // Check visibility
            if !anim_engine::is_element_visible(sequence, &element.id, time) {
                continue;
            }

            if element.vertices.len() < 2 {
                if let Some(v) = element.vertices.first() {
                    let world_pos = transform_point(v.pos);
                    let screen_pos = viewport.world_to_screen(world_pos, canvas_center);
                    painter.circle_filled(
                        egui::pos2(screen_pos.x, screen_pos.y),
                        2.0,
                        ghost_stroke.color,
                    );
                }
                continue;
            }

            // Draw bezier segments as ghost outlines
            for i in 0..element.vertices.len() - 1 {
                let (p0, p1, p2, p3) =
                    math::segment_bezier_points(&element.vertices[i], &element.vertices[i + 1]);

                let tp0 = transform_point(p0);
                let tp1 = transform_point(p1);
                let tp2 = transform_point(p2);
                let tp3 = transform_point(p3);

                let sp0 = viewport.world_to_screen(tp0, canvas_center);
                let sp1 = viewport.world_to_screen(tp1, canvas_center);
                let sp2 = viewport.world_to_screen(tp2, canvas_center);
                let sp3 = viewport.world_to_screen(tp3, canvas_center);

                let shape = egui::epaint::CubicBezierShape::from_points_stroke(
                    [sp0.into(), sp1.into(), sp2.into(), sp3.into()],
                    false,
                    egui::Color32::TRANSPARENT,
                    ghost_stroke,
                );
                painter.add(shape);
            }

            // Closing segment for closed elements
            if element.closed && element.vertices.len() >= 2 {
                let last = element.vertices.last().unwrap();
                let first = element.vertices.first().unwrap();
                let (p0, p1, p2, p3) = math::segment_bezier_points(last, first);

                let tp0 = transform_point(p0);
                let tp1 = transform_point(p1);
                let tp2 = transform_point(p2);
                let tp3 = transform_point(p3);

                let sp0 = viewport.world_to_screen(tp0, canvas_center);
                let sp1 = viewport.world_to_screen(tp1, canvas_center);
                let sp2 = viewport.world_to_screen(tp2, canvas_center);
                let sp3 = viewport.world_to_screen(tp3, canvas_center);

                let shape = egui::epaint::CubicBezierShape::from_points_stroke(
                    [sp0.into(), sp1.into(), sp2.into(), sp3.into()],
                    false,
                    egui::Color32::TRANSPARENT,
                    ghost_stroke,
                );
                painter.add(shape);
            }
        }
    }
}

fn draw_canvas_boundary(
    painter: &egui::Painter,
    viewport: &crate::state::editor::ViewportState,
    canvas_center: Vec2,
    canvas_width: u32,
    canvas_height: u32,
    current_theme: Theme,
) {
    let w = canvas_width as f32;
    let h = canvas_height as f32;

    // Canvas is centered at world origin
    let top_left = Vec2::new(-w / 2.0, -h / 2.0);
    let bottom_right = Vec2::new(w / 2.0, h / 2.0);

    let screen_tl = viewport.world_to_screen(top_left, canvas_center);
    let screen_br = viewport.world_to_screen(bottom_right, canvas_center);

    let rect = egui::Rect::from_min_max(
        egui::pos2(screen_tl.x, screen_tl.y),
        egui::pos2(screen_br.x, screen_br.y),
    );

    // Dashed rectangle
    let boundary_color = match current_theme {
        Theme::Dark => egui::Color32::from_rgba_unmultiplied(0xfb, 0xbb, 0xad, 80),
        Theme::Light => egui::Color32::from_rgba_unmultiplied(0x25, 0x21, 0x3e, 80),
    };
    let stroke = egui::Stroke::new(1.0, boundary_color);

    // Draw dashed lines for each edge
    draw_dashed_line(painter, rect.left_top(), rect.right_top(), stroke, 6.0, 4.0);
    draw_dashed_line(painter, rect.right_top(), rect.right_bottom(), stroke, 6.0, 4.0);
    draw_dashed_line(painter, rect.right_bottom(), rect.left_bottom(), stroke, 6.0, 4.0);
    draw_dashed_line(painter, rect.left_bottom(), rect.left_top(), stroke, 6.0, 4.0);
}

fn draw_dashed_line(
    painter: &egui::Painter,
    from: egui::Pos2,
    to: egui::Pos2,
    stroke: egui::Stroke,
    dash_len: f32,
    gap_len: f32,
) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let total_len = (dx * dx + dy * dy).sqrt();
    if total_len < 0.1 {
        return;
    }

    let dir_x = dx / total_len;
    let dir_y = dy / total_len;
    let cycle = dash_len + gap_len;

    let mut pos = 0.0;
    while pos < total_len {
        let dash_end = (pos + dash_len).min(total_len);
        let p1 = egui::pos2(from.x + dir_x * pos, from.y + dir_y * pos);
        let p2 = egui::pos2(from.x + dir_x * dash_end, from.y + dir_y * dash_end);
        painter.line_segment([p1, p2], stroke);
        pos += cycle;
    }
}

/// Apply a socket transform to a world-space point.
fn apply_socket_transform(pos: Vec2, transform: &socket_engine::SocketTransform) -> Vec2 {
    let cos_r = transform.rotation.cos();
    let sin_r = transform.rotation.sin();
    let rx = pos.x * cos_r - pos.y * sin_r;
    let ry = pos.x * sin_r + pos.y * cos_r;
    Vec2::new(rx + transform.position.x, ry + transform.position.y)
}

/// Resolve skin overrides for an element. Returns (stroke_color_index, fill_color_index, stroke_width).
/// If the skin has an override for the element, use the override value; otherwise use the base value.
fn resolve_skin_overrides(
    skin: &Skin,
    element_id: &str,
    base_stroke_color_index: usize,
    base_fill_color_index: usize,
    base_stroke_width: f32,
) -> (usize, usize, f32) {
    if let Some(ovr) = skin.overrides.iter().find(|o| o.element_id == element_id) {
        (
            ovr.stroke_color_index.unwrap_or(base_stroke_color_index),
            ovr.fill_color_index.unwrap_or(base_fill_color_index),
            ovr.stroke_width.unwrap_or(base_stroke_width),
        )
    } else {
        (base_stroke_color_index, base_fill_color_index, base_stroke_width)
    }
}

#[allow(clippy::too_many_arguments)]
fn render_elements(
    painter: &egui::Painter,
    viewport: &crate::state::editor::ViewportState,
    canvas_center: Vec2,
    sprite: &Sprite,
    palette: &Palette,
    editor_state: &EditorState,
    _current_theme: Theme,
    active_sequence: Option<&crate::model::sprite::AnimationSequence>,
    current_time: f32,
) {
    // Resolve the active skin for override lookups
    let active_skin: Option<&Skin> = editor_state.active_skin_id.as_ref().and_then(|skin_id| {
        sprite.skins.iter().find(|s| s.id == *skin_id)
    });

    for layer in &sprite.layers {
        if !layer.visible {
            continue;
        }

        // Resolve socket transform for this layer
        let socket_transform = socket_engine::resolve_socket_transform(sprite, &layer.id);
        let has_socket = layer.socket.is_some();

        for element in &layer.elements {
            // Check element visibility via animation
            if let Some(seq) = active_sequence
                && !anim_engine::is_element_visible(seq, &element.id, current_time) {
                    continue;
                }

            // Apply skin overrides if active
            let (effective_stroke_color_index, effective_fill_color_index, effective_stroke_width) =
                if let Some(skin) = active_skin {
                    resolve_skin_overrides(skin, &element.id, element.stroke_color_index, element.fill_color_index, element.stroke_width)
                } else {
                    (element.stroke_color_index, element.fill_color_index, element.stroke_width)
                };

            // Look up palette colors by index (indexed color rendering)
            let stroke_color = palette
                .colors
                .get(effective_stroke_color_index)
                .map(|c| c.to_color32())
                .unwrap_or(egui::Color32::WHITE);

            let fill_color = palette
                .colors
                .get(effective_fill_color_index)
                .map(|c| c.to_color32())
                .unwrap_or(egui::Color32::TRANSPARENT);

            let stroke_width = effective_stroke_width * viewport.zoom;

            // Skip elements with transparent stroke and fill
            if stroke_color.a() == 0 && fill_color.a() == 0 {
                continue;
            }

            // Helper closure to transform a point through the socket chain
            let transform_point = |p: Vec2| -> Vec2 {
                if has_socket {
                    apply_socket_transform(p, &socket_transform)
                } else {
                    p
                }
            };

            // Draw fill for closed elements (render fill first, then stroke on top)
            if element.closed && fill_color.a() > 0 && element.vertices.len() >= 3 {
                let mut fill_points = Vec::new();
                for i in 0..element.vertices.len() {
                    let next = (i + 1) % element.vertices.len();
                    let (p0, p1, p2, p3) =
                        math::segment_bezier_points(&element.vertices[i], &element.vertices[next]);
                    math::flatten_cubic_bezier(p0, p1, p2, p3, 1.0 / viewport.zoom, &mut fill_points);
                }

                let screen_points: Vec<egui::Pos2> = fill_points
                    .iter()
                    .map(|p| {
                        let tp = transform_point(*p);
                        let sp = viewport.world_to_screen(tp, canvas_center);
                        egui::pos2(sp.x, sp.y)
                    })
                    .collect();

                if screen_points.len() >= 3 {
                    let shape = egui::epaint::PathShape::convex_polygon(
                        screen_points,
                        fill_color,
                        egui::Stroke::NONE,
                    );
                    painter.add(shape);
                }
            }

            if element.vertices.len() < 2 {
                // Draw single vertex as a dot
                if let Some(v) = element.vertices.first() {
                    let world_pos = transform_point(v.pos);
                    let screen_pos = viewport.world_to_screen(world_pos, canvas_center);
                    painter.circle_filled(
                        egui::pos2(screen_pos.x, screen_pos.y),
                        (stroke_width / 2.0).max(2.0),
                        stroke_color,
                    );
                }
                continue;
            }

            // Draw bezier segments
            for i in 0..element.vertices.len() - 1 {
                let (p0, p1, p2, p3) =
                    math::segment_bezier_points(&element.vertices[i], &element.vertices[i + 1]);

                let tp0 = transform_point(p0);
                let tp1 = transform_point(p1);
                let tp2 = transform_point(p2);
                let tp3 = transform_point(p3);

                let sp0 = viewport.world_to_screen(tp0, canvas_center);
                let sp1 = viewport.world_to_screen(tp1, canvas_center);
                let sp2 = viewport.world_to_screen(tp2, canvas_center);
                let sp3 = viewport.world_to_screen(tp3, canvas_center);

                let shape = egui::epaint::CubicBezierShape::from_points_stroke(
                    [sp0.into(), sp1.into(), sp2.into(), sp3.into()],
                    false,
                    egui::Color32::TRANSPARENT,
                    egui::Stroke::new(stroke_width, stroke_color),
                );
                painter.add(shape);
            }

            // If closed, draw closing segment
            if element.closed && element.vertices.len() >= 2 {
                let last = element.vertices.last().unwrap();
                let first = element.vertices.first().unwrap();
                let (p0, p1, p2, p3) = math::segment_bezier_points(last, first);

                let tp0 = transform_point(p0);
                let tp1 = transform_point(p1);
                let tp2 = transform_point(p2);
                let tp3 = transform_point(p3);

                let sp0 = viewport.world_to_screen(tp0, canvas_center);
                let sp1 = viewport.world_to_screen(tp1, canvas_center);
                let sp2 = viewport.world_to_screen(tp2, canvas_center);
                let sp3 = viewport.world_to_screen(tp3, canvas_center);

                let shape = egui::epaint::CubicBezierShape::from_points_stroke(
                    [sp0.into(), sp1.into(), sp2.into(), sp3.into()],
                    false,
                    egui::Color32::TRANSPARENT,
                    egui::Stroke::new(stroke_width, stroke_color),
                );
                painter.add(shape);
            }

            // Draw vertex dots
            let is_selected = editor_state.selection.is_element_selected(&element.id);
            for v in &element.vertices {
                let world_pos = transform_point(v.pos);
                let screen_pos = viewport.world_to_screen(world_pos, canvas_center);
                let dot_radius = if is_selected { 4.0 } else { 3.0 };
                let dot_color = if editor_state.selection.is_vertex_selected(&v.id) {
                    egui::Color32::from_rgb(0xff, 0xff, 0x00)
                } else if is_selected {
                    egui::Color32::WHITE
                } else {
                    stroke_color
                };

                painter.circle_filled(
                    egui::pos2(screen_pos.x, screen_pos.y),
                    dot_radius,
                    dot_color,
                );
            }
        }
    }
}

/// Render IK target elements as crosshair icons on the canvas.
fn render_ik_targets(
    painter: &egui::Painter,
    viewport: &crate::state::editor::ViewportState,
    canvas_center: Vec2,
    sprite: &Sprite,
    _current_theme: Theme,
) {
    let crosshair_color = egui::Color32::from_rgb(0xff, 0x80, 0x00); // Orange
    let crosshair_size = 12.0;
    let circle_radius = 8.0;

    for layer in &sprite.layers {
        if !layer.visible {
            continue;
        }

        for ik_target in &layer.ik_targets {
            let screen_pos = viewport.world_to_screen(ik_target.position, canvas_center);
            let center = egui::pos2(screen_pos.x, screen_pos.y);

            // Draw crosshair
            let stroke = egui::Stroke::new(1.5, crosshair_color);

            // Horizontal line
            painter.line_segment(
                [
                    egui::pos2(center.x - crosshair_size, center.y),
                    egui::pos2(center.x + crosshair_size, center.y),
                ],
                stroke,
            );

            // Vertical line
            painter.line_segment(
                [
                    egui::pos2(center.x, center.y - crosshair_size),
                    egui::pos2(center.x, center.y + crosshair_size),
                ],
                stroke,
            );

            // Circle
            painter.circle_stroke(center, circle_radius, stroke);

            // Label (if named)
            if let Some(ref name) = ik_target.name {
                painter.text(
                    egui::pos2(center.x + crosshair_size + 2.0, center.y - 6.0),
                    egui::Align2::LEFT_TOP,
                    name,
                    egui::FontId::proportional(10.0),
                    crosshair_color,
                );
            }
        }
    }
}

/// Render debug overlays: bone chains, constraint gizmos, spring targets.
fn render_debug_overlays(
    painter: &egui::Painter,
    viewport: &crate::state::editor::ViewportState,
    canvas_center: Vec2,
    sprite: &Sprite,
    overlays: &crate::state::editor::DebugOverlays,
    _current_theme: Theme,
) {
    // Bone chains: draw lines connecting socketed layers
    if overlays.show_bones {
        let bone_color = egui::Color32::from_rgb(0x00, 0xff, 0x80); // Green
        let bone_stroke = egui::Stroke::new(2.0, bone_color);

        for layer in &sprite.layers {
            if let Some(ref socket) = layer.socket {
                // Find parent vertex position
                let parent_pos = find_vertex_world_pos(sprite, &socket.parent_element_id, &socket.parent_vertex_id);
                // Find child layer origin
                let child_transform = socket_engine::resolve_socket_transform(sprite, &layer.id);
                let child_pos = child_transform.position;

                if let Some(parent) = parent_pos {
                    let screen_parent = viewport.world_to_screen(parent, canvas_center);
                    let screen_child = viewport.world_to_screen(child_pos, canvas_center);

                    painter.line_segment(
                        [
                            egui::pos2(screen_parent.x, screen_parent.y),
                            egui::pos2(screen_child.x, screen_child.y),
                        ],
                        bone_stroke,
                    );

                    // Joint dots
                    painter.circle_filled(
                        egui::pos2(screen_parent.x, screen_parent.y),
                        3.0,
                        bone_color,
                    );
                    painter.circle_filled(
                        egui::pos2(screen_child.x, screen_child.y),
                        3.0,
                        bone_color,
                    );
                }
            }
        }
    }

    // IK target overlay (enhanced with chain lines)
    if overlays.show_ik_targets {
        let ik_color = egui::Color32::from_rgb(0xff, 0x40, 0x40); // Red

        for layer in &sprite.layers {
            for ik_target in &layer.ik_targets {
                let screen_pos = viewport.world_to_screen(ik_target.position, canvas_center);

                // Draw a diamond around the target
                let size = 10.0;
                let center = egui::pos2(screen_pos.x, screen_pos.y);
                let points = vec![
                    egui::pos2(center.x, center.y - size),
                    egui::pos2(center.x + size, center.y),
                    egui::pos2(center.x, center.y + size),
                    egui::pos2(center.x - size, center.y),
                ];
                painter.add(egui::epaint::PathShape::closed_line(
                    points,
                    egui::Stroke::new(2.0, ik_color),
                ));
            }
        }
    }

    // Constraint gizmos: look-at direction arrows
    if overlays.show_constraints {
        let constraint_color = egui::Color32::from_rgb(0xff, 0xff, 0x00); // Yellow

        for layer in &sprite.layers {
            // Look-at: draw an arrow from the layer origin in the facing direction
            if let Some(ref look_at) = layer.constraints.look_at {
                if look_at.target_element_id.is_empty() {
                    continue;
                }
                // Get layer world position
                let transform = socket_engine::resolve_socket_transform(sprite, &layer.id);
                let origin = if let Some(first_elem) = layer.elements.first() {
                    Vec2::new(
                        transform.position.x + first_elem.position.x + first_elem.origin.x,
                        transform.position.y + first_elem.position.y + first_elem.origin.y,
                    )
                } else {
                    transform.position
                };
                let rotation = layer.elements.first().map(|e| e.rotation).unwrap_or(0.0);

                // Arrow from origin in the direction the element is facing
                let arrow_len = 30.0 / viewport.zoom; // Constant screen-space length
                let arrow_end = Vec2::new(
                    origin.x + rotation.cos() * arrow_len,
                    origin.y + rotation.sin() * arrow_len,
                );

                let screen_origin = viewport.world_to_screen(origin, canvas_center);
                let screen_end = viewport.world_to_screen(arrow_end, canvas_center);

                painter.line_segment(
                    [
                        egui::pos2(screen_origin.x, screen_origin.y),
                        egui::pos2(screen_end.x, screen_end.y),
                    ],
                    egui::Stroke::new(2.0, constraint_color),
                );

                // Arrowhead
                let dx = screen_end.x - screen_origin.x;
                let dy = screen_end.y - screen_origin.y;
                let len = (dx * dx + dy * dy).sqrt();
                if len > 1.0 {
                    let ux = dx / len;
                    let uy = dy / len;
                    let head_size = 6.0;
                    painter.line_segment(
                        [
                            egui::pos2(screen_end.x, screen_end.y),
                            egui::pos2(
                                screen_end.x - head_size * (ux + uy * 0.5),
                                screen_end.y - head_size * (uy - ux * 0.5),
                            ),
                        ],
                        egui::Stroke::new(2.0, constraint_color),
                    );
                    painter.line_segment(
                        [
                            egui::pos2(screen_end.x, screen_end.y),
                            egui::pos2(
                                screen_end.x - head_size * (ux - uy * 0.5),
                                screen_end.y - head_size * (uy + ux * 0.5),
                            ),
                        ],
                        egui::Stroke::new(2.0, constraint_color),
                    );
                }

                // Draw angle limits as arcs
                let limit_color = egui::Color32::from_rgba_unmultiplied(0xff, 0xff, 0x00, 80);
                let rest = look_at.rest_angle;
                let arc_radius = 20.0; // screen pixels
                let steps = 16;
                // Min angle arc
                let min_global = rest + look_at.min_angle;
                let max_global = rest + look_at.max_angle;
                let mut arc_points = Vec::new();
                for i in 0..=steps {
                    let t = i as f32 / steps as f32;
                    let a = min_global + (max_global - min_global) * t;
                    arc_points.push(egui::pos2(
                        screen_origin.x + arc_radius * a.cos(),
                        screen_origin.y + arc_radius * a.sin(),
                    ));
                }
                if arc_points.len() >= 2 {
                    for i in 0..arc_points.len() - 1 {
                        painter.line_segment(
                            [arc_points[i], arc_points[i + 1]],
                            egui::Stroke::new(1.0, limit_color),
                        );
                    }
                }
            }

            // Volume preserve indicator
            if layer.constraints.volume_preserve {
                let transform = socket_engine::resolve_socket_transform(sprite, &layer.id);
                let pos = transform.position;
                let screen_pos = viewport.world_to_screen(pos, canvas_center);
                painter.text(
                    egui::pos2(screen_pos.x - 5.0, screen_pos.y - 15.0),
                    egui::Align2::LEFT_TOP,
                    "VP",
                    egui::FontId::proportional(9.0),
                    egui::Color32::from_rgb(0x80, 0xff, 0xff),
                );
            }
        }
    }

    // Spring targets: show where the spring is targeting vs where the element actually is
    if overlays.show_spring_targets {
        let spring_color = egui::Color32::from_rgb(0xff, 0x80, 0xff); // Magenta

        for layer in &sprite.layers {
            if layer.constraints.physics.is_some() {
                let transform = socket_engine::resolve_socket_transform(sprite, &layer.id);
                let pos = if let Some(first_elem) = layer.elements.first() {
                    Vec2::new(
                        transform.position.x + first_elem.position.x,
                        transform.position.y + first_elem.position.y,
                    )
                } else {
                    transform.position
                };
                let screen_pos = viewport.world_to_screen(pos, canvas_center);

                // Draw a spring indicator (small circle with S label)
                painter.circle_stroke(
                    egui::pos2(screen_pos.x, screen_pos.y),
                    6.0,
                    egui::Stroke::new(1.5, spring_color),
                );
                painter.text(
                    egui::pos2(screen_pos.x + 8.0, screen_pos.y - 4.0),
                    egui::Align2::LEFT_TOP,
                    "S",
                    egui::FontId::proportional(9.0),
                    spring_color,
                );
            }
        }
    }
}

/// Find a specific vertex's world-space position on a specific element.
fn find_vertex_world_pos(sprite: &Sprite, element_id: &str, vertex_id: &str) -> Option<Vec2> {
    for layer in &sprite.layers {
        for element in &layer.elements {
            if element.id == element_id {
                for vertex in &element.vertices {
                    if vertex.id == vertex_id {
                        let st = socket_engine::resolve_socket_transform(sprite, &layer.id);
                        return Some(Vec2::new(
                            st.position.x + element.position.x + vertex.pos.x,
                            st.position.y + element.position.y + vertex.pos.y,
                        ));
                    }
                }
                return None;
            }
        }
    }
    None
}

/// Draw selection highlight outlines on selected elements
fn draw_selection_highlights(
    painter: &egui::Painter,
    sprite: &Sprite,
    editor_state: &EditorState,
    canvas_center: Vec2,
) {
    if editor_state.selection.selected_element_ids.is_empty() {
        return;
    }

    let highlight_color = egui::Color32::from_rgb(0x4a, 0xd0, 0xff);
    let highlight_stroke = egui::Stroke::new(2.0, highlight_color);

    for layer in &sprite.layers {
        if !layer.visible {
            continue;
        }

        // Resolve socket transform for highlights
        let socket_transform = socket_engine::resolve_socket_transform(sprite, &layer.id);
        let has_socket = layer.socket.is_some();

        let transform_point = |p: Vec2| -> Vec2 {
            if has_socket {
                apply_socket_transform(p, &socket_transform)
            } else {
                p
            }
        };

        for element in &layer.elements {
            if !editor_state.selection.is_element_selected(&element.id) {
                continue;
            }
            if element.vertices.len() < 2 {
                if let Some(v) = element.vertices.first() {
                    let wp = transform_point(v.pos);
                    let sp = editor_state.viewport.world_to_screen(wp, canvas_center);
                    painter.circle_stroke(
                        egui::pos2(sp.x, sp.y),
                        6.0,
                        highlight_stroke,
                    );
                }
                continue;
            }

            // Draw outline along segments
            for i in 0..element.vertices.len() - 1 {
                let (p0, p1, p2, p3) =
                    math::segment_bezier_points(&element.vertices[i], &element.vertices[i + 1]);

                let tp0 = transform_point(p0);
                let tp1 = transform_point(p1);
                let tp2 = transform_point(p2);
                let tp3 = transform_point(p3);

                let sp0 = editor_state.viewport.world_to_screen(tp0, canvas_center);
                let sp1 = editor_state.viewport.world_to_screen(tp1, canvas_center);
                let sp2 = editor_state.viewport.world_to_screen(tp2, canvas_center);
                let sp3 = editor_state.viewport.world_to_screen(tp3, canvas_center);

                let shape = egui::epaint::CubicBezierShape::from_points_stroke(
                    [sp0.into(), sp1.into(), sp2.into(), sp3.into()],
                    false,
                    egui::Color32::TRANSPARENT,
                    highlight_stroke,
                );
                painter.add(shape);
            }

            if element.closed && element.vertices.len() >= 2 {
                let last = element.vertices.last().unwrap();
                let first = element.vertices.first().unwrap();
                let (p0, p1, p2, p3) = math::segment_bezier_points(last, first);

                let tp0 = transform_point(p0);
                let tp1 = transform_point(p1);
                let tp2 = transform_point(p2);
                let tp3 = transform_point(p3);

                let sp0 = editor_state.viewport.world_to_screen(tp0, canvas_center);
                let sp1 = editor_state.viewport.world_to_screen(tp1, canvas_center);
                let sp2 = editor_state.viewport.world_to_screen(tp2, canvas_center);
                let sp3 = editor_state.viewport.world_to_screen(tp3, canvas_center);

                let shape = egui::epaint::CubicBezierShape::from_points_stroke(
                    [sp0.into(), sp1.into(), sp2.into(), sp3.into()],
                    false,
                    egui::Color32::TRANSPARENT,
                    highlight_stroke,
                );
                painter.add(shape);
            }
        }
    }
}

fn handle_viewport_controls(
    response: &egui::Response,
    editor_state: &mut EditorState,
    canvas_center: Vec2,
    ctx: &egui::Context,
) {
    // Middle-click pan
    if (response.middle_clicked() || (response.dragged_by(egui::PointerButton::Middle)))
        && let Some(pos) = response.hover_pos()
            && !editor_state.is_panning {
                editor_state.is_panning = true;
                editor_state.pan_start_pos = Some(Vec2::from(pos));
                editor_state.pan_start_offset = Some(editor_state.viewport.offset);
            }

    if editor_state.is_panning {
        if let (Some(start), Some(start_offset), Some(current)) = (
            editor_state.pan_start_pos,
            editor_state.pan_start_offset,
            ctx.input(|i| i.pointer.hover_pos()),
        ) {
            let delta = Vec2::new(
                (current.x - start.x) / editor_state.viewport.zoom,
                (current.y - start.y) / editor_state.viewport.zoom,
            );
            editor_state.viewport.offset = Vec2::new(
                start_offset.x + delta.x,
                start_offset.y + delta.y,
            );
        }

        if ctx.input(|i| !i.pointer.middle_down()) {
            editor_state.is_panning = false;
            editor_state.pan_start_pos = None;
            editor_state.pan_start_offset = None;
        }
    }

    // Scroll wheel zoom (centered on cursor)
    let scroll_delta = ctx.input(|i| i.raw_scroll_delta.y);
    if scroll_delta != 0.0 && response.hovered()
        && let Some(hover_pos) = response.hover_pos() {
            let hover_screen = Vec2::from(hover_pos);
            let world_before = editor_state
                .viewport
                .screen_to_world(hover_screen, canvas_center);

            let zoom_factor = if scroll_delta > 0.0 { 1.1 } else { 1.0 / 1.1 };
            let new_zoom = (editor_state.viewport.zoom * zoom_factor)
                .clamp(editor_state.viewport.zoom_min, editor_state.viewport.zoom_max);
            editor_state.viewport.zoom = new_zoom;

            // Adjust offset so the world point under cursor stays put
            let world_after = editor_state
                .viewport
                .screen_to_world(hover_screen, canvas_center);
            let correction = world_after - world_before;
            editor_state.viewport.offset = editor_state.viewport.offset + correction;
        }
}

#[allow(clippy::too_many_arguments)]
fn handle_line_tool(
    response: &egui::Response,
    sprite: &mut Sprite,
    editor_state: &mut EditorState,
    canvas_center: Vec2,
    grid_size: f32,
    painter: &egui::Painter,
    current_theme: Theme,
    history: &mut History,
    sprite_index: usize,
    actions: &mut Vec<CanvasAction>,
    grid_mode: GridMode,
) {
    let layer_index = editor_state.active_layer_index;

    // Validate layer index
    if layer_index >= sprite.layers.len() {
        return;
    }

    // Don't draw on locked layers
    if sprite.layers[layer_index].locked {
        return;
    }

    // Get snapped cursor position
    let snapped_world_pos = editor_state.cursor_world_pos.map(|pos| {
        engine::snap_to_grid(pos, grid_size, grid_mode)
    });

    // Draw preview line from last vertex to cursor
    if let Some(ref active_id) = editor_state.line_tool_state.active_element_id.clone()
        && let Some(snapped_pos) = snapped_world_pos
            && let Some(element) = sprite.layers[layer_index]
                .elements
                .iter()
                .find(|e| &e.id == active_id)
                && let Some(last_vertex) = element.vertices.last() {
                    let screen_from = editor_state
                        .viewport
                        .world_to_screen(last_vertex.pos, canvas_center);
                    let screen_to = editor_state
                        .viewport
                        .world_to_screen(snapped_pos, canvas_center);

                    let preview_color = theme::secondary_color(current_theme);
                    painter.line_segment(
                        [
                            egui::pos2(screen_from.x, screen_from.y),
                            egui::pos2(screen_to.x, screen_to.y),
                        ],
                        egui::Stroke::new(1.0, preview_color),
                    );

                    // Draw snapped cursor indicator
                    painter.circle_stroke(
                        egui::pos2(screen_to.x, screen_to.y),
                        5.0,
                        egui::Stroke::new(1.0, preview_color),
                    );
                }

    // Handle click to place vertex
    if response.clicked()
        && let Some(snapped_pos) = snapped_world_pos {
            // Check for double-click (finishing the element)
            let is_double_click = if let Some(ref active_id) = editor_state.line_tool_state.active_element_id {
                sprite.layers[layer_index]
                    .elements
                    .iter()
                    .find(|e| &e.id == active_id)
                    .and_then(|e| e.vertices.last())
                    .map(|v| v.pos.distance(snapped_pos) < 0.01)
                    .unwrap_or(false)
            } else {
                false
            };

            if is_double_click {
                // Finish element
                finish_line_element(sprite, editor_state, layer_index, history, sprite_index);
                actions.push(CanvasAction::SpriteChanged);
                return;
            }

            let before = sprite.clone();

            // Place a new vertex
            let new_vertex = PathVertex::new(snapped_pos);

            if let Some(ref active_id) = editor_state.line_tool_state.active_element_id.clone() {
                // Add vertex to existing element
                if let Some(element) = sprite.layers[layer_index]
                    .elements
                    .iter_mut()
                    .find(|e| &e.id == active_id)
                {
                    element.vertices.push(new_vertex);
                    if editor_state.curve_mode {
                        math::recompute_auto_curves(&mut element.vertices, element.closed);
                    }
                    // If straight mode, ensure no control points
                    // (vertices already start with cp1=None, cp2=None)
                }
            } else {
                // Start a new element
                let mut element = StrokeElement::new();
                element.stroke_width = editor_state.stroke_width;
                element.stroke_color_index = editor_state.active_color_index;
                let elem_id = element.id.clone();
                element.vertices.push(new_vertex);
                sprite.layers[layer_index].elements.push(element);
                editor_state.line_tool_state.active_element_id = Some(elem_id);
            }

            history.push(SnapshotCommand {
                description: "Place vertex".to_string(),
                sprite_index,
                before,
                after: sprite.clone(),
            });
            actions.push(CanvasAction::SpriteChanged);
        }

    // Double-click to finish element (egui double_clicked)
    if response.double_clicked() {
        finish_line_element(sprite, editor_state, layer_index, history, sprite_index);
        actions.push(CanvasAction::SpriteChanged);
    }
}

fn finish_line_element(
    sprite: &mut Sprite,
    editor_state: &mut EditorState,
    layer_index: usize,
    _history: &mut History,
    _sprite_index: usize,
) {
    if let Some(ref active_id) = editor_state.line_tool_state.active_element_id {
        // Remove the element if it has fewer than 2 vertices
        if let Some(element) = sprite.layers[layer_index]
            .elements
            .iter()
            .find(|e| &e.id == active_id)
            && element.vertices.len() < 2 {
                let id = active_id.clone();
                sprite.layers[layer_index]
                    .elements
                    .retain(|e| e.id != id);
            }
    }
    editor_state.line_tool_state.active_element_id = None;
}

/// Handle the Select tool interactions
#[allow(clippy::too_many_arguments)]
fn handle_select_tool(
    ctx: &egui::Context,
    response: &egui::Response,
    sprite: &mut Sprite,
    editor_state: &mut EditorState,
    canvas_center: Vec2,
    _painter: &egui::Painter,
    _current_theme: Theme,
    history: &mut History,
    sprite_index: usize,
    actions: &mut Vec<CanvasAction>,
) {
    let shift_held = ctx.input(|i| i.modifiers.shift);

    // Handle drag for moving selected elements, IK targets, handles, or marquee selection
    if response.drag_started_by(egui::PointerButton::Primary)
        && let Some(pos) = response.hover_pos() {
            let world_pos = editor_state.viewport.screen_to_world(Vec2::from(pos), canvas_center);

            // First check if clicking on a transform handle (scale/rotate)
            let transform_hit = hit_test_transform_handle(
                sprite, world_pos, editor_state, canvas_center,
            );
            if let Some((kind, handle_idx, pivot)) = transform_hit {
                editor_state.transform_handle.is_dragging = true;
                editor_state.transform_handle.kind = kind;
                editor_state.transform_handle.handle_index = handle_idx;
                editor_state.transform_handle.start_world = Some(world_pos);
                editor_state.transform_handle.pivot = Some(pivot);
                editor_state.transform_handle.before_snapshot = Some(sprite.clone());
                if kind == crate::state::editor::TransformHandleKind::Rotate {
                    let diff = world_pos - pivot;
                    editor_state.transform_handle.start_angle = diff.y.atan2(diff.x);
                }
            }
            // Then check if clicking on a curve handle (cp1/cp2)
            else if let Some((elem_id, vtx_id, is_cp1, handle_pos)) = hit_test_handle(sprite, world_pos, editor_state) {
                editor_state.handle_drag.is_dragging = true;
                editor_state.handle_drag.element_id = Some(elem_id);
                editor_state.handle_drag.vertex_id = Some(vtx_id);
                editor_state.handle_drag.is_cp1 = is_cp1;
                editor_state.handle_drag.original_pos = Some(handle_pos);
            }
            // Then check if clicking on an IK target
            else if let Some((target_id, target_pos)) = hit_test_ik_target(sprite, world_pos, &editor_state.viewport, canvas_center) {
                editor_state.dragging_ik_target = Some(target_id);
                editor_state.ik_target_drag_start = Some(target_pos);
            } else {
                // Check if clicking on a selected element to start dragging
                let hit = hit_test_all_visible_layers(sprite, world_pos, editor_state);
                if let Some(ref hit_result) = hit {
                    if editor_state.selection.is_element_selected(&hit_result.element_id) {
                        // Start dragging selected elements
                        editor_state.select_drag.is_dragging = true;
                        editor_state.select_drag.drag_start_world = Some(world_pos);
                        editor_state.select_drag.drag_last_world = Some(world_pos);
                    } else {
                        // Start marquee
                        editor_state.marquee.is_active = true;
                        editor_state.marquee.start_world = Some(world_pos);
                        editor_state.marquee.current_world = Some(world_pos);
                    }
                } else {
                    // Start marquee
                    editor_state.marquee.is_active = true;
                    editor_state.marquee.start_world = Some(world_pos);
                    editor_state.marquee.current_world = Some(world_pos);
                }
            }
        }

    // Update drag/marquee during drag
    if response.dragged_by(egui::PointerButton::Primary)
        && let Some(pos) = response.hover_pos() {
            let world_pos = editor_state.viewport.screen_to_world(Vec2::from(pos), canvas_center);

            if editor_state.transform_handle.is_dragging {
                // Scale/rotate transform handle drag
                if let Some(pivot) = editor_state.transform_handle.pivot {
                    match editor_state.transform_handle.kind {
                        crate::state::editor::TransformHandleKind::Scale => {
                            if let Some(start) = editor_state.transform_handle.start_world {
                                let start_dist = (start - pivot).length().max(1.0);
                                let curr_dist = (world_pos - pivot).length().max(1.0);
                                let scale_factor = curr_dist / start_dist;
                                // Apply scale to all selected elements
                                for layer in &mut sprite.layers {
                                    for element in &mut layer.elements {
                                        if editor_state.selection.selected_element_ids.iter().any(|id| id == &element.id) {
                                            for vertex in &mut element.vertices {
                                                vertex.pos = Vec2::new(
                                                    pivot.x + (vertex.pos.x - pivot.x) * scale_factor,
                                                    pivot.y + (vertex.pos.y - pivot.y) * scale_factor,
                                                );
                                                if let Some(ref mut cp1) = vertex.cp1 {
                                                    *cp1 = Vec2::new(
                                                        pivot.x + (cp1.x - pivot.x) * scale_factor,
                                                        pivot.y + (cp1.y - pivot.y) * scale_factor,
                                                    );
                                                }
                                                if let Some(ref mut cp2) = vertex.cp2 {
                                                    *cp2 = Vec2::new(
                                                        pivot.x + (cp2.x - pivot.x) * scale_factor,
                                                        pivot.y + (cp2.y - pivot.y) * scale_factor,
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                                editor_state.transform_handle.start_world = Some(world_pos);
                            }
                        }
                        crate::state::editor::TransformHandleKind::Rotate => {
                            let diff = world_pos - pivot;
                            let current_angle = diff.y.atan2(diff.x);
                            let delta_angle = current_angle - editor_state.transform_handle.start_angle;
                            let cos_a = delta_angle.cos();
                            let sin_a = delta_angle.sin();
                            for layer in &mut sprite.layers {
                                for element in &mut layer.elements {
                                    if editor_state.selection.selected_element_ids.iter().any(|id| id == &element.id) {
                                        for vertex in &mut element.vertices {
                                            let dx = vertex.pos.x - pivot.x;
                                            let dy = vertex.pos.y - pivot.y;
                                            vertex.pos = Vec2::new(
                                                pivot.x + dx * cos_a - dy * sin_a,
                                                pivot.y + dx * sin_a + dy * cos_a,
                                            );
                                            if let Some(ref mut cp1) = vertex.cp1 {
                                                let dx = cp1.x - pivot.x;
                                                let dy = cp1.y - pivot.y;
                                                *cp1 = Vec2::new(
                                                    pivot.x + dx * cos_a - dy * sin_a,
                                                    pivot.y + dx * sin_a + dy * cos_a,
                                                );
                                            }
                                            if let Some(ref mut cp2) = vertex.cp2 {
                                                let dx = cp2.x - pivot.x;
                                                let dy = cp2.y - pivot.y;
                                                *cp2 = Vec2::new(
                                                    pivot.x + dx * cos_a - dy * sin_a,
                                                    pivot.y + dx * sin_a + dy * cos_a,
                                                );
                                            }
                                        }
                                        element.rotation += delta_angle;
                                    }
                                }
                            }
                            editor_state.transform_handle.start_angle = current_angle;
                        }
                        _ => {}
                    }
                }
            } else if editor_state.handle_drag.is_dragging {
                // Drag curve handle
                if let (Some(ref elem_id), Some(ref vtx_id)) =
                    (editor_state.handle_drag.element_id.clone(), editor_state.handle_drag.vertex_id.clone())
                {
                    let is_cp1 = editor_state.handle_drag.is_cp1;
                    for layer in &mut sprite.layers {
                        for element in &mut layer.elements {
                            if element.id == *elem_id
                                && let Some(vertex) = element.vertices.iter_mut().find(|v| v.id == *vtx_id) {
                                    if is_cp1 {
                                        vertex.cp1 = Some(world_pos);
                                    } else {
                                        vertex.cp2 = Some(world_pos);
                                    }
                                }
                        }
                    }
                }
            } else if let Some(ref target_id) = editor_state.dragging_ik_target.clone() {
                // Drag IK target
                for layer in &mut sprite.layers {
                    if let Some(ik_target) = layer.ik_targets.iter_mut().find(|t| t.id == *target_id) {
                        ik_target.position = world_pos;
                    }
                }
            } else if editor_state.select_drag.is_dragging {
                // Move selected elements
                if let Some(last_pos) = editor_state.select_drag.drag_last_world {
                    let delta = world_pos - last_pos;
                    move_selected_elements(sprite, &editor_state.selection.selected_element_ids, delta);
                }
                editor_state.select_drag.drag_last_world = Some(world_pos);
            } else if editor_state.marquee.is_active {
                editor_state.marquee.current_world = Some(world_pos);
            }
        }

    // End drag/marquee
    if response.drag_stopped_by(egui::PointerButton::Primary) {
        if editor_state.transform_handle.is_dragging {
            // Commit transform with undo
            if let Some(before) = editor_state.transform_handle.before_snapshot.take() {
                let after = sprite.clone();
                history.push(SnapshotCommand {
                    description: match editor_state.transform_handle.kind {
                        crate::state::editor::TransformHandleKind::Scale => "Scale elements".to_string(),
                        crate::state::editor::TransformHandleKind::Rotate => "Rotate elements".to_string(),
                        _ => "Transform elements".to_string(),
                    },
                    sprite_index,
                    before,
                    after,
                });
                actions.push(CanvasAction::SpriteChanged);
            }
            editor_state.transform_handle = crate::state::editor::TransformHandleState::default();
        } else if editor_state.handle_drag.is_dragging {
            // Commit handle move with undo
            if let Some(original_pos) = editor_state.handle_drag.original_pos {
                let after = sprite.clone();
                let mut before_sprite = sprite.clone();
                // Restore original handle position in before_sprite
                if let (Some(ref elem_id), Some(ref vtx_id)) =
                    (editor_state.handle_drag.element_id.clone(), editor_state.handle_drag.vertex_id.clone())
                {
                    let is_cp1 = editor_state.handle_drag.is_cp1;
                    for layer in &mut before_sprite.layers {
                        for element in &mut layer.elements {
                            if element.id == *elem_id
                                && let Some(vertex) = element.vertices.iter_mut().find(|v| v.id == *vtx_id) {
                                    if is_cp1 {
                                        vertex.cp1 = Some(original_pos);
                                    } else {
                                        vertex.cp2 = Some(original_pos);
                                    }
                                }
                        }
                    }
                }
                history.push(SnapshotCommand {
                    description: "Move curve handle".to_string(),
                    sprite_index,
                    before: before_sprite,
                    after,
                });
                actions.push(CanvasAction::SpriteChanged);
            }
            editor_state.handle_drag = HandleDragState::default();
        } else if editor_state.dragging_ik_target.is_some() {
            // Commit IK target move with undo
            if let Some(start_pos) = editor_state.ik_target_drag_start {
                // Build before/after for undo
                let after = sprite.clone();
                let mut before_sprite = sprite.clone();
                // Reset the target position in before_sprite
                if let Some(ref target_id) = editor_state.dragging_ik_target {
                    for layer in &mut before_sprite.layers {
                        if let Some(ik_target) = layer.ik_targets.iter_mut().find(|t| t.id == *target_id) {
                            ik_target.position = start_pos;
                        }
                    }
                }
                history.push(SnapshotCommand {
                    description: "Move IK target".to_string(),
                    sprite_index,
                    before: before_sprite,
                    after,
                });
                actions.push(CanvasAction::SpriteChanged);
            }
            editor_state.dragging_ik_target = None;
            editor_state.ik_target_drag_start = None;
        } else if editor_state.select_drag.is_dragging {
            // Commit the move with undo
            if let (Some(start), Some(end)) = (
                editor_state.select_drag.drag_start_world,
                editor_state.select_drag.drag_last_world,
            ) {
                let total_delta = end - start;
                if total_delta.length() > 0.01 {
                    // We already moved the elements during drag, so we need the before state
                    // Undo: move elements back by total_delta, then save as before
                    let after = sprite.clone();
                    let mut before_sprite = sprite.clone();
                    let neg_delta = -total_delta;
                    move_selected_elements(&mut before_sprite, &editor_state.selection.selected_element_ids, neg_delta);
                    history.push(SnapshotCommand {
                        description: "Move elements".to_string(),
                        sprite_index,
                        before: before_sprite,
                        after,
                    });
                    actions.push(CanvasAction::SpriteChanged);
                }
            }
            editor_state.select_drag.is_dragging = false;
            editor_state.select_drag.drag_start_world = None;
            editor_state.select_drag.drag_last_world = None;
        } else if editor_state.marquee.is_active {
            // Finalize marquee selection
            if let (Some(start), Some(end)) = (
                editor_state.marquee.start_world,
                editor_state.marquee.current_world,
            ) {
                let min_x = start.x.min(end.x);
                let max_x = start.x.max(end.x);
                let min_y = start.y.min(end.y);
                let max_y = start.y.max(end.y);

                // Only select if marquee has some area
                if (max_x - min_x) > 1.0 || (max_y - min_y) > 1.0 {
                    if !shift_held {
                        editor_state.selection.clear();
                    }

                    for layer in &sprite.layers {
                        if !layer.visible || layer.locked {
                            continue;
                        }
                        for element in &layer.elements {
                            // Check if any vertex is inside the marquee
                            let inside = element.vertices.iter().any(|v| {
                                v.pos.x >= min_x && v.pos.x <= max_x
                                    && v.pos.y >= min_y && v.pos.y <= max_y
                            });
                            if inside {
                                editor_state.selection.select_element(&element.id);
                            }
                        }
                    }
                }
            }
            editor_state.marquee.is_active = false;
            editor_state.marquee.start_world = None;
            editor_state.marquee.current_world = None;
        }
    }

    // Handle click (non-drag) for selection
    if response.clicked() && !editor_state.select_drag.is_dragging && !editor_state.marquee.is_active
        && let Some(pos) = response.hover_pos() {
            let world_pos = editor_state.viewport.screen_to_world(Vec2::from(pos), canvas_center);

            let hit = hit_test_all_visible_layers(sprite, world_pos, editor_state);

            if let Some(hit_result) = hit {
                if shift_held {
                    editor_state.selection.toggle_element(&hit_result.element_id);
                } else {
                    editor_state.selection.clear();
                    editor_state.selection.select_element(&hit_result.element_id);
                }
            } else if !shift_held {
                editor_state.selection.clear();
            }
        }
}

/// Hit test across all visible, unlocked layers
fn hit_test_all_visible_layers(
    sprite: &Sprite,
    world_pos: Vec2,
    editor_state: &EditorState,
) -> Option<engine::hit_test::HitResult> {
    let threshold = 10.0 / editor_state.viewport.zoom;
    let mut best: Option<engine::hit_test::HitResult> = None;

    for layer in &sprite.layers {
        if !layer.visible || layer.locked {
            continue;
        }
        if let Some(hit) = engine::hit_test_elements(world_pos, &layer.elements, threshold)
            && best.as_ref().is_none_or(|b| hit.distance < b.distance) {
                best = Some(hit);
            }
    }

    best
}

/// Hit test IK targets: check if a world position is near any IK target element.
/// Returns the target ID and its current position if hit.
fn hit_test_ik_target(
    sprite: &Sprite,
    world_pos: Vec2,
    _viewport: &crate::state::editor::ViewportState,
    _canvas_center: Vec2,
) -> Option<(String, Vec2)> {
    let hit_radius = 15.0; // World-space hit radius for IK targets

    for layer in &sprite.layers {
        if !layer.visible {
            continue;
        }
        for ik_target in &layer.ik_targets {
            let dist = world_pos.distance(ik_target.position);
            if dist < hit_radius {
                return Some((ik_target.id.clone(), ik_target.position));
            }
        }
    }

    None
}

/// Hit-test transform handles (scale corners + rotation circle).
/// Returns (TransformHandleKind, handle_index, pivot) if a handle is hit.
fn hit_test_transform_handle(
    sprite: &Sprite,
    world_pos: Vec2,
    editor_state: &EditorState,
    canvas_center: Vec2,
) -> Option<(crate::state::editor::TransformHandleKind, usize, Vec2)> {
    if editor_state.selection.selected_element_ids.is_empty() {
        return None;
    }
    let (bb_min, bb_max) = compute_selection_bounds(sprite, &editor_state.selection.selected_element_ids)?;

    let pivot = Vec2::new((bb_min.x + bb_max.x) / 2.0, (bb_min.y + bb_max.y) / 2.0);
    let hit_radius = 8.0 / editor_state.viewport.zoom;

    // Check corner handles (scale)
    let corners = [
        bb_min,
        Vec2::new(bb_max.x, bb_min.y),
        bb_max,
        Vec2::new(bb_min.x, bb_max.y),
    ];
    for (i, corner) in corners.iter().enumerate() {
        if world_pos.distance(*corner) < hit_radius {
            return Some((crate::state::editor::TransformHandleKind::Scale, i, pivot));
        }
    }

    // Check rotation handle (above top-center, in screen space offset)
    let top_center = Vec2::new((bb_min.x + bb_max.x) / 2.0, bb_min.y);
    let top_center_screen = editor_state.viewport.world_to_screen(top_center, canvas_center);
    let rot_handle_screen = Vec2::new(top_center_screen.x, top_center_screen.y - 20.0);
    let rot_handle_world = editor_state.viewport.screen_to_world(rot_handle_screen, canvas_center);
    if world_pos.distance(rot_handle_world) < hit_radius {
        return Some((crate::state::editor::TransformHandleKind::Rotate, 0, pivot));
    }

    None
}

/// Hit-test curve control point handles.
/// Returns (element_id, vertex_id, is_cp1, handle_world_pos) if a handle is hit.
fn hit_test_handle(
    sprite: &Sprite,
    world_pos: Vec2,
    editor_state: &EditorState,
) -> Option<(String, String, bool, Vec2)> {
    let hit_radius = 8.0 / editor_state.viewport.zoom;

    for layer in &sprite.layers {
        if !layer.visible {
            continue;
        }
        for element in &layer.elements {
            if !editor_state.selection.is_element_selected(&element.id) {
                continue;
            }
            for vertex in &element.vertices {
                if let Some(cp1) = vertex.cp1
                    && world_pos.distance(cp1) < hit_radius {
                        return Some((element.id.clone(), vertex.id.clone(), true, cp1));
                    }
                if let Some(cp2) = vertex.cp2
                    && world_pos.distance(cp2) < hit_radius {
                        return Some((element.id.clone(), vertex.id.clone(), false, cp2));
                    }
            }
        }
    }
    None
}

/// Compute the bounding box of all selected elements (in world space).
/// Returns (min, max) corners, or None if no selected elements have vertices.
fn compute_selection_bounds(sprite: &Sprite, selected_ids: &[String]) -> Option<(Vec2, Vec2)> {
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    let mut found = false;

    for layer in &sprite.layers {
        for element in &layer.elements {
            if !selected_ids.iter().any(|id| id == &element.id) {
                continue;
            }
            for vertex in &element.vertices {
                min_x = min_x.min(vertex.pos.x);
                min_y = min_y.min(vertex.pos.y);
                max_x = max_x.max(vertex.pos.x);
                max_y = max_y.max(vertex.pos.y);
                found = true;
            }
        }
    }

    if found {
        Some((Vec2::new(min_x, min_y), Vec2::new(max_x, max_y)))
    } else {
        None
    }
}

/// Move the specified elements by delta
fn move_selected_elements(sprite: &mut Sprite, selected_ids: &[String], delta: Vec2) {
    for layer in &mut sprite.layers {
        for element in &mut layer.elements {
            if selected_ids.iter().any(|id| id == &element.id) {
                for vertex in &mut element.vertices {
                    vertex.pos = vertex.pos + delta;
                    if let Some(ref mut cp1) = vertex.cp1 {
                        *cp1 = *cp1 + delta;
                    }
                    if let Some(ref mut cp2) = vertex.cp2 {
                        *cp2 = *cp2 + delta;
                    }
                }
            }
        }
    }
}

/// Handle the Fill tool
#[allow(clippy::too_many_arguments)]
fn handle_fill_tool(
    response: &egui::Response,
    sprite: &mut Sprite,
    editor_state: &mut EditorState,
    canvas_center: Vec2,
    _grid_size: f32,
    history: &mut History,
    sprite_index: usize,
    actions: &mut Vec<CanvasAction>,
    _grid_mode: GridMode,
) {
    if !response.clicked() {
        return;
    }

    let Some(pos) = response.hover_pos() else {
        return;
    };

    let world_pos = editor_state.viewport.screen_to_world(Vec2::from(pos), canvas_center);
    // Try to hit inside an element polygon
    let mut hit_closed = false;
    let before = sprite.clone();

    for layer in &mut sprite.layers {
        if !layer.visible || layer.locked {
            continue;
        }
        for element in &mut layer.elements {
            if element.vertices.len() < 3 {
                continue;
            }
            // Flatten the element into a polygon and do point-in-polygon test
            let mut polygon = Vec::new();
            let vert_count = element.vertices.len();
            for i in 0..vert_count {
                let next = if element.closed { (i + 1) % vert_count } else if i + 1 < vert_count { i + 1 } else { break };
                let (p0, p1, p2, p3) =
                    math::segment_bezier_points(&element.vertices[i], &element.vertices[next]);
                math::flatten_cubic_bezier(p0, p1, p2, p3, 0.5, &mut polygon);
            }
            if polygon.len() >= 3 && math::point_in_polygon(world_pos, &polygon) {
                if !element.closed {
                    element.closed = true;
                }
                element.fill_color_index = editor_state.active_color_index;
                hit_closed = true;
                break;
            }
        }
        if hit_closed {
            break;
        }
    }

    if hit_closed {
        history.push(SnapshotCommand {
            description: "Fill element".to_string(),
            sprite_index,
            before,
            after: sprite.clone(),
        });
        actions.push(CanvasAction::SpriteChanged);
    } else {
        // Fill the background
        let before = sprite.clone();
        sprite.background_color_index = editor_state.active_color_index;
        history.push(SnapshotCommand {
            description: "Set background color".to_string(),
            sprite_index,
            before,
            after: sprite.clone(),
        });
        actions.push(CanvasAction::SpriteChanged);
    }
}

/// Handle the Eraser tool
#[allow(clippy::too_many_arguments)]
fn handle_eraser_tool(
    response: &egui::Response,
    sprite: &mut Sprite,
    editor_state: &mut EditorState,
    canvas_center: Vec2,
    _grid_size: f32,
    history: &mut History,
    sprite_index: usize,
    actions: &mut Vec<CanvasAction>,
    _grid_mode: GridMode,
) {
    if !response.clicked() {
        return;
    }

    let Some(pos) = response.hover_pos() else {
        return;
    };

    let world_pos = editor_state.viewport.screen_to_world(Vec2::from(pos), canvas_center);
    let threshold = 10.0 / editor_state.viewport.zoom;
    let layer_index = editor_state.active_layer_index;

    if layer_index >= sprite.layers.len() || sprite.layers[layer_index].locked {
        return;
    }

    // Find the nearest vertex to erase
    let mut best_vertex: Option<(String, String, f32)> = None; // (element_id, vertex_id, dist)

    for element in &sprite.layers[layer_index].elements {
        for vertex in &element.vertices {
            let d = world_pos.distance(vertex.pos);
            if d <= threshold
                && best_vertex.as_ref().is_none_or(|(_, _, bd)| d < *bd) {
                    best_vertex = Some((element.id.clone(), vertex.id.clone(), d));
                }
        }
    }

    let Some((element_id, vertex_id, _)) = best_vertex else {
        return;
    };

    let before = sprite.clone();

    // Check if this vertex is a socket parent for any layers.
    // If so, detach those child layers before deleting the vertex.
    let child_layer_ids = socket_engine::find_child_layers_for_vertex(sprite, &element_id, &vertex_id);
    if !child_layer_ids.is_empty() {
        // Show a toast warning
        let child_names: Vec<String> = child_layer_ids.iter().filter_map(|lid| {
            sprite.layers.iter().find(|l| l.id == *lid).map(|l| l.name.clone())
        }).collect();
        editor_state.toast = Some(crate::state::editor::ToastMessage {
            text: format!("Detached socket children: {}", child_names.join(", ")),
            created: std::time::Instant::now(),
        });

        // Detach each child layer to world-space position
        for child_id in &child_layer_ids {
            socket_engine::detach_layer_to_world_space(sprite, child_id);
        }
    }

    // Find the element and vertex index
    let elem_idx = sprite.layers[layer_index]
        .elements
        .iter()
        .position(|e| e.id == element_id);

    let Some(elem_idx) = elem_idx else { return };

    let vert_idx = sprite.layers[layer_index].elements[elem_idx]
        .vertices
        .iter()
        .position(|v| v.id == vertex_id);

    let Some(vert_idx) = vert_idx else { return };

    let element = &sprite.layers[layer_index].elements[elem_idx];
    let num_vertices = element.vertices.len();

    if num_vertices <= 1 {
        // Remove the entire element - also detach any layers socketed to this element
        let elem_children = socket_engine::find_child_layers_for_element(sprite, &element_id);
        for child_id in &elem_children {
            if !child_layer_ids.contains(child_id) {
                // Only detach if not already handled above
                socket_engine::detach_layer_to_world_space(sprite, child_id);
            }
        }
        sprite.layers[layer_index].elements.remove(elem_idx);
    } else if num_vertices == 2 {
        // Removing one vertex from a 2-vertex element leaves 1 vertex - remove the element
        let elem_children = socket_engine::find_child_layers_for_element(sprite, &element_id);
        for child_id in &elem_children {
            if !child_layer_ids.contains(child_id) {
                socket_engine::detach_layer_to_world_space(sprite, child_id);
            }
        }
        sprite.layers[layer_index].elements.remove(elem_idx);
    } else if element.closed {
        // For closed elements, removing a vertex just opens the path at that point
        let elem = &mut sprite.layers[layer_index].elements[elem_idx];
        elem.vertices.remove(vert_idx);
        if elem.vertices.len() < 3 {
            elem.closed = false;
        }
    } else if vert_idx == 0 || vert_idx == num_vertices - 1 {
        // Removing from start or end - just remove the vertex
        let elem = &mut sprite.layers[layer_index].elements[elem_idx];
        elem.vertices.remove(vert_idx);
    } else {
        // Removing a middle vertex splits the path into two elements
        let original = sprite.layers[layer_index].elements[elem_idx].clone();

        // First part: vertices 0..vert_idx
        let mut elem1 = StrokeElement::new();
        elem1.stroke_width = original.stroke_width;
        elem1.stroke_color_index = original.stroke_color_index;
        elem1.fill_color_index = original.fill_color_index;
        elem1.position = original.position;
        elem1.rotation = original.rotation;
        elem1.scale = original.scale;
        elem1.origin = original.origin;
        elem1.vertices = original.vertices[0..vert_idx].to_vec();

        // Second part: vertices (vert_idx+1)..
        let mut elem2 = StrokeElement::new();
        elem2.stroke_width = original.stroke_width;
        elem2.stroke_color_index = original.stroke_color_index;
        elem2.fill_color_index = original.fill_color_index;
        elem2.position = original.position;
        elem2.rotation = original.rotation;
        elem2.scale = original.scale;
        elem2.origin = original.origin;
        elem2.vertices = original.vertices[(vert_idx + 1)..].to_vec();

        // Remove original and add the two parts
        sprite.layers[layer_index].elements.remove(elem_idx);
        if elem1.vertices.len() >= 2 {
            sprite.layers[layer_index].elements.push(elem1);
        }
        if elem2.vertices.len() >= 2 {
            sprite.layers[layer_index].elements.push(elem2);
        }
    }

    history.push(SnapshotCommand {
        description: "Erase vertex".to_string(),
        sprite_index,
        before,
        after: sprite.clone(),
    });
    actions.push(CanvasAction::SpriteChanged);
}

fn draw_curve_handles(
    painter: &egui::Painter,
    sprite: &Sprite,
    editor_state: &EditorState,
    canvas_center: Vec2,
    _current_theme: Theme,
) {
    let handle_color = egui::Color32::from_rgb(0x4a, 0x7a, 0x96);
    let handle_line_color = egui::Color32::from_rgba_unmultiplied(0x4a, 0x7a, 0x96, 128);

    for layer in &sprite.layers {
        if !layer.visible {
            continue;
        }

        // Resolve socket transform for curve handles
        let socket_transform = socket_engine::resolve_socket_transform(sprite, &layer.id);
        let has_socket = layer.socket.is_some();

        let transform_point = |p: Vec2| -> Vec2 {
            if has_socket {
                apply_socket_transform(p, &socket_transform)
            } else {
                p
            }
        };

        for element in &layer.elements {
            let is_selected = editor_state.selection.is_element_selected(&element.id);
            if !is_selected {
                continue;
            }

            for vertex in &element.vertices {
                let vertex_world = transform_point(vertex.pos);
                let vertex_screen = editor_state
                    .viewport
                    .world_to_screen(vertex_world, canvas_center);

                // Draw cp1 handle (incoming)
                if let Some(cp1) = vertex.cp1 {
                    let cp1_world = transform_point(cp1);
                    let cp1_screen = editor_state
                        .viewport
                        .world_to_screen(cp1_world, canvas_center);
                    painter.line_segment(
                        [
                            egui::pos2(vertex_screen.x, vertex_screen.y),
                            egui::pos2(cp1_screen.x, cp1_screen.y),
                        ],
                        egui::Stroke::new(1.0, handle_line_color),
                    );
                    painter.circle_filled(
                        egui::pos2(cp1_screen.x, cp1_screen.y),
                        3.0,
                        handle_color,
                    );
                }

                // Draw cp2 handle (outgoing)
                if let Some(cp2) = vertex.cp2 {
                    let cp2_world = transform_point(cp2);
                    let cp2_screen = editor_state
                        .viewport
                        .world_to_screen(cp2_world, canvas_center);
                    painter.line_segment(
                        [
                            egui::pos2(vertex_screen.x, vertex_screen.y),
                            egui::pos2(cp2_screen.x, cp2_screen.y),
                        ],
                        egui::Stroke::new(1.0, handle_line_color),
                    );
                    painter.circle_filled(
                        egui::pos2(cp2_screen.x, cp2_screen.y),
                        3.0,
                        handle_color,
                    );
                }
            }
        }
    }
}

/// Draw scale/rotate transform handles around the selection bounding box.
fn draw_transform_handles(
    painter: &egui::Painter,
    sprite: &Sprite,
    editor_state: &EditorState,
    canvas_center: Vec2,
) {
    let Some((bb_min, bb_max)) = compute_selection_bounds(sprite, &editor_state.selection.selected_element_ids) else {
        return;
    };

    let handle_size = 6.0;
    let handle_color = egui::Color32::WHITE;
    let handle_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(0x4a, 0x7a, 0x96));
    let box_color = egui::Color32::from_rgba_unmultiplied(0x4a, 0x7a, 0x96, 128);

    // Convert bounding box corners to screen space
    let tl = editor_state.viewport.world_to_screen(bb_min, canvas_center);
    let br = editor_state.viewport.world_to_screen(bb_max, canvas_center);
    let tr = editor_state.viewport.world_to_screen(Vec2::new(bb_max.x, bb_min.y), canvas_center);
    let bl = editor_state.viewport.world_to_screen(Vec2::new(bb_min.x, bb_max.y), canvas_center);

    // Draw bounding box outline
    painter.line_segment([egui::pos2(tl.x, tl.y), egui::pos2(tr.x, tr.y)], egui::Stroke::new(1.0, box_color));
    painter.line_segment([egui::pos2(tr.x, tr.y), egui::pos2(br.x, br.y)], egui::Stroke::new(1.0, box_color));
    painter.line_segment([egui::pos2(br.x, br.y), egui::pos2(bl.x, bl.y)], egui::Stroke::new(1.0, box_color));
    painter.line_segment([egui::pos2(bl.x, bl.y), egui::pos2(tl.x, tl.y)], egui::Stroke::new(1.0, box_color));

    // Draw corner handles (scale)
    let corners = [tl, tr, br, bl];
    for corner in &corners {
        let rect = egui::Rect::from_center_size(
            egui::pos2(corner.x, corner.y),
            egui::vec2(handle_size, handle_size),
        );
        painter.rect_filled(rect, 0.0, handle_color);
        painter.rect_stroke(rect, 0.0, handle_stroke, egui::epaint::StrokeKind::Outside);
    }

    // Draw rotation handle (circle above top-center)
    let top_center = Vec2::new((tl.x + tr.x) / 2.0, tl.y.min(tr.y) - 20.0);
    painter.line_segment(
        [
            egui::pos2((tl.x + tr.x) / 2.0, tl.y.min(tr.y)),
            egui::pos2(top_center.x, top_center.y),
        ],
        egui::Stroke::new(1.0, box_color),
    );
    painter.circle_filled(
        egui::pos2(top_center.x, top_center.y),
        4.0,
        handle_color,
    );
    painter.circle_stroke(
        egui::pos2(top_center.x, top_center.y),
        4.0,
        handle_stroke,
    );
}
