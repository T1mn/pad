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
    let file_name = title.rsplit('/').next().unwrap_or(title);
    language_for_exact_file_name(file_name).or_else(|| language_for_extension(file_name))
}

fn language_for_exact_file_name(file_name: &str) -> Option<CodeLanguage> {
    const EXACT_NAMES: &[(&[&str], CodeLanguage)] = &[
        (
            &["dockerfile", "makefile", "justfile", "bashrc", "zshrc"],
            CodeLanguage::Shell,
        ),
        (&["cargo.toml"], CodeLanguage::Toml),
        (
            &["package.json", "tsconfig.json", "deno.json"],
            CodeLanguage::Json,
        ),
    ];
    language_from_table(file_name, EXACT_NAMES)
}

fn language_for_extension(file_name: &str) -> Option<CodeLanguage> {
    let ext = file_name.rsplit_once('.').map(|(_, ext)| ext)?;
    const EXTENSIONS: &[(&[&str], CodeLanguage)] = &[
        (&["rs"], CodeLanguage::Rust),
        (&["ts", "tsx", "mts", "cts"], CodeLanguage::TypeScript),
        (&["js", "jsx", "mjs", "cjs"], CodeLanguage::JavaScript),
        (&["py", "pyw"], CodeLanguage::Python),
        (&["go"], CodeLanguage::Go),
        (&["java"], CodeLanguage::Java),
        (&["kt", "kts"], CodeLanguage::Kotlin),
        (&["swift"], CodeLanguage::Swift),
        (&["c", "h"], CodeLanguage::C),
        (&["cc", "cpp", "cxx", "hpp", "hh", "hxx"], CodeLanguage::Cpp),
        (&["cs"], CodeLanguage::CSharp),
        (&["rb"], CodeLanguage::Ruby),
        (&["php"], CodeLanguage::Php),
        (&["sh", "bash", "zsh", "fish"], CodeLanguage::Shell),
        (&["json", "jsonc"], CodeLanguage::Json),
        (&["toml"], CodeLanguage::Toml),
        (&["yaml", "yml"], CodeLanguage::Yaml),
        (&["html", "htm", "xml", "svg"], CodeLanguage::Html),
        (&["css", "scss", "sass", "less"], CodeLanguage::Css),
        (&["sql"], CodeLanguage::Sql),
        (&["lua"], CodeLanguage::Lua),
        (&["dart"], CodeLanguage::Dart),
        (&["scala", "sc"], CodeLanguage::Scala),
    ];
    language_from_table(ext, EXTENSIONS)
}

fn language_from_table(value: &str, table: &[(&[&str], CodeLanguage)]) -> Option<CodeLanguage> {
    table.iter().find_map(|(values, language)| {
        matches_any_ignore_ascii_case(value, values).then_some(*language)
    })
}

fn matches_any_ignore_ascii_case(value: &str, candidates: &[&str]) -> bool {
    candidates
        .iter()
        .any(|candidate| value.eq_ignore_ascii_case(candidate))
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
