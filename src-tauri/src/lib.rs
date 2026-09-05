mod adapters;
mod commands;
mod core;

use crate::core::state::AppState;
pub fn run() {
    // Initialize tracing for core DB diagnostics
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")))
        .try_init();
    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build());

    #[cfg(all(debug_assertions, feature = "mcp"))]
    {
        builder = builder.plugin(tauri_plugin_mcp_bridge::init());
    }

    builder
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::connection::connection_status,
            commands::connection::test_connection,
            commands::connection::connect,
            commands::connection::disconnect,
            commands::connection::switch_session,
            commands::connection::disconnect_session,
            commands::secrets::secret_set,
            commands::secrets::secret_get,
            commands::secrets::secret_delete,
            commands::query::run_query,
            commands::explorer::get_database_explorer,
            commands::explorer::get_object_definition,
            commands::explorer::list_databases,
            commands::explorer::select_database,
            commands::editing::apply_table_changes,
            commands::system::get_launch_sql_file,
            commands::system::get_launch_sqlite_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}


