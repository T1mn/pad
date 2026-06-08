use super::{format_token_summary, session_stats_select, SessionStats};
use rusqlite::Connection;

#[test]
fn token_summary_formats_total_breakdown_and_cache() {
    let stats = SessionStats {
        tokens_input: 1200,
        tokens_output: 340,
        tokens_reasoning: 60,
        tokens_cache_read: 5000,
        tokens_cache_write: 70,
        ..SessionStats::default()
    };

    assert_eq!(
        format_token_summary(&stats).as_deref(),
        Some("tok 6.7k · in 1.2k · out 340 · reason 60 · cache 5.0k/70")
    );
}

#[test]
fn token_summary_omits_empty_stats() {
    assert_eq!(format_token_summary(&SessionStats::default()), None);
}

#[test]
fn session_stats_select_uses_fallbacks_for_old_schema() {
    let connection = Connection::open_in_memory().expect("open db");
    connection
        .execute_batch(
            r#"
                CREATE TABLE session (
                    id text PRIMARY KEY,
                    directory text NOT NULL,
                    title text NOT NULL
                );
            "#,
        )
        .expect("create session table");

    assert_eq!(
        session_stats_select(&connection).expect("select stats"),
        "NULL AS share_url, 0 AS cost, 0 AS tokens_input, 0 AS tokens_output, 0 AS tokens_reasoning, 0 AS tokens_cache_read, 0 AS tokens_cache_write"
    );
}
