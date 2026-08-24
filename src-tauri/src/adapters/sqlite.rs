use async_trait::async_trait;
use rusqlite::types::ValueRef;
use serde_json::Value;
use std::collections::HashMap;

use crate::adapters::traits::{DbAdapter, SqliteAdapter};
use crate::core::error::DbError;
use crate::core::types::{
    ApplyTableChangesParams, ApplyTableChangesResponse, ConnectionInput, ConnectionStatus,
    DatabaseExplorer, DatabaseType, QueryResultPayload, TestConnectionResponse, UpdatedRowCtid,
};

fn sqlite_value_to_json(value: ValueRef<'_>) -> Value {
    crate::core::db::sqlite_value_to_json(value)
}

fn sqlite_literal(value: &Value) -> String {
    crate::core::sql::value_to_sql_literal_for(crate::core::types::DatabaseType::Sqlite, value)
}

#[async_trait]
impl DbAdapter for SqliteAdapter {
    async fn test_connection(&self, connection: &ConnectionInput) -> Result<TestConnectionResponse, DbError> {
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
        let server_version = crate::core::db::get_sqlite_server_version(&conn).map_err(DbError::internal)?;
        Ok(TestConnectionResponse {
            ok: true,
            message: "Connection successful".to_string(),
            server_version,
        })
    }

    async fn connect(&self, connection: &ConnectionInput) -> Result<ConnectionStatus, DbError> {
        let conn = crate::core::db::open_sqlite_connection(connection).map_err(DbError::internal)?;
        let server_version = crate::core::db::get_sqlite_server_version(&conn).map_err(DbError::internal)?;
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

    async fn run_query(&self, connection: &ConnectionInput, sql: &str) -> Result<QueryResultPayload, DbError> {
        crate::core::db::run_sqlite_query(connection, sql).map_err(DbError::internal)
    }

    async fn get_database_explorer(&self, connection: &ConnectionInput) -> Result<DatabaseExplorer, DbError> {
        crate::core::db::get_sqlite_database_explorer(connection).map_err(DbError::internal)
    }

    async fn list_databases(&self, connection: &ConnectionInput) -> Result<Vec<String>, DbError> {
        Ok(crate::core::db::list_sqlite_databases(connection))
    }

    async fn select_database(
        &self,
        connection: &ConnectionInput,
        database: &str,
    ) -> Result<(ConnectionInput, ConnectionStatus), DbError> {
        let next_connection = crate::core::db::with_new_database(connection, database);
        // For SQLite, ensure file-specific fields are cleared (host/port/etc not used)
        let mut next = next_connection;
        if next.database_type == crate::core::types::DatabaseType::Sqlite {
            next.host = String::new();
            next.port = 0;
            next.user = String::new();
            next.ssl = false;
        }
        let status = self.connect(&next).await?;
        Ok((next, status))
    }

    async fn apply_table_changes(
        &self,
        connection: &ConnectionInput,
        params: &ApplyTableChangesParams,
    ) -> Result<ApplyTableChangesResponse, DbError> {
        let schema = params.schema.trim();
        let table = params.table.trim();
        if schema.is_empty() || table.is_empty() {
            return Err(DbError::validation("Schema and table are required"));
        }

        let mut conn = crate::core::db::open_sqlite_connection(connection).map_err(DbError::internal)?;
        let tx = conn
            .transaction()
            .map_err(|e| DbError::internal(format!("SQLite transaction start failed: {e}")))?;

        let safe_table = format!(
            "{}.{}",
            crate::core::sql::quote_ident(schema),
            crate::core::sql::quote_ident(table)
        );

        // Detect WITHOUT ROWID and primary key for better error messages
        let is_without_rowid = {
            let stmt = tx
                .prepare("SELECT sql FROM sqlite_master WHERE type='table' AND name = ?1")
                .ok();
            if let Some(mut stmt) = stmt {
                stmt.query_row([table], |row| row.get::<_, Option<String>>(0))
                    .ok()
                    .flatten()
                    .map(|s| s.to_lowercase().contains("without rowid"))
                    .unwrap_or(false)
            } else {
                false
            }
        };

        let mut updated = 0usize;
        let mut deleted = 0usize;
        let mut inserted = 0usize;
        let mut updated_rows: Vec<UpdatedRowCtid> = Vec::new();

        for update in &params.changes.updates {
            let rowid = update
                .ctid
                .trim()
                .parse::<i64>()
                .map_err(|_| {
                    if is_without_rowid {
                        DbError::validation(format!(
                            "Table '{}' uses WITHOUT ROWID and requires primary-key editing; rowid {} is invalid. Use SQL directly.",
                            table, update.ctid
                        ))
                    } else {
                        DbError::validation(format!("Invalid SQLite row id: {}", update.ctid))
                    }
                })?;

            let entries: Vec<_> = update
                .values
                .iter()
                .filter(|(key, _)| key.as_str() != "_querycastle_ctid")
                .collect();
            if entries.is_empty() {
                continue;
            }

            let set_clause = entries
                .iter()
                .map(|(column, value)| {
                    format!(
                        "{} = {}",
                        crate::core::sql::quote_ident(column),
                        sqlite_literal(value)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            let update_sql = format!("update {safe_table} set {set_clause} where rowid = {rowid}");
            let affected = tx
                .execute(update_sql.as_str(), [])
                .map_err(|e| DbError::internal(format!("SQLite update failed: {e}")))?;
            if affected == 0 {
                return Err(DbError::NotFound(format!(
                    "Could not update row with id {}. It may have changed. Refresh and retry.",
                    update.ctid
                )));
            }

            let select_sql = format!("select * from {safe_table} where rowid = {rowid} limit 1");
            let mut stmt = tx
                .prepare(select_sql.as_str())
                .map_err(|e| DbError::internal(format!("SQLite row refresh prepare failed: {e}")))?;
            let row_values = stmt
                .query_row([], |row| {
                    let mut mapped = HashMap::new();
                    for (index, column_name) in row.as_ref().column_names().iter().enumerate() {
                        let value = row.get_ref(index).map(sqlite_value_to_json)?;
                        mapped.insert(column_name.to_string(), value);
                    }
                    Ok::<HashMap<String, Value>, rusqlite::Error>(mapped)
                })
                .map_err(|e| DbError::internal(format!("SQLite row refresh failed: {e}")))?;

            updated_rows.push(UpdatedRowCtid {
                old_ctid: update.ctid.clone(),
                new_ctid: rowid.to_string(),
                values: row_values,
            });
            updated += 1;
        }

        for ctid in &params.changes.deletes {
            let rowid = ctid
                .trim()
                .parse::<i64>()
                .map_err(|_| {
                    if is_without_rowid {
                        DbError::validation(format!(
                            "Table '{}' uses WITHOUT ROWID and requires primary-key editing; rowid {} is invalid. Use SQL directly.",
                            table, ctid
                        ))
                    } else {
                        DbError::validation(format!("Invalid SQLite row id: {ctid}"))
                    }
                })?;
            let delete_sql = format!("delete from {safe_table} where rowid = {rowid}");
            let affected = tx
                .execute(delete_sql.as_str(), [])
                .map_err(|e| DbError::internal(format!("SQLite delete failed: {e}")))?;
            if affected == 0 {
                return Err(DbError::NotFound(format!(
                    "Could not delete row with id {}. It may have changed. Refresh and retry.",
                    ctid
                )));
            }
            deleted += 1;
        }

        for row in &params.changes.inserts {
            let entries: Vec<_> = row
                .iter()
                .filter(|(key, _)| key.as_str() != "_querycastle_ctid")
                .collect();
            if entries.is_empty() {
                continue;
            }

            let cols = entries
                .iter()
                .map(|(column, _)| crate::core::sql::quote_ident(column))
                .collect::<Vec<_>>()
                .join(", ");
            let values = entries
                .iter()
                .map(|(_, value)| sqlite_literal(value))
                .collect::<Vec<_>>()
                .join(", ");

            let insert_sql = format!("insert into {safe_table} ({cols}) values ({values})");
            tx.execute(insert_sql.as_str(), [])
                .map_err(|e| DbError::internal(format!("SQLite insert failed: {e}")))?;
            inserted += 1;
        }

        tx.commit()
            .map_err(|e| DbError::internal(format!("SQLite transaction commit failed: {e}")))?;

        Ok(ApplyTableChangesResponse {
            ok: true,
            updated,
            deleted,
            inserted,
            updated_rows,
        })
    }
}
