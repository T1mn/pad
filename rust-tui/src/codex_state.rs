mod archive;
mod cache {
    use super::model::{
        CacheKey, CachedThreads, ThreadArchiveFilter, THREAD_CACHE, THREAD_CACHE_TTL,
    };
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    use std::time::Instant;

    pub(crate) fn load_cached_threads(
        db_path: &Path,
        filter: ThreadArchiveFilter,
    ) -> Option<Vec<super::CodexThreadRef>> {
        let cache_key = CacheKey {
            db_path: db_path.to_path_buf(),
            filter,
        };
        let cache = THREAD_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        let Ok(guard) = cache.lock() else {
            return None;
        };
        guard.get(&cache_key).and_then(|cached| {
            if cached.loaded_at.elapsed() < THREAD_CACHE_TTL {
                Some(cached.threads.clone())
            } else {
                None
            }
        })
    }

    pub(crate) fn store_cached_threads(
        db_path: PathBuf,
        filter: ThreadArchiveFilter,
        threads: &[super::CodexThreadRef],
    ) {
        let cache_key = CacheKey { db_path, filter };
        let cache = THREAD_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        if let Ok(mut guard) = cache.lock() {
            guard.insert(
                cache_key,
                CachedThreads {
                    loaded_at: Instant::now(),
                    threads: threads.to_vec(),
                },
            );
        }
    }

    pub(crate) fn invalidate_thread_cache(db_path: &Path) {
        let Some(cache) = THREAD_CACHE.get() else {
            return;
        };
        if let Ok(mut guard) = cache.lock() {
            guard.retain(|key, _| key.db_path != db_path);
        }
    }
}
mod migration;
mod model {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};
    use std::time::{Duration, Instant};

    pub(crate) const THREAD_CACHE_TTL: Duration = Duration::from_secs(10);
    pub(crate) const ACTIVE_THREAD_MAX_AGE_SECS: i64 = 7 * 24 * 60 * 60;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct CodexThreadRef {
        pub thread_id: String,
        pub cwd: PathBuf,
        pub updated_at: i64,
        pub rollout_path: PathBuf,
        pub title: Option<String>,
        pub first_user_message: Option<String>,
        pub source: Option<String>,
        pub archived: bool,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub enum ThreadArchiveFilter {
        ActiveOnly,
        ArchivedOnly,
    }

    #[derive(Clone)]
    pub(crate) struct CachedThreads {
        pub loaded_at: Instant,
        pub threads: Vec<CodexThreadRef>,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub(crate) struct CacheKey {
        pub db_path: PathBuf,
        pub filter: ThreadArchiveFilter,
    }

    pub(crate) static THREAD_CACHE: OnceLock<Mutex<HashMap<CacheKey, CachedThreads>>> =
        OnceLock::new();

    pub(crate) struct ThreadRow {
        pub rollout_path: String,
        pub archived: bool,
    }
}
mod pathing {
    use super::model::CodexThreadRef;
    use std::path::{Path, PathBuf};

    pub(crate) fn normalize_path(path: &Path) -> PathBuf {
        std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
    }

    pub(crate) fn select_latest_thread_for_cwd<'a>(
        cwd: &Path,
        threads: &'a [CodexThreadRef],
    ) -> Option<&'a CodexThreadRef> {
        let normalized = normalize_path(cwd);

        threads
            .iter()
            .filter(|thread| normalize_path(&thread.cwd) == normalized)
            .max_by_key(|thread| thread.updated_at)
            .or_else(|| {
                threads
                    .iter()
                    .filter_map(|thread| {
                        let thread_cwd = normalize_path(&thread.cwd);
                        relation_score(&normalized, &thread_cwd).map(|score| (score, thread))
                    })
                    .max_by_key(|(score, thread)| (*score, thread.updated_at))
                    .map(|(_, thread)| thread)
            })
    }

    pub(crate) fn relation_score(lhs: &Path, rhs: &Path) -> Option<usize> {
        if is_component_prefix(lhs, rhs) || is_component_prefix(rhs, lhs) {
            Some(common_component_count(lhs, rhs))
        } else {
            None
        }
    }

    pub(crate) fn common_component_count(lhs: &Path, rhs: &Path) -> usize {
        lhs.components()
            .zip(rhs.components())
            .take_while(|(left, right)| left == right)
            .count()
    }

    pub(crate) fn is_component_prefix(prefix: &Path, candidate: &Path) -> bool {
        let mut candidate_components = candidate.components();
        prefix
            .components()
            .all(|prefix_component| candidate_components.next() == Some(prefix_component))
    }
}
mod query;
mod util {
    use std::io;

    pub(crate) use crate::time::unix_now_ts;

    pub(crate) fn to_io_error(err: rusqlite::Error) -> io::Error {
        io::Error::other(err)
    }
}

pub use archive::{archive_thread, unarchive_thread};
pub use migration::normalize_pad_codex_home_rollout_paths;
pub use model::CodexThreadRef;
#[cfg(test)]
pub use model::ThreadArchiveFilter;
pub use query::{
    all_archived_threads, all_threads, archived_threads_for_cwd, latest_thread_for_cwd,
    subagent_parent_thread_id, thread_for_id, threads_for_cwd,
};

#[cfg(test)]
mod tests;
