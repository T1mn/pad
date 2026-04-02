use super::model::CodexThreadRef;
use std::path::{Path, PathBuf};

pub(crate) fn normalize_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[allow(dead_code)]
pub(crate) fn select_latest_thread_for_cwd<'a>(
    cwd: &Path,
    threads: &'a [CodexThreadRef],
) -> Option<&'a CodexThreadRef> {
    let normalized = normalize_path(cwd);

    threads
        .iter()
        .filter(|thread| normalize_path(&thread.cwd) == normalized)
        .max_by_key(|thread| thread.updated_at)
        .or_else(|| {
            threads
                .iter()
                .filter_map(|thread| {
                    let thread_cwd = normalize_path(&thread.cwd);
                    relation_score(&normalized, &thread_cwd).map(|score| (score, thread))
                })
                .max_by_key(|(score, thread)| (*score, thread.updated_at))
                .map(|(_, thread)| thread)
        })
}

#[allow(dead_code)]
pub(crate) fn relation_score(lhs: &Path, rhs: &Path) -> Option<usize> {
    if is_component_prefix(lhs, rhs) || is_component_prefix(rhs, lhs) {
        Some(common_component_count(lhs, rhs))
    } else {
        None
    }
}

#[allow(dead_code)]
pub(crate) fn common_component_count(lhs: &Path, rhs: &Path) -> usize {
    lhs.components()
        .zip(rhs.components())
        .take_while(|(left, right)| left == right)
        .count()
}

#[allow(dead_code)]
pub(crate) fn is_component_prefix(prefix: &Path, candidate: &Path) -> bool {
    let prefix_components = prefix.components().collect::<Vec<_>>();
    let candidate_components = candidate.components().collect::<Vec<_>>();
    prefix_components.len() <= candidate_components.len()
        && prefix_components
            .iter()
            .zip(candidate_components.iter())
            .all(|(left, right)| left == right)
}
