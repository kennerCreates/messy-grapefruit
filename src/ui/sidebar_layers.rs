use crate::model::project::Project;
use crate::model::sprite::{Layer, LayerGroup, Sprite, StrokeElement};
use crate::model::vec2::Vec2;
use crate::state::editor::EditorState;
use crate::state::history::History;
use crate::theme;

use super::icons;

pub(super) fn show_layer_list(
    ui: &mut egui::Ui,
    sprite: &mut Sprite,
    editor: &mut EditorState,
    project: &mut Project,
    history: &mut History,
) {
    let theme = project.editor_preferences.theme;
    let active_idx = editor.layer.resolve_active_idx(sprite);

    // --- Header buttons row ---
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;

        // Add layer
        if ui
            .add(icons::small_icon_button(icons::layer_add(), ui))
            .on_hover_text("Add Layer")
            .clicked()
        {
            let before = sprite.clone();
            let n = sprite.layers.len() + 1;
            let layer = Layer::new(format!("Layer {n}"));
            let new_id = layer.id.clone();
            // Insert above active layer
            let insert_idx = (active_idx + 1).min(sprite.layers.len());
            sprite.layers.insert(insert_idx, layer);
            editor.layer.active_layer_id = Some(new_id);
            history.push("Add layer".into(), before, sprite.clone());
        }

        // Remove layer (disabled if only 1 layer)
        let can_remove = sprite.layers.len() > 1;
        if ui
            .add_enabled(can_remove, icons::small_icon_button(icons::layer_remove(), ui))
            .on_hover_text("Remove Layer")
            .clicked()
        {
            let before = sprite.clone();
            let removed_id = sprite.layers[active_idx].id.clone();
            // Clear selections on elements in this layer
            let removed_element_ids: Vec<String> = sprite.layers[active_idx]
                .elements.iter().map(|e| e.id.clone()).collect();
            editor.selection.selected_ids.retain(|id| !removed_element_ids.contains(id));
            editor.clear_vertex_selection();

            sprite.layers.remove(active_idx);
            // Select the layer that's now at the same position (or the one below)
            let new_idx = active_idx.min(sprite.layers.len().saturating_sub(1));
            editor.layer.set_active_by_idx(new_idx, sprite);

            // If soloed layer was removed, clear solo
            if editor.layer.solo_layer_id.as_deref() == Some(&removed_id) {
                editor.layer.solo_layer_id = None;
            }
            history.push("Remove layer".into(), before, sprite.clone());
        }

        // Duplicate layer
        if ui
            .add(icons::small_icon_button(icons::layer_duplicate(), ui))
            .on_hover_text("Duplicate Layer")
            .clicked()
        {
            let before = sprite.clone();
            let mut new_layer = deep_clone_layer(&sprite.layers[active_idx]);
            new_layer.name = format!("{} copy", sprite.layers[active_idx].name);
            let new_id = new_layer.id.clone();
            sprite.layers.insert(active_idx + 1, new_layer);
            editor.layer.active_layer_id = Some(new_id);
            history.push("Duplicate layer".into(), before, sprite.clone());
        }

        // Mirror layer (horizontal flip)
        if ui
            .add(icons::small_icon_button(icons::layer_mirror(), ui))
            .on_hover_text("Mirror Layer (Horizontal)")
            .clicked()
            && !sprite.layers[active_idx].elements.is_empty()
        {
            let before = sprite.clone();
            mirror_layer_horizontal(&mut sprite.layers[active_idx]);
            history.push("Mirror layer".into(), before, sprite.clone());
        }

        // Combine (merge active layer into the one below)
        let can_combine = sprite.layers.len() > 1 && active_idx > 0;
        if ui
            .add_enabled(can_combine, icons::small_icon_button(icons::layer_combine(), ui))
            .on_hover_text("Merge Down")
            .clicked()
        {
            let before = sprite.clone();
            let elements: Vec<StrokeElement> = sprite.layers[active_idx].elements.clone();
            sprite.layers[active_idx - 1].elements.extend(elements);
            sprite.layers.remove(active_idx);
            let new_idx = active_idx - 1;
            editor.layer.set_active_by_idx(new_idx, sprite);
            history.push("Merge layers".into(), before, sprite.clone());
        }

        // Create group
        if ui
            .add(icons::small_icon_button(icons::layer_group_create(), ui))
            .on_hover_text("Create Group")
            .clicked()
        {
            let before = sprite.clone();
            let n = sprite.layer_groups.len() + 1;
            let group = LayerGroup::new(format!("Group {n}"));
            let gid = group.id.clone();
            sprite.layer_groups.push(group);
            sprite.layers[active_idx].group_id = Some(gid);
            history.push("Create group".into(), before, sprite.clone());
        }

        // Solo clear (only shown when solo is active)
        if editor.layer.solo_layer_id.is_some()
            && ui
                .add(icons::small_icon_button(icons::layer_solo_clear(), ui))
                .on_hover_text("Clear Solo")
                .clicked()
            {
                editor.layer.solo_layer_id = None;
            }
    });

    ui.add_space(4.0);

    let sel_color = theme::selected_color(theme);
    // Re-resolve after potential mutations above
    let active_idx = editor.layer.resolve_active_idx(sprite);

    // --- Build display list (groups + layers) ---
    let items = build_panel_items(sprite);

    // Collect row rects for drag reorder (layer_idx -> row rect)
    let mut row_rects: Vec<(usize, egui::Rect)> = Vec::new();
    // Track if we're currently dragging
    let is_dragging = editor.layer.drag_reorder.is_some();
    let drag_layer_id = editor.layer.drag_reorder.as_ref().map(|d| d.layer_id.clone());

    for item in &items {
        match item {
            LayerPanelItem::GroupHeader { group_idx } => {
                // Copy group state to avoid borrow conflicts in closures
                let gidx = *group_idx;
                let gid = sprite.layer_groups[gidx].id.clone();
                let g_collapsed = sprite.layer_groups[gidx].collapsed;
                let g_visible = sprite.layer_groups[gidx].visible;
                let g_locked = sprite.layer_groups[gidx].locked;

                ui.horizontal(|ui| {
                    // Collapse/expand toggle
                    let collapse_icon = if g_collapsed {
                        icons::layer_group_expand()
                    } else {
                        icons::layer_group_collapse()
                    };
                    if ui
                        .add(icons::small_icon_button(collapse_icon, ui))
                        .on_hover_text(if g_collapsed { "Expand Group" } else { "Collapse Group" })
                        .clicked()
                    {
                        sprite.layer_groups[gidx].collapsed = !g_collapsed;
                    }

                    // Group visibility cascade
                    let vis_icon = if g_visible {
                        icons::layer_visible()
                    } else {
                        icons::layer_hidden()
                    };
                    if ui
                        .add(icons::small_icon_button(vis_icon, ui))
                        .on_hover_text("Toggle Group Visibility")
                        .clicked()
                    {
                        let before = sprite.clone();
                        let new_vis = !g_visible;
                        sprite.layer_groups[gidx].visible = new_vis;
                        for layer in sprite.layers.iter_mut() {
                            if layer.group_id.as_deref() == Some(gid.as_str()) {
                                layer.visible = new_vis;
                            }
                        }
                        history.push("Toggle group visibility".into(), before, sprite.clone());
                    }

                    // Group lock cascade
                    let lock_icon = if g_locked {
                        icons::layer_locked()
                    } else {
                        icons::layer_unlocked()
                    };
                    if ui
                        .add(icons::small_icon_button(lock_icon, ui))
                        .on_hover_text("Toggle Group Lock")
                        .clicked()
                    {
                        let before = sprite.clone();
                        let new_lock = !g_locked;
                        sprite.layer_groups[gidx].locked = new_lock;
                        for layer in sprite.layers.iter_mut() {
                            if layer.group_id.as_deref() == Some(gid.as_str()) {
                                layer.locked = new_lock;
                            }
                        }
                        history.push("Toggle group lock".into(), before, sprite.clone());
                    }

                    // Group name (double-click to rename)
                    let is_renaming_group = editor.layer.renaming_layer_id.as_deref() == Some(gid.as_str());
                    if is_renaming_group {
                        let name = &mut sprite.layer_groups[gidx].name;
                        let te = egui::TextEdit::singleline(name)
                            .desired_width(ui.available_width())
                            .font(egui::TextStyle::Body);
                        let resp = ui.add(te);
                        if resp.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            editor.layer.renaming_layer_id = None;
                        }
                        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            editor.layer.renaming_layer_id = None;
                        }
                    } else {
                        let resp = ui.add(
                            egui::Label::new(
                                egui::RichText::new(&sprite.layer_groups[gidx].name).strong()
                            ).sense(egui::Sense::click())
                        );
                        if resp.double_clicked() {
                            editor.layer.renaming_layer_id = Some(gid.clone());
                        }
                        // Right-click to delete group (ungroup layers)
                        resp.context_menu(|ui| {
                            if ui.button("Ungroup").clicked() {
                                let before = sprite.clone();
                                for layer in sprite.layers.iter_mut() {
                                    if layer.group_id.as_deref() == Some(gid.as_str()) {
                                        layer.group_id = None;
                                    }
                                }
                                sprite.layer_groups.retain(|g| g.id != gid);
                                history.push("Ungroup".into(), before, sprite.clone());
                                ui.close_menu();
                            }
                        });
                    }
                });
            }

            LayerPanelItem::Layer { layer_idx, indented } => {
                let is_active = *layer_idx == active_idx;
                let layer_id = sprite.layers[*layer_idx].id.clone();
                let is_solo = editor.layer.solo_layer_id.as_deref() == Some(&layer_id);
                let is_being_dragged = drag_layer_id.as_deref() == Some(&layer_id);

                let row_resp = ui.horizontal(|ui| {
                    // Indent for grouped layers
                    if *indented {
                        ui.add_space(12.0);
                    }

                    // Solo toggle
                    let solo_icon = if is_solo {
                        icons::layer_solo()
                    } else {
                        icons::layer_solo_clear()
                    };
                    if ui
                        .add(icons::small_icon_button(solo_icon, ui))
                        .on_hover_text(if is_solo { "Unsolo" } else { "Solo Layer" })
                        .clicked()
                    {
                        if is_solo {
                            editor.layer.solo_layer_id = None;
                        } else {
                            editor.layer.solo_layer_id = Some(layer_id.clone());
                        }
                    }

                    // Visibility toggle
                    let vis_icon = if sprite.layers[*layer_idx].visible {
                        icons::layer_visible()
                    } else {
                        icons::layer_hidden()
                    };
                    if ui
                        .add(icons::small_icon_button(vis_icon, ui))
                        .on_hover_text(if sprite.layers[*layer_idx].visible {
                            "Hide Layer"
                        } else {
                            "Show Layer"
                        })
                        .clicked()
                    {
                        let before = sprite.clone();
                        sprite.layers[*layer_idx].visible = !sprite.layers[*layer_idx].visible;
                        history.push("Toggle layer visibility".into(), before, sprite.clone());
                    }

                    // Lock toggle
                    let lock_icon = if sprite.layers[*layer_idx].locked {
                        icons::layer_locked()
                    } else {
                        icons::layer_unlocked()
                    };
                    if ui
                        .add(icons::small_icon_button(lock_icon, ui))
                        .on_hover_text(if sprite.layers[*layer_idx].locked {
                            "Unlock Layer"
                        } else {
                            "Lock Layer"
                        })
                        .clicked()
                    {
                        let before = sprite.clone();
                        sprite.layers[*layer_idx].locked = !sprite.layers[*layer_idx].locked;
                        history.push("Toggle layer lock".into(), before, sprite.clone());
                    }

                    // Layer name (double-click to rename, click to select, drag to reorder)
                    let is_renaming = editor.layer.renaming_layer_id.as_deref() == Some(&layer_id);
                    if is_renaming {
                        let name = &mut sprite.layers[*layer_idx].name;
                        let te = egui::TextEdit::singleline(name)
                            .desired_width(ui.available_width())
                            .font(egui::TextStyle::Body);
                        let resp = ui.add(te);
                        if resp.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            editor.layer.renaming_layer_id = None;
                        }
                        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            editor.layer.renaming_layer_id = None;
                        }
                    } else {
                        // Dim non-soloed layers when solo is active, dim dragged layer
                        let dimmed = (editor.layer.solo_layer_id.is_some() && !is_solo) || is_being_dragged;
                        let name_text = if dimmed {
                            egui::RichText::new(&sprite.layers[*layer_idx].name)
                                .color(ui.visuals().weak_text_color())
                        } else {
                            egui::RichText::new(&sprite.layers[*layer_idx].name)
                        };

                        let label = egui::SelectableLabel::new(is_active, name_text);
                        let resp = ui.add(label);
                        if is_active {
                            let rect = resp.rect;
                            ui.painter().line_segment(
                                [rect.left_bottom(), rect.right_bottom()],
                                egui::Stroke::new(2.0, sel_color),
                            );
                        }

                        // Drag reorder: start drag on primary button drag
                        if resp.drag_started_by(egui::PointerButton::Primary) && !is_dragging {
                            editor.layer.drag_reorder = Some(crate::state::editor::LayerDragState {
                                layer_id: layer_id.clone(),
                                target_idx: None,
                                target_group_id: None,
                            });
                        }

                        if resp.clicked() {
                            editor.layer.set_active_by_idx(*layer_idx, sprite);
                        }
                        if resp.double_clicked() {
                            editor.layer.renaming_layer_id = Some(layer_id.clone());
                        }
                    }
                });

                // Collect row rect for drag target computation
                row_rects.push((*layer_idx, row_resp.response.rect));
            }
        }
    }

    // --- Drag reorder logic ---
    if let Some(drag) = &editor.layer.drag_reorder {
        let drag_id = drag.layer_id.clone();

        // Get pointer position
        if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
            // Find target insertion position from row rects
            // Row rects are in display order (top-to-bottom = highest layer index first)
            let mut best_target_idx: Option<usize> = None;
            let mut best_y = f32::MAX;

            for (layer_idx, rect) in &row_rects {
                let mid_y = rect.center().y;
                // If pointer is above this row's midpoint, insert above (higher index in layers vec)
                // If pointer is below, insert below (lower index)
                if pointer_pos.y < mid_y {
                    // Insert above this row: target_idx = layer_idx + 1
                    let target = *layer_idx + 1;
                    if (mid_y - pointer_pos.y) < best_y {
                        best_y = mid_y - pointer_pos.y;
                        best_target_idx = Some(target);
                    }
                } else {
                    // Insert below this row: target_idx = layer_idx
                    let target = *layer_idx;
                    if (pointer_pos.y - mid_y) < best_y {
                        best_y = pointer_pos.y - mid_y;
                        best_target_idx = Some(target);
                    }
                }
            }

            // Draw insertion indicator line
            if let Some(target_idx) = best_target_idx {
                // Find the Y position for the indicator
                let indicator_y = if target_idx >= sprite.layers.len() {
                    // Insert at top (above highest layer) — use top of first row
                    row_rects.first().map(|(_, r)| r.top()).unwrap_or(0.0)
                } else {
                    // Insert above target_idx — find the row for target_idx
                    row_rects.iter()
                        .find(|(idx, _)| *idx == target_idx)
                        .map(|(_, r)| r.bottom())
                        .unwrap_or_else(|| {
                            row_rects.last().map(|(_, r)| r.bottom()).unwrap_or(0.0)
                        })
                };

                let line_left = row_rects.first().map(|(_, r)| r.left()).unwrap_or(0.0);
                let line_right = row_rects.first().map(|(_, r)| r.right()).unwrap_or(100.0);

                ui.painter().line_segment(
                    [egui::Pos2::new(line_left, indicator_y), egui::Pos2::new(line_right, indicator_y)],
                    egui::Stroke::new(2.0, sel_color),
                );

                // Update drag state target
                if let Some(drag) = &mut editor.layer.drag_reorder {
                    drag.target_idx = Some(target_idx);
                }
            }
        }

        // Check for drag release
        let released = ui.ctx().input(|i| i.pointer.any_released());
        let cancelled = ui.ctx().input(|i| i.key_pressed(egui::Key::Escape));

        if released {
            if let Some(drag) = editor.layer.drag_reorder.take()
                && let (Some(src_idx), Some(target_idx)) = (sprite.layer_idx_by_id(&drag.layer_id), drag.target_idx)
                    && src_idx != target_idx && src_idx + 1 != target_idx {
                        let before = sprite.clone();
                        let layer = sprite.layers.remove(src_idx);
                        let insert_at = if target_idx > src_idx {
                            target_idx - 1
                        } else {
                            target_idx
                        };
                        let insert_at = insert_at.min(sprite.layers.len());
                        sprite.layers.insert(insert_at, layer);
                        editor.layer.active_layer_id = Some(drag_id);
                        history.push("Reorder layer".into(), before, sprite.clone());
                    }
        } else if cancelled {
            editor.layer.drag_reorder = None;
        }
    }
}

// --- Display list ---

enum LayerPanelItem {
    GroupHeader { group_idx: usize },
    Layer { layer_idx: usize, indented: bool },
}

/// Build the display list for the layer panel.
/// Layers are displayed top-to-bottom (highest index first).
/// Grouped layers appear under their group header.
fn build_panel_items(sprite: &Sprite) -> Vec<LayerPanelItem> {
    let mut items = Vec::new();
    let mut emitted_groups: Vec<String> = Vec::new();

    // Walk layers top-to-bottom (reverse index order)
    let layer_count = sprite.layers.len();
    for display_idx in 0..layer_count {
        let layer_idx = layer_count - 1 - display_idx;
        let layer = &sprite.layers[layer_idx];

        if let Some(gid) = &layer.group_id {
            // If this group hasn't been emitted yet, emit the header
            if !emitted_groups.contains(gid) {
                if let Some(group_idx) = sprite.layer_groups.iter().position(|g| g.id == *gid) {
                    items.push(LayerPanelItem::GroupHeader { group_idx });
                    emitted_groups.push(gid.clone());

                    // If collapsed, skip rendering child layers but still continue the outer loop
                    if sprite.layer_groups[group_idx].collapsed {
                        continue;
                    }
                }
            } else {
                // Group already emitted — check if collapsed
                if let Some(group_idx) = sprite.layer_groups.iter().position(|g| g.id == *gid)
                    && sprite.layer_groups[group_idx].collapsed {
                        continue;
                    }
            }

            items.push(LayerPanelItem::Layer { layer_idx, indented: true });
        } else {
            items.push(LayerPanelItem::Layer { layer_idx, indented: false });
        }
    }

    items
}

// --- Layer operations ---

/// Deep clone a layer, generating new UUIDs for the layer and all its elements/vertices.
fn deep_clone_layer(source: &Layer) -> Layer {
    let mut layer = Layer::new(&source.name);
    layer.visible = source.visible;
    layer.locked = source.locked;
    layer.group_id = source.group_id.clone();
    layer.elements = source.elements.iter().map(|e| {
        let mut elem = e.clone();
        elem.id = uuid::Uuid::new_v4().to_string();
        for v in &mut elem.vertices {
            v.id = uuid::Uuid::new_v4().to_string();
        }
        elem
    }).collect();
    layer
}

/// Mirror all elements in a layer horizontally around the layer's AABB center.
fn mirror_layer_horizontal(layer: &mut Layer) {
    // Compute AABB center of all vertices
    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    for elem in &layer.elements {
        for v in &elem.vertices {
            let world_x = v.pos.x + elem.position.x;
            min_x = min_x.min(world_x);
            max_x = max_x.max(world_x);
        }
    }
    if min_x >= max_x {
        return;
    }
    let center_x = (min_x + max_x) / 2.0;

    // Flip each element's vertices and position around center_x
    for elem in &mut layer.elements {
        // Flip position
        elem.position.x = 2.0 * center_x - elem.position.x;

        // Flip each vertex
        for v in &mut elem.vertices {
            v.pos.x = -v.pos.x;
            // Swap and flip control points for correct curve direction
            if let (Some(cp1), Some(cp2)) = (v.cp1, v.cp2) {
                v.cp1 = Some(Vec2::new(-cp2.x, cp2.y));
                v.cp2 = Some(Vec2::new(-cp1.x, cp1.y));
            } else {
                if let Some(cp) = &mut v.cp1 {
                    cp.x = -cp.x;
                }
                if let Some(cp) = &mut v.cp2 {
                    cp.x = -cp.x;
                }
            }
        }

        // Flip origin
        elem.origin.x = -elem.origin.x;

        // Reverse vertex order to maintain winding direction
        elem.vertices.reverse();
    }
}
