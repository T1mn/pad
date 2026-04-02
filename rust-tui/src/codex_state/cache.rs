use super::model::{CacheKey, CachedThreads, ThreadArchiveFilter, THREAD_CACHE, THREAD_CACHE_TTL};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

pub(crate) fn load_cached_threads(db_path: &Path, filter: ThreadArchiveFilter) -> Option<Vec<super::CodexThreadRef>> {
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
