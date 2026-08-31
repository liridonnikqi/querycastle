pub mod mysql;
pub mod postgres;
pub mod sqlite;

use crate::core::error::DbError;
use crate::core::pool::{create_pool, Pool};
use crate::core::types::*;

pub async fn run_query(pool: &Pool, sql: &str) -> Result<QueryResultPayload, DbError> {
    match pool {
        Pool::Postgres(p) => postgres::run_query(p, sql).await,
        Pool::Mysql(p) => mysql::run_query(p, sql).await,
        Pool::Sqlite(p) => sqlite::run_query(p, sql).await,
    }
}

pub async fn get_database_explorer(pool: &Pool) -> Result<DatabaseExplorer, DbError> {
    match pool {
        Pool::Postgres(p) => postgres::get_database_explorer(p).await,
        Pool::Mysql(p) => mysql::get_database_explorer(p).await,
        Pool::Sqlite(p) => sqlite::get_database_explorer(p).await,
    }
}

pub async fn get_object_definition(
    pool: &Pool,
    params: &ObjectDefinitionParams,
) -> Result<ObjectDefinition, DbError> {
    match pool {
        Pool::Postgres(p) => postgres::get_object_definition(p, params).await,
        Pool::Mysql(p) => mysql::get_object_definition(p, params).await,
        Pool::Sqlite(p) => sqlite::get_object_definition(p, params).await,
    }
}

pub async fn list_databases(pool: &Pool) -> Result<Vec<String>, DbError> {
    match pool {
        Pool::Postgres(p) => postgres::list_databases(p).await,
        Pool::Mysql(p) => mysql::list_databases(p).await,
        Pool::Sqlite(p) => sqlite::list_databases(p).await,
    }
}

pub async fn apply_table_changes(
    pool: &Pool,
    params: &ApplyTableChangesParams,
) -> Result<ApplyTableChangesResponse, DbError> {
    match pool {
        Pool::Postgres(p) => postgres::apply_table_changes(p, params).await,
        Pool::Mysql(p) => mysql::apply_table_changes(p, params).await,
        Pool::Sqlite(p) => sqlite::apply_table_changes(p, params).await,
    }
}

pub async fn server_version(pool: &Pool) -> Result<Option<String>, DbError> {
    match pool {
        Pool::Postgres(p) => postgres::server_version(p).await,
        Pool::Mysql(p) => mysql::server_version(p).await,
        Pool::Sqlite(p) => sqlite::server_version(p).await,
    }
}

pub async fn test_connection(connection: &ConnectionInput) -> Result<TestConnectionResponse, DbError> {
    let pool = match create_pool(connection) {
        Ok(pool) => pool,
        Err(err) => {
            return Ok(TestConnectionResponse {
                ok: false,
                message: err.to_string(),
                server_version: None,
            });
        }
    };

    match server_version(&pool).await {
        Ok(server_version) => Ok(TestConnectionResponse {
            ok: true,
            message: "Connection successful".to_string(),
            server_version,
        }),
        Err(err) => Ok(TestConnectionResponse {
            ok: false,
            message: err.to_string(),
            server_version: None,
        }),
    }
}
