use super::mode::ExportMode;
use std::path::{Path, PathBuf};

pub(super) fn opencode_export_path(session_id: &str, dir: &Path, mode: ExportMode) -> PathBuf {
    let suffix = match mode {
        ExportMode::Raw => "json",
        ExportMode::Sanitized => "sanitized.json",
    };
    dir.join(format!(
        "{}.{}",
        super::super::opencode_cli::safe_filename(session_id),
        suffix
    ))
}
