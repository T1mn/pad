mod build;
mod model {
    use std::path::PathBuf;

    #[derive(Clone)]
    pub struct TreeRow {
        pub depth: usize,
        pub path: PathBuf,
        pub label: String,
        pub is_dir: bool,
        pub expanded: bool,
    }
}
mod scan;

pub use build::build_tree;
pub use model::TreeRow;
pub use scan::scan_files;
