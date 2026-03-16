use crate::model::sprite::{Sprite, StrokeElement};
use crate::model::vec2::Vec2;
use crate::state::editor::EditorState;
use crate::state::history::History;

/// Clipboard JSON wrapper for cross-sprite copy/paste.
#[derive(serde::Serialize, serde::Deserialize)]
struct ClipboardData {
    messy_grapefruit_clipboard: bool,
    elements: Vec<StrokeElement>,
}

/// Copy selected elements to both internal and system clipboard.
pub fn copy_selected(
    editor: &EditorState,
    sprite: &Sprite,
    internal_clipboard: &mut Option<Vec<StrokeElement>>,
) {
    if editor.selection.is_empty() {
        return;
    }
    let mut elements = Vec::new();
    for layer in &sprite.layers {
        for element in &layer.elements {
            if editor.selection.is_selected(&element.id) {
                elements.push(element.clone());
            }
        }
    }
    if elements.is_empty() {
        return;
    }

    // Always store in internal clipboard
    *internal_clipboard = Some(elements.clone());

    // Also try system clipboard
    let data = ClipboardData {
        messy_grapefruit_clipboard: true,
        elements,
    };
    if let Ok(json) = serde_json::to_string(&data)
        && let Ok(mut clipboard) = arboard::Clipboard::new()
    {
        let _ = clipboard.set_text(json);
    }
}

/// Paste elements from system clipboard (or internal fallback) into the sprite.
pub fn paste(
    editor: &mut EditorState,
    sprite: &mut Sprite,
    history: &mut History,
    internal_clipboard: &Option<Vec<StrokeElement>>,
) {
    let elements = if let Ok(mut clipboard) = arboard::Clipboard::new()
        && let Ok(json) = clipboard.get_text()
        && let Ok(data) = serde_json::from_str::<ClipboardData>(&json)
        && data.messy_grapefruit_clipboard
        && !data.elements.is_empty()
    {
        data.elements
    } else if let Some(elements) = internal_clipboard {
        elements.clone()
    } else {
        return;
    };

    let before = sprite.clone();
    let layer_idx = editor.layer.active_idx.min(sprite.layers.len().saturating_sub(1));
    let mut new_ids = Vec::new();

    for mut element in elements {
        element.id = uuid::Uuid::new_v4().to_string();
        for v in &mut element.vertices {
            v.id = uuid::Uuid::new_v4().to_string();
        }
        element.position += Vec2::new(10.0, 10.0);
        new_ids.push(element.id.clone());
        sprite.layers[layer_idx].elements.push(element);
    }

    history.push("Paste elements".into(), before, sprite.clone());
    editor.clear_vertex_selection();
    editor.selection.select_all(new_ids);
}

/// Cut selected elements: copy then delete.
pub fn cut(
    editor: &mut EditorState,
    sprite: &mut Sprite,
    history: &mut History,
    internal_clipboard: &mut Option<Vec<StrokeElement>>,
) {
    copy_selected(editor, sprite, internal_clipboard);
    editor.clear_vertex_selection();

    if !editor.selection.is_empty() {
        let before = sprite.clone();
        let selected = editor.selection.selected_ids.clone();
        for layer in sprite.layers.iter_mut() {
            layer.elements.retain(|e| !selected.iter().any(|id| id == &e.id));
        }
        history.push("Cut elements".into(), before, sprite.clone());
        editor.selection.clear();
    }
}
