use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub(crate) fn existing_rollout_path(path: &Path) -> Option<PathBuf> {
    if path.is_file() {
        return Some(path.to_path_buf());
    }
    if path.extension() != Some(OsStr::new("jsonl")) {
        return None;
    }

    let compressed = compressed_sibling(path);
    compressed.is_file().then_some(compressed)
}

pub(crate) fn is_compressed_rollout(path: &Path) -> bool {
    path.extension() == Some(OsStr::new("zst"))
        && path.file_stem().map(Path::new).and_then(Path::extension) == Some(OsStr::new("jsonl"))
}

fn compressed_sibling(path: &Path) -> PathBuf {
    let mut compressed = path.as_os_str().to_os_string();
    compressed.push(".zst");
    compressed.into()
}
