use crate::model::sprite::PathVertex;
use crate::model::vec2::Vec2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    Select,
    Line,
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
        let padding = 0.9; // 90% fill
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

#[derive(Debug, Clone)]
pub struct EditorState {
    pub tool: ToolKind,
    pub viewport: ViewportState,
    pub line_tool: LineToolState,
    pub selection: SelectionState,
    pub select_drag: Option<SelectDragKind>,
    pub selection_stack_popup: Option<SelectionStackPopup>,
    pub active_stroke_width: f32,
    pub active_color_index: u8,
    pub active_layer_idx: usize,
    pub hover_element_id: Option<String>,
    pub zoom_to_fit_requested: bool,
    pub sidebar_expanded: bool,
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
            active_stroke_width: 2.0,
            active_color_index: 1, // black
            active_layer_idx: 0,
            hover_element_id: None,
            zoom_to_fit_requested: true,
            sidebar_expanded: false,
        }
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
