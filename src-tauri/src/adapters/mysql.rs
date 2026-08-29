use async_trait::async_trait;
use mysql_async::{Row as MySqlRow, Value as MySqlValue};
use mysql_async::prelude::Queryable;
use serde_json::Value;
use std::collections::HashMap;

use crate::adapters::traits::{DbAdapter, MySqlAdapter};
use crate::core::error::{DbError, sanitize_mysql_error_to_db_error};
use crate::core::types::{
    ApplyTableChangesParams, ApplyTableChangesResponse, ConnectionInput, ConnectionStatus,
    DatabaseExplorer, DatabaseType, ObjectDefinition, ObjectDefinitionParams, QueryResultPayload,
    TestConnectionResponse, UpdatedRowCtid,
};

fn mysql_quote_ident(value: &str) -> String {
    crate::core::sql::quote_ident_for(crate::core::types::DatabaseType::Mysql, value)
}

fn mysql_value_literal(value: &Value) -> String {
    crate::core::sql::value_to_sql_literal_for(crate::core::types::DatabaseType::Mysql, value)
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
    crate::core::db::mysql_value_to_json(value)
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
    async fn test_connection(&self, connection: &ConnectionInput) -> Result<TestConnectionResponse, DbError> {
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
        let server_version = crate::core::db::get_mysql_server_version(&mut conn).await.map_err(|e| DbError::internal(e))?;
        Ok(TestConnectionResponse {
            ok: true,
            message: "Connection successful".to_string(),
            server_version,
        })
    }

    async fn connect(&self, connection: &ConnectionInput) -> Result<ConnectionStatus, DbError> {
        let mut conn = crate::core::db::connect_mysql_client(connection).await.map_err(DbError::internal)?;
        let server_version = crate::core::db::get_mysql_server_version(&mut conn).await.map_err(|e| DbError::internal(e))?;
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

    async fn run_query(&self, connection: &ConnectionInput, sql: &str) -> Result<QueryResultPayload, DbError> {
        crate::core::db::run_mysql_query(connection, sql).await.map_err(DbError::internal)
    }

    async fn get_database_explorer(&self, connection: &ConnectionInput) -> Result<DatabaseExplorer, DbError> {
        crate::core::db::get_mysql_database_explorer(connection).await.map_err(DbError::internal)
    }

    async fn get_object_definition(
        &self,
        connection: &ConnectionInput,
        params: &ObjectDefinitionParams,
    ) -> Result<ObjectDefinition, DbError> {
        crate::core::db::get_mysql_object_definition(connection, params)
            .await
            .map_err(DbError::internal)
    }

    async fn list_databases(&self, connection: &ConnectionInput) -> Result<Vec<String>, DbError> {
        let mut conn = crate::core::db::connect_mysql_client(connection).await.map_err(DbError::internal)?;
        let dbs: Vec<String> = conn
            .query("show databases")
            .await
            .map_err(sanitize_mysql_error_to_db_error)?;
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
    ) -> Result<(ConnectionInput, ConnectionStatus), DbError> {
        let next_connection = crate::core::db::with_new_database(connection, database);
        let status = self.connect(&next_connection).await?;
        Ok((next_connection, status))
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

        let mut conn = crate::core::db::connect_mysql_client(connection).await.map_err(DbError::internal)?;
        conn.query_drop("start transaction")
            .await
            .map_err(sanitize_mysql_error_to_db_error)?;

        let safe_schema = mysql_quote_ident(schema);
        let safe_table = mysql_quote_ident(table);
        let safe_table_ref = format!("{safe_schema}.{safe_table}");

        let column_query = "select column_name from information_schema.columns where table_schema = ? and table_name = ? order by ordinal_position";
        let columns: Vec<String> = conn
            .exec(column_query, (schema, table))
            .await
            .map_err(sanitize_mysql_error_to_db_error)?;
        if columns.is_empty() {
            let _ = conn.query_drop("rollback").await;
            return Err(DbError::validation("Could not load table columns for MySQL table editing."));
        }
        // Prefer primary key columns for identity if available (reduces hash collision & full-scan)
        let pk_sql = "select column_name from information_schema.columns where table_schema = ? and table_name = ? and column_key = 'PRI' order by ordinal_position";
        let pk_columns: Vec<String> = conn
            .exec(pk_sql, (schema, table))
            .await
            .map_err(sanitize_mysql_error_to_db_error)
            .unwrap_or_default();
        let hash_columns = if pk_columns.is_empty() { columns.clone() } else { pk_columns.clone() };
        let row_hash_expr = mysql_row_hash_expression(&hash_columns);

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
                "select * from {safe_table_ref} where {row_hash_expr} = ? limit 1"
            );
            let current_row: Option<MySqlRow> = conn
                .exec_first(select_sql, (update.ctid.clone(),))
                .await
                .map_err(sanitize_mysql_error_to_db_error)?;
            let Some(current_row) = current_row else {
                let _ = conn.query_drop("rollback").await;
                return Err(DbError::NotFound(format!(
                    "Could not update row {}. It may have changed. Refresh and retry.",
                    update.ctid
                )));
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

            let new_hash_sql = mysql_hash_sql_from_row_values(&hash_columns, &merged_values);
            let new_ctid: Option<String> = conn
                .query_first(new_hash_sql)
                .await
                .map_err(sanitize_mysql_error_to_db_error)?;
            let Some(new_ctid) = new_ctid else {
                let _ = conn.query_drop("rollback").await;
                return Err(DbError::internal("Could not compute updated MySQL row identity."));
            };

            let update_sql = format!(
                "update {safe_table_ref} set {set_clause} where {row_hash_expr} = ? limit 1"
            );
            let affected = conn
                .exec_drop(update_sql, (update.ctid.clone(),))
                .await
                .map(|_| conn.affected_rows())
                .map_err(sanitize_mysql_error_to_db_error)?;
            if affected == 0 {
                let _ = conn.query_drop("rollback").await;
                return Err(DbError::NotFound(format!(
                    "Could not update row {}. It may have changed. Refresh and retry.",
                    update.ctid
                )));
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
                "delete from {safe_table_ref} where {row_hash_expr} = ? limit 1"
            );
            let affected = conn
                .exec_drop(delete_sql, (ctid.clone(),))
                .await
                .map(|_| conn.affected_rows())
                .map_err(sanitize_mysql_error_to_db_error)?;
            if affected == 0 {
                let _ = conn.query_drop("rollback").await;
                return Err(DbError::NotFound(format!(
                    "Could not delete row {}. It may have changed. Refresh and retry.",
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
                .map_err(sanitize_mysql_error_to_db_error)?;
            inserted += 1;
        }

        conn.query_drop("commit")
            .await
            .map_err(sanitize_mysql_error_to_db_error)?;

        Ok(ApplyTableChangesResponse {
            ok: true,
            updated,
            deleted,
            inserted,
            updated_rows,
        })
    }
}
