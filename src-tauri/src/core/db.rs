use tauri::State;
use url::Url;

use crate::core::state::AppState;
use crate::core::types::{ConnectionInput, DatabaseType};

#[path = "db_mysql.rs"]
mod db_mysql;
#[path = "db_postgres.rs"]
mod db_postgres;
#[path = "db_sqlite.rs"]
mod db_sqlite;

pub(crate) use db_mysql::{
    connect_mysql_client, get_mysql_database_explorer, get_mysql_server_version,
    mysql_value_to_json, run_mysql_query,
};
pub(crate) use db_postgres::{connect_client, get_server_version};
pub(crate) use db_sqlite::{
    get_sqlite_database_explorer, get_sqlite_server_version, list_sqlite_databases,
    open_sqlite_connection, run_sqlite_query, sqlite_value_to_json,
};

pub(crate) const QUERY_TIMEOUT_MS: u64 = 30_000;
pub(crate) const MAX_QUERY_ROWS: usize = 1_000;

#[allow(dead_code)]
pub(crate) fn sanitize_error(error: impl ToString) -> String {
    error.to_string()
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
            _ => {
                let db = parsed.path().trim_start_matches('/').to_string();
                if db.is_empty() {
                    default_database(database_type).to_string()
                } else {
                    db
                }
            }
        };

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
    } else if input.host.trim().is_empty() || input.user.trim().is_empty() {
        return Err("Host and user are required".to_string());
    }

    Ok(ConnectionInput {
        database_type,
        name: if input.name.trim().is_empty() {
            if !input.database.trim().is_empty() {
                input.database.clone()
            } else if !input.host.trim().is_empty() {
                input.host.trim().to_string()
            } else {
                default_database(database_type).to_string()
            }
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
    let guard = state.inner.read().await;
    match guard.as_ref() {
        Some(active) => Ok((active.input.clone(), active.server_version.clone())),
        None => Err("No active database connection".to_string()),
    }
}

#[allow(dead_code)]
pub(crate) async fn get_active_connection(
    state: &State<'_, AppState>,
) -> Result<crate::core::state::ActiveConnection, String> {
    let guard = state.inner.read().await;
    guard.clone().ok_or_else(|| "No active database connection".to_string())
}

/// Helper to create a new ConnectionInput with a different database, preserving connection_string if used.
/// For Postgres/MySQL, updates the URL path to the new database so subsequent `connect_client` uses the new DB.
pub(crate) fn with_new_database(connection: &ConnectionInput, new_database: &str) -> ConnectionInput {
    let new_db = new_database.trim().to_string();
    if connection.use_connection_string.unwrap_or(false) {
        if let Some(raw) = connection.connection_string.as_deref() {
            let raw = raw.trim();
            if !raw.is_empty() {
                if let Ok(mut url) = Url::parse(raw) {
                    // Only mutate path for network DBs (postgres/mysql). For sqlite the path is the file, handled elsewhere.
                    match connection.database_type {
                        DatabaseType::Postgres | DatabaseType::Mysql => {
                            url.set_path(&format!("/{}", new_db));
                            return ConnectionInput {
                                database: new_db.clone(),
                                connection_string: Some(url.to_string()),
                                ..connection.clone()
                            };
                        }
                        DatabaseType::Sqlite => {
                            // For sqlite, connection_string like sqlite:///path/to/file.db – update path
                            url.set_path(&format!("/{}", new_db.trim_start_matches('/')));
                            return ConnectionInput {
                                database: new_db.clone(),
                                connection_string: Some(url.to_string()),
                                ..connection.clone()
                            };
                        }
                    }
                }
            }
        }
    }
    ConnectionInput {
        database: new_db,
        ..connection.clone()
    }
}