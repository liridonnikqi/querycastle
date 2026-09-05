use tauri::State;

use crate::core::error::{DbError, StructuredDbError};
use crate::core::pool::create_pool;
use crate::core::state::{status_from_active, AppState};
use crate::core::types::{
    ConnectionStatus, DatabaseExplorer, ObjectDefinition, ObjectDefinitionParams, SelectDatabaseParams,
    SessionIdParams,
};

#[tauri::command]
pub async fn get_database_explorer(
    params: SessionIdParams,
    state: State<'_, AppState>,
) -> Result<DatabaseExplorer, StructuredDbError> {
    let session = state
        .require_session(&params.session_id)
        .await
        .map_err(StructuredDbError::from)?;
    crate::adapters::get_database_explorer(&session.pool)
        .await
        .map_err(StructuredDbError::from)
}

#[tauri::command]
pub async fn get_object_definition(
    params: ObjectDefinitionParams,
    state: State<'_, AppState>,
) -> Result<ObjectDefinition, StructuredDbError> {
    let session = state
        .require_session(&params.session_id)
        .await
        .map_err(StructuredDbError::from)?;
    crate::adapters::get_object_definition(&session.pool, &params)
        .await
        .map_err(StructuredDbError::from)
}

#[tauri::command]
pub async fn list_databases(
    params: SessionIdParams,
    state: State<'_, AppState>,
) -> Result<Vec<String>, StructuredDbError> {
    let session = state
        .require_session(&params.session_id)
        .await
        .map_err(StructuredDbError::from)?;
    crate::adapters::list_databases(&session.pool)
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

    let session = state
        .require_session(&params.session_id)
        .await
        .map_err(StructuredDbError::from)?;
    if session.input.database == next_database {
        return Ok(status_from_active(&session));
    }

    let session_id = session.id.clone();
    let next_connection = crate::core::connection::with_new_database(&session.input, next_database);
    let pool = create_pool(&next_connection).map_err(StructuredDbError::from)?;
    let server_version = crate::adapters::server_version(&pool)
        .await
        .map_err(StructuredDbError::from)?;

    let next = std::sync::Arc::new(crate::core::state::ActiveConnection {
        id: session_id.clone(),
        input: next_connection,
        server_version,
        pool,
    });
    let status = status_from_active(&next);
    let mut guard = state.inner.write().await;
    if !guard.sessions.contains_key(&session_id) {
        return Err(DbError::NotFound("Connection session not found".to_string()).into());
    }
    guard.sessions.insert(session_id, next);
    Ok(status)
}
