use std::collections::HashMap;

use tauri::State;

use crate::adapters::traits::get_adapter;
use crate::core::state::{disconnected_status, AppState};
use crate::core::types::{ConnectionInput, ConnectionStatus, TestConnectionResponse};

#[tauri::command]
pub async fn connection_status(state: State<'_, AppState>) -> Result<ConnectionStatus, String> {
    let connection = state.connection.lock().await.clone();
    let server_version = state.server_version.lock().await.clone();

    if let Some(active) = connection {
        return Ok(ConnectionStatus {
            connected: true,
            database_type: active.database_type,
            name: active.name,
            host: active.host,
            port: active.port,
            database: active.database,
            user: active.user,
            server_version,
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
}

#[tauri::command]
pub async fn connect(
    params: ConnectionInput,
    state: State<'_, AppState>,
) -> Result<ConnectionStatus, String> {
    let normalized = crate::core::db::normalize_connection_input(params)?;
    let status = get_adapter(normalized.database_type)
        .connect(&normalized)
        .await?;

    {
        let mut connection = state.connection.lock().await;
        *connection = Some(normalized.clone());
    }
    {
        let mut version = state.server_version.lock().await;
        *version = status.server_version.clone();
    }

    Ok(status)
}

#[tauri::command]
pub async fn disconnect(state: State<'_, AppState>) -> Result<HashMap<String, bool>, String> {
    {
        let mut connection = state.connection.lock().await;
        *connection = None;
    }
    {
        let mut version = state.server_version.lock().await;
        *version = None;
    }

    Ok(HashMap::from([(String::from("ok"), true)]))
}
