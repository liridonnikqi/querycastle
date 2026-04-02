use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::time::Instant;
use tauri::State;
use tokio::sync::Mutex;
use tokio_postgres::{Client, SimpleQueryMessage};
use url::Url;

const QUERY_TIMEOUT_MS: u64 = 30_000;
const MAX_QUERY_ROWS: usize = 1_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConnectionInput {
    name: String,
    host: String,
    port: u16,
    user: String,
    password: String,
    database: String,
    ssl: bool,
    use_connection_string: Option<bool>,
    connection_string: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConnectionStatus {
    connected: bool,
    name: String,
    host: String,
    port: u16,
    database: String,
    user: String,
    server_version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct QueryResultPayload {
    columns: Vec<String>,
    rows: Vec<HashMap<String, Value>>,
    row_count: usize,
    duration_ms: u128,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TestConnectionResponse {
    ok: bool,
    message: String,
    server_version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApplyTableChangesResponse {
    ok: bool,
    updated: usize,
    deleted: usize,
    inserted: usize,
    updated_rows: Vec<UpdatedRowCtid>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdatedRowCtid {
    old_ctid: String,
    new_ctid: String,
    values: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DatabaseColumn {
    name: String,
    data_type: String,
    not_null: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DatabaseForeignKey {
    column: String,
    referenced_schema: String,
    referenced_table: String,
    referenced_column: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DatabaseTable {
    schema: String,
    name: String,
    kind: String,
    columns: Vec<DatabaseColumn>,
    foreign_keys: Vec<DatabaseForeignKey>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DatabaseSchema {
    name: String,
    tables: Vec<DatabaseTable>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DatabaseExplorer {
    database: String,
    schemas: Vec<DatabaseSchema>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct QueryParams {
    sql: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SelectDatabaseParams {
    database: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApplyTableChangesParams {
    schema: String,
    table: String,
    changes: TableChangesPayload,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TableChangesPayload {
    updates: Vec<TableUpdatePayload>,
    deletes: Vec<String>,
    inserts: Vec<HashMap<String, Value>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TableUpdatePayload {
    ctid: String,
    values: HashMap<String, Value>,
}

struct AppState {
    connection: Mutex<Option<ConnectionInput>>,
    server_version: Mutex<Option<String>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            connection: Mutex::new(None),
            server_version: Mutex::new(None),
        }
    }
}

fn sanitize_error(error: impl ToString) -> String {
    error.to_string()
}

fn sanitize_pg_error(error: tokio_postgres::Error) -> String {
    if let Some(db_error) = error.as_db_error() {
        let mut lines = vec![format!(
            "{} (SQLSTATE {})",
            db_error.message(),
            db_error.code().code()
        )];
        if let Some(detail) = db_error.detail() {
            lines.push(format!("Detail: {detail}"));
        }
        if let Some(hint) = db_error.hint() {
            lines.push(format!("Hint: {hint}"));
        }
        if let Some(position) = db_error.position() {
            lines.push(format!("Position: {position:?}"));
        }
        return lines.join("\n");
    }

    let base = error.to_string();
    if base.trim().eq_ignore_ascii_case("db error") {
        let mut causes = Vec::new();
        let mut source = error.source();
        while let Some(cause) = source {
            causes.push(cause.to_string());
            source = cause.source();
        }
        if !causes.is_empty() {
            return format!("Database error\nCaused by: {}", causes.join("\nCaused by: "));
        }
    }
    base
}

fn quote_ident(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn escape_sql_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "''")
}

fn value_to_sql_literal(value: &Value) -> String {
    match value {
        Value::Null => "NULL".to_string(),
        Value::Bool(v) => {
            if *v {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        Value::Number(v) => v.to_string(),
        Value::String(v) => format!("'{}'", escape_sql_string(v)),
        Value::Array(_) | Value::Object(_) => {
            format!("'{}'::jsonb", escape_sql_string(&value.to_string()))
        }
    }
}

fn normalize_connection_input(input: ConnectionInput) -> Result<ConnectionInput, String> {
    if input.use_connection_string.unwrap_or(false) {
        let raw = input.connection_string.clone().unwrap_or_default().trim().to_string();
        if raw.is_empty() {
            return Err("Connection string is required".to_string());
        }

        let parsed = Url::parse(&raw).map_err(|_| "Invalid connection string format")?;
        if parsed.scheme() != "postgresql" && parsed.scheme() != "postgres" {
            return Err("Connection string must start with postgresql:// or postgres://".to_string());
        }

        let database = parsed.path().trim_start_matches('/').to_string();
        if database.is_empty() {
            return Err("Database name is missing in connection string".to_string());
        }

        let resolved_port = parsed.port().unwrap_or(5432);
        let sslmode = parsed
            .query_pairs()
            .find(|(key, _)| key == "sslmode")
            .map(|(_, val)| val.to_lowercase());
        let ssl_from_url = sslmode
            .as_deref()
            .map(|mode| ["require", "verify-ca", "verify-full"].contains(&mode));

        return Ok(ConnectionInput {
            name: if input.name.trim().is_empty() {
                database.clone()
            } else {
                input.name.trim().to_string()
            },
            host: if parsed.host_str().unwrap_or_default().is_empty() {
                "localhost".to_string()
            } else {
                parsed.host_str().unwrap_or_default().to_string()
            },
            port: resolved_port,
            user: parsed.username().to_string(),
            password: parsed.password().unwrap_or_default().to_string(),
            database,
            ssl: ssl_from_url.unwrap_or(input.ssl),
            use_connection_string: Some(true),
            connection_string: Some(raw),
        });
    }

    if input.host.trim().is_empty() || input.database.trim().is_empty() || input.user.trim().is_empty() {
        return Err("Host, database, and user are required".to_string());
    }

    Ok(ConnectionInput {
        name: if input.name.trim().is_empty() {
            input.database.clone()
        } else {
            input.name.trim().to_string()
        },
        host: input.host,
        port: if input.port == 0 { 5432 } else { input.port },
        user: input.user,
        password: input.password,
        database: input.database,
        ssl: input.ssl,
        use_connection_string: Some(false),
        connection_string: Some(String::new()),
    })
}

async fn connect_client(connection: &ConnectionInput) -> Result<Client, String> {
    let mut hosts = vec![connection.host.clone()];
    if connection.host.eq_ignore_ascii_case("localhost") {
        hosts.push("127.0.0.1".to_string());
        hosts.push("::1".to_string());
    }

    let mut attempts: Vec<String> = Vec::new();
    for host in hosts {
        let mut config = tokio_postgres::Config::new();
        config
            .host(&host)
            .port(connection.port)
            .user(&connection.user)
            .password(&connection.password)
            .dbname(&connection.database)
            .connect_timeout(std::time::Duration::from_secs(5));

        if connection.ssl {
            let mut builder = native_tls::TlsConnector::builder();
            builder.danger_accept_invalid_certs(true);
            let connector = builder
                .build()
                .map_err(|error| format!("Failed to build TLS connector: {error}"))?;
            let tls = postgres_native_tls::MakeTlsConnector::new(connector);
            match config.connect(tls).await {
                Ok((client, connection_task)) => {
                    tauri::async_runtime::spawn(async move {
                        if let Err(error) = connection_task.await {
                            eprintln!("postgres connection error: {error}");
                        }
                    });
                    return Ok(client);
                }
                Err(error) => attempts.push(format!("{host}: {}", sanitize_pg_error(error))),
            }
        } else {
            match config.connect(tokio_postgres::NoTls).await {
                Ok((client, connection_task)) => {
                    tauri::async_runtime::spawn(async move {
                        if let Err(error) = connection_task.await {
                            eprintln!("postgres connection error: {error}");
                        }
                    });
                    return Ok(client);
                }
                Err(error) => attempts.push(format!("{host}: {}", sanitize_pg_error(error))),
            }
        }
    }

    Err(format!(
        "Unable to connect to PostgreSQL at {}:{} (database '{}', user '{}'). Attempts: {}",
        connection.host,
        connection.port,
        connection.database,
        connection.user,
        attempts.join(" | ")
    ))
}

async fn get_server_version(client: &Client) -> Result<Option<String>, String> {
    let row = client
        .query_one("select current_setting('server_version') as server_version", &[])
        .await
        .map_err(sanitize_pg_error)?;
    let server_version: Option<String> = row.try_get("server_version").map_err(sanitize_pg_error)?;
    Ok(server_version)
}

fn disconnected_status() -> ConnectionStatus {
    ConnectionStatus {
        connected: false,
        name: "Disconnected".to_string(),
        host: String::new(),
        port: 5432,
        database: String::new(),
        user: String::new(),
        server_version: None,
    }
}

async fn get_connection_snapshot(
    state: &State<'_, AppState>,
) -> Result<(ConnectionInput, Option<String>), String> {
    let connection = state.connection.lock().await.clone();
    let server_version = state.server_version.lock().await.clone();
    match connection {
        Some(connection) => Ok((connection, server_version)),
        None => Err("No active PostgreSQL connection".to_string()),
    }
}

#[tauri::command]
async fn connection_status(state: State<'_, AppState>) -> Result<ConnectionStatus, String> {
    let connection = state.connection.lock().await.clone();
    let server_version = state.server_version.lock().await.clone();

    if let Some(active) = connection {
        return Ok(ConnectionStatus {
            connected: true,
            name: active.name,
            host: active.host,
            port: active.port,
            database: active.database,
            user: active.user,
            server_version,
        });
    }

    Ok(disconnected_status())
}

#[tauri::command]
async fn test_connection(params: ConnectionInput) -> Result<TestConnectionResponse, String> {
    let normalized = normalize_connection_input(params)?;
    let client = match connect_client(&normalized).await {
        Ok(client) => client,
        Err(error) => {
            return Ok(TestConnectionResponse {
                ok: false,
                message: error,
                server_version: None,
            })
        }
    };

    let server_version = match get_server_version(&client).await {
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

#[tauri::command]
async fn connect(params: ConnectionInput, state: State<'_, AppState>) -> Result<ConnectionStatus, String> {
    let normalized = normalize_connection_input(params)?;
    let client = connect_client(&normalized).await?;
    let server_version = get_server_version(&client).await?;
    client
        .batch_execute(&format!("set statement_timeout = {QUERY_TIMEOUT_MS}"))
        .await
        .map_err(sanitize_pg_error)?;

    {
        let mut connection = state.connection.lock().await;
        *connection = Some(normalized.clone());
    }
    {
        let mut version = state.server_version.lock().await;
        *version = server_version.clone();
    }

    Ok(ConnectionStatus {
        connected: true,
        name: normalized.name,
        host: normalized.host,
        port: normalized.port,
        database: normalized.database,
        user: normalized.user,
        server_version,
    })
}

#[tauri::command]
async fn disconnect(state: State<'_, AppState>) -> Result<HashMap<String, bool>, String> {
    {
        let mut connection = state.connection.lock().await;
        *connection = None;
    }
    {
        let mut version = state.server_version.lock().await;
        *version = None;
    }

    Ok(HashMap::from([(String::from("ok"), true)]))
}

#[tauri::command]
async fn run_query(params: QueryParams, state: State<'_, AppState>) -> Result<QueryResultPayload, String> {
    let (connection, _) = get_connection_snapshot(&state).await?;
    let client = connect_client(&connection).await?;
    client
        .batch_execute(&format!("set statement_timeout = {QUERY_TIMEOUT_MS}"))
        .await
        .map_err(sanitize_pg_error)?;

    let started = Instant::now();
    let messages = client
        .simple_query(params.sql.as_str())
        .await
        .map_err(sanitize_pg_error)?;

    let mut columns: Vec<String> = Vec::new();
    let mut rows: Vec<HashMap<String, Value>> = Vec::new();

    for message in messages {
        if let SimpleQueryMessage::Row(row) = message {
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
    let limited_rows = if row_count > MAX_QUERY_ROWS {
        rows.into_iter().take(MAX_QUERY_ROWS).collect()
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

#[tauri::command]
async fn get_database_explorer(state: State<'_, AppState>) -> Result<DatabaseExplorer, String> {
    let (connection, _) = get_connection_snapshot(&state).await?;
    let client = connect_client(&connection).await?;

    let db_row = client
        .query_one("select current_database() as current_database", &[])
        .await
        .map_err(sanitize_pg_error)?;
    let current_database: String = db_row.try_get("current_database").map_err(sanitize_pg_error)?;

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
        .map_err(sanitize_pg_error)?;

    let mut schema_map: HashMap<String, DatabaseSchema> = HashMap::new();
    let mut table_map: HashMap<String, DatabaseTable> = HashMap::new();

    for row in rows {
        let schema_name: String = row.try_get("schema_name").map_err(sanitize_pg_error)?;
        let table_name: String = row.try_get("table_name").map_err(sanitize_pg_error)?;
        let relkind: String = row.try_get("relkind").map_err(sanitize_pg_error)?;
        let column_name: Option<String> = row.try_get("column_name").map_err(sanitize_pg_error)?;
        let data_type: Option<String> = row.try_get("data_type").map_err(sanitize_pg_error)?;
        let not_null: Option<bool> = row.try_get("not_null").map_err(sanitize_pg_error)?;

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
        .map_err(sanitize_pg_error)?;

    for fk in fk_rows {
        let table_schema: String = fk.try_get("table_schema").map_err(sanitize_pg_error)?;
        let table_name: String = fk.try_get("table_name").map_err(sanitize_pg_error)?;
        let table_key = format!("{table_schema}.{table_name}");
        if let Some(table) = table_map.get_mut(&table_key) {
            table.foreign_keys.push(DatabaseForeignKey {
                column: fk.try_get("column_name").map_err(sanitize_pg_error)?,
                referenced_schema: fk.try_get("foreign_table_schema").map_err(sanitize_pg_error)?,
                referenced_table: fk.try_get("foreign_table_name").map_err(sanitize_pg_error)?,
                referenced_column: fk.try_get("foreign_column_name").map_err(sanitize_pg_error)?,
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

#[tauri::command]
async fn list_databases(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let (connection, _) = get_connection_snapshot(&state).await?;
    let current_database = connection.database.clone();
    let client = connect_client(&connection).await?;

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
                let name: String = row.try_get("datname").map_err(sanitize_pg_error)?;
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

#[tauri::command]
async fn select_database(
    params: SelectDatabaseParams,
    state: State<'_, AppState>,
) -> Result<ConnectionStatus, String> {
    let next_database = params.database.trim();
    if next_database.is_empty() {
        return Err("Database name is required".to_string());
    }

    let (connection, _) = get_connection_snapshot(&state).await?;
    if connection.database == next_database {
        return connection_status(state).await;
    }

    let next_connection = ConnectionInput {
        database: next_database.to_string(),
        ..connection
    };

    let client = connect_client(&next_connection).await?;
    let server_version = get_server_version(&client).await?;
    client
        .batch_execute(&format!("set statement_timeout = {QUERY_TIMEOUT_MS}"))
        .await
        .map_err(sanitize_pg_error)?;

    {
        let mut active = state.connection.lock().await;
        *active = Some(next_connection.clone());
    }
    {
        let mut version = state.server_version.lock().await;
        *version = server_version.clone();
    }

    Ok(ConnectionStatus {
        connected: true,
        name: next_connection.name,
        host: next_connection.host,
        port: next_connection.port,
        database: next_connection.database,
        user: next_connection.user,
        server_version,
    })
}

#[tauri::command]
async fn apply_table_changes(
    params: ApplyTableChangesParams,
    state: State<'_, AppState>,
) -> Result<ApplyTableChangesResponse, String> {
    let schema = params.schema.trim();
    let table = params.table.trim();
    if schema.is_empty() || table.is_empty() {
        return Err("Schema and table are required".to_string());
    }

    let (connection, _) = get_connection_snapshot(&state).await?;
    let mut client = connect_client(&connection).await?;
    client
        .batch_execute(&format!("set statement_timeout = {QUERY_TIMEOUT_MS}"))
        .await
        .map_err(sanitize_pg_error)?;

    let mut updated = 0usize;
    let mut deleted = 0usize;
    let mut inserted = 0usize;
    let mut updated_rows: Vec<UpdatedRowCtid> = Vec::new();

    let tx = client.transaction().await.map_err(sanitize_pg_error)?;
    let safe_table = format!("{}.{}", quote_ident(schema), quote_ident(table));

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
            .map(|(column, value)| format!("{} = {}", quote_ident(column), value_to_sql_literal(value)))
            .collect::<Vec<_>>()
            .join(", ");
        let query = format!(
            "update {safe_table} as t set {set_clause} where t.ctid = '{}'::tid returning t.ctid::text as _querycastle_ctid, to_jsonb(t)::text as _querycastle_row_json",
            escape_sql_string(update.ctid.as_str())
        );
        let updated_row = tx.query_opt(query.as_str(), &[]).await.map_err(sanitize_pg_error)?;
        let Some(updated_row) = updated_row else {
            return Err(format!(
                "Could not update row with ctid {}. It may have changed. Refresh and retry.",
                update.ctid
            ));
        };
        let new_ctid: String = updated_row.try_get("_querycastle_ctid").map_err(sanitize_pg_error)?;
        let row_json: String = updated_row.try_get("_querycastle_row_json").map_err(sanitize_pg_error)?;
        let values: HashMap<String, Value> = serde_json::from_str(&row_json).map_err(sanitize_error)?;
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
            escape_sql_string(ctid)
        );
        let affected = tx.execute(query.as_str(), &[]).await.map_err(sanitize_pg_error)?;
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
            .map(|(column, _)| quote_ident(column))
            .collect::<Vec<_>>()
            .join(", ");
        let values = entries
            .iter()
            .map(|(_, value)| value_to_sql_literal(value))
            .collect::<Vec<_>>()
            .join(", ");

        let query = format!("insert into {safe_table} ({cols}) values ({values})");
        tx.execute(query.as_str(), &[]).await.map_err(sanitize_pg_error)?;
        inserted += 1;
    }

    tx.commit().await.map_err(sanitize_pg_error)?;

    Ok(ApplyTableChangesResponse {
        ok: true,
        updated,
        deleted,
        inserted,
        updated_rows,
    })
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            connection_status,
            test_connection,
            connect,
            disconnect,
            run_query,
            get_database_explorer,
            list_databases,
            select_database,
            apply_table_changes,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}


