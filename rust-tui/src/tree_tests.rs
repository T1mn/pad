use super::{FileTree, PreviewType};
use std::fs;

#[test]
fn search_filters_entries_case_insensitively() {
    let root = crate::test_support::temp_path("pad-tree", "search-case");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("Alpha.md"), "alpha").unwrap();
    fs::write(root.join("beta.md"), "beta").unwrap();

    let mut tree = FileTree::new(root.clone());
    tree.start_search();
    for ch in "alpha".chars() {
        tree.search_input(ch);
    }

    let names = tree
        .entries
        .iter()
        .map(|entry| entry.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(names, vec!["Alpha.md"]);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn preview_type_detects_known_suffixes_case_insensitively() {
    assert_eq!(
        PreviewType::from_path(std::path::Path::new("README.MD")),
        PreviewType::Markdown
    );
    assert_eq!(
        PreviewType::from_path(std::path::Path::new("diagram.PNG")),
        PreviewType::Image
    );
    assert_eq!(
        PreviewType::from_path(std::path::Path::new("archive.BIN")),
        PreviewType::Binary
    );
    assert_eq!(
        PreviewType::from_path(std::path::Path::new(".GITIGNORE")),
        PreviewType::Text
    );
}
