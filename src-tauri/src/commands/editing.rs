use tauri::State;

use crate::core::error::StructuredDbError;
use crate::core::state::AppState;
use crate::core::types::{ApplyTableChangesParams, ApplyTableChangesResponse};

#[tauri::command]
pub async fn apply_table_changes(
    params: ApplyTableChangesParams,
    state: State<'_, AppState>,
) -> Result<ApplyTableChangesResponse, StructuredDbError> {
    let session = state
        .require_session(&params.session_id)
        .await
        .map_err(StructuredDbError::from)?;
    crate::adapters::apply_table_changes(&session.pool, &params)
        .await
        .map_err(StructuredDbError::from)
}
