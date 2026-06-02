#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PreviewKind {
    Markdown,
    Text,
    Diff,
    Directory,
    Missing,
}

#[derive(Clone, PartialEq, Eq)]
pub struct FullscreenPreview {
    pub path: std::path::PathBuf,
    pub preview: FilePreview,
}

#[derive(Clone, PartialEq, Eq)]
pub struct FilePreview {
    pub title: String,
    pub content: String,
    pub kind: PreviewKind,
    pub scroll: u16,
}

#[derive(Clone)]
pub struct RenderedFilePreview {
    pub revision: u64,
    pub width: u16,
    pub show_line_numbers: bool,
    pub text_zoom: i8,
    pub lines: Vec<ratatui::text::Line<'static>>,
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
