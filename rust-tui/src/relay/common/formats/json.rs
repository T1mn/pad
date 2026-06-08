use serde_json::json;
use std::path::Path;

pub(in crate::relay) fn read_json_value(
    path: &Path,
    fallback: serde_json::Value,
) -> serde_json::Value {
    let Ok(content) = std::fs::read_to_string(path) else {
        return fallback;
    };
    let parsed = serde_json::from_str::<serde_json::Value>(&strip_json_comments(&content))
        .unwrap_or(fallback);
    if parsed.is_object() {
        parsed
    } else {
        json!({})
    }
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

pub(in crate::relay) fn serialize_json_pretty(value: &serde_json::Value) -> String {
    let mut serialized = serde_json::to_string_pretty(value).unwrap_or_default();
    if !serialized.ends_with('\n') {
        serialized.push('\n');
    }
    serialized
}

fn strip_json_comments(content: &str) -> String {
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
                    for next in chars.by_ref() {
                        if previous == '*' && next == '/' {
                            break;
                        }
                        previous = next;
                    }
                    continue;
                }
                _ => {}
            }
        }

        out.push(ch);
    }

    out
}
