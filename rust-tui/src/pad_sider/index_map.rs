use super::ignore::skip_dir_name;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexRow {
    pub depth: usize,
    pub dir_label: String,
    pub path: PathBuf,
}

pub fn build_index_map(root: &Path) -> Vec<IndexRow> {
    let mut rows = Vec::new();
    collect_index_rows(root, root, 0, &mut rows);
    rows
}

fn collect_index_rows(root: &Path, dir: &Path, depth: usize, rows: &mut Vec<IndexRow>) {
    let index_path = dir.join("index.md");
    if index_path.is_file() {
        rows.push(IndexRow {
            depth,
            dir_label: dir_label(root, dir),
            path: index_path,
        });
    }

    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut dirs = Vec::new();
    for entry in entries.flatten() {
        if !entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name();
        if skip_dir_name(&name.to_string_lossy()) {
            continue;
        }
        dirs.push(entry.path());
    }
    dirs.sort_unstable();

    for child in dirs {
        collect_index_rows(root, &child, depth + 1, rows);
    }
}

fn dir_label(root: &Path, dir: &Path) -> String {
    if dir == root {
        return ".".into();
    }
    dir.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(".")
        .to_string()
}

#[cfg(test)]
#[path = "index_map_tests.rs"]
mod tests;
