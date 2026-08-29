use mysql_async::prelude::Queryable;
use mysql_async::{Conn as MySqlConn, Opts as MySqlOpts, OptsBuilder as MySqlOptsBuilder, Row as MySqlRow, Value as MySqlValue};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Instant;

use crate::core::types::ConnectionInput;
use crate::core::db::MAX_QUERY_ROWS;
use crate::core::types::{
    DatabaseColumn, DatabaseExplorer, DatabaseForeignKey, DatabaseIndex, DatabaseRoutine,
    DatabaseSchema, DatabaseTable, DatabaseTrigger, ObjectDefinition, ObjectDefinitionParams,
    QueryResultPayload,
};

pub(crate) fn sanitize_mysql_error(error: mysql_async::Error) -> String {
    error.to_string()
}

pub(crate) fn mysql_value_to_json(value: &MySqlValue) -> Value {
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

pub(crate) async fn connect_mysql_client(connection: &ConnectionInput) -> Result<MySqlConn, String> {
    let opts = if connection.use_connection_string.unwrap_or(false)
        && connection.connection_string.as_deref().unwrap_or_default().trim().starts_with("mysql://")
    {
        MySqlOpts::from_url(connection.connection_string.as_deref().unwrap_or_default())
            .map_err(|error| format!("Invalid MySQL connection string: {error}"))?
    } else {
        let mut builder = MySqlOptsBuilder::default();
        builder = builder
            .ip_or_hostname(connection.host.clone())
            .tcp_port(connection.port)
            .user(Some(connection.user.clone()))
            .pass(Some(connection.password.clone()))
            .db_name(Some(connection.database.clone()));
        MySqlOpts::from(builder)
    };

    MySqlConn::new(opts)
        .await
        .map_err(|error| format!("Unable to connect to MySQL: {}", sanitize_mysql_error(error)))
}

pub(crate) async fn get_mysql_server_version(conn: &mut MySqlConn) -> Result<Option<String>, String> {
    conn.query_first::<String, _>("select version()")
        .await
        .map_err(sanitize_mysql_error)
}

pub(crate) async fn run_mysql_query(connection: &ConnectionInput, sql: &str) -> Result<QueryResultPayload, String> {
    let mut conn = connect_mysql_client(connection).await?;
    let started = Instant::now();
    let rows: Vec<MySqlRow> = conn.query(sql).await.map_err(sanitize_mysql_error)?;

    let mut columns: Vec<String> = Vec::new();
    let mut mapped_rows: Vec<HashMap<String, Value>> = Vec::new();
    for row in rows {
        if columns.is_empty() {
            columns = row
                .columns_ref()
                .iter()
                .map(|column| column.name_str().to_string())
                .collect();
        }
        let mut mapped = HashMap::new();
        for (index, column_name) in columns.iter().enumerate() {
            let value = row.as_ref(index).map(mysql_value_to_json).unwrap_or(Value::Null);
            mapped.insert(column_name.clone(), value);
        }
        mapped_rows.push(mapped);
    }

    let row_count = mapped_rows.len();
    let limited_rows = if row_count > MAX_QUERY_ROWS {
        mapped_rows.into_iter().take(MAX_QUERY_ROWS).collect()
    } else {
        mapped_rows
    };

    Ok(QueryResultPayload {
        columns,
        rows: limited_rows,
        row_count,
        duration_ms: started.elapsed().as_millis(),
    })
}

pub(crate) async fn get_mysql_database_explorer(
    connection: &ConnectionInput,
) -> Result<DatabaseExplorer, String> {
    let mut conn = connect_mysql_client(connection).await?;
    let current_database = connection.database.clone();

    let table_rows: Vec<(
        String,
        String,
        String,
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
                c.column_key as column_key
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
        .map_err(sanitize_mysql_error)?;

    let mut schema_map: HashMap<String, DatabaseSchema> = HashMap::new();
    let mut table_map: HashMap<String, DatabaseTable> = HashMap::new();

    for (schema_name, table_name, table_type, column_name, data_type, is_nullable, column_key) in table_rows {
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
                table.columns.push(DatabaseColumn {
                    name: column_name,
                    data_type: data_type.unwrap_or_else(|| "unknown".to_string()),
                    not_null: is_nullable
                        .map(|value| value.eq_ignore_ascii_case("NO"))
                        .unwrap_or(false),
                    is_primary: column_key
                        .map(|value| value.eq_ignore_ascii_case("PRI"))
                        .unwrap_or(false),
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
        .map_err(sanitize_mysql_error)?;

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
    conn: &mut MySqlConn,
    table_map: &mut HashMap<String, DatabaseTable>,
) -> Result<(), String> {
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
        .map_err(sanitize_mysql_error)?;

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
    conn: &mut MySqlConn,
    table_map: &mut HashMap<String, DatabaseTable>,
) -> Result<(), String> {
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
        .map_err(sanitize_mysql_error)?;

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
    conn: &mut MySqlConn,
    schema_map: &mut HashMap<String, DatabaseSchema>,
) -> Result<(), String> {
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
        .map_err(sanitize_mysql_error)?;

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

pub(crate) async fn get_mysql_object_definition(
    connection: &ConnectionInput,
    params: &ObjectDefinitionParams,
) -> Result<ObjectDefinition, String> {
    let mut conn = connect_mysql_client(connection).await?;
    let kind = params.kind.trim().to_ascii_lowercase();
    let name = params.name.trim();
    if name.is_empty() {
        return Err("Name is required".to_string());
    }
    let quoted = crate::core::sql::quote_ident_mysql(name);
    let sql = match kind.as_str() {
        "function" => format!("show create function {quoted}"),
        "procedure" => format!("show create procedure {quoted}"),
        "trigger" => format!("show create trigger {quoted}"),
        "view" => {
            let schema = params.schema.trim();
            let qualified = if schema.is_empty() {
                quoted
            } else {
                format!("{}.{}", crate::core::sql::quote_ident_mysql(schema), quoted)
            };
            format!("show create view {qualified}")
        }
        "index" => {
            let table = params.table.as_deref().unwrap_or("").trim();
            if table.is_empty() {
                return Err("Table is required for index definition".to_string());
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
                .map_err(sanitize_mysql_error)?;
            if rows.is_empty() {
                return Err("Index not found".to_string());
            }
            let unique = rows[0].0 == 0;
            let columns = rows
                .into_iter()
                .map(|(_, column)| crate::core::sql::quote_ident_mysql(&column))
                .collect::<Vec<_>>()
                .join(", ");
            let unique_sql = if unique { " unique" } else { "" };
            let table_sql = crate::core::sql::quote_ident_mysql(table);
            let sql_text = if name.eq_ignore_ascii_case("PRIMARY") {
                format!("alter table {table_sql} add primary key ({columns});")
            } else {
                format!("create{unique_sql} index {quoted} on {table_sql} ({columns});")
            };
            return Ok(ObjectDefinition {
                title: name.to_string(),
                sql: sql_text,
            });
        }
        _ => return Err(format!("Unsupported object type: {kind}")),
    };

    let rows: Vec<MySqlRow> = conn.query(sql).await.map_err(sanitize_mysql_error)?;
    let create_sql = rows
        .first()
        .and_then(extract_mysql_create_sql)
        .ok_or_else(|| "Could not load object definition".to_string())?;

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
