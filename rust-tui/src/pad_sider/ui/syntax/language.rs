use super::styles::{FG, TYPE, WARNING};
use ratatui::style::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CodeLanguage {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Java,
    Kotlin,
    Swift,
    C,
    Cpp,
    CSharp,
    Ruby,
    Php,
    Shell,
    Json,
    Toml,
    Yaml,
    Html,
    Css,
    Sql,
    Lua,
    Dart,
    Scala,
}

pub(in crate::pad_sider::ui::syntax) fn language_for_title(title: &str) -> Option<CodeLanguage> {
    let file_name = title
        .rsplit('/')
        .next()
        .unwrap_or(title)
        .to_ascii_lowercase();
    match file_name.as_str() {
        "dockerfile" | "makefile" | "justfile" | "bashrc" | "zshrc" => {
            return Some(CodeLanguage::Shell);
        }
        "cargo.toml" => return Some(CodeLanguage::Toml),
        "package.json" | "tsconfig.json" | "deno.json" => return Some(CodeLanguage::Json),
        _ => {}
    }

    let ext = file_name.rsplit_once('.').map(|(_, ext)| ext)?;
    match ext {
        "rs" => Some(CodeLanguage::Rust),
        "ts" | "tsx" | "mts" | "cts" => Some(CodeLanguage::TypeScript),
        "js" | "jsx" | "mjs" | "cjs" => Some(CodeLanguage::JavaScript),
        "py" | "pyw" => Some(CodeLanguage::Python),
        "go" => Some(CodeLanguage::Go),
        "java" => Some(CodeLanguage::Java),
        "kt" | "kts" => Some(CodeLanguage::Kotlin),
        "swift" => Some(CodeLanguage::Swift),
        "c" | "h" => Some(CodeLanguage::C),
        "cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" => Some(CodeLanguage::Cpp),
        "cs" => Some(CodeLanguage::CSharp),
        "rb" => Some(CodeLanguage::Ruby),
        "php" => Some(CodeLanguage::Php),
        "sh" | "bash" | "zsh" | "fish" => Some(CodeLanguage::Shell),
        "json" | "jsonc" => Some(CodeLanguage::Json),
        "toml" => Some(CodeLanguage::Toml),
        "yaml" | "yml" => Some(CodeLanguage::Yaml),
        "html" | "htm" | "xml" | "svg" => Some(CodeLanguage::Html),
        "css" | "scss" | "sass" | "less" => Some(CodeLanguage::Css),
        "sql" => Some(CodeLanguage::Sql),
        "lua" => Some(CodeLanguage::Lua),
        "dart" => Some(CodeLanguage::Dart),
        "scala" | "sc" => Some(CodeLanguage::Scala),
        _ => None,
    }
}

pub(in crate::pad_sider::ui) fn language_label_for_title(title: &str) -> Option<&'static str> {
    language_for_title(title).map(|language| match language {
        CodeLanguage::Rust => "Rust",
        CodeLanguage::TypeScript => "TypeScript",
        CodeLanguage::JavaScript => "JavaScript",
        CodeLanguage::Python => "Python",
        CodeLanguage::Go => "Go",
        CodeLanguage::Java => "Java",
        CodeLanguage::Kotlin => "Kotlin",
        CodeLanguage::Swift => "Swift",
        CodeLanguage::C => "C",
        CodeLanguage::Cpp => "C++",
        CodeLanguage::CSharp => "C#",
        CodeLanguage::Ruby => "Ruby",
        CodeLanguage::Php => "PHP",
        CodeLanguage::Shell => "Shell",
        CodeLanguage::Json => "JSON",
        CodeLanguage::Toml => "TOML",
        CodeLanguage::Yaml => "YAML",
        CodeLanguage::Html => "HTML",
        CodeLanguage::Css => "CSS",
        CodeLanguage::Sql => "SQL",
        CodeLanguage::Lua => "Lua",
        CodeLanguage::Dart => "Dart",
        CodeLanguage::Scala => "Scala",
    })
}

pub(in crate::pad_sider::ui) fn accent_for_title(title: &str, is_dir: bool) -> Color {
    if is_dir {
        return Color::Rgb(86, 156, 214);
    }
    language_for_title(title)
        .map(accent_for_language)
        .unwrap_or(FG)
}

fn accent_for_language(language: CodeLanguage) -> Color {
    match language {
        CodeLanguage::Rust => Color::Rgb(222, 118, 34),
        CodeLanguage::TypeScript => Color::Rgb(49, 120, 198),
        CodeLanguage::JavaScript => Color::Rgb(247, 223, 30),
        CodeLanguage::Python => Color::Rgb(255, 212, 59),
        CodeLanguage::Go => Color::Rgb(0, 173, 216),
        CodeLanguage::Java => Color::Rgb(234, 45, 46),
        CodeLanguage::Kotlin => Color::Rgb(169, 123, 255),
        CodeLanguage::Swift => Color::Rgb(255, 112, 67),
        CodeLanguage::C | CodeLanguage::Cpp | CodeLanguage::CSharp => TYPE,
        CodeLanguage::Ruby => Color::Rgb(204, 52, 45),
        CodeLanguage::Php => Color::Rgb(119, 123, 180),
        CodeLanguage::Shell => Color::Rgb(137, 221, 121),
        CodeLanguage::Json | CodeLanguage::Toml | CodeLanguage::Yaml => WARNING,
        CodeLanguage::Html => Color::Rgb(227, 76, 38),
        CodeLanguage::Css => Color::Rgb(38, 77, 228),
        CodeLanguage::Sql => Color::Rgb(181, 206, 168),
        CodeLanguage::Lua => Color::Rgb(0, 0, 128),
        CodeLanguage::Dart => Color::Rgb(0, 180, 255),
        CodeLanguage::Scala => Color::Rgb(220, 30, 35),
    }
}

pub(in crate::pad_sider::ui::syntax) fn comment_prefixes(
    language: CodeLanguage,
) -> &'static [&'static str] {
    match language {
        CodeLanguage::Python
        | CodeLanguage::Shell
        | CodeLanguage::Ruby
        | CodeLanguage::Yaml
        | CodeLanguage::Toml => &["#"],
        CodeLanguage::Sql => &["--"],
        CodeLanguage::Html => &["<!--"],
        _ => &["//", "/*"],
    }
}
