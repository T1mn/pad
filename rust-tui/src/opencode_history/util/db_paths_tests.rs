use super::configured_db_paths_from;
use std::ffi::OsString;
use std::path::PathBuf;

#[test]
fn configured_db_paths_prefers_env_and_dedupes_defaults() {
    let paths = configured_db_paths_from(|key| match key {
        "OPENCODE_DB" => Some(OsString::from("/tmp/opencode/custom.db")),
        "XDG_DATA_HOME" => Some(OsString::from("/tmp/opencode-data")),
        "HOME" => Some(OsString::from("/tmp/home")),
        _ => None,
    });

    assert_eq!(paths[0], PathBuf::from("/tmp/opencode/custom.db"));
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
