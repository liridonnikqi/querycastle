use crate::core::types::DatabaseType;

pub(crate) const HIDDEN_ROW_ID_COLUMN: &str = "_querycastle_row_id";

pub(crate) fn quote_ident(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

pub(crate) fn quote_ident_mysql(value: &str) -> String {
    format!("`{}`", value.replace('`', "``"))
}

pub(crate) fn quote_ident_for(dialect: DatabaseType, value: &str) -> String {
    match dialect {
        DatabaseType::Mysql => quote_ident_mysql(value),
        DatabaseType::Postgres | DatabaseType::Sqlite => quote_ident(value),
    }
}

pub(crate) fn escape_single_quotes_pragma(value: &str) -> String {
    value.replace('"', "\"\"").replace('\'', "''")
}
