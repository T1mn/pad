use super::super::ignore::skip_dir_name;
use super::TreeRow;
use std::collections::HashSet;
use std::ffi::OsString;
use std::fs::{self, DirEntry};
use std::path::{Path, PathBuf};

pub fn build_tree(root: &Path, expanded: &HashSet<PathBuf>) -> Vec<TreeRow> {
    let mut rows = Vec::new();
    walk(root, root, expanded, 0, &mut rows);
    rows
}

fn walk(
    root: &Path,
    dir: &Path,
    expanded: &HashSet<PathBuf>,
    depth: usize,
    rows: &mut Vec<TreeRow>,
) {
    push_dir_row(root, dir, expanded, depth, rows);
    if depth > 0 && !expanded.contains(dir) {
        return;
    }

    for entry in sorted_entries(dir) {
        if entry.is_dir {
            walk(root, &entry.path, expanded, depth + 1, rows);
        } else {
            rows.push(TreeRow {
                depth: depth + 1,
                path: entry.path,
                label: entry.name,
                is_dir: false,
                expanded: false,
            });
        }
    }
}

fn push_dir_row(
    root: &Path,
    dir: &Path,
    expanded: &HashSet<PathBuf>,
    depth: usize,
    rows: &mut Vec<TreeRow>,
) {
    rows.push(TreeRow {
        depth,
        path: dir.to_path_buf(),
        label: dir_label(root, dir),
        is_dir: true,
        expanded: depth == 0 || expanded.contains(dir),
    });
}

fn dir_label(root: &Path, dir: &Path) -> String {
    path_file_label(if dir == root { root } else { dir })
}

fn path_file_label(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("/")
        .to_string()
}

struct TreeEntry {
    path: PathBuf,
    name: String,
    sort_name: OsString,
    is_dir: bool,
}

fn sorted_entries(dir: &Path) -> Vec<TreeEntry> {
    let mut entries = match fs::read_dir(dir) {
        Ok(entries) => entries.filter_map(read_tree_entry).collect::<Vec<_>>(),
        Err(_) => return Vec::new(),
    };
    entries.sort_by(|left, right| match (left.is_dir, right.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => left.sort_name.cmp(&right.sort_name),
    });
    entries
}

fn read_tree_entry(entry: Result<DirEntry, std::io::Error>) -> Option<TreeEntry> {
    let entry = entry.ok()?;
    let sort_name = entry.file_name();
    let is_dir = entry
        .file_type()
        .map(|value| value.is_dir())
        .unwrap_or(false);
    let name = {
        let name = sort_name.to_string_lossy();
        if is_dir && skip_dir_name(&name) {
            return None;
        }
        name.into_owned()
    };
    Some(TreeEntry {
        path: entry.path(),
        name,
        sort_name,
        is_dir,
    })
}

#[cfg(test)]
#[path = "build_tests.rs"]
mod tests;
