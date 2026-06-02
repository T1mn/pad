use super::syntax;
use ratatui::style::Color;

pub(super) fn icon(label: &str, is_dir: bool) -> &'static str {
    if is_dir {
        return "dir";
    }
    let ext = label.rsplit_once('.').map(|(_, ext)| ext).unwrap_or("");
    match ext {
        "rs" => "rs",
        "ts" | "tsx" => "ts",
        "js" | "jsx" | "mjs" | "cjs" => "js",
        "py" => "py",
        "go" => "go",
        "java" => "jv",
        "md" => "md",
        "json" | "toml" | "yaml" | "yml" => "{}",
        "html" | "css" | "scss" => "<>",
        _ => "--",
    }
}

pub(super) fn accent(label: &str, is_dir: bool) -> Color {
    syntax::accent_for_title(label, is_dir)
}
