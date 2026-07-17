use super::{configured_db_paths_from, default_db_paths};
use std::ffi::OsString;
use std::path::PathBuf;

#[test]
fn configured_db_paths_uses_absolute_env_exclusively() {
    let paths = configured_db_paths_from(|key| match key {
        "OPENCODE_DB" => Some(OsString::from("/tmp/opencode/custom.db")),
        "XDG_DATA_HOME" => Some(OsString::from("/tmp/opencode-data")),
        "HOME" => Some(OsString::from("/tmp/home")),
        _ => None,
    });

    assert_eq!(paths, vec![PathBuf::from("/tmp/opencode/custom.db")]);
}

#[test]
fn configured_db_paths_resolves_relative_env_under_xdg_data() {
    let paths = configured_db_paths_from(|key| match key {
        "OPENCODE_DB" => Some(OsString::from("channels/custom.db")),
        "XDG_DATA_HOME" => Some(OsString::from("/tmp/opencode-data")),
        "HOME" => Some(OsString::from("/tmp/home")),
        _ => None,
    });

    assert_eq!(
        paths,
        vec![PathBuf::from(
            "/tmp/opencode-data/opencode/channels/custom.db"
        )]
    );
}

#[test]
fn configured_db_paths_skips_in_memory_database() {
    let paths = configured_db_paths_from(|key| match key {
        "OPENCODE_DB" => Some(OsString::from(":memory:")),
        "XDG_DATA_HOME" => Some(OsString::from("/tmp/opencode-data")),
        _ => None,
    });

    assert!(paths.is_empty());
}

#[test]
fn default_db_paths_does_not_fall_back_for_in_memory_override() {
    let _guard = crate::test_support::home_env_lock()
        .lock()
        .expect("lock environment for test");
    let previous = std::env::var_os("OPENCODE_DB");
    std::env::set_var("OPENCODE_DB", ":memory:");
    let paths = default_db_paths();
    if let Some(previous) = previous {
        std::env::set_var("OPENCODE_DB", previous);
    } else {
        std::env::remove_var("OPENCODE_DB");
    }

    assert!(paths.is_empty());
}

#[test]
fn configured_db_paths_keeps_default_discovery_without_env_override() {
    let paths = configured_db_paths_from(|key| match key {
        "XDG_DATA_HOME" => Some(OsString::from("/tmp/opencode-data")),
        "HOME" => Some(OsString::from("/tmp/home")),
        _ => None,
    });

    assert!(paths.contains(&PathBuf::from("/tmp/opencode-data/opencode/opencode.db")));
    assert!(paths.contains(&PathBuf::from(
        "/tmp/home/.local/share/opencode/opencode.db"
    )));
    assert_eq!(
        paths
            .iter()
            .filter(|path| path.ends_with("opencode.db"))
            .count(),
        2
    );
}
