use std::path::PathBuf;

#[derive(Clone)]
pub struct TreeRow {
    pub depth: usize,
    pub path: PathBuf,
    pub label: String,
    pub is_dir: bool,
    pub expanded: bool,
}
