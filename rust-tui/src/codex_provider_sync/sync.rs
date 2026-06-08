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
