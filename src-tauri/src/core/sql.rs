use crate::core::types::DatabaseType;

const MUTATING_KEYWORDS: &[&str] = &[
    "INSERT", "UPDATE", "DELETE", "MERGE", "CREATE", "DROP", "ALTER", "TRUNCATE", "GRANT",
    "REVOKE", "REPLACE", "CALL", "EXEC", "EXECUTE", "COPY", "LOAD", "IMPORT", "VACUUM",
    "REINDEX", "ATTACH", "DETACH", "COMMENT",
];

fn strip_sql_comments(sql: &str) -> String {
    let mut out = String::with_capacity(sql.len());
    let bytes = sql.as_bytes();
    let mut i = 0;
    let mut in_single = false;
    let mut in_double = false;
    while i < bytes.len() {
        let c = bytes[i];
        if in_single {
            out.push(c as char);
            if c == b'\'' && bytes.get(i + 1) == Some(&b'\'') {
                out.push('\'');
                i += 2;
                continue;
            }
            if c == b'\'' {
                in_single = false;
            }
            i += 1;
            continue;
        }
        if in_double {
            out.push(c as char);
            if c == b'"' && bytes.get(i + 1) == Some(&b'"') {
                out.push('"');
                i += 2;
                continue;
            }
            if c == b'"' {
                in_double = false;
            }
            i += 1;
            continue;
        }
        if c == b'-' && bytes.get(i + 1) == Some(&b'-') {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if c == b'/' && bytes.get(i + 1) == Some(&b'*') {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = i.saturating_add(2);
            out.push(' ');
            continue;
        }
        if c == b'\'' {
            in_single = true;
            out.push('\'');
            i += 1;
            continue;
        }
        if c == b'"' {
            in_double = true;
            out.push('"');
            i += 1;
            continue;
        }
        out.push(c as char);
        i += 1;
    }
    out
}

fn first_keyword(sql: &str) -> Option<String> {
    let cleaned = strip_sql_comments(sql);
    cleaned
        .split_whitespace()
        .next()
        .map(|word| word.trim_matches(|c: char| !c.is_ascii_alphabetic()).to_ascii_uppercase())
        .filter(|word| !word.is_empty())
}

pub(crate) fn is_mutating_sql(sql: &str) -> bool {
    let cleaned = strip_sql_comments(sql);
    let first = match first_keyword(&cleaned) {
        Some(word) => word,
        None => return false,
    };
    if MUTATING_KEYWORDS.contains(&first.as_str()) {
        return true;
    }
    if first == "WITH" {
        let upper = cleaned.to_ascii_uppercase();
        return MUTATING_KEYWORDS
            .iter()
            .any(|keyword| upper.split_whitespace().any(|word| word == *keyword));
    }
    false
}

#[cfg(test)]
mod mutating_tests {
    use super::is_mutating_sql;

    #[test]
    fn select_is_not_mutating() {
        assert!(!is_mutating_sql("select * from users"));
        assert!(!is_mutating_sql("-- note\nSELECT 1"));
        assert!(!is_mutating_sql("with cte as (select 1) select * from cte"));
    }

    #[test]
    fn writes_are_mutating() {
        assert!(is_mutating_sql("update users set name = 'a'"));
        assert!(is_mutating_sql("insert into t values (1)"));
        assert!(is_mutating_sql("with x as (select 1) delete from t"));
        assert!(is_mutating_sql("/* c */ drop table t"));
    }
}

pub(crate) const HIDDEN_ROW_ID_COLUMN: &str = "_querycastle_row_id";

pub(crate) fn quote_ident(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

pub(crate) fn quote_ident_mysql(value: &str) -> String {
    format!("`{}`", value.replace('`', "``"))
}

pub(crate) fn quote_ident_mssql(value: &str) -> String {
    format!("[{}]", value.replace(']', "]]"))
}

pub(crate) fn quote_ident_for(dialect: DatabaseType, value: &str) -> String {
    match dialect {
        DatabaseType::Mysql => quote_ident_mysql(value),
        DatabaseType::Mssql => quote_ident_mssql(value),
        DatabaseType::Postgres | DatabaseType::Sqlite => quote_ident(value),
    }
}

pub(crate) fn escape_single_quotes_pragma(value: &str) -> String {
    value.replace('"', "\"\"").replace('\'', "''")
}
