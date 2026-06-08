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
