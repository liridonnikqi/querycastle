use async_trait::async_trait;
use mysql_async::prelude::Queryable;

use crate::adapters::traits::{DbAdapter, MySqlAdapter};
use crate::core::types::{
    ApplyTableChangesParams, ApplyTableChangesResponse, ConnectionInput, ConnectionStatus,
    DatabaseExplorer, DatabaseType, QueryResultPayload, TestConnectionResponse,
};

#[async_trait]
impl DbAdapter for MySqlAdapter {
    async fn test_connection(&self, connection: &ConnectionInput) -> Result<TestConnectionResponse, String> {
        let mut conn = match crate::core::db::connect_mysql_client(connection).await {
            Ok(conn) => conn,
            Err(error) => {
                return Ok(TestConnectionResponse {
                    ok: false,
                    message: error,
                    server_version: None,
                })
            }
        };
        let server_version = crate::core::db::get_mysql_server_version(&mut conn).await?;
        Ok(TestConnectionResponse {
            ok: true,
            message: "Connection successful".to_string(),
            server_version,
        })
    }

    async fn connect(&self, connection: &ConnectionInput) -> Result<ConnectionStatus, String> {
        let mut conn = crate::core::db::connect_mysql_client(connection).await?;
        let server_version = crate::core::db::get_mysql_server_version(&mut conn).await?;
        Ok(ConnectionStatus {
            connected: true,
            database_type: DatabaseType::Mysql,
            name: connection.name.clone(),
            host: connection.host.clone(),
            port: connection.port,
            database: connection.database.clone(),
            user: connection.user.clone(),
            server_version,
        })
    }

    async fn run_query(&self, connection: &ConnectionInput, sql: &str) -> Result<QueryResultPayload, String> {
        crate::core::db::run_mysql_query(connection, sql).await
    }

    async fn get_database_explorer(&self, connection: &ConnectionInput) -> Result<DatabaseExplorer, String> {
        Ok(DatabaseExplorer {
            database: connection.database.clone(),
            schemas: Vec::new(),
        })
    }

    async fn list_databases(&self, connection: &ConnectionInput) -> Result<Vec<String>, String> {
        let mut conn = crate::core::db::connect_mysql_client(connection).await?;
        let dbs: Vec<String> = conn
            .query("show databases")
            .await
            .map_err(crate::core::db::sanitize_mysql_error)?;
        if dbs.is_empty() {
            Ok(vec![connection.database.clone()])
        } else {
            Ok(dbs)
        }
    }

    async fn select_database(
        &self,
        connection: &ConnectionInput,
        database: &str,
    ) -> Result<(ConnectionInput, ConnectionStatus), String> {
        let next_connection = ConnectionInput {
            database: database.to_string(),
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
        Err("Row editing for MySQL is planned in phase 3".to_string())
    }
}
