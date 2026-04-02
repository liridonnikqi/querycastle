use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use crate::adapters::traits::{DbAdapter, PostgresAdapter};
use crate::core::types::{
    ApplyTableChangesParams, ApplyTableChangesResponse, ConnectionInput, ConnectionStatus,
    DatabaseColumn, DatabaseExplorer, DatabaseForeignKey, DatabaseSchema, DatabaseTable,
    DatabaseType, QueryResultPayload, TestConnectionResponse, UpdatedRowCtid,
};

#[async_trait]
impl DbAdapter for PostgresAdapter {
    async fn test_connection(&self, connection: &ConnectionInput) -> Result<TestConnectionResponse, String> {
        let client = match crate::core::db::connect_client(connection).await {
            Ok(client) => client,
            Err(error) => {
                return Ok(TestConnectionResponse {
                    ok: false,
                    message: error,
                    server_version: None,
                })
            }
        };

        let server_version = match crate::core::db::get_server_version(&client).await {
            Ok(version) => version,
            Err(error) => {
                return Ok(TestConnectionResponse {
                    ok: false,
                    message: error,
                    server_version: None,
                })
            }
        };

        Ok(TestConnectionResponse {
            ok: true,
            message: "Connection successful".to_string(),
            server_version,
        })
    }

    async fn connect(&self, connection: &ConnectionInput) -> Result<ConnectionStatus, String> {
        let client = crate::core::db::connect_client(connection).await?;
        let server_version = crate::core::db::get_server_version(&client).await?;
        client
            .batch_execute(&format!("set statement_timeout = {}", crate::core::db::QUERY_TIMEOUT_MS))
            .await
            .map_err(crate::core::db::sanitize_pg_error)?;

        Ok(ConnectionStatus {
            connected: true,
            database_type: DatabaseType::Postgres,
            name: connection.name.clone(),
            host: connection.host.clone(),
            port: connection.port,
            database: connection.database.clone(),
            user: connection.user.clone(),
            server_version,
        })
    }

    async fn run_query(&self, connection: &ConnectionInput, sql: &str) -> Result<QueryResultPayload, String> {
        let client = crate::core::db::connect_client(connection).await?;
        client
            .batch_execute(&format!("set statement_timeout = {}", crate::core::db::QUERY_TIMEOUT_MS))
            .await
            .map_err(crate::core::db::sanitize_pg_error)?;

        let started = std::time::Instant::now();
        let messages = client.simple_query(sql).await.map_err(crate::core::db::sanitize_pg_error)?;

        let mut columns: Vec<String> = Vec::new();
        let mut rows: Vec<HashMap<String, Value>> = Vec::new();
        for message in messages {
            if let tokio_postgres::SimpleQueryMessage::Row(row) = message {
                if columns.is_empty() {
                    columns = row
                        .columns()
                        .iter()
                        .map(|column| column.name().to_string())
                        .collect();
                }

                let mut mapped = HashMap::new();
                for (index, column_name) in columns.iter().enumerate() {
                    let value = row
                        .get(index)
                        .map(|entry| Value::String(entry.to_string()))
                        .unwrap_or(Value::Null);
                    mapped.insert(column_name.clone(), value);
                }
                rows.push(mapped);
            }
        }

        let row_count = rows.len();
        let limited_rows = if row_count > crate::core::db::MAX_QUERY_ROWS {
            rows.into_iter().take(crate::core::db::MAX_QUERY_ROWS).collect()
        } else {
            rows
        };

        Ok(QueryResultPayload {
            columns,
            rows: limited_rows,
            row_count,
            duration_ms: started.elapsed().as_millis(),
        })
    }

    async fn get_database_explorer(&self, connection: &ConnectionInput) -> Result<DatabaseExplorer, String> {
        let client = crate::core::db::connect_client(connection).await?;

        let db_row = client
            .query_one("select current_database() as current_database", &[])
            .await
            .map_err(crate::core::db::sanitize_pg_error)?;
        let current_database: String =
            db_row.try_get("current_database").map_err(crate::core::db::sanitize_pg_error)?;

        let rows = client
            .query(
                "
            select
                n.nspname as schema_name,
                c.relname as table_name,
                c.relkind::text as relkind,
                a.attname as column_name,
                pg_catalog.format_type(a.atttypid, a.atttypmod) as data_type,
                a.attnotnull as not_null
            from pg_catalog.pg_class c
            join pg_catalog.pg_namespace n on n.oid = c.relnamespace
            left join pg_catalog.pg_attribute a
                on a.attrelid = c.oid
                and a.attnum > 0
                and not a.attisdropped
            where c.relkind in ('r', 'p', 'v', 'm', 'f')
                and n.nspname not in ('pg_catalog', 'information_schema')
            order by n.nspname, c.relname, a.attnum
            ",
                &[],
            )
            .await
            .map_err(crate::core::db::sanitize_pg_error)?;

        let mut schema_map: HashMap<String, DatabaseSchema> = HashMap::new();
        let mut table_map: HashMap<String, DatabaseTable> = HashMap::new();

        for row in rows {
            let schema_name: String = row.try_get("schema_name").map_err(crate::core::db::sanitize_pg_error)?;
            let table_name: String = row.try_get("table_name").map_err(crate::core::db::sanitize_pg_error)?;
            let relkind: String = row.try_get("relkind").map_err(crate::core::db::sanitize_pg_error)?;
            let column_name: Option<String> = row.try_get("column_name").map_err(crate::core::db::sanitize_pg_error)?;
            let data_type: Option<String> = row.try_get("data_type").map_err(crate::core::db::sanitize_pg_error)?;
            let not_null: Option<bool> = row.try_get("not_null").map_err(crate::core::db::sanitize_pg_error)?;

            schema_map
                .entry(schema_name.clone())
                .or_insert_with(|| DatabaseSchema {
                    name: schema_name.clone(),
                    tables: Vec::new(),
                });

            let table_key = format!("{schema_name}.{table_name}");
            table_map
                .entry(table_key.clone())
                .or_insert_with(|| DatabaseTable {
                    schema: schema_name.clone(),
                    name: table_name.clone(),
                    kind: if relkind == "v" || relkind == "m" {
                        "view".to_string()
                    } else {
                        "table".to_string()
                    },
                    columns: Vec::new(),
                    foreign_keys: Vec::new(),
                });

            if let Some(column) = column_name {
                if let Some(table) = table_map.get_mut(&table_key) {
                    table.columns.push(DatabaseColumn {
                        name: column,
                        data_type: data_type.unwrap_or_else(|| "unknown".to_string()),
                        not_null: not_null.unwrap_or(false),
                    });
                }
            }
        }

        let fk_rows = client
            .query(
                "
            select
                tc.table_schema,
                tc.table_name,
                kcu.column_name,
                ccu.table_schema as foreign_table_schema,
                ccu.table_name as foreign_table_name,
                ccu.column_name as foreign_column_name
            from information_schema.table_constraints tc
            join information_schema.key_column_usage kcu
                on tc.constraint_name = kcu.constraint_name
                and tc.table_schema = kcu.table_schema
            join information_schema.constraint_column_usage ccu
                on tc.constraint_name = ccu.constraint_name
                and tc.table_schema = ccu.table_schema
            where tc.constraint_type = 'FOREIGN KEY'
                and tc.table_schema not in ('pg_catalog', 'information_schema')
            order by tc.table_schema, tc.table_name, kcu.ordinal_position
            ",
                &[],
            )
            .await
            .map_err(crate::core::db::sanitize_pg_error)?;

        for fk in fk_rows {
            let table_schema: String = fk.try_get("table_schema").map_err(crate::core::db::sanitize_pg_error)?;
            let table_name: String = fk.try_get("table_name").map_err(crate::core::db::sanitize_pg_error)?;
            let table_key = format!("{table_schema}.{table_name}");
            if let Some(table) = table_map.get_mut(&table_key) {
                table.foreign_keys.push(DatabaseForeignKey {
                    column: fk.try_get("column_name").map_err(crate::core::db::sanitize_pg_error)?,
                    referenced_schema: fk.try_get("foreign_table_schema").map_err(crate::core::db::sanitize_pg_error)?,
                    referenced_table: fk.try_get("foreign_table_name").map_err(crate::core::db::sanitize_pg_error)?,
                    referenced_column: fk.try_get("foreign_column_name").map_err(crate::core::db::sanitize_pg_error)?,
                });
            }
        }

        for table in table_map.into_values() {
            if let Some(schema) = schema_map.get_mut(&table.schema) {
                schema.tables.push(table);
            }
        }

        let mut schemas: Vec<DatabaseSchema> = schema_map.into_values().collect();
        schemas.sort_by(|a, b| a.name.cmp(&b.name));
        for schema in &mut schemas {
            schema.tables.sort_by(|a, b| a.name.cmp(&b.name));
        }

        Ok(DatabaseExplorer {
            database: current_database,
            schemas,
        })
    }

    async fn list_databases(&self, connection: &ConnectionInput) -> Result<Vec<String>, String> {
        let current_database = connection.database.clone();
        let client = crate::core::db::connect_client(connection).await?;

        let rows = client
            .query(
                "
            select datname
            from pg_database
            where datallowconn = true
                and datistemplate = false
            order by datname
            ",
                &[],
            )
            .await;

        match rows {
            Ok(rows) => {
                let mut names = Vec::new();
                for row in rows {
                    let name: String = row.try_get("datname").map_err(crate::core::db::sanitize_pg_error)?;
                    names.push(name);
                }
                if names.is_empty() {
                    Ok(vec![current_database])
                } else {
                    Ok(names)
                }
            }
            Err(_) => Ok(vec![current_database]),
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

        let mut client = crate::core::db::connect_client(connection).await?;
        client
            .batch_execute(&format!("set statement_timeout = {}", crate::core::db::QUERY_TIMEOUT_MS))
            .await
            .map_err(crate::core::db::sanitize_pg_error)?;

        let mut updated = 0usize;
        let mut deleted = 0usize;
        let mut inserted = 0usize;
        let mut updated_rows: Vec<UpdatedRowCtid> = Vec::new();

        let tx = client.transaction().await.map_err(crate::core::db::sanitize_pg_error)?;
        let safe_table = format!("{}.{}", crate::core::db::quote_ident(schema), crate::core::db::quote_ident(table));

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
                .map(|(column, value)| format!("{} = {}", crate::core::db::quote_ident(column), crate::core::db::value_to_sql_literal(value)))
                .collect::<Vec<_>>()
                .join(", ");
            let query = format!(
                "update {safe_table} as t set {set_clause} where t.ctid = '{}'::tid returning t.ctid::text as _querycastle_ctid, to_jsonb(t)::text as _querycastle_row_json",
                crate::core::db::escape_sql_string(update.ctid.as_str())
            );
            let updated_row = tx.query_opt(query.as_str(), &[]).await.map_err(crate::core::db::sanitize_pg_error)?;
            let Some(updated_row) = updated_row else {
                return Err(format!(
                    "Could not update row with ctid {}. It may have changed. Refresh and retry.",
                    update.ctid
                ));
            };
            let new_ctid: String = updated_row.try_get("_querycastle_ctid").map_err(crate::core::db::sanitize_pg_error)?;
            let row_json: String = updated_row.try_get("_querycastle_row_json").map_err(crate::core::db::sanitize_pg_error)?;
            let values: HashMap<String, Value> = serde_json::from_str(&row_json).map_err(crate::core::db::sanitize_error)?;
            updated_rows.push(UpdatedRowCtid {
                old_ctid: update.ctid.clone(),
                new_ctid,
                values,
            });
            updated += 1;
        }

        for ctid in &params.changes.deletes {
            let query = format!(
                "delete from {safe_table} where ctid = '{}'::tid",
                crate::core::db::escape_sql_string(ctid)
            );
            let affected = tx.execute(query.as_str(), &[]).await.map_err(crate::core::db::sanitize_pg_error)?;
            if affected == 0 {
                return Err(format!(
                    "Could not delete row with ctid {}. It may have changed. Refresh and retry.",
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
                .map(|(column, _)| crate::core::db::quote_ident(column))
                .collect::<Vec<_>>()
                .join(", ");
            let values = entries
                .iter()
                .map(|(_, value)| crate::core::db::value_to_sql_literal(value))
                .collect::<Vec<_>>()
                .join(", ");

            let query = format!("insert into {safe_table} ({cols}) values ({values})");
            tx.execute(query.as_str(), &[]).await.map_err(crate::core::db::sanitize_pg_error)?;
            inserted += 1;
        }

        tx.commit().await.map_err(crate::core::db::sanitize_pg_error)?;

        Ok(ApplyTableChangesResponse {
            ok: true,
            updated,
            deleted,
            inserted,
            updated_rows,
        })
    }
}
