use async_trait::async_trait;

use crate::core::error::DbError;
use crate::core::types::{
    ApplyTableChangesParams, ApplyTableChangesResponse, ConnectionInput, ConnectionStatus,
    DatabaseExplorer, DatabaseType, ObjectDefinition, ObjectDefinitionParams, QueryResultPayload,
    TestConnectionResponse,
};

#[async_trait]
pub trait DbAdapter: Sync + Send {
    async fn test_connection(&self, connection: &ConnectionInput)
        -> Result<TestConnectionResponse, DbError>;
    async fn connect(&self, connection: &ConnectionInput) -> Result<ConnectionStatus, DbError>;
    async fn run_query(
        &self,
        connection: &ConnectionInput,
        sql: &str,
    ) -> Result<QueryResultPayload, DbError>;
    async fn get_database_explorer(
        &self,
        connection: &ConnectionInput,
    ) -> Result<DatabaseExplorer, DbError>;
    async fn get_object_definition(
        &self,
        connection: &ConnectionInput,
        params: &ObjectDefinitionParams,
    ) -> Result<ObjectDefinition, DbError>;
    async fn list_databases(&self, connection: &ConnectionInput) -> Result<Vec<String>, DbError>;
    async fn select_database(
        &self,
        connection: &ConnectionInput,
        database: &str,
    ) -> Result<(ConnectionInput, ConnectionStatus), DbError>;
    async fn apply_table_changes(
        &self,
        connection: &ConnectionInput,
        params: &ApplyTableChangesParams,
    ) -> Result<ApplyTableChangesResponse, DbError>;
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
