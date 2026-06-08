pub(super) fn yaml_string(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n");
    format!("\"{escaped}\"")
}

pub(super) fn parse_yaml_string(value: &str) -> Result<String, String> {
    if value.eq_ignore_ascii_case("null") {
        return Ok(String::new());
    }
    if !(value.starts_with('"') && value.ends_with('"')) {
        return Ok(value.to_string());
    }

    let inner = &value[1..value.len().saturating_sub(1)];
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some('n') => out.push('\n'),
            Some(other) => out.push(other),
            None => return Err("invalid escape sequence".to_string()),
        }
    }
    Ok(out)
}
