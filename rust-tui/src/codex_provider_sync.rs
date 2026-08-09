use std::io;

mod backup {
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_BACKUP_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[derive(Debug)]
    pub(super) struct TempBackup {
        root: PathBuf,
    }

    impl TempBackup {
        pub(super) fn create() -> io::Result<Self> {
            let stamp = crate::time::unix_now_nanos();
            let counter = TEMP_BACKUP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "pad-codex-provider-sync-{}-{stamp}-{counter}",
                std::process::id()
            ));
            fs::create_dir_all(&root)?;
            Ok(Self { root })
        }

        pub(super) fn backup_file(&self, codex_home: &Path, file_path: &Path) -> io::Result<()> {
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

        pub(super) fn restore_file(&self, codex_home: &Path, file_path: &Path) -> io::Result<()> {
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

        pub(super) fn cleanup(self) {
            let _ = fs::remove_dir_all(self.root);
        }
    }
}
#[cfg(test)]
mod helpers {
    use rusqlite::Connection;
    use std::path::{Path, PathBuf};

    pub(super) fn temp_codex_home(name: &str) -> PathBuf {
        let path = crate::test_support::temp_path("pad-codex-provider-sync", name);
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create temp codex home");
        path
    }

    pub(super) fn with_temp_home<T>(name: &str, f: impl FnOnce(&Path) -> T) -> T {
        crate::test_support::with_temp_home("pad-codex-provider-sync-home", name, f)
    }

    pub(super) fn write_rollout(path: &Path, thread_id: &str, provider: &str) {
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

    pub(super) fn write_state_db(path: &Path) {
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

    pub(super) fn rollout_text(home: &Path, relative: &str) -> String {
        std::fs::read_to_string(home.join(relative)).expect("read rollout")
    }
}
mod model {
    #[derive(Debug, Default, PartialEq, Eq)]
    pub struct ProviderSyncResult {
        pub updated_rollout_files: usize,
        pub updated_sqlite_rows: usize,
    }
}
pub(crate) mod rollout;
mod state_db {
    use rusqlite::{Connection, OpenFlags};
    use std::io;
    use std::path::Path;

    pub(super) const STATE_DB_BASENAME: &str = "state_5.sqlite";

    pub(super) fn update_sqlite_provider(
        sqlite_path: &Path,
        target_provider: &str,
    ) -> io::Result<usize> {
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
}
mod sync {
    use super::backup::TempBackup;
    use super::model::ProviderSyncResult;
    use super::rollout::{apply_rollout_changes, collect_rollout_changes};
    use super::state_db::{update_sqlite_provider, STATE_DB_BASENAME};
    use std::io;
    use std::path::Path;

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
}
#[cfg(test)]
pub(crate) mod tests;
mod worker {
    use crate::log_debug;
    use std::sync::{mpsc, OnceLock};

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
            match super::sync_to_provider(&provider) {
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
}

pub use model::ProviderSyncResult;
pub use worker::enqueue_sync_to_provider;

pub fn sync_to_provider(target_provider: &str) -> io::Result<ProviderSyncResult> {
    let codex_home = crate::paths::pad_codex_home_dir();
    sync_to_provider_at(&codex_home, target_provider)
}

pub(crate) use sync::sync_to_provider_at;
