use crate::model::project::{HatchPattern, PaletteColor};
use crate::model::sprite::{GradientFill, StrokeElement};

/// A single merge entry: replace one element with a merged version.
pub struct MergeEntry {
    pub merged_element: StrokeElement,
    pub replace_element_id: String,
}

/// Actions produced by UI code, dispatched by App.
/// No direct sprite mutation from UI — all mutations go through dispatch.
pub enum AppAction {
    CommitStroke(StrokeElement),
    MergeStroke {
        merged_element: StrokeElement,
        replace_element_id: String,
    },
    /// Merge multiple strokes atomically (primary merge + symmetry mirror merges).
    MergeSymmetricStrokes(Vec<MergeEntry>),
    /// Commit multiple strokes atomically (symmetry drawing).
    CommitSymmetricStrokes(Vec<StrokeElement>),
    /// Set fill color on a closed element.
    SetFillColor {
        element_id: String,
        fill_color_index: u8,
    },
    /// Set the sprite background color.
    SetBackgroundColor {
        background_color_index: u8,
    },
    /// Erase a vertex (may split element).
    EraseVertex {
        element_id: String,
        vertex_id: String,
    },
    /// Erase a segment (may split element).
    EraseSegment {
        element_id: String,
        segment_index: usize,
    },
    /// Add a new color to the project palette (project-level, no sprite undo).
    AddPaletteColor(PaletteColor),
    /// Delete a color from the palette and remap all sprite indices.
    DeletePaletteColor(u8),
    /// Edit an existing palette color (project-level, no sprite undo).
    EditPaletteColor {
        index: u8,
        color: PaletteColor,
    },
    /// Replace the entire palette (e.g., Lospec import). Project-level, no sprite undo.
    ImportPalette(Vec<PaletteColor>),
    /// Add a reference image to the sprite.
    AddReferenceImage(crate::model::sprite::ReferenceImage),
    /// Remove a reference image by ID.
    RemoveReferenceImage(String),

    // ── Phase 6: Gradient & Hatch Fills ─────────────────────────────

    /// Set a gradient fill on a closed element (sprite-level, undo tracked).
    SetGradientFill {
        element_id: String,
        gradient_fill: GradientFill,
    },
    /// Remove gradient fill from an element (revert to flat fill).
    #[allow(dead_code)]
    ClearGradientFill {
        element_id: String,
    },
    /// Set a hatch fill pattern on a closed element.
    SetHatchFill {
        element_id: String,
        hatch_fill_id: String,
    },
    /// Remove hatch fill from an element.
    ClearHatchFill {
        element_id: String,
    },
    /// Add a new hatch pattern to the project library (project-level, no sprite undo).
    AddHatchPattern(HatchPattern),
    /// Update an existing hatch pattern (project-level, no sprite undo).
    #[allow(dead_code)]
    UpdateHatchPattern(HatchPattern),
    /// Delete a hatch pattern. Elements referencing it will have their hatch_fill_id cleared.
    DeleteHatchPattern(String),
    /// Import hatch patterns from a .hatchpatterns file (project-level).
    ImportHatchPatterns(Vec<HatchPattern>),

    // ── Phase 7: Animation ───────────────────────────────────────────────────

    /// Create a new animation sequence.
    CreateSequence { name: String },
    /// Delete an animation sequence by ID.
    DeleteSequence { sequence_id: String },
    /// Rename an animation sequence.
    RenameSequence { sequence_id: String, name: String },
    /// Select (or deselect) an animation sequence. Resets playhead to 0 and stops playback.
    SelectSequence { sequence_id: Option<String> },
    /// Insert a pose keyframe at the current playhead time.
    /// Captures all visible elements (if selected_ids is None) or only the given elements.
    InsertPose { sequence_id: String, selected_ids: Option<Vec<String>> },
    /// Delete a keyframe from a sequence.
    DeleteKeyframe { sequence_id: String, keyframe_id: String },
    /// Set the playhead time (navigation — no undo).
    SetPlayheadTime { time_secs: f32 },
    /// Set the duration of a sequence.
    SetSequenceDuration { sequence_id: String, duration_secs: f32 },
    /// Set whether a sequence loops.
    SetSequenceLooping { sequence_id: String, looping: bool },
    /// Set the easing curve on a keyframe.
    SetEasingCurve {
        sequence_id: String,
        keyframe_id: String,
        easing: crate::model::animation::EasingCurve,
    },
}
