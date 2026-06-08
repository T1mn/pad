use super::read_text_file;
use crate::test_support;
use std::fs;

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

fn temp_file(name: &str) -> std::path::PathBuf {
    test_support::temp_path("pad_sider_fs", name)
}
