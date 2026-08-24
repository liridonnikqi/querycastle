use mysql_async::prelude::Queryable;
use mysql_async::{Conn as MySqlConn, Opts as MySqlOpts, OptsBuilder as MySqlOptsBuilder, Row as MySqlRow, Value as MySqlValue};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Instant;

use crate::core::types::ConnectionInput;
use crate::core::db::MAX_QUERY_ROWS;
use crate::core::types::{
    DatabaseColumn, DatabaseExplorer, DatabaseForeignKey, DatabaseSchema, DatabaseTable,
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
    )> = conn
        .query(
            r#"
            select
                t.table_schema as schema_name,
                t.table_name as table_name,
                t.table_type as table_type,
                c.column_name as column_name,
                c.column_type as data_type,
                c.is_nullable as is_nullable
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

    for (schema_name, table_name, table_type, column_name, data_type, is_nullable) in table_rows {
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
                kind: if table_type == "VIEW" {
                    "view".to_string()
                } else {
                    "table".to_string()
                },
                columns: Vec::new(),
                foreign_keys: Vec::new(),
            });

        if let Some(column_name) = column_name {
            if let Some(table) = table_map.get_mut(&table_key) {
                table.columns.push(DatabaseColumn {
                    name: column_name,
                    data_type: data_type.unwrap_or_else(|| "unknown".to_string()),
                    not_null: is_nullable
                        .map(|value| value.eq_ignore_ascii_case("NO"))
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
