use crate::engine::socket;
use crate::model::project::{GridMode, Palette, PaletteColor};
use crate::model::sprite::{
    BlendMode, GravityForce, LayerConstraints, LookAtConstraint, PhysicsConstraint,
    ProceduralModifier, SolverType, SpringSmoothing, Sprite, Waveform, WindForce,
};
use crate::state::editor::{EditorState, ToolKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarTab {
    Layers,
    Palette,
    Skins,
    Settings,
}

/// Actions that the sidebar can request
pub enum SidebarAction {
    SetActiveLayer(usize),
    AddLayer,
    RemoveLayer(usize),
    ToggleLayerVisibility(usize),
    ToggleLayerLock(usize),
    SetActiveColor(usize),
    SetStrokeWidth(f32),
    DuplicateLayer(usize),
    MirrorLayerH(usize),
    MirrorLayerV(usize),
    CombineLayerDown(usize),
    MoveLayerUp(usize),
    MoveLayerDown(usize),
    AddPaletteColor(PaletteColor),
    DeletePaletteColor(usize),
    UpdatePaletteColor(usize, PaletteColor),
    ImportLospecPalette(String),
    ToggleTheme,
    SetGridMode(GridMode),
    /// Resize the active sprite's canvas
    ResizeCanvas(u32, u32),
    /// Set socket: attach layer at `layer_index` to a parent element vertex
    SetSocket {
        layer_index: usize,
        parent_element_id: String,
        parent_vertex_id: String,
    },
    /// Clear socket: detach layer from its parent, snap to world-space position
    ClearSocket(usize),
    /// Create a new skin
    CreateSkin,
    /// Delete a skin by ID
    DeleteSkin(String),
    /// Rename a skin
    RenameSkin(String, String),
    /// Duplicate a skin by ID
    DuplicateSkin(String),
    /// Set the active skin for preview (None = base/default)
    SetActiveSkin(Option<String>),
    /// Set or update a skin override for an element
    SetSkinOverride {
        skin_id: String,
        element_id: String,
        stroke_color_index: Option<usize>,
        fill_color_index: Option<usize>,
        stroke_width: Option<f32>,
    },
    /// Remove a skin override for an element
    RemoveSkinOverride {
        skin_id: String,
        element_id: String,
    },
    /// Create an IK chain from selected socketed layers
    CreateIKChain {
        layer_ids: Vec<String>,
        name: String,
    },
    /// Delete an IK chain by ID
    DeleteIKChain(String),
    /// Set solver type for an IK chain
    SetIKChainSolver {
        chain_id: String,
        solver: SolverType,
    },
    /// Set bend direction for an IK chain
    SetIKChainBendDirection {
        chain_id: String,
        bend_direction: i8,
    },
    /// Set IK chain mix value
    SetIKChainMix {
        chain_id: String,
        mix: f32,
    },
    /// Add/update an angle constraint on an IK chain
    SetIKAngleConstraint {
        chain_id: String,
        layer_id: String,
        min: f32,
        max: f32,
    },
    /// Remove an angle constraint from an IK chain
    RemoveIKAngleConstraint {
        chain_id: String,
        layer_id: String,
    },
    /// Update the full layer constraints for a layer
    SetLayerConstraints {
        layer_index: usize,
        constraints: LayerConstraints,
    },
    /// Toggle debug overlays
    ToggleDebugBones,
    ToggleDebugIKTargets,
    ToggleDebugConstraints,
    ToggleDebugSpringTargets,
    /// Set element property (position/rotation/scale/origin)
    SetElementProperty {
        element_id: String,
        position: crate::model::Vec2,
        rotation: f32,
        scale: crate::model::Vec2,
        origin: crate::model::Vec2,
    },
}

/// Persistent state for the sidebar (color editing, lospec input, etc.)
#[derive(Default)]
pub struct SidebarState {
    pub editing_color_index: Option<usize>,
    pub editing_color_rgb: [u8; 3],
    pub lospec_slug: String,
    pub lospec_error: Option<String>,
    /// If Some, the socket picker is open for this layer index
    pub socket_picker_layer: Option<usize>,
    /// If Some, the skin with this ID is being renamed
    pub renaming_skin_id: Option<String>,
    /// Buffer for skin rename text input
    pub skin_rename_buffer: String,
    /// If true, the IK chain creation mode is active
    pub ik_chain_creating: bool,
    /// Layer IDs selected for IK chain creation (ordered root-to-tip)
    pub ik_chain_layer_ids: Vec<String>,
    /// Name buffer for new IK chain
    pub ik_chain_name_buffer: String,
}


/// Draw the right sidebar panel.
pub fn draw_sidebar(
    ctx: &egui::Context,
    editor_state: &EditorState,
    sprite: &Sprite,
    palette: &Palette,
    sidebar_tab: &mut SidebarTab,
    sidebar_state: &mut SidebarState,
    grid_mode: GridMode,
) -> Vec<SidebarAction> {
    let mut actions = Vec::new();

    egui::SidePanel::right("sidebar")
        .min_width(200.0)
        .max_width(300.0)
        .show(ctx, |ui| {
            // === Top zone: context-sensitive tool options ===
            ui.heading("Tool Options");
            ui.separator();

            match editor_state.active_tool {
                ToolKind::Line => {
                    draw_line_tool_options(ui, editor_state, palette, &mut actions);
                }
                ToolKind::Select => {
                    draw_select_tool_options(ui, editor_state, sprite, &mut actions);
                    draw_constraints_ui(ui, sprite, editor_state, &mut actions);
                    draw_ik_chain_ui(ui, sprite, editor_state, &mut actions, sidebar_state);
                    draw_debug_overlays_ui(ui, editor_state, &mut actions);
                }
                ToolKind::Fill => {
                    ui.label("Click a closed element to fill");
                    ui.label("Click empty canvas to set background");
                    draw_active_color_display(ui, editor_state, palette);
                }
                ToolKind::Eraser => {
                    ui.label("Click a vertex to delete it");
                    ui.label("Splits path if needed");
                }
            }

            ui.add_space(16.0);
            ui.separator();

            // === Bottom zone: fixed tabs ===
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(*sidebar_tab == SidebarTab::Layers, "Layers")
                    .clicked()
                {
                    *sidebar_tab = SidebarTab::Layers;
                }
                if ui
                    .selectable_label(*sidebar_tab == SidebarTab::Palette, "Palette")
                    .clicked()
                {
                    *sidebar_tab = SidebarTab::Palette;
                }
                if ui
                    .selectable_label(*sidebar_tab == SidebarTab::Skins, "Skins")
                    .clicked()
                {
                    *sidebar_tab = SidebarTab::Skins;
                }
                if ui
                    .selectable_label(*sidebar_tab == SidebarTab::Settings, "Settings")
                    .clicked()
                {
                    *sidebar_tab = SidebarTab::Settings;
                }
            });
            ui.separator();

            match sidebar_tab {
                SidebarTab::Layers => {
                    draw_layers_panel(ui, sprite, editor_state.active_layer_index, &mut actions, sidebar_state);
                }
                SidebarTab::Palette => {
                    draw_palette_panel(ui, palette, editor_state.active_color_index, &mut actions, sidebar_state);
                }
                SidebarTab::Skins => {
                    draw_skins_panel(ui, sprite, editor_state, palette, &mut actions, sidebar_state);
                }
                SidebarTab::Settings => {
                    draw_settings_panel(ui, grid_mode, sprite, &mut actions);
                }
            }
        });

    actions
}

fn draw_line_tool_options(
    ui: &mut egui::Ui,
    editor_state: &EditorState,
    palette: &Palette,
    actions: &mut Vec<SidebarAction>,
) {
    ui.label("Stroke Width");
    let mut width = editor_state.stroke_width;
    if ui
        .add(egui::Slider::new(&mut width, 0.5..=20.0).text("px"))
        .changed()
    {
        actions.push(SidebarAction::SetStrokeWidth(width));
    }

    ui.add_space(4.0);

    let mode_text = if editor_state.curve_mode {
        "Mode: Curve [C]"
    } else {
        "Mode: Straight [C]"
    };
    ui.label(mode_text);

    ui.add_space(4.0);
    draw_active_color_display(ui, editor_state, palette);
}

fn draw_select_tool_options(
    ui: &mut egui::Ui,
    editor_state: &EditorState,
    sprite: &Sprite,
    actions: &mut Vec<SidebarAction>,
) {
    let count = editor_state.selection.selected_element_ids.len();
    if count == 0 {
        ui.label("Click to select elements");
        ui.label("Shift+click for multi-select");
        ui.label("Ctrl+A to select all");
        return;
    }

    ui.label(format!("Selected: {} element(s)", count));
    ui.add_space(4.0);

    // Show property editors for the first selected element
    if let Some(ref elem_id) = editor_state.selection.selected_element_ids.first()
        && let Some(element) = sprite.layers.iter()
            .flat_map(|l| l.elements.iter())
            .find(|e| &e.id == *elem_id)
    {
            let mut pos_x = element.position.x;
            let mut pos_y = element.position.y;
            let mut rotation_deg = element.rotation.to_degrees();
            let mut scale_x = element.scale.x;
            let mut scale_y = element.scale.y;
            let mut origin_x = element.origin.x;
            let mut origin_y = element.origin.y;
            let mut changed = false;

            egui::CollapsingHeader::new("Position")
                .default_open(true)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("X:");
                        if ui.add(egui::DragValue::new(&mut pos_x).speed(0.5)).changed() {
                            changed = true;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Y:");
                        if ui.add(egui::DragValue::new(&mut pos_y).speed(0.5)).changed() {
                            changed = true;
                        }
                    });
                });

            egui::CollapsingHeader::new("Rotation")
                .default_open(true)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui.add(egui::DragValue::new(&mut rotation_deg).speed(1.0).suffix("°")).changed() {
                            changed = true;
                        }
                    });
                });

            egui::CollapsingHeader::new("Scale")
                .default_open(true)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("X:");
                        if ui.add(egui::DragValue::new(&mut scale_x).speed(0.01).range(0.01..=100.0)).changed() {
                            changed = true;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Y:");
                        if ui.add(egui::DragValue::new(&mut scale_y).speed(0.01).range(0.01..=100.0)).changed() {
                            changed = true;
                        }
                    });
                });

            egui::CollapsingHeader::new("Origin")
                .default_open(true)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("X:");
                        if ui.add(egui::DragValue::new(&mut origin_x).speed(0.5)).changed() {
                            changed = true;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Y:");
                        if ui.add(egui::DragValue::new(&mut origin_y).speed(0.5)).changed() {
                            changed = true;
                        }
                    });
                });

            if changed {
                actions.push(SidebarAction::SetElementProperty {
                    element_id: elem_id.to_string(),
                    position: crate::model::Vec2::new(pos_x, pos_y),
                    rotation: rotation_deg.to_radians(),
                    scale: crate::model::Vec2::new(scale_x, scale_y),
                    origin: crate::model::Vec2::new(origin_x, origin_y),
                });
            }
    }

    ui.add_space(4.0);
    ui.small("Drag to move | Ctrl+C/V copy/paste | Delete to remove");
}

/// Draw IK chain management UI in the select tool panel (shown when an animation is selected)
fn draw_ik_chain_ui(
    ui: &mut egui::Ui,
    sprite: &Sprite,
    editor_state: &EditorState,
    actions: &mut Vec<SidebarAction>,
    sidebar_state: &mut SidebarState,
) {
    let Some(ref seq_id) = editor_state.animation.selected_sequence_id else {
        return;
    };
    let Some(seq) = sprite.animations.iter().find(|a| a.id == *seq_id) else {
        return;
    };

    ui.separator();
    ui.label("IK Chains");
    ui.add_space(2.0);

    // List existing IK chains for this sequence
    for chain in &seq.ik_chains {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(format!("{} ({})", chain.name, if chain.solver == SolverType::TwoBone { "2-bone" } else { "FABRIK" }));
                if ui.small_button("X").on_hover_text("Delete IK chain").clicked() {
                    actions.push(SidebarAction::DeleteIKChain(chain.id.clone()));
                }
            });

            // Solver type selector
            ui.horizontal(|ui| {
                ui.label("Solver:");
                if ui.selectable_label(chain.solver == SolverType::TwoBone, "2-Bone").clicked() {
                    actions.push(SidebarAction::SetIKChainSolver {
                        chain_id: chain.id.clone(),
                        solver: SolverType::TwoBone,
                    });
                }
                if ui.selectable_label(chain.solver == SolverType::Fabrik, "FABRIK").clicked() {
                    actions.push(SidebarAction::SetIKChainSolver {
                        chain_id: chain.id.clone(),
                        solver: SolverType::Fabrik,
                    });
                }
            });

            // Bend direction (only for 2-bone)
            if chain.solver == SolverType::TwoBone {
                ui.horizontal(|ui| {
                    ui.label("Bend:");
                    if ui.selectable_label(chain.bend_direction >= 0, "+1").clicked() {
                        actions.push(SidebarAction::SetIKChainBendDirection {
                            chain_id: chain.id.clone(),
                            bend_direction: 1,
                        });
                    }
                    if ui.selectable_label(chain.bend_direction < 0, "-1").clicked() {
                        actions.push(SidebarAction::SetIKChainBendDirection {
                            chain_id: chain.id.clone(),
                            bend_direction: -1,
                        });
                    }
                });
            }

            // Mix slider
            let mut mix = chain.mix;
            if ui.add(egui::Slider::new(&mut mix, 0.0..=1.0).text("FK/IK Mix")).changed() {
                actions.push(SidebarAction::SetIKChainMix {
                    chain_id: chain.id.clone(),
                    mix,
                });
            }

            // Layers in chain
            ui.label(format!("Layers: {}", chain.layer_ids.len()));
            for layer_id in &chain.layer_ids {
                let layer_name = sprite.layers.iter()
                    .find(|l| l.id == *layer_id)
                    .map(|l| l.name.as_str())
                    .unwrap_or("?");
                ui.label(format!("  - {}", layer_name));
            }

            // Angle constraints (2-bone only)
            if chain.solver == SolverType::TwoBone {
                ui.collapsing("Angle Constraints", |ui| {
                    for (i, layer_id) in chain.layer_ids.iter().enumerate() {
                        let layer_name = sprite.layers.iter()
                            .find(|l| l.id == *layer_id)
                            .map(|l| l.name.as_str())
                            .unwrap_or("?");
                        let existing = chain.angle_constraints.iter()
                            .find(|c| c.layer_id == *layer_id);

                        ui.horizontal(|ui| {
                            ui.label(format!("Joint {}: {}", i, layer_name));
                            if let Some(constraint) = existing {
                                let mut min_deg = constraint.min.to_degrees();
                                let mut max_deg = constraint.max.to_degrees();
                                let mut changed = false;

                                if ui.add(egui::DragValue::new(&mut min_deg).speed(1.0).prefix("min:").suffix("\u{00B0}")).changed() {
                                    changed = true;
                                }
                                if ui.add(egui::DragValue::new(&mut max_deg).speed(1.0).prefix("max:").suffix("\u{00B0}")).changed() {
                                    changed = true;
                                }

                                if changed {
                                    actions.push(SidebarAction::SetIKAngleConstraint {
                                        chain_id: chain.id.clone(),
                                        layer_id: layer_id.clone(),
                                        min: min_deg.to_radians(),
                                        max: max_deg.to_radians(),
                                    });
                                }

                                if ui.small_button("X").clicked() {
                                    actions.push(SidebarAction::RemoveIKAngleConstraint {
                                        chain_id: chain.id.clone(),
                                        layer_id: layer_id.clone(),
                                    });
                                }
                            } else if ui.small_button("+ Constrain").clicked() {
                                actions.push(SidebarAction::SetIKAngleConstraint {
                                    chain_id: chain.id.clone(),
                                    layer_id: layer_id.clone(),
                                    min: -std::f32::consts::PI,
                                    max: std::f32::consts::PI,
                                });
                            }
                        });
                    }
                });
            }
        });
        ui.add_space(2.0);
    }

    // Create new IK chain UI
    if sidebar_state.ik_chain_creating {
        ui.group(|ui| {
            ui.label("New IK Chain");
            ui.text_edit_singleline(&mut sidebar_state.ik_chain_name_buffer);

            // Show available socketed layers to add to chain
            ui.label("Select layers (root to tip):");
            for (i, layer) in sprite.layers.iter().enumerate() {
                if layer.socket.is_some() || i == 0 {
                    let is_in_chain = sidebar_state.ik_chain_layer_ids.contains(&layer.id);
                    let mut checked = is_in_chain;
                    if ui.checkbox(&mut checked, &layer.name).changed() {
                        if checked {
                            sidebar_state.ik_chain_layer_ids.push(layer.id.clone());
                        } else {
                            sidebar_state.ik_chain_layer_ids.retain(|id| id != &layer.id);
                        }
                    }
                }
            }

            ui.label(format!("Selected: {} layers", sidebar_state.ik_chain_layer_ids.len()));

            ui.horizontal(|ui| {
                let can_create = sidebar_state.ik_chain_layer_ids.len() >= 2
                    && !sidebar_state.ik_chain_name_buffer.is_empty();
                if ui.add_enabled(can_create, egui::Button::new("Create")).clicked() {
                    actions.push(SidebarAction::CreateIKChain {
                        layer_ids: sidebar_state.ik_chain_layer_ids.clone(),
                        name: sidebar_state.ik_chain_name_buffer.clone(),
                    });
                    sidebar_state.ik_chain_creating = false;
                    sidebar_state.ik_chain_layer_ids.clear();
                    sidebar_state.ik_chain_name_buffer.clear();
                }
                if ui.button("Cancel").clicked() {
                    sidebar_state.ik_chain_creating = false;
                    sidebar_state.ik_chain_layer_ids.clear();
                    sidebar_state.ik_chain_name_buffer.clear();
                }
            });
        });
    } else if ui.button("+ New IK Chain").clicked() {
        sidebar_state.ik_chain_creating = true;
        sidebar_state.ik_chain_name_buffer = format!("IK Chain {}", seq.ik_chains.len() + 1);
    }
}

fn draw_active_color_display(
    ui: &mut egui::Ui,
    editor_state: &EditorState,
    palette: &Palette,
) {
    ui.label("Active Color");
    if let Some(color) = palette.colors.get(editor_state.active_color_index) {
        let color32 = color.to_color32();
        let (rect, _) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 2.0, color32);
        if color32.a() == 0 {
            // Draw an X for transparent
            let stroke = egui::Stroke::new(1.0, egui::Color32::GRAY);
            ui.painter()
                .line_segment([rect.left_top(), rect.right_bottom()], stroke);
            ui.painter()
                .line_segment([rect.right_top(), rect.left_bottom()], stroke);
        }
    }
}

fn draw_layers_panel(
    ui: &mut egui::Ui,
    sprite: &Sprite,
    active_layer_index: usize,
    actions: &mut Vec<SidebarAction>,
    sidebar_state: &mut SidebarState,
) {
    // Top action buttons
    ui.horizontal(|ui| {
        if ui.button("+ Add").clicked() {
            actions.push(SidebarAction::AddLayer);
        }
        if sprite.layers.len() > 1
            && ui.button("- Remove").clicked() {
                actions.push(SidebarAction::RemoveLayer(active_layer_index));
            }
    });

    ui.horizontal(|ui| {
        if ui.button("Duplicate").clicked() {
            actions.push(SidebarAction::DuplicateLayer(active_layer_index));
        }
        if ui.button("Combine").on_hover_text("Merge into layer above").clicked() {
            actions.push(SidebarAction::CombineLayerDown(active_layer_index));
        }
    });

    ui.horizontal(|ui| {
        if ui.button("Mirror H").on_hover_text("Flip horizontally").clicked() {
            actions.push(SidebarAction::MirrorLayerH(active_layer_index));
        }
        if ui.button("Mirror V").on_hover_text("Flip vertically").clicked() {
            actions.push(SidebarAction::MirrorLayerV(active_layer_index));
        }
    });

    ui.horizontal(|ui| {
        let can_move_up = active_layer_index + 1 < sprite.layers.len();
        let can_move_down = active_layer_index > 0;

        if ui.add_enabled(can_move_up, egui::Button::new("\u{2191} Up")).clicked() {
            actions.push(SidebarAction::MoveLayerUp(active_layer_index));
        }
        if ui.add_enabled(can_move_down, egui::Button::new("\u{2193} Down")).clicked() {
            actions.push(SidebarAction::MoveLayerDown(active_layer_index));
        }
    });

    ui.add_space(4.0);

    // Layer list (bottom to top for natural ordering)
    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            for (i, layer) in sprite.layers.iter().enumerate().rev() {
                let is_active = i == active_layer_index;
                let mut frame = egui::Frame::NONE.inner_margin(egui::Margin::same(4));
                if is_active {
                    frame = frame.fill(ui.visuals().selection.bg_fill);
                }

                frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Visibility toggle
                        let vis_label = if layer.visible { "\u{25C9}" } else { "\u{25CE}" };
                        if ui.small_button(vis_label).clicked() {
                            actions.push(SidebarAction::ToggleLayerVisibility(i));
                        }

                        // Lock toggle
                        let lock_label = if layer.locked { "\u{1F512}" } else { "\u{1F513}" };
                        if ui.small_button(lock_label).clicked() {
                            actions.push(SidebarAction::ToggleLayerLock(i));
                        }

                        // Socket indicator
                        if layer.socket.is_some() {
                            ui.label("\u{1F517}"); // chain link icon
                        }

                        // Layer name (clickable to select)
                        let label = format!(
                            "{} ({} elements)",
                            layer.name,
                            layer.elements.len()
                        );
                        if ui
                            .selectable_label(is_active, label)
                            .clicked()
                        {
                            actions.push(SidebarAction::SetActiveLayer(i));
                        }
                    });

                    // Socket info for active layer
                    if is_active {
                        draw_layer_socket_ui(ui, sprite, i, actions, sidebar_state);
                    }
                });
            }
        });
}

/// Draw socket UI for the active layer
fn draw_layer_socket_ui(
    ui: &mut egui::Ui,
    sprite: &Sprite,
    layer_index: usize,
    actions: &mut Vec<SidebarAction>,
    sidebar_state: &mut SidebarState,
) {
    let layer = &sprite.layers[layer_index];

    ui.indent(format!("socket_ui_{}", layer_index), |ui| {
        if let Some(ref socket_info) = layer.socket {
            // Show current socket parent info
            let parent_info = get_socket_parent_display(sprite, &socket_info.parent_element_id, &socket_info.parent_vertex_id);
            ui.label(format!("Socket: {}", parent_info));

            if ui.small_button("Clear Socket").on_hover_text("Detach from parent (snap to world position)").clicked() {
                actions.push(SidebarAction::ClearSocket(layer_index));
                sidebar_state.socket_picker_layer = None;
            }
        } else {
            ui.label("Socket: None");

            let picker_open = sidebar_state.socket_picker_layer == Some(layer_index);

            if picker_open {
                if ui.small_button("Cancel").clicked() {
                    sidebar_state.socket_picker_layer = None;
                }

                // Show list of available socket targets
                let targets = socket::get_all_socket_targets(sprite, &layer.id);

                if targets.is_empty() {
                    ui.label("No vertices available on other layers");
                } else {
                    ui.label("Pick a parent vertex:");
                    egui::ScrollArea::vertical()
                        .id_salt(format!("socket_picker_{}", layer_index))
                        .max_height(120.0)
                        .show(ui, |ui| {
                            for (_target_layer_id, target_layer_name, target_elem_id, target_elem_name, target_vtx_id, target_vtx_pos) in &targets {
                                let label = format!(
                                    "{} / {} @ ({:.0}, {:.0})",
                                    target_layer_name, target_elem_name, target_vtx_pos.x, target_vtx_pos.y
                                );
                                if ui.small_button(&label).clicked() {
                                    // Check for circular reference
                                    if socket::would_create_cycle(sprite, &layer.id, target_elem_id) {
                                        // Reject: show nothing, just don't assign
                                        // In a real UI we'd show a toast, but we handle that in main.rs
                                    } else {
                                        actions.push(SidebarAction::SetSocket {
                                            layer_index,
                                            parent_element_id: target_elem_id.clone(),
                                            parent_vertex_id: target_vtx_id.clone(),
                                        });
                                    }
                                    sidebar_state.socket_picker_layer = None;
                                }
                            }
                        });
                }
            } else if ui.small_button("Set Socket").on_hover_text("Attach this layer to a parent vertex").clicked() {
                sidebar_state.socket_picker_layer = Some(layer_index);
            }
        }
    });
}

/// Get a human-readable name for the socket parent
fn get_socket_parent_display(sprite: &Sprite, element_id: &str, vertex_id: &str) -> String {
    for layer in &sprite.layers {
        for element in &layer.elements {
            if element.id == element_id {
                let elem_name = element.name.clone().unwrap_or_else(|| {
                    format!("Elem {}", &element.id[..6.min(element.id.len())])
                });
                for vertex in &element.vertices {
                    if vertex.id == vertex_id {
                        return format!(
                            "{}/{} @ ({:.0}, {:.0})",
                            layer.name, elem_name, vertex.pos.x, vertex.pos.y
                        );
                    }
                }
                return format!("{}/{} (vertex missing)", layer.name, elem_name);
            }
        }
    }
    "Unknown (element missing)".to_string()
}

fn draw_palette_panel(
    ui: &mut egui::Ui,
    palette: &Palette,
    active_color_index: usize,
    actions: &mut Vec<SidebarAction>,
    sidebar_state: &mut SidebarState,
) {
    ui.label(format!("Palette: {} ({} colors)", palette.name, palette.colors.len()));
    ui.add_space(4.0);

    // Add/Delete color buttons
    ui.horizontal(|ui| {
        if palette.colors.len() < 256 {
            if ui.button("+ Add Color").clicked() {
                actions.push(SidebarAction::AddPaletteColor(PaletteColor {
                    hex: "808080ff".to_string(),
                    name: None,
                }));
            }
        } else {
            ui.label("Palette full (256 max)");
        }

        // Don't allow deleting index 0 (transparent)
        if active_color_index > 0 && active_color_index < palette.colors.len()
            && ui.button("- Delete").clicked() {
                actions.push(SidebarAction::DeletePaletteColor(active_color_index));
            }
    });

    ui.add_space(4.0);

    // Color swatches grid
    let swatch_size = 24.0;
    let swatches_per_row = 8;

    egui::Grid::new("palette_grid")
        .spacing(egui::vec2(2.0, 2.0))
        .show(ui, |ui| {
            for (i, color) in palette.colors.iter().enumerate() {
                let color32 = color.to_color32();
                let is_active = i == active_color_index;

                let (rect, response) = ui.allocate_exact_size(
                    egui::vec2(swatch_size, swatch_size),
                    egui::Sense::click(),
                );

                // Draw swatch
                if color32.a() == 0 {
                    // Checkerboard for transparent
                    ui.painter().rect_filled(rect, 2.0, egui::Color32::WHITE);
                    let half = rect.width() / 2.0;
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(rect.min, egui::vec2(half, half)),
                        0.0,
                        egui::Color32::LIGHT_GRAY,
                    );
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(
                            rect.min + egui::vec2(half, half),
                            egui::vec2(half, half),
                        ),
                        0.0,
                        egui::Color32::LIGHT_GRAY,
                    );
                } else {
                    ui.painter().rect_filled(rect, 2.0, color32);
                }

                // Active indicator
                if is_active {
                    ui.painter().rect_stroke(
                        rect.expand(2.0),
                        2.0,
                        egui::Stroke::new(2.0, egui::Color32::WHITE),
                        egui::epaint::StrokeKind::Outside,
                    );
                }

                if response.clicked() {
                    actions.push(SidebarAction::SetActiveColor(i));
                }

                // Double-click to edit color
                if response.double_clicked() && i > 0 {
                    sidebar_state.editing_color_index = Some(i);
                    let c = color.to_color32();
                    sidebar_state.editing_color_rgb = [c.r(), c.g(), c.b()];
                }

                // Show tooltip with color info
                response.on_hover_text(format!(
                    "#{} {}",
                    color.hex,
                    color.name.as_deref().unwrap_or("")
                ));

                if (i + 1) % swatches_per_row == 0 {
                    ui.end_row();
                }
            }
        });

    ui.add_space(8.0);

    // RGB color editor (shown when editing a color)
    if let Some(edit_idx) = sidebar_state.editing_color_index {
        if edit_idx < palette.colors.len() {
            ui.separator();
            ui.label(format!("Edit Color #{}", edit_idx));

            let mut changed = false;

            let mut r = sidebar_state.editing_color_rgb[0] as f32;
            if ui.add(egui::Slider::new(&mut r, 0.0..=255.0).text("R")).changed() {
                sidebar_state.editing_color_rgb[0] = r as u8;
                changed = true;
            }

            let mut g = sidebar_state.editing_color_rgb[1] as f32;
            if ui.add(egui::Slider::new(&mut g, 0.0..=255.0).text("G")).changed() {
                sidebar_state.editing_color_rgb[1] = g as u8;
                changed = true;
            }

            let mut b = sidebar_state.editing_color_rgb[2] as f32;
            if ui.add(egui::Slider::new(&mut b, 0.0..=255.0).text("B")).changed() {
                sidebar_state.editing_color_rgb[2] = b as u8;
                changed = true;
            }

            // Preview
            let preview_color = egui::Color32::from_rgb(
                sidebar_state.editing_color_rgb[0],
                sidebar_state.editing_color_rgb[1],
                sidebar_state.editing_color_rgb[2],
            );
            let (preview_rect, _) = ui.allocate_exact_size(egui::vec2(48.0, 24.0), egui::Sense::hover());
            ui.painter().rect_filled(preview_rect, 2.0, preview_color);

            if changed {
                let hex = format!(
                    "{:02x}{:02x}{:02x}ff",
                    sidebar_state.editing_color_rgb[0],
                    sidebar_state.editing_color_rgb[1],
                    sidebar_state.editing_color_rgb[2],
                );
                actions.push(SidebarAction::UpdatePaletteColor(
                    edit_idx,
                    PaletteColor {
                        hex,
                        name: palette.colors[edit_idx].name.clone(),
                    },
                ));
            }

            if ui.button("Done").clicked() {
                sidebar_state.editing_color_index = None;
            }
        } else {
            sidebar_state.editing_color_index = None;
        }
    }

    ui.add_space(8.0);
    ui.separator();

    // Lospec importer
    ui.label("Lospec Import");
    ui.horizontal(|ui| {
        ui.text_edit_singleline(&mut sidebar_state.lospec_slug);
        if ui.button("Import").clicked() && !sidebar_state.lospec_slug.is_empty() {
            let slug = sidebar_state.lospec_slug.clone();
            actions.push(SidebarAction::ImportLospecPalette(slug));
        }
    });

    if let Some(ref err) = sidebar_state.lospec_error {
        ui.colored_label(egui::Color32::RED, err);
    }
}

fn draw_skins_panel(
    ui: &mut egui::Ui,
    sprite: &Sprite,
    editor_state: &EditorState,
    palette: &Palette,
    actions: &mut Vec<SidebarAction>,
    sidebar_state: &mut SidebarState,
) {
    ui.label(format!("Skins ({})", sprite.skins.len()));
    ui.add_space(4.0);

    // Create new skin button
    ui.horizontal(|ui| {
        if ui.button("+ New Skin").clicked() {
            actions.push(SidebarAction::CreateSkin);
        }
    });

    ui.add_space(4.0);

    // Active skin selector: "Default (Base)" + all skins
    {
        let current_label = match &editor_state.active_skin_id {
            None => "Default (Base)".to_string(),
            Some(id) => sprite
                .skins
                .iter()
                .find(|s| s.id == *id)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| "Unknown".to_string()),
        };

        egui::ComboBox::from_id_salt("active_skin_selector")
            .selected_text(&current_label)
            .width(160.0)
            .show_ui(ui, |ui| {
                let is_default = editor_state.active_skin_id.is_none();
                if ui.selectable_label(is_default, "Default (Base)").clicked() {
                    actions.push(SidebarAction::SetActiveSkin(None));
                }
                for skin in &sprite.skins {
                    let is_selected = editor_state.active_skin_id.as_ref() == Some(&skin.id);
                    if ui.selectable_label(is_selected, &skin.name).clicked() {
                        actions.push(SidebarAction::SetActiveSkin(Some(skin.id.clone())));
                    }
                }
            });
    }

    ui.add_space(4.0);
    ui.separator();

    // Skin list
    egui::ScrollArea::vertical()
        .id_salt("skins_list_scroll")
        .auto_shrink([false; 2])
        .max_height(200.0)
        .show(ui, |ui| {
            let mut skin_to_finish_rename: Option<(String, String)> = None;

            for skin in &sprite.skins {
                let is_active = editor_state.active_skin_id.as_ref() == Some(&skin.id);
                let mut frame = egui::Frame::NONE.inner_margin(egui::Margin::same(4));
                if is_active {
                    frame = frame.fill(ui.visuals().selection.bg_fill);
                }

                frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Check if this skin is being renamed
                        if sidebar_state.renaming_skin_id.as_ref() == Some(&skin.id) {
                            let response = ui.text_edit_singleline(&mut sidebar_state.skin_rename_buffer);
                            if response.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                let new_name = sidebar_state.skin_rename_buffer.clone();
                                if !new_name.is_empty() {
                                    skin_to_finish_rename = Some((skin.id.clone(), new_name));
                                }
                                sidebar_state.renaming_skin_id = None;
                            }
                            if ui.small_button("Cancel").clicked() {
                                sidebar_state.renaming_skin_id = None;
                            }
                        } else {
                            // Skin name (clickable to select)
                            let label = format!("{} ({} overrides)", skin.name, skin.overrides.len());
                            let response = ui.selectable_label(is_active, label);
                            if response.clicked() {
                                actions.push(SidebarAction::SetActiveSkin(Some(skin.id.clone())));
                            }

                            // Context menu for rename/duplicate/delete
                            response.context_menu(|ui| {
                                if ui.button("Rename").clicked() {
                                    sidebar_state.renaming_skin_id = Some(skin.id.clone());
                                    sidebar_state.skin_rename_buffer = skin.name.clone();
                                    ui.close_menu();
                                }
                                if ui.button("Duplicate").clicked() {
                                    actions.push(SidebarAction::DuplicateSkin(skin.id.clone()));
                                    ui.close_menu();
                                }
                                if ui.button("Delete").clicked() {
                                    actions.push(SidebarAction::DeleteSkin(skin.id.clone()));
                                    ui.close_menu();
                                }
                            });
                        }
                    });
                });
            }

            // Process deferred rename
            if let Some((id, new_name)) = skin_to_finish_rename {
                actions.push(SidebarAction::RenameSkin(id, new_name));
            }
        });

    // Per-element override editor: only shown when a skin is active and element(s) are selected
    if let Some(ref active_skin_id) = editor_state.active_skin_id
        && let Some(skin) = sprite.skins.iter().find(|s| s.id == *active_skin_id) {
            if !editor_state.selection.selected_element_ids.is_empty() {
                ui.separator();
                ui.label("Skin Overrides");
                ui.add_space(4.0);

                // Show override editor for each selected element
                for selected_elem_id in &editor_state.selection.selected_element_ids {
                    // Find the base element to get its current values
                    let base_element = sprite.layers.iter()
                        .flat_map(|l| l.elements.iter())
                        .find(|e| e.id == *selected_elem_id);

                    let Some(base_elem) = base_element else { continue };

                    let elem_name = base_elem.name.clone().unwrap_or_else(|| {
                        format!("Elem {}", &base_elem.id[..6.min(base_elem.id.len())])
                    });

                    // Find existing override for this element
                    let existing_override = skin.overrides.iter()
                        .find(|o| o.element_id == *selected_elem_id);

                    ui.group(|ui| {
                        ui.label(format!("Element: {}", elem_name));

                        let has_stroke_color_override = existing_override
                            .and_then(|o| o.stroke_color_index)
                            .is_some();
                        let has_fill_color_override = existing_override
                            .and_then(|o| o.fill_color_index)
                            .is_some();
                        let has_stroke_width_override = existing_override
                            .and_then(|o| o.stroke_width)
                            .is_some();

                        let current_stroke_color = existing_override
                            .and_then(|o| o.stroke_color_index)
                            .unwrap_or(base_elem.stroke_color_index);
                        let current_fill_color = existing_override
                            .and_then(|o| o.fill_color_index)
                            .unwrap_or(base_elem.fill_color_index);
                        let current_stroke_width = existing_override
                            .and_then(|o| o.stroke_width)
                            .unwrap_or(base_elem.stroke_width);

                        let mut changed = false;
                        let mut new_stroke_color_idx: Option<usize> = existing_override.and_then(|o| o.stroke_color_index);
                        let mut new_fill_color_idx: Option<usize> = existing_override.and_then(|o| o.fill_color_index);
                        let mut new_stroke_width: Option<f32> = existing_override.and_then(|o| o.stroke_width);

                        // Stroke Color Override
                        ui.horizontal(|ui| {
                            let mut override_on = has_stroke_color_override;
                            if ui.checkbox(&mut override_on, "Stroke Color").changed() {
                                changed = true;
                                if override_on {
                                    new_stroke_color_idx = Some(current_stroke_color);
                                } else {
                                    new_stroke_color_idx = None;
                                }
                            }

                            if override_on {
                                // Show a few swatches for quick picking
                                let max_show = palette.colors.len().min(16);
                                for (ci, color) in palette.colors.iter().enumerate().take(max_show) {
                                    let color32 = color.to_color32();
                                    let is_current = ci == current_stroke_color;
                                    let (rect, resp) = ui.allocate_exact_size(
                                        egui::vec2(14.0, 14.0),
                                        egui::Sense::click(),
                                    );
                                    if color32.a() == 0 {
                                        ui.painter().rect_filled(rect, 1.0, egui::Color32::WHITE);
                                        let s = egui::Stroke::new(1.0, egui::Color32::GRAY);
                                        ui.painter().line_segment([rect.left_top(), rect.right_bottom()], s);
                                    } else {
                                        ui.painter().rect_filled(rect, 1.0, color32);
                                    }
                                    if is_current {
                                        ui.painter().rect_stroke(
                                            rect.expand(1.0), 1.0,
                                            egui::Stroke::new(2.0, egui::Color32::WHITE),
                                            egui::epaint::StrokeKind::Outside,
                                        );
                                    }
                                    if resp.clicked() {
                                        new_stroke_color_idx = Some(ci);
                                        changed = true;
                                    }
                                }
                            }
                        });

                        // Fill Color Override
                        ui.horizontal(|ui| {
                            let mut override_on = has_fill_color_override;
                            if ui.checkbox(&mut override_on, "Fill Color").changed() {
                                changed = true;
                                if override_on {
                                    new_fill_color_idx = Some(current_fill_color);
                                } else {
                                    new_fill_color_idx = None;
                                }
                            }

                            if override_on {
                                let max_show = palette.colors.len().min(16);
                                for (ci, color) in palette.colors.iter().enumerate().take(max_show) {
                                    let color32 = color.to_color32();
                                    let is_current = ci == current_fill_color;
                                    let (rect, resp) = ui.allocate_exact_size(
                                        egui::vec2(14.0, 14.0),
                                        egui::Sense::click(),
                                    );
                                    if color32.a() == 0 {
                                        ui.painter().rect_filled(rect, 1.0, egui::Color32::WHITE);
                                        let s = egui::Stroke::new(1.0, egui::Color32::GRAY);
                                        ui.painter().line_segment([rect.left_top(), rect.right_bottom()], s);
                                    } else {
                                        ui.painter().rect_filled(rect, 1.0, color32);
                                    }
                                    if is_current {
                                        ui.painter().rect_stroke(
                                            rect.expand(1.0), 1.0,
                                            egui::Stroke::new(2.0, egui::Color32::WHITE),
                                            egui::epaint::StrokeKind::Outside,
                                        );
                                    }
                                    if resp.clicked() {
                                        new_fill_color_idx = Some(ci);
                                        changed = true;
                                    }
                                }
                            }
                        });

                        // Stroke Width Override
                        ui.horizontal(|ui| {
                            let mut override_on = has_stroke_width_override;
                            if ui.checkbox(&mut override_on, "Stroke Width").changed() {
                                changed = true;
                                if override_on {
                                    new_stroke_width = Some(current_stroke_width);
                                } else {
                                    new_stroke_width = None;
                                }
                            }

                            if override_on {
                                let mut w = new_stroke_width.unwrap_or(current_stroke_width);
                                if ui.add(egui::Slider::new(&mut w, 0.5..=20.0).text("px")).changed() {
                                    new_stroke_width = Some(w);
                                    changed = true;
                                }
                            }
                        });

                        if changed {
                            // If all overrides are None, remove the override entry
                            if new_stroke_color_idx.is_none()
                                && new_fill_color_idx.is_none()
                                && new_stroke_width.is_none()
                            {
                                actions.push(SidebarAction::RemoveSkinOverride {
                                    skin_id: active_skin_id.clone(),
                                    element_id: selected_elem_id.clone(),
                                });
                            } else {
                                actions.push(SidebarAction::SetSkinOverride {
                                    skin_id: active_skin_id.clone(),
                                    element_id: selected_elem_id.clone(),
                                    stroke_color_index: new_stroke_color_idx,
                                    fill_color_index: new_fill_color_idx,
                                    stroke_width: new_stroke_width,
                                });
                            }
                        }
                    });

                    ui.add_space(2.0);
                }
            } else {
                ui.separator();
                ui.label("Select an element to edit skin overrides");
            }
        }
}

fn draw_settings_panel(
    ui: &mut egui::Ui,
    grid_mode: GridMode,
    sprite: &Sprite,
    actions: &mut Vec<SidebarAction>,
) {
    ui.label("Canvas Size");
    let mut w = sprite.canvas_width as f32;
    let mut h = sprite.canvas_height as f32;
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("W:");
        if ui.add(egui::DragValue::new(&mut w).range(1..=4096).speed(1.0)).changed() {
            changed = true;
        }
        ui.label("H:");
        if ui.add(egui::DragValue::new(&mut h).range(1..=4096).speed(1.0)).changed() {
            changed = true;
        }
    });
    if changed {
        actions.push(SidebarAction::ResizeCanvas(w as u32, h as u32));
    }

    ui.add_space(8.0);

    ui.label("Grid Mode");
    ui.horizontal(|ui| {
        if ui.selectable_label(grid_mode == GridMode::Standard, "Standard").clicked() {
            actions.push(SidebarAction::SetGridMode(GridMode::Standard));
        }
        if ui.selectable_label(grid_mode == GridMode::Isometric, "Isometric").clicked() {
            actions.push(SidebarAction::SetGridMode(GridMode::Isometric));
        }
    });

    ui.add_space(8.0);

    ui.label("Theme");
    if ui.button("Toggle Dark/Light").clicked() {
        actions.push(SidebarAction::ToggleTheme);
    }
}

/// Draw constraints UI for the active layer in the select tool panel.
fn draw_constraints_ui(
    ui: &mut egui::Ui,
    sprite: &Sprite,
    editor_state: &EditorState,
    actions: &mut Vec<SidebarAction>,
) {
    let layer_idx = editor_state.active_layer_index;
    let Some(layer) = sprite.layers.get(layer_idx) else {
        return;
    };

    ui.separator();
    ui.label("Constraints");
    ui.add_space(2.0);

    let mut constraints = layer.constraints.clone();
    let mut changed = false;

    // --- Volume Preserve ---
    ui.horizontal(|ui| {
        let mut vp = constraints.volume_preserve;
        if ui.checkbox(&mut vp, "Volume Preserve").changed() {
            constraints.volume_preserve = vp;
            changed = true;
        }
    });

    // --- Physics ---
    let has_physics = constraints.physics.is_some();
    let mut show_physics = has_physics;
    if ui.checkbox(&mut show_physics, "Spring Physics").changed() {
        if show_physics && !has_physics {
            constraints.physics = Some(PhysicsConstraint::default());
        } else if !show_physics {
            constraints.physics = None;
        }
        changed = true;
    }

    if let Some(ref mut phys) = constraints.physics {
        egui::CollapsingHeader::new("Physics Settings")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Freq:");
                    if ui
                        .add(egui::DragValue::new(&mut phys.frequency).range(0.1..=10.0).speed(0.1))
                        .changed()
                    {
                        changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Damp:");
                    if ui
                        .add(egui::DragValue::new(&mut phys.damping).range(0.0..=2.0).speed(0.05))
                        .changed()
                    {
                        changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Mix:");
                    if ui
                        .add(egui::Slider::new(&mut phys.mix, 0.0..=1.0))
                        .changed()
                    {
                        changed = true;
                    }
                });

                // Gravity
                let has_gravity = phys.gravity.is_some();
                let mut show_gravity = has_gravity;
                if ui.checkbox(&mut show_gravity, "Gravity").changed() {
                    if show_gravity && !has_gravity {
                        phys.gravity = Some(GravityForce::default());
                    } else if !show_gravity {
                        phys.gravity = None;
                    }
                    changed = true;
                }
                if let Some(ref mut gravity) = phys.gravity {
                    ui.horizontal(|ui| {
                        ui.label("Angle:");
                        if ui
                            .add(egui::DragValue::new(&mut gravity.angle).range(0.0..=360.0).speed(1.0).suffix("\u{00B0}"))
                            .changed()
                        {
                            changed = true;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Strength:");
                        if ui
                            .add(egui::DragValue::new(&mut gravity.strength).range(0.0..=1000.0).speed(1.0))
                            .changed()
                        {
                            changed = true;
                        }
                    });
                }

                // Wind
                let has_wind = phys.wind.is_some();
                let mut show_wind = has_wind;
                if ui.checkbox(&mut show_wind, "Wind").changed() {
                    if show_wind && !has_wind {
                        phys.wind = Some(WindForce::default());
                    } else if !show_wind {
                        phys.wind = None;
                    }
                    changed = true;
                }
                if let Some(ref mut wind) = phys.wind {
                    ui.horizontal(|ui| {
                        ui.label("Strength:");
                        if ui
                            .add(egui::DragValue::new(&mut wind.strength).range(0.0..=500.0).speed(1.0))
                            .changed()
                        {
                            changed = true;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Freq:");
                        if ui
                            .add(egui::DragValue::new(&mut wind.frequency).range(0.01..=10.0).speed(0.1))
                            .changed()
                        {
                            changed = true;
                        }
                    });
                }
            });
    }

    // --- Look-At Constraint ---
    let has_look_at = constraints.look_at.is_some();
    let mut show_look_at = has_look_at;
    if ui.checkbox(&mut show_look_at, "Look-At").changed() {
        if show_look_at && !has_look_at {
            constraints.look_at = Some(LookAtConstraint::default());
        } else if !show_look_at {
            constraints.look_at = None;
        }
        changed = true;
    }

    if let Some(ref mut look_at) = constraints.look_at {
        egui::CollapsingHeader::new("Look-At Settings")
            .default_open(true)
            .show(ui, |ui| {
                // Target element picker
                ui.horizontal(|ui| {
                    ui.label("Target:");
                    let target_label = if look_at.target_element_id.is_empty() {
                        "None".to_string()
                    } else {
                        format!("...{}", &look_at.target_element_id[..6.min(look_at.target_element_id.len())])
                    };
                    egui::ComboBox::from_id_salt("look_at_target")
                        .selected_text(&target_label)
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(look_at.target_element_id.is_empty(), "None").clicked() {
                                look_at.target_element_id.clear();
                                look_at.target_vertex_id = None;
                                changed = true;
                            }
                            for other_layer in &sprite.layers {
                                if other_layer.id == layer.id {
                                    continue;
                                }
                                for elem in &other_layer.elements {
                                    let elem_label = elem.name.clone().unwrap_or_else(|| {
                                        format!("...{}", &elem.id[..6.min(elem.id.len())])
                                    });
                                    let selected = look_at.target_element_id == elem.id;
                                    if ui.selectable_label(selected, format!("{}: {}", other_layer.name, elem_label)).clicked() {
                                        look_at.target_element_id = elem.id.clone();
                                        look_at.target_vertex_id = None;
                                        changed = true;
                                    }
                                }
                            }
                        });
                });

                ui.horizontal(|ui| {
                    ui.label("Rest Angle:");
                    let mut rest_deg = look_at.rest_angle.to_degrees();
                    if ui
                        .add(egui::DragValue::new(&mut rest_deg).range(-180.0..=180.0).speed(1.0).suffix("\u{00B0}"))
                        .changed()
                    {
                        look_at.rest_angle = rest_deg.to_radians();
                        changed = true;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Min Angle:");
                    let mut min_deg = look_at.min_angle.to_degrees();
                    if ui
                        .add(egui::DragValue::new(&mut min_deg).range(-180.0..=180.0).speed(1.0).suffix("\u{00B0}"))
                        .changed()
                    {
                        look_at.min_angle = min_deg.to_radians();
                        changed = true;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Max Angle:");
                    let mut max_deg = look_at.max_angle.to_degrees();
                    if ui
                        .add(egui::DragValue::new(&mut max_deg).range(-180.0..=180.0).speed(1.0).suffix("\u{00B0}"))
                        .changed()
                    {
                        look_at.max_angle = max_deg.to_radians();
                        changed = true;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Mix:");
                    if ui
                        .add(egui::Slider::new(&mut look_at.mix, 0.0..=1.0))
                        .changed()
                    {
                        changed = true;
                    }
                });

                // Spring smoothing toggle
                let has_smooth = look_at.smooth.is_some();
                let mut show_smooth = has_smooth;
                if ui.checkbox(&mut show_smooth, "Spring Smoothing").changed() {
                    if show_smooth && !has_smooth {
                        look_at.smooth = Some(SpringSmoothing::default());
                    } else if !show_smooth {
                        look_at.smooth = None;
                    }
                    changed = true;
                }
                if let Some(ref mut smooth) = look_at.smooth {
                    ui.horizontal(|ui| {
                        ui.label("Freq:");
                        if ui
                            .add(egui::DragValue::new(&mut smooth.frequency).range(0.1..=20.0).speed(0.1))
                            .changed()
                        {
                            changed = true;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Damp:");
                        if ui
                            .add(egui::DragValue::new(&mut smooth.damping).range(0.0..=2.0).speed(0.05))
                            .changed()
                        {
                            changed = true;
                        }
                    });
                }
            });
    }

    // --- Procedural Modifiers ---
    egui::CollapsingHeader::new("Procedural Modifiers")
        .default_open(false)
        .show(ui, |ui| {
            let mut to_remove: Option<usize> = None;

            for (i, modifier) in constraints.procedural.iter_mut().enumerate() {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("#{}", i + 1));
                        if ui.small_button("X").clicked() {
                            to_remove = Some(i);
                            changed = true;
                        }
                    });

                    // Property selector
                    ui.horizontal(|ui| {
                        ui.label("Prop:");
                        let properties = ["position.x", "position.y", "rotation", "scale.x", "scale.y"];
                        egui::ComboBox::from_id_salt(format!("proc_prop_{}", i))
                            .selected_text(&modifier.property)
                            .show_ui(ui, |ui| {
                                for prop in &properties {
                                    if ui
                                        .selectable_label(modifier.property == *prop, *prop)
                                        .clicked()
                                    {
                                        modifier.property = prop.to_string();
                                        changed = true;
                                    }
                                }
                            });
                    });

                    // Waveform selector
                    ui.horizontal(|ui| {
                        ui.label("Wave:");
                        if ui.selectable_label(modifier.waveform == Waveform::Sine, "Sine").clicked() {
                            modifier.waveform = Waveform::Sine;
                            changed = true;
                        }
                        if ui.selectable_label(modifier.waveform == Waveform::Noise, "Noise").clicked() {
                            modifier.waveform = Waveform::Noise;
                            changed = true;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Amp:");
                        if ui
                            .add(egui::DragValue::new(&mut modifier.amplitude).speed(0.1))
                            .changed()
                        {
                            changed = true;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Freq:");
                        if ui
                            .add(egui::DragValue::new(&mut modifier.frequency).range(0.01..=50.0).speed(0.1))
                            .changed()
                        {
                            changed = true;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Phase:");
                        if ui
                            .add(egui::DragValue::new(&mut modifier.phase).range(0.0..=360.0).speed(1.0).suffix("\u{00B0}"))
                            .changed()
                        {
                            changed = true;
                        }
                    });

                    // Blend mode
                    ui.horizontal(|ui| {
                        ui.label("Blend:");
                        if ui.selectable_label(modifier.blend == BlendMode::Additive, "Add").clicked() {
                            modifier.blend = BlendMode::Additive;
                            changed = true;
                        }
                        if ui.selectable_label(modifier.blend == BlendMode::Multiplicative, "Mult").clicked() {
                            modifier.blend = BlendMode::Multiplicative;
                            changed = true;
                        }
                    });
                });
            }

            if let Some(idx) = to_remove {
                constraints.procedural.remove(idx);
            }

            if ui.button("+ Add Modifier").clicked() {
                constraints.procedural.push(ProceduralModifier::default());
                changed = true;
            }
        });

    if changed {
        actions.push(SidebarAction::SetLayerConstraints {
            layer_index: layer_idx,
            constraints,
        });
    }
}

/// Draw debug overlay toggles in the select tool panel.
fn draw_debug_overlays_ui(
    ui: &mut egui::Ui,
    editor_state: &EditorState,
    actions: &mut Vec<SidebarAction>,
) {
    ui.separator();
    egui::CollapsingHeader::new("Debug Overlays")
        .default_open(false)
        .show(ui, |ui| {
            if ui
                .checkbox(&mut editor_state.debug_overlays.show_bones.clone(), "Bone Chains")
                .clicked()
            {
                actions.push(SidebarAction::ToggleDebugBones);
            }
            if ui
                .checkbox(&mut editor_state.debug_overlays.show_ik_targets.clone(), "IK Targets")
                .clicked()
            {
                actions.push(SidebarAction::ToggleDebugIKTargets);
            }
            if ui
                .checkbox(&mut editor_state.debug_overlays.show_constraints.clone(), "Constraint Gizmos")
                .clicked()
            {
                actions.push(SidebarAction::ToggleDebugConstraints);
            }
            if ui
                .checkbox(&mut editor_state.debug_overlays.show_spring_targets.clone(), "Spring Targets")
                .clicked()
            {
                actions.push(SidebarAction::ToggleDebugSpringTargets);
            }
        });
}
