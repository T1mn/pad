use super::*;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_db_path() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("pad-opencode-preview-{stamp}.db"))
}

#[test]
fn parses_opencode_sqlite_messages_into_turns() {
    let path = temp_db_path();
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch(
            r#"
            CREATE TABLE message (id text PRIMARY KEY, session_id text NOT NULL, time_created integer NOT NULL, data text NOT NULL);
            CREATE TABLE part (id text PRIMARY KEY, message_id text NOT NULL, session_id text NOT NULL, time_created integer NOT NULL, data text NOT NULL);
            "#,
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
            params!["prt_1", "msg_1", "ses_1", 1_i64, r#"{"type":"text","text":"hello"}"#],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO message (id, session_id, time_created, data) VALUES (?1, ?2, ?3, ?4)",
            params!["msg_2", "ses_1", 2_i64, r#"{"role":"assistant"}"#],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO part (id, message_id, session_id, time_created, data) VALUES (?1, ?2, ?3, ?4, ?5)",
            params!["prt_2", "msg_2", "ses_1", 2_i64, r#"{"type":"text","text":"world"}"#],
        )
        .unwrap();
    drop(connection);

    let turns = parse_transcript(
        &path,
        Some("ses_1"),
        super::super::SessionReadMode::FullBackfill,
    )
    .unwrap();
    std::fs::remove_file(&path).ok();

    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0].question, "hello");
    assert_eq!(turns[0].answer.as_deref(), Some("world"));
}
