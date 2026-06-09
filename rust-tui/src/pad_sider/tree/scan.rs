use super::ignore::skip_dir_name;
use std::fs;
use std::path::{Path, PathBuf};

pub fn scan_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_files(root, &mut files);
    files.sort();
    files
}

fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let is_dir = entry
            .file_type()
            .map(|value| value.is_dir())
            .unwrap_or(false);
        if is_dir {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if skip_dir_name(&name) {
                continue;
            }
            collect_files(&path, files);
        } else {
            files.push(path);
        }
    }
}

#[cfg(test)]
#[path = "scan_tests.rs"]
mod tests;
