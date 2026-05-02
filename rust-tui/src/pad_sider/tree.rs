use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct TreeRow {
    pub depth: usize,
    pub path: PathBuf,
    pub label: String,
    pub is_dir: bool,
    pub expanded: bool,
}

pub fn build_tree(root: &Path, expanded: &HashSet<PathBuf>) -> Vec<TreeRow> {
    let mut rows = Vec::new();
    walk(root, root, expanded, 0, &mut rows);
    rows
}

pub fn scan_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_files(root, &mut files);
    files.sort();
    files
}

fn walk(
    root: &Path,
    dir: &Path,
    expanded: &HashSet<PathBuf>,
    depth: usize,
    rows: &mut Vec<TreeRow>,
) {
    if depth == 0 {
        rows.push(TreeRow {
            depth,
            path: root.to_path_buf(),
            label: root
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("/")
                .to_string(),
            is_dir: true,
            expanded: true,
        });
    }
    if depth > 0 {
        rows.push(TreeRow {
            depth,
            path: dir.to_path_buf(),
            label: dir
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("/")
                .to_string(),
            is_dir: true,
            expanded: expanded.contains(dir),
        });
    }
    if depth > 0 && !expanded.contains(dir) {
        return;
    }

    let mut entries = match fs::read_dir(dir) {
        Ok(entries) => entries.filter_map(|entry| entry.ok()).collect::<Vec<_>>(),
        Err(_) => return,
    };
    entries.sort_by(|left, right| {
        let left_dir = left
            .file_type()
            .map(|value| value.is_dir())
            .unwrap_or(false);
        let right_dir = right
            .file_type()
            .map(|value| value.is_dir())
            .unwrap_or(false);
        match (left_dir, right_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => left.file_name().cmp(&right.file_name()),
        }
    });

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry
            .file_type()
            .map(|value| value.is_dir())
            .unwrap_or(false);
        if is_dir && skip_dir_name(&name) {
            continue;
        }
        if is_dir {
            walk(root, &path, expanded, depth + 1, rows);
        } else {
            rows.push(TreeRow {
                depth: depth + 1,
                path,
                label: name,
                is_dir: false,
                expanded: false,
            });
        }
    }
}

fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry
            .file_type()
            .map(|value| value.is_dir())
            .unwrap_or(false);
        if is_dir {
            if skip_dir_name(&name) {
                continue;
            }
            collect_files(&path, files);
        } else {
            files.push(path);
        }
    }
}

fn skip_dir_name(name: &str) -> bool {
    matches!(
        name,
        ".git" | "node_modules" | "target" | "dist" | "build" | "__pycache__"
    )
}

#[cfg(test)]
mod tests {
    use super::scan_files;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn scan_files_skips_ignored_directories() {
        let root = temp_dir("scan_files_skips_ignored_directories");
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(root.join("docs/readme.md"), "# ok").unwrap();
        fs::write(root.join(".git/config"), "ignored").unwrap();

        let files = scan_files(&root);
        assert!(files.iter().any(|path| path.ends_with("docs/readme.md")));
        assert!(!files.iter().any(|path| path.ends_with(".git/config")));

        fs::remove_dir_all(root).unwrap();
    }

    fn temp_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("pad_sider_{name}_{unique}"))
    }
}
