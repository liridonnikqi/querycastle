#![allow(dead_code)]
use std::time::Duration;

use crate::core::error::DbError;
use crate::core::types::{ConnectionInput, DatabaseType};

#[derive(Debug, Clone)]
pub enum Pool {
    Postgres(deadpool_postgres::Pool),
    Mysql(mysql_async::Pool),
    Sqlite(r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>),
}

impl Pool {
    pub fn database_type(&self) -> DatabaseType {
        match self {
            Pool::Postgres(_) => DatabaseType::Postgres,
            Pool::Mysql(_) => DatabaseType::Mysql,
            Pool::Sqlite(_) => DatabaseType::Sqlite,
        }
    }
}

pub fn create_pool(connection: &ConnectionInput) -> Result<Pool, DbError> {
    match connection.database_type {
        DatabaseType::Postgres => create_postgres_pool(connection).map(Pool::Postgres),
        DatabaseType::Mysql => create_mysql_pool(connection).map(Pool::Mysql),
        DatabaseType::Sqlite => create_sqlite_pool(connection).map(Pool::Sqlite),
    }
}

fn create_postgres_pool(connection: &ConnectionInput) -> Result<deadpool_postgres::Pool, DbError> {
    // If connection string is provided and valid postgres URL, try to parse it
    if connection.use_connection_string.unwrap_or(false) {
        if let Some(raw) = connection.connection_string.as_deref() {
            let raw = raw.trim();
            if !raw.is_empty() && (raw.starts_with("postgres://") || raw.starts_with("postgresql://")) {
                if let Ok(cfg) = raw.parse::<tokio_postgres::Config>() {
                    return build_postgres_pool(cfg, connection.ssl);
                }
            }
        }
    }

    let mut cfg = tokio_postgres::Config::new();
    // Handle localhost -> try 127.0.0.1 handling via pool? deadpool will handle single host; we keep original host
    // For localhost we could expand but keep simple
    cfg.host(&connection.host);
    cfg.port(connection.port);
    cfg.user(&connection.user);
    cfg.password(&connection.password);
    cfg.dbname(&connection.database);
    cfg.connect_timeout(Duration::from_secs(5));
    // application_name for tracing
    cfg.application_name("querycastle");

    build_postgres_pool(cfg, connection.ssl)
}

fn build_postgres_pool(cfg: tokio_postgres::Config, ssl: bool) -> Result<deadpool_postgres::Pool, DbError> {
    if ssl {
        let mut builder = native_tls::TlsConnector::builder();
        // Keep permissive as before for now; strict mode can be made configurable via SslMode later
        builder.danger_accept_invalid_certs(true);
        let connector = builder.build().map_err(|e| DbError::connection(format!("Failed to build TLS connector: {e}")))?;
        let tls = postgres_native_tls::MakeTlsConnector::new(connector);
        let mgr_config = deadpool_postgres::ManagerConfig { recycling_method: deadpool_postgres::RecyclingMethod::Fast };
        let mgr = deadpool_postgres::Manager::from_config(cfg, tls, mgr_config);
        deadpool_postgres::Pool::builder(mgr)
            .max_size(5)
            .build()
            .map_err(|e| DbError::connection(format!("Failed to build Postgres pool: {e}")))
    } else {
        let mgr_config = deadpool_postgres::ManagerConfig { recycling_method: deadpool_postgres::RecyclingMethod::Fast };
        let mgr = deadpool_postgres::Manager::from_config(cfg, tokio_postgres::NoTls, mgr_config);
        deadpool_postgres::Pool::builder(mgr)
            .max_size(5)
            .build()
            .map_err(|e| DbError::connection(format!("Failed to build Postgres pool: {e}")))
    }
}

fn create_mysql_pool(connection: &ConnectionInput) -> Result<mysql_async::Pool, DbError> {
    let opts = if connection.use_connection_string.unwrap_or(false)
        && connection.connection_string.as_deref().unwrap_or_default().trim().starts_with("mysql://")
    {
        mysql_async::Opts::from_url(connection.connection_string.as_deref().unwrap_or_default())
            .map_err(|e| DbError::validation(format!("Invalid MySQL connection string: {e}")))?
    } else {
        let mut builder = mysql_async::OptsBuilder::default();
        builder = builder
            .ip_or_hostname(connection.host.clone())
            .tcp_port(connection.port)
            .user(Some(connection.user.clone()))
            .pass(Some(connection.password.clone()))
            .db_name(Some(connection.database.clone()));
        // TODO: picks SSL handling when ssl=true; mysql_async supports ssl_opts, but we keep simple for now
        mysql_async::Opts::from(builder)
    };
    // Pool::new creates pool lazily; we can validate with Pool::new and optional test? Keep as is
    Ok(mysql_async::Pool::new(opts))
}

fn create_sqlite_pool(connection: &ConnectionInput) -> Result<r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>, DbError> {
    let path = connection.database.trim();
    if path.is_empty() {
        return Err(DbError::validation("Database path is required for SQLite"));
    }
    let manager = r2d2_sqlite::SqliteConnectionManager::file(path)
        .with_init(|conn| {
            conn.execute_batch("PRAGMA busy_timeout = 5000; PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")?;
            Ok(())
        });
    r2d2::Pool::builder()
        .max_size(5)
        .connection_timeout(Duration::from_secs(5))
        .build(manager)
        .map_err(|e| DbError::connection(format!("Failed to build SQLite pool: {e}")))
}

// Helpers for testing health
pub async fn test_postgres_pool(pool: &deadpool_postgres::Pool) -> Result<Option<String>, DbError> {
    let client = pool.get().await.map_err(|e| DbError::connection(format!("Pool get failed: {e}")))?;
    let row = client
        .query_one("select current_setting('server_version') as server_version", &[])
        .await
        .map_err(crate::core::error::sanitize_pg_error_to_db_error)?;
    let v: Option<String> = row.try_get("server_version").map_err(crate::core::error::sanitize_pg_error_to_db_error)?;
    Ok(v)
}

pub async fn test_mysql_pool(pool: &mysql_async::Pool) -> Result<Option<String>, DbError> {
    let mut conn = pool.get_conn().await.map_err(|e| DbError::connection(format!("MySQL pool get failed: {e}")))?;
    use mysql_async::prelude::Queryable;
    let v: Option<String> = conn.query_first("select version()").await.map_err(crate::core::error::sanitize_mysql_error_to_db_error)?;
    Ok(v)
}

pub fn test_sqlite_pool(pool: &r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>) -> Result<Option<String>, DbError> {
    let conn = pool.get().map_err(|e| DbError::connection(format!("SQLite pool get failed: {e}")))?;
    let v: String = conn
        .query_row("select sqlite_version()", [], |row| row.get(0))
        .map_err(crate::core::error::sanitize_sqlite_error_to_db_error)?;
    Ok(Some(v))
}
