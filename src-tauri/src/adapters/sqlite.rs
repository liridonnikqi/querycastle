use async_trait::async_trait;

use crate::adapters::traits::{DbAdapter, SqliteAdapter};
use crate::core::types::{
    ApplyTableChangesParams, ApplyTableChangesResponse, ConnectionInput, ConnectionStatus,
    DatabaseExplorer, DatabaseType, QueryResultPayload, TestConnectionResponse,
};

#[async_trait]
impl DbAdapter for SqliteAdapter {
    async fn test_connection(&self, connection: &ConnectionInput) -> Result<TestConnectionResponse, String> {
        let conn = match crate::core::db::open_sqlite_connection(connection) {
            Ok(conn) => conn,
            Err(error) => {
                return Ok(TestConnectionResponse {
                    ok: false,
                    message: error,
                    server_version: None,
                })
            }
        };
        let server_version = crate::core::db::get_sqlite_server_version(&conn)?;
        Ok(TestConnectionResponse {
            ok: true,
            message: "Connection successful".to_string(),
            server_version,
        })
    }

    async fn connect(&self, connection: &ConnectionInput) -> Result<ConnectionStatus, String> {
        let conn = crate::core::db::open_sqlite_connection(connection)?;
        let server_version = crate::core::db::get_sqlite_server_version(&conn)?;
        Ok(ConnectionStatus {
            connected: true,
            database_type: DatabaseType::Sqlite,
            name: connection.name.clone(),
            host: String::new(),
            port: 0,
            database: connection.database.clone(),
            user: String::new(),
            server_version,
        })
    }

    async fn run_query(&self, connection: &ConnectionInput, sql: &str) -> Result<QueryResultPayload, String> {
        crate::core::db::run_sqlite_query(connection, sql)
    }

    async fn get_database_explorer(&self, connection: &ConnectionInput) -> Result<DatabaseExplorer, String> {
        crate::core::db::get_sqlite_database_explorer(connection)
    }

    async fn list_databases(&self, connection: &ConnectionInput) -> Result<Vec<String>, String> {
        Ok(crate::core::db::list_sqlite_databases(connection))
    }

    async fn select_database(
        &self,
        connection: &ConnectionInput,
        database: &str,
    ) -> Result<(ConnectionInput, ConnectionStatus), String> {
        let next_connection = ConnectionInput {
            database: database.to_string(),
            host: String::new(),
            port: 0,
            user: String::new(),
            ssl: false,
            ..connection.clone()
        };
        let status = self.connect(&next_connection).await?;
        Ok((next_connection, status))
    }

    async fn apply_table_changes(
        &self,
        _connection: &ConnectionInput,
        _params: &ApplyTableChangesParams,
    ) -> Result<ApplyTableChangesResponse, String> {
        Err("SQLite support is planned for phase 2".to_string())
    }
}
