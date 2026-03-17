use crate::model::project::{HatchPattern, PaletteColor};
use crate::model::sprite::{FlowCurve, GradientFill, StrokeElement};

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
    /// Set or update the flow curve on an element.
    #[allow(dead_code)]
    SetFlowCurve {
        element_id: String,
        flow_curve: FlowCurve,
    },
    /// Remove flow curve from an element.
    #[allow(dead_code)]
    ClearFlowCurve {
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
    /// Add a hatch mask polygon to an element (suppresses hatch lines in that region).
    #[allow(dead_code)]
    AddHatchMask {
        element_id: String,
        mask_polygon: Vec<crate::model::vec2::Vec2>,
    },
    /// Clear all hatch masks from an element.
    #[allow(dead_code)]
    ClearHatchMasks {
        element_id: String,
    },
}
