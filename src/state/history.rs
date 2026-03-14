use crate::model::sprite::Sprite;

#[derive(Debug, Clone)]
pub struct SnapshotCommand {
    #[allow(dead_code)]
    pub description: String,
    pub sprite_index: usize,
    pub before: Sprite,
    pub after: Sprite,
}

pub struct History {
    pub undo_stack: Vec<SnapshotCommand>,
    pub redo_stack: Vec<SnapshotCommand>,
    pub max_depth: usize,
}

impl History {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_depth: 200,
        }
    }

    pub fn push(&mut self, command: SnapshotCommand) {
        self.redo_stack.clear();
        self.undo_stack.push(command);
        if self.undo_stack.len() > self.max_depth {
            self.undo_stack.remove(0);
        }
    }

    pub fn undo(&mut self) -> Option<SnapshotCommand> {
        if let Some(cmd) = self.undo_stack.pop() {
            self.redo_stack.push(cmd.clone());
            Some(cmd)
        } else {
            None
        }
    }

    pub fn redo(&mut self) -> Option<SnapshotCommand> {
        if let Some(cmd) = self.redo_stack.pop() {
            self.undo_stack.push(cmd.clone());
            Some(cmd)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    #[allow(dead_code)]
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}
