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
    let mut dirs = entries
        .flatten()
        .filter(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
        .filter(|entry| !skip_dir_name(&entry.file_name().to_string_lossy()))
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    dirs.sort();

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

fn skip_dir_name(name: &str) -> bool {
    matches!(
        name,
        ".git" | "node_modules" | "target" | "dist" | "build" | "__pycache__"
    )
}

#[cfg(test)]
mod tests {
    use super::build_index_map;
    use crate::test_support;
    use std::fs;

    #[test]
    fn builds_nested_index_map_and_skips_ignored_dirs() {
        let root = temp_dir("builds_nested_index_map_and_skips_ignored_dirs");
        fs::create_dir_all(root.join("docs/guide")).unwrap();
        fs::create_dir_all(root.join("target/hidden")).unwrap();
        fs::write(root.join("index.md"), "# root").unwrap();
        fs::write(root.join("docs/index.md"), "# docs").unwrap();
        fs::write(root.join("docs/guide/index.md"), "# guide").unwrap();
        fs::write(root.join("target/hidden/index.md"), "# hidden").unwrap();

        let rows = build_index_map(&root);
        let labels = rows
            .iter()
            .map(|row| (row.depth, row.dir_label.as_str()))
            .collect::<Vec<_>>();

        assert_eq!(labels, vec![(0, "."), (1, "docs"), (2, "guide")]);
        assert!(!rows
            .iter()
            .any(|row| row.path.ends_with("target/hidden/index.md")));

        fs::remove_dir_all(root).unwrap();
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        test_support::temp_path("pad_sider_index_map", name)
    }
}
