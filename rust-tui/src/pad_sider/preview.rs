#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PreviewKind {
    Markdown,
    Text,
    Diff,
    Directory,
    Missing,
}

#[derive(Clone, PartialEq, Eq)]
pub struct MarkdownPreview {
    pub path: std::path::PathBuf,
    pub content: String,
    pub scroll: u16,
}

#[derive(Clone, PartialEq, Eq)]
pub struct FilePreview {
    pub title: String,
    pub content: String,
    pub kind: PreviewKind,
    pub scroll: u16,
}

impl FilePreview {
    pub fn empty() -> Self {
        Self::new(
            "preview".into(),
            "No file selected".into(),
            PreviewKind::Missing,
        )
    }

    pub fn new(title: String, content: String, kind: PreviewKind) -> Self {
        Self {
            title,
            content,
            kind,
            scroll: 0,
        }
    }
}
