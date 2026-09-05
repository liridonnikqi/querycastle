use std::time::Duration;

use crate::core::error::DbError;
use crate::core::types::{ConnectionInput, DatabaseType};

#[derive(Debug, Clone)]
pub enum Pool {
    Postgres(deadpool_postgres::Pool),
    Mysql(mysql_async::Pool),
    Sqlite(r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>),
}

pub fn create_pool(connection: &ConnectionInput) -> Result<Pool, DbError> {
    match connection.database_type {
        DatabaseType::Postgres => create_postgres_pool(connection).map(Pool::Postgres),
        DatabaseType::Mysql => create_mysql_pool(connection).map(Pool::Mysql),
        DatabaseType::Sqlite => create_sqlite_pool(connection).map(Pool::Sqlite),
    }
}

fn create_postgres_pool(connection: &ConnectionInput) -> Result<deadpool_postgres::Pool, DbError> {
    if connection.use_connection_string {
        let raw = connection.connection_string.trim();
        if !raw.is_empty() && (raw.starts_with("postgres://") || raw.starts_with("postgresql://")) {
            if let Ok(cfg) = raw.parse::<tokio_postgres::Config>() {
                return build_postgres_pool(cfg, connection.ssl, connection.ssl_insecure);
            }
        }
    }

    let mut cfg = tokio_postgres::Config::new();
    cfg.host(&connection.host);
    cfg.port(connection.port);
    cfg.user(&connection.user);
    cfg.password(&connection.password);
    cfg.dbname(&connection.database);
    cfg.connect_timeout(Duration::from_secs(5));
    cfg.application_name("querycastle");

    build_postgres_pool(cfg, connection.ssl, connection.ssl_insecure)
}

fn build_postgres_pool(
    cfg: tokio_postgres::Config,
    ssl: bool,
    ssl_insecure: bool,
) -> Result<deadpool_postgres::Pool, DbError> {
    if ssl {
        let mut builder = native_tls::TlsConnector::builder();
        if ssl_insecure {
            builder.danger_accept_invalid_certs(true);
            builder.danger_accept_invalid_hostnames(true);
        }
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
    let base = if connection.use_connection_string
        && connection.connection_string.trim().starts_with("mysql://")
    {
        mysql_async::Opts::from_url(connection.connection_string.trim())
            .map_err(|e| DbError::validation(format!("Invalid MySQL connection string: {e}")))?
    } else {
        let builder = mysql_async::OptsBuilder::default()
            .ip_or_hostname(connection.host.clone())
            .tcp_port(connection.port)
            .user(Some(connection.user.clone()))
            .pass(Some(connection.password.clone()))
            .db_name(Some(connection.database.clone()));
        mysql_async::Opts::from(builder)
    };

    let opts = if connection.ssl {
        let mut ssl_opts = mysql_async::SslOpts::default();
        if connection.ssl_insecure {
            ssl_opts = ssl_opts
                .with_danger_accept_invalid_certs(true)
                .with_danger_skip_domain_validation(true);
        }
        mysql_async::Opts::from(mysql_async::OptsBuilder::from_opts(base).ssl_opts(Some(ssl_opts)))
    } else {
        base
    };

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
