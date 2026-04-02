use tauri::State;

use crate::adapters::traits::get_adapter;
use crate::core::state::AppState;
use crate::core::types::{ConnectionStatus, DatabaseExplorer, SelectDatabaseParams};

#[tauri::command]
pub async fn get_database_explorer(
    state: State<'_, AppState>,
) -> Result<DatabaseExplorer, String> {
    let (connection, _) = crate::core::db::get_connection_snapshot(&state).await?;
    get_adapter(connection.database_type)
        .get_database_explorer(&connection)
        .await
}

#[tauri::command]
pub async fn list_databases(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let (connection, _) = crate::core::db::get_connection_snapshot(&state).await?;
    get_adapter(connection.database_type)
        .list_databases(&connection)
        .await
}

#[tauri::command]
pub async fn select_database(
    params: SelectDatabaseParams,
    state: State<'_, AppState>,
) -> Result<ConnectionStatus, String> {
    let next_database = params.database.trim();
    if next_database.is_empty() {
        return Err("Database name is required".to_string());
    }

    let (connection, _) = crate::core::db::get_connection_snapshot(&state).await?;
    if connection.database == next_database {
        return crate::commands::connection::connection_status(state).await;
    }

    let (next_connection, status) = get_adapter(connection.database_type)
        .select_database(&connection, next_database)
        .await?;

    {
        let mut active = state.connection.lock().await;
        *active = Some(next_connection.clone());
    }
    {
        let mut version = state.server_version.lock().await;
        *version = status.server_version.clone();
    }

    Ok(status)
}
