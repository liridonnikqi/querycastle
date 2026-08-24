#![allow(dead_code)]
use thiserror::Error;

#[derive(Debug, Clone, serde::Serialize)]
pub enum DbErrorKind {
    Connection,
    Query,
    Timeout,
    Auth,
    Validation,
    NotFound,
    Internal,
}

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

    pub fn query(msg: impl Into<String>) -> Self {
        DbError::Query { message: msg.into(), code: None }
    }
}

impl From<DbError> for String {
    fn from(err: DbError) -> Self {
        err.to_string()
    }
}

// Structured error for frontend when needed
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

pub fn sanitize_pg_error_to_db_error(err: tokio_postgres::Error) -> DbError {
    use std::error::Error;
    if let Some(db_err) = err.as_db_error() {
        let mut lines = vec![format!("{} (SQLSTATE {})", db_err.message(), db_err.code().code())];
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
        let code = Some(db_err.code().code().to_string());
        // Map auth vs connection vs query by code prefix
        let code_str = db_err.code().code();
        if code_str == "28P01" || code_str == "28000" || code_str == "28P00" {
            return DbError::Auth { message: msg };
        }
        if code_str.starts_with("08") {
            return DbError::Connection { message: msg, code };
        }
        return DbError::Query { message: msg, code };
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
            return DbError::Internal(format!("Database error\nCaused by: {}", causes.join("\nCaused by: ")));
        }
    }
    // Heuristic timeout
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
        return DbError::NotFound(msg);
    }
    if lower.contains("connection") || lower.contains("can't connect") {
        return DbError::Connection { message: msg.clone(), code: None };
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
