use tauri::State;

use crate::adapters::traits::get_adapter;
use crate::core::state::AppState;
use crate::core::types::{ApplyTableChangesParams, ApplyTableChangesResponse};

#[tauri::command]
pub async fn apply_table_changes(
    params: ApplyTableChangesParams,
    state: State<'_, AppState>,
) -> Result<ApplyTableChangesResponse, String> {
    let (connection, _) = crate::core::db::get_connection_snapshot(&state).await?;
    get_adapter(connection.database_type)
        .apply_table_changes(&connection, &params)
        .await
}
