use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub(super) struct TempBackup {
    root: PathBuf,
}

impl TempBackup {
    pub(super) fn create() -> io::Result<Self> {
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
