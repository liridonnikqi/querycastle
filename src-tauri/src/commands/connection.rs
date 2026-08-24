use std::collections::HashMap;

use tauri::State;
use tracing::info;

use crate::adapters::traits::get_adapter;
use crate::core::pool::create_pool;
use crate::core::state::{disconnected_status, ActiveConnection, AppState};
use crate::core::types::{ConnectionInput, ConnectionStatus, TestConnectionResponse};

#[tauri::command]
pub async fn connection_status(state: State<'_, AppState>) -> Result<ConnectionStatus, String> {
    let guard = state.inner.read().await;
    if let Some(active) = guard.as_ref() {
        return Ok(ConnectionStatus {
            connected: true,
            database_type: active.input.database_type,
            name: active.input.name.clone(),
            host: active.input.host.clone(),
            port: active.input.port,
            database: active.input.database.clone(),
            user: active.input.user.clone(),
            server_version: active.server_version.clone(),
        });
    }

    Ok(disconnected_status())
}

#[tauri::command]
pub async fn test_connection(params: ConnectionInput) -> Result<TestConnectionResponse, String> {
    let normalized = crate::core::db::normalize_connection_input(params)?;
    get_adapter(normalized.database_type)
        .test_connection(&normalized)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn connect(
    params: ConnectionInput,
    state: State<'_, AppState>,
) -> Result<ConnectionStatus, String> {
    let normalized = crate::core::db::normalize_connection_input(params)?;
    // Create pool first for validation; pooling errors surface as connection errors
    let pool = create_pool(&normalized).map_err(|e| e.to_string())?;
    info!("Creating pool for {:?} at {}:{}", normalized.database_type, normalized.host, normalized.port);
    let status = get_adapter(normalized.database_type)
        .connect(&normalized)
        .await
        .map_err(|e| e.to_string())?;

    {
        let mut guard = state.inner.write().await;
        *guard = Some(ActiveConnection {
            input: normalized.clone(),
            server_version: status.server_version.clone(),
            pool,
        });
    }

    Ok(status)
}

#[tauri::command]
pub async fn disconnect(state: State<'_, AppState>) -> Result<HashMap<String, bool>, String> {
    {
        let mut guard = state.inner.write().await;
        if guard.is_some() {
            info!("Disconnecting and dropping pool");
        }
        *guard = None;
    }

    Ok(HashMap::from([(String::from("ok"), true)]))
}
