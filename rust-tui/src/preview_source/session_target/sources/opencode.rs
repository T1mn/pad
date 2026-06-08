use std::path::Path;

pub(super) fn opencode_thread_for_working_dir(
    working_dir: &str,
) -> Option<crate::opencode_history::OpenCodeThreadRef> {
    crate::opencode_history::threads_for_cwd(Path::new(working_dir))
        .ok()
        .and_then(|threads| threads.into_iter().next())
}
