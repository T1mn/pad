use std::path::Path;

const MARKDOWN_SUFFIXES: &[&str] = &[".md", ".markdown"];
const IMAGE_SUFFIXES: &[&str] = &[".png", ".jpg", ".jpeg", ".gif", ".bmp", ".webp"];
const BINARY_SUFFIXES: &[&str] = &[".exe", ".dll", ".so", ".dylib", ".bin"];
const TEXT_SUFFIXES: &[&str] = &[
    ".rs",
    ".py",
    ".js",
    ".ts",
    ".go",
    ".java",
    ".c",
    ".cpp",
    ".h",
    ".hpp",
    ".rb",
    ".php",
    ".swift",
    ".kt",
    ".scala",
    ".r",
    ".sh",
    ".bash",
    ".zsh",
    ".fish",
    ".json",
    ".toml",
    ".yaml",
    ".yml",
    ".xml",
    ".html",
    ".css",
    ".sql",
    ".txt",
    ".log",
    ".conf",
    ".config",
    ".ini",
    ".env",
    ".gitignore",
    ".dockerignore",
];

/// File preview type
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PreviewType {
    Text,      // Source code, config files
    Markdown,  // Markdown files
    Image,     // Image files (PNG, JPG, etc)
    Binary,    // Binary files (cannot preview)
    Directory, // Directory
    Unknown,   // Unknown type
}

impl PreviewType {
    /// Detect preview type from file path
    pub fn from_path(path: &Path) -> Self {
        if path.is_dir() {
            return PreviewType::Directory;
        }

        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if has_any_suffix(name, MARKDOWN_SUFFIXES) {
            PreviewType::Markdown
        } else if has_any_suffix(name, IMAGE_SUFFIXES) {
            PreviewType::Image
        } else if has_any_suffix(name, BINARY_SUFFIXES) {
            PreviewType::Binary
        } else if has_any_suffix(name, TEXT_SUFFIXES) {
            PreviewType::Text
        } else {
            PreviewType::Unknown
        }
    }

    /// Check if file can be previewed as text
    pub fn is_text(&self) -> bool {
        matches!(self, PreviewType::Text | PreviewType::Markdown)
    }

    /// Check if file is an image
    pub fn is_image(&self) -> bool {
        matches!(self, PreviewType::Image)
    }
}

fn has_any_suffix(name: &str, suffixes: &[&str]) -> bool {
    suffixes
        .iter()
        .any(|suffix| has_suffix_ignore_ascii_case(name, suffix))
}

fn has_suffix_ignore_ascii_case(name: &str, suffix: &str) -> bool {
    let name = name.as_bytes();
    let suffix = suffix.as_bytes();
    if name.len() < suffix.len() {
        return false;
    }
    name[name.len() - suffix.len()..].eq_ignore_ascii_case(suffix)
}
