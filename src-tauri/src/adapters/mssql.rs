use std::collections::HashMap;
use std::time::Duration;

use deadpool::managed::{Manager, Metrics, Pool, RecycleError, RecycleResult};
use futures_util::TryStreamExt;
use serde_json::Value;
use tiberius::{AuthMethod, Client, ColumnData, Config, EncryptionLevel, QueryItem, Row};
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

use crate::core::error::DbError;
use crate::core::limits::{MAX_QUERY_ROWS, QUERY_TIMEOUT_MS};
use crate::core::sql;
use crate::core::types::{
    ApplyTableChangesParams, ApplyTableChangesResponse, ConnectionInput, DatabaseColumn,
    DatabaseExplorer, DatabaseForeignKey, DatabaseIndex, DatabaseRoutine, DatabaseSchema,
    DatabaseSequence, DatabaseTable, DatabaseTrigger, ObjectDefinition,
    ObjectDefinitionParams, QueryResultPayload, UpdatedRow,
};
use tokio_util::sync::CancellationToken;

pub type MssqlClient = Client<Compat<TcpStream>>;
pub type MssqlPool = Pool<MssqlManager>;

#[derive(Debug, Clone)]
pub struct MssqlManager {
    config: Config,
}

impl Manager for MssqlManager {
    type Type = MssqlClient;
    type Error = DbError;

    async fn create(&self) -> Result<MssqlClient, DbError> {
        let tcp = TcpStream::connect(self.config.get_addr())
            .await
            .map_err(|e| DbError::connection(format!("Failed to connect to SQL Server: {e}")))?;
        tcp.set_nodelay(true)
            .map_err(|e| DbError::connection(format!("Failed to set TCP_NODELAY: {e}")))?;
        Ok(Client::connect(self.config.clone(), tcp.compat_write()).await?)
    }

    async fn recycle(&self, conn: &mut MssqlClient, _: &Metrics) -> RecycleResult<DbError> {
        conn.simple_query("select 1")
            .await
            .map_err(|e| RecycleError::Message(e.to_string().into()))?;
        Ok(())
    }
}

pub fn pool_from_input(connection: &ConnectionInput) -> Result<MssqlPool, DbError> {
    let config = config_from_input(connection)?;
    Pool::builder(MssqlManager { config })
        .max_size(5)
        .runtime(deadpool::Runtime::Tokio1)
        .create_timeout(Some(Duration::from_secs(5)))
        .build()
        .map_err(|e| DbError::connection(format!("Failed to build SQL Server pool: {e}")))
}

fn config_from_input(connection: &ConnectionInput) -> Result<Config, DbError> {
    let raw = connection.connection_string.trim();
    if connection.use_connection_string && !raw.is_empty() {
        let lower = raw.to_ascii_lowercase();
        let mut cfg = if lower.starts_with("jdbc:sqlserver:") {
            Config::from_jdbc_string(raw)
                .map_err(|e| DbError::validation(format!("Invalid JDBC connection string: {e}")))?
        } else if lower.starts_with("sqlserver://") || lower.starts_with("mssql://") {
            return Ok(config_from_fields(connection));
        } else {
            Config::from_ado_string(raw)
                .map_err(|e| DbError::validation(format!("Invalid ADO.NET connection string: {e}")))?
        };
        cfg.application_name("querycastle");
        Ok(cfg)
    } else {
        Ok(config_from_fields(connection))
    }
}

fn config_from_fields(connection: &ConnectionInput) -> Config {
    let mut cfg = Config::new();
    cfg.host(&connection.host);
    cfg.port(connection.port);
    cfg.database(&connection.database);
    cfg.authentication(AuthMethod::sql_server(&connection.user, &connection.password));
    cfg.application_name("querycastle");
    cfg.encryption(if connection.ssl {
        EncryptionLevel::Required
    } else {
        EncryptionLevel::Off
    });
    if connection.ssl_insecure {
        cfg.trust_cert();
    }
    cfg
}

fn quote(name: &str) -> String {
    sql::quote_ident_mssql(name)
}

fn json_to_text(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::Bool(v) => Some(if *v { "1".to_string() } else { "0".to_string() }),
        Value::Number(n) => Some(n.to_string()),
        Value::String(s) => Some(s.clone()),
        Value::Array(_) | Value::Object(_) => Some(value.to_string()),
    }
}

fn row_str(row: &Row, idx: impl Into<MssqlIdx>) -> Option<String> {
    match idx.into() {
        MssqlIdx::Pos(i) => row.try_get::<&str, _>(i).ok().flatten().map(|value| value.to_string()),
        MssqlIdx::Name(name) => row
            .try_get::<&str, _>(name.as_str())
            .ok()
            .flatten()
            .map(|value| value.to_string()),
    }
}

fn row_column_data<'a>(row: &'a Row, idx: MssqlIdx) -> Option<ColumnData<'a>> {
    match idx {
        MssqlIdx::Pos(i) => row.try_get::<ColumnData<'_>, _>(i).ok().flatten(),
        MssqlIdx::Name(name) => row.try_get::<ColumnData<'_>, _>(name.as_str()).ok().flatten(),
    }
}

fn column_data_is_true(data: ColumnData<'_>) -> bool {
    match data {
        ColumnData::Bit(Some(value)) => value,
        ColumnData::U8(Some(value)) => value != 0,
        ColumnData::I16(Some(value)) => value != 0,
        ColumnData::I32(Some(value)) => value != 0,
        ColumnData::I64(Some(value)) => value != 0,
        _ => false,
    }
}

fn row_bool(row: &Row, idx: impl Into<MssqlIdx>) -> bool {
    row_column_data(row, idx.into()).is_some_and(column_data_is_true)
}

enum MssqlIdx {
    Pos(usize),
    Name(String),
}

impl From<usize> for MssqlIdx {
    fn from(value: usize) -> Self {
        Self::Pos(value)
    }
}

impl From<&str> for MssqlIdx {
    fn from(value: &str) -> Self {
        Self::Name(value.to_string())
    }
}

fn column_data_to_json(data: ColumnData<'_>) -> Value {
    match data {
        ColumnData::U8(v) => v.map(|n| Value::Number(n.into())).unwrap_or(Value::Null),
        ColumnData::I16(v) => v.map(|n| Value::Number(n.into())).unwrap_or(Value::Null),
        ColumnData::I32(v) => v.map(|n| Value::Number(n.into())).unwrap_or(Value::Null),
        ColumnData::I64(v) => v.map(|n| Value::Number(n.into())).unwrap_or(Value::Null),
        ColumnData::F32(v) => v
            .and_then(|n| serde_json::Number::from_f64(n as f64).map(Value::Number))
            .unwrap_or(Value::Null),
        ColumnData::F64(v) => v
            .and_then(|n| serde_json::Number::from_f64(n).map(Value::Number))
            .unwrap_or(Value::Null),
        ColumnData::Bit(v) => v.map(Value::Bool).unwrap_or(Value::Null),
        ColumnData::String(v) => v.map(|s| Value::String(s.into_owned())).unwrap_or(Value::Null),
        ColumnData::Guid(v) => v.map(|g| Value::String(g.to_string())).unwrap_or(Value::Null),
        ColumnData::Binary(v) => v
            .map(|bytes| {
                let mut out = String::from("0x");
                for byte in bytes.iter() {
                    out.push_str(&format!("{byte:02x}"));
                }
                Value::String(out)
            })
            .unwrap_or(Value::Null),
        ColumnData::Numeric(v) => v.map(|n| Value::String(n.to_string())).unwrap_or(Value::Null),
        ColumnData::DateTime(v) => v.map(|d| Value::String(format!("{d:?}"))).unwrap_or(Value::Null),
        ColumnData::SmallDateTime(v) => v.map(|d| Value::String(format!("{d:?}"))).unwrap_or(Value::Null),
        ColumnData::DateTime2(v) => v.map(|d| Value::String(format!("{d:?}"))).unwrap_or(Value::Null),
        ColumnData::DateTimeOffset(v) => v.map(|d| Value::String(format!("{d:?}"))).unwrap_or(Value::Null),
        ColumnData::Time(v) => v.map(|d| Value::String(format!("{d:?}"))).unwrap_or(Value::Null),
        ColumnData::Date(v) => v.map(|d| Value::String(format!("{d:?}"))).unwrap_or(Value::Null),
        ColumnData::Xml(v) => v.map(|xml| Value::String(xml.to_string())).unwrap_or(Value::Null),
    }
}

fn cell_to_json(row: &Row, index: usize) -> Value {
    match row.try_get::<ColumnData<'_>, _>(index) {
        Ok(Some(data)) => column_data_to_json(data),
        Ok(None) | Err(_) => Value::Null,
    }
}

fn row_to_map(row: &Row) -> HashMap<String, Value> {
    let mut mapped = HashMap::new();
    for (index, column) in row.columns().iter().enumerate() {
        mapped.insert(column.name().to_string(), cell_to_json(row, index));
    }
    mapped
}

fn mssql_md5_expr(payload: &str) -> String {
    format!("convert(varchar(32), hashbytes('MD5', {payload}), 2)")
}

fn mssql_hash_payload(parts: &[String]) -> String {
    if parts.len() == 1 {
        parts[0].clone()
    } else {
        format!("concat_ws(char(31), {})", parts.join(", "))
    }
}

fn mssql_row_hash_expression(columns: &[String]) -> String {
    let parts = columns
        .iter()
        .map(|column| {
            format!(
                "coalesce(convert(varchar(max), {}), '__querycastle_null__')",
                quote(column)
            )
        })
        .collect::<Vec<_>>();
    mssql_md5_expr(&mssql_hash_payload(&parts))
}

pub async fn server_version(pool: &MssqlPool) -> Result<Option<String>, DbError> {
    let mut conn = pool.get().await?;
    let mut stream = conn.simple_query("select @@version as version").await?;
    while let Some(item) = stream.try_next().await? {
        if let QueryItem::Row(row) = item {
            if let Some(version) = row_str(&row, "version").or_else(|| row_str(&row, 0)) {
                return Ok(Some(version));
            }
        }
    }
    Ok(None)
}

pub async fn run_query(
    pool: &MssqlPool,
    sql_text: &str,
    cancel: CancellationToken,
) -> Result<QueryResultPayload, DbError> {
    let mut conn = pool.get().await?;
    let started = std::time::Instant::now();
    let sql_text = sql_text.to_string();
    let fut = async {
        let mut stream = conn.simple_query(sql_text).await?;
        let mut columns: Vec<String> = Vec::new();
        let mut rows: Vec<HashMap<String, Value>> = Vec::new();
        let mut truncated = false;
        while let Some(item) = stream.try_next().await? {
            match item {
                QueryItem::Metadata(meta) => {
                    if columns.is_empty() {
                        columns = meta
                            .columns()
                            .iter()
                            .map(|column| column.name().to_string())
                            .collect();
                    }
                }
                QueryItem::Row(row) => {
                    if columns.is_empty() {
                        columns = row
                            .columns()
                            .iter()
                            .map(|column| column.name().to_string())
                            .collect();
                    }
                    if rows.len() >= MAX_QUERY_ROWS {
                        truncated = true;
                        continue;
                    }
                    rows.push(row_to_map(&row));
                }
            }
        }
        let row_count = rows.len();
        Ok(QueryResultPayload {
            columns,
            rows,
            row_count,
            duration_ms: started.elapsed().as_millis(),
            truncated,
        })
    };

    match tokio::select! {
        _ = cancel.cancelled() => Err(DbError::cancelled()),
        result = tokio::time::timeout(Duration::from_millis(QUERY_TIMEOUT_MS), fut) => {
            match result {
                Ok(result) => result,
                Err(_) => Err(DbError::Timeout {
                    message: format!("Query exceeded {QUERY_TIMEOUT_MS}ms"),
                }),
            }
        }
    } {
        Ok(payload) => Ok(payload),
        Err(_) if cancel.is_cancelled() => Err(DbError::cancelled()),
        Err(err) => Err(err),
    }
}

pub async fn get_database_explorer(pool: &MssqlPool) -> Result<DatabaseExplorer, DbError> {
    let mut conn = pool.get().await?;
    let current_database = scalar_string(
        &mut conn,
        "select db_name() as name",
    )
    .await?
    .unwrap_or_default();

    let mut stream = conn
        .simple_query(
            r#"
            select
                s.name as schema_name,
                o.name as table_name,
                case when o.type = 'V' then 'view' else 'table' end as kind,
                c.name as column_name,
                ty.name as data_type,
                c.is_nullable as is_nullable,
                case when i.is_primary_key = 1 then 1 else 0 end as is_primary,
                case when c.default_object_id <> 0 or c.is_identity = 1 then 1 else 0 end as has_default
            from sys.objects o
            join sys.schemas s on s.schema_id = o.schema_id
            join sys.columns c on c.object_id = o.object_id
            join sys.types ty on ty.user_type_id = c.user_type_id
            left join sys.index_columns ic
                on ic.object_id = c.object_id and ic.column_id = c.column_id
            left join sys.indexes i
                on i.object_id = ic.object_id and i.index_id = ic.index_id and i.is_primary_key = 1
            where o.type in ('U', 'V')
                and s.name not in ('sys', 'INFORMATION_SCHEMA', 'guest', 'db_owner', 'db_accessadmin',
                    'db_securityadmin', 'db_ddladmin', 'db_backupoperator', 'db_datareader',
                    'db_datawriter', 'db_denydatareader', 'db_denydatawriter')
            order by s.name, o.name, c.column_id
            "#,
        )
        .await?;

    let mut schema_map: HashMap<String, DatabaseSchema> = HashMap::new();
    let mut table_map: HashMap<String, DatabaseTable> = HashMap::new();

    while let Some(item) = stream.try_next().await? {
        let QueryItem::Row(row) = item else {
            continue;
        };
        let schema_name = row_str(&row, "schema_name").unwrap_or_else(|| "dbo".to_string());
        let table_name = row_str(&row, "table_name").unwrap_or_default();
        let kind = row_str(&row, "kind").unwrap_or_else(|| "table".to_string());
        schema_map
            .entry(schema_name.clone())
            .or_insert_with(|| DatabaseSchema::new(schema_name.clone()));
        let table_key = format!("{schema_name}.{table_name}");
        table_map.entry(table_key.clone()).or_insert_with(|| {
            DatabaseTable::new(schema_name.clone(), table_name.clone(), kind)
        });
        if let Some(column_name) = row_str(&row, "column_name") {
            if let Some(table) = table_map.get_mut(&table_key) {
                if table.columns.iter().any(|column| column.name == column_name) {
                    if row_bool(&row, "is_primary") {
                        if let Some(column) = table.columns.iter_mut().find(|column| column.name == column_name) {
                            column.is_primary = true;
                        }
                    }
                    continue;
                }
                table.columns.push(DatabaseColumn {
                    name: column_name,
                    data_type: row_str(&row, "data_type").unwrap_or_else(|| "unknown".to_string()),
                    not_null: !row.try_get::<bool, _>("is_nullable").ok().flatten().unwrap_or(true),
                    is_primary: row_bool(&row, "is_primary"),
                    has_default: row_bool(&row, "has_default"),
                });
            }
        }
    }
    drop(stream);

    load_foreign_keys(&mut conn, &mut table_map).await?;
    load_indexes(&mut conn, &mut table_map).await?;
    apply_primary_keys_from_indexes(&mut table_map);
    load_triggers(&mut conn, &mut table_map).await?;
    load_routines(&mut conn, &mut schema_map).await?;
    load_sequences(&mut conn, &mut schema_map).await?;

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
        schema.sequences.sort_by(|a, b| a.name.cmp(&b.name));
    }

    Ok(DatabaseExplorer {
        database: current_database,
        schemas,
    })
}

async fn scalar_string(conn: &mut MssqlClient, sql_text: &str) -> Result<Option<String>, DbError> {
    let mut stream = conn.simple_query(sql_text).await?;
    while let Some(item) = stream.try_next().await? {
        if let QueryItem::Row(row) = item {
            return Ok(row_str(&row, 0));
        }
    }
    Ok(None)
}

async fn load_foreign_keys(
    conn: &mut MssqlClient,
    table_map: &mut HashMap<String, DatabaseTable>,
) -> Result<(), DbError> {
    let mut stream = conn
        .simple_query(
            r#"
            select
                schema_name(t.schema_id) as table_schema,
                t.name as table_name,
                col.name as column_name,
                schema_name(rt.schema_id) as referenced_schema,
                rt.name as referenced_table,
                rcol.name as referenced_column
            from sys.foreign_key_columns fkc
            join sys.tables t on t.object_id = fkc.parent_object_id
            join sys.columns col on col.object_id = fkc.parent_object_id and col.column_id = fkc.parent_column_id
            join sys.tables rt on rt.object_id = fkc.referenced_object_id
            join sys.columns rcol on rcol.object_id = fkc.referenced_object_id and rcol.column_id = fkc.referenced_column_id
            "#,
        )
        .await?;
    while let Some(item) = stream.try_next().await? {
        let QueryItem::Row(row) = item else { continue };
        let schema = row_str(&row, 0).unwrap_or_default();
        let table = row_str(&row, 1).unwrap_or_default();
        let key = format!("{schema}.{table}");
        if let Some(table) = table_map.get_mut(&key) {
            table.foreign_keys.push(DatabaseForeignKey {
                column: row_str(&row, 2).unwrap_or_default(),
                referenced_schema: row_str(&row, 3).unwrap_or_default(),
                referenced_table: row_str(&row, 4).unwrap_or_default(),
                referenced_column: row_str(&row, 5).unwrap_or_default(),
            });
        }
    }
    Ok(())
}

fn apply_primary_keys_from_indexes(table_map: &mut HashMap<String, DatabaseTable>) {
    for table in table_map.values_mut() {
        let pk_names: Vec<String> = table
            .indexes
            .iter()
            .filter(|index| index.is_primary)
            .flat_map(|index| {
                index
                    .columns
                    .split(',')
                    .map(|name| name.trim().to_string())
                    .filter(|name| !name.is_empty())
            })
            .collect();
        if pk_names.is_empty() {
            continue;
        }
        for column in &mut table.columns {
            if pk_names
                .iter()
                .any(|name| name.eq_ignore_ascii_case(&column.name))
            {
                column.is_primary = true;
            }
        }
    }
}

async fn load_indexes(
    conn: &mut MssqlClient,
    table_map: &mut HashMap<String, DatabaseTable>,
) -> Result<(), DbError> {
    let mut stream = conn
        .simple_query(
            r#"
            select
                schema_name(t.schema_id) as table_schema,
                t.name as table_name,
                i.name as index_name,
                i.is_unique as is_unique,
                i.is_primary_key as is_primary,
                col.name as column_name,
                ic.key_ordinal as key_ordinal
            from sys.indexes i
            join sys.tables t on t.object_id = i.object_id
            join sys.index_columns ic on ic.object_id = i.object_id and ic.index_id = i.index_id
            join sys.columns col on col.object_id = ic.object_id and col.column_id = ic.column_id
            where i.name is not null
            order by t.name, i.name, ic.key_ordinal
            "#,
        )
        .await?;
    let mut grouped: HashMap<(String, String, String), (bool, bool, Vec<String>)> = HashMap::new();
    while let Some(item) = stream.try_next().await? {
        let QueryItem::Row(row) = item else { continue };
        let schema = row_str(&row, 0).unwrap_or_default();
        let table = row_str(&row, 1).unwrap_or_default();
        let name = row_str(&row, 2).unwrap_or_default();
        let unique = row_bool(&row, 3);
        let is_primary = row_bool(&row, 4);
        let column = row_str(&row, 5).unwrap_or_default();
        let entry = grouped.entry((schema, table, name)).or_insert((unique, is_primary, Vec::new()));
        entry.2.push(column);
    }
    for ((schema, table, name), (unique, is_primary, columns)) in grouped {
        let key = format!("{schema}.{table}");
        if let Some(table) = table_map.get_mut(&key) {
            table.indexes.push(DatabaseIndex {
                name,
                columns: columns.join(", "),
                unique,
                is_primary,
                definition: None,
            });
        }
    }
    Ok(())
}

async fn load_triggers(
    conn: &mut MssqlClient,
    table_map: &mut HashMap<String, DatabaseTable>,
) -> Result<(), DbError> {
    let mut stream = conn
        .simple_query(
            r#"
            select
                schema_name(t.schema_id) as table_schema,
                t.name as table_name,
                tr.name as trigger_name,
                object_definition(tr.object_id) as definition
            from sys.triggers tr
            join sys.tables t on t.object_id = tr.parent_id
            "#,
        )
        .await?;
    while let Some(item) = stream.try_next().await? {
        let QueryItem::Row(row) = item else { continue };
        let schema = row_str(&row, 0).unwrap_or_default();
        let table = row_str(&row, 1).unwrap_or_default();
        let key = format!("{schema}.{table}");
        if let Some(table) = table_map.get_mut(&key) {
            table.triggers.push(DatabaseTrigger {
                name: row_str(&row, 2).unwrap_or_default(),
                definition: row_str(&row, 3),
            });
        }
    }
    Ok(())
}

async fn load_routines(
    conn: &mut MssqlClient,
    schema_map: &mut HashMap<String, DatabaseSchema>,
) -> Result<(), DbError> {
    let mut stream = conn
        .simple_query(
            r#"
            select
                schema_name(o.schema_id) as schema_name,
                o.name as name,
                case when o.type in ('P', 'PC') then 'procedure' else 'function' end as kind,
                o.object_id as object_id
            from sys.objects o
            where o.type in ('P', 'PC', 'FN', 'IF', 'TF', 'FS', 'FT')
                and schema_name(o.schema_id) not in ('sys')
            order by o.name
            "#,
        )
        .await?;
    while let Some(item) = stream.try_next().await? {
        let QueryItem::Row(row) = item else { continue };
        let schema_name = row_str(&row, 0).unwrap_or_else(|| "dbo".to_string());
        let name = row_str(&row, 1).unwrap_or_default();
        let kind = row_str(&row, 2).unwrap_or_else(|| "procedure".to_string());
        let object_id = row
            .try_get::<i32, _>(3)
            .ok()
            .flatten()
            .map(|id| id.to_string())
            .unwrap_or_else(|| format!("{schema_name}.{name}"));
        schema_map
            .entry(schema_name.clone())
            .or_insert_with(|| DatabaseSchema::new(schema_name.clone()));
        if let Some(schema) = schema_map.get_mut(&schema_name) {
            schema.routines.push(DatabaseRoutine {
                schema: schema_name,
                name,
                kind,
                identity_args: String::new(),
                language: Some("tsql".to_string()),
                return_type: None,
                object_id,
            });
        }
    }
    Ok(())
}

async fn load_sequences(
    conn: &mut MssqlClient,
    schema_map: &mut HashMap<String, DatabaseSchema>,
) -> Result<(), DbError> {
    let mut stream = conn
        .simple_query(
            r#"
            select schema_name(schema_id) as schema_name, name, type_desc
            from sys.sequences
            "#,
        )
        .await?;
    while let Some(item) = stream.try_next().await? {
        let QueryItem::Row(row) = item else { continue };
        let schema_name = row_str(&row, 0).unwrap_or_else(|| "dbo".to_string());
        schema_map
            .entry(schema_name.clone())
            .or_insert_with(|| DatabaseSchema::new(schema_name.clone()));
        if let Some(schema) = schema_map.get_mut(&schema_name) {
            schema.sequences.push(DatabaseSequence {
                schema: schema_name,
                name: row_str(&row, 1).unwrap_or_default(),
                data_type: row_str(&row, 2),
            });
        }
    }
    Ok(())
}

pub async fn get_object_definition(
    pool: &MssqlPool,
    params: &ObjectDefinitionParams,
) -> Result<ObjectDefinition, DbError> {
    let mut conn = pool.get().await?;
    let schema = params.schema.trim();
    let name = params.name.trim();
    if name.is_empty() {
        return Err(DbError::validation("Name is required"));
    }
    let qualified = if schema.is_empty() {
        name.to_string()
    } else {
        format!("{schema}.{name}")
    };
    let sql_text = if let Some(object_id) = params.object_id.as_deref().filter(|value| !value.is_empty()) {
        if let Ok(oid) = object_id.parse::<i32>() {
            let query = format!("select object_definition({oid}) as definition");
            scalar_string(&mut conn, &query).await?
        } else {
            None
        }
    } else {
        None
    };
    let sql_text = match sql_text {
        Some(text) if !text.trim().is_empty() => text,
        _ => {
            let escaped = qualified.replace('\'', "''");
            scalar_string(
                &mut conn,
                &format!("select object_definition(object_id(N'{escaped}')) as definition"),
            )
            .await?
            .unwrap_or_default()
        }
    };
    if sql_text.trim().is_empty() {
        return Err(DbError::validation("Could not load object definition"));
    }
    Ok(ObjectDefinition {
        title: name.to_string(),
        sql: if sql_text.trim_end().ends_with(';') {
            sql_text
        } else {
            format!("{};", sql_text.trim_end())
        },
    })
}

pub async fn list_databases(pool: &MssqlPool) -> Result<Vec<String>, DbError> {
    let mut conn = pool.get().await?;
    let mut stream = conn
        .simple_query(
            "select name from sys.databases where state_desc = 'ONLINE' order by name",
        )
        .await?;
    let mut names = Vec::new();
    while let Some(item) = stream.try_next().await? {
        if let QueryItem::Row(row) = item {
            if let Some(name) = row_str(&row, 0) {
                names.push(name);
            }
        }
    }
    drop(stream);
    if names.is_empty() {
        if let Some(current) = scalar_string(&mut conn, "select db_name()").await? {
            names.push(current);
        }
    }
    Ok(names)
}

pub async fn apply_table_changes(
    pool: &MssqlPool,
    params: &ApplyTableChangesParams,
) -> Result<ApplyTableChangesResponse, DbError> {
    let schema = params.schema.trim();
    let table = params.table.trim();
    if schema.is_empty() || table.is_empty() {
        return Err(DbError::validation("Schema and table are required"));
    }

    let mut conn = pool.get().await?;
    conn.simple_query("begin transaction").await?;

    let apply_result = apply_table_changes_inner(&mut conn, schema, table, params).await;
    match apply_result {
        Ok(result) => {
            conn.simple_query("commit transaction").await?;
            Ok(result)
        }
        Err(err) => {
            let _ = conn.simple_query("rollback transaction").await;
            Err(err)
        }
    }
}

async fn apply_table_changes_inner(
    conn: &mut MssqlClient,
    schema: &str,
    table: &str,
    params: &ApplyTableChangesParams,
) -> Result<ApplyTableChangesResponse, DbError> {
    let safe_table = format!("{}.{}", quote(schema), quote(table));
    let pk_columns = load_pk_columns(conn, schema, table).await?;
    if pk_columns.is_empty() {
        return Err(DbError::validation(
            "SQL Server table editing requires a PRIMARY KEY. This table has none.",
        ));
    }
    let row_hash_expr = mssql_row_hash_expression(&pk_columns);

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
            .enumerate()
            .map(|(index, (column, _))| format!("{} = @P{}", quote(column), index + 1))
            .collect::<Vec<_>>()
            .join(", ");
        let hash_param = entries.len() + 1;
        let sql_text = format!(
            "update {safe_table} set {set_clause} where {row_hash_expr} = @P{hash_param}"
        );
        let mut owned: Vec<Option<String>> = entries.iter().map(|(_, value)| json_to_text(value)).collect();
        owned.push(Some(update.row_id.clone()));
        let refs: Vec<&dyn tiberius::ToSql> = owned.iter().map(|value| value as &dyn tiberius::ToSql).collect();
        let result = conn.execute(&sql_text, refs.as_slice()).await?;
        if result.total() == 0 {
            return Err(DbError::NotFound(format!(
                "Could not update row {}. It may have changed. Refresh and retry.",
                update.row_id
            )));
        }

        let select_sql = format!("select * from {safe_table} where {row_hash_expr} = @P1");
        // After PK edits the hash changes; reload using new values when possible.
        let new_hash = compute_hash(conn, &pk_columns, &merge_update_values(update)).await?;
        let mut stream = conn.query(&select_sql, &[&new_hash]).await?;
        let mut new_values = HashMap::new();
        while let Some(item) = stream.try_next().await? {
            if let QueryItem::Row(row) = item {
                new_values = row_to_map(&row);
                break;
            }
        }
        drop(stream);
        updated_rows.push(UpdatedRow {
            old_row_id: update.row_id.clone(),
            new_row_id: new_hash,
            values: new_values,
        });
        updated += 1;
    }

    for row_id in &params.changes.deletes {
        let sql_text = format!("delete from {safe_table} where {row_hash_expr} = @P1");
        let result = conn.execute(&sql_text, &[&row_id.as_str()]).await?;
        if result.total() == 0 {
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
        let cols = entries.iter().map(|(column, _)| quote(column)).collect::<Vec<_>>().join(", ");
        let placeholders = entries
            .iter()
            .enumerate()
            .map(|(index, _)| format!("@P{}", index + 1))
            .collect::<Vec<_>>()
            .join(", ");
        let owned: Vec<Option<String>> = entries.iter().map(|(_, value)| json_to_text(value)).collect();
        let refs: Vec<&dyn tiberius::ToSql> = owned.iter().map(|value| value as &dyn tiberius::ToSql).collect();
        let sql_text = format!("insert into {safe_table} ({cols}) values ({placeholders})");
        conn.execute(&sql_text, refs.as_slice()).await?;
        inserted += 1;
    }

    Ok(ApplyTableChangesResponse {
        ok: true,
        updated,
        deleted,
        inserted,
        updated_rows,
    })
}

fn merge_update_values(update: &crate::core::types::TableUpdatePayload) -> HashMap<String, Value> {
    update.values.clone()
}

async fn compute_hash(
    conn: &mut MssqlClient,
    pk_columns: &[String],
    values: &HashMap<String, Value>,
) -> Result<String, DbError> {
    let parts = pk_columns
        .iter()
        .enumerate()
        .map(|(index, _column)| {
            format!(
                "coalesce(convert(varchar(max), @P{}), '__querycastle_null__')",
                index + 1
            )
        })
        .collect::<Vec<_>>();
    let sql_text = format!("select {} as h", mssql_md5_expr(&mssql_hash_payload(&parts)));
    let owned: Vec<Option<String>> = pk_columns
        .iter()
        .map(|column| json_to_text(values.get(column).unwrap_or(&Value::Null)))
        .collect();
    let refs: Vec<&dyn tiberius::ToSql> = owned.iter().map(|value| value as &dyn tiberius::ToSql).collect();
    let mut stream = conn.query(&sql_text, refs.as_slice()).await?;
    while let Some(item) = stream.try_next().await? {
        if let QueryItem::Row(row) = item {
            if let Some(hash) = row_str(&row, 0) {
                return Ok(hash);
            }
        }
    }
    Err(DbError::internal("Could not compute updated SQL Server row identity."))
}

async fn load_pk_columns(
    conn: &mut MssqlClient,
    schema: &str,
    table: &str,
) -> Result<Vec<String>, DbError> {
    let sql_text = r#"
        select c.name
        from sys.indexes i
        join sys.index_columns ic on ic.object_id = i.object_id and ic.index_id = i.index_id
        join sys.columns c on c.object_id = ic.object_id and c.column_id = ic.column_id
        join sys.tables t on t.object_id = i.object_id
        join sys.schemas s on s.schema_id = t.schema_id
        where i.is_primary_key = 1 and s.name = @P1 and t.name = @P2
        order by ic.key_ordinal
    "#;
    let mut stream = conn.query(sql_text, &[&schema, &table]).await?;
    let mut columns = Vec::new();
    while let Some(item) = stream.try_next().await? {
        if let QueryItem::Row(row) = item {
            if let Some(name) = row_str(&row, 0) {
                columns.push(name);
            }
        }
    }
    Ok(columns)
}
