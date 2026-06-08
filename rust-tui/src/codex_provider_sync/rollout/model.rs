use std::path::PathBuf;

#[derive(Clone, Debug)]
pub(in crate::codex_provider_sync) struct RolloutChange {
    pub(in crate::codex_provider_sync) path: PathBuf,
    pub(in crate::codex_provider_sync::rollout) original_first_line: String,
    pub(in crate::codex_provider_sync::rollout) original_separator: String,
    pub(in crate::codex_provider_sync::rollout) updated_first_line: String,
}
