use tauri::State;

use crate::core::error::{DbError, StructuredDbError};
use crate::core::pool::create_pool;
use crate::core::state::{status_from_active, AppState};
use crate::core::types::{
    ConnectionStatus, DatabaseExplorer, ObjectDefinition, ObjectDefinitionParams, SelectDatabaseParams,
};

#[tauri::command]
pub async fn get_database_explorer(
    state: State<'_, AppState>,
) -> Result<DatabaseExplorer, StructuredDbError> {
    let active = state.require_active().await.map_err(StructuredDbError::from)?;
    crate::adapters::get_database_explorer(&active.pool)
        .await
        .map_err(StructuredDbError::from)
}

#[tauri::command]
pub async fn get_object_definition(
    params: ObjectDefinitionParams,
    state: State<'_, AppState>,
) -> Result<ObjectDefinition, StructuredDbError> {
    let active = state.require_active().await.map_err(StructuredDbError::from)?;
    crate::adapters::get_object_definition(&active.pool, &params)
        .await
        .map_err(StructuredDbError::from)
}

#[tauri::command]
pub async fn list_databases(state: State<'_, AppState>) -> Result<Vec<String>, StructuredDbError> {
    let active = state.require_active().await.map_err(StructuredDbError::from)?;
    crate::adapters::list_databases(&active.pool)
        .await
        .map_err(StructuredDbError::from)
}

#[tauri::command]
pub async fn select_database(
    params: SelectDatabaseParams,
    state: State<'_, AppState>,
) -> Result<ConnectionStatus, StructuredDbError> {
    let next_database = params.database.trim();
    if next_database.is_empty() {
        return Err(DbError::validation("Database name is required").into());
    }

    let active = state.require_active().await.map_err(StructuredDbError::from)?;
    if active.input.database == next_database {
        return Ok(status_from_active(&active));
    }

    let session_id = active.id.clone();
    let next_connection = crate::core::connection::with_new_database(&active.input, next_database);
    let pool = create_pool(&next_connection).map_err(StructuredDbError::from)?;
    let server_version = crate::adapters::server_version(&pool)
        .await
        .map_err(StructuredDbError::from)?;

    let mut guard = state.inner.write().await;
    let Some(session) = guard.sessions.get_mut(&session_id) else {
        return Err(DbError::NotFound("Connection session not found".to_string()).into());
    };
    session.input = next_connection;
    session.server_version = server_version;
    session.pool = pool;
    Ok(status_from_active(session))
}
