mod adapters;
mod commands;
mod core;
mod services;

use crate::core::state::AppState;
pub fn run() {
    // Initialize tracing for core DB diagnostics
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")))
        .try_init();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::connection::connection_status,
            commands::connection::test_connection,
            commands::connection::connect,
            commands::connection::disconnect,
            commands::query::run_query,
            commands::explorer::get_database_explorer,
            commands::explorer::list_databases,
            commands::explorer::select_database,
            commands::editing::apply_table_changes,
            commands::system::get_launch_sql_file,
            commands::system::get_launch_sqlite_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}


