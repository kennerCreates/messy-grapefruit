use crate::model::Vec2;

#[derive(Debug, Clone, PartialEq)]
pub enum ToolKind {
    Line,
    Select,
    Fill,
    Eraser,
}

impl Default for ToolKind {
    fn default() -> Self { Self::Line }
}

pub struct EditorState {
    pub active_tool: ToolKind,
    pub active_color_index: usize,
    pub viewport: ViewportState,
    pub selection: SelectionState,
    pub line_tool_state: LineToolState,
    pub cursor_world_pos: Vec2,
    pub cursor_screen_pos: Vec2,
    pub curve_mode: bool,
    pub show_merge_preview: bool,
    pub merge_target: Option<MergeTarget>,
    pub active_layer_index: usize,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            active_tool: ToolKind::Line,
            active_color_index: 1,
            viewport: ViewportState::default(),
            selection: SelectionState::default(),
            line_tool_state: LineToolState::default(),
            cursor_world_pos: Vec2::default(),
            cursor_screen_pos: Vec2::default(),
            curve_mode: true,
            show_merge_preview: false,
            merge_target: None,
            active_layer_index: 0,
        }
    }
}

pub struct ViewportState {
    pub offset: Vec2,
    pub zoom: f32,
    pub zoom_min: f32,
    pub zoom_max: f32,
}

impl Default for ViewportState {
    fn default() -> Self {
        Self {
            offset: Vec2::default(),
            zoom: 1.0,
            zoom_min: 0.1,
            zoom_max: 20.0,
        }
    }
}

#[derive(Default)]
pub struct SelectionState {
    pub selected_elements: Vec<String>,
    pub selected_vertices: Vec<String>,
    pub marquee: Option<[Vec2; 2]>,  // top-left, bottom-right in world coords
    pub dragging: bool,
    pub drag_start: Option<Vec2>,
    pub transform_mode: TransformMode,
}

#[derive(Default, Debug, Clone, PartialEq)]
pub enum TransformMode {
    #[default]
    None,
    Move,
    Scale,
    Rotate,
}

#[derive(Default)]
pub struct LineToolState {
    pub active_element_id: Option<String>,
    pub preview_vertex: Option<Vec2>,
}

pub struct MergeTarget {
    pub element_id: String,
    pub vertex_id: String,
    pub position: Vec2,
}
