use crate::model::sprite::{PathVertex, Sprite};
use crate::model::vec2::Vec2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    Select,
    Line,
    Fill,
    Eyedropper,
}

#[derive(Debug, Clone, Default)]
pub struct SelectionState {
    pub selected_ids: Vec<String>,
}

impl SelectionState {
    pub fn is_selected(&self, id: &str) -> bool {
        self.selected_ids.iter().any(|s| s == id)
    }

    pub fn clear(&mut self) {
        self.selected_ids.clear();
    }

    pub fn select_single(&mut self, id: String) {
        self.selected_ids.clear();
        self.selected_ids.push(id);
    }

    pub fn toggle(&mut self, id: &str) {
        if let Some(pos) = self.selected_ids.iter().position(|s| s == id) {
            self.selected_ids.remove(pos);
        } else {
            self.selected_ids.push(id.to_string());
        }
    }

    pub fn select_all(&mut self, ids: Vec<String>) {
        self.selected_ids = ids;
    }

    pub fn is_empty(&self) -> bool {
        self.selected_ids.is_empty()
    }
}

/// Hover target for vertex editing sub-mode.
#[derive(Debug, Clone)]
pub enum VertexHover {
    Vertex { vertex_id: String },
    Handle { vertex_id: String, is_cp1: bool },
}

/// Which handle on the selection bounding box is being manipulated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandleKind {
    ScaleNW, ScaleN, ScaleNE,
    ScaleE, ScaleSE, ScaleS,
    ScaleSW, ScaleW,
    Rotate,
}

/// Active drag operation in select tool.
#[derive(Debug, Clone)]
pub enum SelectDragKind {
    /// Dragging selected elements to move them.
    Move {
        start_world: Vec2,
        last_snapped_delta: Vec2,
    },
    /// Dragging a marquee rectangle to select elements.
    Marquee {
        start_screen: egui::Pos2,
        start_world: Vec2,
    },
    /// Dragging a scale handle on the selection AABB.
    Scale {
        handle: HandleKind,
        /// AABB at drag start (min, max).
        initial_bounds: (Vec2, Vec2),
        /// Element scales at drag start, keyed by element ID.
        initial_scales: Vec<(String, Vec2)>,
        /// Element positions at drag start.
        initial_positions: Vec<(String, Vec2)>,
        /// The anchor point (opposite corner/edge) in world space.
        anchor: Vec2,
    },
    /// Dragging the rotation handle.
    Rotate {
        /// Center of rotation (AABB center).
        pivot: Vec2,
        /// Starting angle from pivot to cursor (radians).
        start_angle: f32,
        /// Element rotations at drag start.
        initial_rotations: Vec<(String, f32)>,
        /// Element positions at drag start.
        initial_positions: Vec<(String, Vec2)>,
    },
    /// Dragging a vertex to move it.
    VertexMove {
        element_id: String,
        vertex_id: String,
        start_world: Vec2,
        initial_local_pos: Vec2,
    },
    /// Dragging a control point handle.
    HandleMove {
        element_id: String,
        vertex_id: String,
        is_cp1: bool,
        start_world: Vec2,
        initial_local_pos: Vec2,
    },
}

#[derive(Debug, Clone)]
pub struct ViewportState {
    pub offset: Vec2,
    pub zoom: f32,
    pub flipped: bool,
}

impl Default for ViewportState {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            zoom: 1.0,
            flipped: false,
        }
    }
}

impl ViewportState {
    /// Convert world coordinates to screen coordinates.
    pub fn world_to_screen(&self, world: Vec2, canvas_center: egui::Pos2) -> egui::Pos2 {
        let mut x = (world.x + self.offset.x) * self.zoom;
        let y = (world.y + self.offset.y) * self.zoom;
        if self.flipped {
            x = -x;
        }
        egui::Pos2::new(canvas_center.x + x, canvas_center.y + y)
    }

    /// Convert screen coordinates to world coordinates.
    pub fn screen_to_world(&self, screen: egui::Pos2, canvas_center: egui::Pos2) -> Vec2 {
        let mut x = screen.x - canvas_center.x;
        if self.flipped {
            x = -x;
        }
        let y = screen.y - canvas_center.y;
        Vec2::new(x / self.zoom - self.offset.x, y / self.zoom - self.offset.y)
    }

    /// Zoom in/out centered on a screen position.
    pub fn zoom_at(&mut self, screen_pos: egui::Pos2, factor: f32, canvas_center: egui::Pos2) {
        let world_before = self.screen_to_world(screen_pos, canvas_center);
        self.zoom = (self.zoom * factor).clamp(0.1, 64.0);
        let world_after = self.screen_to_world(screen_pos, canvas_center);
        self.offset += world_after - world_before;
    }

    /// Adjust viewport to frame the given world-space bounds with padding.
    pub fn zoom_to_fit(&mut self, bounds_min: Vec2, bounds_max: Vec2, canvas_size: egui::Vec2) {
        let bounds_size = bounds_max - bounds_min;
        if bounds_size.x < 1.0 && bounds_size.y < 1.0 {
            return;
        }
        let padding = 0.75; // 75% fill
        let zoom_x = (canvas_size.x * padding) / bounds_size.x;
        let zoom_y = (canvas_size.y * padding) / bounds_size.y;
        self.zoom = zoom_x.min(zoom_y).clamp(0.1, 64.0);

        let center = (bounds_min + bounds_max) * 0.5;
        self.offset = -center;
    }
}

#[derive(Debug, Clone, Default)]
pub struct LineToolState {
    pub vertices: Vec<PathVertex>,
    pub curve_mode: bool,
    pub is_drawing: bool,
}

impl LineToolState {
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.is_drawing = false;
    }
}

/// Entry in the selection stack popup (Alt+click on overlapping elements).
#[derive(Debug, Clone)]
pub struct StackEntry {
    pub element_id: String,
    pub display_name: String,
    pub stroke_color_index: u8,
}

/// Popup showing all elements under the cursor for pick-selection.
#[derive(Debug, Clone)]
pub struct SelectionStackPopup {
    pub screen_pos: egui::Pos2,
    pub entries: Vec<StackEntry>,
}

/// Active brush/drawing properties.
#[derive(Debug, Clone)]
pub struct BrushState {
    pub stroke_width: f32,
    pub color_index: u8,
    pub fill_color_index: u8,
}

impl Default for BrushState {
    fn default() -> Self {
        Self {
            stroke_width: 2.0,
            color_index: 1, // black
            fill_color_index: 0, // transparent
        }
    }
}

/// Drag-reorder state for a layer being dragged in the layer panel.
#[derive(Debug, Clone)]
pub struct LayerDragState {
    /// ID of the layer being dragged.
    pub layer_id: String,
    /// Target insertion index (in `sprite.layers` order).
    pub target_idx: Option<usize>,
    /// Target group ID (None = ungrouped, Some = drop into group).
    pub target_group_id: Option<String>,
}

/// Layer panel state.
#[derive(Debug, Clone, Default)]
pub struct LayerState {
    /// ID of the active (selected) layer.
    pub active_layer_id: Option<String>,
    /// Layer ID currently in solo mode, if any.
    pub solo_layer_id: Option<String>,
    /// Layer ID being renamed (inline TextEdit), if any.
    pub renaming_layer_id: Option<String>,
    /// Drag-reorder state, if a layer is being dragged.
    pub drag_reorder: Option<LayerDragState>,
}

impl LayerState {
    /// Resolve the active layer index from the stored ID.
    /// Falls back to 0 if the ID is not found or not set.
    pub fn resolve_active_idx(&self, sprite: &Sprite) -> usize {
        if let Some(id) = &self.active_layer_id {
            sprite.layer_idx_by_id(id).unwrap_or(0)
        } else {
            0
        }
    }

    /// Set the active layer by index, storing the ID.
    pub fn set_active_by_idx(&mut self, idx: usize, sprite: &Sprite) {
        if let Some(layer) = sprite.layers.get(idx) {
            self.active_layer_id = Some(layer.id.clone());
        }
    }

    /// Ensure the active layer ID is valid; fix up if not.
    pub fn validate(&mut self, sprite: &Sprite) {
        if let Some(id) = &self.active_layer_id
            && sprite.layer_idx_by_id(id).is_some() {
                return;
            }
        // Fall back to first layer
        if let Some(first) = sprite.layers.first() {
            self.active_layer_id = Some(first.id.clone());
        } else {
            self.active_layer_id = None;
        }
    }
}

#[derive(Debug, Clone)]
pub struct EditorState {
    pub tool: ToolKind,
    pub viewport: ViewportState,
    pub line_tool: LineToolState,
    pub selection: SelectionState,
    pub select_drag: Option<SelectDragKind>,
    pub selection_stack_popup: Option<SelectionStackPopup>,
    pub brush: BrushState,
    pub layer: LayerState,
    pub hover_element_id: Option<String>,
    pub selected_vertex_id: Option<String>,
    pub hover_vertex: Option<VertexHover>,
    pub zoom_to_fit_requested: bool,
    pub sidebar_expanded: bool,
    /// Last 8 used color indices (session-only, most recent first).
    pub recent_colors: Vec<u8>,
    /// When set, eyedropper was activated temporarily (Alt+click) and should
    /// return to this tool after sampling.
    pub eyedropper_return_tool: Option<ToolKind>,
    /// Lospec import popup state: slug text input.
    pub lospec_slug: String,
    /// Lospec import error message, if any.
    pub lospec_error: Option<String>,
    /// Whether the Lospec import popup is open.
    pub lospec_popup_open: bool,
    /// Cached color ramp results (indices sorted by lightness).
    pub color_ramp: Vec<u8>,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            tool: ToolKind::Line,
            viewport: ViewportState::default(),
            line_tool: LineToolState {
                vertices: Vec::new(),
                curve_mode: true,
                is_drawing: false,
            },
            selection: SelectionState::default(),
            select_drag: None,
            selection_stack_popup: None,
            brush: BrushState::default(),
            layer: LayerState::default(),
            hover_element_id: None,
            selected_vertex_id: None,
            hover_vertex: None,
            zoom_to_fit_requested: true,
            sidebar_expanded: false,
            recent_colors: Vec::new(),
            eyedropper_return_tool: None,
            lospec_slug: String::new(),
            lospec_error: None,
            lospec_popup_open: false,
            color_ramp: Vec::new(),
        }
    }
}

impl EditorState {
    pub fn clear_vertex_selection(&mut self) {
        self.selected_vertex_id = None;
        self.hover_vertex = None;
    }

    /// Track a color index in the recent colors list (deduplicates, max 8).
    pub fn track_recent_color(&mut self, index: u8) {
        if index == 0 {
            return; // don't track transparent
        }
        self.recent_colors.retain(|&i| i != index);
        self.recent_colors.insert(0, index);
        self.recent_colors.truncate(8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_screen_round_trip() {
        let vp = ViewportState { offset: Vec2::new(10.0, 20.0), zoom: 2.0, flipped: false };
        let center = egui::Pos2::new(400.0, 300.0);
        let world = Vec2::new(5.0, 10.0);
        let screen = vp.world_to_screen(world, center);
        let back = vp.screen_to_world(screen, center);
        assert!((back.x - world.x).abs() < 1e-4);
        assert!((back.y - world.y).abs() < 1e-4);
    }

    #[test]
    fn test_world_screen_round_trip_flipped() {
        let vp = ViewportState { offset: Vec2::new(10.0, 20.0), zoom: 2.0, flipped: true };
        let center = egui::Pos2::new(400.0, 300.0);
        let world = Vec2::new(5.0, 10.0);
        let screen = vp.world_to_screen(world, center);
        let back = vp.screen_to_world(screen, center);
        assert!((back.x - world.x).abs() < 1e-4);
        assert!((back.y - world.y).abs() < 1e-4);
    }

    #[test]
    fn test_zoom_at_preserves_cursor_position() {
        let mut vp = ViewportState { offset: Vec2::ZERO, zoom: 1.0, flipped: false };
        let center = egui::Pos2::new(400.0, 300.0);
        let cursor = egui::Pos2::new(500.0, 350.0);
        let world_before = vp.screen_to_world(cursor, center);
        vp.zoom_at(cursor, 2.0, center);
        let world_after = vp.screen_to_world(cursor, center);
        assert!((world_before.x - world_after.x).abs() < 1e-3);
        assert!((world_before.y - world_after.y).abs() < 1e-3);
    }
}
