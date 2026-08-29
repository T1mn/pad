use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

pub fn pad_home_dir() -> PathBuf {
    resolve_pad_home_dir(
        std::env::var_os("PAD_HOME").map(PathBuf::from),
        std::env::var_os("HOME").map(PathBuf::from),
        dirs::home_dir(),
    )
}

/// PAD Desktop's private application-data root. It intentionally does not
/// reuse the legacy `PAD_HOME`/`~/.pad` tree so the Desktop store can be
/// migrated, backed up and removed independently from the native TUI.
pub fn pad_desktop_data_dir() -> PathBuf {
    if let Some(override_dir) =
        std::env::var_os("PAD_DESKTOP_DATA_DIR").filter(|value| !value.is_empty())
    {
        return PathBuf::from(override_dir);
    }

    dirs::data_dir()
        .or_else(|| dirs::home_dir().map(|home| home.join("Library").join("Application Support")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("PAD Desktop")
}

/// Resolve a custom Codex home only as a protected boundary. Electron maps
/// the user's `CODEX_HOME` into the PAD-specific name so it never becomes Pi
/// configuration, while native/test hosts retain the compatibility fallback.
pub(crate) fn protected_codex_home() -> Option<PathBuf> {
    std::env::var_os("PAD_PROTECTED_CODEX_HOME")
        .filter(|value| !value.is_empty())
        .or_else(|| std::env::var_os("CODEX_HOME").filter(|value| !value.is_empty()))
        .map(PathBuf::from)
}

/// Validate and canonicalize the Desktop data root before any directory,
/// permission or lock-file mutation occurs.
pub(crate) fn validate_pad_desktop_data_root(path: &Path) -> io::Result<PathBuf> {
    validate_pad_desktop_data_root_with_inputs(
        path,
        dirs::home_dir().as_deref(),
        std::env::var_os("PAD_HOME").as_deref().map(Path::new),
        protected_codex_home().as_deref(),
    )
}

fn validate_pad_desktop_data_root_with_inputs(
    path: &Path,
    home: Option<&Path>,
    legacy_pad_home: Option<&Path>,
    custom_codex_home: Option<&Path>,
) -> io::Result<PathBuf> {
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(unsafe_data_root(path));
    }

    let resolved = canonicalize_existing_prefix_without_writes(path)?;
    for candidate in [path, resolved.as_path()] {
        if is_broad_system_root(candidate) || home.is_some_and(|home| candidate == home) {
            return Err(unsafe_data_root(path));
        }

        let mut protected = home
            .map(crate::permission_policy::default_protected_namespaces)
            .unwrap_or_default()
            .into_iter()
            .filter(|namespace| namespace.name != "pad-desktop-application-support")
            .map(|namespace| namespace.root)
            .collect::<Vec<_>>();
        if let Some(root) = legacy_pad_home {
            protected.push(root.to_path_buf());
        }
        if let Some(root) = custom_codex_home {
            protected.push(root.to_path_buf());
        }
        if protected.iter().any(|root| {
            !root.as_os_str().is_empty()
                && (candidate == root || candidate.starts_with(root) || root.starts_with(candidate))
        }) {
            return Err(unsafe_data_root(path));
        }
    }
    Ok(resolved)
}

fn canonicalize_existing_prefix_without_writes(path: &Path) -> io::Result<PathBuf> {
    let mut existing = path.to_path_buf();
    while !existing.exists() {
        if !existing.pop() {
            return Err(unsafe_data_root(path));
        }
    }
    let canonical = fs::canonicalize(&existing)?;
    let remainder = path
        .strip_prefix(&existing)
        .map_err(|_| unsafe_data_root(path))?;
    Ok(canonical.join(remainder))
}

fn is_broad_system_root(path: &Path) -> bool {
    [
        "/",
        "/Applications",
        "/System",
        "/Library",
        "/usr",
        "/bin",
        "/sbin",
        "/etc",
        "/var",
        "/private",
        "/private/var",
        "/tmp",
        "/private/tmp",
    ]
    .into_iter()
    .any(|root| path == Path::new(root))
}

fn unsafe_data_root(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("refusing unsafe PAD Desktop data root: {}", path.display()),
    )
}

#[cfg(test)]
pub fn pad_desktop_store_path() -> PathBuf {
    pad_desktop_data_dir()
        .join("v1")
        .join("store")
        .join("pad.sqlite")
}

/// Create the PAD Desktop-owned directory hierarchy with owner-only access.
///
/// `create_dir_all` honours the process umask only for newly-created paths and
/// does not repair an older preview build's permissions.  Applying the mode
/// explicitly keeps the Desktop database, Profile configuration and Pi
/// sessions private after an in-place upgrade as well.
pub(crate) fn ensure_pad_desktop_private_layout(root: &Path) -> io::Result<()> {
    for directory in [
        root.to_path_buf(),
        root.join("v1"),
        root.join("v1").join("store"),
        root.join("v1").join("profiles"),
    ] {
        ensure_private_dir(&directory)?;
    }
    Ok(())
}

/// Ensure one PAD-owned directory exists and is accessible only by its owner.
pub(crate) fn ensure_private_dir(path: &Path) -> io::Result<()> {
    reject_symlink(path)?;
    fs::create_dir_all(path)?;
    reject_symlink(path)?;
    set_private_mode(path, 0o700)
}

/// Repair permissions below a PAD-owned root without following symlinks.
/// Directories become `0700` and regular files become `0600`; symlinks are
/// deliberately left untouched so a malicious link cannot change a target
/// outside the Profile boundary.
pub(crate) fn harden_private_tree(path: &Path) -> io::Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        return Ok(());
    }
    if file_type.is_dir() {
        set_private_mode(path, 0o700)?;
        for entry in fs::read_dir(path)? {
            harden_private_tree(&entry?.path())?;
        }
    } else if file_type.is_file() {
        set_private_mode(path, 0o600)?;
    }
    Ok(())
}

fn set_private_mode(path: &Path, mode: u32) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::symlink_metadata(path)?.permissions();
        permissions.set_mode(mode);
        fs::set_permissions(path, permissions)?;
    }
    #[cfg(not(unix))]
    let _ = (path, mode);
    Ok(())
}

fn reject_symlink(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("private directory cannot be a symlink: {}", path.display()),
        )),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn resolve_pad_home_dir(
    override_dir: Option<PathBuf>,
    environment_home: Option<PathBuf>,
    platform_home: Option<PathBuf>,
) -> PathBuf {
    override_dir
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| {
            environment_home
                .or(platform_home)
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".pad")
        })
}

pub fn config_path() -> PathBuf {
    pad_home_dir().join("config.toml")
}

pub fn relay_export_path() -> PathBuf {
    pad_home_dir().join("relay.yaml")
}

pub fn opencode_exports_dir() -> PathBuf {
    pad_home_dir().join("opencode-exports")
}

pub fn opencode_stats_dir() -> PathBuf {
    pad_home_dir().join("opencode-stats")
}

pub fn opencode_diagnostics_dir() -> PathBuf {
    pad_home_dir().join("opencode-diagnostics")
}

pub fn terminal_workspace_path() -> PathBuf {
    pad_home_dir().join("terminal-workspace.json")
}

pub fn pad_db_path() -> PathBuf {
    pad_home_dir().join("pad.db")
}

pub fn legacy_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".config")
        })
        .join("pad")
        .join("config.toml")
}

pub fn logs_dir() -> PathBuf {
    pad_home_dir().join("logs")
}

pub fn log_path() -> PathBuf {
    logs_dir().join("pad.log")
}

pub fn telegram_bot_log_path() -> PathBuf {
    logs_dir().join("telegram-bot.log")
}

pub fn hook_events_path() -> PathBuf {
    logs_dir().join("hook-events.jsonl")
}

pub fn notifications_dir() -> PathBuf {
    pad_home_dir().join("notifications")
}

pub fn notification_inbox_path() -> PathBuf {
    notifications_dir().join("inbox.json")
}

pub fn session_continuity_log_path() -> PathBuf {
    logs_dir().join("session-continuity.jsonl")
}

pub fn scripts_dir() -> PathBuf {
    pad_home_dir().join("scripts")
}

pub fn prompts_dir() -> PathBuf {
    pad_home_dir().join("prompt")
}

pub fn sessions_dir() -> PathBuf {
    pad_home_dir().join("sessions")
}

pub fn sessions_index_path() -> PathBuf {
    sessions_dir().join("index.json")
}

pub fn session_continuity_state_path() -> PathBuf {
    sessions_dir().join("continuity.json")
}

pub fn claude_hook_bridge_path() -> PathBuf {
    scripts_dir().join("claude_hook_bridge.py")
}

pub fn codex_hook_bridge_path() -> PathBuf {
    scripts_dir().join("codex_hook_bridge.py")
}

pub fn pad_codex_wrapper_path() -> PathBuf {
    scripts_dir().join("pad-codex")
}

#[cfg(test)]
#[path = "base_tests.rs"]
pub(crate) mod tests;
