use crate::log_debug;
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const SESSION_DIRS: [&str; 2] = ["sessions", "archived_sessions"];
const STATE_DB_BASENAME: &str = "state_5.sqlite";

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ProviderSyncResult {
    pub updated_rollout_files: usize,
    pub updated_sqlite_rows: usize,
}

#[derive(Clone, Debug)]
struct RolloutChange {
    path: PathBuf,
    original_first_line: String,
    original_separator: String,
    updated_first_line: String,
}

#[derive(Debug)]
struct TempBackup {
    root: PathBuf,
}

impl TempBackup {
    fn create() -> io::Result<Self> {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let root = std::env::temp_dir().join(format!(
            "pad-codex-provider-sync-{}-{stamp}",
            std::process::id()
        ));
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    fn backup_file(&self, codex_home: &Path, file_path: &Path) -> io::Result<()> {
        if !file_path.exists() {
            return Ok(());
        }
        let relative = file_path.strip_prefix(codex_home).unwrap_or(file_path);
        let backup_path = self.root.join(relative);
        if let Some(parent) = backup_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(file_path, backup_path)?;
        Ok(())
    }

    fn restore_file(&self, codex_home: &Path, file_path: &Path) -> io::Result<()> {
        let relative = file_path.strip_prefix(codex_home).unwrap_or(file_path);
        let backup_path = self.root.join(relative);
        if !backup_path.exists() {
            return Ok(());
        }
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(backup_path, file_path)?;
        Ok(())
    }

    fn cleanup(self) {
        let _ = fs::remove_dir_all(self.root);
    }
}

pub fn sync_to_provider(target_provider: &str) -> io::Result<ProviderSyncResult> {
    let codex_home = dirs::home_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "home directory not found"))?
        .join(".codex");
    sync_to_provider_at(&codex_home, target_provider)
}

pub fn enqueue_sync_to_provider(target_provider: String) {
    let target_provider = target_provider.trim().to_string();
    if target_provider.is_empty() {
        return;
    }

    let sender = provider_sync_sender();
    if let Err(err) = sender.send(target_provider) {
        log_debug!(
            "codex_provider_sync: failed to enqueue background sync: {}",
            err
        );
    }
}

fn provider_sync_sender() -> &'static mpsc::Sender<String> {
    static SENDER: OnceLock<mpsc::Sender<String>> = OnceLock::new();
    SENDER.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<String>();
        std::thread::Builder::new()
            .name("pad-codex-provider-sync".to_string())
            .spawn(move || provider_sync_worker(rx))
            .expect("spawn provider sync worker");
        tx
    })
}

fn provider_sync_worker(rx: mpsc::Receiver<String>) {
    while let Ok(mut provider) = rx.recv() {
        while let Ok(next_provider) = rx.try_recv() {
            provider = next_provider;
        }
        match sync_to_provider(&provider) {
            Ok(result) => {
                log_debug!(
                    "codex_provider_sync: target_provider={} rollout_files={} sqlite_rows={}",
                    provider,
                    result.updated_rollout_files,
                    result.updated_sqlite_rows
                );
            }
            Err(err) => {
                log_debug!(
                    "codex_provider_sync: FAILED target_provider={} err={}",
                    provider,
                    err
                );
            }
        }
    }
}

pub(crate) fn sync_to_provider_at(
    codex_home: &Path,
    target_provider: &str,
) -> io::Result<ProviderSyncResult> {
    let target_provider = target_provider.trim();
    if target_provider.is_empty() || !codex_home.exists() {
        return Ok(ProviderSyncResult::default());
    }

    let rollout_changes = collect_rollout_changes(codex_home, target_provider)?;
    let sqlite_path = codex_home.join(STATE_DB_BASENAME);
    let needs_sqlite_backup = sqlite_path.exists();

    if rollout_changes.is_empty() && !needs_sqlite_backup {
        return Ok(ProviderSyncResult::default());
    }

    let backup = TempBackup::create()?;
    for change in &rollout_changes {
        backup.backup_file(codex_home, &change.path)?;
    }
    if needs_sqlite_backup {
        backup.backup_file(codex_home, &sqlite_path)?;
    }

    let result = (|| {
        let updated_sqlite_rows = update_sqlite_provider(&sqlite_path, target_provider)?;
        let updated_rollout_files = apply_rollout_changes(&rollout_changes)?;
        Ok(ProviderSyncResult {
            updated_rollout_files,
            updated_sqlite_rows,
        })
    })();

    match result {
        Ok(result) => {
            backup.cleanup();
            Ok(result)
        }
        Err(err) => {
            for change in &rollout_changes {
                let _ = backup.restore_file(codex_home, &change.path);
            }
            if needs_sqlite_backup {
                let _ = backup.restore_file(codex_home, &sqlite_path);
            }
            backup.cleanup();
            Err(err)
        }
    }
}

fn collect_rollout_changes(
    codex_home: &Path,
    target_provider: &str,
) -> io::Result<Vec<RolloutChange>> {
    let mut changes = Vec::new();

    for scope in SESSION_DIRS {
        let root = codex_home.join(scope);
        if !root.exists() {
            continue;
        }
        collect_rollout_changes_in_dir(&root, target_provider, &mut changes)?;
    }

    Ok(changes)
}

fn collect_rollout_changes_in_dir(
    dir: &Path,
    target_provider: &str,
    changes: &mut Vec<RolloutChange>,
) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect_rollout_changes_in_dir(&path, target_provider, changes)?;
            continue;
        }
        if !entry.file_type()?.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !file_name.starts_with("rollout-") || !file_name.ends_with(".jsonl") {
            continue;
        }

        let content = fs::read_to_string(&path)?;
        let Some((first_line, separator, _rest)) = split_first_line(&content) else {
            continue;
        };
        let Some(updated_first_line) = rewrite_rollout_first_line(first_line, target_provider)?
        else {
            continue;
        };
        changes.push(RolloutChange {
            path,
            original_first_line: first_line.to_string(),
            original_separator: separator.to_string(),
            updated_first_line,
        });
    }

    Ok(())
}

fn rewrite_rollout_first_line(
    first_line: &str,
    target_provider: &str,
) -> io::Result<Option<String>> {
    if first_line.trim().is_empty() {
        return Ok(None);
    }

    let mut value = match serde_json::from_str::<Value>(first_line) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let is_session_meta = value
        .get("type")
        .and_then(Value::as_str)
        .is_some_and(|value| value == "session_meta");
    if !is_session_meta {
        return Ok(None);
    }

    let Some(payload) = value.get_mut("payload").and_then(Value::as_object_mut) else {
        return Ok(None);
    };

    let current_provider = payload
        .get("model_provider")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if current_provider == target_provider {
        return Ok(None);
    }

    payload.insert(
        "model_provider".to_string(),
        Value::String(target_provider.to_string()),
    );
    serde_json::to_string(&value)
        .map(Some)
        .map_err(|err| io::Error::other(err.to_string()))
}

fn apply_rollout_changes(changes: &[RolloutChange]) -> io::Result<usize> {
    let mut updated = 0usize;
    for change in changes {
        apply_rollout_change(change)?;
        updated += 1;
    }
    Ok(updated)
}

fn apply_rollout_change(change: &RolloutChange) -> io::Result<()> {
    let current = fs::read_to_string(&change.path)?;
    let Some((first_line, separator, rest)) = split_first_line(&current) else {
        return Err(io::Error::other(format!(
            "rollout file is missing first line: {}",
            change.path.display()
        )));
    };

    if first_line != change.original_first_line || separator != change.original_separator {
        return Err(io::Error::other(format!(
            "rollout file changed during provider sync: {}",
            change.path.display()
        )));
    }

    let updated_content = format!(
        "{}{}{}",
        change.updated_first_line, change.original_separator, rest
    );
    let tmp_path = change
        .path
        .with_extension(format!("jsonl.pad-sync.{}", std::process::id()));
    fs::write(&tmp_path, updated_content)?;
    fs::rename(tmp_path, &change.path)?;
    Ok(())
}

fn split_first_line(content: &str) -> Option<(&str, &str, &str)> {
    if let Some(index) = content.find("\r\n") {
        let rest = &content[index + 2..];
        return Some((&content[..index], "\r\n", rest));
    }
    if let Some(index) = content.find('\n') {
        let rest = &content[index + 1..];
        return Some((&content[..index], "\n", rest));
    }
    Some((content, "", ""))
}

fn update_sqlite_provider(sqlite_path: &Path, target_provider: &str) -> io::Result<usize> {
    if !sqlite_path.exists() {
        return Ok(0);
    }

    let connection = Connection::open_with_flags(
        sqlite_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(to_io_error)?;
    connection
        .pragma_update(None, "busy_timeout", 5000_i64)
        .map_err(to_io_error)?;
    connection
        .execute_batch("BEGIN IMMEDIATE")
        .map_err(to_io_error)?;

    let result = connection.execute(
        "UPDATE threads
         SET model_provider = ?1
         WHERE COALESCE(model_provider, '') <> ?1",
        [target_provider],
    );

    match result {
        Ok(updated) => {
            connection.execute_batch("COMMIT").map_err(to_io_error)?;
            Ok(updated)
        }
        Err(err) => {
            let _ = connection.execute_batch("ROLLBACK");
            Err(to_io_error(err))
        }
    }
}

fn to_io_error(err: rusqlite::Error) -> io::Error {
    io::Error::other(err)
}

#[cfg(test)]
mod tests {
    use super::sync_to_provider_at;
    use rusqlite::Connection;

    fn temp_codex_home(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "pad-codex-provider-sync-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create temp codex home");
        path
    }

    fn write_rollout(path: &std::path::Path, thread_id: &str, provider: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create rollout dir");
        }
        let first_line = serde_json::json!({
            "timestamp": "2026-04-10T00:00:00Z",
            "type": "session_meta",
            "payload": {
                "id": thread_id,
                "model_provider": provider,
                "cwd": "/repo"
            }
        });
        std::fs::write(
            path,
            format!(
                "{}\n{{\"type\":\"event_msg\",\"payload\":{{}}}}\n",
                first_line
            ),
        )
        .expect("write rollout");
    }

    fn write_state_db(path: &std::path::Path) {
        let connection = Connection::open(path).expect("open db");
        connection
            .execute_batch(
                "CREATE TABLE threads (
                    id TEXT PRIMARY KEY,
                    model_provider TEXT NOT NULL,
                    archived INTEGER NOT NULL DEFAULT 0
                );
                INSERT INTO threads (id, model_provider, archived) VALUES
                    ('thread-a', 'old', 0),
                    ('thread-b', 'older', 1);",
            )
            .expect("seed db");
    }

    #[test]
    fn sync_updates_rollout_files_and_sqlite_provider() {
        let codex_home = temp_codex_home("success");
        write_rollout(
            &codex_home.join("sessions/2026/04/10/rollout-a.jsonl"),
            "thread-a",
            "old",
        );
        write_rollout(
            &codex_home.join("archived_sessions/2026/04/09/rollout-b.jsonl"),
            "thread-b",
            "older",
        );
        write_state_db(&codex_home.join("state_5.sqlite"));

        let result = sync_to_provider_at(&codex_home, "openai").expect("sync provider");

        assert_eq!(
            result,
            super::ProviderSyncResult {
                updated_rollout_files: 2,
                updated_sqlite_rows: 2,
            }
        );

        let session_rollout =
            std::fs::read_to_string(codex_home.join("sessions/2026/04/10/rollout-a.jsonl"))
                .expect("read rollout");
        assert!(session_rollout.contains("\"model_provider\":\"openai\""));
        assert!(session_rollout.contains("\"type\":\"event_msg\""));

        let archived_rollout = std::fs::read_to_string(
            codex_home.join("archived_sessions/2026/04/09/rollout-b.jsonl"),
        )
        .expect("read archived rollout");
        assert!(archived_rollout.contains("\"model_provider\":\"openai\""));

        let connection = Connection::open(codex_home.join("state_5.sqlite")).expect("open db");
        let providers = connection
            .prepare("SELECT model_provider FROM threads ORDER BY id")
            .expect("prepare query")
            .query_map([], |row| row.get::<_, String>(0))
            .expect("query providers")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect providers");
        assert_eq!(providers, vec!["openai".to_string(), "openai".to_string()]);

        let _ = std::fs::remove_dir_all(&codex_home);
    }

    #[test]
    fn sync_skips_when_state_db_is_missing() {
        let codex_home = temp_codex_home("no-db");
        write_rollout(
            &codex_home.join("sessions/2026/04/10/rollout-a.jsonl"),
            "thread-a",
            "old",
        );

        let result = sync_to_provider_at(&codex_home, "openai").expect("sync provider");

        assert_eq!(result.updated_rollout_files, 1);
        assert_eq!(result.updated_sqlite_rows, 0);

        let rollout =
            std::fs::read_to_string(codex_home.join("sessions/2026/04/10/rollout-a.jsonl"))
                .expect("read rollout");
        assert!(rollout.contains("\"model_provider\":\"openai\""));

        let _ = std::fs::remove_dir_all(&codex_home);
    }
}
