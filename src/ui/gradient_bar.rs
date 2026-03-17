use egui::{Color32, Pos2, Rect, Sense, Ui, Vec2};

use crate::model::project::{Palette, Theme};
use crate::model::sprite::GradientStop;
use crate::state::editor::EditorState;
use crate::theme;

/// Height of the gradient preview bar in pixels.
const BAR_HEIGHT: f32 = 20.0;
/// Height of the stop handle area below the bar.
const HANDLE_AREA_HEIGHT: f32 = 14.0;
/// Height of the midpoint handle area.
const MIDPOINT_AREA_HEIGHT: f32 = 10.0;
/// Size of stop handle diamonds.
const STOP_HANDLE_SIZE: f32 = 8.0;
/// Size of midpoint handle diamonds.
const MIDPOINT_HANDLE_SIZE: f32 = 6.0;
/// Vertical distance to drag a stop off the bar to delete it.
const DELETE_DRAG_THRESHOLD: f32 = 30.0;

/// Internal drag state for the gradient bar widget.
#[derive(Debug, Clone, Copy, PartialEq)]
enum BarDrag {
    Stop(usize),
    Midpoint(usize),
}

/// Render the gradient bar widget with draggable stop and midpoint handles.
/// Returns true if any value was changed.
pub(super) fn render_gradient_bar(
    ui: &mut Ui,
    editor: &mut EditorState,
    palette: &Palette,
    theme_mode: Theme,
) -> bool {
    let mut changed = false;
    let available_width = ui.available_width().max(60.0);
    let total_height = BAR_HEIGHT + HANDLE_AREA_HEIGHT + MIDPOINT_AREA_HEIGHT;

    let (full_rect, response) = ui.allocate_exact_size(
        Vec2::new(available_width, total_height),
        Sense::click_and_drag(),
    );

    let bar_rect = Rect::from_min_size(
        full_rect.min,
        Vec2::new(available_width, BAR_HEIGHT),
    );
    let stop_area_top = bar_rect.max.y;
    let midpoint_area_top = stop_area_top + HANDLE_AREA_HEIGHT;

    let painter = ui.painter_at(full_rect);

    // === DRAWING PHASE (immutable borrows) ===
    {
        let stops = &editor.brush.gradient_stops;
        let midpoints = &editor.brush.gradient_midpoints;

        // Draw gradient preview bar
        let num_strips = (available_width as usize).max(1);
        for i in 0..num_strips {
            let t = i as f32 / (num_strips - 1).max(1) as f32;
            let color = sample_gradient_preview(t, stops, midpoints, palette);
            let x = bar_rect.min.x + t * bar_rect.width();
            let strip = Rect::from_min_max(
                Pos2::new(x, bar_rect.min.y),
                Pos2::new(x + 1.5, bar_rect.max.y),
            );
            painter.rect_filled(strip, 0.0, color);
        }

        // Border around bar
        let border_color = theme::theme_colors(theme_mode).mid;
        painter.rect_stroke(bar_rect, 1.0, egui::Stroke::new(1.0, border_color), egui::StrokeKind::Outside);

        // Draw stop handles (below bar)
        let sel_color = theme::selected_color(theme_mode);
        let handle_color = theme::theme_colors(theme_mode).icon_text;
        let sel_idx = editor.brush.selected_stop_index.unwrap_or(0);

        for (i, stop) in stops.iter().enumerate() {
            let x = bar_rect.min.x + stop.position * bar_rect.width();
            let y = stop_area_top + HANDLE_AREA_HEIGHT * 0.5;
            let fill = palette.get_color(stop.color_index).to_color32();
            draw_diamond(&painter, Pos2::new(x, y), STOP_HANDLE_SIZE, fill,
                if i == sel_idx { sel_color } else { handle_color });
        }

        // Draw midpoint handles (between stop pairs)
        for (i, &mp) in midpoints.iter().enumerate() {
            if i + 1 >= stops.len() { break; }
            let pos_a = stops[i].position;
            let pos_b = stops[i + 1].position;
            let mp_pos = pos_a + mp * (pos_b - pos_a);
            let x = bar_rect.min.x + mp_pos * bar_rect.width();
            let y = midpoint_area_top + MIDPOINT_AREA_HEIGHT * 0.5;
            draw_diamond(&painter, Pos2::new(x, y), MIDPOINT_HANDLE_SIZE,
                border_color, handle_color);
        }
    }
    // === END DRAWING PHASE — immutable borrows dropped ===

    // === INTERACTION PHASE (mutable access) ===
    let pointer_pos = response.interact_pointer_pos();

    // Click to select stop or add new stop
    if response.clicked()
        && let Some(pos) = pointer_pos
    {
        let hit_stop = hit_test_stop(pos, &editor.brush.gradient_stops, bar_rect, stop_area_top);
        if let Some(hit_idx) = hit_stop {
            editor.brush.selected_stop_index = Some(hit_idx);
        } else if pos.y >= bar_rect.min.y && pos.y <= stop_area_top + HANDLE_AREA_HEIGHT {
            let t = ((pos.x - bar_rect.min.x) / bar_rect.width()).clamp(0.0, 1.0);
            let nearest_color = nearest_stop_color(t, &editor.brush.gradient_stops);
            let new_stop = GradientStop { position: t, color_index: nearest_color };
            editor.brush.gradient_stops.push(new_stop);
            editor.brush.gradient_stops.sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap());
            editor.brush.gradient_midpoints.resize(
                editor.brush.gradient_stops.len().saturating_sub(1), 0.5,
            );
            editor.brush.selected_stop_index = editor.brush.gradient_stops
                .iter().position(|s| (s.position - t).abs() < 0.001);
            changed = true;
        }
    }

    // Drag start
    if response.drag_started()
        && let Some(pos) = pointer_pos
    {
        if let Some(hit_idx) = hit_test_stop(pos, &editor.brush.gradient_stops, bar_rect, stop_area_top) {
            editor.brush.selected_stop_index = Some(hit_idx);
            ui.memory_mut(|mem| mem.data.insert_temp(response.id, BarDrag::Stop(hit_idx)));
        } else if let Some(hit_idx) = hit_test_midpoint(
            pos, &editor.brush.gradient_stops, &editor.brush.gradient_midpoints,
            bar_rect, midpoint_area_top,
        ) {
            ui.memory_mut(|mem| mem.data.insert_temp(response.id, BarDrag::Midpoint(hit_idx)));
        }
    }

    // Drag update
    if response.dragged()
        && let Some(pos) = pointer_pos
    {
        let drag_kind: Option<BarDrag> = ui.memory(|mem| mem.data.get_temp(response.id));
        if let Some(kind) = drag_kind {
            match kind {
                BarDrag::Stop(idx) => {
                    if idx < editor.brush.gradient_stops.len() {
                        let dy = (pos.y - (stop_area_top + HANDLE_AREA_HEIGHT * 0.5)).abs();
                        if !(dy > DELETE_DRAG_THRESHOLD && editor.brush.gradient_stops.len() > 2) {
                            let t = ((pos.x - bar_rect.min.x) / bar_rect.width()).clamp(0.0, 1.0);
                            editor.brush.gradient_stops[idx].position = t;
                            changed = true;
                        }
                    }
                }
                BarDrag::Midpoint(idx) => {
                    if idx < editor.brush.gradient_midpoints.len() && idx + 1 < editor.brush.gradient_stops.len() {
                        let pos_a = editor.brush.gradient_stops[idx].position;
                        let pos_b = editor.brush.gradient_stops[idx + 1].position;
                        let seg_len = pos_b - pos_a;
                        if seg_len > 0.001 {
                            let t = ((pos.x - bar_rect.min.x) / bar_rect.width()).clamp(0.0, 1.0);
                            let local = ((t - pos_a) / seg_len).clamp(0.05, 0.95);
                            editor.brush.gradient_midpoints[idx] = local;
                            changed = true;
                        }
                    }
                }
            }
        }
    }

    // Drag end
    if response.drag_stopped()
        && let Some(pos) = pointer_pos
    {
        let drag_kind: Option<BarDrag> = ui.memory(|mem| mem.data.get_temp(response.id));
        if let Some(BarDrag::Stop(idx)) = drag_kind {
            let dy = (pos.y - (stop_area_top + HANDLE_AREA_HEIGHT * 0.5)).abs();
            if dy > DELETE_DRAG_THRESHOLD && editor.brush.gradient_stops.len() > 2
                && idx < editor.brush.gradient_stops.len()
            {
                editor.brush.gradient_stops.remove(idx);
                editor.brush.gradient_midpoints.resize(
                    editor.brush.gradient_stops.len().saturating_sub(1), 0.5,
                );
                editor.brush.selected_stop_index = Some(
                    idx.min(editor.brush.gradient_stops.len().saturating_sub(1)),
                );
                changed = true;
            } else {
                let selected_pos = editor.brush.gradient_stops.get(idx).map(|s| s.position);
                editor.brush.gradient_stops.sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap());
                if let Some(sp) = selected_pos {
                    editor.brush.selected_stop_index = editor.brush.gradient_stops
                        .iter().position(|s| (s.position - sp).abs() < 0.001);
                }
            }
        }
        ui.memory_mut(|mem| mem.data.remove::<BarDrag>(response.id));
    }

    changed
}

/// Draw a diamond shape at the given center position.
fn draw_diamond(painter: &egui::Painter, center: Pos2, size: f32, fill: Color32, stroke_color: Color32) {
    let half = size * 0.5;
    let points = vec![
        Pos2::new(center.x, center.y - half),
        Pos2::new(center.x + half, center.y),
        Pos2::new(center.x, center.y + half),
        Pos2::new(center.x - half, center.y),
    ];
    painter.add(egui::Shape::convex_polygon(
        points,
        fill,
        egui::Stroke::new(1.0, stroke_color),
    ));
}

/// Hit-test stop handles. Returns the index of the hit stop, if any.
fn hit_test_stop(pos: Pos2, stops: &[GradientStop], bar_rect: Rect, stop_area_top: f32) -> Option<usize> {
    let y_center = stop_area_top + HANDLE_AREA_HEIGHT * 0.5;
    stops.iter().enumerate()
        .filter(|(_, s)| {
            let x = bar_rect.min.x + s.position * bar_rect.width();
            (pos.x - x).abs() < STOP_HANDLE_SIZE && (pos.y - y_center).abs() < STOP_HANDLE_SIZE
        })
        .min_by(|(_, a), (_, b)| {
            let da = (pos.x - (bar_rect.min.x + a.position * bar_rect.width())).abs();
            let db = (pos.x - (bar_rect.min.x + b.position * bar_rect.width())).abs();
            da.partial_cmp(&db).unwrap()
        })
        .map(|(i, _)| i)
}

/// Hit-test midpoint handles. Returns the midpoint index if hit.
fn hit_test_midpoint(
    pos: Pos2, stops: &[GradientStop], midpoints: &[f32],
    bar_rect: Rect, midpoint_area_top: f32,
) -> Option<usize> {
    let y_center = midpoint_area_top + MIDPOINT_AREA_HEIGHT * 0.5;
    midpoints.iter().enumerate()
        .find(|(i, mp)| {
            if *i + 1 >= stops.len() { return false; }
            let mp_pos = stops[*i].position + **mp * (stops[*i + 1].position - stops[*i].position);
            let x = bar_rect.min.x + mp_pos * bar_rect.width();
            (pos.x - x).abs() < MIDPOINT_HANDLE_SIZE && (pos.y - y_center).abs() < MIDPOINT_HANDLE_SIZE
        })
        .map(|(i, _)| i)
}

/// Find the nearest stop's color index for a given t position.
fn nearest_stop_color(t: f32, stops: &[GradientStop]) -> u8 {
    stops.iter()
        .min_by(|a, b| (a.position - t).abs().partial_cmp(&(b.position - t).abs()).unwrap())
        .map(|s| s.color_index)
        .unwrap_or(1)
}

/// Sample gradient color for preview bar rendering.
fn sample_gradient_preview(t: f32, stops: &[GradientStop], midpoints: &[f32], palette: &Palette) -> Color32 {
    if stops.is_empty() { return Color32::TRANSPARENT; }
    if stops.len() == 1 || t <= stops[0].position {
        return palette.get_color(stops[0].color_index).to_color32();
    }
    if t >= stops[stops.len() - 1].position {
        return palette.get_color(stops[stops.len() - 1].color_index).to_color32();
    }

    let mut i = 0;
    while i + 1 < stops.len() && stops[i + 1].position < t { i += 1; }
    if i + 1 >= stops.len() {
        return palette.get_color(stops[stops.len() - 1].color_index).to_color32();
    }

    let col_a = palette.get_color(stops[i].color_index).to_color32();
    let col_b = palette.get_color(stops[i + 1].color_index).to_color32();
    let seg_len = stops[i + 1].position - stops[i].position;
    if seg_len < 0.0001 { return col_a; }

    let local_t = ((t - stops[i].position) / seg_len).clamp(0.0, 1.0);
    let m = midpoints.get(i).copied().unwrap_or(0.5).clamp(0.01, 0.99);
    let adjusted = if local_t <= m {
        0.5 * (local_t / m)
    } else {
        0.5 + 0.5 * ((local_t - m) / (1.0 - m))
    };

    let inv = 1.0 - adjusted;
    Color32::from_rgba_unmultiplied(
        (col_a.r() as f32 * inv + col_b.r() as f32 * adjusted) as u8,
        (col_a.g() as f32 * inv + col_b.g() as f32 * adjusted) as u8,
        (col_a.b() as f32 * inv + col_b.b() as f32 * adjusted) as u8,
        (col_a.a() as f32 * inv + col_b.a() as f32 * adjusted) as u8,
    )
}
