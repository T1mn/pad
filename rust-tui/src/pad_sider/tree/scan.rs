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
