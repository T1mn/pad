use crate::log_debug;
use std::io;
use std::path::Path;
use std::sync::{mpsc, OnceLock};

mod backup;
#[cfg(test)]
mod helpers;
mod rollout;
mod state_db;
#[cfg(test)]
mod tests;

use backup::TempBackup;
use rollout::{apply_rollout_changes, collect_rollout_changes};
use state_db::{update_sqlite_provider, STATE_DB_BASENAME};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ProviderSyncResult {
    pub updated_rollout_files: usize,
    pub updated_sqlite_rows: usize,
}

pub fn sync_to_provider(target_provider: &str) -> io::Result<ProviderSyncResult> {
    let codex_home = crate::paths::pad_codex_home_dir();
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
