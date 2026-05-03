use super::fs::{is_markdown_file, read_text_file, relative_path_label};
use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PreviewKind {
    Markdown,
    Text,
    Directory,
    Missing,
}

pub struct MarkdownPreview {
    pub path: std::path::PathBuf,
    pub content: String,
    pub scroll: u16,
}

pub struct FilePreview {
    pub title: String,
    pub content: String,
    pub kind: PreviewKind,
    pub scroll: u16,
}

impl FilePreview {
    pub fn empty() -> Self {
        Self {
            title: "preview".into(),
            content: "No file selected".into(),
            kind: PreviewKind::Missing,
            scroll: 0,
        }
    }

    pub fn from_path(cwd: &Path, path: &Path) -> Self {
        let title = relative_path_label(cwd, path);
        if path.is_dir() {
            return Self {
                title,
                content: "Directory selected".into(),
                kind: PreviewKind::Directory,
                scroll: 0,
            };
        }
        if !path.is_file() {
            return Self {
                title,
                content: "File is missing".into(),
                kind: PreviewKind::Missing,
                scroll: 0,
            };
        }
        let kind = if is_markdown_file(path) {
            PreviewKind::Markdown
        } else {
            PreviewKind::Text
        };
        Self {
            title,
            content: read_text_file(path),
            kind,
            scroll: 0,
        }
    }
}
