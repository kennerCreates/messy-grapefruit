/// Canvas rendering — public API.
///
/// Internally split into two submodules:
/// - `canvas_render_strokes` — stroke/fill/hatch/gradient rendering
/// - `canvas_render_overlays` — handles, selection, boundary, snap, symmetry, ref images
///
/// All public items are re-exported here so callers keep using `canvas_render::xxx`.

pub use super::canvas_render_overlays::{
    VERTEX_HIT_RADIUS,
    cursor_for_handle,
    draw_dashed_line,
    hit_test_handles,
    render_canvas_boundary,
    render_canvas_state_border,
    render_cp_handles,
    render_reference_images,
    render_symmetry_axis,
    render_symmetry_ghost,
    render_transform_handles,
    render_vertex_dots,
    render_vertex_snap_indicator,
};

pub use super::canvas_render_strokes::{
    render_background,
    render_elements,
    render_hover_highlight,
    render_line_tool_preview,
    render_onion_ghost,
    render_selection_highlights,
};
