pub(crate) fn set_toml_bool_in_section(
    content: &str,
    section: &str,
    key: &str,
    value: bool,
) -> String {
    let target_header = format!("[{}]", section);
    let key_prefix = format!("{} =", key);
    let new_line = format!("{} = {}", key, value);

    let mut result =
        String::with_capacity(content.len() + target_header.len() + new_line.len() + 4);
    let mut wrote_line = false;
    let mut in_target = false;
    let mut section_found = false;
    let mut key_written = false;
    let mut last_line_empty = true;

    for line in content.lines() {
        let trimmed = line.trim();
        let is_section = trimmed.starts_with('[') && trimmed.ends_with(']');

        if is_section && in_target && !key_written {
            push_toml_line(&mut result, &mut wrote_line, &new_line);
            key_written = true;
        }

        if trimmed == target_header {
            section_found = true;
            in_target = true;
            push_toml_line(&mut result, &mut wrote_line, line);
            last_line_empty = line.is_empty();
            continue;
        }

        if is_section {
            in_target = false;
        }

        if in_target && trimmed.starts_with(&key_prefix) {
            push_toml_line(&mut result, &mut wrote_line, &new_line);
            key_written = true;
            last_line_empty = false;
        } else {
            push_toml_line(&mut result, &mut wrote_line, line);
            last_line_empty = line.is_empty();
        }
    }

    if section_found {
        if !key_written {
            if wrote_line && !last_line_empty {
                push_toml_line(&mut result, &mut wrote_line, "");
            }
            push_toml_line(&mut result, &mut wrote_line, &new_line);
        }
    } else {
        if wrote_line && !last_line_empty {
            push_toml_line(&mut result, &mut wrote_line, "");
        }
        push_toml_line(&mut result, &mut wrote_line, &target_header);
        push_toml_line(&mut result, &mut wrote_line, &new_line);
    }

    finish_toml_result(result)
}

pub(crate) fn remove_toml_key_in_section(content: &str, section: &str, key: &str) -> String {
    let target_header = format!("[{}]", section);
    let key_prefix = format!("{} =", key);

    let mut result = String::with_capacity(content.len());
    let mut wrote_line = false;
    let mut in_target = false;

    for line in content.lines() {
        let trimmed = line.trim();
        let is_section = trimmed.starts_with('[') && trimmed.ends_with(']');

        if trimmed == target_header {
            in_target = true;
            push_toml_line(&mut result, &mut wrote_line, line);
            continue;
        }

        if is_section {
            in_target = false;
        }

        if in_target && trimmed.starts_with(&key_prefix) {
            continue;
        }

        push_toml_line(&mut result, &mut wrote_line, line);
    }

    finish_toml_result(result)
}

fn push_toml_line(result: &mut String, wrote_line: &mut bool, line: &str) {
    if *wrote_line {
        result.push('\n');
    }
    result.push_str(line);
    *wrote_line = true;
}

fn finish_toml_result(mut result: String) -> String {
    if !result.ends_with('\n') {
        result.push('\n');
    }
    result
}
