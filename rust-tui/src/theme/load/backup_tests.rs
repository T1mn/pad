use super::preserve_broken_config;

fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = crate::test_support::temp_path("pad-config-backup", name);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

pub(crate) fn backup_reuses_slot_for_identical_content_and_advances_for_new_damage() {
    let dir = temp_dir("slots");
    let path = dir.join("config.toml");
    std::fs::write(&path, "broken = [").expect("write broken config");

    let first = preserve_broken_config(&path).expect("first backup");
    assert_eq!(first, dir.join("config.toml.bak"));
    let again = preserve_broken_config(&path).expect("same content backup");
    assert_eq!(
        again, first,
        "identical content must not spawn a new backup"
    );

    std::fs::write(&path, "other = [").expect("write different damage");
    let second = preserve_broken_config(&path).expect("second backup");
    assert_eq!(second, dir.join("config.toml.bak.1"));
    assert_eq!(
        std::fs::read_to_string(&first).expect("read first backup"),
        "broken = ["
    );

    let _ = std::fs::remove_dir_all(&dir);
}

pub(crate) fn backup_returns_none_when_source_is_unreadable() {
    let dir = temp_dir("missing-source");
    assert!(preserve_broken_config(&dir.join("nope.toml")).is_none());
    let _ = std::fs::remove_dir_all(&dir);
}
