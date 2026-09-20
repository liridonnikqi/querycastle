use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum DbError {
    #[error("{message}")]
    Connection { message: String, code: Option<String> },
    #[error("{message}")]
    Query { message: String, code: Option<String> },
    #[error("Timeout: {message}")]
    Timeout { message: String },
    #[error("Authentication failed: {message}")]
    Auth { message: String },
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Internal(String),
    #[error("Query cancelled")]
    Cancelled,
}

impl DbError {
    pub fn kind(&self) -> &'static str {
        match self {
            DbError::Connection { .. } => "connection",
            DbError::Query { .. } => "query",
            DbError::Timeout { .. } => "timeout",
            DbError::Auth { .. } => "auth",
            DbError::Validation(_) => "validation",
            DbError::NotFound(_) => "not_found",
            DbError::Internal(_) => "internal",
            DbError::Cancelled => "cancelled",
        }
    }

    pub fn message(&self) -> String {
        self.to_string()
    }

    pub fn code(&self) -> Option<String> {
        match self {
            DbError::Connection { code, .. } => code.clone(),
            DbError::Query { code, .. } => code.clone(),
            _ => None,
        }
    }

    pub fn validation(msg: impl Into<String>) -> Self {
        DbError::Validation(msg.into())
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        DbError::Internal(msg.into())
    }

    pub fn connection(msg: impl Into<String>) -> Self {
        DbError::Connection { message: msg.into(), code: None }
    }

    pub fn auth(msg: impl Into<String>) -> Self {
        DbError::Auth { message: msg.into() }
    }

    pub fn query(msg: impl Into<String>) -> Self {
        DbError::Query { message: msg.into(), code: None }
    }

    pub fn cancelled() -> Self {
        DbError::Cancelled
    }

    pub fn read_only() -> Self {
        DbError::validation("This connection is read-only. Write queries are blocked.")
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StructuredDbError {
    pub kind: String,
    pub message: String,
    pub code: Option<String>,
}

impl From<DbError> for StructuredDbError {
    fn from(err: DbError) -> Self {
        Self {
            kind: err.kind().to_string(),
            message: err.message(),
            code: err.code(),
        }
    }
}

fn missing_database_message(server_message: &str) -> String {
    format!(
        "{server_message}\nIt may have been dropped. Edit this connection and choose a database that still exists on the server."
    )
}

fn from_pg_db_error(db_err: &tokio_postgres::error::DbError) -> DbError {
    let code_str = db_err.code().code();
    let mut lines = vec![format!("{} (SQLSTATE {})", db_err.message(), code_str)];
    if let Some(detail) = db_err.detail() {
        lines.push(format!("Detail: {detail}"));
    }
    if let Some(hint) = db_err.hint() {
        lines.push(format!("Hint: {hint}"));
    }
    if let Some(pos) = db_err.position() {
        lines.push(format!("Position: {pos:?}"));
    }
    let msg = lines.join("\n");
    let code = Some(code_str.to_string());
    if code_str == "28P01" || code_str == "28000" || code_str == "28P00" {
        return DbError::Auth { message: msg };
    }
    if code_str == "3D000" {
        return DbError::NotFound(missing_database_message(&msg));
    }
    if code_str.starts_with("08") {
        return DbError::Connection { message: msg, code };
    }
    DbError::Query { message: msg, code }
}

pub fn sanitize_pg_error_to_db_error(err: tokio_postgres::Error) -> DbError {
    use std::error::Error;
    if let Some(db_err) = err.as_db_error() {
        return from_pg_db_error(db_err);
    }
    let base = err.to_string();
    if base.trim().eq_ignore_ascii_case("db error") {
        let mut causes = Vec::new();
        let mut source = err.source();
        while let Some(cause) = source {
            causes.push(cause.to_string());
            source = cause.source();
        }
        if !causes.is_empty() {
            let joined = causes.join("\n");
            if joined.to_lowercase().contains("does not exist") {
                return DbError::NotFound(missing_database_message(&joined));
            }
            return DbError::Internal(format!("Database error\nCaused by: {}", causes.join("\nCaused by: ")));
        }
    }
    if base.to_lowercase().contains("timeout") {
        return DbError::Timeout { message: base };
    }
    DbError::Internal(base)
}

pub fn sanitize_mysql_error_to_db_error(err: mysql_async::Error) -> DbError {
    let msg = err.to_string();
    let lower = msg.to_lowercase();
    if lower.contains("access denied") || lower.contains("authentication") {
        return DbError::Auth { message: msg };
    }
    if lower.contains("timeout") || lower.contains("timed out") {
        return DbError::Timeout { message: msg };
    }
    if lower.contains("unknown database") || lower.contains("doesn't exist") {
        return DbError::NotFound(missing_database_message(&msg));
    }
    if lower.contains("connection") || lower.contains("can't connect") {
        return DbError::Connection { message: msg.clone(), code: None };
    }
    DbError::Query { message: msg, code: None }
}

pub fn sanitize_mssql_error_to_db_error(err: tiberius::error::Error) -> DbError {
    let msg = err.to_string();
    let lower = msg.to_lowercase();
    if lower.contains("login failed") || lower.contains("authentication") || lower.contains("login is from an untrusted") {
        return DbError::Auth { message: msg };
    }
    if lower.contains("timeout") || lower.contains("timed out") {
        return DbError::Timeout { message: msg };
    }
    if lower.contains("cannot open database") || lower.contains("not found") {
        return DbError::NotFound(missing_database_message(&msg));
    }
    if lower.contains("unable to complete login") || lower.contains("connection") {
        return DbError::Connection { message: msg, code: None };
    }
    DbError::Query { message: msg, code: None }
}

pub fn sanitize_sqlite_error_to_db_error(err: rusqlite::Error) -> DbError {
    let msg = err.to_string();
    let lower = msg.to_lowercase();
    if lower.contains("no such table") || lower.contains("no such column") {
        return DbError::NotFound(msg);
    }
    DbError::Query { message: msg, code: None }
}

impl From<tokio_postgres::Error> for DbError {
    fn from(err: tokio_postgres::Error) -> Self {
        sanitize_pg_error_to_db_error(err)
    }
}

impl From<mysql_async::Error> for DbError {
    fn from(err: mysql_async::Error) -> Self {
        sanitize_mysql_error_to_db_error(err)
    }
}

impl From<rusqlite::Error> for DbError {
    fn from(err: rusqlite::Error) -> Self {
        sanitize_sqlite_error_to_db_error(err)
    }
}

impl From<tiberius::error::Error> for DbError {
    fn from(err: tiberius::error::Error) -> Self {
        sanitize_mssql_error_to_db_error(err)
    }
}

impl From<deadpool_postgres::PoolError> for DbError {
    fn from(err: deadpool_postgres::PoolError) -> Self {
        match err {
            deadpool_postgres::PoolError::Backend(pg_err) => sanitize_pg_error_to_db_error(pg_err),
            deadpool_postgres::PoolError::Timeout(kind) => {
                let when = match kind {
                    deadpool::managed::TimeoutType::Wait => {
                        "waiting for a free PostgreSQL connection"
                    }
                    deadpool::managed::TimeoutType::Create => "connecting to PostgreSQL",
                    deadpool::managed::TimeoutType::Recycle => {
                        "reusing a PostgreSQL connection"
                    }
                };
                DbError::Timeout {
                    message: format!("Timed out while {when}"),
                }
            }
            deadpool_postgres::PoolError::Closed => {
                DbError::connection("The PostgreSQL connection pool is closed")
            }
            deadpool_postgres::PoolError::NoRuntimeSpecified => {
                DbError::internal("PostgreSQL pool is missing a runtime")
            }
            other => {
                if let Some(source) = std::error::Error::source(&other) {
                    if let Some(pg_err) = source.downcast_ref::<tokio_postgres::Error>() {
                        if let Some(db_err) = pg_err.as_db_error() {
                            return from_pg_db_error(db_err);
                        }
                    }
                }
                DbError::connection(other.to_string())
            }
        }
    }
}

impl From<deadpool::managed::PoolError<DbError>> for DbError {
    fn from(err: deadpool::managed::PoolError<DbError>) -> Self {
        match err {
            deadpool::managed::PoolError::Backend(err) => err,
            other => DbError::connection(format!("SQL Server pool get failed: {other}")),
        }
    }
}

impl From<r2d2::Error> for DbError {
    fn from(err: r2d2::Error) -> Self {
        DbError::connection(format!("SQLite pool get failed: {err}"))
    }
}

impl From<tokio::task::JoinError> for DbError {
    fn from(err: tokio::task::JoinError) -> Self {
        DbError::internal(format!("SQLite task failed: {err}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_database_message_explains_the_drop() {
        let msg = missing_database_message("database \"app\" does not exist (SQLSTATE 3D000)");
        assert!(msg.contains("app"));
        assert!(msg.contains("dropped"));
        assert!(msg.contains("choose a database"));
    }
}
