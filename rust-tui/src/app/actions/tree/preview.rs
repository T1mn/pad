use crate::app::App;
use crate::tree::PreviewType;
use std::path::PathBuf;

impl App {
    pub fn update_file_preview(&mut self) {
        if let Some(ref tree) = self.sidebar.file_tree {
            if let Some(entry) = tree.selected() {
                let path = &entry.path;
                let preview_type = PreviewType::from_path(path);

                if preview_type.is_text() {
                    self.preview.file_preview_path = Some(path.clone());
                    self.preview.file_preview_content = Self::load_text_file(path, 500);
                    self.preview.file_preview_scroll = 0;
                } else if preview_type.is_image() {
                    self.preview.file_preview_path = Some(path.clone());
                    self.preview.file_preview_content = format!(
                        "🖼️  Image file: {}\n\n(Use terminal image viewer like 'icat' to preview images)",
                        path.display()
                    );
                } else if preview_type == PreviewType::Directory {
                    self.preview.file_preview_path = Some(path.clone());
                    self.preview.file_preview_content = Self::load_directory_info(path);
                } else {
                    self.preview.file_preview_path = Some(path.clone());
                    self.preview.file_preview_content = format!(
                        "📦 Binary file: {}\n\nSize: {}\nType: {:?}",
                        path.display(),
                        Self::format_file_size(path),
                        preview_type
                    );
                }
            } else {
                self.preview.file_preview_path = None;
                self.preview.file_preview_content = "No file selected".to_string();
            }
        }
        self.dirty = true;
    }

    pub fn load_text_file(path: &PathBuf, max_lines: usize) -> String {
        use std::io::BufRead;

        let file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(e) => return format!("Error opening file: {}", e),
        };

        let reader = std::io::BufReader::new(file);
        let mut content = String::new();
        let mut line_count = 0;

        for line in reader.lines() {
            if line_count >= max_lines {
                content.push_str("\n... (truncated)");
                break;
            }
            match line {
                Ok(l) => {
                    content.push_str(&l);
                    content.push('\n');
                    line_count += 1;
                }
                Err(e) => {
                    content.push_str(&format!("\n[Error reading line: {}]", e));
                    break;
                }
            }
        }

        content
    }

    pub fn load_directory_info(path: &PathBuf) -> String {
        let mut content = format!("📁 Directory: {}\n\n", path.display());

        if let Ok(entries) = std::fs::read_dir(path) {
            let mut count = 0;
            for entry in entries.flatten() {
                if count >= 50 {
                    content.push_str("\n... (more items)");
                    break;
                }
                let name = entry.file_name().to_string_lossy().to_string();
                let metadata = entry.metadata();
                let icon = if metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false) {
                    "📁"
                } else {
                    "📄"
                };
                content.push_str(&format!("{} {}\n", icon, name));
                count += 1;
            }
            if count == 0 {
                content.push_str("(empty directory)");
            }
        } else {
            content.push_str("(cannot read directory)");
        }

        content
    }

    pub fn format_file_size(path: &PathBuf) -> String {
        match std::fs::metadata(path) {
            Ok(metadata) => {
                let size = metadata.len();
                if size < 1024 {
                    format!("{} B", size)
                } else if size < 1024 * 1024 {
                    format!("{:.1} KB", size as f64 / 1024.0)
                } else if size < 1024 * 1024 * 1024 {
                    format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
                } else {
                    format!("{:.1} GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
                }
            }
            Err(_) => "Unknown".to_string(),
        }
    }
}
