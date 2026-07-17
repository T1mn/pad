use std::path::Path;

pub(super) fn opencode_thread_for_working_dir(
    working_dir: &str,
    require_unique: bool,
) -> Option<crate::opencode_history::OpenCodeThreadRef> {
    let threads = crate::opencode_history::threads_for_cwd(Path::new(working_dir)).ok()?;
    super::path::select_cwd_candidate(threads, require_unique)
}
