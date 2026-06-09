use super::build_tree;
use crate::test_support;
use std::collections::HashSet;
use std::fs;

#[test]
fn build_tree_skips_ignored_directories_before_rows() {
    let root = test_support::temp_path("pad_sider_tree", "build_tree_skips_ignored_directories");
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::create_dir_all(root.join("target/hidden")).unwrap();
    fs::write(root.join("docs/readme.md"), "# ok").unwrap();
    fs::write(root.join("target/hidden/readme.md"), "# hidden").unwrap();

    let rows = build_tree(&root, &HashSet::new());

    assert!(rows.iter().any(|row| row.path.ends_with("docs")));
    assert!(!rows.iter().any(|row| row.path.ends_with("target")));
    assert!(!rows
        .iter()
        .any(|row| row.path.ends_with("target/hidden/readme.md")));

    fs::remove_dir_all(root).unwrap();
}
