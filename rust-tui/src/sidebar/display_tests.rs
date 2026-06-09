use super::{clean_title, folder_display_label};

#[test]
fn clean_title_trims_and_keeps_first_line() {
    assert_eq!(clean_title("  hello\nworld  ").as_deref(), Some("hello"));
    assert_eq!(clean_title(" \n\t "), None);
}

#[test]
fn folder_display_label_includes_parent_leaf() {
    let path = std::path::Path::new("workspace").join("project");
    assert_eq!(
        folder_display_label(&path.to_string_lossy()),
        "project · workspace"
    );
}
