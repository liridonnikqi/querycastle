use std::error::Error;

use tokio_postgres::Client;

use crate::core::types::ConnectionInput;

pub(crate) fn sanitize_pg_error(error: tokio_postgres::Error) -> String {
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

pub(crate) async fn connect_client(connection: &ConnectionInput) -> Result<Client, String> {
    // If a valid Postgres connection string is provided, use it directly (preserves application_name, keepalives, etc.)
    if connection.use_connection_string.unwrap_or(false) {
        if let Some(raw) = connection.connection_string.as_deref() {
            let raw = raw.trim();
            if !raw.is_empty() && (raw.starts_with("postgres://") || raw.starts_with("postgresql://")) {
                if let Ok(cfg) = raw.parse::<tokio_postgres::Config>() {
                    // Use parsed config directly – attempt single connection (pool creation handles pooling)
                    if connection.ssl {
                        let mut builder = native_tls::TlsConnector::builder();
                        builder.danger_accept_invalid_certs(true);
                        let connector = builder.build().map_err(|e| format!("TLS build failed: {e}"))?;
                        let tls = postgres_native_tls::MakeTlsConnector::new(connector);
                        match cfg.connect(tls).await {
                            Ok((client, task)) => {
                                tauri::async_runtime::spawn(async move {
                                    if let Err(e) = task.await { tracing::error!("postgres connection error: {e}"); }
                                });
                                return Ok(client);
                            }
                            Err(e) => {
                                return Err(format!("Unable to connect via connection string: {}", sanitize_pg_error(e)));
                            }
                        }
                    } else {
                        match cfg.connect(tokio_postgres::NoTls).await {
                            Ok((client, task)) => {
                                tauri::async_runtime::spawn(async move {
                                    if let Err(e) = task.await { tracing::error!("postgres connection error: {e}"); }
                                });
                                return Ok(client);
                            }
                            Err(e) => {
                                return Err(format!("Unable to connect via connection string: {}", sanitize_pg_error(e)));
                            }
                        }
                    }
                }
            }
        }
    }

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
            .connect_timeout(std::time::Duration::from_secs(5))
            .application_name("querycastle");

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
                            tracing::error!("postgres connection error: {error}");
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
                            tracing::error!("postgres connection error: {error}");
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

pub(crate) async fn get_server_version(client: &Client) -> Result<Option<String>, String> {
    let row = client
        .query_one("select current_setting('server_version') as server_version", &[])
        .await
        .map_err(sanitize_pg_error)?;
    let server_version: Option<String> = row.try_get("server_version").map_err(sanitize_pg_error)?;
    Ok(server_version)
}
