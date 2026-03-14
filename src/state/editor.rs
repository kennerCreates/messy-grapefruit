use crate::model::Vec2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    Line,
    Select,
    Fill,
    Eraser,
}

impl ToolKind {
    pub fn name(&self) -> &'static str {
        match self {
            ToolKind::Line => "Line",
            ToolKind::Select => "Select",
            ToolKind::Fill => "Fill",
            ToolKind::Eraser => "Eraser",
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformMode {
    None,
    Move,
    Scale,
    Rotate,
}

#[derive(Debug, Clone)]
pub struct ViewportState {
    pub offset: Vec2,
    pub zoom: f32,
    pub zoom_min: f32,
    pub zoom_max: f32,
}

impl Default for ViewportState {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            zoom: 1.0,
            zoom_min: 0.1,
            zoom_max: 32.0,
        }
    }
}

impl ViewportState {
    /// Convert world coordinates to screen coordinates
    pub fn world_to_screen(&self, world_pos: Vec2, canvas_center: Vec2) -> Vec2 {
        Vec2 {
            x: (world_pos.x + self.offset.x) * self.zoom + canvas_center.x,
            y: (world_pos.y + self.offset.y) * self.zoom + canvas_center.y,
        }
    }

    /// Convert screen coordinates to world coordinates
    pub fn screen_to_world(&self, screen_pos: Vec2, canvas_center: Vec2) -> Vec2 {
        Vec2 {
            x: (screen_pos.x - canvas_center.x) / self.zoom - self.offset.x,
            y: (screen_pos.y - canvas_center.y) / self.zoom - self.offset.y,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SelectionState {
    pub selected_element_ids: Vec<String>,
    pub selected_vertex_ids: Vec<String>,
}

impl SelectionState {
    pub fn clear(&mut self) {
        self.selected_element_ids.clear();
        self.selected_vertex_ids.clear();
    }

    pub fn is_element_selected(&self, id: &str) -> bool {
        self.selected_element_ids.iter().any(|s| s == id)
    }

    #[allow(dead_code)]
    pub fn is_vertex_selected(&self, id: &str) -> bool {
        self.selected_vertex_ids.iter().any(|s| s == id)
    }

    pub fn toggle_element(&mut self, id: &str) {
        if let Some(pos) = self.selected_element_ids.iter().position(|s| s == id) {
            self.selected_element_ids.remove(pos);
        } else {
            self.selected_element_ids.push(id.to_string());
        }
    }

    pub fn select_element(&mut self, id: &str) {
        if !self.is_element_selected(id) {
            self.selected_element_ids.push(id.to_string());
        }
    }
}

#[derive(Debug, Clone)]
#[derive(Default)]
pub struct LineToolState {
    /// The element currently being drawn (None = not drawing)
    pub active_element_id: Option<String>,
}


/// State for the select tool's drag-to-move functionality
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct SelectDragState {
    pub is_dragging: bool,
    pub drag_start_world: Option<Vec2>,
    pub drag_last_world: Option<Vec2>,
}


/// State for marquee (rectangular area) selection
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct MarqueeState {
    pub is_active: bool,
    pub start_world: Option<Vec2>,
    pub current_world: Option<Vec2>,
}


/// State for dragging a control point handle (cp1/cp2)
#[derive(Debug, Clone, Default)]
pub struct HandleDragState {
    /// Whether a handle is currently being dragged
    pub is_dragging: bool,
    /// Which element owns the vertex with the handle
    pub element_id: Option<String>,
    /// Which vertex owns the handle
    pub vertex_id: Option<String>,
    /// Which handle: true = cp1 (incoming), false = cp2 (outgoing)
    pub is_cp1: bool,
    /// The original handle position before drag (for undo)
    pub original_pos: Option<Vec2>,
}

/// State for scale/rotate transform handles
#[derive(Debug, Clone, Default)]
pub struct TransformHandleState {
    /// Whether a transform handle is being dragged
    pub is_dragging: bool,
    /// Which type of transform: "scale" or "rotate"
    pub kind: TransformHandleKind,
    /// Which corner (0=TL, 1=TR, 2=BR, 3=BL) for scale, or rotation start angle
    pub handle_index: usize,
    /// Starting mouse position in world space
    pub start_world: Option<Vec2>,
    /// Starting rotation angle (for rotate mode)
    pub start_angle: f32,
    /// Bounding box center (pivot point)
    pub pivot: Option<Vec2>,
    /// Snapshot of sprite before transform started (for undo)
    pub before_snapshot: Option<crate::model::sprite::Sprite>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TransformHandleKind {
    #[default]
    None,
    Scale,
    Rotate,
}

/// Clipboard data for copy/paste
#[derive(Debug, Clone)]
pub struct ClipboardData {
    pub elements: Vec<crate::model::sprite::StrokeElement>,
}

/// Toast notification to show temporarily
#[derive(Debug, Clone)]
pub struct ToastMessage {
    pub text: String,
    pub created: std::time::Instant,
}

/// Debug overlay toggles
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct DebugOverlays {
    /// Show bone chain lines connecting socketed layers
    pub show_bones: bool,
    /// Show IK target crosshairs
    pub show_ik_targets: bool,
    /// Show constraint gizmos (look-at arrows, spring indicators)
    pub show_constraints: bool,
    /// Show spring target indicators
    pub show_spring_targets: bool,
}


/// Animation playback and editing state
#[derive(Debug, Clone)]
pub struct AnimationState {
    /// Currently selected animation sequence ID (None = rest pose / no animation selected)
    pub selected_sequence_id: Option<String>,
    /// Current playhead time in seconds
    pub current_time: f32,
    /// Whether animation is currently playing
    pub playing: bool,
    /// Whether playback should loop
    pub looping: bool,
    /// Timestamp when playback started (used to compute elapsed time)
    pub playback_start_instant: Option<std::time::Instant>,
    /// The time value when playback was started (so we can compute current_time = start_time + elapsed)
    pub playback_start_time: f32,
    /// Onion skinning enabled
    pub onion_skinning: bool,
    /// Number of ghost frames to show before current frame
    pub onion_before: usize,
    /// Number of ghost frames to show after current frame
    pub onion_after: usize,
    /// Onion skinning time step in seconds (1/fps)
    pub onion_step: f32,
    /// Currently selected track index in the timeline (for curve editor etc.)
    pub selected_track_index: Option<usize>,
    /// Currently selected keyframe ID (for editing in curve editor)
    pub selected_keyframe_id: Option<String>,
    /// Whether the timeline panel is expanded/visible
    pub timeline_visible: bool,
    /// Sequence name being edited (for rename)
    #[allow(dead_code)]
    pub renaming_sequence_id: Option<String>,
    #[allow(dead_code)]
    pub rename_buffer: String,
    /// Selected easing preset for new keyframes
    pub current_easing: crate::model::sprite::EasingPreset,
    /// Whether the curve editor sub-panel is open
    pub curve_editor_open: bool,
}

impl Default for AnimationState {
    fn default() -> Self {
        Self {
            selected_sequence_id: None,
            current_time: 0.0,
            playing: false,
            looping: true,
            playback_start_instant: None,
            playback_start_time: 0.0,
            onion_skinning: false,
            onion_before: 2,
            onion_after: 2,
            onion_step: 1.0 / 12.0,
            selected_track_index: None,
            selected_keyframe_id: None,
            timeline_visible: true,
            renaming_sequence_id: None,
            rename_buffer: String::new(),
            current_easing: crate::model::sprite::EasingPreset::Linear,
            curve_editor_open: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EditorState {
    pub active_tool: ToolKind,
    pub active_color_index: usize,
    pub viewport: ViewportState,
    pub selection: SelectionState,
    pub line_tool_state: LineToolState,
    pub cursor_screen_pos: Option<Vec2>,
    pub cursor_world_pos: Option<Vec2>,
    pub curve_mode: bool,
    pub active_layer_index: usize,
    #[allow(dead_code)]
    pub transform_mode: TransformMode,
    pub is_panning: bool,
    pub pan_start_pos: Option<Vec2>,
    pub pan_start_offset: Option<Vec2>,
    pub stroke_width: f32,
    /// State for select tool drag
    pub select_drag: SelectDragState,
    /// State for marquee selection
    pub marquee: MarqueeState,
    /// Clipboard for copy/paste
    pub clipboard: Option<ClipboardData>,
    /// Toast message (displayed temporarily)
    pub toast: Option<ToastMessage>,
    /// Animation state
    pub animation: AnimationState,
    /// Active skin ID for preview (None = base/default skin)
    pub active_skin_id: Option<String>,
    /// IK target being dragged (target element ID)
    pub dragging_ik_target: Option<String>,
    /// Start position of IK target drag (for undo)
    pub ik_target_drag_start: Option<crate::model::Vec2>,
    /// Debug overlay toggles
    pub debug_overlays: DebugOverlays,
    /// State for dragging curve control point handles
    pub handle_drag: HandleDragState,
    /// State for scale/rotate transform handles
    pub transform_handle: TransformHandleState,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            active_tool: ToolKind::Line,
            active_color_index: 1,
            viewport: ViewportState::default(),
            selection: SelectionState::default(),
            line_tool_state: LineToolState::default(),
            cursor_screen_pos: None,
            cursor_world_pos: None,
            curve_mode: false,
            active_layer_index: 0,
            transform_mode: TransformMode::None,
            is_panning: false,
            pan_start_pos: None,
            pan_start_offset: None,
            stroke_width: 2.0,
            select_drag: SelectDragState::default(),
            marquee: MarqueeState::default(),
            clipboard: None,
            toast: None,
            animation: AnimationState::default(),
            active_skin_id: None,
            dragging_ik_target: None,
            ik_target_drag_start: None,
            debug_overlays: DebugOverlays::default(),
            handle_drag: HandleDragState::default(),
            transform_handle: TransformHandleState::default(),
        }
    }
}
