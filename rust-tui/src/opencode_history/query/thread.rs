use super::super::model::OpenCodeThreadRef;
use super::super::stats::{format_cost, format_token_summary, SessionStats};
use super::messages::MessageSummary;
use super::model_parse::parse_model;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub(super) struct SessionRow {
    pub(super) id: String,
    pub(super) directory: String,
    pub(super) path: Option<String>,
    pub(super) title: Option<String>,
    pub(super) updated_at: i64,
    pub(super) archived_at: Option<i64>,
    pub(super) model: Option<String>,
    pub(super) stats: SessionStats,
}

pub(super) fn build_thread(
    db_path: &Path,
    row: &SessionRow,
    summary: Option<&MessageSummary>,
) -> Option<OpenCodeThreadRef> {
    let cwd = if row.directory.trim().is_empty() {
        row.path.as_deref().unwrap_or("")
    } else {
        row.directory.as_str()
    };
    if cwd.trim().is_empty() {
        return None;
    }
    let (provider_name, model_name) = parse_model(&row.model);
    Some(OpenCodeThreadRef {
        session_id: row.id.clone(),
        cwd: PathBuf::from(cwd),
        updated_at: row.updated_at,
        db_path: db_path.to_path_buf(),
        title: row.title.clone().filter(|title| !title.trim().is_empty()),
        first_user_message: summary.and_then(|summary| summary.first_user.clone()),
        last_user_message: summary.and_then(|summary| summary.last_user.clone()),
        last_assistant_message: summary.and_then(|summary| summary.last_assistant.clone()),
        provider_name,
        model_name,
        share_url: row
            .stats
            .share_url
            .clone()
            .filter(|url| !url.trim().is_empty()),
        cost: format_cost(row.stats.cost),
        token_summary: format_token_summary(&row.stats),
        archived: row.archived_at.is_some(),
    })
}
