use super::fs::{is_markdown_file, read_text_file, relative_path_label};
use super::preview::{FilePreview, PreviewKind};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Clone, Copy, PartialEq, Eq)]
struct FileSignature {
    len: u64,
    modified: Option<SystemTime>,
}

#[derive(Clone)]
struct CachedFilePreview {
    signature: FileSignature,
    preview: FilePreview,
}

#[derive(Default)]
pub struct FilePreviewCache {
    entries: HashMap<PathBuf, CachedFilePreview>,
}

impl FilePreviewCache {
    pub fn preview_for(&mut self, cwd: &Path, path: &Path) -> FilePreview {
        let started_at = std::time::Instant::now();
        let preview = self.preview_for_inner(cwd, path);
        let elapsed = started_at.elapsed();
        if elapsed >= std::time::Duration::from_millis(8) {
            crate::log_debug!(
                "pad_sider.preview: load_slow path={} elapsed_ms={} bytes={}",
                path.display(),
                elapsed.as_millis(),
                preview.content.len()
            );
        }
        preview
    }

    fn preview_for_inner(&mut self, cwd: &Path, path: &Path) -> FilePreview {
        let title = relative_path_label(cwd, path);
        if path.is_dir() {
            return FilePreview::new(title, "Directory selected".into(), PreviewKind::Directory);
        }
        if !path.is_file() {
            self.entries.remove(path);
            return FilePreview::new(title, "File is missing".into(), PreviewKind::Missing);
        }

        let signature = file_signature(path);
        if let Some(cached) = self.entries.get(path) {
            if cached.signature == signature && cached.preview.title == title {
                return cached.preview.clone();
            }
        }

        let kind = if is_markdown_file(path) {
            PreviewKind::Markdown
        } else {
            PreviewKind::Text
        };
        let preview = FilePreview::new(title, read_text_file(path), kind);
        self.entries.insert(
            path.to_path_buf(),
            CachedFilePreview {
                signature,
                preview: preview.clone(),
            },
        );
        preview
    }
}

fn file_signature(path: &Path) -> FileSignature {
    let metadata = std::fs::metadata(path).ok();
    FileSignature {
        len: metadata.as_ref().map(|value| value.len()).unwrap_or(0),
        modified: metadata.and_then(|value| value.modified().ok()),
    }
}
