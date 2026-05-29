pub(crate) fn set_toml_bool_in_section(
    content: &str,
    section: &str,
    key: &str,
    value: bool,
) -> String {
    let target_header = format!("[{}]", section);
    let key_prefix = format!("{} =", key);
    let new_line = format!("{} = {}", key, value);

    let mut lines: Vec<String> = Vec::new();
    let mut in_target = false;
    let mut section_found = false;
    let mut key_written = false;

    for line in content.lines() {
        let trimmed = line.trim();
        let is_section = trimmed.starts_with('[') && trimmed.ends_with(']');

        if is_section && in_target && !key_written {
            lines.push(new_line.clone());
            key_written = true;
        }

        if trimmed == target_header {
            section_found = true;
            in_target = true;
            lines.push(line.to_string());
            continue;
        }

        if is_section {
            in_target = false;
        }

        if in_target && trimmed.starts_with(&key_prefix) {
            lines.push(new_line.clone());
            key_written = true;
        } else {
            lines.push(line.to_string());
        }
    }

    if section_found {
        if !key_written {
            if !lines.is_empty() && !lines.last().is_some_and(|line| line.is_empty()) {
                lines.push(String::new());
            }
            lines.push(new_line);
        }
    } else {
        if !lines.is_empty() && !lines.last().is_some_and(|line| line.is_empty()) {
            lines.push(String::new());
        }
        lines.push(target_header);
        lines.push(new_line);
    }

    let mut result = lines.join("\n");
    if !result.ends_with('\n') {
        result.push('\n');
    }
    result
}

pub(crate) fn remove_toml_key_in_section(content: &str, section: &str, key: &str) -> String {
    let target_header = format!("[{}]", section);
    let key_prefix = format!("{} =", key);

    let mut lines: Vec<String> = Vec::new();
    let mut in_target = false;

    for line in content.lines() {
        let trimmed = line.trim();
        let is_section = trimmed.starts_with('[') && trimmed.ends_with(']');

        if trimmed == target_header {
            in_target = true;
            lines.push(line.to_string());
            continue;
        }

        if is_section {
            in_target = false;
        }

        if in_target && trimmed.starts_with(&key_prefix) {
            continue;
        }

        lines.push(line.to_string());
    }

    let mut result = lines.join("\n");
    if !result.ends_with('\n') {
        result.push('\n');
    }
    result
}
