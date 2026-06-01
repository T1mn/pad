use rusqlite::OpenFlags;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(crate) fn default_db_paths() -> Vec<PathBuf> {
    let mut paths = configured_db_paths();
    if paths.iter().any(|path| path.exists()) {
        return paths;
    }
    if let Some(path) = opencode_cli_db_path() {
        push_unique(&mut paths, path);
    }
    paths
}

fn configured_db_paths() -> Vec<PathBuf> {
    configured_db_paths_from(|key| std::env::var_os(key))
}

fn configured_db_paths_from<F>(get_env: F) -> Vec<PathBuf>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    let mut paths = Vec::new();
    if let Some(path) = get_env("OPENCODE_DB") {
        paths.push(PathBuf::from(path));
    }
    if let Some(data_home) = get_env("XDG_DATA_HOME") {
        push_unique(
            &mut paths,
            PathBuf::from(data_home)
                .join("opencode")
                .join("opencode.db"),
        );
    }
    if let Some(home) = get_env("HOME") {
        let home = PathBuf::from(home);
        push_unique(
            &mut paths,
            home.join(".local")
                .join("share")
                .join("opencode")
                .join("opencode.db"),
        );
        push_unique(
            &mut paths,
            home.join(".local")
                .join("share")
                .join("opencode")
                .join("opencode-local.db"),
        );
    }
    paths
}

fn opencode_cli_db_path() -> Option<PathBuf> {
    let output = Command::new(default_opencode_command())
        .args(["db", "path"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        None
    } else {
        Some(PathBuf::from(path))
    }
}

fn default_opencode_command() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        let home_bin = PathBuf::from(&home).join(".opencode/bin/opencode");
        if home_bin.exists() {
            return home_bin;
        }
    }
    PathBuf::from("opencode")
}

fn push_unique(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.iter().any(|existing| existing == &path) {
        paths.push(path);
    }
}

pub(crate) fn open_readonly(path: &Path) -> io::Result<rusqlite::Connection> {
    rusqlite::Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(to_io_error)
}

pub(crate) fn open_write(path: &Path) -> io::Result<rusqlite::Connection> {
    rusqlite::Connection::open(path).map_err(to_io_error)
}

pub(crate) fn to_io_error(err: rusqlite::Error) -> io::Error {
    io::Error::other(err.to_string())
}

#[cfg(test)]
mod tests {
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
}
