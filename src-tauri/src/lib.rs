mod adapters;
mod commands;
mod core;
mod services;

use crate::core::state::AppState;
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}


