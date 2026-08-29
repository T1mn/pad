//! PAD-owned metadata store.
//!
//! `PadStore` is the control-plane database for the Desktop shell.  It stores
//! only PAD metadata (profiles, projects, tasks, and sidebar sections); Pi's
//! JSONL session journal remains the source of truth for conversation content.
//! In particular, this module never opens, migrates, or updates a Codex or
//! ChatGPT database.  The database path is supplied by the caller and is
//! checked against known provider-owned namespace names before opening.

mod repository;
mod schema;

#[cfg(test)]
mod tests;

pub(crate) use crate::permission_policy::{
    Profile, Project, Section, SectionItem, Task, TaskEnvironment, TaskStatus,
};

use rusqlite::{Connection, OpenFlags};
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

pub(crate) use repository::PadStore;
pub(crate) use schema::CURRENT_SCHEMA_VERSION;

pub(crate) type StoreResult<T> = Result<T, StoreError>;

/// Open the Desktop store at its platform-private default location.  The
/// native PAD TUI does not call this helper; the future macOS host does so
/// only for its own Desktop process.
pub(crate) fn open_default() -> StoreResult<PadStore> {
    PadStore::open(crate::paths::pad_desktop_store_path())
}

/// Errors returned by the PAD private store.
#[derive(Debug)]
pub(crate) enum StoreError {
    Sqlite(rusqlite::Error),
    Io(std::io::Error),
    Serialization(serde_json::Error),
    InvalidDatabasePath(PathBuf),
    UnsupportedSchemaVersion { found: i64, supported: i64 },
    NotFound { kind: &'static str, id: String },
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(error) => write!(formatter, "PAD store SQLite error: {error}"),
            Self::Io(error) => write!(formatter, "PAD store I/O error: {error}"),
            Self::Serialization(error) => {
                write!(formatter, "PAD store serialization error: {error}")
            }
            Self::InvalidDatabasePath(path) => write!(
                formatter,
                "refusing PAD store path inside a provider-owned namespace: {}",
                path.display()
            ),
            Self::UnsupportedSchemaVersion { found, supported } => write!(
                formatter,
                "PAD store schema version {found} is newer than supported version {supported}"
            ),
            Self::NotFound { kind, id } => write!(formatter, "{kind} '{id}' was not found"),
        }
    }
}

impl Error for StoreError {}

impl From<rusqlite::Error> for StoreError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

impl From<std::io::Error> for StoreError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialization(error)
    }
}

pub(crate) fn json_encode<T: serde::Serialize>(value: &T) -> StoreResult<String> {
    serde_json::to_string(value).map_err(StoreError::from)
}

pub(crate) fn json_decode<T: serde::de::DeserializeOwned>(value: &str) -> StoreResult<T> {
    serde_json::from_str(value).map_err(StoreError::from)
}

pub(crate) fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

pub(crate) fn string_to_path(value: String) -> PathBuf {
    PathBuf::from(value)
}

/// Open a PAD-owned database at `path` and apply migrations.
///
/// The caller controls the location, which makes tests and portable builds
/// straightforward.  `validate_database_path` resolves the existing parent
/// first, so a symlink cannot bypass the provider namespace guard.
pub(crate) fn open_database(path: impl AsRef<Path>) -> StoreResult<(Connection, PathBuf)> {
    let path = validate_database_path(path.as_ref())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut connection = Connection::open_with_flags(
        &path,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    configure_connection(&mut connection)?;
    schema::apply_migrations(&mut connection)?;
    make_database_file_private(&path)?;
    Ok((connection, path))
}

pub(crate) fn open_memory_database() -> StoreResult<Connection> {
    let mut connection = Connection::open_in_memory()?;
    configure_connection(&mut connection)?;
    schema::apply_migrations(&mut connection)?;
    Ok(connection)
}

fn configure_connection(connection: &mut Connection) -> StoreResult<()> {
    connection.busy_timeout(Duration::from_secs(5))?;
    // WAL and NORMAL synchronous mode are safe for a single PAD process and
    // make sidebar writes independent from readers.  In-memory SQLite simply
    // falls back to its normal journal behavior.
    connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
    Ok(())
}

/// Reject paths that could mutate provider-owned state.
///
/// We intentionally match namespace components rather than searching for an
/// arbitrary substring.  A user project named `codex-examples` is not a
/// provider database, while `.codex` and macOS OpenAI container names are.
pub(crate) fn validate_database_path(path: &Path) -> StoreResult<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };

    let resolved = canonicalize_existing_prefix(&absolute)?;
    if contains_provider_namespace(&absolute) || contains_provider_namespace(&resolved) {
        return Err(StoreError::InvalidDatabasePath(absolute));
    }
    Ok(resolved)
}

fn canonicalize_existing_prefix(path: &Path) -> StoreResult<PathBuf> {
    let mut existing = path.to_path_buf();
    while !existing.exists() {
        if !existing.pop() {
            return Err(StoreError::InvalidDatabasePath(path.to_path_buf()));
        }
    }

    let canonical_existing = fs::canonicalize(&existing)?;
    let remainder = path
        .strip_prefix(&existing)
        .map_err(|_| StoreError::InvalidDatabasePath(path.to_path_buf()))?;
    if remainder.as_os_str().is_empty() {
        Ok(canonical_existing)
    } else {
        Ok(canonical_existing.join(remainder))
    }
}

fn contains_provider_namespace(path: &Path) -> bool {
    path.components().any(|component| {
        let Component::Normal(value) = component else {
            return false;
        };
        let segment = value.to_string_lossy().to_ascii_lowercase();
        matches!(
            segment.as_str(),
            ".codex"
                | "codex"
                | ".chatgpt"
                | "chatgpt"
                | "com.openai.codex"
                | "com.openai.chatgpt"
                | "com.openai.chat"
                | "group.com.openai.codex"
                | "group.com.openai.chatgpt"
                | "group.com.openai.chat"
        )
    })
}

fn make_database_file_private(path: &Path) -> StoreResult<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(0o600);
        fs::set_permissions(path, permissions)?;
    }
    Ok(())
}
