use super::write_file_atomically;
use std::fs;

#[test]
fn atomic_write_removes_temp_after_rename_failure() {
    let root = crate::test_support::temp_path("pad-rollout-apply", "rename-failure");
    let target = root.join("rollout.jsonl");
    fs::create_dir_all(target.join("child")).expect("create non-file target");
    let permissions = fs::metadata(&target).expect("stat target").permissions();

    write_file_atomically(&target, b"secret rollout", permissions)
        .expect_err("rename over directory must fail");

    let temp_files = fs::read_dir(&root)
        .expect("read temp root")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().contains(".pad-sync"))
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    assert!(temp_files.is_empty(), "leftover temp files: {temp_files:?}");
    let _ = fs::remove_dir_all(root);
}
