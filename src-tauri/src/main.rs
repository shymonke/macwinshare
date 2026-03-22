//! MacWinShare Tauri Application

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;

use tauri::Manager;
use tracing_subscriber::EnvFilter;

fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("macwinshare=debug,info")),
        )
        .init();

    tracing::info!("Starting MacWinShare v{}", env!("CARGO_PKG_VERSION"));

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Initialize application state
            let config = macwinshare_core::Config::load()
                .unwrap_or_else(|_| macwinshare_core::Config::default());
            
            app.manage(std::sync::Arc::new(tokio::sync::RwLock::new(config)));

            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::start_server,
            commands::start_client,
            commands::stop,
            commands::get_status,
            commands::discover_peers,
            commands::get_displays,
            commands::check_accessibility,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
