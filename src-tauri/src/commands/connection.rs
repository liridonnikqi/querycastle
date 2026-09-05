use std::sync::Arc;

use tauri::State;
use tracing::info;

use crate::core::error::{DbError, StructuredDbError};
use crate::core::pool::create_pool;
use crate::core::state::{
    disconnected_status, new_session_id, status_from_active, ActiveConnection, AppState,
};
use crate::core::types::{ConnectionInput, ConnectionStatus, SessionIdParams, TestConnectionResponse};

#[tauri::command]
pub async fn connection_status(state: State<'_, AppState>) -> Result<ConnectionStatus, StructuredDbError> {
    if let Some(active) = state.get_active().await {
        return Ok(status_from_active(&active));
    }
    Ok(disconnected_status())
}

#[tauri::command]
pub async fn test_connection(params: ConnectionInput) -> Result<TestConnectionResponse, StructuredDbError> {
    let normalized = crate::core::connection::normalize_connection_input(params).map_err(StructuredDbError::from)?;
    crate::adapters::test_connection(&normalized)
        .await
        .map_err(StructuredDbError::from)
}

#[tauri::command]
pub async fn connect(
    params: ConnectionInput,
    state: State<'_, AppState>,
) -> Result<ConnectionStatus, StructuredDbError> {
    let normalized = crate::core::connection::normalize_connection_input(params).map_err(StructuredDbError::from)?;
    let pool = create_pool(&normalized).map_err(StructuredDbError::from)?;
    info!(
        "Creating pool for {:?} at {}:{}",
        normalized.database_type, normalized.host, normalized.port
    );
    let server_version = crate::adapters::server_version(&pool)
        .await
        .map_err(StructuredDbError::from)?;

    let id = new_session_id();
    let active = ActiveConnection {
        id: id.clone(),
        input: normalized,
        server_version,
        pool,
    };
    let status = status_from_active(&active);
    {
        let mut guard = state.inner.write().await;
        guard.sessions.insert(id.clone(), Arc::new(active));
        guard.active_id = Some(id);
    }

    Ok(status)
}

#[tauri::command]
pub async fn disconnect(state: State<'_, AppState>) -> Result<(), StructuredDbError> {
    let mut guard = state.inner.write().await;
    let n = guard.sessions.len();
    guard.sessions.clear();
    guard.active_id = None;
    info!("Disconnected all sessions ({n})");
    Ok(())
}

#[tauri::command]
pub async fn switch_session(
    params: SessionIdParams,
    state: State<'_, AppState>,
) -> Result<ConnectionStatus, StructuredDbError> {
    let id = params.session_id.trim();
    if id.is_empty() {
        return Err(DbError::validation("Session id is required").into());
    }
    let mut guard = state.inner.write().await;
    let Some(session) = guard.sessions.get(id) else {
        return Err(DbError::NotFound("Connection session not found".to_string()).into());
    };
    let status = status_from_active(session);
    guard.active_id = Some(id.to_string());
    Ok(status)
}

#[tauri::command]
pub async fn disconnect_session(
    params: SessionIdParams,
    state: State<'_, AppState>,
) -> Result<ConnectionStatus, StructuredDbError> {
    let id = params.session_id.trim();
    if id.is_empty() {
        return Err(DbError::validation("Session id is required").into());
    }
    let mut guard = state.inner.write().await;
    if guard.sessions.remove(id).is_none() {
        return Err(DbError::NotFound("Connection session not found".to_string()).into());
    }
    info!("Disconnected session {id}");
    if guard.active_id.as_deref() == Some(id) {
        guard.active_id = guard.sessions.keys().next().cloned();
    }
    if let Some(active_id) = guard.active_id.clone() {
        if let Some(active) = guard.sessions.get(&active_id) {
            return Ok(status_from_active(active));
        }
    }
    Ok(disconnected_status())
}
