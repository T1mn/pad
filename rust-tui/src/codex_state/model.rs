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

pub(crate) static THREAD_CACHE: OnceLock<Mutex<HashMap<CacheKey, CachedThreads>>> = OnceLock::new();

pub(crate) struct ThreadRow {
    pub rollout_path: String,
    pub archived: bool,
}
