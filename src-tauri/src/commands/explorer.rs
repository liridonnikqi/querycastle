use tauri::State;

use crate::adapters::traits::get_adapter;
use crate::core::state::AppState;
use crate::core::types::{
    ConnectionStatus, DatabaseExplorer, ObjectDefinition, ObjectDefinitionParams, SelectDatabaseParams,
};

#[tauri::command]
pub async fn get_database_explorer(
    state: State<'_, AppState>,
) -> Result<DatabaseExplorer, String> {
    let (connection, _) = crate::core::db::get_connection_snapshot(&state).await?;
    get_adapter(connection.database_type)
        .get_database_explorer(&connection)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_object_definition(
    params: ObjectDefinitionParams,
    state: State<'_, AppState>,
) -> Result<ObjectDefinition, String> {
    let (connection, _) = crate::core::db::get_connection_snapshot(&state).await?;
    get_adapter(connection.database_type)
        .get_object_definition(&connection, &params)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_databases(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let (connection, _) = crate::core::db::get_connection_snapshot(&state).await?;
    get_adapter(connection.database_type)
        .list_databases(&connection)
        .await
        .map_err(|e| e.to_string())
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
        .await
        .map_err(|e| e.to_string())?;

    // Recreate pool for the newly selected database
    let pool = crate::core::pool::create_pool(&next_connection).map_err(|e| e.to_string())?;
    {
        let mut guard = state.inner.write().await;
        *guard = Some(crate::core::state::ActiveConnection {
            input: next_connection.clone(),
            server_version: status.server_version.clone(),
            pool,
        });
    }

    Ok(status)
}
