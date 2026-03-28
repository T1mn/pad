use std::path::Path;

pub fn best_thread_title(primary: Option<&str>, fallback: Option<&str>) -> String {
    [primary, fallback]
        .into_iter()
        .flatten()
        .find_map(clean_title)
        .unwrap_or_else(|| "untitled".to_string())
}

pub fn clean_title(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let single_line = trimmed.lines().next().unwrap_or(trimmed).trim();
    if single_line.is_empty() {
        None
    } else {
        Some(single_line.to_string())
    }
}

pub fn folder_display_label(path: &str) -> String {
    let path = Path::new(path);
    let leaf = path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or(path.to_string_lossy().as_ref())
        .to_string();
    let parent = path
        .parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(|name| name.to_string());

    match parent {
        Some(parent) => format!("{} · {}", leaf, parent),
        None => leaf,
    }
}
