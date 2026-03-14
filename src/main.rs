mod engine;
mod export;
mod io;
mod math;
mod model;
mod state;
mod theme;
mod ui;

use model::project::PaletteColor;
use model::sprite::{AnimationSequence, Layer, Sprite};
use state::editor::{ClipboardData, EditorState, ToastMessage, ToolKind};
use state::history::{History, SnapshotCommand};
use state::project::{ActiveTab, OpenSprite, ProjectState};
use ui::sidebar::{SidebarState, SidebarTab};
use ui::timeline;

struct App {
    project_state: ProjectState,
    history: History,
    sidebar_tab: SidebarTab,
    sidebar_state: SidebarState,
    current_grid_size: f32,
    theme_applied: bool,
    export_dialog: ui::export_dialog::ExportDialogState,
    watcher_state: export::watcher::WatcherState,
    overview_state: ui::project_overview::OverviewState,
    new_sprite_dialog: ui::new_sprite_dialog::NewSpriteDialogState,
}

impl App {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            project_state: ProjectState::new(),
            history: History::new(),
            sidebar_tab: SidebarTab::Layers,
            sidebar_state: SidebarState::default(),
            current_grid_size: 8.0,
            theme_applied: false,
            export_dialog: ui::export_dialog::ExportDialogState::default(),
            watcher_state: export::watcher::WatcherState::new(),
            overview_state: ui::project_overview::OverviewState::new(),
            new_sprite_dialog: ui::new_sprite_dialog::NewSpriteDialogState::default(),
        }
    }

    /// Save project and all open sprites.
    fn save_project(&mut self) {
        if self.project_state.project_path.is_none() {
            // First save: prompt for directory
            if let Some(dir) = rfd::FileDialog::new()
                .set_title("Choose project directory")
                .pick_folder()
            {
                let proj_file = dir.join(format!(
                    "{}.spriteproj",
                    sanitize_filename(&self.project_state.project.name)
                ));
                self.project_state.project_path = Some(proj_file.to_string_lossy().to_string());
            } else {
                return; // User cancelled
            }
        }

        let proj_path_str = self.project_state.project_path.clone().unwrap();
        let proj_path = std::path::Path::new(&proj_path_str);
        let proj_dir = proj_path.parent().unwrap_or(std::path::Path::new("."));

        // Sync all open sprites back to overview_sprites before saving
        for idx in 0..self.project_state.open_sprites.len() {
            self.project_state.sync_overview_sprite(idx);
        }

        // Save each sprite to its own .sprite file
        // Update project sprite refs with file paths
        // Preserve existing positions before clearing
        let old_refs: Vec<_> = self.project_state.project.sprites.drain(..).collect();
        for (i, sprite) in self.project_state.overview_sprites.iter().enumerate() {
            let filename = format!("{}.sprite", sanitize_filename(&sprite.name));
            let sprite_path = proj_dir.join(&filename);

            if let Err(e) = io::save_sprite(sprite, &sprite_path) {
                eprintln!("Failed to save sprite {}: {}", sprite.name, e);
                if let Some(open) = self.project_state.active_sprite_mut() {
                    open.editor_state.toast = Some(ToastMessage {
                        text: format!("Save failed: {}", e),
                        created: std::time::Instant::now(),
                    });
                }
                return;
            }

            // Build ProjectSpriteRef, preserving position from old refs
            let old_ref = old_refs.iter().find(|r| r.id == sprite.id);
            let sprite_ref = model::project::ProjectSpriteRef {
                id: sprite.id.clone(),
                file_path: filename,
                position: old_ref
                    .map(|r| r.position)
                    .unwrap_or_else(|| model::Vec2::new(i as f32 * 180.0, 0.0)),
                rotation: old_ref.map(|r| r.rotation).unwrap_or(0.0),
                z_order: old_ref.map(|r| r.z_order).unwrap_or(i as i32),
                selected_animation_id: old_ref.and_then(|r| r.selected_animation_id.clone()),
                selected_skin_id: old_ref.and_then(|r| r.selected_skin_id.clone()),
            };
            self.project_state.project.sprites.push(sprite_ref);

            // Update open sprite file_path too
            if let Some(open) = self.project_state.open_sprites.iter_mut()
                .find(|os| os.sprite.id == sprite.id)
            {
                open.file_path = Some(sprite_path.to_string_lossy().to_string());
            }
        }

        // Save the project file
        match io::save_project(&self.project_state.project, proj_path) {
            Ok(()) => {
                self.project_state.last_save_time = Some(std::time::Instant::now());
                if let Some(open) = self.project_state.active_sprite_mut() {
                    open.editor_state.toast = Some(ToastMessage {
                        text: "Project saved.".to_string(),
                        created: std::time::Instant::now(),
                    });
                }
            }
            Err(e) => {
                eprintln!("Failed to save project: {}", e);
                if let Some(open) = self.project_state.active_sprite_mut() {
                    open.editor_state.toast = Some(ToastMessage {
                        text: format!("Save failed: {}", e),
                        created: std::time::Instant::now(),
                    });
                }
            }
        }
    }

    /// Load a project from a file dialog.
    fn load_project(&mut self) {
        let file = rfd::FileDialog::new()
            .set_title("Open Project")
            .add_filter("Sprite Project", &["spriteproj"])
            .pick_file();

        let Some(proj_path) = file else { return };

        let proj_dir = proj_path.parent().unwrap_or(std::path::Path::new("."));

        match io::load_project(&proj_path) {
            Ok(project) => {
                // Load all referenced sprites
                let mut overview_sprites = Vec::new();
                for sprite_ref in &project.sprites {
                    let sprite_path = proj_dir.join(&sprite_ref.file_path);
                    match io::load_sprite(&sprite_path) {
                        Ok(sprite) => {
                            overview_sprites.push(sprite);
                        }
                        Err(e) => {
                            eprintln!("Failed to load sprite {}: {}", sprite_ref.file_path, e);
                            // Create a placeholder
                            let mut placeholder = Sprite::new(&sprite_ref.id, 256, 256);
                            placeholder.name = format!("(missing: {})", sprite_ref.file_path);
                            overview_sprites.push(placeholder);
                        }
                    }
                }

                self.project_state.project = project;
                self.project_state.project_path = Some(proj_path.to_string_lossy().to_string());
                self.project_state.overview_sprites = overview_sprites;
                self.project_state.open_sprites.clear();
                self.project_state.active_tab = ActiveTab::Overview;
                self.project_state.active_sprite_index = 0;
                self.project_state.last_change_time = None;
                self.project_state.last_save_time = Some(std::time::Instant::now());
                self.history = History::new();
            }
            Err(e) => {
                eprintln!("Failed to load project: {}", e);
            }
        }
    }

    /// Create a new empty project.
    fn new_project(&mut self) {
        self.project_state = ProjectState::new();
        self.history = History::new();
        self.overview_state = ui::project_overview::OverviewState::new();
    }

    /// Open a sprite from overview_sprites in a new editor tab (or switch to existing tab).
    fn open_sprite_tab(&mut self, overview_idx: usize) {
        if overview_idx >= self.project_state.overview_sprites.len() {
            return;
        }

        let sprite = &self.project_state.overview_sprites[overview_idx];
        let sprite_id = sprite.id.clone();

        // Check if already open
        if let Some(open_idx) = self.project_state.open_sprites.iter().position(|os| os.sprite.id == sprite_id) {
            self.project_state.active_tab = ActiveTab::Sprite(open_idx);
            self.project_state.active_sprite_index = open_idx;
            return;
        }

        // Open new tab
        let open_sprite = OpenSprite {
            sprite: sprite.clone(),
            file_path: None,
            editor_state: EditorState::default(),
            physics_state: engine::physics::PhysicsState::new(),
        };
        self.project_state.open_sprites.push(open_sprite);
        let new_idx = self.project_state.open_sprites.len() - 1;
        self.project_state.active_tab = ActiveTab::Sprite(new_idx);
        self.project_state.active_sprite_index = new_idx;
    }

    /// Add a new sprite to the project.
    fn add_new_sprite(&mut self, name: String, width: u32, height: u32) {
        let sprite = Sprite::new(&name, width, height);
        let sprite_id = sprite.id.clone();

        // Add to overview sprites
        self.project_state.overview_sprites.push(sprite.clone());

        // Also create a ProjectSpriteRef for positioning
        let idx = self.project_state.overview_sprites.len() - 1;
        let sprite_ref = model::project::ProjectSpriteRef {
            id: sprite_id.clone(),
            file_path: format!("{}.sprite", sanitize_filename(&name)),
            position: model::Vec2::new(idx as f32 * 180.0, 0.0),
            rotation: 0.0,
            z_order: idx as i32,
            selected_animation_id: None,
            selected_skin_id: None,
        };
        self.project_state.project.sprites.push(sprite_ref);

        // Open in a new tab
        let open_sprite = OpenSprite {
            sprite,
            file_path: None,
            editor_state: EditorState::default(),
            physics_state: engine::physics::PhysicsState::new(),
        };
        self.project_state.open_sprites.push(open_sprite);
        let new_idx = self.project_state.open_sprites.len() - 1;
        self.project_state.active_tab = ActiveTab::Sprite(new_idx);
        self.project_state.active_sprite_index = new_idx;
        self.project_state.mark_changed();
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let current_theme = self.project_state.project.editor_preferences.theme;

        // Apply theme on first frame or when it changes
        if !self.theme_applied {
            theme::apply_theme(ctx, current_theme);
            self.theme_applied = true;
        }
        // Re-apply theme every frame (cheap operation, handles theme toggle)
        theme::apply_theme(ctx, current_theme);

        // Update animation playback (must happen before rendering)
        update_animation_playback(&mut self.project_state);

        // Handle keyboard shortcuts
        let shortcut_flags = handle_keyboard_shortcuts(ctx, &mut self.project_state, &mut self.history);
        if shortcut_flags.save_project {
            self.save_project();
        }
        if shortcut_flags.new_sprite {
            self.new_sprite_dialog.open = true;
        }

        // --- Autosave ---
        if let Some(change_time) = self.project_state.last_change_time {
            let should_save = if let Some(save_time) = self.project_state.last_save_time {
                change_time > save_time
                    && change_time.elapsed() >= self.project_state.autosave_delay
            } else {
                // No save yet -- only autosave if a project path exists
                self.project_state.project_path.is_some()
                    && change_time.elapsed() >= self.project_state.autosave_delay
            };
            if should_save {
                self.save_project();
            }
        }

        // --- File menu (Ctrl+S handled in shortcuts, but also add a menu bar) ---

        // --- Toolbar ---
        let toolbar_action = {
            let active_tool;
            let curve_mode;
            let skin_info;
            if let Some(open) = self.project_state.active_sprite() {
                active_tool = open.editor_state.active_tool;
                curve_mode = open.editor_state.curve_mode;
                skin_info = Some(ui::toolbar::ToolbarSkinInfo {
                    active_skin_id: open.editor_state.active_skin_id.clone(),
                    skins: open.sprite.skins.clone(),
                });
            } else {
                active_tool = ToolKind::Line;
                curve_mode = true;
                skin_info = None;
            }
            ui::toolbar::draw_toolbar(ctx, active_tool, curve_mode, skin_info.as_ref())
        };

        if let Some(action) = toolbar_action {
            match action {
                ui::toolbar::ToolAction::OpenExportDialog => {
                    self.export_dialog.open = true;
                    self.export_dialog.settings =
                        self.project_state.project.export_settings.clone();
                }
                _ => {
                    if let Some(open) = self.project_state.active_sprite_mut() {
                        match action {
                            ui::toolbar::ToolAction::SelectTool(tool) => {
                                open.editor_state.active_tool = tool;
                            }
                            ui::toolbar::ToolAction::ToggleCurveMode => {
                                open.editor_state.curve_mode =
                                    !open.editor_state.curve_mode;
                            }
                            ui::toolbar::ToolAction::SetActiveSkin(skin_id) => {
                                open.editor_state.active_skin_id = skin_id;
                            }
                            ui::toolbar::ToolAction::OpenExportDialog => unreachable!(),
                        }
                    }
                }
            }
        }

        // --- Status Bar ---
        if let Some(open) = self.project_state.active_sprite() {
            ui::status_bar::draw_status_bar(ctx, &open.editor_state, self.current_grid_size);
        }

        // --- Tab Bar ---
        {
            let mut new_active_tab = None;
            let mut close_tab = None;
            let mut do_new_project = false;
            let mut do_open_project = false;
            let mut do_save_project = false;
            let mut do_new_sprite = false;

            egui::TopBottomPanel::top("tab_bar").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // File menu
                    ui.menu_button("File", |ui| {
                        if ui.button("New Project").clicked() {
                            ui.close_menu();
                            do_new_project = true;
                        }
                        if ui.button("Open Project...").clicked() {
                            ui.close_menu();
                            do_open_project = true;
                        }
                        if ui.button("Save Project (Ctrl+S)").clicked() {
                            ui.close_menu();
                            do_save_project = true;
                        }
                        ui.separator();
                        if ui.button("New Sprite (Ctrl+N)").clicked() {
                            ui.close_menu();
                            do_new_sprite = true;
                        }
                    });

                    ui.separator();

                    // Project Overview tab
                    let is_overview = self.project_state.active_tab == ActiveTab::Overview;
                    if ui.selectable_label(is_overview, "Project").clicked() {
                        // Sync current sprite back before switching
                        if let ActiveTab::Sprite(idx) = self.project_state.active_tab {
                            self.project_state.sync_overview_sprite(idx);
                        }
                        new_active_tab = Some(ActiveTab::Overview);
                    }

                    // Collect tab names first to avoid borrow issues
                    let tab_names: Vec<(usize, String)> = self.project_state.open_sprites.iter()
                        .enumerate()
                        .map(|(idx, os)| (idx, os.sprite.name.clone()))
                        .collect();

                    // Sprite tabs
                    let current_active = self.project_state.active_tab;
                    for (idx, tab_label) in &tab_names {
                        let is_active = current_active == ActiveTab::Sprite(*idx);

                        let clicked = ui.selectable_label(is_active, tab_label).clicked();
                        let close_clicked = ui.small_button("x").clicked();

                        if clicked {
                            new_active_tab = Some(ActiveTab::Sprite(*idx));
                        }
                        if close_clicked {
                            close_tab = Some(*idx);
                        }
                    }
                });
            });

            // Handle tab close
            if let Some(close_idx) = close_tab {
                // Sync before closing
                self.project_state.sync_overview_sprite(close_idx);
                self.project_state.open_sprites.remove(close_idx);

                // Fix active tab index
                match self.project_state.active_tab {
                    ActiveTab::Sprite(idx) if idx == close_idx => {
                        if self.project_state.open_sprites.is_empty() {
                            self.project_state.active_tab = ActiveTab::Overview;
                        } else if close_idx >= self.project_state.open_sprites.len() {
                            self.project_state.active_tab =
                                ActiveTab::Sprite(self.project_state.open_sprites.len() - 1);
                        }
                        // else: idx is still valid after removal
                    }
                    ActiveTab::Sprite(idx) if idx > close_idx => {
                        self.project_state.active_tab = ActiveTab::Sprite(idx - 1);
                    }
                    _ => {}
                }
                self.project_state.active_sprite_index =
                    self.project_state.current_sprite_index();
            }

            // Apply tab switch
            if let Some(tab) = new_active_tab {
                // Sync current sprite back to overview before switching
                if let ActiveTab::Sprite(old_idx) = self.project_state.active_tab
                    && ActiveTab::Sprite(old_idx) != tab {
                        self.project_state.sync_overview_sprite(old_idx);
                    }
                self.project_state.active_tab = tab;
                self.project_state.active_sprite_index =
                    self.project_state.current_sprite_index();
            }

            // Process file menu actions
            if do_new_project {
                self.new_project();
            }
            if do_open_project {
                self.load_project();
            }
            if do_save_project {
                self.save_project();
            }
            if do_new_sprite {
                self.new_sprite_dialog.open = true;
            }
        }

        // Determine which view to show based on active tab
        let showing_overview = self.project_state.active_tab == ActiveTab::Overview;

        if showing_overview {
            // --- Project Overview ---
            let overview_actions = {
                let sprite_refs = &self.project_state.project.sprites;
                let sprites = &self.project_state.overview_sprites;
                let palette = &self.project_state.project.palette;
                ui::project_overview::draw_project_overview(
                    ctx,
                    &mut self.overview_state,
                    sprite_refs,
                    sprites,
                    palette,
                    current_theme,
                )
            };

            for action in overview_actions {
                match action {
                    ui::project_overview::OverviewAction::OpenSprite(idx) => {
                        self.open_sprite_tab(idx);
                    }
                    ui::project_overview::OverviewAction::DeleteSprite(idx) => {
                        if idx < self.project_state.overview_sprites.len() {
                            let sprite_id = self.project_state.overview_sprites[idx].id.clone();

                            // Close the tab if open
                            if let Some(open_idx) = self.project_state.open_sprites.iter().position(|os| os.sprite.id == sprite_id) {
                                self.project_state.open_sprites.remove(open_idx);
                                // Fix active tab
                                if let ActiveTab::Sprite(active_idx) = self.project_state.active_tab {
                                    if active_idx == open_idx {
                                        self.project_state.active_tab = ActiveTab::Overview;
                                    } else if active_idx > open_idx {
                                        self.project_state.active_tab = ActiveTab::Sprite(active_idx - 1);
                                    }
                                }
                            }

                            self.project_state.overview_sprites.remove(idx);
                            if idx < self.project_state.project.sprites.len() {
                                self.project_state.project.sprites.remove(idx);
                            }
                            self.project_state.mark_changed();
                        }
                    }
                    ui::project_overview::OverviewAction::RenameSprite(idx, new_name) => {
                        if idx < self.project_state.overview_sprites.len() {
                            self.project_state.overview_sprites[idx].name = new_name;
                            self.project_state.mark_changed();
                        }
                    }
                    ui::project_overview::OverviewAction::MoveSprite(idx, new_pos) => {
                        if idx < self.project_state.project.sprites.len() {
                            self.project_state.project.sprites[idx].position = new_pos;
                        }
                    }
                    ui::project_overview::OverviewAction::SetSpriteAnimation(idx, anim_id) => {
                        if idx < self.project_state.project.sprites.len() {
                            self.project_state.project.sprites[idx].selected_animation_id = anim_id;
                        }
                    }
                    ui::project_overview::OverviewAction::SetSpriteSkin(idx, skin_id) => {
                        if idx < self.project_state.project.sprites.len() {
                            self.project_state.project.sprites[idx].selected_skin_id = skin_id;
                        }
                    }
                    ui::project_overview::OverviewAction::NewSprite => {
                        self.new_sprite_dialog.open = true;
                    }
                }
            }
        } else {
            // --- Sprite Editor View ---

            // --- Timeline ---
            let timeline_actions = if let Some(open) = self.project_state.active_sprite() {
                let sprite = &open.sprite;
                let anim_state = &open.editor_state.animation;
                ui::timeline::draw_timeline(ctx, sprite, anim_state)
            } else {
                Vec::new()
            };

            // Process timeline actions
            let sprite_index = self.project_state.current_sprite_index();
            for action in timeline_actions {
                process_timeline_action(action, &mut self.project_state, &mut self.history, sprite_index);
            }

            // --- Sidebar ---
            let grid_mode = self.project_state.project.editor_preferences.grid_mode;
            let sidebar_actions = if let Some(open) = self.project_state.active_sprite() {
                let editor_state = &open.editor_state;
                let sprite = &open.sprite;
                let palette = &self.project_state.project.palette;
                ui::sidebar::draw_sidebar(
                    ctx,
                    editor_state,
                    sprite,
                    palette,
                    &mut self.sidebar_tab,
                    &mut self.sidebar_state,
                    grid_mode,
                )
            } else {
                Vec::new()
            };

            // Process sidebar actions
            let sprite_index = self.project_state.current_sprite_index();
            for action in sidebar_actions {
                process_sidebar_action(action, &mut self.project_state, &mut self.history, sprite_index, &mut self.sidebar_state);
            }

            // --- Canvas ---
            let grid_size_base = self.project_state.project.editor_preferences.grid_size;
            let palette = self.project_state.project.palette.clone();
            let grid_mode = self.project_state.project.editor_preferences.grid_mode;
            let sprite_index = self.project_state.current_sprite_index();

            if let Some(open) = self.project_state.open_sprites.get_mut(sprite_index) {
                let _canvas_actions = ui::canvas::draw_canvas(
                    ctx,
                    &mut open.sprite,
                    &mut open.editor_state,
                    &palette,
                    current_theme,
                    grid_size_base,
                    &mut self.history,
                    sprite_index,
                    grid_mode,
                    &mut open.physics_state,
                );
            }

            // Request repaint during animation playback
            if let Some(open) = self.project_state.active_sprite()
                && open.editor_state.animation.playing {
                    ctx.request_repaint();
                }
        }

        // Clear expired toasts
        if let Some(open) = self.project_state.active_sprite_mut()
            && let Some(ref toast) = open.editor_state.toast
                && toast.created.elapsed().as_secs() >= 4 {
                    open.editor_state.toast = None;
                }

        // --- New Sprite Dialog ---
        let new_sprite_actions = ui::new_sprite_dialog::draw_new_sprite_dialog(
            ctx,
            &mut self.new_sprite_dialog,
        );
        for action in new_sprite_actions {
            match action {
                ui::new_sprite_dialog::NewSpriteDialogAction::Create { name, width, height } => {
                    self.new_sprite_dialog.open = false;
                    self.new_sprite_dialog.reset();
                    self.add_new_sprite(name, width, height);
                }
                ui::new_sprite_dialog::NewSpriteDialogAction::Close => {
                    self.new_sprite_dialog.open = false;
                    self.new_sprite_dialog.reset();
                }
            }
        }

        // --- Export Dialog ---
        let export_actions =
            ui::export_dialog::draw_export_dialog(ctx, &mut self.export_dialog);

        for action in export_actions {
            match action {
                ui::export_dialog::ExportDialogAction::ConfirmExport => {
                    // Save settings back to project
                    self.project_state.project.export_settings =
                        self.export_dialog.settings.clone();

                    // Perform export
                    let status = perform_export(
                        &self.project_state,
                        &self.export_dialog.settings,
                    );
                    self.export_dialog.last_export_status = Some(status);
                }
                ui::export_dialog::ExportDialogAction::RefreshPreview => {
                    let summary = generate_export_preview(
                        &self.project_state,
                        &self.export_dialog.settings,
                    );
                    self.export_dialog.summary = summary;
                }
                ui::export_dialog::ExportDialogAction::ToggleAutoExport(enabled) => {
                    self.export_dialog.auto_export_enabled = enabled;
                }
                ui::export_dialog::ExportDialogAction::ToggleWatcher(enabled) => {
                    if enabled {
                        if let Some(ref project_path) = self.project_state.project_path {
                            let dir = std::path::Path::new(project_path)
                                .parent()
                                .unwrap_or(std::path::Path::new("."));
                            match self.watcher_state.start_watching(dir) {
                                Ok(()) => {
                                    self.export_dialog.watcher_active = true;
                                }
                                Err(e) => {
                                    self.export_dialog.last_export_status =
                                        Some(format!("Watcher error: {}", e));
                                    self.export_dialog.watcher_active = false;
                                }
                            }
                        } else {
                            self.export_dialog.last_export_status =
                                Some("Save the project first to enable file watching.".to_string());
                            self.export_dialog.watcher_active = false;
                        }
                    } else {
                        self.watcher_state.stop_watching();
                        self.export_dialog.watcher_active = false;
                    }
                }
                ui::export_dialog::ExportDialogAction::Close => {
                    self.export_dialog.open = false;
                }
            }
        }

        // --- File Watcher: process pending changes ---
        if self.watcher_state.is_watching() {
            let changed_paths = self.watcher_state.drain_pending();
            for path in changed_paths {
                // Find which sprite corresponds to this path
                let sprite_idx = self
                    .project_state
                    .open_sprites
                    .iter()
                    .position(|os| {
                        os.file_path
                            .as_ref()
                            .map(|fp| std::path::Path::new(fp) == path)
                            .unwrap_or(false)
                    });

                if let Some(idx) = sprite_idx {
                    // Re-export this sprite using last-used settings
                    let status = perform_export_for_sprite(
                        &self.project_state,
                        idx,
                        &self.project_state.project.export_settings,
                    );
                    eprintln!("Auto-export (watcher): {}", status);
                }
            }
        }

        // --- Auto-export on save ---
        if self.export_dialog.auto_export_enabled
            && let Some(save_time) = self.project_state.last_save_time {
                // Only auto-export once per save (check if save is newer than last auto-export)
                let should_export = self.export_dialog.last_auto_export_time
                    .is_none_or(|last_export| save_time > last_export);
                if should_export {
                    let status = perform_export(
                        &self.project_state,
                        &self.project_state.project.export_settings,
                    );
                    eprintln!("Auto-export: {}", status);
                    self.export_dialog.last_auto_export_time = Some(std::time::Instant::now());
                }
            }
    }
}

/// Process sidebar actions. Extracted from inline to reduce nesting.
fn process_sidebar_action(
    action: ui::sidebar::SidebarAction,
    project_state: &mut ProjectState,
    history: &mut History,
    sprite_index: usize,
    sidebar_state: &mut SidebarState,
) {
    match action {
        ui::sidebar::SidebarAction::SetActiveLayer(idx) => {
            if let Some(open) = project_state.active_sprite_mut() {
                open.editor_state.active_layer_index = idx;
            }
        }
        ui::sidebar::SidebarAction::AddLayer => {
            let (before, after) = {
                if let Some(open) = project_state.active_sprite_mut() {
                    let before = open.sprite.clone();
                    let layer_num = open.sprite.layers.len() + 1;
                    let layer = Layer::new(&format!("Layer {}", layer_num));
                    open.sprite.layers.push(layer);
                    open.editor_state.active_layer_index = open.sprite.layers.len() - 1;
                    (Some(before), Some(open.sprite.clone()))
                } else {
                    (None, None)
                }
            };
            if let (Some(before), Some(after)) = (before, after) {
                history.push(SnapshotCommand {
                    description: "Add layer".to_string(),
                    sprite_index,
                    before,
                    after,
                });
                project_state.mark_changed();
            }
        }
        ui::sidebar::SidebarAction::RemoveLayer(idx) => {
            let (before, after) = {
                if let Some(open) = project_state.active_sprite_mut() {
                    if open.sprite.layers.len() > 1 && idx < open.sprite.layers.len() {
                        let before = open.sprite.clone();

                        // Before removing, detach any layers socketed to elements
                        // on this layer (they snap to world-space position)
                        let element_ids: Vec<String> = open.sprite.layers[idx]
                            .elements
                            .iter()
                            .map(|e| e.id.clone())
                            .collect();
                        for elem_id in &element_ids {
                            let children = engine::socket::find_child_layers_for_element(&open.sprite, elem_id);
                            for child_id in &children {
                                engine::socket::detach_layer_to_world_space(&mut open.sprite, child_id);
                            }
                        }

                        open.sprite.layers.remove(idx);
                        if open.editor_state.active_layer_index >= open.sprite.layers.len() {
                            open.editor_state.active_layer_index =
                                open.sprite.layers.len() - 1;
                        }
                        (Some(before), Some(open.sprite.clone()))
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                }
            };
            if let (Some(before), Some(after)) = (before, after) {
                history.push(SnapshotCommand {
                    description: "Remove layer".to_string(),
                    sprite_index,
                    before,
                    after,
                });
                project_state.mark_changed();
            }
        }
        ui::sidebar::SidebarAction::ToggleLayerVisibility(idx) => {
            if let Some(open) = project_state.active_sprite_mut()
                && let Some(layer) = open.sprite.layers.get_mut(idx) {
                    layer.visible = !layer.visible;
                }
        }
        ui::sidebar::SidebarAction::ToggleLayerLock(idx) => {
            if let Some(open) = project_state.active_sprite_mut()
                && let Some(layer) = open.sprite.layers.get_mut(idx) {
                    layer.locked = !layer.locked;
                }
        }
        ui::sidebar::SidebarAction::SetActiveColor(idx) => {
            if let Some(open) = project_state.active_sprite_mut() {
                open.editor_state.active_color_index = idx;
            }
        }
        ui::sidebar::SidebarAction::SetStrokeWidth(w) => {
            if let Some(open) = project_state.active_sprite_mut() {
                open.editor_state.stroke_width = w;
            }
        }
        ui::sidebar::SidebarAction::DuplicateLayer(idx) => {
            let (before, after) = {
                if let Some(open) = project_state.active_sprite_mut() {
                    if idx < open.sprite.layers.len() {
                        let before = open.sprite.clone();
                        let source_layer = &open.sprite.layers[idx];
                        let mut new_layer = Layer::new(&format!("{} copy", source_layer.name));
                        for elem in &source_layer.elements {
                            let mut new_elem = elem.clone();
                            new_elem.id = uuid::Uuid::new_v4().to_string();
                            for v in &mut new_elem.vertices {
                                v.id = uuid::Uuid::new_v4().to_string();
                            }
                            new_layer.elements.push(new_elem);
                        }
                        open.sprite.layers.insert(idx + 1, new_layer);
                        open.editor_state.active_layer_index = idx + 1;
                        (Some(before), Some(open.sprite.clone()))
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                }
            };
            if let (Some(before), Some(after)) = (before, after) {
                history.push(SnapshotCommand {
                    description: "Duplicate layer".to_string(),
                    sprite_index,
                    before,
                    after,
                });
                project_state.mark_changed();
            }
        }
        ui::sidebar::SidebarAction::MirrorLayerH(idx) => {
            let (before, after) = {
                if let Some(open) = project_state.active_sprite_mut() {
                    if idx < open.sprite.layers.len() {
                        let before = open.sprite.clone();
                        mirror_layer(&mut open.sprite.layers[idx], true);
                        (Some(before), Some(open.sprite.clone()))
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                }
            };
            if let (Some(before), Some(after)) = (before, after) {
                history.push(SnapshotCommand {
                    description: "Mirror layer horizontally".to_string(),
                    sprite_index,
                    before,
                    after,
                });
                project_state.mark_changed();
            }
        }
        ui::sidebar::SidebarAction::MirrorLayerV(idx) => {
            let (before, after) = {
                if let Some(open) = project_state.active_sprite_mut() {
                    if idx < open.sprite.layers.len() {
                        let before = open.sprite.clone();
                        mirror_layer(&mut open.sprite.layers[idx], false);
                        (Some(before), Some(open.sprite.clone()))
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                }
            };
            if let (Some(before), Some(after)) = (before, after) {
                history.push(SnapshotCommand {
                    description: "Mirror layer vertically".to_string(),
                    sprite_index,
                    before,
                    after,
                });
                project_state.mark_changed();
            }
        }
        ui::sidebar::SidebarAction::CombineLayerDown(idx) => {
            let (before, after) = {
                if let Some(open) = project_state.active_sprite_mut() {
                    if idx > 0 && idx < open.sprite.layers.len() {
                        let before = open.sprite.clone();

                        let top_socket = open.sprite.layers[idx].socket.clone();
                        let bottom_had_socket = open.sprite.layers[idx - 1].socket.is_some();
                        let top_layer_id = open.sprite.layers[idx].id.clone();
                        let bottom_layer_id = open.sprite.layers[idx - 1].id.clone();

                        let source_elements = open.sprite.layers[idx].elements.clone();
                        open.sprite.layers[idx - 1].elements.extend(source_elements);

                        open.sprite.layers[idx - 1].socket = top_socket;

                        if bottom_had_socket && open.sprite.layers[idx - 1].socket.is_none() {
                            open.editor_state.toast = Some(state::editor::ToastMessage {
                                text: "Combined layer lost socket (top layer had no socket)".to_string(),
                                created: std::time::Instant::now(),
                            });
                        }

                        open.sprite.layers.remove(idx);

                        let _ = (top_layer_id, bottom_layer_id);

                        open.editor_state.active_layer_index = idx - 1;
                        (Some(before), Some(open.sprite.clone()))
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                }
            };
            if let (Some(before), Some(after)) = (before, after) {
                history.push(SnapshotCommand {
                    description: "Combine layers".to_string(),
                    sprite_index,
                    before,
                    after,
                });
                project_state.mark_changed();
            }
        }
        ui::sidebar::SidebarAction::MoveLayerUp(idx) => {
            let (before, after) = {
                if let Some(open) = project_state.active_sprite_mut() {
                    if idx + 1 < open.sprite.layers.len() {
                        let before = open.sprite.clone();
                        open.sprite.layers.swap(idx, idx + 1);
                        open.editor_state.active_layer_index = idx + 1;
                        (Some(before), Some(open.sprite.clone()))
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                }
            };
            if let (Some(before), Some(after)) = (before, after) {
                history.push(SnapshotCommand {
                    description: "Move layer up".to_string(),
                    sprite_index,
                    before,
                    after,
                });
                project_state.mark_changed();
            }
        }
        ui::sidebar::SidebarAction::MoveLayerDown(idx) => {
            let (before, after) = {
                if let Some(open) = project_state.active_sprite_mut() {
                    if idx > 0 {
                        let before = open.sprite.clone();
                        open.sprite.layers.swap(idx, idx - 1);
                        open.editor_state.active_layer_index = idx - 1;
                        (Some(before), Some(open.sprite.clone()))
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                }
            };
            if let (Some(before), Some(after)) = (before, after) {
                history.push(SnapshotCommand {
                    description: "Move layer down".to_string(),
                    sprite_index,
                    before,
                    after,
                });
                project_state.mark_changed();
            }
        }
        ui::sidebar::SidebarAction::AddPaletteColor(color) => {
            if project_state.project.palette.colors.len() < 256 {
                project_state.project.palette.colors.push(color);
                project_state.mark_changed();
            } else if let Some(open) = project_state.active_sprite_mut() {
                open.editor_state.toast = Some(ToastMessage {
                    text: "Palette full -- 256 color maximum".to_string(),
                    created: std::time::Instant::now(),
                });
            }
        }
        ui::sidebar::SidebarAction::DeletePaletteColor(idx) => {
            if idx > 0 && idx < project_state.project.palette.colors.len() {
                project_state.project.palette.colors.remove(idx);
                let palette_len = project_state.project.palette.colors.len();
                if let Some(open) = project_state.active_sprite_mut()
                    && open.editor_state.active_color_index >= palette_len {
                        open.editor_state.active_color_index = palette_len - 1;
                    }
                project_state.mark_changed();
            }
        }
        ui::sidebar::SidebarAction::UpdatePaletteColor(idx, color) => {
            if idx < project_state.project.palette.colors.len() {
                project_state.project.palette.colors[idx] = color;
                project_state.mark_changed();
            }
        }
        ui::sidebar::SidebarAction::ImportLospecPalette(slug) => {
            match io::fetch_lospec_palette(&slug) {
                Ok(hex_colors) => {
                    let mut new_colors = vec![PaletteColor {
                        hex: "00000000".to_string(),
                        name: Some("Transparent".to_string()),
                    }];
                    for hex in hex_colors.iter().take(255) {
                        new_colors.push(PaletteColor {
                            hex: format!("{}ff", hex),
                            name: None,
                        });
                    }
                    project_state.project.palette.colors = new_colors;
                    project_state.project.palette.name = slug;
                    sidebar_state.lospec_error = None;
                    project_state.mark_changed();
                }
                Err(e) => {
                    sidebar_state.lospec_error = Some(e.to_string());
                }
            }
        }
        ui::sidebar::SidebarAction::ToggleTheme => {
            let new_theme = match project_state.project.editor_preferences.theme {
                model::project::Theme::Dark => model::project::Theme::Light,
                model::project::Theme::Light => model::project::Theme::Dark,
            };
            project_state.project.editor_preferences.theme = new_theme;
            // Theme will be applied on next frame via ctx (we don't have ctx here)
            project_state.mark_changed();
        }
        ui::sidebar::SidebarAction::SetGridMode(mode) => {
            project_state.project.editor_preferences.grid_mode = mode;
            project_state.mark_changed();
        }
        ui::sidebar::SidebarAction::ResizeCanvas(new_w, new_h) => {
            if let Some(open) = project_state.active_sprite_mut()
                && new_w > 0 && new_h > 0 && (open.sprite.canvas_width != new_w || open.sprite.canvas_height != new_h) {
                    let before = open.sprite.clone();
                    open.sprite.canvas_width = new_w;
                    open.sprite.canvas_height = new_h;
                    let after = open.sprite.clone();
                    history.push(SnapshotCommand {
                        description: format!("Resize canvas to {}x{}", new_w, new_h),
                        sprite_index: project_state.active_sprite_index,
                        before,
                        after,
                    });
                    project_state.mark_changed();
                }
        }
        ui::sidebar::SidebarAction::SetSocket {
            layer_index,
            parent_element_id,
            parent_vertex_id,
        } => {
            let (before, after) = {
                if let Some(open) = project_state.active_sprite_mut() {
                    if layer_index < open.sprite.layers.len() {
                        let layer_id = open.sprite.layers[layer_index].id.clone();
                        if engine::socket::would_create_cycle(&open.sprite, &layer_id, &parent_element_id) {
                            open.editor_state.toast = Some(state::editor::ToastMessage {
                                text: "Cannot set socket: would create circular reference".to_string(),
                                created: std::time::Instant::now(),
                            });
                            (None, None)
                        } else {
                            let before = open.sprite.clone();
                            open.sprite.layers[layer_index].socket = Some(model::sprite::LayerSocket {
                                parent_element_id,
                                parent_vertex_id,
                            });
                            (Some(before), Some(open.sprite.clone()))
                        }
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                }
            };
            if let (Some(before), Some(after)) = (before, after) {
                history.push(SnapshotCommand {
                    description: "Set layer socket".to_string(),
                    sprite_index,
                    before,
                    after,
                });
                project_state.mark_changed();
            }
        }
        ui::sidebar::SidebarAction::ClearSocket(layer_index) => {
            let (before, after) = {
                if let Some(open) = project_state.active_sprite_mut() {
                    if layer_index < open.sprite.layers.len()
                        && open.sprite.layers[layer_index].socket.is_some()
                    {
                        let before = open.sprite.clone();
                        let layer_id = open.sprite.layers[layer_index].id.clone();
                        engine::socket::detach_layer_to_world_space(&mut open.sprite, &layer_id);
                        (Some(before), Some(open.sprite.clone()))
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                }
            };
            if let (Some(before), Some(after)) = (before, after) {
                history.push(SnapshotCommand {
                    description: "Clear layer socket".to_string(),
                    sprite_index,
                    before,
                    after,
                });
                project_state.mark_changed();
            }
        }
        ui::sidebar::SidebarAction::CreateSkin => {
            let (before, after, new_skin_id) = {
                if let Some(open) = project_state.active_sprite_mut() {
                    let before = open.sprite.clone();
                    let skin_num = open.sprite.skins.len() + 1;
                    let skin = model::sprite::Skin::new(&format!("Skin {}", skin_num));
                    let skin_id = skin.id.clone();
                    open.sprite.skins.push(skin);
                    (Some(before), Some(open.sprite.clone()), Some(skin_id))
                } else {
                    (None, None, None)
                }
            };
            if let (Some(before), Some(after)) = (before, after) {
                if let Some(open) = project_state.active_sprite_mut() {
                    open.editor_state.active_skin_id = new_skin_id;
                }
                history.push(SnapshotCommand {
                    description: "Create skin".to_string(),
                    sprite_index,
                    before,
                    after,
                });
                project_state.mark_changed();
            }
        }
        ui::sidebar::SidebarAction::DeleteSkin(skin_id) => {
            let (before, after) = {
                if let Some(open) = project_state.active_sprite_mut() {
                    let before = open.sprite.clone();
                    open.sprite.skins.retain(|s| s.id != skin_id);
                    if open.editor_state.active_skin_id.as_ref() == Some(&skin_id) {
                        open.editor_state.active_skin_id = None;
                    }
                    (Some(before), Some(open.sprite.clone()))
                } else {
                    (None, None)
                }
            };
            if let (Some(before), Some(after)) = (before, after) {
                history.push(SnapshotCommand {
                    description: "Delete skin".to_string(),
                    sprite_index,
                    before,
                    after,
                });
                project_state.mark_changed();
            }
        }
        ui::sidebar::SidebarAction::RenameSkin(skin_id, new_name) => {
            let (before, after) = {
                if let Some(open) = project_state.active_sprite_mut() {
                    let before = open.sprite.clone();
                    if let Some(skin) = open.sprite.skins.iter_mut().find(|s| s.id == skin_id) {
                        skin.name = new_name;
                    }
                    (Some(before), Some(open.sprite.clone()))
                } else {
                    (None, None)
                }
            };
            if let (Some(before), Some(after)) = (before, after) {
                history.push(SnapshotCommand {
                    description: "Rename skin".to_string(),
                    sprite_index,
                    before,
                    after,
                });
                project_state.mark_changed();
            }
        }
        ui::sidebar::SidebarAction::DuplicateSkin(skin_id) => {
            let (before, after) = {
                if let Some(open) = project_state.active_sprite_mut() {
                    if let Some(source_skin) = open.sprite.skins.iter().find(|s| s.id == skin_id).cloned() {
                        let before = open.sprite.clone();
                        let mut new_skin = model::sprite::Skin::new(&format!("{} copy", source_skin.name));
                        new_skin.overrides = source_skin.overrides.clone();
                        let new_id = new_skin.id.clone();
                        open.sprite.skins.push(new_skin);
                        open.editor_state.active_skin_id = Some(new_id);
                        (Some(before), Some(open.sprite.clone()))
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                }
            };
            if let (Some(before), Some(after)) = (before, after) {
                history.push(SnapshotCommand {
                    description: "Duplicate skin".to_string(),
                    sprite_index,
                    before,
                    after,
                });
                project_state.mark_changed();
            }
        }
        ui::sidebar::SidebarAction::SetActiveSkin(skin_id) => {
            if let Some(open) = project_state.active_sprite_mut() {
                open.editor_state.active_skin_id = skin_id;
            }
        }
        ui::sidebar::SidebarAction::SetSkinOverride {
            skin_id,
            element_id,
            stroke_color_index,
            fill_color_index,
            stroke_width,
        } => {
            let (before, after) = {
                if let Some(open) = project_state.active_sprite_mut() {
                    let before = open.sprite.clone();
                    if let Some(skin) = open.sprite.skins.iter_mut().find(|s| s.id == skin_id) {
                        if let Some(ovr) = skin.overrides.iter_mut().find(|o| o.element_id == element_id) {
                            ovr.stroke_color_index = stroke_color_index;
                            ovr.fill_color_index = fill_color_index;
                            ovr.stroke_width = stroke_width;
                        } else {
                            skin.overrides.push(model::sprite::SkinOverride {
                                element_id,
                                stroke_color_index,
                                fill_color_index,
                                stroke_width,
                            });
                        }
                    }
                    (Some(before), Some(open.sprite.clone()))
                } else {
                    (None, None)
                }
            };
            if let (Some(before), Some(after)) = (before, after) {
                history.push(SnapshotCommand {
                    description: "Set skin override".to_string(),
                    sprite_index,
                    before,
                    after,
                });
                project_state.mark_changed();
            }
        }
        ui::sidebar::SidebarAction::RemoveSkinOverride {
            skin_id,
            element_id,
        } => {
            let (before, after) = {
                if let Some(open) = project_state.active_sprite_mut() {
                    let before = open.sprite.clone();
                    if let Some(skin) = open.sprite.skins.iter_mut().find(|s| s.id == skin_id) {
                        skin.overrides.retain(|o| o.element_id != element_id);
                    }
                    (Some(before), Some(open.sprite.clone()))
                } else {
                    (None, None)
                }
            };
            if let (Some(before), Some(after)) = (before, after) {
                history.push(SnapshotCommand {
                    description: "Remove skin override".to_string(),
                    sprite_index,
                    before,
                    after,
                });
                project_state.mark_changed();
            }
        }
        ui::sidebar::SidebarAction::CreateIKChain { layer_ids, name } => {
            let (before, after) = {
                if let Some(open) = project_state.active_sprite_mut() {
                    if let Some(ref seq_id) = open.editor_state.animation.selected_sequence_id.clone() {
                        let before = open.sprite.clone();

                        let chain = model::sprite::IKChain::new(&name, layer_ids.clone(), "");

                        let chain_id = chain.id.clone();
                        let target = model::sprite::IKTargetElement::new(
                            model::vec2::Vec2::ZERO,
                            &chain_id,
                        );
                        let target_id = target.id.clone();

                        let initial_pos = if let Some(tip_layer_id) = layer_ids.last() {
                            let st = engine::socket::resolve_socket_transform(&open.sprite, tip_layer_id);
                            st.position
                        } else {
                            model::vec2::Vec2::ZERO
                        };

                        if let Some(tip_layer_id) = layer_ids.last()
                            && let Some(tip_layer) = open.sprite.layers.iter_mut().find(|l| l.id == *tip_layer_id) {
                                let mut target = target;
                                target.position = initial_pos;
                                tip_layer.ik_targets.push(target);
                            }

                        if let Some(seq) = open.sprite.animations.iter_mut().find(|a| a.id == *seq_id) {
                            let mut chain = chain;
                            chain.target_element_id = target_id;
                            seq.ik_chains.push(chain);
                        }

                        (Some(before), Some(open.sprite.clone()))
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                }
            };
            if let (Some(before), Some(after)) = (before, after) {
                history.push(SnapshotCommand {
                    description: "Create IK chain".to_string(),
                    sprite_index,
                    before,
                    after,
                });
                project_state.mark_changed();
            }
        }
        ui::sidebar::SidebarAction::DeleteIKChain(chain_id) => {
            let (before, after) = {
                if let Some(open) = project_state.active_sprite_mut() {
                    if let Some(ref seq_id) = open.editor_state.animation.selected_sequence_id.clone() {
                        let before = open.sprite.clone();

                        if let Some(seq) = open.sprite.animations.iter().find(|a| a.id == *seq_id)
                            && let Some(chain) = seq.ik_chains.iter().find(|c| c.id == chain_id) {
                                let target_id = chain.target_element_id.clone();
                                for layer in &mut open.sprite.layers {
                                    layer.ik_targets.retain(|t| t.id != target_id);
                                }
                            }

                        if let Some(seq) = open.sprite.animations.iter_mut().find(|a| a.id == *seq_id) {
                            seq.ik_chains.retain(|c| c.id != chain_id);
                        }

                        (Some(before), Some(open.sprite.clone()))
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                }
            };
            if let (Some(before), Some(after)) = (before, after) {
                history.push(SnapshotCommand {
                    description: "Delete IK chain".to_string(),
                    sprite_index,
                    before,
                    after,
                });
                project_state.mark_changed();
            }
        }
        ui::sidebar::SidebarAction::SetIKChainSolver { chain_id, solver } => {
            let (before, after) = {
                if let Some(open) = project_state.active_sprite_mut() {
                    if let Some(ref seq_id) = open.editor_state.animation.selected_sequence_id.clone() {
                        let before = open.sprite.clone();
                        if let Some(seq) = open.sprite.animations.iter_mut().find(|a| a.id == *seq_id)
                            && let Some(chain) = seq.ik_chains.iter_mut().find(|c| c.id == chain_id) {
                                chain.solver = solver;
                            }
                        (Some(before), Some(open.sprite.clone()))
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                }
            };
            if let (Some(before), Some(after)) = (before, after) {
                history.push(SnapshotCommand {
                    description: "Set IK solver".to_string(),
                    sprite_index,
                    before,
                    after,
                });
                project_state.mark_changed();
            }
        }
        ui::sidebar::SidebarAction::SetIKChainBendDirection { chain_id, bend_direction } => {
            let (before, after) = {
                if let Some(open) = project_state.active_sprite_mut() {
                    if let Some(ref seq_id) = open.editor_state.animation.selected_sequence_id.clone() {
                        let before = open.sprite.clone();
                        if let Some(seq) = open.sprite.animations.iter_mut().find(|a| a.id == *seq_id)
                            && let Some(chain) = seq.ik_chains.iter_mut().find(|c| c.id == chain_id) {
                                chain.bend_direction = bend_direction;
                            }
                        (Some(before), Some(open.sprite.clone()))
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                }
            };
            if let (Some(before), Some(after)) = (before, after) {
                history.push(SnapshotCommand {
                    description: "Set IK bend direction".to_string(),
                    sprite_index,
                    before,
                    after,
                });
                project_state.mark_changed();
            }
        }
        ui::sidebar::SidebarAction::SetIKChainMix { chain_id, mix } => {
            let (before, after) = {
                if let Some(open) = project_state.active_sprite_mut() {
                    if let Some(ref seq_id) = open.editor_state.animation.selected_sequence_id.clone() {
                        let before = open.sprite.clone();
                        if let Some(seq) = open.sprite.animations.iter_mut().find(|a| a.id == *seq_id)
                            && let Some(chain) = seq.ik_chains.iter_mut().find(|c| c.id == chain_id) {
                                chain.mix = mix;
                            }
                        (Some(before), Some(open.sprite.clone()))
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                }
            };
            if let (Some(before), Some(after)) = (before, after) {
                history.push(SnapshotCommand {
                    description: "Set IK mix".to_string(),
                    sprite_index,
                    before,
                    after,
                });
                project_state.mark_changed();
            }
        }
        ui::sidebar::SidebarAction::SetIKAngleConstraint { chain_id, layer_id, min, max } => {
            let (before, after) = {
                if let Some(open) = project_state.active_sprite_mut() {
                    if let Some(ref seq_id) = open.editor_state.animation.selected_sequence_id.clone() {
                        let before = open.sprite.clone();
                        if let Some(seq) = open.sprite.animations.iter_mut().find(|a| a.id == *seq_id)
                            && let Some(chain) = seq.ik_chains.iter_mut().find(|c| c.id == chain_id) {
                                if let Some(constraint) = chain.angle_constraints.iter_mut().find(|c| c.layer_id == layer_id) {
                                    constraint.min = min;
                                    constraint.max = max;
                                } else {
                                    chain.angle_constraints.push(model::sprite::AngleConstraint {
                                        layer_id,
                                        min,
                                        max,
                                    });
                                }
                            }
                        (Some(before), Some(open.sprite.clone()))
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                }
            };
            if let (Some(before), Some(after)) = (before, after) {
                history.push(SnapshotCommand {
                    description: "Set IK angle constraint".to_string(),
                    sprite_index,
                    before,
                    after,
                });
                project_state.mark_changed();
            }
        }
        ui::sidebar::SidebarAction::RemoveIKAngleConstraint { chain_id, layer_id } => {
            let (before, after) = {
                if let Some(open) = project_state.active_sprite_mut() {
                    if let Some(ref seq_id) = open.editor_state.animation.selected_sequence_id.clone() {
                        let before = open.sprite.clone();
                        if let Some(seq) = open.sprite.animations.iter_mut().find(|a| a.id == *seq_id)
                            && let Some(chain) = seq.ik_chains.iter_mut().find(|c| c.id == chain_id) {
                                chain.angle_constraints.retain(|c| c.layer_id != layer_id);
                            }
                        (Some(before), Some(open.sprite.clone()))
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                }
            };
            if let (Some(before), Some(after)) = (before, after) {
                history.push(SnapshotCommand {
                    description: "Remove IK angle constraint".to_string(),
                    sprite_index,
                    before,
                    after,
                });
                project_state.mark_changed();
            }
        }
        ui::sidebar::SidebarAction::SetLayerConstraints { layer_index, constraints } => {
            let (before, after) = {
                if let Some(open) = project_state.active_sprite_mut() {
                    if layer_index < open.sprite.layers.len() {
                        let before = open.sprite.clone();
                        open.sprite.layers[layer_index].constraints = constraints;
                        (Some(before), Some(open.sprite.clone()))
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                }
            };
            if let (Some(before), Some(after)) = (before, after) {
                history.push(SnapshotCommand {
                    description: "Update layer constraints".to_string(),
                    sprite_index,
                    before,
                    after,
                });
                project_state.mark_changed();
            }
        }
        ui::sidebar::SidebarAction::ToggleDebugBones => {
            if let Some(open) = project_state.active_sprite_mut() {
                open.editor_state.debug_overlays.show_bones =
                    !open.editor_state.debug_overlays.show_bones;
            }
        }
        ui::sidebar::SidebarAction::ToggleDebugIKTargets => {
            if let Some(open) = project_state.active_sprite_mut() {
                open.editor_state.debug_overlays.show_ik_targets =
                    !open.editor_state.debug_overlays.show_ik_targets;
            }
        }
        ui::sidebar::SidebarAction::ToggleDebugConstraints => {
            if let Some(open) = project_state.active_sprite_mut() {
                open.editor_state.debug_overlays.show_constraints =
                    !open.editor_state.debug_overlays.show_constraints;
            }
        }
        ui::sidebar::SidebarAction::ToggleDebugSpringTargets => {
            if let Some(open) = project_state.active_sprite_mut() {
                open.editor_state.debug_overlays.show_spring_targets =
                    !open.editor_state.debug_overlays.show_spring_targets;
            }
        }
    }
}

/// Perform bone export for the active sprite using the given settings.
fn perform_export(
    project_state: &ProjectState,
    settings: &model::project::ExportSettings,
) -> String {
    let idx = project_state.current_sprite_index();
    perform_export_for_sprite(project_state, idx, settings)
}

/// Perform export for a specific sprite by index.
fn perform_export_for_sprite(
    project_state: &ProjectState,
    sprite_index: usize,
    settings: &model::project::ExportSettings,
) -> String {
    let Some(open) = project_state.open_sprites.get(sprite_index) else {
        return "No sprite to export.".to_string();
    };

    // Determine output directory
    let output_dir = if let Some(ref proj_path) = project_state.project_path {
        let base = std::path::Path::new(proj_path).parent().unwrap_or(std::path::Path::new("."));
        base.join(&project_state.project.export_dir)
    } else {
        std::path::PathBuf::from(&project_state.project.export_dir)
    };

    let palette = &project_state.project.palette;

    match settings.mode.as_str() {
        "bone" => {
            match export::bone_export::export_bone_animation(
                &open.sprite,
                palette,
                &output_dir,
                settings.padding,
            ) {
                Ok(result) => {
                    format!(
                        "Export successful!\n\nRON: {}\nAtlas files: {}",
                        result.ron_path.display(),
                        result.atlas_paths.len()
                    )
                }
                Err(e) => format!("Export failed: {}", e),
            }
        }
        "spritesheet" => {
            // Find the first animation sequence for this sprite
            if let Some(seq) = open.sprite.animations.first() {
                match export::spritesheet::export_spritesheet(
                    &open.sprite,
                    seq,
                    palette,
                    None,
                    settings,
                    &output_dir,
                ) {
                    Ok(result) => result.summary,
                    Err(e) => format!("Spritesheet export failed: {}", e),
                }
            } else {
                "No animation sequences to export as spritesheet.".to_string()
            }
        }
        _ => format!("Unknown export mode: {}", settings.mode),
    }
}

/// Generate a preview summary for the export dialog.
fn generate_export_preview(
    project_state: &ProjectState,
    settings: &model::project::ExportSettings,
) -> String {
    let Some(open) = project_state.active_sprite() else {
        return "No sprite to preview.".to_string();
    };

    let palette = &project_state.project.palette;

    match settings.mode.as_str() {
        "bone" => {
            match export::bone_export::preview_bone_export(
                &open.sprite,
                palette,
                settings.padding,
            ) {
                Ok((ron_data, _atlas_bytes)) => {
                    let mut summary = String::new();
                    summary.push_str(&format!("Sprite: {}\n", ron_data.name));
                    summary.push_str(&format!(
                        "Canvas: {}x{}\n",
                        ron_data.canvas_width, ron_data.canvas_height
                    ));
                    summary.push_str(&format!("Parts: {}\n", ron_data.parts.len()));
                    summary.push_str(&format!("Animations: {}\n", ron_data.animations.len()));
                    for anim in &ron_data.animations {
                        summary.push_str(&format!(
                            "  - {} ({:.1}s, {} tracks, {})\n",
                            anim.name,
                            anim.duration,
                            anim.tracks.len(),
                            if anim.looping { "looping" } else { "once" }
                        ));
                    }
                    summary.push_str(&format!("IK Chains: {}\n", ron_data.ik_chains.len()));
                    summary.push_str(&format!(
                        "Layers with dynamics: {}\n",
                        ron_data.layer_dynamics.len()
                    ));
                    summary.push_str(&format!("Skins: {}\n", ron_data.skins.len()));
                    for skin in &ron_data.skins {
                        summary.push_str(&format!(
                            "  - {} -> {}\n",
                            skin.name, skin.atlas_file
                        ));
                    }
                    summary
                }
                Err(e) => format!("Preview generation failed: {}", e),
            }
        }
        "spritesheet" => {
            if let Some(seq) = open.sprite.animations.first() {
                export::spritesheet::preview_spritesheet(&open.sprite, seq, settings)
            } else {
                "No animation sequences to preview.".to_string()
            }
        }
        _ => format!("Unknown export mode: {}", settings.mode),
    }
}

/// Update animation playback timing
fn update_animation_playback(project_state: &mut ProjectState) {
    if let Some(open) = project_state.active_sprite_mut() {
        if !open.editor_state.animation.playing {
            return;
        }

        let Some(ref seq_id) = open.editor_state.animation.selected_sequence_id else {
            open.editor_state.animation.playing = false;
            return;
        };

        let duration = open
            .sprite
            .animations
            .iter()
            .find(|a| a.id == *seq_id)
            .map(|a| a.duration)
            .unwrap_or(1.0);

        if let Some(start_instant) = open.editor_state.animation.playback_start_instant {
            let elapsed = start_instant.elapsed().as_secs_f32();
            let new_time = open.editor_state.animation.playback_start_time + elapsed;

            if open.editor_state.animation.looping {
                open.editor_state.animation.current_time = new_time % duration;
            } else if new_time >= duration {
                open.editor_state.animation.current_time = duration;
                open.editor_state.animation.playing = false;
                open.editor_state.animation.playback_start_instant = None;
            } else {
                open.editor_state.animation.current_time = new_time;
            }
        }
    }
}

/// Process a timeline action
fn process_timeline_action(
    action: timeline::TimelineAction,
    project_state: &mut ProjectState,
    history: &mut History,
    sprite_index: usize,
) {
    match action {
        timeline::TimelineAction::SelectSequence(id) => {
            if let Some(open) = project_state.active_sprite_mut() {
                open.editor_state.animation.selected_sequence_id = id;
                open.editor_state.animation.current_time = 0.0;
                open.editor_state.animation.playing = false;
                open.editor_state.animation.playback_start_instant = None;
                open.editor_state.animation.selected_track_index = None;
                open.editor_state.animation.selected_keyframe_id = None;
            }
        }
        timeline::TimelineAction::CreateSequence => {
            if let Some(open) = project_state.active_sprite_mut() {
                let before = open.sprite.clone();
                let seq_num = open.sprite.animations.len() + 1;
                let seq = AnimationSequence::new(&format!("Anim {}", seq_num));
                let seq_id = seq.id.clone();
                open.sprite.animations.push(seq);
                open.editor_state.animation.selected_sequence_id = Some(seq_id);
                open.editor_state.animation.current_time = 0.0;
                history.push(SnapshotCommand {
                    description: "Create animation".to_string(),
                    sprite_index,
                    before,
                    after: open.sprite.clone(),
                });
                project_state.mark_changed();
            }
        }
        timeline::TimelineAction::DeleteSequence(id) => {
            if let Some(open) = project_state.active_sprite_mut() {
                let before = open.sprite.clone();
                open.sprite.animations.retain(|a| a.id != id);
                if open.editor_state.animation.selected_sequence_id.as_ref() == Some(&id) {
                    open.editor_state.animation.selected_sequence_id = None;
                    open.editor_state.animation.playing = false;
                    open.editor_state.animation.playback_start_instant = None;
                }
                history.push(SnapshotCommand {
                    description: "Delete animation".to_string(),
                    sprite_index,
                    before,
                    after: open.sprite.clone(),
                });
                project_state.mark_changed();
            }
        }
        timeline::TimelineAction::RenameSequence(id, new_name) => {
            if let Some(open) = project_state.active_sprite_mut() {
                let before = open.sprite.clone();
                if let Some(seq) = open.sprite.animations.iter_mut().find(|a| a.id == id) {
                    seq.name = new_name;
                }
                history.push(SnapshotCommand {
                    description: "Rename animation".to_string(),
                    sprite_index,
                    before,
                    after: open.sprite.clone(),
                });
                project_state.mark_changed();
            }
        }
        timeline::TimelineAction::SetTime(time) => {
            if let Some(open) = project_state.active_sprite_mut() {
                open.editor_state.animation.current_time = time;
                if open.editor_state.animation.playing {
                    open.editor_state.animation.playback_start_time = time;
                    open.editor_state.animation.playback_start_instant =
                        Some(std::time::Instant::now());
                }
            }
        }
        timeline::TimelineAction::TogglePlayback => {
            if let Some(open) = project_state.active_sprite_mut() {
                if open.editor_state.animation.playing {
                    open.editor_state.animation.playing = false;
                    open.editor_state.animation.playback_start_instant = None;
                } else {
                    open.editor_state.animation.playing = true;
                    open.editor_state.animation.playback_start_time =
                        open.editor_state.animation.current_time;
                    open.editor_state.animation.playback_start_instant =
                        Some(std::time::Instant::now());
                    if open.editor_state.animation.current_time < 0.01 {
                        open.physics_state.reset();
                    }
                }
            }
        }
        timeline::TimelineAction::JumpToStart => {
            if let Some(open) = project_state.active_sprite_mut() {
                open.editor_state.animation.current_time = 0.0;
                open.physics_state.reset();
                if open.editor_state.animation.playing {
                    open.editor_state.animation.playback_start_time = 0.0;
                    open.editor_state.animation.playback_start_instant =
                        Some(std::time::Instant::now());
                }
            }
        }
        timeline::TimelineAction::SkipBackward => {
            if let Some(open) = project_state.active_sprite_mut() {
                let current_time = open.editor_state.animation.current_time;
                if let Some(ref seq_id) = open.editor_state.animation.selected_sequence_id.clone()
                    && let Some(seq) = open.sprite.animations.iter().find(|a| a.id == *seq_id) {
                        if let Some(prev_t) = engine::animation::prev_keyframe_time(seq, current_time) {
                            open.editor_state.animation.current_time = prev_t;
                        } else {
                            open.editor_state.animation.current_time = 0.0;
                        }
                    }
                if open.editor_state.animation.playing {
                    open.editor_state.animation.playback_start_time =
                        open.editor_state.animation.current_time;
                    open.editor_state.animation.playback_start_instant =
                        Some(std::time::Instant::now());
                }
            }
        }
        timeline::TimelineAction::SkipForward => {
            if let Some(open) = project_state.active_sprite_mut() {
                let current_time = open.editor_state.animation.current_time;
                if let Some(ref seq_id) = open.editor_state.animation.selected_sequence_id.clone()
                    && let Some(seq) = open.sprite.animations.iter().find(|a| a.id == *seq_id)
                        && let Some(next_t) = engine::animation::next_keyframe_time(seq, current_time) {
                            open.editor_state.animation.current_time = next_t;
                        }
                if open.editor_state.animation.playing {
                    open.editor_state.animation.playback_start_time =
                        open.editor_state.animation.current_time;
                    open.editor_state.animation.playback_start_instant =
                        Some(std::time::Instant::now());
                }
            }
        }
        timeline::TimelineAction::ToggleLoop => {
            if let Some(open) = project_state.active_sprite_mut() {
                open.editor_state.animation.looping = !open.editor_state.animation.looping;
            }
        }
        timeline::TimelineAction::ToggleOnionSkinning => {
            if let Some(open) = project_state.active_sprite_mut() {
                open.editor_state.animation.onion_skinning =
                    !open.editor_state.animation.onion_skinning;
            }
        }
        timeline::TimelineAction::SetOnionBefore(count) => {
            if let Some(open) = project_state.active_sprite_mut() {
                open.editor_state.animation.onion_before = count;
            }
        }
        timeline::TimelineAction::SetOnionAfter(count) => {
            if let Some(open) = project_state.active_sprite_mut() {
                open.editor_state.animation.onion_after = count;
            }
        }
        timeline::TimelineAction::AddKeyframe {
            track_index,
            time,
            value,
            easing,
        } => {
            if let Some(open) = project_state.active_sprite_mut() {
                let before = open.sprite.clone();
                if let Some(ref seq_id) = open.editor_state.animation.selected_sequence_id.clone()
                    && let Some(seq) = open.sprite.animations.iter_mut().find(|a| a.id == *seq_id)
                        && track_index < seq.tracks.len() {
                            seq.tracks[track_index].set_keyframe(time, value, easing);
                            seq.auto_extend_duration();
                        }
                history.push(SnapshotCommand {
                    description: "Add keyframe".to_string(),
                    sprite_index,
                    before,
                    after: open.sprite.clone(),
                });
                project_state.mark_changed();
            }
        }
        timeline::TimelineAction::RemoveKeyframe {
            track_index,
            keyframe_id,
        } => {
            if let Some(open) = project_state.active_sprite_mut() {
                let before = open.sprite.clone();
                if let Some(ref seq_id) = open.editor_state.animation.selected_sequence_id.clone()
                    && let Some(seq) = open.sprite.animations.iter_mut().find(|a| a.id == *seq_id)
                        && track_index < seq.tracks.len() {
                            seq.tracks[track_index].remove_keyframe(&keyframe_id);
                        }
                history.push(SnapshotCommand {
                    description: "Remove keyframe".to_string(),
                    sprite_index,
                    before,
                    after: open.sprite.clone(),
                });
                project_state.mark_changed();
            }
        }
        timeline::TimelineAction::SelectTrack(idx) => {
            if let Some(open) = project_state.active_sprite_mut() {
                open.editor_state.animation.selected_track_index = idx;
            }
        }
        timeline::TimelineAction::SelectKeyframe(id) => {
            if let Some(open) = project_state.active_sprite_mut() {
                open.editor_state.animation.selected_keyframe_id = id;
            }
        }
        timeline::TimelineAction::UpdateKeyframeEasing {
            track_index,
            keyframe_id,
            easing,
        } => {
            if let Some(open) = project_state.active_sprite_mut() {
                let before = open.sprite.clone();
                if let Some(ref seq_id) = open.editor_state.animation.selected_sequence_id.clone()
                    && let Some(seq) = open.sprite.animations.iter_mut().find(|a| a.id == *seq_id)
                        && track_index < seq.tracks.len()
                            && let Some(kf) = seq.tracks[track_index]
                                .keyframes
                                .iter_mut()
                                .find(|k| k.id == keyframe_id)
                            {
                                kf.easing = easing;
                            }
                history.push(SnapshotCommand {
                    description: "Update easing".to_string(),
                    sprite_index,
                    before,
                    after: open.sprite.clone(),
                });
                project_state.mark_changed();
            }
        }
        timeline::TimelineAction::SetCurrentEasing(preset) => {
            if let Some(open) = project_state.active_sprite_mut() {
                open.editor_state.animation.current_easing = preset;
            }
        }
        timeline::TimelineAction::ToggleCurveEditor => {
            if let Some(open) = project_state.active_sprite_mut() {
                open.editor_state.animation.curve_editor_open =
                    !open.editor_state.animation.curve_editor_open;
            }
        }
        timeline::TimelineAction::SetDuration(dur) => {
            if let Some(open) = project_state.active_sprite_mut() {
                let before = open.sprite.clone();
                if let Some(ref seq_id) = open.editor_state.animation.selected_sequence_id.clone()
                    && let Some(seq) = open.sprite.animations.iter_mut().find(|a| a.id == *seq_id) {
                        seq.duration = dur.max(0.1);
                    }
                history.push(SnapshotCommand {
                    description: "Set duration".to_string(),
                    sprite_index,
                    before,
                    after: open.sprite.clone(),
                });
                project_state.mark_changed();
            }
        }
        timeline::TimelineAction::AddTrack {
            property,
            element_id,
            layer_id,
        } => {
            if let Some(open) = project_state.active_sprite_mut() {
                // Resolve element and layer IDs from selection if empty
                let (resolved_element_id, resolved_layer_id) = if element_id.is_empty() {
                    match &property {
                        model::sprite::AnimatableProperty::IKTargetX
                        | model::sprite::AnimatableProperty::IKTargetY => {
                            let mut found = None;
                            for layer in &open.sprite.layers {
                                if let Some(target) = layer.ik_targets.first() {
                                    found = Some((target.id.clone(), layer.id.clone()));
                                    break;
                                }
                            }
                            if let Some((target_id, layer_id)) = found {
                                (target_id, layer_id)
                            } else {
                                open.editor_state.toast = Some(ToastMessage {
                                    text: "No IK targets found. Create an IK chain first.".to_string(),
                                    created: std::time::Instant::now(),
                                });
                                return;
                            }
                        }
                        model::sprite::AnimatableProperty::IKMix => {
                            if let Some(ref seq_id) = open.editor_state.animation.selected_sequence_id.clone() {
                                let chain_id = open.sprite.animations.iter()
                                    .find(|a| a.id == *seq_id)
                                    .and_then(|seq| seq.ik_chains.first())
                                    .map(|c| c.id.clone());
                                if let Some(chain_id) = chain_id {
                                    (chain_id, String::new())
                                } else {
                                    open.editor_state.toast = Some(ToastMessage {
                                        text: "No IK chains found. Create an IK chain first.".to_string(),
                                        created: std::time::Instant::now(),
                                    });
                                    return;
                                }
                            } else {
                                return;
                            }
                        }
                        _ => {
                            if let Some(selected_id) =
                                open.editor_state.selection.selected_element_ids.first()
                            {
                                let mut found_layer_id = String::new();
                                for layer in &open.sprite.layers {
                                    if layer.elements.iter().any(|e| e.id == *selected_id) {
                                        found_layer_id = layer.id.clone();
                                        break;
                                    }
                                }
                                (selected_id.clone(), found_layer_id)
                            } else {
                                open.editor_state.toast = Some(ToastMessage {
                                    text: "Select an element first to add a track".to_string(),
                                    created: std::time::Instant::now(),
                                });
                                return;
                            }
                        }
                    }
                } else {
                    (element_id, layer_id)
                };

                let before = open.sprite.clone();
                if let Some(ref seq_id) = open.editor_state.animation.selected_sequence_id.clone()
                    && let Some(seq) = open.sprite.animations.iter_mut().find(|a| a.id == *seq_id) {
                        seq.find_or_create_track(property, &resolved_element_id, &resolved_layer_id);
                    }
                history.push(SnapshotCommand {
                    description: "Add track".to_string(),
                    sprite_index,
                    before,
                    after: open.sprite.clone(),
                });
                project_state.mark_changed();
            }
        }
        timeline::TimelineAction::RemoveTrack(track_index) => {
            if let Some(open) = project_state.active_sprite_mut() {
                let before = open.sprite.clone();
                if let Some(ref seq_id) = open.editor_state.animation.selected_sequence_id.clone()
                    && let Some(seq) = open.sprite.animations.iter_mut().find(|a| a.id == *seq_id)
                        && track_index < seq.tracks.len() {
                            seq.tracks.remove(track_index);
                        }
                if open.editor_state.animation.selected_track_index == Some(track_index) {
                    open.editor_state.animation.selected_track_index = None;
                    open.editor_state.animation.selected_keyframe_id = None;
                }
                history.push(SnapshotCommand {
                    description: "Remove track".to_string(),
                    sprite_index,
                    before,
                    after: open.sprite.clone(),
                });
                project_state.mark_changed();
            }
        }
    }
}

/// Keyboard shortcut flags returned from handle_keyboard_shortcuts
struct ShortcutFlags {
    save_project: bool,
    new_sprite: bool,
}

fn handle_keyboard_shortcuts(
    ctx: &egui::Context,
    project_state: &mut ProjectState,
    history: &mut History,
) -> ShortcutFlags {
    let mut do_save = false;
    let mut do_new_sprite = false;

    ctx.input(|i| {
        // Ctrl+S: save project
        if i.modifiers.ctrl && i.key_pressed(egui::Key::S) {
            do_save = true;
        }

        // Ctrl+N: new sprite
        if i.modifiers.ctrl && i.key_pressed(egui::Key::N) {
            do_new_sprite = true;
        }

        // Tool switching (only when a sprite editor is active)
        if i.key_pressed(egui::Key::Num1)
            && let Some(open) = project_state.active_sprite_mut() {
                open.editor_state.active_tool = ToolKind::Line;
            }
        if i.key_pressed(egui::Key::Num2)
            && let Some(open) = project_state.active_sprite_mut() {
                open.editor_state.active_tool = ToolKind::Select;
            }
        if i.key_pressed(egui::Key::Num3)
            && let Some(open) = project_state.active_sprite_mut() {
                open.editor_state.active_tool = ToolKind::Fill;
            }
        if i.key_pressed(egui::Key::Num4)
            && let Some(open) = project_state.active_sprite_mut() {
                open.editor_state.active_tool = ToolKind::Eraser;
            }

        // Curve mode toggle
        if i.key_pressed(egui::Key::C) && !i.modifiers.ctrl
            && let Some(open) = project_state.active_sprite_mut()
                && open.editor_state.active_tool == ToolKind::Line {
                    open.editor_state.curve_mode = !open.editor_state.curve_mode;
                }

        // Space bar: toggle animation playback
        if i.key_pressed(egui::Key::Space) && !i.modifiers.ctrl
            && let Some(open) = project_state.active_sprite_mut()
                && open.editor_state.animation.selected_sequence_id.is_some() {
                    if open.editor_state.animation.playing {
                        open.editor_state.animation.playing = false;
                        open.editor_state.animation.playback_start_instant = None;
                    } else {
                        open.editor_state.animation.playing = true;
                        open.editor_state.animation.playback_start_time =
                            open.editor_state.animation.current_time;
                        open.editor_state.animation.playback_start_instant =
                            Some(std::time::Instant::now());
                        if open.editor_state.animation.current_time < 0.01 {
                            open.physics_state.reset();
                        }
                    }
                }

        // Home key: jump to start
        if i.key_pressed(egui::Key::Home)
            && let Some(open) = project_state.active_sprite_mut() {
                open.editor_state.animation.current_time = 0.0;
                if open.editor_state.animation.playing {
                    open.editor_state.animation.playback_start_time = 0.0;
                    open.editor_state.animation.playback_start_instant =
                        Some(std::time::Instant::now());
                }
            }

        // Escape to finish current line element
        if i.key_pressed(egui::Key::Escape)
            && let Some(open) = project_state.active_sprite_mut()
                && open.editor_state.line_tool_state.active_element_id.is_some() {
                    let layer_idx = open.editor_state.active_layer_index;
                    if let Some(ref active_id) = open.editor_state.line_tool_state.active_element_id
                        && layer_idx < open.sprite.layers.len()
                            && let Some(element) = open.sprite.layers[layer_idx]
                                .elements
                                .iter()
                                .find(|e| &e.id == active_id)
                                && element.vertices.len() < 2 {
                                    let id = active_id.clone();
                                    open.sprite.layers[layer_idx]
                                        .elements
                                        .retain(|e| e.id != id);
                                }
                    open.editor_state.line_tool_state.active_element_id = None;
                }

        // Delete key: remove selected elements
        if i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace) {
            let sprite_index = project_state.current_sprite_index();
            if let Some(open) = project_state.active_sprite_mut()
                && open.editor_state.active_tool == ToolKind::Select
                    && !open.editor_state.selection.selected_element_ids.is_empty()
                {
                    let before = open.sprite.clone();
                    let selected_ids = open.editor_state.selection.selected_element_ids.clone();
                    for layer in &mut open.sprite.layers {
                        layer.elements.retain(|e| !selected_ids.contains(&e.id));
                    }
                    open.editor_state.selection.clear();
                    history.push(SnapshotCommand {
                        description: "Delete elements".to_string(),
                        sprite_index,
                        before,
                        after: open.sprite.clone(),
                    });
                }
        }

        // Ctrl+A: select all elements on unlocked layers
        if i.modifiers.ctrl && i.key_pressed(egui::Key::A)
            && let Some(open) = project_state.active_sprite_mut()
                && open.editor_state.active_tool == ToolKind::Select {
                    open.editor_state.selection.clear();
                    for layer in &open.sprite.layers {
                        if !layer.locked && layer.visible {
                            for element in &layer.elements {
                                open.editor_state.selection.select_element(&element.id);
                            }
                        }
                    }
                }

        // Ctrl+C: copy selected elements
        if i.modifiers.ctrl && i.key_pressed(egui::Key::C)
            && let Some(open) = project_state.active_sprite_mut()
                && !open.editor_state.selection.selected_element_ids.is_empty() {
                    let mut copied_elements = Vec::new();
                    for layer in &open.sprite.layers {
                        for element in &layer.elements {
                            if open.editor_state.selection.is_element_selected(&element.id) {
                                copied_elements.push(element.clone());
                            }
                        }
                    }
                    open.editor_state.clipboard = Some(ClipboardData {
                        elements: copied_elements,
                    });
                }

        // Ctrl+V: paste copied elements (creates new layer with copies)
        if i.modifiers.ctrl && i.key_pressed(egui::Key::V) {
            let sprite_index = project_state.current_sprite_index();
            if let Some(open) = project_state.active_sprite_mut()
                && let Some(ref clipboard) = open.editor_state.clipboard.clone()
                    && !clipboard.elements.is_empty() {
                        let before = open.sprite.clone();
                        let mut paste_layer = Layer::new("Pasted");

                        for elem in &clipboard.elements {
                            let mut new_elem = elem.clone();
                            new_elem.id = uuid::Uuid::new_v4().to_string();
                            for v in &mut new_elem.vertices {
                                v.id = uuid::Uuid::new_v4().to_string();
                                v.pos.x += 10.0;
                                v.pos.y += 10.0;
                                if let Some(ref mut cp1) = v.cp1 {
                                    cp1.x += 10.0;
                                    cp1.y += 10.0;
                                }
                                if let Some(ref mut cp2) = v.cp2 {
                                    cp2.x += 10.0;
                                    cp2.y += 10.0;
                                }
                            }
                            paste_layer.elements.push(new_elem);
                        }

                        open.sprite.layers.push(paste_layer);
                        let new_layer_idx = open.sprite.layers.len() - 1;
                        open.editor_state.active_layer_index = new_layer_idx;

                        open.editor_state.selection.clear();
                        for elem in &open.sprite.layers[new_layer_idx].elements {
                            open.editor_state.selection.select_element(&elem.id);
                        }

                        history.push(SnapshotCommand {
                            description: "Paste elements".to_string(),
                            sprite_index,
                            before,
                            after: open.sprite.clone(),
                        });
                    }
        }

        // Undo: Ctrl+Z
        if i.modifiers.ctrl && i.key_pressed(egui::Key::Z) && !i.modifiers.shift
            && let Some(cmd) = history.undo()
                && let Some(open) = project_state.open_sprites.get_mut(cmd.sprite_index) {
                    open.sprite = cmd.before.clone();
                }

        // Redo: Ctrl+Y or Ctrl+Shift+Z
        if ((i.modifiers.ctrl && i.key_pressed(egui::Key::Y))
            || (i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Z)))
            && let Some(cmd) = history.redo()
                && let Some(open) = project_state.open_sprites.get_mut(cmd.sprite_index) {
                    open.sprite = cmd.after.clone();
                }
    });

    ShortcutFlags {
        save_project: do_save,
        new_sprite: do_new_sprite,
    }
}

/// Mirror all elements in a layer horizontally (h=true) or vertically (h=false)
fn mirror_layer(layer: &mut Layer, horizontal: bool) {
    if layer.elements.is_empty() {
        return;
    }

    // Compute bounding box center
    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    let mut min_y = f32::MAX;
    let mut max_y = f32::MIN;

    for element in &layer.elements {
        for vertex in &element.vertices {
            min_x = min_x.min(vertex.pos.x);
            max_x = max_x.max(vertex.pos.x);
            min_y = min_y.min(vertex.pos.y);
            max_y = max_y.max(vertex.pos.y);
        }
    }

    let center_x = (min_x + max_x) / 2.0;
    let center_y = (min_y + max_y) / 2.0;

    for element in &mut layer.elements {
        for vertex in &mut element.vertices {
            if horizontal {
                vertex.pos.x = 2.0 * center_x - vertex.pos.x;
                if let Some(ref mut cp1) = vertex.cp1 {
                    cp1.x = 2.0 * center_x - cp1.x;
                }
                if let Some(ref mut cp2) = vertex.cp2 {
                    cp2.x = 2.0 * center_x - cp2.x;
                }
            } else {
                vertex.pos.y = 2.0 * center_y - vertex.pos.y;
                if let Some(ref mut cp1) = vertex.cp1 {
                    cp1.y = 2.0 * center_y - cp1.y;
                }
                if let Some(ref mut cp2) = vertex.cp2 {
                    cp2.y = 2.0 * center_y - cp2.y;
                }
            }
        }
    }
}

/// Sanitize a string for use as a filename.
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("Sprite Tool"),
        ..Default::default()
    };

    eframe::run_native(
        "Sprite Tool",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
