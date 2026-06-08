use super::scan_files;
use crate::test_support;
use std::fs;

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

fn temp_dir(name: &str) -> std::path::PathBuf {
    test_support::temp_path("pad_sider", name)
}
