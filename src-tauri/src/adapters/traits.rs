use async_trait::async_trait;

use crate::core::types::{
    ApplyTableChangesParams, ApplyTableChangesResponse, ConnectionInput, ConnectionStatus,
    DatabaseExplorer, DatabaseType, QueryResultPayload, TestConnectionResponse,
};

#[async_trait]
pub trait DbAdapter: Sync + Send {
    async fn test_connection(&self, connection: &ConnectionInput)
        -> Result<TestConnectionResponse, String>;
    async fn connect(&self, connection: &ConnectionInput) -> Result<ConnectionStatus, String>;
    async fn run_query(
        &self,
        connection: &ConnectionInput,
        sql: &str,
    ) -> Result<QueryResultPayload, String>;
    async fn get_database_explorer(
        &self,
        connection: &ConnectionInput,
    ) -> Result<DatabaseExplorer, String>;
    async fn list_databases(&self, connection: &ConnectionInput) -> Result<Vec<String>, String>;
    async fn select_database(
        &self,
        connection: &ConnectionInput,
        database: &str,
    ) -> Result<(ConnectionInput, ConnectionStatus), String>;
    async fn apply_table_changes(
        &self,
        connection: &ConnectionInput,
        params: &ApplyTableChangesParams,
    ) -> Result<ApplyTableChangesResponse, String>;
}

pub struct PostgresAdapter;
pub struct MySqlAdapter;
pub struct SqliteAdapter;

pub static POSTGRES_ADAPTER: PostgresAdapter = PostgresAdapter;
pub static MYSQL_ADAPTER: MySqlAdapter = MySqlAdapter;
pub static SQLITE_ADAPTER: SqliteAdapter = SqliteAdapter;

pub fn get_adapter(database_type: DatabaseType) -> &'static dyn DbAdapter {
    match database_type {
        DatabaseType::Postgres => &POSTGRES_ADAPTER as &dyn DbAdapter,
        DatabaseType::Mysql => &MYSQL_ADAPTER as &dyn DbAdapter,
        DatabaseType::Sqlite => &SQLITE_ADAPTER as &dyn DbAdapter,
    }
}
