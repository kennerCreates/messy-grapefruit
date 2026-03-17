use crate::model::project::PaletteColor;
use crate::model::sprite::StrokeElement;

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
}
