use rusqlite::types::ValueRef;
use rusqlite::Connection;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Instant;

use crate::core::types::{
    ConnectionInput, DatabaseColumn, DatabaseExplorer, DatabaseForeignKey, DatabaseSchema,
    DatabaseTable, QueryResultPayload,
};

fn escape_single_quotes(value: &str) -> String {
    crate::core::sql::escape_single_quotes_pragma(value)
}

pub(crate) fn sqlite_value_to_json(value: ValueRef<'_>) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(v) => Value::Number(v.into()),
        ValueRef::Real(v) => serde_json::Number::from_f64(v)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        ValueRef::Text(v) => Value::String(String::from_utf8_lossy(v).to_string()),
        ValueRef::Blob(v) => {
            // Unified hex representation (consistent with adapter)
            let mut out = String::with_capacity(v.len() * 2 + 2);
            out.push_str("0x");
            for byte in v {
                out.push_str(&format!("{byte:02x}"));
            }
            Value::String(out)
        }
    }
}

pub(crate) fn open_sqlite_connection(connection: &ConnectionInput) -> Result<Connection, String> {
    let conn = Connection::open(&connection.database).map_err(|error| format!("Unable to open SQLite database: {error}"))?;
    // Mitigate SQLITE_BUSY and improve concurrency for desktop use
    let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
    // WAL + foreign_keys are set via pool init; for direct opens we best-effort enable them (ignore errors for in-memory)
    let _ = conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;");
    Ok(conn)
}

pub(crate) fn get_sqlite_server_version(conn: &Connection) -> Result<Option<String>, String> {
    conn.query_row("select sqlite_version()", [], |row| row.get::<_, String>(0))
        .map(Some)
        .map_err(|error| format!("Unable to read SQLite version: {error}"))
}

pub(crate) fn run_sqlite_query(connection: &ConnectionInput, sql: &str) -> Result<QueryResultPayload, String> {
    let conn = open_sqlite_connection(connection)?;
    let started = Instant::now();

    let mut stmt = conn
        .prepare(sql)
        .map_err(|error| format!("SQLite query prepare failed: {error}"))?;
    let column_count = stmt.column_count();

    if column_count == 0 {
        drop(stmt);
        let affected = conn
            .execute(sql, [])
            .map_err(|error| format!("SQLite query execution failed: {error}"))?;
        return Ok(QueryResultPayload {
            columns: Vec::new(),
            rows: Vec::new(),
            row_count: affected,
            duration_ms: started.elapsed().as_millis(),
        });
    }

    let columns: Vec<String> = stmt.column_names().iter().map(|name| (*name).to_string()).collect();

    let mut row_count = 0usize;
    let mut rows: Vec<HashMap<String, Value>> = Vec::new();
    let mut result_rows = stmt
        .query([])
        .map_err(|error| format!("SQLite query execution failed: {error}"))?;

    while let Some(row) = result_rows
        .next()
        .map_err(|error| format!("SQLite row read failed: {error}"))?
    {
        row_count += 1;
        if rows.len() < crate::core::db::MAX_QUERY_ROWS {
            let mut mapped = HashMap::new();
            for (index, column_name) in columns.iter().enumerate() {
                let value = row
                    .get_ref(index)
                    .map(sqlite_value_to_json)
                    .unwrap_or(Value::Null);
                mapped.insert(column_name.clone(), value);
            }
            rows.push(mapped);
        }
    }

    Ok(QueryResultPayload {
        columns,
        rows,
        row_count,
        duration_ms: started.elapsed().as_millis(),
    })
}

pub(crate) fn get_sqlite_database_explorer(connection: &ConnectionInput) -> Result<DatabaseExplorer, String> {
    let conn = open_sqlite_connection(connection)?;

    let mut table_stmt = conn
        .prepare(
            "
            select name, type
            from sqlite_master
            where type in ('table', 'view')
              and name not like 'sqlite_%'
            order by name
            ",
        )
        .map_err(|error| format!("SQLite schema query failed: {error}"))?;

    let table_rows = table_stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| format!("SQLite schema read failed: {error}"))?;

    let mut tables: Vec<DatabaseTable> = Vec::new();

    for entry in table_rows {
        let (table_name, table_type) = entry.map_err(|error| format!("SQLite table read failed: {error}"))?;

        let pragma_name = escape_single_quotes(&table_name);
        let columns_sql = format!("PRAGMA table_info('{pragma_name}')");
        let mut columns_stmt = conn
            .prepare(&columns_sql)
            .map_err(|error| format!("SQLite table info query failed: {error}"))?;
        let column_rows = columns_stmt
            .query_map([], |row| {
                Ok(DatabaseColumn {
                    name: row.get::<_, String>(1)?,
                    data_type: row.get::<_, String>(2).unwrap_or_else(|_| "TEXT".to_string()),
                    not_null: row.get::<_, i64>(3).unwrap_or(0) != 0,
                    is_primary: row.get::<_, i64>(5).unwrap_or(0) != 0,
                })
            })
            .map_err(|error| format!("SQLite table info read failed: {error}"))?;

        let mut columns: Vec<DatabaseColumn> = Vec::new();
        for column in column_rows {
            columns.push(column.map_err(|error| format!("SQLite column parse failed: {error}"))?);
        }

        let fk_sql = format!("PRAGMA foreign_key_list('{pragma_name}')");
        let mut fk_stmt = conn
            .prepare(&fk_sql)
            .map_err(|error| format!("SQLite foreign key query failed: {error}"))?;
        let fk_rows = fk_stmt
            .query_map([], |row| {
                Ok(DatabaseForeignKey {
                    column: row.get::<_, String>(3)?,
                    referenced_schema: "main".to_string(),
                    referenced_table: row.get::<_, String>(2)?,
                    referenced_column: row.get::<_, String>(4)?,
                })
            })
            .map_err(|error| format!("SQLite foreign key read failed: {error}"))?;

        let mut foreign_keys: Vec<DatabaseForeignKey> = Vec::new();
        for fk in fk_rows {
            foreign_keys.push(fk.map_err(|error| format!("SQLite foreign key parse failed: {error}"))?);
        }

        tables.push(DatabaseTable {
            schema: "main".to_string(),
            name: table_name,
            kind: if table_type == "view" {
                "view".to_string()
            } else {
                "table".to_string()
            },
            columns,
            foreign_keys,
        });
    }

    Ok(DatabaseExplorer {
        database: connection.database.clone(),
        schemas: vec![DatabaseSchema {
            name: "main".to_string(),
            tables,
        }],
    })
}

pub(crate) fn list_sqlite_databases(connection: &ConnectionInput) -> Vec<String> {
    vec![connection.database.clone()]
}
