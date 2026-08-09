use super::archive_thread;

pub(crate) fn missing_thread_archive_returns_not_found() {
    let err = archive_thread("missing-session").unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
}

pub(crate) mod archive {
    use super::super::archive::set_archived_at;

    pub(crate) fn archive_matches_upstream_semantics_without_reordering_session() {
        let path = crate::test_support::temp_path("pad-opencode", "archive.db");
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE session (
                    id TEXT PRIMARY KEY, time_updated INTEGER NOT NULL, time_archived INTEGER
                );
                INSERT INTO session (id, time_updated) VALUES ('session-1', 42);",
            )
            .unwrap();

        set_archived_at(&path, "session-1", true).unwrap();
        let archived: (i64, Option<i64>) = connection
            .query_row(
                "SELECT time_updated, time_archived FROM session WHERE id='session-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(archived.0, 42);
        assert!(archived.1.is_some());

        set_archived_at(&path, "session-1", false).unwrap();
        let restored: (i64, Option<i64>) = connection
            .query_row(
                "SELECT time_updated, time_archived FROM session WHERE id='session-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(restored, (42, None));
        let _ = std::fs::remove_file(path);
    }
}

pub(crate) mod query {
    use super::super::query::query_threads_at;
    use rusqlite::{params, Connection};
    use std::path::Path;

    fn temp_db_path(name: &str) -> std::path::PathBuf {
        crate::test_support::temp_path("pad-opencode-history", name)
    }

    fn seed_db(path: &Path) {
        let connection = Connection::open(path).unwrap();
        connection
            .execute_batch(
                r#"
                    CREATE TABLE session (
                        id text PRIMARY KEY,
                        directory text NOT NULL,
                        path text,
                        title text NOT NULL,
                        time_updated integer NOT NULL,
                        time_archived integer,
                        model text,
                        share_url text,
                        cost real DEFAULT 0 NOT NULL,
                        tokens_input integer DEFAULT 0 NOT NULL,
                        tokens_output integer DEFAULT 0 NOT NULL,
                        tokens_reasoning integer DEFAULT 0 NOT NULL,
                        tokens_cache_read integer DEFAULT 0 NOT NULL,
                        tokens_cache_write integer DEFAULT 0 NOT NULL
                    );
                    CREATE TABLE message (
                        id text PRIMARY KEY,
                        session_id text NOT NULL,
                        time_created integer NOT NULL,
                        data text NOT NULL
                    );
                    CREATE TABLE part (
                        id text PRIMARY KEY,
                        message_id text NOT NULL,
                        session_id text NOT NULL,
                        time_created integer NOT NULL,
                        data text NOT NULL
                    );
                    "#,
            )
            .unwrap();
        connection
                .execute(
                    "INSERT INTO session (id, directory, path, title, time_updated, time_archived, model, share_url, cost, tokens_input, tokens_output, tokens_reasoning, tokens_cache_read, tokens_cache_write) VALUES (?1, ?2, NULL, ?3, ?4, NULL, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                    params![
                        "ses_1",
                        "/repo",
                        "Build feature",
                        100_i64,
                        r#"{"providerID":"wzw","id":"gpt-5.4"}"#,
                        "https://opencode.ai/s/abc",
                        0.01234_f64,
                        1200_i64,
                        340_i64,
                        60_i64,
                        5000_i64,
                        70_i64
                    ],
                )
                .unwrap();
        connection
            .execute(
                "INSERT INTO message (id, session_id, time_created, data) VALUES (?1, ?2, ?3, ?4)",
                params!["msg_1", "ses_1", 1_i64, r#"{"role":"user"}"#],
            )
            .unwrap();
        connection
                .execute(
                    "INSERT INTO part (id, message_id, session_id, time_created, data) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params!["prt_1", "msg_1", "ses_1", 2_i64, r#"{"type":"text","text":"hello"}"#],
                )
                .unwrap();
        connection
            .execute(
                "INSERT INTO message (id, session_id, time_created, data) VALUES (?1, ?2, ?3, ?4)",
                params!["msg_2", "ses_1", 3_i64, r#"{"role":"assistant"}"#],
            )
            .unwrap();
        connection
                .execute(
                    "INSERT INTO part (id, message_id, session_id, time_created, data) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params!["prt_2", "msg_2", "ses_1", 4_i64, r#"{"type":"text","text":"world"}"#],
                )
                .unwrap();
    }

    pub(crate) fn query_threads_supports_older_opencode_schema_without_stats() {
        let path = temp_db_path("query-old");
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                r#"
                    CREATE TABLE session (
                        id text PRIMARY KEY,
                        directory text NOT NULL,
                        path text,
                        title text NOT NULL,
                        time_updated integer NOT NULL,
                        time_archived integer,
                        model text
                    );
                    CREATE TABLE message (
                        id text PRIMARY KEY,
                        session_id text NOT NULL,
                        time_created integer NOT NULL,
                        data text NOT NULL
                    );
                    CREATE TABLE part (
                        id text PRIMARY KEY,
                        message_id text NOT NULL,
                        session_id text NOT NULL,
                        time_created integer NOT NULL,
                        data text NOT NULL
                    );
                    "#,
            )
            .unwrap();
        connection
                .execute(
                    "INSERT INTO session (id, directory, path, title, time_updated, time_archived, model) VALUES (?1, ?2, NULL, ?3, ?4, NULL, ?5)",
                    params![
                        "ses_old",
                        "/repo",
                        "Old schema",
                        90_i64,
                        r#"{"providerID":"old","id":"model"}"#
                    ],
                )
                .unwrap();

        let threads = query_threads_at(&path, Some(false)).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].session_id, "ses_old");
        assert_eq!(threads[0].share_url, None);
        assert_eq!(threads[0].cost, None);
        assert_eq!(threads[0].token_summary, None);
    }

    pub(crate) fn query_threads_reads_opencode_sqlite() {
        let path = temp_db_path("query");
        seed_db(&path);

        let threads = query_threads_at(&path, Some(false)).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].session_id, "ses_1");
        assert_eq!(threads[0].last_user_message.as_deref(), Some("hello"));
        assert_eq!(threads[0].last_assistant_message.as_deref(), Some("world"));
        assert_eq!(threads[0].provider_name.as_deref(), Some("wzw"));
        assert_eq!(
            threads[0].share_url.as_deref(),
            Some("https://opencode.ai/s/abc")
        );
        assert_eq!(threads[0].cost.as_deref(), Some("$0.0123"));
        assert_eq!(
            threads[0].token_summary.as_deref(),
            Some("tok 6.7k · in 1.2k · out 340 · reason 60 · cache 5.0k/70")
        );
    }
}

pub(crate) mod stats {
    use super::super::stats::{format_token_summary, session_stats_select, SessionStats};
    use rusqlite::Connection;

    pub(crate) fn token_summary_formats_total_breakdown_and_cache() {
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

    pub(crate) fn token_summary_omits_empty_stats() {
        assert_eq!(format_token_summary(&SessionStats::default()), None);
    }

    pub(crate) fn session_stats_select_uses_fallbacks_for_old_schema() {
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
}
