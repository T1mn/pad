#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DiffDocument {
    pub prelude: Vec<String>,
    pub files: Vec<DiffFile>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DiffFile {
    pub title: String,
    pub meta: Vec<String>,
    pub hunks: Vec<DiffHunk>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DiffHunk {
    pub header: String,
    pub rows: Vec<DiffRow>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DiffRow {
    pub old_no: Option<usize>,
    pub new_no: Option<usize>,
    pub old_text: String,
    pub new_text: String,
    pub kind: DiffRowKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DiffRowKind {
    Context,
    Delete,
    Add,
    Change,
}
