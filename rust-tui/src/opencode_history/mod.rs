mod archive;
mod model {
    use std::path::PathBuf;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct OpenCodeThreadRef {
        pub session_id: String,
        pub cwd: PathBuf,
        pub updated_at: i64,
        pub db_path: PathBuf,
        pub title: Option<String>,
        pub first_user_message: Option<String>,
        pub last_user_message: Option<String>,
        pub last_assistant_message: Option<String>,
        pub provider_name: Option<String>,
        pub model_name: Option<String>,
        pub share_url: Option<String>,
        pub cost: Option<String>,
        pub token_summary: Option<String>,
        pub archived: bool,
    }
}
mod query;
mod stats;
mod util;

use std::io;
use std::path::Path;

pub use model::OpenCodeThreadRef;

pub fn all_threads() -> io::Result<Vec<OpenCodeThreadRef>> {
    query::all_threads(Some(false))
}

pub fn all_archived_threads() -> io::Result<Vec<OpenCodeThreadRef>> {
    query::all_threads(Some(true))
}

pub fn thread_for_id(session_id: &str) -> io::Result<Option<OpenCodeThreadRef>> {
    query::thread_for_id(session_id)
}

pub fn threads_for_cwd(cwd: &Path) -> io::Result<Vec<OpenCodeThreadRef>> {
    query::threads_for_cwd(cwd)
}

pub fn archive_thread(session_id: &str) -> io::Result<()> {
    archive::set_archived(session_id, true)
}

pub fn unarchive_thread(session_id: &str) -> io::Result<()> {
    archive::set_archived(session_id, false)
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
