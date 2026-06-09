use std::path::Path;

use super::super::helpers::trim_wrapping_quotes;

pub(in crate::app::actions) fn normalize_import_source(text: &str) -> Result<String, &'static str> {
    let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
    let Some(first) = lines.next() else {
        return Err("Clipboard is empty");
    };
    if lines.next().is_some() {
        return Err("Clipboard must contain one JSON path or OpenCode share URL");
    }

    let source = trim_wrapping_quotes(first);
    if is_opencode_share_url(source) || is_json_path(source) {
        Ok(source.to_string())
    } else {
        Err("Clipboard must contain a JSON path or OpenCode share URL")
    }
}

fn is_opencode_share_url(value: &str) -> bool {
    value.starts_with("https://") && value.contains("/s/")
}

fn is_json_path(value: &str) -> bool {
    value.ends_with(".json") || value.ends_with(".sanitized.json") || Path::new(value).exists()
}
