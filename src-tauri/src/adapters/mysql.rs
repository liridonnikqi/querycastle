use mysql_async::prelude::Queryable;
use mysql_async::{Row as MySqlRow, TxOpts, Value as MySqlValue};
use serde_json::Value;
use std::collections::HashMap;

use crate::core::error::DbError;
use crate::core::limits::{apply_select_row_cap, MAX_QUERY_ROWS, QUERY_TIMEOUT_MS};
use crate::core::sql;
use crate::core::types::{
    ApplyTableChangesParams, ApplyTableChangesResponse, DatabaseColumn, DatabaseExplorer,
    DatabaseForeignKey, DatabaseIndex, DatabaseRoutine, DatabaseSchema, DatabaseTable,
    DatabaseTrigger, ObjectDefinition, ObjectDefinitionParams, QueryResultPayload, UpdatedRow,
};

fn json_to_mysql_value(value: &Value) -> MySqlValue {
    match value {
        Value::Null => MySqlValue::NULL,
        Value::Bool(v) => MySqlValue::Int(if *v { 1 } else { 0 }),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                MySqlValue::Int(i)
            } else if let Some(u) = n.as_u64() {
                MySqlValue::UInt(u)
            } else if let Some(f) = n.as_f64() {
                MySqlValue::Double(f)
            } else {
                MySqlValue::Bytes(n.to_string().into_bytes())
            }
        }
        Value::String(s) => MySqlValue::Bytes(s.clone().into_bytes()),
        Value::Array(_) | Value::Object(_) => MySqlValue::Bytes(value.to_string().into_bytes()),
    }
}

fn mysql_row_hash_expression(columns: &[String]) -> String {
    let parts = columns
        .iter()
        .map(|column| {
            format!(
                "coalesce(cast({} as char), '__querycastle_null__')",
                sql::quote_ident_mysql(column)
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

pub async fn server_version(pool: &mysql_async::Pool) -> Result<Option<String>, DbError> {
    let mut conn = pool.get_conn().await?;
    Ok(conn.query_first("select version()").await?)
}

pub async fn run_query(pool: &mysql_async::Pool, sql: &str) -> Result<QueryResultPayload, DbError> {
    let mut conn = pool.get_conn().await?;
    if let Err(error) = conn
        .query_drop(format!("SET SESSION max_execution_time = {QUERY_TIMEOUT_MS}"))
        .await
    {
        tracing::warn!("Could not set MySQL max_execution_time: {error}");
    }

    let started = std::time::Instant::now();
    let sql = apply_select_row_cap(sql).into_owned();
    let fut = async {
        let mut result = conn.query_iter(sql).await?;

        let mut columns: Vec<String> = Vec::new();
        let mut mapped_rows: Vec<HashMap<String, Value>> = Vec::new();
        let mut truncated = false;

        while let Some(row) = result.next().await? {
            if columns.is_empty() {
                columns = row
                    .columns_ref()
                    .iter()
                    .map(|column| column.name_str().to_string())
                    .collect();
            }
            if mapped_rows.len() >= MAX_QUERY_ROWS {
                truncated = true;
                break;
            }
            let mut mapped = HashMap::new();
            for (index, column_name) in columns.iter().enumerate() {
                let value = row.as_ref(index).map(mysql_value_to_json).unwrap_or(Value::Null);
                mapped.insert(column_name.clone(), value);
            }
            mapped_rows.push(mapped);
        }

        let row_count = mapped_rows.len();
        Ok(QueryResultPayload {
            columns,
            rows: mapped_rows,
            row_count,
            duration_ms: started.elapsed().as_millis(),
            truncated,
        })
    };

    match tokio::time::timeout(std::time::Duration::from_millis(QUERY_TIMEOUT_MS), fut).await {
        Ok(result) => result,
        Err(_) => Err(DbError::Timeout {
            message: format!("Query exceeded {QUERY_TIMEOUT_MS}ms"),
        }),
    }
}

pub async fn get_database_explorer(pool: &mysql_async::Pool) -> Result<DatabaseExplorer, DbError> {
    let mut conn = pool.get_conn().await?;
    let current_database: String = conn
        .query_first("select database()")
        .await
        ?
        .unwrap_or_default();

    let table_rows: Vec<(
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    )> = conn
        .query(
            r#"
            select
                t.table_schema as schema_name,
                t.table_name as table_name,
                t.table_type as table_type,
                c.column_name as column_name,
                c.column_type as data_type,
                c.is_nullable as is_nullable,
                c.column_key as column_key,
                c.column_default as column_default,
                c.extra as extra
            from information_schema.tables t
            left join information_schema.columns c
                on c.table_schema = t.table_schema
                and c.table_name = t.table_name
            where t.table_schema = database()
                and t.table_type in ('BASE TABLE', 'VIEW')
            order by t.table_name, c.ordinal_position
            "#,
        )
        .await
        ?;

    let mut schema_map: HashMap<String, DatabaseSchema> = HashMap::new();
    let mut table_map: HashMap<String, DatabaseTable> = HashMap::new();

    for (schema_name, table_name, table_type, column_name, data_type, is_nullable, column_key, column_default, extra) in table_rows {
        schema_map
            .entry(schema_name.clone())
            .or_insert_with(|| DatabaseSchema::new(schema_name.clone()));

        let table_key = format!("{schema_name}.{table_name}");
        table_map.entry(table_key.clone()).or_insert_with(|| {
            DatabaseTable::new(
                schema_name.clone(),
                table_name.clone(),
                if table_type == "VIEW" {
                    "view".to_string()
                } else {
                    "table".to_string()
                },
            )
        });

        if let Some(column_name) = column_name {
            if let Some(table) = table_map.get_mut(&table_key) {
                let has_default = column_default
                    .map(|value| !value.is_empty())
                    .unwrap_or(false)
                    || extra
                        .map(|value| {
                            let lower = value.to_ascii_lowercase();
                            lower.contains("auto_increment")
                                || lower.contains("default_generated")
                        })
                        .unwrap_or(false);
                table.columns.push(DatabaseColumn {
                    name: column_name,
                    data_type: data_type.unwrap_or_else(|| "unknown".to_string()),
                    not_null: is_nullable
                        .map(|value| value.eq_ignore_ascii_case("NO"))
                        .unwrap_or(false),
                    is_primary: column_key
                        .map(|value| value.eq_ignore_ascii_case("PRI"))
                        .unwrap_or(false),
                    has_default,
                });
            }
        }
    }

    let fk_rows: Vec<(String, String, String, String, String, String)> = conn
        .query(
            r#"
            select
                k.table_schema as table_schema,
                k.table_name as table_name,
                k.column_name as column_name,
                k.referenced_table_schema as referenced_schema,
                k.referenced_table_name as referenced_table,
                k.referenced_column_name as referenced_column
            from information_schema.key_column_usage k
            where k.table_schema = database()
                and k.referenced_table_name is not null
            order by k.table_name, k.ordinal_position
            "#,
        )
        .await
        ?;

    for (schema, table, column, ref_schema, ref_table, ref_column) in fk_rows {
        let table_key = format!("{schema}.{table}");
        if let Some(table) = table_map.get_mut(&table_key) {
            table.foreign_keys.push(DatabaseForeignKey {
                column,
                referenced_schema: ref_schema,
                referenced_table: ref_table,
                referenced_column: ref_column,
            });
        }
    }

    load_mysql_indexes(&mut conn, &mut table_map).await?;
    load_mysql_triggers(&mut conn, &mut table_map).await?;
    load_mysql_routines(&mut conn, &mut schema_map).await?;

    for table in table_map.into_values() {
        if let Some(schema) = schema_map.get_mut(&table.schema) {
            schema.tables.push(table);
        }
    }

    let mut schemas: Vec<DatabaseSchema> = schema_map.into_values().collect();
    schemas.sort_by(|a, b| a.name.cmp(&b.name));
    for schema in &mut schemas {
        schema.tables.sort_by(|a, b| a.name.cmp(&b.name));
        schema.routines.sort_by(|a, b| a.name.cmp(&b.name));
    }

    Ok(DatabaseExplorer {
        database: current_database,
        schemas,
    })
}

async fn load_mysql_indexes(
    conn: &mut mysql_async::Conn,
    table_map: &mut HashMap<String, DatabaseTable>,
) -> Result<(), DbError> {
    let rows: Vec<(String, String, String, i64, String)> = conn
        .query(
            r#"
            select
                table_schema,
                table_name,
                index_name,
                non_unique,
                column_name
            from information_schema.statistics
            where table_schema = database()
            order by table_name, index_name, seq_in_index
            "#,
        )
        .await
        ?;

    let mut grouped: HashMap<(String, String, String), (bool, bool, Vec<String>)> = HashMap::new();
    for (schema, table, index_name, non_unique, column_name) in rows {
        let key = (schema, table, index_name.clone());
        let entry = grouped.entry(key).or_insert((non_unique == 0, index_name.eq_ignore_ascii_case("PRIMARY"), Vec::new()));
        entry.2.push(column_name);
    }

    for ((schema, table, index_name), (unique, is_primary, columns)) in grouped {
        let table_key = format!("{schema}.{table}");
        if let Some(table) = table_map.get_mut(&table_key) {
            table.indexes.push(DatabaseIndex {
                name: index_name,
                columns: columns.join(", "),
                unique,
                is_primary,
                definition: None,
            });
        }
    }
    for table in table_map.values_mut() {
        table.indexes.sort_by(|a, b| a.name.cmp(&b.name));
    }
    Ok(())
}

async fn load_mysql_triggers(
    conn: &mut mysql_async::Conn,
    table_map: &mut HashMap<String, DatabaseTable>,
) -> Result<(), DbError> {
    let rows: Vec<(String, String, String, String, String, Option<String>)> = conn
        .query(
            r#"
            select
                event_object_schema,
                event_object_table,
                trigger_name,
                action_timing,
                event_manipulation,
                action_statement
            from information_schema.triggers
            where trigger_schema = database()
            order by event_object_table, trigger_name
            "#,
        )
        .await
        ?;

    let mut grouped: HashMap<(String, String, String), (Vec<String>, Option<String>)> = HashMap::new();
    for (schema, table, name, timing, event, statement) in rows {
        let key = (schema, table, name);
        let entry = grouped.entry(key).or_insert((Vec::new(), statement.clone()));
        let label = format!("{timing} {event}");
        if !entry.0.contains(&label) {
            entry.0.push(label);
        }
        if entry.1.is_none() {
            entry.1 = statement;
        }
    }

    for ((schema, table, name), (events, statement)) in grouped {
        let table_key = format!("{schema}.{table}");
        if let Some(table) = table_map.get_mut(&table_key) {
            let definition = statement.map(|body| {
                format!(
                    "-- {}\n{}",
                    events.join(", "),
                    body
                )
            });
            table.triggers.push(DatabaseTrigger { name, definition });
        }
    }
    Ok(())
}

async fn load_mysql_routines(
    conn: &mut mysql_async::Conn,
    schema_map: &mut HashMap<String, DatabaseSchema>,
) -> Result<(), DbError> {
    let rows: Vec<(String, String, String, Option<String>, Option<String>)> = conn
        .query(
            r#"
            select
                routine_schema,
                routine_name,
                routine_type,
                dtd_identifier,
                external_language
            from information_schema.routines
            where routine_schema = database()
            order by routine_type, routine_name
            "#,
        )
        .await
        ?;

    for (schema_name, name, routine_type, return_type, language) in rows {
        schema_map
            .entry(schema_name.clone())
            .or_insert_with(|| DatabaseSchema::new(schema_name.clone()));
        if let Some(schema) = schema_map.get_mut(&schema_name) {
            let kind = if routine_type.eq_ignore_ascii_case("PROCEDURE") {
                "procedure"
            } else {
                "function"
            };
            schema.routines.push(DatabaseRoutine {
                object_id: format!("{schema_name}.{name}.{kind}"),
                schema: schema_name,
                name,
                kind: kind.to_string(),
                identity_args: String::new(),
                language,
                return_type,
            });
        }
    }
    Ok(())
}

pub async fn get_object_definition(
    pool: &mysql_async::Pool,
    params: &ObjectDefinitionParams,
) -> Result<ObjectDefinition, DbError> {
    let mut conn = pool.get_conn().await?;
    let kind = params.kind.trim().to_ascii_lowercase();
    let name = params.name.trim();
    if name.is_empty() {
        return Err(DbError::validation("Name is required"));
    }
    let quoted = sql::quote_ident_mysql(name);
    let sql_text = match kind.as_str() {
        "function" => format!("show create function {quoted}"),
        "procedure" => format!("show create procedure {quoted}"),
        "trigger" => format!("show create trigger {quoted}"),
        "view" => {
            let schema = params.schema.trim();
            let qualified = if schema.is_empty() {
                quoted
            } else {
                format!("{}.{}", sql::quote_ident_mysql(schema), quoted)
            };
            format!("show create view {qualified}")
        }
        "index" => {
            let table = params.table.as_deref().unwrap_or("").trim();
            if table.is_empty() {
                return Err(DbError::validation("Table is required for index definition"));
            }
            let rows: Vec<(i64, String)> = conn
                .exec(
                    r#"
                    select non_unique, column_name
                    from information_schema.statistics
                    where table_schema = database()
                      and table_name = ?
                      and index_name = ?
                    order by seq_in_index
                    "#,
                    (table, name),
                )
                .await
                ?;
            if rows.is_empty() {
                return Err(DbError::NotFound("Index not found".to_string()));
            }
            let unique = rows[0].0 == 0;
            let columns = rows
                .into_iter()
                .map(|(_, column)| sql::quote_ident_mysql(&column))
                .collect::<Vec<_>>()
                .join(", ");
            let unique_sql = if unique { " unique" } else { "" };
            let table_sql = sql::quote_ident_mysql(table);
            let sql_out = if name.eq_ignore_ascii_case("PRIMARY") {
                format!("alter table {table_sql} add primary key ({columns});")
            } else {
                format!("create{unique_sql} index {quoted} on {table_sql} ({columns});")
            };
            return Ok(ObjectDefinition {
                title: name.to_string(),
                sql: sql_out,
            });
        }
        _ => return Err(DbError::validation(format!("Unsupported object type: {kind}"))),
    };

    let rows: Vec<MySqlRow> = conn.query(sql_text).await?;
    let create_sql = rows
        .first()
        .and_then(extract_mysql_create_sql)
        .ok_or_else(|| DbError::validation("Could not load object definition"))?;

    Ok(ObjectDefinition {
        title: name.to_string(),
        sql: if create_sql.trim_end().ends_with(';') {
            create_sql
        } else {
            format!("{};", create_sql.trim_end())
        },
    })
}

fn extract_mysql_create_sql(row: &MySqlRow) -> Option<String> {
    let mut best: Option<String> = None;
    for index in 0..row.len() {
        let value = row.as_ref(index).map(mysql_value_to_json).unwrap_or(Value::Null);
        if let Value::String(text) = value {
            let trimmed = text.trim();
            if trimmed.to_ascii_lowercase().starts_with("create") {
                return Some(text);
            }
            if trimmed.len() > best.as_ref().map(|item| item.len()).unwrap_or(0) {
                best = Some(text);
            }
        }
    }
    best
}

pub async fn list_databases(pool: &mysql_async::Pool) -> Result<Vec<String>, DbError> {
    let mut conn = pool.get_conn().await?;
    let dbs: Vec<String> = conn
        .query("show databases")
        .await
        ?;
    if dbs.is_empty() {
        let current: Option<String> = conn
            .query_first("select database()")
            .await
            ?;
        Ok(vec![current.unwrap_or_default()])
    } else {
        Ok(dbs)
    }
}

async fn mysql_hash_from_values<Q: Queryable>(
    conn: &mut Q,
    columns: &[String],
    row: &HashMap<String, Value>,
) -> Result<String, DbError> {
    let placeholders = columns
        .iter()
        .map(|_| "coalesce(cast(? as char), '__querycastle_null__')")
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!("select md5(concat_ws(char(31), {placeholders}))");
    let params: Vec<MySqlValue> = columns
        .iter()
        .map(|column| json_to_mysql_value(row.get(column).unwrap_or(&Value::Null)))
        .collect();
    conn.exec_first(sql, params)
        .await
        ?
        .ok_or_else(|| DbError::internal("Could not compute updated MySQL row identity."))
}

pub async fn apply_table_changes(
    pool: &mysql_async::Pool,
    params: &ApplyTableChangesParams,
) -> Result<ApplyTableChangesResponse, DbError> {
    let schema = params.schema.trim();
    let table = params.table.trim();
    if schema.is_empty() || table.is_empty() {
        return Err(DbError::validation("Schema and table are required"));
    }

    let mut conn = pool.get_conn().await?;
    let mut tx = conn
        .start_transaction(TxOpts::default())
        .await
        ?;

    let safe_schema = sql::quote_ident_mysql(schema);
    let safe_table = sql::quote_ident_mysql(table);
    let safe_table_ref = format!("{safe_schema}.{safe_table}");

    let column_query = "select column_name from information_schema.columns where table_schema = ? and table_name = ? order by ordinal_position";
    let columns: Vec<String> = tx
        .exec(column_query, (schema, table))
        .await
        ?;
    if columns.is_empty() {
        return Err(DbError::validation("Could not load table columns for MySQL table editing."));
    }

    let pk_sql = "select column_name from information_schema.columns where table_schema = ? and table_name = ? and column_key = 'PRI' order by ordinal_position";
    let pk_columns: Vec<String> = tx
        .exec(pk_sql, (schema, table))
        .await
        ?;
    if pk_columns.is_empty() {
        return Err(DbError::validation(
            "MySQL table editing requires a PRIMARY KEY. This table has none.",
        ));
    }
    let row_hash_expr = mysql_row_hash_expression(&pk_columns);

    let mut updated = 0usize;
    let mut deleted = 0usize;
    let mut inserted = 0usize;
    let mut updated_rows: Vec<UpdatedRow> = Vec::new();

    for update in &params.changes.updates {
        let entries: Vec<_> = update
            .values
            .iter()
            .filter(|(key, _)| key.as_str() != sql::HIDDEN_ROW_ID_COLUMN)
            .collect();
        if entries.is_empty() {
            continue;
        }

        let set_clause = entries
            .iter()
            .map(|(column, _)| format!("{} = ?", sql::quote_ident_mysql(column)))
            .collect::<Vec<_>>()
            .join(", ");

        let select_sql = format!("select * from {safe_table_ref} where {row_hash_expr} = ? limit 1");
        let current_row: Option<MySqlRow> = tx
            .exec_first(select_sql, (update.row_id.clone(),))
            .await?;
        let Some(current_row) = current_row else {
            return Err(DbError::NotFound(format!(
                "Could not update row {}. It may have changed. Refresh and retry.",
                update.row_id
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
            if column.as_str() == sql::HIDDEN_ROW_ID_COLUMN {
                continue;
            }
            merged_values.insert(column.clone(), value.clone());
        }

        let new_row_id = mysql_hash_from_values(&mut tx, &pk_columns, &merged_values).await?;

        let mut params_vec: Vec<MySqlValue> = entries.iter().map(|(_, value)| json_to_mysql_value(value)).collect();
        params_vec.push(MySqlValue::Bytes(update.row_id.clone().into_bytes()));
        let update_sql = format!("update {safe_table_ref} set {set_clause} where {row_hash_expr} = ? limit 1");
        tx.exec_drop(update_sql, params_vec)
            .await
            ?;
        let affected = tx.affected_rows();
        if affected == 0 {
            return Err(DbError::NotFound(format!(
                "Could not update row {}. It may have changed. Refresh and retry.",
                update.row_id
            )));
        }
        updated_rows.push(UpdatedRow {
            old_row_id: update.row_id.clone(),
            new_row_id,
            values: merged_values,
        });
        updated += 1;
    }

    for row_id in &params.changes.deletes {
        let delete_sql = format!("delete from {safe_table_ref} where {row_hash_expr} = ? limit 1");
        tx.exec_drop(delete_sql, (row_id.clone(),)).await?;
        let affected = tx.affected_rows();
        if affected == 0 {
            return Err(DbError::NotFound(format!(
                "Could not delete row {row_id}. It may have changed. Refresh and retry."
            )));
        }
        deleted += 1;
    }

    for row in &params.changes.inserts {
        let entries: Vec<_> = row
            .iter()
            .filter(|(key, _)| key.as_str() != sql::HIDDEN_ROW_ID_COLUMN)
            .collect();
        if entries.is_empty() {
            continue;
        }

        let cols = entries
            .iter()
            .map(|(column, _)| sql::quote_ident_mysql(column))
            .collect::<Vec<_>>()
            .join(", ");
        let placeholders = entries.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let params_vec: Vec<MySqlValue> = entries.iter().map(|(_, value)| json_to_mysql_value(value)).collect();
        let insert_sql = format!("insert into {safe_table_ref} ({cols}) values ({placeholders})");
        tx.exec_drop(insert_sql, params_vec)
            .await
            ?;
        inserted += 1;
    }

    tx.commit().await?;

    Ok(ApplyTableChangesResponse {
        ok: true,
        updated,
        deleted,
        inserted,
        updated_rows,
    })
}
