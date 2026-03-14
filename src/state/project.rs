use crate::model::sprite::Sprite;
use crate::model::project::Project;
use crate::state::editor::EditorState;
use crate::engine::physics::PhysicsState;

#[derive(Debug, Clone)]
pub struct OpenSprite {
    pub sprite: Sprite,
    pub file_path: Option<String>,
    pub editor_state: EditorState,
    /// Physics simulation state (spring positions/velocities).
    /// Not serialized; resets on animation restart.
    #[allow(dead_code)]
    pub physics_state: PhysicsState,
}

/// Active tab: either the project overview or a sprite editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    /// Project overview (always tab index 0 in the UI)
    Overview,
    /// Sprite editor at the given index in `open_sprites`
    Sprite(usize),
}

pub struct ProjectState {
    pub project: Project,
    pub project_path: Option<String>,
    pub open_sprites: Vec<OpenSprite>,
    /// Which tab is active.
    pub active_tab: ActiveTab,
    /// Legacy field kept for compatibility with history. Maps to sprite index.
    pub active_sprite_index: usize,
    pub last_change_time: Option<std::time::Instant>,
    pub last_save_time: Option<std::time::Instant>,
    pub autosave_delay: std::time::Duration,
    /// Loaded sprite data for project overview (mirrors project.sprites order)
    pub overview_sprites: Vec<Sprite>,
}

impl ProjectState {
    pub fn new() -> Self {
        let project = Project::default();
        let sprite = Sprite::new("Sprite 1", 256, 256);
        let open_sprites = vec![OpenSprite {
            sprite: sprite.clone(),
            file_path: None,
            editor_state: EditorState::default(),
            physics_state: PhysicsState::new(),
        }];

        Self {
            project,
            project_path: None,
            open_sprites,
            active_tab: ActiveTab::Sprite(0),
            active_sprite_index: 0,
            last_change_time: None,
            last_save_time: None,
            autosave_delay: std::time::Duration::from_secs(3),
            overview_sprites: vec![sprite],
        }
    }

    pub fn active_sprite(&self) -> Option<&OpenSprite> {
        match self.active_tab {
            ActiveTab::Sprite(idx) => self.open_sprites.get(idx),
            ActiveTab::Overview => None,
        }
    }

    pub fn active_sprite_mut(&mut self) -> Option<&mut OpenSprite> {
        match self.active_tab {
            ActiveTab::Sprite(idx) => self.open_sprites.get_mut(idx),
            ActiveTab::Overview => None,
        }
    }

    /// Get the sprite index for undo/redo purposes.
    pub fn current_sprite_index(&self) -> usize {
        match self.active_tab {
            ActiveTab::Sprite(idx) => idx,
            ActiveTab::Overview => 0,
        }
    }

    pub fn mark_changed(&mut self) {
        self.last_change_time = Some(std::time::Instant::now());
    }

    /// Sync a particular open sprite back to the overview_sprites list.
    pub fn sync_overview_sprite(&mut self, open_idx: usize) {
        if let Some(open) = self.open_sprites.get(open_idx) {
            // Find the overview sprite with matching ID
            if let Some(ov_sprite) = self.overview_sprites.iter_mut().find(|s| s.id == open.sprite.id) {
                *ov_sprite = open.sprite.clone();
            }
        }
    }
}
