mod commands;
mod models;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            commands::file::new_project,
            commands::file::open_project,
            commands::file::save_project,
            commands::file::new_sprite,
            commands::file::open_sprite,
            commands::file::save_sprite,
            commands::palette::fetch_lospec_palette,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
