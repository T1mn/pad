use super::{
    harden_private_tree, pad_desktop_store_path, resolve_pad_home_dir, terminal_workspace_path,
    validate_pad_desktop_data_root_with_inputs,
};
use std::path::{Path, PathBuf};

pub(crate) fn explicit_pad_home_is_used_without_rewriting_process_home() {
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

pub(crate) fn terminal_workspace_lives_under_pad_home() {
    let path = terminal_workspace_path();
    assert_eq!(
        path.file_name().and_then(|name| name.to_str()),
        Some("terminal-workspace.json")
    );
}

pub(crate) fn desktop_store_uses_a_separate_application_data_suffix() {
    let path = pad_desktop_store_path();
    assert_eq!(
        path.file_name().and_then(|name| name.to_str()),
        Some("pad.sqlite")
    );
    assert!(path.ends_with("v1/store/pad.sqlite"));
}

pub(crate) fn desktop_data_root_is_rejected_before_touching_provider_or_broad_paths() {
    let home = Path::new("/Users/example");
    for unsafe_root in [
        "/",
        "/Users/example",
        "/Users/example/.codex",
        "/Users/example/.pi/desktop",
        "/Users/example/.pad",
        "/Users/example/.chatgpt/sessions",
        "/Users/example/Library/Application Support/com.openai.codex/child",
        "/Users/example/Library/Application Support/Codex/Session Storage",
        "/Users/example/Library/Application Support/OpenAI/Codex/session",
        "/Users/example/Library/Application Support/ChatGPT/session",
        "/Users/example/Library/Containers/com.openai.chat",
        "/Users/example/Library/Group Containers/2DC432GLL2.com.openai.codex.notifications/state",
        "/Users/example/Library/Group Containers/2DC432GLL2.com.openai.sky.CUAService/state",
        "/Users/example/Library/Caches/Codex/Cache_Data",
        "/Users/example/Library/HTTPStorages/com.openai.codex.binarycookies",
        "/Users/example/Library/Preferences/com.openai.codex.plist",
        "/Users/example/custom-codex-home/session",
        "/Users",
    ] {
        let result = validate_pad_desktop_data_root_with_inputs(
            Path::new(unsafe_root),
            Some(home),
            Some(Path::new("/Users/example/.pad")),
            Some(Path::new("/Users/example/custom-codex-home")),
        );
        assert!(
            result.is_err(),
            "unsafe data root was accepted: {unsafe_root}"
        );
    }
}

pub(crate) fn desktop_data_root_accepts_only_a_scoped_safe_directory_without_creating_it() {
    let parent = std::env::temp_dir().join(format!(
        "pad-data-root-validation-{}-{}",
        std::process::id(),
        crate::time::unix_now_nanos()
    ));
    std::fs::create_dir_all(&parent).unwrap();
    let root = parent.join("nested").join("PAD Desktop");
    let resolved = validate_pad_desktop_data_root_with_inputs(
        &root,
        Some(Path::new("/Users/example")),
        Some(Path::new("/Users/example/.pad")),
        Some(Path::new("/Users/example/custom-codex-home")),
    )
    .unwrap();
    assert!(resolved.is_absolute());
    assert!(!root.exists(), "validation mutated the candidate data root");
    let _ = std::fs::remove_dir_all(parent);
}

pub(crate) fn private_tree_repairs_directory_and_file_modes_without_following_symlinks() {
    #[cfg(not(unix))]
    return;

    #[cfg(unix)]
    {
        use std::fs;
        use std::os::unix::fs::{symlink, PermissionsExt};

        let root = std::env::temp_dir().join(format!(
            "pad-private-tree-{}-{}",
            std::process::id(),
            crate::time::unix_now_nanos()
        ));
        let nested = root.join("profile/sessions");
        let outside = root.with_extension("outside");
        fs::create_dir_all(&nested).unwrap();
        fs::create_dir_all(&outside).unwrap();
        let session = nested.join("session.jsonl");
        let outside_file = outside.join("keep.txt");
        fs::write(&session, "{}\n").unwrap();
        fs::write(&outside_file, "outside\n").unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
        fs::set_permissions(&nested, fs::Permissions::from_mode(0o755)).unwrap();
        fs::set_permissions(&session, fs::Permissions::from_mode(0o644)).unwrap();
        fs::set_permissions(&outside_file, fs::Permissions::from_mode(0o644)).unwrap();
        symlink(&outside, root.join("outside-link")).unwrap();

        harden_private_tree(&root).unwrap();

        assert_eq!(
            fs::metadata(&root).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(&nested).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(&session).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(&outside_file).unwrap().permissions().mode() & 0o777,
            0o644,
            "symlink target outside the private root must not be changed"
        );

        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&outside);
    }
}
