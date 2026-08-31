use std::collections::HashMap;

use serde::Deserialize;
use tauri::State;
use tracing::info;

use crate::core::error::{DbError, StructuredDbError};
use crate::core::pool::create_pool;
use crate::core::state::{
    disconnected_status, new_session_id, status_from_active, ActiveConnection, AppState,
};
use crate::core::types::{ConnectionInput, ConnectionStatus, TestConnectionResponse};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionIdParams {
    pub session_id: String,
}

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
        guard.sessions.insert(id.clone(), active);
        guard.active_id = Some(id);
    }

    Ok(status)
}

#[tauri::command]
pub async fn disconnect(state: State<'_, AppState>) -> Result<HashMap<String, bool>, StructuredDbError> {
    disconnect_active_session(&state).await;
    Ok(HashMap::from([(String::from("ok"), true)]))
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
    if !guard.sessions.contains_key(id) {
        return Err(DbError::NotFound("Connection session not found".to_string()).into());
    }
    guard.active_id = Some(id.to_string());
    let active = guard.sessions.get(id).expect("session exists");
    Ok(status_from_active(active))
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

async fn disconnect_active_session(state: &State<'_, AppState>) {
    let mut guard = state.inner.write().await;
    let Some(id) = guard.active_id.clone() else {
        guard.sessions.clear();
        return;
    };
    guard.sessions.remove(&id);
    info!("Disconnecting session {id}");
    guard.active_id = guard.sessions.keys().next().cloned();
}
