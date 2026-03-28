use super::model::{SessionCacheIndex, CACHE_VERSION, RETENTION_SECS};
use super::util::now_ts;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub(super) fn load_index() -> SessionCacheIndex {
    let path = crate::paths::sessions_index_path();
    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(_) => {
            return SessionCacheIndex {
                version: CACHE_VERSION,
                ..SessionCacheIndex::default()
            };
        }
    };

    serde_json::from_str(&content).unwrap_or_else(|err| {
        log_debug!("session_cache: failed to parse {}: {}", path.display(), err);
        SessionCacheIndex {
            version: CACHE_VERSION,
            ..SessionCacheIndex::default()
        }
    })
}

pub(super) fn save_index(index: &SessionCacheIndex) -> io::Result<()> {
    let path = crate::paths::sessions_index_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let tmp_path = temporary_index_path(&path);
    let content = serde_json::to_string_pretty(index)?;
    fs::write(&tmp_path, content)?;
    fs::rename(&tmp_path, &path)?;
    Ok(())
}

pub(super) fn prune_index(index: &mut SessionCacheIndex) -> bool {
    if index.version != CACHE_VERSION {
        index.version = CACHE_VERSION;
    }

    let now = now_ts();
    let min_ts = now.saturating_sub(RETENTION_SECS);

    let before_sessions = index.sessions.len();
    index.sessions.retain(|record| {
        if record.updated_at < min_ts {
            return false;
        }
        if let Some(path) = record.transcript_path.as_deref() {
            return Path::new(path).exists();
        }
        true
    });

    let valid_session_ids = index
        .sessions
        .iter()
        .map(|record| record.agent_session_id.clone())
        .collect::<HashSet<_>>();

    let before_bindings = index.pane_bindings.len();
    index.pane_bindings.retain(|binding| {
        binding.updated_at >= min_ts && valid_session_ids.contains(&binding.agent_session_id)
    });

    before_sessions != index.sessions.len() || before_bindings != index.pane_bindings.len()
}

fn temporary_index_path(path: &Path) -> PathBuf {
    let pid = std::process::id();
    let stamp = now_ts();
    path.with_extension(format!("tmp.{}.{}", pid, stamp))
}
