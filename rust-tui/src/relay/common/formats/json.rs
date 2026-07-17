use serde_json::json;
use std::path::Path;

pub(in crate::relay) fn read_json_value(
    path: &Path,
    fallback: serde_json::Value,
) -> serde_json::Value {
    let Ok(content) = std::fs::read_to_string(path) else {
        return fallback;
    };
    let parsed = strip_json_comments(&content)
        .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
        .unwrap_or(fallback);
    if parsed.is_object() {
        parsed
    } else {
        json!({})
    }
}

pub(in crate::relay) fn read_json_object_for_update(
    path: &Path,
    fallback: serde_json::Value,
) -> Option<serde_json::Value> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Some(fallback),
        Err(_) => return None,
    };

    parse_json_object_strict(&strip_json_comments(&content)?)
}

pub(in crate::relay) fn write_json_value(path: &Path, value: &serde_json::Value) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let Ok(mut content) = serde_json::to_string_pretty(value) else {
        return;
    };
    if !content.ends_with('\n') {
        content.push('\n');
    }
    let _ = std::fs::write(path, content);
}

pub(in crate::relay) fn parse_json_object(content: &str) -> serde_json::Value {
    let mut obj = serde_json::from_str::<serde_json::Value>(content).unwrap_or_else(|_| json!({}));
    if !obj.is_object() {
        obj = json!({});
    }
    obj
}

pub(in crate::relay) fn parse_json_object_strict(content: &str) -> Option<serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(content)
        .ok()
        .filter(serde_json::Value::is_object)
}

pub(in crate::relay) fn serialize_json_pretty(value: &serde_json::Value) -> String {
    let mut serialized = serde_json::to_string_pretty(value).unwrap_or_default();
    if !serialized.ends_with('\n') {
        serialized.push('\n');
    }
    serialized
}

fn strip_json_comments(content: &str) -> Option<String> {
    let mut out = String::with_capacity(content.len());
    let mut chars = content.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;

    while let Some(ch) = chars.next() {
        if in_string {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if ch == '"' {
            in_string = true;
            out.push(ch);
            continue;
        }

        if ch == '/' {
            match chars.peek().copied() {
                Some('/') => {
                    chars.next();
                    for next in chars.by_ref() {
                        if next == '\n' {
                            out.push('\n');
                            break;
                        }
                    }
                    continue;
                }
                Some('*') => {
                    chars.next();
                    let mut previous = '\0';
                    let mut closed = false;
                    for next in chars.by_ref() {
                        if previous == '*' && next == '/' {
                            closed = true;
                            break;
                        }
                        previous = next;
                    }
                    if !closed {
                        return None;
                    }
                    out.push(' ');
                    continue;
                }
                _ => {}
            }
        }

        out.push(ch);
    }

    Some(out)
}
