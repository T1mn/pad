use std::path::PathBuf;
use std::process::Command;

pub(crate) fn default_db_paths() -> Vec<PathBuf> {
    let mut paths = configured_db_paths();
    if opencode_db_is_configured() || paths.iter().any(|path| path.exists()) {
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
    if let Some(path) = get_env("OPENCODE_DB").filter(|path| !path.is_empty()) {
        let path = PathBuf::from(path);
        if path == std::path::Path::new(":memory:") {
            return paths;
        }
        if path.is_absolute() {
            paths.push(path);
            return paths;
        }
        let data_home = get_env("XDG_DATA_HOME").map(PathBuf::from).or_else(|| {
            get_env("HOME").map(|home| PathBuf::from(home).join(".local").join("share"))
        });
        paths.push(
            data_home
                .map(|root| root.join("opencode").join(&path))
                .unwrap_or(path),
        );
        return paths;
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

fn opencode_db_is_configured() -> bool {
    std::env::var_os("OPENCODE_DB")
        .filter(|path| !path.is_empty())
        .is_some()
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

#[cfg(test)]
#[path = "db_paths_tests.rs"]
mod tests;
