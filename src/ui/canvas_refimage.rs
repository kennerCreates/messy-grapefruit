use crate::action::AppAction;
use crate::model::sprite::Sprite;
use crate::model::vec2::Vec2;
use crate::state::editor::{EditorState, RefImageDragKind, RefImageDragState};
use crate::state::history::History;

/// Size of the corner resize handle in screen pixels.
const HANDLE_SIZE: f32 = 20.0;

/// Handle reference image interactions: click to select, drag to move, corner to resize.
/// Called from `show_canvas` before tool-specific input so ref image dragging takes priority.
/// Returns `true` if a ref image interaction consumed the pointer (tools should skip).
pub fn handle_ref_image_input(
    response: &egui::Response,
    editor: &mut EditorState,
    sprite: &mut Sprite,
    history: &mut History,
    canvas_rect: egui::Rect,
    actions: &mut Vec<AppAction>,
) -> bool {
    let canvas_center = canvas_rect.center();

    // --- Delete key removes selected ref image ---
    if editor.selected_ref_image_id.is_some()
        && !response.ctx.wants_keyboard_input()
        && response.ctx.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace))
    {
        let id = editor.selected_ref_image_id.take().unwrap();
        actions.push(AppAction::RemoveReferenceImage(id));
    }

    // --- Hover cursor: show resize cursor when over handle ---
    if let Some(hover_pos) = response.hover_pos()
        && is_over_resize_handle(editor, sprite, hover_pos, canvas_center)
    {
        response.ctx.set_cursor_icon(egui::CursorIcon::ResizeNwSe);
    }

    // --- Continue an in-progress drag ---
    if let Some(drag) = &editor.dragging_ref_image {
        let drag_id = drag.image_id.clone();
        let drag_kind = drag.kind;

        if response.dragged_by(egui::PointerButton::Primary) {
            if let Some(screen_pos) = response.interact_pointer_pos() {
                let world_pos = editor.viewport.screen_to_world(screen_pos, canvas_center);
                let drag = editor.dragging_ref_image.as_ref().unwrap();
                let delta = world_pos - drag.start_world;

                match drag_kind {
                    RefImageDragKind::Move => {
                        let new_pos = drag.initial_position + delta;
                        if let Some(img) = sprite.reference_images.iter_mut().find(|r| r.id == drag_id) {
                            img.position = new_pos;
                        }
                    }
                    RefImageDragKind::Resize => {
                        let initial_scale = drag.initial_scale;
                        let initial_pos = drag.initial_position;
                        let start_dist = (drag.start_world - initial_pos).length();
                        let curr_dist = (world_pos - initial_pos).length();
                        if start_dist > 0.001 {
                            let new_scale = (initial_scale * curr_dist / start_dist).max(0.01);
                            if let Some(img) = sprite.reference_images.iter_mut().find(|r| r.id == drag_id) {
                                img.scale = new_scale;
                            }
                        }
                    }
                }
            }
            return true;
        }

        // Drag ended
        if response.drag_stopped_by(egui::PointerButton::Primary) {
            editor.dragging_ref_image = None;
            history.end_drag(sprite.clone());
            return true;
        }
    }

    // --- Start a new drag or click to select ---
    if !response.drag_started_by(egui::PointerButton::Primary)
        && !response.clicked_by(egui::PointerButton::Primary)
    {
        return false;
    }

    let screen_pos = match response.interact_pointer_pos() {
        Some(p) => p,
        None => return false,
    };

    // Check if dragging the resize handle of the currently selected ref image.
    // This MUST run before the body hit-test so the handle wins over move.
    if response.drag_started_by(egui::PointerButton::Primary)
        && is_over_resize_handle(editor, sprite, screen_pos, canvas_center)
    {
        let sel_id = editor.selected_ref_image_id.as_ref().unwrap().clone();
        let img = sprite.reference_images.iter().find(|r| r.id == sel_id).unwrap();
        let world_pos = editor.viewport.screen_to_world(screen_pos, canvas_center);
        history.begin_drag("Resize reference image".into(), sprite.clone());
        editor.dragging_ref_image = Some(RefImageDragState {
            image_id: sel_id,
            kind: RefImageDragKind::Resize,
            start_world: world_pos,
            initial_position: img.position,
            initial_scale: img.scale,
        });
        return true;
    }

    // Hit test reference images (iterate back-to-front, last = topmost)
    let world_pos = editor.viewport.screen_to_world(screen_pos, canvas_center);
    let mut hit_id: Option<String> = None;
    for img in sprite.reference_images.iter().rev() {
        if !img.visible {
            continue;
        }
        let (w, h) = img.image_size.unwrap_or((100, 100));
        let img_max = Vec2::new(
            img.position.x + w as f32 * img.scale,
            img.position.y + h as f32 * img.scale,
        );
        if world_pos.x >= img.position.x
            && world_pos.x <= img_max.x
            && world_pos.y >= img.position.y
            && world_pos.y <= img_max.y
        {
            hit_id = Some(img.id.clone());
            break;
        }
    }

    if let Some(id) = hit_id {
        let img = sprite.reference_images.iter().find(|r| r.id == id).unwrap();
        let was_selected = editor.selected_ref_image_id.as_deref() == Some(&id);

        // Select (or re-select)
        editor.selected_ref_image_id = Some(id.clone());

        // Start move drag if unlocked and drag started
        if !img.locked && response.drag_started_by(egui::PointerButton::Primary) {
            history.begin_drag("Move reference image".into(), sprite.clone());
            editor.dragging_ref_image = Some(RefImageDragState {
                image_id: id,
                kind: RefImageDragKind::Move,
                start_world: world_pos,
                initial_position: img.position,
                initial_scale: img.scale,
            });
            return true;
        }

        // Click selected a ref image — consume only if it was newly selected
        return !was_selected;
    }

    // Clicked on empty space — deselect ref image (but don't consume, let tool handle it)
    if response.clicked_by(egui::PointerButton::Primary) && editor.selected_ref_image_id.is_some() {
        editor.selected_ref_image_id = None;
    }
    false
}

/// Check if a screen position is over the resize handle of the currently selected ref image.
fn is_over_resize_handle(
    editor: &EditorState,
    sprite: &Sprite,
    screen_pos: egui::Pos2,
    canvas_center: egui::Pos2,
) -> bool {
    let Some(ref sel_id) = editor.selected_ref_image_id else { return false };
    let Some(img) = sprite.reference_images.iter().find(|r| r.id == *sel_id) else { return false };
    if img.locked || !img.visible { return false; }
    let Some(handle_rect) = resize_handle_screen_rect(img, &editor.viewport, canvas_center) else { return false };
    handle_rect.contains(screen_pos)
}

/// Compute the screen-space rect for the bottom-right resize handle of a reference image.
fn resize_handle_screen_rect(
    img: &crate::model::sprite::ReferenceImage,
    viewport: &crate::state::editor::ViewportState,
    canvas_center: egui::Pos2,
) -> Option<egui::Rect> {
    let (w, h) = img.image_size?;
    let corner_world = Vec2::new(
        img.position.x + w as f32 * img.scale,
        img.position.y + h as f32 * img.scale,
    );
    let corner_screen = viewport.world_to_screen(corner_world, canvas_center);
    Some(egui::Rect::from_center_size(
        corner_screen,
        egui::Vec2::splat(HANDLE_SIZE),
    ))
}

/// Render the resize handle for the selected reference image.
pub fn render_ref_image_handles(
    painter: &egui::Painter,
    editor: &EditorState,
    sprite: &Sprite,
    canvas_rect: egui::Rect,
    theme_mode: crate::model::project::Theme,
) {
    let sel_id = match &editor.selected_ref_image_id {
        Some(id) => id,
        None => return,
    };
    let img = match sprite.reference_images.iter().find(|r| r.id == *sel_id) {
        Some(i) => i,
        None => return,
    };
    if img.locked || !img.visible {
        return;
    }
    let canvas_center = canvas_rect.center();
    if let Some(handle_rect) = resize_handle_screen_rect(img, &editor.viewport, canvas_center) {
        let color = crate::theme::selection_highlight_color(theme_mode);
        painter.rect_filled(handle_rect, 2.0, color);
        painter.rect_stroke(
            handle_rect,
            2.0,
            egui::Stroke::new(1.0, egui::Color32::WHITE),
            egui::StrokeKind::Outside,
        );
    }
}
