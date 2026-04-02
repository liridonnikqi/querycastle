use serde_json::Value;
use tauri::State;
use url::Url;

use crate::core::state::AppState;
use crate::core::types::{ConnectionInput, DatabaseType};

#[path = "db_mysql.rs"]
mod db_mysql;
#[path = "db_postgres.rs"]
mod db_postgres;

pub(crate) use db_mysql::{
    connect_mysql_client, get_mysql_server_version, run_mysql_query, sanitize_mysql_error,
};
pub(crate) use db_postgres::{connect_client, get_server_version, sanitize_pg_error};

pub(crate) const QUERY_TIMEOUT_MS: u64 = 30_000;
pub(crate) const MAX_QUERY_ROWS: usize = 1_000;

pub(crate) fn sanitize_error(error: impl ToString) -> String {
    error.to_string()
}

pub(crate) fn quote_ident(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

pub(crate) fn escape_sql_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "''")
}

pub(crate) fn value_to_sql_literal(value: &Value) -> String {
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

fn default_port(database_type: DatabaseType) -> u16 {
    match database_type {
        DatabaseType::Postgres => 5432,
        DatabaseType::Mysql => 3306,
        DatabaseType::Sqlite => 0,
    }
}

fn default_user(database_type: DatabaseType) -> &'static str {
    match database_type {
        DatabaseType::Postgres => "postgres",
        DatabaseType::Mysql => "root",
        DatabaseType::Sqlite => "",
    }
}

fn default_database(database_type: DatabaseType) -> &'static str {
    match database_type {
        DatabaseType::Postgres => "postgres",
        DatabaseType::Mysql => "mysql",
        DatabaseType::Sqlite => "main",
    }
}

pub(crate) fn normalize_connection_input(input: ConnectionInput) -> Result<ConnectionInput, String> {
    let database_type = input.database_type;

    if input.use_connection_string.unwrap_or(false) {
        let raw = input.connection_string.clone().unwrap_or_default().trim().to_string();
        if raw.is_empty() {
            return Err("Connection string is required".to_string());
        }

        let parsed = Url::parse(&raw).map_err(|_| "Invalid connection string format")?;
        let valid_scheme = match database_type {
            DatabaseType::Postgres => parsed.scheme() == "postgresql" || parsed.scheme() == "postgres",
            DatabaseType::Mysql => parsed.scheme() == "mysql",
            DatabaseType::Sqlite => parsed.scheme() == "sqlite",
        };
        if !valid_scheme {
            return Err(match database_type {
                DatabaseType::Postgres => "Connection string must start with postgresql:// or postgres://".to_string(),
                DatabaseType::Mysql => "Connection string must start with mysql://".to_string(),
                DatabaseType::Sqlite => "Connection string must start with sqlite://".to_string(),
            });
        }

        let database = match database_type {
            DatabaseType::Sqlite => {
                let path = parsed.path().trim_start_matches('/').to_string();
                if path.is_empty() {
                    return Err("Database path is missing in connection string".to_string());
                }
                path
            }
            _ => parsed.path().trim_start_matches('/').to_string(),
        };
        if database.is_empty() {
            return Err("Database name is missing in connection string".to_string());
        }

        let resolved_port = parsed.port().unwrap_or(default_port(database_type));
        let ssl_from_url = parsed.query_pairs().find_map(|(key, val)| {
            if key == "sslmode" {
                let mode = val.to_lowercase();
                Some(["require", "verify-ca", "verify-full"].contains(&mode.as_str()))
            } else {
                None
            }
        });

        return Ok(ConnectionInput {
            database_type,
            name: if input.name.trim().is_empty() {
                database.clone()
            } else {
                input.name.trim().to_string()
            },
            host: if parsed.host_str().unwrap_or_default().is_empty() {
                if database_type == DatabaseType::Sqlite {
                    String::new()
                } else {
                    "localhost".to_string()
                }
            } else {
                parsed.host_str().unwrap_or_default().to_string()
            },
            port: resolved_port,
            user: if parsed.username().is_empty() {
                default_user(database_type).to_string()
            } else {
                parsed.username().to_string()
            },
            password: parsed.password().unwrap_or_default().to_string(),
            database,
            ssl: if database_type == DatabaseType::Sqlite {
                false
            } else {
                ssl_from_url.unwrap_or(input.ssl)
            },
            use_connection_string: Some(true),
            connection_string: Some(raw),
        });
    }

    if database_type == DatabaseType::Sqlite {
        if input.database.trim().is_empty() {
            return Err("Database path is required for SQLite".to_string());
        }
    } else if input.host.trim().is_empty() || input.database.trim().is_empty() || input.user.trim().is_empty() {
        return Err("Host, database, and user are required".to_string());
    }

    Ok(ConnectionInput {
        database_type,
        name: if input.name.trim().is_empty() {
            input.database.clone()
        } else {
            input.name.trim().to_string()
        },
        host: if database_type == DatabaseType::Sqlite {
            String::new()
        } else {
            input.host
        },
        port: if input.port == 0 {
            default_port(database_type)
        } else {
            input.port
        },
        user: if input.user.trim().is_empty() {
            default_user(database_type).to_string()
        } else {
            input.user
        },
        password: input.password,
        database: if input.database.trim().is_empty() {
            default_database(database_type).to_string()
        } else {
            input.database
        },
        ssl: if database_type == DatabaseType::Sqlite { false } else { input.ssl },
        use_connection_string: Some(false),
        connection_string: Some(String::new()),
    })
}

pub(crate) async fn get_connection_snapshot(
    state: &State<'_, AppState>,
) -> Result<(ConnectionInput, Option<String>), String> {
    let connection = state.connection.lock().await.clone();
    let server_version = state.server_version.lock().await.clone();
    match connection {
        Some(connection) => Ok((connection, server_version)),
        None => Err("No active database connection".to_string()),
    }
}