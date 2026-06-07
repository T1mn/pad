use std::path::PathBuf;

use crate::model::PreviewSessionOrigin;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SessionTarget {
    pub(crate) origin: PreviewSessionOrigin,
    pub(crate) session_id: Option<String>,
    pub(crate) transcript_path: PathBuf,
    pub(crate) updated_at: Option<i64>,
}
