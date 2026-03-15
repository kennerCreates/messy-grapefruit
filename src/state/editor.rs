use crate::model::sprite::PathVertex;
use crate::model::vec2::Vec2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    Line,
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

#[derive(Debug, Clone)]
pub struct EditorState {
    pub tool: ToolKind,
    pub viewport: ViewportState,
    pub line_tool: LineToolState,
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
