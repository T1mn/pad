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
mod helpers;
mod model;
mod rollout;
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
mod tests;
mod worker;

pub use model::ProviderSyncResult;
pub use worker::enqueue_sync_to_provider;

pub fn sync_to_provider(target_provider: &str) -> io::Result<ProviderSyncResult> {
    let codex_home = crate::paths::pad_codex_home_dir();
    sync_to_provider_at(&codex_home, target_provider)
}

pub(crate) use sync::sync_to_provider_at;
