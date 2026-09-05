use url::Url;

use crate::core::error::DbError;
use crate::core::types::{ConnectionInput, DatabaseType};

pub(crate) const QUERY_TIMEOUT_MS: u64 = 30_000;
pub(crate) const MAX_QUERY_ROWS: usize = 1_000;

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

pub(crate) fn normalize_connection_input(input: ConnectionInput) -> Result<ConnectionInput, DbError> {
    let database_type = input.database_type;

    if input.use_connection_string.unwrap_or(false) {
        let raw = input.connection_string.clone().unwrap_or_default().trim().to_string();
        if raw.is_empty() {
            return Err(DbError::validation("Connection string is required"));
        }

        let parsed = Url::parse(&raw).map_err(|_| DbError::validation("Invalid connection string format"))?;
        let valid_scheme = match database_type {
            DatabaseType::Postgres => parsed.scheme() == "postgresql" || parsed.scheme() == "postgres",
            DatabaseType::Mysql => parsed.scheme() == "mysql",
            DatabaseType::Sqlite => parsed.scheme() == "sqlite",
        };
        if !valid_scheme {
            return Err(DbError::validation(match database_type {
                DatabaseType::Postgres => "Connection string must start with postgresql:// or postgres://",
                DatabaseType::Mysql => "Connection string must start with mysql://",
                DatabaseType::Sqlite => "Connection string must start with sqlite://",
            }));
        }

        let database = match database_type {
            DatabaseType::Sqlite => {
                let path = parsed.path().trim_start_matches('/').to_string();
                if path.is_empty() {
                    return Err(DbError::validation("Database path is missing in connection string"));
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
            ssl_insecure: if database_type == DatabaseType::Sqlite {
                false
            } else {
                input.ssl_insecure
            },
            use_connection_string: Some(true),
            connection_string: Some(raw),
        });
    }

    if database_type == DatabaseType::Sqlite {
        if input.database.trim().is_empty() {
            return Err(DbError::validation("Database path is required for SQLite"));
        }
    } else if input.host.trim().is_empty() || input.user.trim().is_empty() {
        return Err(DbError::validation("Host and user are required"));
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
        ssl_insecure: if database_type == DatabaseType::Sqlite {
            false
        } else {
            input.ssl_insecure
        },
        use_connection_string: Some(false),
        connection_string: Some(String::new()),
    })
}

pub(crate) fn with_new_database(connection: &ConnectionInput, new_database: &str) -> ConnectionInput {
    let new_db = new_database.trim().to_string();
    if connection.use_connection_string.unwrap_or(false) {
        if let Some(raw) = connection.connection_string.as_deref() {
            let raw = raw.trim();
            if !raw.is_empty() {
                if let Ok(mut url) = Url::parse(raw) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ConnectionInput {
        ConnectionInput {
            database_type: DatabaseType::Postgres,
            name: "n".into(),
            host: "localhost".into(),
            port: 5432,
            user: "postgres".into(),
            password: "secret".into(),
            database: "postgres".into(),
            ssl: true,
            ssl_insecure: true,
            use_connection_string: Some(false),
            connection_string: Some(String::new()),
        }
    }

    #[test]
    fn preserves_ssl_insecure() {
        let out = normalize_connection_input(sample()).unwrap();
        assert!(out.ssl);
        assert!(out.ssl_insecure);
    }

    #[test]
    fn sqlite_forces_ssl_off() {
        let mut input = sample();
        input.database_type = DatabaseType::Sqlite;
        input.database = "C:/tmp/x.db".into();
        let out = normalize_connection_input(input).unwrap();
        assert!(!out.ssl);
        assert!(!out.ssl_insecure);
    }
}
