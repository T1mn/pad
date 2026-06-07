use std::path::Path;

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

        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();

        if name.ends_with(".md") || name.ends_with(".markdown") {
            PreviewType::Markdown
        } else if name.ends_with(".png")
            || name.ends_with(".jpg")
            || name.ends_with(".jpeg")
            || name.ends_with(".gif")
            || name.ends_with(".bmp")
            || name.ends_with(".webp")
        {
            PreviewType::Image
        } else if name.ends_with(".exe")
            || name.ends_with(".dll")
            || name.ends_with(".so")
            || name.ends_with(".dylib")
            || name.ends_with(".bin")
        {
            PreviewType::Binary
        } else if name.ends_with(".rs")
            || name.ends_with(".py")
            || name.ends_with(".js")
            || name.ends_with(".ts")
            || name.ends_with(".go")
            || name.ends_with(".java")
            || name.ends_with(".c")
            || name.ends_with(".cpp")
            || name.ends_with(".h")
            || name.ends_with(".hpp")
            || name.ends_with(".rb")
            || name.ends_with(".php")
            || name.ends_with(".swift")
            || name.ends_with(".kt")
            || name.ends_with(".scala")
            || name.ends_with(".r")
            || name.ends_with(".sh")
            || name.ends_with(".bash")
            || name.ends_with(".zsh")
            || name.ends_with(".fish")
            || name.ends_with(".json")
            || name.ends_with(".toml")
            || name.ends_with(".yaml")
            || name.ends_with(".yml")
            || name.ends_with(".xml")
            || name.ends_with(".html")
            || name.ends_with(".css")
            || name.ends_with(".sql")
            || name.ends_with(".txt")
            || name.ends_with(".log")
            || name.ends_with(".conf")
            || name.ends_with(".config")
            || name.ends_with(".ini")
            || name.ends_with(".env")
            || name.ends_with(".gitignore")
            || name.ends_with(".dockerignore")
        {
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
