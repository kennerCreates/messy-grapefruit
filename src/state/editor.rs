use crate::model::sprite::{GradientStop, PathVertex, SpreadMethod, Sprite};
use crate::model::vec2::Vec2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    Select,
    Line,
    Fill,
    Eyedropper,
    Eraser,
}

/// Active fill color mode (flat or gradient).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FillMode {
    #[default]
    Flat,
    LinearGradient,
    RadialGradient,
}

/// Eraser tool hover target: vertex or segment.
#[derive(Debug, Clone)]
pub enum EraserHover {
    Vertex { element_id: String, vertex_id: String, layer_id: String },
    Segment { element_id: String, segment_index: usize, layer_id: String },
}

/// Symmetry axis mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymmetryAxis {
    Vertical,
    Horizontal,
    Both,
}

/// Symmetry drawing state.
#[derive(Debug, Clone)]
pub struct SymmetryState {
    pub active: bool,
    pub axis: SymmetryAxis,
    /// Axis position in world space (x for vertical, y for horizontal, both for Both).
    pub axis_position: Vec2,
    pub dragging_axis: bool,
}

impl Default for SymmetryState {
    fn default() -> Self {
        Self {
            active: false,
            axis: SymmetryAxis::Vertical,
            axis_position: Vec2::new(128.0, 128.0),
            dragging_axis: false,
        }
    }
}

/// What kind of ref image drag is in progress.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RefImageDragKind {
    Move,
    Resize,
}

/// Drag state for repositioning or resizing a reference image.
#[derive(Debug, Clone)]
pub struct RefImageDragState {
    pub image_id: String,
    pub kind: RefImageDragKind,
    pub start_world: Vec2,
    pub initial_position: Vec2,
    pub initial_scale: f32,
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
    /// Request a zoom-to-fit on the next frame (set by toolbar / file open).
    pub zoom_to_fit_requested: bool,
}

impl Default for ViewportState {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            zoom: 1.0,
            flipped: false,
            zoom_to_fit_requested: true,
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
    pub fill_mode: FillMode,
    /// Gradient color stops (minimum 2).
    pub gradient_stops: Vec<GradientStop>,
    /// Midpoint values between each pair of adjacent stops (0.0-1.0, default 0.5).
    pub gradient_midpoints: Vec<f32>,
    /// Gradient angle in radians (for linear gradients).
    pub gradient_angle: f32,
    /// How the gradient extends beyond its range.
    pub gradient_spread: SpreadMethod,
    /// Radial gradient center (normalized 0..1).
    pub radial_center: Vec2,
    /// Radial gradient radius (normalized 0..1).
    pub radial_radius: f32,
    /// Radial focal point offset (normalized 0..1 within AABB).
    pub radial_focal_offset: Vec2,
    /// Index of the currently selected stop in the gradient bar UI.
    pub selected_stop_index: Option<usize>,
    /// Whether clicking also applies the selected hatch pattern.
    pub hatch_apply_enabled: bool,
}

impl Default for BrushState {
    fn default() -> Self {
        Self {
            stroke_width: 2.0,
            color_index: 1, // black
            fill_color_index: 1, // default to first real color
            fill_mode: FillMode::Flat,
            gradient_stops: vec![
                GradientStop { position: 0.0, color_index: 1 },
                GradientStop { position: 1.0, color_index: 15 },
            ],
            gradient_midpoints: vec![0.5],
            gradient_angle: std::f32::consts::FRAC_PI_2, // vertical
            gradient_spread: SpreadMethod::Pad,
            radial_center: Vec2::new(0.5, 0.5),
            radial_radius: 0.5,
            radial_focal_offset: Vec2::new(0.5, 0.5),
            selected_stop_index: Some(0),
            hatch_apply_enabled: false,
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

/// Transient UI panel / popup state (sidebar toggles, color pickers, popups).
/// Grouped separately so animation can add its own UI toggles in `TimelineState`.
#[derive(Debug, Clone)]
pub struct UIState {
    pub sidebar_expanded: bool,
    /// Lospec palette import: slug text input.
    pub lospec_slug: String,
    /// Lospec import error message, if any.
    pub lospec_error: Option<String>,
    /// Whether the Lospec import popup is open.
    pub lospec_popup_open: bool,
    /// Whether the theme color settings panel is expanded.
    pub theme_settings_open: bool,
    /// Which theme role swatch has its palette picker open (0..5), if any.
    pub theme_role_picker: Option<usize>,
    /// Whether the hatch pattern editor panel is visible.
    pub hatch_editor_open: bool,
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            sidebar_expanded: false,
            lospec_slug: String::new(),
            lospec_error: None,
            lospec_popup_open: false,
            theme_settings_open: false,
            theme_role_picker: None,
            hatch_editor_open: false,
        }
    }
}

/// Onion skin display mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnionSkinMode {
    /// Ghost at adjacent keyframes.
    Keyframe,
    /// Ghost at fixed time offsets.
    Frame,
    /// Both keyframe and frame ghosts.
    Both,
}

/// State for the inline easing curve editor popup.
#[derive(Debug, Clone)]
pub struct EasingPopupState {
    /// Keyframe whose easing is being edited (the "right" keyframe in the transition).
    pub keyframe_id: String,
    /// Sequence containing the keyframe.
    pub sequence_id: String,
    /// Screen position to anchor the popup.
    pub screen_pos: egui::Pos2,
}

/// Clipboard for pose copy/paste operations.
#[derive(Debug, Clone)]
pub struct PoseClipboard {
    pub element_poses: Vec<crate::model::animation::ElementPose>,
}

/// Timeline/animation editor state.
#[derive(Debug, Clone)]
pub struct TimelineState {
    /// ID of the animation sequence currently open in the timeline.
    pub selected_sequence_id: Option<String>,
    /// Current playhead position in seconds.
    pub playhead_time: f32,
    /// Auto-key mode: create/update keyframes automatically on edit (Phase 8).
    pub auto_key: bool,
    /// Show ghost of adjacent keyframes while editing (Phase 8).
    pub onion_skin_enabled: bool,
    /// ID of the currently selected keyframe diamond.
    pub selected_keyframe_id: Option<String>,
    /// Whether the timeline panel is visible.
    pub is_timeline_visible: bool,
    /// Inline rename: sequence ID being renamed, if any.
    pub renaming_sequence_id: Option<String>,

    // ── Phase 8: Onion skin config ───────────────────────────────────
    /// Onion skin display mode.
    pub onion_skin_mode: OnionSkinMode,
    /// Number of previous keyframe/frame ghosts to show.
    pub onion_skin_prev_count: u8,
    /// Number of next keyframe/frame ghosts to show.
    pub onion_skin_next_count: u8,
    /// RGB color for previous-frame ghosts (default red).
    pub onion_skin_prev_color: [u8; 3],
    /// RGB color for next-frame ghosts (default green).
    pub onion_skin_next_color: [u8; 3],
    /// Base opacity for ghost overlays (0.0–1.0).
    pub onion_skin_opacity: f32,

    // ── Phase 8: Easing popup ────────────────────────────────────────
    /// When set, the easing curve editor popup is open for this keyframe.
    pub easing_popup: Option<EasingPopupState>,

    // ── Phase 8: Pose clipboard ──────────────────────────────────────
    /// Pose clipboard for copy/paste operations.
    pub pose_clipboard: Option<PoseClipboard>,

    // ── Phase 8: Event marker editing ────────────────────────────────
    /// Event marker currently being dragged on the timeline.
    pub dragging_event_marker_id: Option<String>,
    /// Event marker currently being renamed inline.
    pub renaming_event_marker_id: Option<String>,

    // ── Phase 8: Keyframe dragging ───────────────────────────────────
    /// Keyframe currently being dragged on the timeline.
    pub dragging_keyframe_id: Option<String>,
    /// Preview time for the keyframe being dragged (visual only, committed on release).
    pub dragging_keyframe_preview_time: Option<f32>,

    // ── Phase 8: Keyframe context menu ───────────────────────────────
    /// Keyframe whose context menu is open.
    pub context_menu_keyframe_id: Option<String>,
    /// Screen position of the context menu.
    pub context_menu_screen_pos: Option<egui::Pos2>,

    // ── Phase 8: Onion skin settings ─────────────────────────────────
    /// Whether the onion skin settings popup is open.
    pub onion_skin_settings_open: bool,
}

impl Default for TimelineState {
    fn default() -> Self {
        Self {
            selected_sequence_id: None,
            playhead_time: 0.0,
            auto_key: false,
            onion_skin_enabled: false,
            selected_keyframe_id: None,
            is_timeline_visible: false,
            renaming_sequence_id: None,
            onion_skin_mode: OnionSkinMode::Keyframe,
            onion_skin_prev_count: 1,
            onion_skin_next_count: 1,
            onion_skin_prev_color: [200, 60, 60],
            onion_skin_next_color: [60, 200, 60],
            onion_skin_opacity: 0.3,
            easing_popup: None,
            pose_clipboard: None,
            dragging_event_marker_id: None,
            renaming_event_marker_id: None,
            dragging_keyframe_id: None,
            dragging_keyframe_preview_time: None,
            context_menu_keyframe_id: None,
            context_menu_screen_pos: None,
            onion_skin_settings_open: false,
        }
    }
}

/// Animation playback state.
#[derive(Debug, Clone)]
pub struct PlaybackState {
    pub playing: bool,
    /// Playback speed multiplier (1.0 = normal).
    pub speed: f32,
    /// Whether to loop when the end is reached.
    pub loop_mode: bool,
    /// Time of the last rendered frame (f64 from ctx.input time). None when stopped.
    pub last_frame_time: Option<f64>,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            playing: false,
            speed: 1.0,
            loop_mode: true,
            last_frame_time: None,
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
    /// Last 8 used color indices (session-only, most recent first).
    pub recent_colors: Vec<u8>,
    /// When set, eyedropper was activated temporarily (Alt+click) and should
    /// return to this tool after sampling.
    pub eyedropper_return_tool: Option<ToolKind>,
    /// Vertex snap toggle (magnetic snap to existing vertices).
    pub vertex_snap_enabled: bool,
    /// World position of the vertex snap target (for rendering indicator).
    pub snap_vertex_target: Option<Vec2>,
    /// Eraser tool hover target.
    pub eraser_hover: Option<EraserHover>,
    /// Symmetry drawing state.
    pub symmetry: SymmetryState,
    /// Pending vertex-join target during endpoint vertex drag (world pos for indicator).
    pub vertex_join_target: Option<Vec2>,
    /// Selected reference image ID (for drag/properties).
    pub selected_ref_image_id: Option<String>,
    /// Active reference image drag state.
    pub dragging_ref_image: Option<RefImageDragState>,
    /// Selected hatch pattern ID in the pattern library.
    pub selected_hatch_pattern_id: Option<String>,
    /// Transient UI panel / popup state.
    pub ui: UIState,
    /// Timeline editor state (Phase 7+).
    pub timeline: TimelineState,
    /// Playback state (Phase 7+).
    pub playback: PlaybackState,
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
            recent_colors: Vec::new(),
            eyedropper_return_tool: None,
            vertex_snap_enabled: true,
            snap_vertex_target: None,
            eraser_hover: None,
            symmetry: SymmetryState::default(),
            vertex_join_target: None,
            selected_ref_image_id: None,
            dragging_ref_image: None,
            selected_hatch_pattern_id: None,
            ui: UIState::default(),
            timeline: TimelineState::default(),
            playback: PlaybackState::default(),
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
        let vp = ViewportState { offset: Vec2::new(10.0, 20.0), zoom: 2.0, flipped: false, zoom_to_fit_requested: false };
        let center = egui::Pos2::new(400.0, 300.0);
        let world = Vec2::new(5.0, 10.0);
        let screen = vp.world_to_screen(world, center);
        let back = vp.screen_to_world(screen, center);
        assert!((back.x - world.x).abs() < 1e-4);
        assert!((back.y - world.y).abs() < 1e-4);
    }

    #[test]
    fn test_world_screen_round_trip_flipped() {
        let vp = ViewportState { offset: Vec2::new(10.0, 20.0), zoom: 2.0, flipped: true, zoom_to_fit_requested: false };
        let center = egui::Pos2::new(400.0, 300.0);
        let world = Vec2::new(5.0, 10.0);
        let screen = vp.world_to_screen(world, center);
        let back = vp.screen_to_world(screen, center);
        assert!((back.x - world.x).abs() < 1e-4);
        assert!((back.y - world.y).abs() < 1e-4);
    }

    #[test]
    fn test_zoom_at_preserves_cursor_position() {
        let mut vp = ViewportState { offset: Vec2::ZERO, zoom: 1.0, flipped: false, zoom_to_fit_requested: false };
        let center = egui::Pos2::new(400.0, 300.0);
        let cursor = egui::Pos2::new(500.0, 350.0);
        let world_before = vp.screen_to_world(cursor, center);
        vp.zoom_at(cursor, 2.0, center);
        let world_after = vp.screen_to_world(cursor, center);
        assert!((world_before.x - world_after.x).abs() < 1e-3);
        assert!((world_before.y - world_after.y).abs() < 1e-3);
    }
}
