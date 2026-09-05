use rusqlite::types::ValueRef;
use rusqlite::{params_from_iter, Connection};
use serde_json::Value;
use std::collections::HashMap;

use crate::core::error::DbError;
use crate::core::limits::{apply_select_row_cap, MAX_QUERY_ROWS, QUERY_TIMEOUT_MS};
use crate::core::sql;
use crate::core::types::{
    ApplyTableChangesParams, ApplyTableChangesResponse, DatabaseColumn, DatabaseExplorer,
    DatabaseForeignKey, DatabaseIndex, DatabaseSchema, DatabaseTable, DatabaseTrigger,
    ObjectDefinition, ObjectDefinitionParams, QueryResultPayload, UpdatedRow,
};

type SqlitePool = r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>;

fn sqlite_value_to_json(value: ValueRef<'_>) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(v) => Value::Number(v.into()),
        ValueRef::Real(v) => serde_json::Number::from_f64(v)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        ValueRef::Text(v) => Value::String(String::from_utf8_lossy(v).to_string()),
        ValueRef::Blob(v) => {
            let mut out = String::with_capacity(v.len() * 2 + 2);
            out.push_str("0x");
            for byte in v {
                out.push_str(&format!("{byte:02x}"));
            }
            Value::String(out)
        }
    }
}

fn json_to_sqlite_value(value: &Value) -> rusqlite::types::Value {
    match value {
        Value::Null => rusqlite::types::Value::Null,
        Value::Bool(v) => rusqlite::types::Value::Integer(if *v { 1 } else { 0 }),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                rusqlite::types::Value::Integer(i)
            } else if let Some(f) = n.as_f64() {
                rusqlite::types::Value::Real(f)
            } else {
                rusqlite::types::Value::Text(n.to_string())
            }
        }
        Value::String(s) => rusqlite::types::Value::Text(s.clone()),
        Value::Array(_) | Value::Object(_) => rusqlite::types::Value::Text(value.to_string()),
    }
}

async fn with_pool<T, F>(pool: &SqlitePool, f: F) -> Result<T, DbError>
where
    T: Send + 'static,
    F: FnOnce(&SqlitePool) -> Result<T, DbError> + Send + 'static,
{
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || f(&pool)).await?
}

fn sqlite_main_database(conn: &Connection) -> Result<String, DbError> {
    Ok(conn.query_row("PRAGMA database_list", [], |row| {
        let name: String = row.get(1)?;
        let file: String = row.get(2)?;
        Ok(if file.is_empty() { name } else { file })
    })?)
}

pub async fn server_version(pool: &SqlitePool) -> Result<Option<String>, DbError> {
    with_pool(pool, |pool| {
        let conn = pool.get()?;
        Ok(conn
            .query_row("select sqlite_version()", [], |row| row.get::<_, String>(0))
            .map(Some)?)
    })
    .await
}

pub async fn run_query(pool: &SqlitePool, sql: &str) -> Result<QueryResultPayload, DbError> {
    let sql = apply_select_row_cap(sql).into_owned();
    let fut = with_pool(pool, move |pool| {
        let conn = pool.get()?;
        let started = std::time::Instant::now();

        let mut stmt = conn.prepare(&sql)?;
        let column_count = stmt.column_count();

        if column_count == 0 {
            drop(stmt);
            let affected = conn.execute(&sql, [])?;
            return Ok(QueryResultPayload {
                columns: Vec::new(),
                rows: Vec::new(),
                row_count: affected,
                duration_ms: started.elapsed().as_millis(),
                truncated: false,
            });
        }

        let columns: Vec<String> = stmt.column_names().iter().map(|name| (*name).to_string()).collect();

        let mut rows: Vec<HashMap<String, Value>> = Vec::new();
        let mut truncated = false;
        let mut result_rows = stmt.query([])?;

        while let Some(row) = result_rows.next()? {
            if rows.len() >= MAX_QUERY_ROWS {
                truncated = true;
                break;
            }
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

        let row_count = rows.len();
        Ok(QueryResultPayload {
            columns,
            rows,
            row_count,
            duration_ms: started.elapsed().as_millis(),
            truncated,
        })
    });

    match tokio::time::timeout(std::time::Duration::from_millis(QUERY_TIMEOUT_MS), fut).await {
        Ok(result) => result,
        Err(_) => Err(DbError::Timeout {
            message: format!("Query exceeded {QUERY_TIMEOUT_MS}ms"),
        }),
    }
}

pub async fn get_database_explorer(pool: &SqlitePool) -> Result<DatabaseExplorer, DbError> {
    with_pool(pool, |pool| {
        let conn = pool.get()?;
        get_sqlite_database_explorer(&conn)
    })
    .await
}

fn get_sqlite_database_explorer(conn: &Connection) -> Result<DatabaseExplorer, DbError> {
    let current_database = sqlite_main_database(conn)?;

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
        ?;

    let table_rows = table_stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        ?;

    let mut tables: Vec<DatabaseTable> = Vec::new();

    for entry in table_rows {
        let (table_name, table_type) = entry?;

        let pragma_name = sql::escape_single_quotes_pragma(&table_name);
        let columns_sql = format!("PRAGMA table_info('{pragma_name}')");
        let mut columns_stmt = conn
            .prepare(&columns_sql)
            ?;
        let column_rows = columns_stmt
            .query_map([], |row| {
                Ok(DatabaseColumn {
                    name: row.get::<_, String>(1)?,
                    data_type: row.get::<_, String>(2).unwrap_or_else(|_| "TEXT".to_string()),
                    not_null: row.get::<_, i64>(3).unwrap_or(0) != 0,
                    is_primary: row.get::<_, i64>(5).unwrap_or(0) != 0,
                    has_default: row.get::<_, Option<String>>(4)?.is_some(),
                })
            })
            ?;

        let mut columns: Vec<DatabaseColumn> = Vec::new();
        for column in column_rows {
            columns.push(column?);
        }

        let fk_sql = format!("PRAGMA foreign_key_list('{pragma_name}')");
        let mut fk_stmt = conn
            .prepare(&fk_sql)
            ?;
        let fk_rows = fk_stmt
            .query_map([], |row| {
                Ok(DatabaseForeignKey {
                    column: row.get::<_, String>(3)?,
                    referenced_schema: "main".to_string(),
                    referenced_table: row.get::<_, String>(2)?,
                    referenced_column: row.get::<_, String>(4)?,
                })
            })
            ?;

        let mut foreign_keys: Vec<DatabaseForeignKey> = Vec::new();
        for fk in fk_rows {
            foreign_keys.push(fk?);
        }

        let indexes = load_sqlite_indexes(conn, &table_name)?;
        let triggers = load_sqlite_triggers(conn, &table_name)?;

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
            indexes,
            triggers,
        });
    }

    Ok(DatabaseExplorer {
        database: current_database,
        schemas: vec![DatabaseSchema {
            name: "main".to_string(),
            tables,
            routines: Vec::new(),
            sequences: Vec::new(),
        }],
    })
}

fn load_sqlite_indexes(conn: &Connection, table_name: &str) -> Result<Vec<DatabaseIndex>, DbError> {
    let pragma_name = sql::escape_single_quotes_pragma(table_name);
    let sql_text = format!("PRAGMA index_list('{pragma_name}')");
    let mut stmt = conn
        .prepare(&sql_text)
        ?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2).unwrap_or(0) != 0,
                row.get::<_, String>(3).unwrap_or_default(),
            ))
        })
        ?;

    let mut indexes = Vec::new();
    for entry in rows {
        let (name, unique, origin) = entry?;
        let columns = sqlite_index_columns(conn, &name)?;
        let definition = sqlite_master_sql(conn, "index", &name)?;
        indexes.push(DatabaseIndex {
            name,
            columns,
            unique,
            is_primary: origin.eq_ignore_ascii_case("pk"),
            definition,
        });
    }
    Ok(indexes)
}

fn sqlite_index_columns(conn: &Connection, index_name: &str) -> Result<String, DbError> {
    let pragma_name = sql::escape_single_quotes_pragma(index_name);
    let sql_text = format!("PRAGMA index_info('{pragma_name}')");
    let mut stmt = conn
        .prepare(&sql_text)
        ?;
    let rows = stmt
        .query_map([], |row| row.get::<_, Option<String>>(2))
        ?;
    let mut columns = Vec::new();
    for entry in rows {
        if let Some(name) = entry? {
            columns.push(name);
        }
    }
    Ok(columns.join(", "))
}

fn load_sqlite_triggers(conn: &Connection, table_name: &str) -> Result<Vec<DatabaseTrigger>, DbError> {
    let mut stmt = conn
        .prepare(
            "
            select name, sql
            from sqlite_master
            where type = 'trigger' and tbl_name = ?
            order by name
            ",
        )
        ?;
    let rows = stmt
        .query_map([table_name], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })
        ?;
    let mut triggers = Vec::new();
    for entry in rows {
        let (name, definition) = entry?;
        triggers.push(DatabaseTrigger { name, definition });
    }
    Ok(triggers)
}

fn sqlite_master_sql(conn: &Connection, kind: &str, name: &str) -> Result<Option<String>, DbError> {
    conn.query_row(
        "select sql from sqlite_master where type = ?1 and name = ?2",
        [kind, name],
        |row| row.get::<_, Option<String>>(0),
    )
    .or_else(|error| {
        if matches!(error, rusqlite::Error::QueryReturnedNoRows) {
            Ok(None)
        } else {
            Err(error.into())
        }
    })
}

pub async fn get_object_definition(
    pool: &SqlitePool,
    params: &ObjectDefinitionParams,
) -> Result<ObjectDefinition, DbError> {
    let params = params.clone();
    with_pool(pool, move |pool| {
        let conn = pool.get()?;
        get_sqlite_object_definition(&conn, &params)
    })
    .await
}

fn get_sqlite_object_definition(
    conn: &Connection,
    params: &ObjectDefinitionParams,
) -> Result<ObjectDefinition, DbError> {
    let kind = params.kind.trim().to_ascii_lowercase();
    let name = params.name.trim();
    if name.is_empty() {
        return Err(DbError::validation("Name is required"));
    }
    let master_kind = match kind.as_str() {
        "index" => "index",
        "trigger" => "trigger",
        "view" => "view",
        "table" => "table",
        _ => return Err(DbError::validation(format!("SQLite does not support {kind} definitions"))),
    };
    let sql_text = sqlite_master_sql(conn, master_kind, name)?
        .ok_or_else(|| DbError::validation("Could not load object definition"))?;
    Ok(ObjectDefinition {
        title: name.to_string(),
        sql: if sql_text.trim_end().ends_with(';') {
            sql_text
        } else {
            format!("{};", sql_text.trim_end())
        },
    })
}

pub async fn list_databases(pool: &SqlitePool) -> Result<Vec<String>, DbError> {
    with_pool(pool, |pool| {
        let conn = pool.get()?;
        Ok(vec![sqlite_main_database(&conn)?])
    })
    .await
}

pub async fn apply_table_changes(
    pool: &SqlitePool,
    params: &ApplyTableChangesParams,
) -> Result<ApplyTableChangesResponse, DbError> {
    let params = params.clone();
    with_pool(pool, move |pool| apply_sqlite_table_changes(pool, &params)).await
}

fn apply_sqlite_table_changes(
    pool: &SqlitePool,
    params: &ApplyTableChangesParams,
) -> Result<ApplyTableChangesResponse, DbError> {
    let schema = params.schema.trim();
    let table = params.table.trim();
    if schema.is_empty() || table.is_empty() {
        return Err(DbError::validation("Schema and table are required"));
    }

    let mut conn = pool.get()?;
    let tx = conn
        .transaction()
        ?;

    let safe_table = format!("{}.{}", sql::quote_ident(schema), sql::quote_ident(table));

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
    let mut updated_rows: Vec<UpdatedRow> = Vec::new();

    for update in &params.changes.updates {
        let rowid = update.row_id.trim().parse::<i64>().map_err(|_| {
            if is_without_rowid {
                DbError::validation(format!(
                    "Table '{}' uses WITHOUT ROWID and requires primary-key editing; rowid {} is invalid. Use SQL directly.",
                    table, update.row_id
                ))
            } else {
                DbError::validation(format!("Invalid SQLite row id: {}", update.row_id))
            }
        })?;

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
            .enumerate()
            .map(|(index, (column, _))| format!("{} = ?{}", sql::quote_ident(column), index + 1))
            .collect::<Vec<_>>()
            .join(", ");
        let rowid_param = entries.len() + 1;
        let update_sql = format!("update {safe_table} set {set_clause} where rowid = ?{rowid_param}");
        let mut sql_params: Vec<rusqlite::types::Value> =
            entries.iter().map(|(_, value)| json_to_sqlite_value(value)).collect();
        sql_params.push(rusqlite::types::Value::Integer(rowid));
        let affected = tx
            .execute(update_sql.as_str(), params_from_iter(sql_params))
            ?;
        if affected == 0 {
            return Err(DbError::NotFound(format!(
                "Could not update row with id {}. It may have changed. Refresh and retry.",
                update.row_id
            )));
        }

        let select_sql = format!("select * from {safe_table} where rowid = ?1 limit 1");
        let mut stmt = tx
            .prepare(select_sql.as_str())
            ?;
        let row_values = stmt
            .query_row([rowid], |row| {
                let mut mapped = HashMap::new();
                for (index, column_name) in row.as_ref().column_names().iter().enumerate() {
                    let value = row.get_ref(index).map(sqlite_value_to_json)?;
                    mapped.insert(column_name.to_string(), value);
                }
                Ok::<HashMap<String, Value>, rusqlite::Error>(mapped)
            })
            ?;

        updated_rows.push(UpdatedRow {
            old_row_id: update.row_id.clone(),
            new_row_id: rowid.to_string(),
            values: row_values,
        });
        updated += 1;
    }

    for row_id in &params.changes.deletes {
        let rowid = row_id.trim().parse::<i64>().map_err(|_| {
            if is_without_rowid {
                DbError::validation(format!(
                    "Table '{}' uses WITHOUT ROWID and requires primary-key editing; rowid {row_id} is invalid. Use SQL directly.",
                    table
                ))
            } else {
                DbError::validation(format!("Invalid SQLite row id: {row_id}"))
            }
        })?;
        let delete_sql = format!("delete from {safe_table} where rowid = ?1");
        let affected = tx.execute(delete_sql.as_str(), [rowid])?;
        if affected == 0 {
            return Err(DbError::NotFound(format!(
                "Could not delete row with id {row_id}. It may have changed. Refresh and retry."
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
            .map(|(column, _)| sql::quote_ident(column))
            .collect::<Vec<_>>()
            .join(", ");
        let placeholders = (1..=entries.len())
            .map(|index| format!("?{index}"))
            .collect::<Vec<_>>()
            .join(", ");
        let sql_params: Vec<rusqlite::types::Value> =
            entries.iter().map(|(_, value)| json_to_sqlite_value(value)).collect();
        let insert_sql = format!("insert into {safe_table} ({cols}) values ({placeholders})");
        tx.execute(insert_sql.as_str(), params_from_iter(sql_params))
            ?;
        inserted += 1;
    }

    tx.commit()?;

    Ok(ApplyTableChangesResponse {
        ok: true,
        updated,
        deleted,
        inserted,
        updated_rows,
    })
}
