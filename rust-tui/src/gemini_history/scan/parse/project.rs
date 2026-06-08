use crate::gemini_history::util::normalize_path;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub(super) fn project_root_for_session_path(path: &Path) -> io::Result<(PathBuf, String)> {
    let project_dir = path
        .parent()
        .and_then(|parent| parent.parent())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid Gemini session path: {}", path.display()),
            )
        })?;
    let project_alias = project_dir
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| String::from("unknown"));
    let project_root =
        read_project_root_file(project_dir).unwrap_or_else(|| normalize_path(project_dir));
    Ok((project_root, project_alias))
}

fn read_project_root_file(project_dir: &Path) -> Option<PathBuf> {
    fs::read_to_string(project_dir.join(".project_root"))
        .ok()
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
        .map(PathBuf::from)
        .map(|path| normalize_path(&path))
}
