use bytes::BytesMut;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use tokio_postgres::types::{to_sql_checked, Format, IsNull, ToSql, Type};
use tokio_postgres::Transaction;

use crate::core::error::DbError;
use crate::core::limits::{apply_select_row_cap, MAX_QUERY_ROWS, QUERY_TIMEOUT_MS};
use crate::core::sql;
use crate::core::types::{
    ApplyTableChangesParams, ApplyTableChangesResponse, DatabaseColumn, DatabaseExplorer,
    DatabaseForeignKey, DatabaseIndex, DatabaseRoutine, DatabaseSchema, DatabaseSequence,
    DatabaseTable, DatabaseTrigger, DatabaseType, ObjectDefinition, ObjectDefinitionParams,
    QueryResultPayload, UpdatedRow,
};

fn json_to_pg_text(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::Bool(v) => Some(if *v { "true".to_string() } else { "false".to_string() }),
        Value::Number(n) => Some(n.to_string()),
        Value::String(s) => Some(s.clone()),
        Value::Array(_) | Value::Object(_) => Some(value.to_string()),
    }
}

/// Grid edits are JSON. Postgres prepared params are typed (int4, uuid, tid, ...),
/// so we send every value as text and let the server coerce it.
#[derive(Debug)]
struct PgTextParam(Option<String>);

impl ToSql for PgTextParam {
    fn to_sql(
        &self,
        _ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        match &self.0 {
            None => Ok(IsNull::Yes),
            Some(value) => {
                out.extend_from_slice(value.as_bytes());
                Ok(IsNull::No)
            }
        }
    }

    fn accepts(_ty: &Type) -> bool {
        true
    }

    fn encode_format(&self, _ty: &Type) -> Format {
        Format::Text
    }

    to_sql_checked!();
}

fn json_to_pg_param(value: &Value) -> PgTextParam {
    PgTextParam(json_to_pg_text(value))
}

fn pg_param_refs(params: &[PgTextParam]) -> Vec<&(dyn ToSql + Sync)> {
    params
        .iter()
        .map(|value| value as &(dyn ToSql + Sync))
        .collect()
}

async fn exec_untyped(
    tx: &Transaction<'_>,
    sql: &str,
    params: &[PgTextParam],
) -> Result<u64, DbError> {
    let bind_refs = pg_param_refs(params);
    Ok(tx.execute(sql, bind_refs.as_slice()).await?)
}

async fn query_opt_untyped(
    tx: &Transaction<'_>,
    sql: &str,
    params: &[PgTextParam],
) -> Result<Option<tokio_postgres::Row>, DbError> {
    let bind_refs = pg_param_refs(params);
    Ok(tx.query_opt(sql, bind_refs.as_slice()).await?)
}

async fn set_statement_timeout(client: &tokio_postgres::Client) -> Result<(), DbError> {
    client
        .batch_execute(&format!("set statement_timeout = {QUERY_TIMEOUT_MS}"))
        .await?;
    Ok(())
}

pub async fn server_version(pool: &deadpool_postgres::Pool) -> Result<Option<String>, DbError> {
    let client = pool.get().await?;
    let row = client
        .query_one("select current_setting('server_version') as server_version", &[])
        .await?;
    let version: Option<String> = row.try_get("server_version")?;
    Ok(version)
}

pub async fn run_query(pool: &deadpool_postgres::Pool, sql: &str) -> Result<QueryResultPayload, DbError> {
    let client = pool.get().await?;
    set_statement_timeout(&client).await?;

    let started = std::time::Instant::now();
    let sql = apply_select_row_cap(sql);
    let messages = client.simple_query(sql.as_ref()).await?;

    let mut columns: Vec<String> = Vec::new();
    let mut rows: Vec<HashMap<String, Value>> = Vec::new();
    let mut truncated = false;
    for message in messages {
        if let tokio_postgres::SimpleQueryMessage::Row(row) = message {
            if columns.is_empty() {
                columns = row
                    .columns()
                    .iter()
                    .map(|column| column.name().to_string())
                    .collect();
            }

            if rows.len() >= MAX_QUERY_ROWS {
                truncated = true;
                break;
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
    Ok(QueryResultPayload {
        columns,
        rows,
        row_count,
        duration_ms: started.elapsed().as_millis(),
        truncated,
    })
}

pub async fn get_database_explorer(pool: &deadpool_postgres::Pool) -> Result<DatabaseExplorer, DbError> {
    let client = pool.get().await?;

    let db_row = client
        .query_one("select current_database() as current_database", &[])
        .await
        ?;
    let current_database: String =
        db_row.try_get("current_database")?;

    let rows = client
        .query(
            "
            select
                n.nspname as schema_name,
                c.relname as table_name,
                c.relkind::text as relkind,
                a.attname as column_name,
                pg_catalog.format_type(a.atttypid, a.atttypmod) as data_type,
                a.attnotnull as not_null,
                exists (
                    select 1
                    from pg_catalog.pg_index i
                    where i.indrelid = c.oid
                        and i.indisprimary
                        and a.attnum = any (i.indkey)
                ) as is_primary,
                exists (
                    select 1
                    from pg_catalog.pg_attrdef d
                    where d.adrelid = c.oid
                        and d.adnum = a.attnum
                ) as has_default
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
        ?;

    let mut schema_map: HashMap<String, DatabaseSchema> = HashMap::new();
    let mut table_map: HashMap<String, DatabaseTable> = HashMap::new();

    for row in rows {
        let schema_name: String = row.try_get("schema_name")?;
        let table_name: String = row.try_get("table_name")?;
        let relkind: String = row.try_get("relkind")?;
        let column_name: Option<String> = row.try_get("column_name")?;
        let data_type: Option<String> = row.try_get("data_type")?;
        let not_null: Option<bool> = row.try_get("not_null")?;
        let is_primary: Option<bool> = row.try_get("is_primary")?;
        let has_default: Option<bool> = row.try_get("has_default")?;

        schema_map
            .entry(schema_name.clone())
            .or_insert_with(|| DatabaseSchema::new(schema_name.clone()));

        let table_key = format!("{schema_name}.{table_name}");
        table_map.entry(table_key.clone()).or_insert_with(|| {
            DatabaseTable::new(
                schema_name.clone(),
                table_name.clone(),
                if relkind == "v" || relkind == "m" {
                    "view".to_string()
                } else {
                    "table".to_string()
                },
            )
        });

        if let Some(column) = column_name {
            if let Some(table) = table_map.get_mut(&table_key) {
                table.columns.push(DatabaseColumn {
                    name: column,
                    data_type: data_type.unwrap_or_else(|| "unknown".to_string()),
                    not_null: not_null.unwrap_or(false),
                    is_primary: is_primary.unwrap_or(false),
                    has_default: has_default.unwrap_or(false),
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
        ?;

    for fk in fk_rows {
        let table_schema: String = fk.try_get("table_schema")?;
        let table_name: String = fk.try_get("table_name")?;
        let table_key = format!("{table_schema}.{table_name}");
        if let Some(table) = table_map.get_mut(&table_key) {
            table.foreign_keys.push(DatabaseForeignKey {
                column: fk.try_get("column_name")?,
                referenced_schema: fk.try_get("foreign_table_schema")?,
                referenced_table: fk.try_get("foreign_table_name")?,
                referenced_column: fk.try_get("foreign_column_name")?,
            });
        }
    }

    load_postgres_indexes(&client, &mut table_map).await?;
    load_postgres_triggers(&client, &mut table_map).await?;
    load_postgres_routines(&client, &mut schema_map).await?;
    load_postgres_sequences(&client, &mut schema_map).await?;

    for table in table_map.into_values() {
        if let Some(schema) = schema_map.get_mut(&table.schema) {
            schema.tables.push(table);
        }
    }

    let mut schemas: Vec<DatabaseSchema> = schema_map.into_values().collect();
    schemas.sort_by(|a, b| a.name.cmp(&b.name));
    for schema in &mut schemas {
        schema.tables.sort_by(|a, b| a.name.cmp(&b.name));
        schema.routines.sort_by(|a, b| a.name.cmp(&b.name).then(a.identity_args.cmp(&b.identity_args)));
        schema.sequences.sort_by(|a, b| a.name.cmp(&b.name));
    }

    Ok(DatabaseExplorer {
        database: current_database,
        schemas,
    })
}

pub async fn list_databases(pool: &deadpool_postgres::Pool) -> Result<Vec<String>, DbError> {
    let client = pool.get().await?;

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
        .await
        ?;

    let mut names = Vec::new();
    for row in rows {
        let name: String = row.try_get("datname")?;
        names.push(name);
    }
    if names.is_empty() {
        let db_row = client
            .query_one("select current_database() as current_database", &[])
            .await
            ?;
        let current: String = db_row.try_get("current_database")?;
        Ok(vec![current])
    } else {
        Ok(names)
    }
}

pub async fn apply_table_changes(
    pool: &deadpool_postgres::Pool,
    params: &ApplyTableChangesParams,
) -> Result<ApplyTableChangesResponse, DbError> {
    let schema = params.schema.trim();
    let table = params.table.trim();
    if schema.is_empty() || table.is_empty() {
        return Err(DbError::validation("Schema and table are required"));
    }

    let mut client = pool.get().await?;
    set_statement_timeout(&client).await?;

    let mut updated = 0usize;
    let mut deleted = 0usize;
    let mut inserted = 0usize;
    let mut updated_rows: Vec<UpdatedRow> = Vec::new();

    let tx = client.transaction().await?;
    let safe_table = format!(
        "{}.{}",
        sql::quote_ident_for(DatabaseType::Postgres, schema),
        sql::quote_ident_for(DatabaseType::Postgres, table)
    );

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
            .map(|(index, (column, _))| {
                format!(
                    "{} = ${}",
                    sql::quote_ident_for(DatabaseType::Postgres, column),
                    index + 1
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let row_id_param = entries.len() + 1;
        let query = format!(
            "update {safe_table} as t set {set_clause} where t.ctid = ${row_id_param}::text::tid returning t.ctid::text as {}, to_jsonb(t)::text as _querycastle_row_json",
            sql::HIDDEN_ROW_ID_COLUMN
        );

        let mut bind_values: Vec<PgTextParam> = entries
            .iter()
            .map(|(_, value)| json_to_pg_param(value))
            .collect();
        bind_values.push(PgTextParam(Some(update.row_id.clone())));

        let updated_row = query_opt_untyped(&tx, query.as_str(), &bind_values).await?;
        let Some(updated_row) = updated_row else {
            return Err(DbError::NotFound(format!(
                "Could not update row {}. It may have changed. Refresh and retry.",
                update.row_id
            )));
        };
        let new_row_id: String = updated_row.try_get(sql::HIDDEN_ROW_ID_COLUMN)?;
        let row_json: String = updated_row.try_get("_querycastle_row_json")?;
        let values: HashMap<String, Value> = serde_json::from_str(&row_json).map_err(|e| DbError::internal(e.to_string()))?;
        updated_rows.push(UpdatedRow {
            old_row_id: update.row_id.clone(),
            new_row_id,
            values,
        });
        updated += 1;
    }

    for row_id in &params.changes.deletes {
        let query = format!("delete from {safe_table} where ctid = $1::text::tid");
        let affected = exec_untyped(&tx, query.as_str(), &[PgTextParam(Some(row_id.clone()))]).await?;
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
            .map(|(column, _)| sql::quote_ident_for(DatabaseType::Postgres, column))
            .collect::<Vec<_>>()
            .join(", ");
        let placeholders = (1..=entries.len())
            .map(|index| format!("${index}"))
            .collect::<Vec<_>>()
            .join(", ");
        let query = format!("insert into {safe_table} ({cols}) values ({placeholders})");
        let bind_values: Vec<PgTextParam> = entries
            .iter()
            .map(|(_, value)| json_to_pg_param(value))
            .collect();
        exec_untyped(&tx, query.as_str(), &bind_values).await?;
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

pub async fn get_object_definition(
    pool: &deadpool_postgres::Pool,
    params: &ObjectDefinitionParams,
) -> Result<ObjectDefinition, DbError> {
    let client = pool.get().await?;
    postgres_object_definition(&client, params).await
}

async fn load_postgres_indexes(
    client: &tokio_postgres::Client,
    table_map: &mut HashMap<String, DatabaseTable>,
) -> Result<(), DbError> {
    let rows = client
        .query(
            "
            select
                n.nspname as schema_name,
                t.relname as table_name,
                i.relname as index_name,
                ix.indisunique as is_unique,
                ix.indisprimary as is_primary,
                pg_catalog.pg_get_indexdef(ix.indexrelid) as definition
            from pg_catalog.pg_index ix
            join pg_catalog.pg_class t on t.oid = ix.indrelid
            join pg_catalog.pg_class i on i.oid = ix.indexrelid
            join pg_catalog.pg_namespace n on n.oid = t.relnamespace
            where n.nspname not in ('pg_catalog', 'information_schema')
                and t.relkind in ('r', 'p', 'm', 'f')
            order by n.nspname, t.relname, i.relname
            ",
            &[],
        )
        .await
        ?;

    for row in rows {
        let schema_name: String = row.try_get("schema_name")?;
        let table_name: String = row.try_get("table_name")?;
        let key = format!("{schema_name}.{table_name}");
        if let Some(table) = table_map.get_mut(&key) {
            let definition: Option<String> = row.try_get("definition")?;
            table.indexes.push(DatabaseIndex {
                name: row.try_get("index_name")?,
                columns: String::new(),
                unique: row.try_get("is_unique")?,
                is_primary: row.try_get("is_primary")?,
                definition,
            });
        }
    }
    Ok(())
}

async fn load_postgres_triggers(
    client: &tokio_postgres::Client,
    table_map: &mut HashMap<String, DatabaseTable>,
) -> Result<(), DbError> {
    let rows = client
        .query(
            "
            select
                n.nspname as schema_name,
                c.relname as table_name,
                t.tgname as trigger_name,
                pg_catalog.pg_get_triggerdef(t.oid, true) as definition
            from pg_catalog.pg_trigger t
            join pg_catalog.pg_class c on c.oid = t.tgrelid
            join pg_catalog.pg_namespace n on n.oid = c.relnamespace
            where not t.tgisinternal
                and n.nspname not in ('pg_catalog', 'information_schema')
            order by n.nspname, c.relname, t.tgname
            ",
            &[],
        )
        .await
        ?;

    for row in rows {
        let schema_name: String = row.try_get("schema_name")?;
        let table_name: String = row.try_get("table_name")?;
        let key = format!("{schema_name}.{table_name}");
        if let Some(table) = table_map.get_mut(&key) {
            table.triggers.push(DatabaseTrigger {
                name: row.try_get("trigger_name")?,
                definition: row.try_get("definition")?,
            });
        }
    }
    Ok(())
}

async fn load_postgres_routines(
    client: &tokio_postgres::Client,
    schema_map: &mut HashMap<String, DatabaseSchema>,
) -> Result<(), DbError> {
    let rows = client
        .query(
            "
            select
                n.nspname as schema_name,
                p.proname as name,
                p.oid::text as object_id,
                case p.prokind
                    when 'p' then 'procedure'
                    else 'function'
                end as kind,
                pg_catalog.pg_get_function_identity_arguments(p.oid) as identity_args,
                l.lanname as language,
                pg_catalog.pg_get_function_result(p.oid) as return_type
            from pg_catalog.pg_proc p
            join pg_catalog.pg_namespace n on n.oid = p.pronamespace
            join pg_catalog.pg_language l on l.oid = p.prolang
            where n.nspname not in ('pg_catalog', 'information_schema')
                and p.prokind in ('f', 'p')
            order by n.nspname, p.proname
            ",
            &[],
        )
        .await
        ?;

    for row in rows {
        let schema_name: String = row.try_get("schema_name")?;
        schema_map
            .entry(schema_name.clone())
            .or_insert_with(|| DatabaseSchema::new(schema_name.clone()));
        if let Some(schema) = schema_map.get_mut(&schema_name) {
            schema.routines.push(DatabaseRoutine {
                schema: schema_name,
                name: row.try_get("name")?,
                kind: row.try_get("kind")?,
                identity_args: row
                    .try_get::<_, Option<String>>("identity_args")
                    ?
                    .unwrap_or_default(),
                language: row.try_get("language")?,
                return_type: row.try_get("return_type")?,
                object_id: row.try_get("object_id")?,
            });
        }
    }
    Ok(())
}

async fn load_postgres_sequences(
    client: &tokio_postgres::Client,
    schema_map: &mut HashMap<String, DatabaseSchema>,
) -> Result<(), DbError> {
    let rows = client
        .query(
            "
            select
                n.nspname as schema_name,
                c.relname as name,
                pg_catalog.format_type(s.seqtypid, null) as data_type
            from pg_catalog.pg_class c
            join pg_catalog.pg_namespace n on n.oid = c.relnamespace
            left join pg_catalog.pg_sequence s on s.seqrelid = c.oid
            where c.relkind = 'S'
                and n.nspname not in ('pg_catalog', 'information_schema')
            order by n.nspname, c.relname
            ",
            &[],
        )
        .await
        ?;

    for row in rows {
        let schema_name: String = row.try_get("schema_name")?;
        schema_map
            .entry(schema_name.clone())
            .or_insert_with(|| DatabaseSchema::new(schema_name.clone()));
        if let Some(schema) = schema_map.get_mut(&schema_name) {
            schema.sequences.push(DatabaseSequence {
                schema: schema_name,
                name: row.try_get("name")?,
                data_type: row.try_get("data_type")?,
            });
        }
    }
    Ok(())
}

async fn postgres_object_definition(
    client: &tokio_postgres::Client,
    params: &ObjectDefinitionParams,
) -> Result<ObjectDefinition, DbError> {
    let kind = params.kind.trim().to_ascii_lowercase();
    let schema = params.schema.trim();
    let name = params.name.trim();
    if schema.is_empty() || name.is_empty() {
        return Err(DbError::validation("Schema and name are required"));
    }

    let qualified = format!("{}.{}", sql::quote_ident(schema), sql::quote_ident(name));
    let sql_text = match kind.as_str() {
        "function" | "procedure" => postgres_routine_definition(client, params, schema, name).await?,
        "sequence" => {
            let row = client
                .query_opt(
                    "
                    select format(
                        'CREATE SEQUENCE %I.%I AS %s INCREMENT BY %s MINVALUE %s MAXVALUE %s START WITH %s CACHE %s%s;',
                        n.nspname,
                        c.relname,
                        pg_catalog.format_type(s.seqtypid, null),
                        s.seqincrement,
                        s.seqmin,
                        s.seqmax,
                        s.seqstart,
                        s.seqcache,
                        case when s.seqcycle then ' CYCLE' else ' NO CYCLE' end
                    ) as definition
                    from pg_catalog.pg_class c
                    join pg_catalog.pg_namespace n on n.oid = c.relnamespace
                    left join pg_catalog.pg_sequence s on s.seqrelid = c.oid
                    where n.nspname = $1 and c.relname = $2 and c.relkind = 'S'
                    ",
                    &[&schema, &name],
                )
                .await
                ?;
            let definition = extract_definition_row(row)?;
            if definition.trim().is_empty() {
                format!("select * from {qualified}")
            } else {
                definition
            }
        }
        "index" => {
            let row = client
                .query_opt(
                    "
                    select pg_catalog.pg_get_indexdef(c.oid) as definition
                    from pg_catalog.pg_class c
                    join pg_catalog.pg_namespace n on n.oid = c.relnamespace
                    where n.nspname = $1 and c.relname = $2 and c.relkind = 'i'
                    ",
                    &[&schema, &name],
                )
                .await
                ?;
            extract_definition_row(row)?
        }
        "trigger" => {
            let table = params.table.as_deref().unwrap_or("").trim();
            let row = client
                .query_opt(
                    "
                    select pg_catalog.pg_get_triggerdef(t.oid, true) as definition
                    from pg_catalog.pg_trigger t
                    join pg_catalog.pg_class c on c.oid = t.tgrelid
                    join pg_catalog.pg_namespace n on n.oid = c.relnamespace
                    where n.nspname = $1 and t.tgname = $2
                        and ($3 = '' or c.relname = $3)
                    ",
                    &[&schema, &name, &table],
                )
                .await
                ?;
            extract_definition_row(row)?
        }
        "view" => {
            let row = client
                .query_opt(
                    "
                    select pg_catalog.pg_get_viewdef(c.oid, true) as definition
                    from pg_catalog.pg_class c
                    join pg_catalog.pg_namespace n on n.oid = c.relnamespace
                    where n.nspname = $1 and c.relname = $2 and c.relkind in ('v', 'm')
                    ",
                    &[&schema, &name],
                )
                .await
                ?;
            let body = extract_definition_row(row)?;
            if body.is_empty() {
                String::new()
            } else {
                format!("create or replace view {qualified} as\n{body}")
            }
        }
        _ => {
            return Err(DbError::validation(format!("Unsupported object type: {kind}")));
        }
    };

    if sql_text.trim().is_empty() {
        return Err(DbError::validation("Could not load object definition"));
    }

    let title = name.to_string();

    Ok(ObjectDefinition {
        title,
        sql: if sql_text.trim_end().ends_with(';') {
            sql_text
        } else {
            format!("{};", sql_text.trim_end())
        },
    })
}

async fn postgres_routine_definition(
    client: &tokio_postgres::Client,
    params: &ObjectDefinitionParams,
    schema: &str,
    name: &str,
) -> Result<String, DbError> {
    if let Some(object_id) = params.object_id.as_deref().filter(|value| !value.is_empty()) {
        if let Ok(oid) = object_id.parse::<u32>() {
            let row = client
                .query_opt(
                    "select pg_catalog.pg_get_functiondef($1::oid) as definition",
                    &[&oid],
                )
                .await
                ?;
            return extract_definition_row(row);
        }
    }

    let identity = params.identity_args.as_deref().unwrap_or("").trim();
    let row = client
        .query_opt(
            "
            select pg_catalog.pg_get_functiondef(p.oid) as definition
            from pg_catalog.pg_proc p
            join pg_catalog.pg_namespace n on n.oid = p.pronamespace
            where n.nspname = $1
                and p.proname = $2
                and ($3 = '' or pg_catalog.pg_get_function_identity_arguments(p.oid) = $3)
            order by p.oid
            limit 1
            ",
            &[&schema, &name, &identity],
        )
        .await
        ?;
    extract_definition_row(row)
}

fn extract_definition_row(row: Option<tokio_postgres::Row>) -> Result<String, DbError> {
    let row = row.ok_or_else(|| DbError::validation("Could not load object definition"))?;
    let value: Option<String> = row
        .try_get("definition")
        ?;
    Ok(value.unwrap_or_default())
}
