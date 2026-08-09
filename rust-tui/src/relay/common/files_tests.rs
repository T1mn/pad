use super::preserve_backup;

fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = crate::test_support::temp_path("pad-relay-files", name);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

pub(crate) fn preserve_backup_creates_a_private_file() {
    let dir = temp_dir("create");
    let path = dir.join("provider-auth.json");

    preserve_backup(&path, "secret\n").expect("create backup");

    assert_eq!(
        std::fs::read_to_string(&path).expect("read backup"),
        "secret\n"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[cfg(unix)]
pub(crate) fn preserve_backup_tightens_existing_file_without_overwriting_it() {
    use std::os::unix::fs::PermissionsExt;

    let dir = temp_dir("existing");
    let path = dir.join("provider-auth.json");
    std::fs::write(&path, "original\n").expect("seed backup");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
        .expect("loosen backup permissions");

    preserve_backup(&path, "replacement\n").expect("secure existing backup");

    assert_eq!(
        std::fs::read_to_string(&path).expect("read backup"),
        "original\n"
    );
    assert_eq!(
        std::fs::metadata(&path)
            .expect("stat backup")
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[cfg(unix)]
pub(crate) fn preserve_backup_rejects_symlink_path() {
    use std::os::unix::fs::symlink;

    let dir = temp_dir("symlink");
    let target = dir.join("target");
    let link = dir.join("backup");
    std::fs::write(&target, "keep\n").expect("seed target");
    symlink(&target, &link).expect("create symlink");

    assert!(preserve_backup(&link, "replacement\n").is_err());
    assert_eq!(
        std::fs::read_to_string(&target).expect("read target"),
        "keep\n"
    );
    let _ = std::fs::remove_dir_all(dir);
}
