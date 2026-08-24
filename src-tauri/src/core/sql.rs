#![allow(dead_code)]
use serde_json::Value;

use crate::core::types::DatabaseType;

/// Escape a string for SQL single-quoted literal (doubles single quotes, escapes backslashes)
pub(crate) fn escape_sql_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "''")
}

/// Quote an identifier for Postgres/SQLite (double quotes)
pub(crate) fn quote_ident(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

/// Quote an identifier for MySQL (backticks)
pub(crate) fn quote_ident_mysql(value: &str) -> String {
    format!("`{}`", value.replace('`', "``"))
}

/// Dialect-aware identifier quoting
pub(crate) fn quote_ident_for(dialect: DatabaseType, value: &str) -> String {
    match dialect {
        DatabaseType::Mysql => quote_ident_mysql(value),
        DatabaseType::Postgres | DatabaseType::Sqlite => quote_ident(value),
    }
}

/// Escape single quotes for SQLite PRAGMA (both " and ')
pub(crate) fn escape_single_quotes_pragma(value: &str) -> String {
    value.replace('"', "\"\"").replace('\'', "''")
}

/// Dialect-aware literal for a JSON Value
pub(crate) fn value_to_sql_literal(value: &Value) -> String {
    // Default postgres behavior kept for backward compat; prefer dialect-aware
    value_to_sql_literal_for(DatabaseType::Postgres, value)
}

pub(crate) fn value_to_sql_literal_for(dialect: DatabaseType, value: &Value) -> String {
    match value {
        Value::Null => "NULL".to_string(),
        Value::Bool(v) => match dialect {
            DatabaseType::Postgres => {
                if *v { "TRUE".to_string() } else { "FALSE".to_string() }
            }
            DatabaseType::Mysql | DatabaseType::Sqlite => {
                if *v { "1".to_string() } else { "0".to_string() }
            }
        },
        Value::Number(v) => v.to_string(),
        Value::String(v) => format!("'{}'", escape_sql_string(v)),
        Value::Array(_) | Value::Object(_) => {
            // Only Postgres supports ::jsonb; for others just JSON string
            let json = value.to_string();
            match dialect {
                DatabaseType::Postgres => format!("'{}'::jsonb", escape_sql_string(&json)),
                DatabaseType::Mysql | DatabaseType::Sqlite => format!("'{}'", escape_sql_string(&json)),
            }
        }
    }
}

/// Central helper to build explorer table type kind from relkind etc.
pub(crate) fn normalize_table_kind(dialect: DatabaseType, raw: &str) -> String {
    match dialect {
        DatabaseType::Postgres => {
            if raw == "v" || raw == "m" {
                "view".to_string()
            } else {
                "table".to_string()
            }
        }
        DatabaseType::Mysql => {
            if raw == "VIEW" { "view".to_string() } else { "table".to_string() }
        }
        DatabaseType::Sqlite => {
            if raw == "view" { "view".to_string() } else { "table".to_string() }
        }
    }
}