mod actions;
mod app;
mod cli {
    use std::path::PathBuf;

    pub enum Command {
        Toggle {
            target_pane: String,
        },
        Ui {
            cwd: PathBuf,
            target_pane: Option<String>,
        },
    }

    pub fn parse<I>(mut args: I) -> Result<Command, String>
    where
        I: Iterator<Item = String>,
    {
        match args.next().as_deref() {
            Some("toggle") => parse_toggle(args),
            Some("ui") => parse_ui(args),
            Some(other) => Err(format!("unknown command: {other}")),
            None => Err("missing command: expected `toggle` or `ui`".into()),
        }
    }

    fn parse_toggle<I>(mut args: I) -> Result<Command, String>
    where
        I: Iterator<Item = String>,
    {
        let mut target_pane = None;
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--target-pane" => target_pane = args.next(),
                other => return Err(format!("unknown toggle arg: {other}")),
            }
        }
        let target_pane = target_pane.ok_or_else(|| "missing --target-pane".to_string())?;
        Ok(Command::Toggle { target_pane })
    }

    fn parse_ui<I>(mut args: I) -> Result<Command, String>
    where
        I: Iterator<Item = String>,
    {
        let mut cwd = None;
        let mut target_pane = None;
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--cwd" => cwd = args.next().map(PathBuf::from),
                "--target-pane" => target_pane = args.next(),
                other => return Err(format!("unknown ui arg: {other}")),
            }
        }
        let cwd = cwd.ok_or_else(|| "missing --cwd".to_string())?;
        Ok(Command::Ui { cwd, target_pane })
    }
}
mod codex_runs;
mod fs;
mod ignore {
    pub(super) fn skip_dir_name(name: &str) -> bool {
        matches!(
            name,
            ".git" | "node_modules" | "target" | "dist" | "build" | "__pycache__"
        )
    }
}
mod index_map;
mod preview {
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
}
mod preview_cache {
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
                return FilePreview::new(
                    title,
                    "Directory selected".into(),
                    PreviewKind::Directory,
                );
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
}
mod preview_render_cache;
mod search;
mod sizing;
mod tmux;
mod tmux_args;
mod tree;
mod ui;

#[cfg(test)]
mod tests;

pub fn run_args<I>(args: I) -> Result<(), Box<dyn std::error::Error>>
where
    I: Iterator<Item = String>,
{
    match cli::parse(args)? {
        cli::Command::Toggle { target_pane } => tmux::toggle(&target_pane).map_err(Into::into),
        cli::Command::Ui { cwd, target_pane } => ui::run(cwd, target_pane).map_err(Into::into),
    }
}
