use tauri::State;

use crate::core::error::StructuredDbError;
use crate::core::state::AppState;
use crate::core::types::{QueryParams, QueryResultPayload};

#[tauri::command]
pub async fn run_query(
    params: QueryParams,
    state: State<'_, AppState>,
) -> Result<QueryResultPayload, StructuredDbError> {
    let session = state
        .require_session(&params.session_id)
        .await
        .map_err(StructuredDbError::from)?;
    crate::adapters::run_query(&session.pool, params.sql.as_str())
        .await
        .map_err(StructuredDbError::from)
}
