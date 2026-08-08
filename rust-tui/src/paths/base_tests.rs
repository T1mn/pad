use super::{resolve_pad_home_dir, terminal_workspace_path};
use std::path::PathBuf;

#[test]
fn explicit_pad_home_is_used_without_rewriting_process_home() {
    assert_eq!(
        resolve_pad_home_dir(
            Some(PathBuf::from("/tmp/pad-isolated")),
            Some(PathBuf::from("/users/example")),
            None,
        ),
        PathBuf::from("/tmp/pad-isolated")
    );
    assert_eq!(
        resolve_pad_home_dir(None, Some(PathBuf::from("/users/example")), None),
        PathBuf::from("/users/example/.pad")
    );
}

#[test]
fn terminal_workspace_lives_under_pad_home() {
    let path = terminal_workspace_path();
    assert_eq!(
        path.file_name().and_then(|name| name.to_str()),
        Some("terminal-workspace.json")
    );
}
