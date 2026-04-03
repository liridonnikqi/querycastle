use async_trait::async_trait;
use mysql_async::{Row as MySqlRow, Value as MySqlValue};
use mysql_async::prelude::Queryable;
use serde_json::Value;
use std::collections::HashMap;

use crate::adapters::traits::{DbAdapter, MySqlAdapter};
use crate::core::types::{
    ApplyTableChangesParams, ApplyTableChangesResponse, ConnectionInput, ConnectionStatus,
    DatabaseExplorer, DatabaseType, QueryResultPayload, TestConnectionResponse, UpdatedRowCtid,
};

fn mysql_quote_ident(value: &str) -> String {
    format!("`{}`", value.replace('`', "``"))
}

fn mysql_value_literal(value: &Value) -> String {
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

fn mysql_row_hash_expression(columns: &[String]) -> String {
    if columns.is_empty() {
        return "md5('')".to_string();
    }
    let parts = columns
        .iter()
        .map(|column| {
            format!(
                "coalesce(cast({} as char), '__querycastle_null__')",
                mysql_quote_ident(column)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("md5(concat_ws(char(31), {parts}))")
}

fn mysql_value_to_json(value: &MySqlValue) -> Value {
    match value {
        MySqlValue::NULL => Value::Null,
        MySqlValue::Bytes(v) => Value::String(String::from_utf8_lossy(v).to_string()),
        MySqlValue::Int(v) => Value::Number((*v).into()),
        MySqlValue::UInt(v) => Value::Number((*v).into()),
        MySqlValue::Float(v) => {
            serde_json::Number::from_f64(*v as f64).map(Value::Number).unwrap_or(Value::Null)
        }
        MySqlValue::Double(v) => {
            serde_json::Number::from_f64(*v).map(Value::Number).unwrap_or(Value::Null)
        }
        MySqlValue::Date(y, m, d, hh, mm, ss, micros) => Value::String(format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:06}",
            y, m, d, hh, mm, ss, micros
        )),
        MySqlValue::Time(neg, days, hh, mm, ss, micros) => {
            let sign = if *neg { "-" } else { "" };
            Value::String(format!("{sign}{days} {:02}:{:02}:{:02}.{:06}", hh, mm, ss, micros))
        }
    }
}

fn mysql_hash_sql_from_row_values(columns: &[String], row: &HashMap<String, Value>) -> String {
    let parts = columns
        .iter()
        .map(|column| {
            let value = row.get(column).cloned().unwrap_or(Value::Null);
            format!(
                "coalesce(cast({} as char), '__querycastle_null__')",
                mysql_value_literal(&value)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("select md5(concat_ws(char(31), {parts}))")
}

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
        crate::core::db::get_mysql_database_explorer(connection).await
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
        connection: &ConnectionInput,
        params: &ApplyTableChangesParams,
    ) -> Result<ApplyTableChangesResponse, String> {
        let schema = params.schema.trim();
        let table = params.table.trim();
        if schema.is_empty() || table.is_empty() {
            return Err("Schema and table are required".to_string());
        }

        let mut conn = crate::core::db::connect_mysql_client(connection).await?;
        conn.query_drop("start transaction")
            .await
            .map_err(crate::core::db::sanitize_mysql_error)?;

        let safe_schema = mysql_quote_ident(schema);
        let safe_table = mysql_quote_ident(table);
        let safe_table_ref = format!("{safe_schema}.{safe_table}");

        let column_query = format!(
            "select column_name from information_schema.columns where table_schema = '{}' and table_name = '{}' order by ordinal_position",
            crate::core::sql::escape_sql_string(schema),
            crate::core::sql::escape_sql_string(table)
        );
        let columns: Vec<String> = conn
            .query(column_query)
            .await
            .map_err(crate::core::db::sanitize_mysql_error)?;
        if columns.is_empty() {
            let _ = conn.query_drop("rollback").await;
            return Err("Could not load table columns for MySQL table editing.".to_string());
        }
        let row_hash_expr = mysql_row_hash_expression(&columns);

        let mut updated = 0usize;
        let mut deleted = 0usize;
        let mut inserted = 0usize;
        let mut updated_rows: Vec<UpdatedRowCtid> = Vec::new();

        for update in &params.changes.updates {
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
                    format!("{} = {}", mysql_quote_ident(column), mysql_value_literal(value))
                })
                .collect::<Vec<_>>()
                .join(", ");

            let select_sql = format!(
                "select * from {safe_table_ref} where {row_hash_expr} = '{}' limit 1",
                crate::core::sql::escape_sql_string(update.ctid.as_str())
            );
            let current_row: Option<MySqlRow> = conn
                .query_first(select_sql)
                .await
                .map_err(crate::core::db::sanitize_mysql_error)?;
            let Some(current_row) = current_row else {
                let _ = conn.query_drop("rollback").await;
                return Err(format!(
                    "Could not update row {}. It may have changed. Refresh and retry.",
                    update.ctid
                ));
            };

            let mut merged_values: HashMap<String, Value> = HashMap::new();
            for (index, column_name) in columns.iter().enumerate() {
                let value = current_row
                    .as_ref(index)
                    .map(mysql_value_to_json)
                    .unwrap_or(Value::Null);
                merged_values.insert(column_name.clone(), value);
            }
            for (column, value) in &update.values {
                if column == "_querycastle_ctid" {
                    continue;
                }
                merged_values.insert(column.clone(), value.clone());
            }

            let new_hash_sql = mysql_hash_sql_from_row_values(&columns, &merged_values);
            let new_ctid: Option<String> = conn
                .query_first(new_hash_sql)
                .await
                .map_err(crate::core::db::sanitize_mysql_error)?;
            let Some(new_ctid) = new_ctid else {
                let _ = conn.query_drop("rollback").await;
                return Err("Could not compute updated MySQL row identity.".to_string());
            };

            let update_sql = format!(
                "update {safe_table_ref} set {set_clause} where {row_hash_expr} = '{}' limit 1",
                crate::core::sql::escape_sql_string(update.ctid.as_str())
            );
            let affected = conn
                .query_drop(update_sql)
                .await
                .map(|_| conn.affected_rows())
                .map_err(crate::core::db::sanitize_mysql_error)?;
            if affected == 0 {
                let _ = conn.query_drop("rollback").await;
                return Err(format!(
                    "Could not update row {}. It may have changed. Refresh and retry.",
                    update.ctid
                ));
            }
            updated_rows.push(UpdatedRowCtid {
                old_ctid: update.ctid.clone(),
                new_ctid,
                values: merged_values,
            });
            updated += 1;
        }

        for ctid in &params.changes.deletes {
            let delete_sql = format!(
                "delete from {safe_table_ref} where {row_hash_expr} = '{}' limit 1",
                crate::core::sql::escape_sql_string(ctid)
            );
            let affected = conn
                .query_drop(delete_sql)
                .await
                .map(|_| conn.affected_rows())
                .map_err(crate::core::db::sanitize_mysql_error)?;
            if affected == 0 {
                let _ = conn.query_drop("rollback").await;
                return Err(format!(
                    "Could not delete row {}. It may have changed. Refresh and retry.",
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
                .map(|(column, _)| mysql_quote_ident(column))
                .collect::<Vec<_>>()
                .join(", ");
            let values = entries
                .iter()
                .map(|(_, value)| mysql_value_literal(value))
                .collect::<Vec<_>>()
                .join(", ");

            let insert_sql = format!("insert into {safe_table_ref} ({cols}) values ({values})");
            conn.query_drop(insert_sql)
                .await
                .map_err(crate::core::db::sanitize_mysql_error)?;
            inserted += 1;
        }

        conn.query_drop("commit")
            .await
            .map_err(crate::core::db::sanitize_mysql_error)?;

        Ok(ApplyTableChangesResponse {
            ok: true,
            updated,
            deleted,
            inserted,
            updated_rows,
        })
    }
}
