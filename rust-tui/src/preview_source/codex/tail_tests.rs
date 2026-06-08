use super::{grow_tail_bytes, initial_tail_bytes, read_tail_lines};
use std::fs;

fn temp_path(name: &str) -> std::path::PathBuf {
    crate::test_support::temp_path("pad-codex-tail", name).with_extension("jsonl")
}

#[test]
fn tail_window_helpers_clamp_and_grow() {
    assert_eq!(initial_tail_bytes(0), 1);
    assert_eq!(initial_tail_bytes(10), 10);
    assert_eq!(initial_tail_bytes(512), 256);
    assert_eq!(grow_tail_bytes(256, 400), 400);
}

#[test]
fn tail_reader_keeps_whole_file_when_short() {
    let path = temp_path("short");
    fs::write(&path, "one\ntwo\n").unwrap();

    let lines = read_tail_lines(&path, 8, 8).unwrap();
    fs::remove_file(&path).ok();

    assert_eq!(lines, vec!["one".to_string(), "two".to_string()]);
}

#[test]
fn tail_reader_drops_partial_first_line() {
    let path = temp_path("partial");
    fs::write(&path, "first\nsecond\nthird\n").unwrap();

    let lines = read_tail_lines(&path, 19, 13).unwrap();
    fs::remove_file(&path).ok();

    assert_eq!(lines, vec!["second".to_string(), "third".to_string()]);
}
