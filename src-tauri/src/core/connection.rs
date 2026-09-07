use url::Url;

use crate::core::error::DbError;
use crate::core::types::{ConnectionInput, DatabaseType};

fn default_port(database_type: DatabaseType) -> u16 {
    match database_type {
        DatabaseType::Postgres => 5432,
        DatabaseType::Mysql => 3306,
        DatabaseType::Sqlite => 0,
        DatabaseType::Mssql => 1433,
    }
}

fn default_user(database_type: DatabaseType) -> &'static str {
    match database_type {
        DatabaseType::Postgres => "postgres",
        DatabaseType::Mysql => "root",
        DatabaseType::Sqlite => "",
        DatabaseType::Mssql => "sa",
    }
}

fn default_database(database_type: DatabaseType) -> &'static str {
    match database_type {
        DatabaseType::Postgres => "postgres",
        DatabaseType::Mysql => "mysql",
        DatabaseType::Sqlite => "main",
        DatabaseType::Mssql => "master",
    }
}

pub(crate) fn normalize_connection_input(input: ConnectionInput) -> Result<ConnectionInput, DbError> {
    let database_type = input.database_type;

    if input.use_connection_string {
        let raw = input.connection_string.trim().to_string();
        if raw.is_empty() {
            return Err(DbError::validation("Connection string is required"));
        }

        if database_type == DatabaseType::Mssql {
            let lower = raw.to_ascii_lowercase();
            if lower.starts_with("jdbc:sqlserver:") {
                return normalize_mssql_jdbc_string(input, raw);
            }
            if !lower.starts_with("sqlserver://") && !lower.starts_with("mssql://") {
                return normalize_mssql_ado_string(input, raw);
            }
        }

        let parsed = Url::parse(&raw).map_err(|_| DbError::validation("Invalid connection string format"))?;
        let valid_scheme = match database_type {
            DatabaseType::Postgres => parsed.scheme() == "postgresql" || parsed.scheme() == "postgres",
            DatabaseType::Mysql => parsed.scheme() == "mysql",
            DatabaseType::Sqlite => parsed.scheme() == "sqlite",
            DatabaseType::Mssql => parsed.scheme() == "sqlserver" || parsed.scheme() == "mssql",
        };
        if !valid_scheme {
            return Err(DbError::validation(match database_type {
                DatabaseType::Postgres => "Connection string must start with postgresql:// or postgres://",
                DatabaseType::Mysql => "Connection string must start with mysql://",
                DatabaseType::Sqlite => "Connection string must start with sqlite://",
                DatabaseType::Mssql => {
                    "Connection string must be sqlserver://, mssql://, or an ADO.NET string"
                }
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
            use_connection_string: true,
            connection_string: raw,
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
        use_connection_string: false,
        connection_string: String::new(),
    })
}

pub(crate) fn with_new_database(connection: &ConnectionInput, new_database: &str) -> ConnectionInput {
    let new_db = new_database.trim().to_string();
    if connection.use_connection_string {
        let raw = connection.connection_string.trim();
        if !raw.is_empty() {
            if let Ok(mut url) = Url::parse(raw) {
                match connection.database_type {
                    DatabaseType::Postgres | DatabaseType::Mysql | DatabaseType::Mssql => {
                        url.set_path(&format!("/{}", new_db));
                        return ConnectionInput {
                            database: new_db.clone(),
                            connection_string: url.to_string(),
                            ..connection.clone()
                        };
                    }
                    DatabaseType::Sqlite => {
                        url.set_path(&format!("/{}", new_db.trim_start_matches('/')));
                        return ConnectionInput {
                            database: new_db.clone(),
                            connection_string: url.to_string(),
                            ..connection.clone()
                        };
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

fn normalize_mssql_jdbc_string(
    input: ConnectionInput,
    raw: String,
) -> Result<ConnectionInput, DbError> {
    let rest = raw
        .split_once("://")
        .map(|(_, tail)| tail)
        .unwrap_or(raw.as_str());
    let (hostport, params) = rest.split_once(';').unwrap_or((rest, ""));
    let ado = if hostport.trim().is_empty() {
        params.to_string()
    } else {
        format!("Server={hostport};{params}")
    };
    normalize_mssql_ado_string(input, ado).map(|mut parsed| {
        parsed.connection_string = raw;
        parsed
    })
}

fn normalize_mssql_ado_string(
    input: ConnectionInput,
    raw: String,
) -> Result<ConnectionInput, DbError> {
    let mut host = input.host;
    let mut port = if input.port == 0 {
        default_port(DatabaseType::Mssql)
    } else {
        input.port
    };
    let mut user = input.user;
    let mut password = input.password;
    let mut database = input.database;
    let mut ssl = input.ssl;
    let mut ssl_insecure = input.ssl_insecure;

    for part in raw.split(';') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim();
        match key.as_str() {
            "server" | "data source" => {
                let cleaned = value.trim_start_matches("tcp:");
                if let Some((h, p)) = cleaned.rsplit_once(',') {
                    host = h.trim().to_string();
                    if let Ok(parsed) = p.trim().parse::<u16>() {
                        port = parsed;
                    }
                } else {
                    host = cleaned.trim().to_string();
                }
            }
            "database" | "initial catalog" | "databasename" => database = value.to_string(),
            "user id" | "uid" => user = value.to_string(),
            "password" | "pwd" => password = value.to_string(),
            "encrypt" => {
                ssl = !matches!(
                    value.to_ascii_lowercase().as_str(),
                    "no" | "false" | "off" | "disable" | "optional"
                );
            }
            "trustservercertificate" => {
                ssl_insecure = matches!(value.to_ascii_lowercase().as_str(), "yes" | "true" | "1");
            }
            _ => {}
        }
    }

    if database.trim().is_empty() {
        database = default_database(DatabaseType::Mssql).to_string();
    }
    if user.trim().is_empty() {
        user = default_user(DatabaseType::Mssql).to_string();
    }
    if host.trim().is_empty() {
        host = "localhost".to_string();
    }

    Ok(ConnectionInput {
        database_type: DatabaseType::Mssql,
        name: if input.name.trim().is_empty() {
            database.clone()
        } else {
            input.name.trim().to_string()
        },
        host,
        port,
        user,
        password,
        database,
        ssl,
        ssl_insecure,
        use_connection_string: true,
        connection_string: raw,
    })
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
            use_connection_string: false,
            connection_string: String::new(),
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

    #[test]
    fn mssql_url_and_ado_strings() {
        let mut input = sample();
        input.database_type = DatabaseType::Mssql;
        input.use_connection_string = true;
        input.connection_string = "sqlserver://sa:pw@db.example:14333/app".into();
        let out = normalize_connection_input(input.clone()).unwrap();
        assert_eq!(out.host, "db.example");
        assert_eq!(out.port, 14333);
        assert_eq!(out.user, "sa");
        assert_eq!(out.password, "pw");
        assert_eq!(out.database, "app");

        input.connection_string =
            "Server=localhost,1433;Database=shop;User Id=sa;Password=secret;Encrypt=yes;TrustServerCertificate=yes"
                .into();
        let ado = normalize_connection_input(input).unwrap();
        assert_eq!(ado.host, "localhost");
        assert_eq!(ado.port, 1433);
        assert_eq!(ado.database, "shop");
        assert!(ado.ssl);
        assert!(ado.ssl_insecure);
    }
}
