use async_trait::async_trait;
use rusqlite::types::ValueRef;
use serde_json::Value;
use std::collections::HashMap;

use crate::adapters::traits::{DbAdapter, SqliteAdapter};
use crate::core::types::{
    ApplyTableChangesParams, ApplyTableChangesResponse, ConnectionInput, ConnectionStatus,
    DatabaseExplorer, DatabaseType, QueryResultPayload, TestConnectionResponse, UpdatedRowCtid,
};

fn sqlite_value_to_json(value: ValueRef<'_>) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(v) => Value::from(v),
        ValueRef::Real(v) => Value::from(v),
        ValueRef::Text(v) => Value::from(String::from_utf8_lossy(v).to_string()),
        ValueRef::Blob(v) => {
            let mut out = String::with_capacity(v.len() * 2 + 2);
            out.push_str("0x");
            for byte in v {
                out.push_str(format!("{byte:02x}").as_str());
            }
            Value::from(out)
        }
    }
}

fn sqlite_literal(value: &Value) -> String {
    match value {
        Value::Null => "NULL".to_string(),
        Value::Bool(v) => {
            if *v {
                "1".to_string()
            } else {
                "0".to_string()
            }
        }
        Value::Number(v) => v.to_string(),
        Value::String(v) => format!("'{}'", crate::core::sql::escape_sql_string(v)),
        Value::Array(_) | Value::Object(_) => {
            format!("'{}'", crate::core::sql::escape_sql_string(&value.to_string()))
        }
    }
}

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
        connection: &ConnectionInput,
        params: &ApplyTableChangesParams,
    ) -> Result<ApplyTableChangesResponse, String> {
        let schema = params.schema.trim();
        let table = params.table.trim();
        if schema.is_empty() || table.is_empty() {
            return Err("Schema and table are required".to_string());
        }

        let mut conn = crate::core::db::open_sqlite_connection(connection)?;
        let tx = conn
            .transaction()
            .map_err(|error| format!("SQLite transaction start failed: {error}"))?;

        let safe_table = format!(
            "{}.{}",
            crate::core::sql::quote_ident(schema),
            crate::core::sql::quote_ident(table)
        );

        let mut updated = 0usize;
        let mut deleted = 0usize;
        let mut inserted = 0usize;
        let mut updated_rows: Vec<UpdatedRowCtid> = Vec::new();

        for update in &params.changes.updates {
            let rowid = update
                .ctid
                .trim()
                .parse::<i64>()
                .map_err(|_| format!("Invalid SQLite row id: {}", update.ctid))?;

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
                .map_err(|error| format!("SQLite update failed: {error}"))?;
            if affected == 0 {
                return Err(format!(
                    "Could not update row with id {}. It may have changed. Refresh and retry.",
                    update.ctid
                ));
            }

            let select_sql = format!("select * from {safe_table} where rowid = {rowid} limit 1");
            let mut stmt = tx
                .prepare(select_sql.as_str())
                .map_err(|error| format!("SQLite row refresh prepare failed: {error}"))?;
            let row_values = stmt
                .query_row([], |row| {
                    let mut mapped = HashMap::new();
                    for (index, column_name) in row.as_ref().column_names().iter().enumerate() {
                        let value = row.get_ref(index).map(sqlite_value_to_json)?;
                        mapped.insert(column_name.to_string(), value);
                    }
                    Ok::<HashMap<String, Value>, rusqlite::Error>(mapped)
                })
                .map_err(|error| format!("SQLite row refresh failed: {error}"))?;

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
                .map_err(|_| format!("Invalid SQLite row id: {ctid}"))?;
            let delete_sql = format!("delete from {safe_table} where rowid = {rowid}");
            let affected = tx
                .execute(delete_sql.as_str(), [])
                .map_err(|error| format!("SQLite delete failed: {error}"))?;
            if affected == 0 {
                return Err(format!(
                    "Could not delete row with id {}. It may have changed. Refresh and retry.",
                    ctid
                ));
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
                .map_err(|error| format!("SQLite insert failed: {error}"))?;
            inserted += 1;
        }

        tx.commit()
            .map_err(|error| format!("SQLite transaction commit failed: {error}"))?;

        Ok(ApplyTableChangesResponse {
            ok: true,
            updated,
            deleted,
            inserted,
            updated_rows,
        })
    }
}
