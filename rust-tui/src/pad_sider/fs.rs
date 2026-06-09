use std::fs;
use std::path::Path;

#[derive(Clone, Default, PartialEq, Eq)]
pub struct FileStats {
    pub lines: usize,
    pub bytes: u64,
    pub modified: String,
}

pub fn relative_path_label(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .ok()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or(".")
        .to_string()
}

pub fn is_markdown_file(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| value.eq_ignore_ascii_case("md"))
        .unwrap_or(false)
}

pub fn read_text_file(path: &Path) -> String {
    let bytes = fs::read(path)
        .unwrap_or_else(|err| format!("failed to read {}: {err}", path.display()).into_bytes());
    String::from_utf8_lossy(&bytes).into_owned()
}

pub fn read_file_stats(path: &Path) -> FileStats {
    let metadata = fs::metadata(path).ok();
    let bytes = metadata.as_ref().map(|value| value.len()).unwrap_or(0);
    let modified = metadata
        .and_then(|value| value.modified().ok())
        .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|value| value.as_secs().to_string())
        .unwrap_or_else(|| "-".into());
    let lines = fs::read_to_string(path)
        .map(|text| text.lines().count())
        .unwrap_or(0);

    FileStats {
        lines,
        bytes,
        modified,
    }
}

#[cfg(test)]
#[path = "fs_tests.rs"]
mod tests;
