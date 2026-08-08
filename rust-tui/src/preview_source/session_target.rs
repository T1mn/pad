mod resolve;
mod sources;
mod target {
    use std::path::PathBuf;

    use crate::model::PreviewSessionOrigin;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(crate) struct SessionTarget {
        pub(crate) origin: PreviewSessionOrigin,
        pub(crate) session_id: Option<String>,
        pub(crate) transcript_path: PathBuf,
        pub(crate) updated_at: Option<i64>,
    }
}

#[cfg(test)]
mod tests;

pub(crate) use resolve::{persistence_panel_from_request, resolve_session_target};
#[cfg(test)]
pub(crate) use sources::resolved_session_id_for_request;
pub(crate) use sources::transcript_updated_at;
pub(crate) use target::SessionTarget;
