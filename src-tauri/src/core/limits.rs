use std::borrow::Cow;

pub(crate) const QUERY_TIMEOUT_MS: u64 = 30_000;
pub(crate) const MAX_QUERY_ROWS: usize = 1_000;

/// Append `LIMIT n` to a single SELECT-like statement that has no existing limit.
/// Multi-statement SQL and DML are left alone — the caller still truncates the vec.
pub(crate) fn apply_select_row_cap(sql: &str) -> Cow<'_, str> {
    let trimmed = sql.trim();
    if trimmed.is_empty() {
        return Cow::Borrowed(sql);
    }
    let body = trimmed.trim_end_matches(';').trim_end();
    if body.contains(';') {
        return Cow::Borrowed(sql);
    }
    let lower = body.to_ascii_lowercase();
    let select_like = lower.starts_with("select")
        || lower.starts_with("table ")
        || lower.starts_with("values")
        || (lower.starts_with("with")
            && (lower.contains(" select ") || lower.contains(")select"))
            && !lower.contains(" insert ")
            && !lower.contains(" update ")
            && !lower.contains(" delete "));
    if !select_like {
        return Cow::Borrowed(sql);
    }
    if lower.contains(" limit")
        || lower.contains(" fetch first ")
        || lower.contains(" fetch next ")
    {
        return Cow::Borrowed(sql);
    }
    Cow::Owned(format!("{body} limit {MAX_QUERY_ROWS}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn injects_limit_on_simple_select() {
        let capped = apply_select_row_cap("select * from users;");
        assert_eq!(capped.as_ref(), &format!("select * from users limit {MAX_QUERY_ROWS}"));
    }

    #[test]
    fn injects_limit_on_table_and_values() {
        assert_eq!(
            apply_select_row_cap("table users").as_ref(),
            &format!("table users limit {MAX_QUERY_ROWS}")
        );
        assert_eq!(
            apply_select_row_cap("values (1), (2)").as_ref(),
            &format!("values (1), (2) limit {MAX_QUERY_ROWS}")
        );
    }

    #[test]
    fn injects_limit_on_select_cte() {
        let sql = "with x as (select 1 as n) select * from x";
        assert_eq!(
            apply_select_row_cap(sql).as_ref(),
            &format!("{sql} limit {MAX_QUERY_ROWS}")
        );
    }

    #[test]
    fn skips_existing_limit_or_fetch() {
        assert_eq!(
            apply_select_row_cap("select * from users limit 10").as_ref(),
            "select * from users limit 10"
        );
        assert_eq!(
            apply_select_row_cap("select * from users fetch first 5 rows only").as_ref(),
            "select * from users fetch first 5 rows only"
        );
    }

    #[test]
    fn skips_multi_statement_and_dml() {
        let multi = "select 1; select 2";
        assert_eq!(apply_select_row_cap(multi).as_ref(), multi);
        assert_eq!(
            apply_select_row_cap("insert into t values (1)").as_ref(),
            "insert into t values (1)"
        );
        assert_eq!(
            apply_select_row_cap("with x as (select 1) insert into t select * from x").as_ref(),
            "with x as (select 1) insert into t select * from x"
        );
    }

    #[test]
    fn skips_empty_sql() {
        assert_eq!(apply_select_row_cap("").as_ref(), "");
        assert_eq!(apply_select_row_cap("   ").as_ref(), "   ");
    }
}
