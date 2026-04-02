use async_trait::async_trait;

use crate::adapters::traits::{DbAdapter, SqliteAdapter};
use crate::core::types::{
    ApplyTableChangesParams, ApplyTableChangesResponse, ConnectionInput, ConnectionStatus,
    DatabaseExplorer, QueryResultPayload, TestConnectionResponse,
};

#[async_trait]
impl DbAdapter for SqliteAdapter {
    async fn test_connection(&self, _connection: &ConnectionInput) -> Result<TestConnectionResponse, String> {
        Ok(TestConnectionResponse {
            ok: false,
            message: "SQLite support is planned for phase 2".to_string(),
            server_version: None,
        })
    }

    async fn connect(&self, _connection: &ConnectionInput) -> Result<ConnectionStatus, String> {
        Err("SQLite support is planned for phase 2".to_string())
    }

    async fn run_query(&self, _connection: &ConnectionInput, _sql: &str) -> Result<QueryResultPayload, String> {
        Err("SQLite support is planned for phase 2".to_string())
    }

    async fn get_database_explorer(&self, _connection: &ConnectionInput) -> Result<DatabaseExplorer, String> {
        Err("SQLite support is planned for phase 2".to_string())
    }

    async fn list_databases(&self, _connection: &ConnectionInput) -> Result<Vec<String>, String> {
        Err("SQLite support is planned for phase 2".to_string())
    }

    async fn select_database(
        &self,
        _connection: &ConnectionInput,
        _database: &str,
    ) -> Result<(ConnectionInput, ConnectionStatus), String> {
        Err("SQLite support is planned for phase 2".to_string())
    }

    async fn apply_table_changes(
        &self,
        _connection: &ConnectionInput,
        _params: &ApplyTableChangesParams,
    ) -> Result<ApplyTableChangesResponse, String> {
        Err("SQLite support is planned for phase 2".to_string())
    }
}
