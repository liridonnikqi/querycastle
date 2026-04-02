use tauri::State;

use crate::adapters::traits::get_adapter;
use crate::core::state::AppState;
use crate::core::types::{QueryParams, QueryResultPayload};

#[tauri::command]
pub async fn run_query(
    params: QueryParams,
    state: State<'_, AppState>,
) -> Result<QueryResultPayload, String> {
    let (connection, _) = crate::core::db::get_connection_snapshot(&state).await?;
    get_adapter(connection.database_type)
        .run_query(&connection, params.sql.as_str())
        .await
}
