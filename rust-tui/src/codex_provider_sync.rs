use std::io;

mod backup;
#[cfg(test)]
mod helpers;
mod model;
mod rollout;
mod state_db;
mod sync;
#[cfg(test)]
mod tests;
mod worker;

pub use model::ProviderSyncResult;
pub use worker::enqueue_sync_to_provider;

pub fn sync_to_provider(target_provider: &str) -> io::Result<ProviderSyncResult> {
    let codex_home = crate::paths::pad_codex_home_dir();
    sync_to_provider_at(&codex_home, target_provider)
}

pub(crate) use sync::sync_to_provider_at;
