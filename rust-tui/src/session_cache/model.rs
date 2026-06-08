mod context;
mod records;
mod snapshot;
mod support;

pub(super) const CACHE_VERSION: u32 = 1;
pub(super) const RETENTION_SECS: i64 = 30 * 24 * 60 * 60;
pub const SESSION_HISTORY_TURN_LIMIT: usize = 50;

pub(super) use context::HookBindingContext;
pub(super) use records::{CachedPaneBinding, CachedSessionRecord, SessionCacheIndex};
pub(super) use snapshot::snapshot_from_record;
pub use snapshot::SessionCacheSnapshot;
pub(super) use support::supports_cached_session;
