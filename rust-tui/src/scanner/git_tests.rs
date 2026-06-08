use super::unique_working_dirs;

#[test]
fn unique_working_dirs_dedupes_and_skips_empty_paths() {
    let paths = unique_working_dirs(["/repo/a", "", " /repo/b ", "/repo/a"]);

    assert_eq!(paths, vec!["/repo/a".to_string(), "/repo/b".to_string()]);
}
