use mysql_async::prelude::Queryable;
use mysql_async::{Conn as MySqlConn, Opts as MySqlOpts, OptsBuilder as MySqlOptsBuilder, Row as MySqlRow, Value as MySqlValue};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Instant;

use crate::core::types::ConnectionInput;
use crate::core::db::MAX_QUERY_ROWS;
use crate::core::types::QueryResultPayload;

pub(crate) fn sanitize_mysql_error(error: mysql_async::Error) -> String {
    error.to_string()
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
