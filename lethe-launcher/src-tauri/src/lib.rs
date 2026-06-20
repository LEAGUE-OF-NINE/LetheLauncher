mod auth;
mod commands;
mod config;
mod downloader;
mod launcher;
mod logger;
mod manifest;
mod steam;
mod updater;
mod verifier;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logger
    logger::init_logger();

    // Create default config if missing
    config::create_default_config();

    // Log working directory for debugging
    if let Ok(cwd) = std::env::current_dir() {
        lethe_log!("Working directory: {}", cwd.display());
    }

    // Note: DisableAutoUpdate is now a frontend-controlled setting.
    // The UI always loads so the user can login, change settings, and launch.
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_sync,
            commands::launch,
            commands::get_auto_update_status,
            commands::start_oauth,
            commands::get_saved_auth,
            commands::logout,
            commands::check_for_updates,
            commands::download_update,
            commands::get_settings,
            commands::set_setting,
            commands::get_mods,
            commands::toggle_mod,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
