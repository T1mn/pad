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
