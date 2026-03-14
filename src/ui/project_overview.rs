//! Project overview page: a 2D canvas with draggable sprite cards,
//! live animation previews, and per-sprite dropdowns.

use crate::model::project::{Palette, ProjectSpriteRef};
use crate::model::sprite::Sprite;
use crate::model::Vec2;

/// State for the project overview page.
#[derive(Debug, Clone, Default)]
pub struct OverviewState {
    /// Viewport offset (panning)
    pub offset: Vec2,
    /// Zoom level
    pub zoom: f32,
    /// Index of the sprite card being dragged (None = not dragging)
    pub dragging_sprite: Option<usize>,
    /// Drag start position in screen space
    pub drag_start_screen: Option<Vec2>,
    /// Drag start position of the sprite (world space)
    pub drag_start_pos: Option<Vec2>,
    /// Is panning
    pub is_panning: bool,
    pub pan_start_pos: Option<Vec2>,
    pub pan_start_offset: Option<Vec2>,
    /// Right-click context menu target
    #[allow(dead_code)]
    pub context_menu_sprite: Option<usize>,
    /// Sprite being renamed
    #[allow(dead_code)]
    pub renaming_sprite: Option<usize>,
    #[allow(dead_code)]
    pub rename_buffer: String,
    /// Playback time used for live animation previews
    pub preview_time: f32,
    /// Instant when preview playback started (for computing elapsed time)
    pub preview_start: Option<std::time::Instant>,
}

impl OverviewState {
    pub fn new() -> Self {
        Self {
            zoom: 1.0,
            ..Default::default()
        }
    }
}

/// Actions the project overview can request.
pub enum OverviewAction {
    /// Open sprite at project sprite index in a new editor tab
    OpenSprite(usize),
    /// Delete sprite at project sprite index
    DeleteSprite(usize),
    /// Rename sprite at project sprite index
    #[allow(dead_code)]
    RenameSprite(usize, String),
    /// Update sprite position on the dashboard
    MoveSprite(usize, Vec2),
    /// Change the selected animation for a sprite on the dashboard
    SetSpriteAnimation(usize, Option<String>),
    /// Change the selected skin for a sprite on the dashboard
    SetSpriteSkin(usize, Option<String>),
    /// Open the new sprite dialog
    NewSprite,
}

/// Draw the project overview as a central panel.
/// `sprites` is a slice of (ProjectSpriteRef, Sprite) pairs loaded from disk.
/// Returns a list of actions to process.
#[allow(clippy::too_many_arguments)]
pub fn draw_project_overview(
    ctx: &egui::Context,
    overview_state: &mut OverviewState,
    sprite_refs: &[ProjectSpriteRef],
    sprites: &[Sprite],
    palette: &Palette,
    current_theme: crate::model::project::Theme,
) -> Vec<OverviewAction> {
    let mut actions = Vec::new();

    // Update preview time
    if overview_state.preview_start.is_none() {
        overview_state.preview_start = Some(std::time::Instant::now());
    }
    if let Some(start) = overview_state.preview_start {
        overview_state.preview_time = start.elapsed().as_secs_f32();
    }

    // Always request repaint for live animation
    ctx.request_repaint();

    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(crate::theme::canvas_bg_color(current_theme)))
        .show(ctx, |ui| {
            let available_rect = ui.available_rect_before_wrap();
            let canvas_center = Vec2::new(
                (available_rect.min.x + available_rect.max.x) / 2.0,
                (available_rect.min.y + available_rect.max.y) / 2.0,
            );

            let response = ui.allocate_rect(available_rect, egui::Sense::click_and_drag());
            let painter = ui.painter_at(available_rect);

            // --- Pan with middle mouse ---
            if (response.middle_clicked() || (response.dragged_by(egui::PointerButton::Middle)))
                && !overview_state.is_panning {
                    overview_state.is_panning = true;
                    if let Some(pos) = response.hover_pos() {
                        overview_state.pan_start_pos = Some(Vec2::from(pos));
                        overview_state.pan_start_offset = Some(overview_state.offset);
                    }
                }
            if overview_state.is_panning {
                if let (Some(start_pos), Some(start_offset)) =
                    (overview_state.pan_start_pos, overview_state.pan_start_offset)
                    && let Some(current_pos) = response.hover_pos() {
                        let delta = Vec2::from(current_pos) - start_pos;
                        overview_state.offset = Vec2::new(
                            start_offset.x + delta.x / overview_state.zoom,
                            start_offset.y + delta.y / overview_state.zoom,
                        );
                    }
                if !ctx.input(|i| i.pointer.middle_down()) {
                    overview_state.is_panning = false;
                    overview_state.pan_start_pos = None;
                    overview_state.pan_start_offset = None;
                }
            }

            // --- Zoom with scroll wheel ---
            if response.hovered() {
                let scroll = ctx.input(|i| i.raw_scroll_delta.y);
                if scroll != 0.0 {
                    let factor = if scroll > 0.0 { 1.1 } else { 1.0 / 1.1 };
                    overview_state.zoom = (overview_state.zoom * factor).clamp(0.1, 10.0);
                }
            }

            // --- Draw background grid (subtle) ---
            let grid_color = crate::theme::grid_color(current_theme);
            let grid_size = 64.0;
            let top_left = screen_to_world(
                Vec2::new(available_rect.min.x, available_rect.min.y),
                canvas_center,
                overview_state,
            );
            let bottom_right = screen_to_world(
                Vec2::new(available_rect.max.x, available_rect.max.y),
                canvas_center,
                overview_state,
            );

            let start_x = (top_left.x / grid_size).floor() as i32;
            let end_x = (bottom_right.x / grid_size).ceil() as i32;
            let start_y = (top_left.y / grid_size).floor() as i32;
            let end_y = (bottom_right.y / grid_size).ceil() as i32;

            let total = ((end_x - start_x + 1) * (end_y - start_y + 1)) as usize;
            if total < 5000 {
                for gx in start_x..=end_x {
                    for gy in start_y..=end_y {
                        let world_pos = Vec2::new(gx as f32 * grid_size, gy as f32 * grid_size);
                        let sp = world_to_screen(world_pos, canvas_center, overview_state);
                        if available_rect.contains(egui::pos2(sp.x, sp.y)) {
                            painter.circle_filled(egui::pos2(sp.x, sp.y), 1.0, grid_color);
                        }
                    }
                }
            }

            // --- Draw sprite cards ---
            // Sort by z_order for rendering
            let mut sorted_indices: Vec<usize> = (0..sprite_refs.len()).collect();
            sorted_indices.sort_by_key(|&i| sprite_refs[i].z_order);

            for &ref_idx in &sorted_indices {
                if ref_idx >= sprites.len() {
                    continue;
                }
                let sprite_ref = &sprite_refs[ref_idx];
                let sprite = &sprites[ref_idx];

                draw_sprite_card(
                    &painter,
                    overview_state,
                    canvas_center,
                    ref_idx,
                    sprite_ref,
                    sprite,
                    palette,
                    current_theme,
                    available_rect,
                );
            }

            // --- "New Sprite" button card ---
            let new_sprite_world = Vec2::new(0.0, -120.0);
            let new_sp = world_to_screen(new_sprite_world, canvas_center, overview_state);
            let btn_size = 80.0 * overview_state.zoom;
            let btn_rect = egui::Rect::from_center_size(
                egui::pos2(new_sp.x, new_sp.y),
                egui::vec2(btn_size, btn_size * 0.4),
            );
            if available_rect.intersects(btn_rect) {
                painter.rect_filled(
                    btn_rect,
                    4.0,
                    egui::Color32::from_rgba_unmultiplied(60, 80, 100, 180),
                );
                painter.rect_stroke(
                    btn_rect,
                    4.0,
                    egui::Stroke::new(1.5, crate::theme::accent_color(current_theme)),
                    egui::epaint::StrokeKind::Outside,
                );
                painter.text(
                    btn_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "+ New Sprite",
                    egui::FontId::proportional(14.0 * overview_state.zoom.min(2.0)),
                    crate::theme::text_color(current_theme),
                );
            }

            // --- Handle interactions ---
            // Click detection on sprite cards
            if response.clicked()
                && let Some(pos) = response.interact_pointer_pos() {
                    let click_pos = Vec2::from(pos);

                    // Check new sprite button
                    if btn_rect.contains(egui::pos2(click_pos.x, click_pos.y)) {
                        actions.push(OverviewAction::NewSprite);
                    }
                }

            // Double-click to open a sprite
            if response.double_clicked()
                && let Some(pos) = response.interact_pointer_pos() {
                    let click_pos = Vec2::from(pos);
                    for &ref_idx in sorted_indices.iter().rev() {
                        if ref_idx >= sprites.len() {
                            continue;
                        }
                        let card_rect =
                            get_card_rect(sprite_refs, sprites, ref_idx, canvas_center, overview_state);
                        if card_rect.contains(egui::pos2(click_pos.x, click_pos.y)) {
                            actions.push(OverviewAction::OpenSprite(ref_idx));
                            break;
                        }
                    }
                }

            // Drag to move sprite cards (primary button only, not panning)
            if !overview_state.is_panning {
                if response.drag_started_by(egui::PointerButton::Primary)
                    && overview_state.dragging_sprite.is_none()
                    && let Some(pos) = response.interact_pointer_pos() {
                        let click_pos = Vec2::from(pos);
                        // Find which card was clicked (topmost first)
                        for &ref_idx in sorted_indices.iter().rev() {
                            if ref_idx >= sprites.len() {
                                continue;
                            }
                            let card_rect = get_card_rect(
                                sprite_refs,
                                sprites,
                                ref_idx,
                                canvas_center,
                                overview_state,
                            );
                            if card_rect.contains(egui::pos2(click_pos.x, click_pos.y)) {
                                overview_state.dragging_sprite = Some(ref_idx);
                                overview_state.drag_start_screen = Some(click_pos);
                                overview_state.drag_start_pos =
                                    Some(sprite_refs[ref_idx].position);
                                break;
                            }
                        }
                    }

                if let Some(drag_idx) = overview_state.dragging_sprite {
                    if response.dragged_by(egui::PointerButton::Primary)
                        && let (Some(start_screen), Some(start_pos)) = (
                            overview_state.drag_start_screen,
                            overview_state.drag_start_pos,
                        )
                            && let Some(current_pos) = response.hover_pos() {
                                let delta = Vec2::from(current_pos) - start_screen;
                                let new_pos = Vec2::new(
                                    start_pos.x + delta.x / overview_state.zoom,
                                    start_pos.y + delta.y / overview_state.zoom,
                                );
                                actions.push(OverviewAction::MoveSprite(drag_idx, new_pos));
                            }
                    if response.drag_stopped_by(egui::PointerButton::Primary) {
                        overview_state.dragging_sprite = None;
                        overview_state.drag_start_screen = None;
                        overview_state.drag_start_pos = None;
                    }
                }
            }

            // --- Context menus on sprite cards ---
            // Right-click to show context menu
            response.context_menu(|ui| {
                if let Some(pos) = ui.ctx().input(|i| i.pointer.latest_pos()) {
                    let click_pos = Vec2::from(pos);
                    let mut found = false;
                    for &ref_idx in sorted_indices.iter().rev() {
                        if ref_idx >= sprites.len() {
                            continue;
                        }
                        let card_rect = get_card_rect(
                            sprite_refs,
                            sprites,
                            ref_idx,
                            canvas_center,
                            overview_state,
                        );
                        if card_rect.contains(egui::pos2(click_pos.x, click_pos.y)) {
                            ui.label(format!("Sprite: {}", sprites[ref_idx].name));
                            ui.separator();
                            if ui.button("Open").clicked() {
                                actions.push(OverviewAction::OpenSprite(ref_idx));
                                ui.close_menu();
                            }
                            if ui.button("Delete").clicked() {
                                actions.push(OverviewAction::DeleteSprite(ref_idx));
                                ui.close_menu();
                            }
                            found = true;
                            break;
                        }
                    }
                    if !found
                        && ui.button("New Sprite").clicked() {
                            actions.push(OverviewAction::NewSprite);
                            ui.close_menu();
                        }
                }
            });

            // --- Draw per-sprite UI overlays (animation/skin dropdowns) ---
            // We use egui Area widgets positioned near each card
            for (ref_idx, (sprite_ref, sprite)) in
                sprite_refs.iter().zip(sprites.iter()).enumerate()
            {
                let card_rect =
                    get_card_rect(sprite_refs, sprites, ref_idx, canvas_center, overview_state);

                // Only draw dropdowns if the card is visible and big enough
                if !available_rect.intersects(card_rect) || overview_state.zoom < 0.3 {
                    continue;
                }

                // Draw dropdown area just below the card
                let dropdown_top = card_rect.max.y + 2.0;
                let dropdown_left = card_rect.min.x;
                let dropdown_width = card_rect.width();

                if overview_state.zoom >= 0.5 {
                    let area_id =
                        egui::Id::new(format!("sprite_dropdown_{}", ref_idx));
                    egui::Area::new(area_id)
                        .fixed_pos(egui::pos2(dropdown_left, dropdown_top))
                        .order(egui::Order::Foreground)
                        .show(ctx, |ui| {
                            ui.set_max_width(dropdown_width);
                            ui.horizontal(|ui| {
                                // Animation sequence dropdown
                                let current_anim_label =
                                    match &sprite_ref.selected_animation_id {
                                        Some(id) => sprite
                                            .animations
                                            .iter()
                                            .find(|a| a.id == *id)
                                            .map(|a| a.name.clone())
                                            .unwrap_or_else(|| "?".to_string()),
                                        None => "Rest".to_string(),
                                    };

                                egui::ComboBox::from_id_salt(format!(
                                    "ov_anim_{}",
                                    ref_idx
                                ))
                                .selected_text(&current_anim_label)
                                .width(dropdown_width * 0.45)
                                .show_ui(ui, |ui| {
                                    let is_rest =
                                        sprite_ref.selected_animation_id.is_none();
                                    if ui.selectable_label(is_rest, "Rest Pose").clicked()
                                    {
                                        actions.push(
                                            OverviewAction::SetSpriteAnimation(
                                                ref_idx, None,
                                            ),
                                        );
                                    }
                                    for seq in &sprite.animations {
                                        let is_sel = sprite_ref
                                            .selected_animation_id
                                            .as_ref()
                                            == Some(&seq.id);
                                        if ui
                                            .selectable_label(is_sel, &seq.name)
                                            .clicked()
                                        {
                                            actions.push(
                                                OverviewAction::SetSpriteAnimation(
                                                    ref_idx,
                                                    Some(seq.id.clone()),
                                                ),
                                            );
                                        }
                                    }
                                });

                                // Skin dropdown
                                if !sprite.skins.is_empty() {
                                    let current_skin_label =
                                        match &sprite_ref.selected_skin_id {
                                            Some(id) => sprite
                                                .skins
                                                .iter()
                                                .find(|s| s.id == *id)
                                                .map(|s| s.name.clone())
                                                .unwrap_or_else(|| "?".to_string()),
                                            None => "Default".to_string(),
                                        };

                                    egui::ComboBox::from_id_salt(format!(
                                        "ov_skin_{}",
                                        ref_idx
                                    ))
                                    .selected_text(&current_skin_label)
                                    .width(dropdown_width * 0.4)
                                    .show_ui(ui, |ui| {
                                        let is_def =
                                            sprite_ref.selected_skin_id.is_none();
                                        if ui
                                            .selectable_label(is_def, "Default")
                                            .clicked()
                                        {
                                            actions.push(
                                                OverviewAction::SetSpriteSkin(
                                                    ref_idx, None,
                                                ),
                                            );
                                        }
                                        for skin in &sprite.skins {
                                            let is_sel = sprite_ref
                                                .selected_skin_id
                                                .as_ref()
                                                == Some(&skin.id);
                                            if ui
                                                .selectable_label(is_sel, &skin.name)
                                                .clicked()
                                            {
                                                actions.push(
                                                    OverviewAction::SetSpriteSkin(
                                                        ref_idx,
                                                        Some(skin.id.clone()),
                                                    ),
                                                );
                                            }
                                        }
                                    });
                                }
                            });
                        });
                }
            }
        });

    actions
}

/// Draw a single sprite card on the overview canvas.
#[allow(clippy::too_many_arguments)]
fn draw_sprite_card(
    painter: &egui::Painter,
    overview_state: &OverviewState,
    canvas_center: Vec2,
    _ref_idx: usize,
    sprite_ref: &ProjectSpriteRef,
    sprite: &Sprite,
    palette: &Palette,
    current_theme: crate::model::project::Theme,
    _clip_rect: egui::Rect,
) {
    let card_size_w = sprite.canvas_width as f32;
    let card_size_h = sprite.canvas_height as f32;

    let screen_pos = world_to_screen(sprite_ref.position, canvas_center, overview_state);
    let scaled_w = card_size_w * overview_state.zoom * 0.5;
    let scaled_h = card_size_h * overview_state.zoom * 0.5;

    let card_rect = egui::Rect::from_min_size(
        egui::pos2(screen_pos.x - scaled_w / 2.0, screen_pos.y - scaled_h / 2.0),
        egui::vec2(scaled_w, scaled_h),
    );

    // Card background
    let bg_color = if sprite.background_color_index > 0
        && sprite.background_color_index < palette.colors.len()
    {
        palette.colors[sprite.background_color_index].to_color32()
    } else {
        egui::Color32::from_rgba_unmultiplied(40, 45, 55, 230)
    };

    painter.rect_filled(
        card_rect,
        4.0,
        bg_color,
    );
    painter.rect_stroke(
        card_rect,
        4.0,
        egui::Stroke::new(1.5, crate::theme::accent_color(current_theme)),
        egui::epaint::StrokeKind::Outside,
    );

    // Draw a simple preview of the sprite's elements using the palette
    // This renders the actual stroke elements as bezier curves
    draw_sprite_preview(painter, card_rect, sprite, palette, overview_state, sprite_ref);

    // Label with sprite name below the preview
    let label_pos = egui::pos2(card_rect.center().x, card_rect.max.y - 4.0);
    let font_size = (11.0 * overview_state.zoom.min(2.0)).max(8.0);
    painter.text(
        label_pos,
        egui::Align2::CENTER_BOTTOM,
        &sprite.name,
        egui::FontId::proportional(font_size),
        crate::theme::text_color(current_theme),
    );
}

/// Draw a live preview of a sprite's elements within a card rectangle.
fn draw_sprite_preview(
    painter: &egui::Painter,
    card_rect: egui::Rect,
    sprite: &Sprite,
    palette: &Palette,
    overview_state: &OverviewState,
    sprite_ref: &ProjectSpriteRef,
) {
    // Create an animated version if a sequence is selected
    let animated_sprite;
    let display_sprite = if let Some(ref anim_id) = sprite_ref.selected_animation_id {
        if let Some(seq) = sprite.animations.iter().find(|a| a.id == *anim_id) {
            animated_sprite =
                crate::engine::animation::create_animated_sprite(sprite, seq, overview_state.preview_time);
            &animated_sprite
        } else {
            sprite
        }
    } else {
        sprite
    };

    let skin = sprite_ref.selected_skin_id.as_ref().and_then(|sid| {
        display_sprite.skins.iter().find(|s| s.id == *sid)
    });

    // Scale from sprite canvas space to card space
    let scale_x = card_rect.width() / display_sprite.canvas_width as f32;
    let scale_y = card_rect.height() / display_sprite.canvas_height as f32;
    let scale = scale_x.min(scale_y);

    let offset_x = card_rect.min.x + (card_rect.width() - display_sprite.canvas_width as f32 * scale) / 2.0;
    let offset_y = card_rect.min.y + (card_rect.height() - display_sprite.canvas_height as f32 * scale) / 2.0;

    for layer in &display_sprite.layers {
        if !layer.visible {
            continue;
        }

        let socket_tf =
            crate::engine::socket::resolve_socket_transform(display_sprite, &layer.id);

        for element in &layer.elements {
            if element.vertices.len() < 2 {
                continue;
            }

            // Resolve visual properties with skin
            let (stroke_idx, _fill_idx, stroke_w) = if let Some(skin) = skin {
                let ovr = skin.overrides.iter().find(|o| o.element_id == element.id);
                (
                    ovr.and_then(|o| o.stroke_color_index)
                        .unwrap_or(element.stroke_color_index),
                    ovr.and_then(|o| o.fill_color_index)
                        .unwrap_or(element.fill_color_index),
                    ovr.and_then(|o| o.stroke_width)
                        .unwrap_or(element.stroke_width),
                )
            } else {
                (
                    element.stroke_color_index,
                    element.fill_color_index,
                    element.stroke_width,
                )
            };

            let stroke_color = if stroke_idx > 0 && stroke_idx < palette.colors.len() {
                palette.colors[stroke_idx].to_color32()
            } else {
                continue;
            };

            // Draw path segments
            let total_offset_x = element.position.x + socket_tf.position.x;
            let total_offset_y = element.position.y + socket_tf.position.y;

            for i in 0..element.vertices.len() - 1 {
                let v0 = &element.vertices[i];
                let v1 = &element.vertices[i + 1];

                let (p0, cp1, cp2, p3) = crate::math::segment_bezier_points(v0, v1);

                let to_screen = |p: Vec2| -> egui::Pos2 {
                    egui::pos2(
                        offset_x + (p.x + total_offset_x) * scale,
                        offset_y + (p.y + total_offset_y) * scale,
                    )
                };

                // Flatten to polyline and draw
                let mut points = Vec::new();
                crate::math::flatten_cubic_bezier(p0, cp1, cp2, p3, 1.0 / scale.max(0.01), &mut points);

                for j in 0..points.len().saturating_sub(1) {
                    painter.line_segment(
                        [to_screen(points[j]), to_screen(points[j + 1])],
                        egui::Stroke::new((stroke_w * scale).max(0.5), stroke_color),
                    );
                }
            }
        }
    }
}

/// Get the screen-space rectangle of a sprite card.
fn get_card_rect(
    sprite_refs: &[ProjectSpriteRef],
    sprites: &[Sprite],
    ref_idx: usize,
    canvas_center: Vec2,
    overview_state: &OverviewState,
) -> egui::Rect {
    let sprite_ref = &sprite_refs[ref_idx];
    let sprite = &sprites[ref_idx];
    let card_size_w = sprite.canvas_width as f32;
    let card_size_h = sprite.canvas_height as f32;

    let screen_pos = world_to_screen(sprite_ref.position, canvas_center, overview_state);
    let scaled_w = card_size_w * overview_state.zoom * 0.5;
    let scaled_h = card_size_h * overview_state.zoom * 0.5;

    egui::Rect::from_min_size(
        egui::pos2(screen_pos.x - scaled_w / 2.0, screen_pos.y - scaled_h / 2.0),
        egui::vec2(scaled_w, scaled_h),
    )
}

fn world_to_screen(world_pos: Vec2, canvas_center: Vec2, state: &OverviewState) -> Vec2 {
    Vec2::new(
        (world_pos.x + state.offset.x) * state.zoom + canvas_center.x,
        (world_pos.y + state.offset.y) * state.zoom + canvas_center.y,
    )
}

fn screen_to_world(screen_pos: Vec2, canvas_center: Vec2, state: &OverviewState) -> Vec2 {
    Vec2::new(
        (screen_pos.x - canvas_center.x) / state.zoom - state.offset.x,
        (screen_pos.y - canvas_center.y) / state.zoom - state.offset.y,
    )
}
