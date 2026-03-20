use crate::model::sprite::{Layer, Sprite, StrokeElement};
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
/// Each pasted element gets its own layer.
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
    let layer_idx = editor.layer.resolve_active_idx(sprite);
    let group_id = sprite.layers.get(layer_idx).and_then(|l| l.group_id.clone());
    let mut new_ids = Vec::new();
    let mut last_layer_id = None;

    for (i, mut element) in elements.into_iter().enumerate() {
        element.id = uuid::Uuid::new_v4().to_string();
        for v in &mut element.vertices {
            v.id = uuid::Uuid::new_v4().to_string();
        }
        element.position += Vec2::new(10.0, 10.0);
        new_ids.push(element.id.clone());

        let mut new_layer = Layer::new_with_element(element);
        new_layer.group_id = group_id.clone();
        last_layer_id = Some(new_layer.id.clone());
        let insert_idx = (layer_idx + 1 + i).min(sprite.layers.len());
        sprite.layers.insert(insert_idx, new_layer);
    }

    if let Some(id) = last_layer_id {
        editor.layer.active_layer_id = Some(id);
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
        sprite.cleanup_empty_layers();
        editor.layer.validate(sprite);
        history.push("Cut elements".into(), before, sprite.clone());
        editor.selection.clear();
    }
}
