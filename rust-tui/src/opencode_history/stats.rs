use super::util::to_io_error;
use rusqlite::Row;
use std::fmt::Write as _;
use std::io;

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct SessionStats {
    pub share_url: Option<String>,
    pub cost: f64,
    pub tokens_input: i64,
    pub tokens_output: i64,
    pub tokens_reasoning: i64,
    pub tokens_cache_read: i64,
    pub tokens_cache_write: i64,
}

pub(crate) fn session_stats_select(connection: &rusqlite::Connection) -> io::Result<String> {
    let columns = [
        ("share_url", "NULL"),
        ("cost", "0"),
        ("tokens_input", "0"),
        ("tokens_output", "0"),
        ("tokens_reasoning", "0"),
        ("tokens_cache_read", "0"),
        ("tokens_cache_write", "0"),
    ];
    let mut select = String::with_capacity(128);
    for (column, fallback) in columns {
        if !select.is_empty() {
            select.push_str(", ");
        }
        if has_column(connection, "session", column)? {
            select.push_str(column);
        } else {
            select.push_str(fallback);
            select.push_str(" AS ");
            select.push_str(column);
        }
    }
    Ok(select)
}

pub(crate) fn read_session_stats(row: &Row<'_>, offset: usize) -> rusqlite::Result<SessionStats> {
    Ok(SessionStats {
        share_url: row.get(offset)?,
        cost: row.get(offset + 1)?,
        tokens_input: row.get(offset + 2)?,
        tokens_output: row.get(offset + 3)?,
        tokens_reasoning: row.get(offset + 4)?,
        tokens_cache_read: row.get(offset + 5)?,
        tokens_cache_write: row.get(offset + 6)?,
    })
}

pub(crate) fn format_cost(cost: f64) -> Option<String> {
    if cost <= 0.0 {
        return None;
    }
    Some(format!("${cost:.4}"))
}

pub(crate) fn format_token_summary(stats: &SessionStats) -> Option<String> {
    let total = stats
        .tokens_input
        .saturating_add(stats.tokens_output)
        .saturating_add(stats.tokens_reasoning)
        .saturating_add(stats.tokens_cache_read)
        .saturating_add(stats.tokens_cache_write);
    if total <= 0 {
        return None;
    }

    let mut summary = String::with_capacity(80);
    summary.push_str("tok ");
    push_compact_number(&mut summary, total);
    if stats.tokens_input > 0 {
        push_token_number(&mut summary, "in ", stats.tokens_input);
    }
    if stats.tokens_output > 0 {
        push_token_number(&mut summary, "out ", stats.tokens_output);
    }
    if stats.tokens_reasoning > 0 {
        push_token_number(&mut summary, "reason ", stats.tokens_reasoning);
    }
    if stats.tokens_cache_read > 0 || stats.tokens_cache_write > 0 {
        summary.push_str(" · cache ");
        push_compact_number(&mut summary, stats.tokens_cache_read);
        summary.push('/');
        push_compact_number(&mut summary, stats.tokens_cache_write);
    }
    Some(summary)
}

fn push_token_number(summary: &mut String, label: &str, value: i64) {
    summary.push_str(" · ");
    summary.push_str(label);
    push_compact_number(summary, value);
}

fn push_compact_number(summary: &mut String, value: i64) {
    let abs = value.abs();
    if abs >= 1_000_000 {
        write!(summary, "{:.1}m", value as f64 / 1_000_000.0).expect("write to string");
    } else if abs >= 1_000 {
        write!(summary, "{:.1}k", value as f64 / 1_000.0).expect("write to string");
    } else {
        write!(summary, "{value}").expect("write to string");
    }
}

fn has_column(connection: &rusqlite::Connection, table: &str, column: &str) -> io::Result<bool> {
    let mut statement = connection
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(to_io_error)?;
    let mut rows = statement.query([]).map_err(to_io_error)?;
    while let Some(row) = rows.next().map_err(to_io_error)? {
        let name: String = row.get(1).map_err(to_io_error)?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
#[path = "stats_tests.rs"]
mod tests;
