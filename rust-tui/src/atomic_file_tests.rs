use super::write_private;

fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = crate::test_support::temp_path("pad-atomic-file", name);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

pub(crate) fn write_private_creates_missing_parent_dirs() {
    let dir = temp_dir("missing-parent");
    let path = dir.join("nested").join("deep").join("config.toml");

    write_private(&path, "theme = \"default\"\n").expect("atomic write");

    assert_eq!(
        std::fs::read_to_string(&path).expect("read back"),
        "theme = \"default\"\n"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

pub(crate) fn write_private_replaces_existing_content_without_leftover_temp_files() {
    let dir = temp_dir("replace");
    let path = dir.join("config.toml");
    std::fs::write(&path, "old = 1\nold = 2\nold = 3\n").expect("seed file");

    write_private(&path, "new = 1\n").expect("atomic write");

    assert_eq!(
        std::fs::read_to_string(&path).expect("read back"),
        "new = 1\n"
    );
    let leftovers: Vec<_> = std::fs::read_dir(&dir)
        .expect("read dir")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .filter(|name| name != "config.toml")
        .collect();
    assert!(leftovers.is_empty(), "leftover temp files: {leftovers:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(unix)]
pub(crate) fn write_private_forces_owner_only_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let dir = temp_dir("permissions");
    let path = dir.join("config.toml");
    std::fs::write(&path, "old\n").expect("seed file");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).expect("loosen perms");

    write_private(&path, "new\n").expect("atomic write");

    let mode = std::fs::metadata(&path).expect("stat").permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "config must not be world readable");
    let _ = std::fs::remove_dir_all(&dir);
}
