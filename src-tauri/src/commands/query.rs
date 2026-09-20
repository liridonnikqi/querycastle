use tauri::State;
use tokio_util::sync::CancellationToken;

use crate::core::error::{DbError, StructuredDbError};
use crate::core::sql::is_mutating_sql;
use crate::core::state::AppState;
use crate::core::types::{CancelQueryParams, QueryParams, QueryResultPayload};

#[tauri::command]
pub async fn run_query(
    params: QueryParams,
    state: State<'_, AppState>,
) -> Result<QueryResultPayload, StructuredDbError> {
    let session = state
        .require_session(&params.session_id)
        .await
        .map_err(StructuredDbError::from)?;
    if session.input.read_only && is_mutating_sql(&params.sql) {
        return Err(DbError::read_only().into());
    }
    let cancel = CancellationToken::new();
    state
        .register_query(params.query_id.clone(), cancel.clone())
        .await;
    let result = crate::adapters::run_query(&session.pool, params.sql.as_str(), cancel.clone()).await;
    state.unregister_query(&params.query_id).await;
    match result {
        Ok(payload) => Ok(payload),
        Err(_) if cancel.is_cancelled() => Err(DbError::cancelled().into()),
        Err(err) => Err(err.into()),
    }
}

#[tauri::command]
pub async fn cancel_query(
    params: CancelQueryParams,
    state: State<'_, AppState>,
) -> Result<(), StructuredDbError> {
    state
        .cancel_query(&params.query_id)
        .await
        .map_err(StructuredDbError::from)
}
