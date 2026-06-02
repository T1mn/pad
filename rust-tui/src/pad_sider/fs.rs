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
    String::from_utf8_lossy(&bytes).to_string()
}

pub fn read_file_stats(path: &Path) -> FileStats {
    let metadata = fs::metadata(path).ok();
    let bytes = metadata.as_ref().map(|value| value.len()).unwrap_or(0);
    let modified = metadata
        .and_then(|value| value.modified().ok())
        .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|value| format!("{}", value.as_secs()))
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
mod tests {
    use super::read_text_file;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn read_text_file_returns_full_file_without_preview_truncation() {
        let path = temp_file("full_file");
        let body = "x".repeat(260 * 1024);
        fs::write(&path, &body).unwrap();

        let read = read_text_file(&path);

        assert_eq!(read.len(), body.len());
        assert!(!read.contains("truncated preview"));
        fs::remove_file(path).unwrap();
    }

    fn temp_file(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("pad_sider_fs_{name}_{unique}"))
    }
}
