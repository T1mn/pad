use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Clone, Default)]
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

pub fn read_markdown_file(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|err| format!("failed to read {}: {err}", path.display()))
}

pub fn read_changed_files(root: &Path) -> Vec<String> {
    let output = Command::new("git")
        .args(["-C", &root.to_string_lossy(), "status", "--short"])
        .output();
    let Ok(output) = output else {
        return vec!["Not a git repository".into()];
    };
    if !output.status.success() {
        return vec!["Not a git repository".into()];
    }

    let lines = String::from_utf8_lossy(&output.stdout)
        .lines()
        .take(10)
        .map(|line| line.to_string())
        .collect::<Vec<_>>();
    if lines.is_empty() {
        vec!["Working tree clean".into()]
    } else {
        lines
    }
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
