use std::path::PathBuf;
use std::time::Instant;
use crate::model::{Project, Sprite};
use super::editor::EditorState;

pub struct ProjectState {
    pub project: Project,
    pub project_path: Option<PathBuf>,
    pub open_sprites: Vec<OpenSprite>,
    pub active_tab: usize,  // 0 = project overview, 1+ = sprite tabs
    pub autosave_dirty: bool,
    pub autosave_timer: Option<Instant>,
}

impl Default for ProjectState {
    fn default() -> Self {
        Self {
            project: Project::new(),
            project_path: None,
            open_sprites: Vec::new(),
            active_tab: 0,
            autosave_dirty: false,
            autosave_timer: None,
        }
    }
}

pub struct OpenSprite {
    pub sprite: Sprite,
    pub file_path: Option<PathBuf>,
    pub editor_state: EditorState,
}
