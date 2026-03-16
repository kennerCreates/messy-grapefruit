use crate::model::sprite::Sprite;

#[derive(Debug, Clone)]
struct UndoEntry {
    description: String,
    sprite_before: Sprite,
    sprite_after: Sprite,
}

#[derive(Debug)]
pub struct History {
    undo_stack: Vec<UndoEntry>,
    redo_stack: Vec<UndoEntry>,
    max_depth: usize,
    pending_drag: Option<(String, Sprite)>,
}

impl History {
    pub fn new(max_depth: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_depth,
            pending_drag: None,
        }
    }

    pub fn push(&mut self, description: String, before: Sprite, after: Sprite) {
        self.redo_stack.clear();
        self.undo_stack.push(UndoEntry {
            description,
            sprite_before: before,
            sprite_after: after,
        });
        if self.undo_stack.len() > self.max_depth {
            self.undo_stack.remove(0);
        }
    }

    pub fn undo(&mut self, current_sprite: &mut Sprite) -> bool {
        if let Some(entry) = self.undo_stack.pop() {
            *current_sprite = entry.sprite_before.clone();
            self.redo_stack.push(entry);
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self, current_sprite: &mut Sprite) -> bool {
        if let Some(entry) = self.redo_stack.pop() {
            *current_sprite = entry.sprite_after.clone();
            self.undo_stack.push(entry);
            true
        } else {
            false
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Coalescing push: if the top undo entry has the same description,
    /// update its `after` state instead of creating a new entry.
    /// This merges consecutive small edits (like slider drags) into one undo step.
    pub fn push_coalesced(&mut self, description: String, before: Sprite, after: Sprite) {
        if let Some(top) = self.undo_stack.last_mut()
            && top.description == description
        {
            top.sprite_after = after;
            return;
        }
        self.push(description, before, after);
    }

    pub fn is_dragging(&self) -> bool {
        self.pending_drag.is_some()
    }

    pub fn begin_drag(&mut self, description: String, snapshot: Sprite) {
        self.pending_drag = Some((description, snapshot));
    }

    pub fn end_drag(&mut self, final_state: Sprite) {
        if let Some((description, before)) = self.pending_drag.take() {
            self.push(description, before, final_state);
        }
    }

    #[allow(dead_code)]
    pub fn cancel_drag(&mut self) {
        self.pending_drag = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::sprite::Sprite;

    #[test]
    fn test_undo_redo() {
        let mut history = History::new(100);
        let s0 = Sprite::new("v0", 64, 64);
        let s1 = Sprite::new("v1", 64, 64);
        let s2 = Sprite::new("v2", 64, 64);

        history.push("edit 1".into(), s0.clone(), s1.clone());
        history.push("edit 2".into(), s1.clone(), s2.clone());

        let mut current = s2.clone();
        assert!(history.undo(&mut current));
        assert_eq!(current.name, "v1");

        assert!(history.undo(&mut current));
        assert_eq!(current.name, "v0");

        assert!(!history.undo(&mut current)); // nothing left

        assert!(history.redo(&mut current));
        assert_eq!(current.name, "v1");
    }

    #[test]
    fn test_redo_clears_on_new_push() {
        let mut history = History::new(100);
        let s0 = Sprite::new("v0", 64, 64);
        let s1 = Sprite::new("v1", 64, 64);
        let s2 = Sprite::new("v2", 64, 64);

        history.push("edit 1".into(), s0.clone(), s1.clone());
        let mut current = s1.clone();
        history.undo(&mut current);
        assert!(history.can_redo());

        history.push("edit 2".into(), s0.clone(), s2.clone());
        assert!(!history.can_redo());
    }

    #[test]
    fn test_max_depth() {
        let mut history = History::new(3);
        for i in 0..5 {
            let before = Sprite::new(format!("v{i}"), 64, 64);
            let after = Sprite::new(format!("v{}", i + 1), 64, 64);
            history.push(format!("edit {i}"), before, after);
        }
        assert_eq!(history.undo_stack.len(), 3);
    }
}
