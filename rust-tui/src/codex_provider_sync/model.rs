#[derive(Debug, Default, PartialEq, Eq)]
pub struct ProviderSyncResult {
    pub updated_rollout_files: usize,
    pub updated_sqlite_rows: usize,
}
