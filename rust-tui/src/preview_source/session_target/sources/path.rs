use std::fs;
use std::path::{Path, PathBuf};

pub(super) fn same_path(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }

    match (std::fs::canonicalize(left), std::fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

pub(crate) fn transcript_updated_at(path: &Path) -> Option<i64> {
    std::fs::metadata(path)
        .ok()?
        .modified()
        .ok()
        .and_then(crate::time::system_time_unix_secs)
}

pub(in crate::preview_source::session_target) fn find_matching_jsonl<F>(
    root: &Path,
    matcher: F,
) -> Option<PathBuf>
where
    F: Fn(&str) -> bool,
{
    if !root.exists() {
        return None;
    }

    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }

            let file_name = path.file_name()?.to_string_lossy();
            if matcher(&file_name) {
                return Some(path);
            }
        }
    }

    None
}
