use crate::model::Sprite;

/// A snapshot-based undo command that stores the full sprite state before and after a change.
#[derive(Clone)]
pub struct SnapshotCommand {
    pub description: String,
    pub sprite_index: usize,  // Index into open_sprites
    pub before: Sprite,
    pub after: Sprite,
}

pub struct History {
    pub undo_stack: Vec<SnapshotCommand>,
    pub redo_stack: Vec<SnapshotCommand>,
    pub max_depth: usize,
}

impl Default for History {
    fn default() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_depth: 200,
        }
    }
}

impl History {
    pub fn push(&mut self, command: SnapshotCommand) {
        self.redo_stack.clear();
        self.undo_stack.push(command);
        if self.undo_stack.len() > self.max_depth {
            self.undo_stack.remove(0);
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Returns the sprite index and the "before" snapshot to restore
    pub fn undo(&mut self) -> Option<(usize, Sprite)> {
        if let Some(cmd) = self.undo_stack.pop() {
            let sprite_index = cmd.sprite_index;
            let restore = cmd.before.clone();
            self.redo_stack.push(cmd);
            Some((sprite_index, restore))
        } else {
            None
        }
    }

    /// Returns the sprite index and the "after" snapshot to restore
    pub fn redo(&mut self) -> Option<(usize, Sprite)> {
        if let Some(cmd) = self.redo_stack.pop() {
            let sprite_index = cmd.sprite_index;
            let restore = cmd.after.clone();
            self.undo_stack.push(cmd);
            Some((sprite_index, restore))
        } else {
            None
        }
    }
}
