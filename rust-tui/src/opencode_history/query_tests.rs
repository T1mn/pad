use super::query_threads_at;
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_db_path(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("pad-opencode-{name}-{stamp}.db"))
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

#[test]
fn query_threads_supports_older_opencode_schema_without_stats() {
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

#[test]
fn query_threads_reads_opencode_sqlite() {
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
