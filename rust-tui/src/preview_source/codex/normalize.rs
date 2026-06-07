const ENVIRONMENT_CONTEXT_OPEN: &str = "<environment_context>";
const ENVIRONMENT_CONTEXT_CLOSE: &str = "</environment_context>";
const TURN_ABORTED_OPEN: &str = "<turn_aborted>";
const TURN_ABORTED_CLOSE: &str = "</turn_aborted>";
const USER_SHELL_COMMAND_OPEN: &str = "<user_shell_command>";
const USER_SHELL_COMMAND_CLOSE: &str = "</user_shell_command>";
const SKILL_OPEN: &str = "<skill>";
const SKILL_CLOSE: &str = "</skill>";
const AGENTS_MD_INSTRUCTIONS_PREFIX: &str = "# AGENTS.md instructions for ";
const AGENTS_MD_INSTRUCTIONS_SUFFIX: &str = "</INSTRUCTIONS>";

pub(crate) fn normalize_codex_user_text(text: &str, image_count_hint: Option<usize>) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    if let Some(summary) = extract_user_shell_command_summary(trimmed) {
        return summary;
    }

    let stripped_context = strip_non_preview_codex_fragments(trimmed);
    let trimmed = stripped_context.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let contains_image_wrappers =
        trimmed.contains("<image name=[Image #") || trimmed.contains("</image>");
    let starts_with_image_ref = trimmed.starts_with("[Image #");
    let image_count = image_count_hint
        .filter(|count| *count > 0)
        .or_else(|| {
            let open_tag_count = count_image_open_tags(trimmed);
            (open_tag_count > 0).then_some(open_tag_count)
        })
        .or_else(|| starts_with_image_ref.then(|| count_image_refs(trimmed)))
        .unwrap_or(0);

    if image_count == 0 && !contains_image_wrappers && !starts_with_image_ref {
        return trimmed.to_string();
    }

    let without_wrappers = trimmed
        .lines()
        .filter(|line| !is_image_wrapper_line(line.trim()))
        .collect::<Vec<_>>()
        .join("\n");
    let stripped = strip_all_image_refs(&without_wrappers);
    let body = stripped
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    if image_count == 0 {
        return trimmed.to_string();
    }

    if body.is_empty() {
        format!("[Image x{}]", image_count)
    } else {
        format!("[Image x{}] {}", image_count, body)
    }
}

fn strip_non_preview_codex_fragments(text: &str) -> String {
    let mut stripped = text.to_string();
    for (open, close) in [
        (ENVIRONMENT_CONTEXT_OPEN, ENVIRONMENT_CONTEXT_CLOSE),
        (TURN_ABORTED_OPEN, TURN_ABORTED_CLOSE),
        (USER_SHELL_COMMAND_OPEN, USER_SHELL_COMMAND_CLOSE),
        (SKILL_OPEN, SKILL_CLOSE),
        (AGENTS_MD_INSTRUCTIONS_PREFIX, AGENTS_MD_INSTRUCTIONS_SUFFIX),
    ] {
        stripped = strip_wrapped_block(&stripped, open, close);
    }
    stripped
}

fn strip_wrapped_block(text: &str, open: &str, close: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;

    while let Some(start) = rest.find(open) {
        out.push_str(&rest[..start]);
        let after_open = &rest[start + open.len()..];
        let Some(end) = after_open.find(close) else {
            out.push_str(&rest[start..]);
            return out;
        };
        rest = &after_open[end + close.len()..];
    }

    out.push_str(rest);
    out
}

fn extract_user_shell_command_summary(text: &str) -> Option<String> {
    let inner = exact_wrapped_fragment(
        text.trim(),
        USER_SHELL_COMMAND_OPEN,
        USER_SHELL_COMMAND_CLOSE,
    )?;
    let command = find_wrapped_fragment(inner.trim(), "<command>", "</command>")
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    Some(format!("[shell] {}", command))
}

fn exact_wrapped_fragment<'a>(text: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let trimmed = text.trim();
    let inner = trimmed.strip_prefix(open)?.strip_suffix(close)?;
    Some(inner)
}

fn find_wrapped_fragment<'a>(text: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let start = text.find(open)?;
    let after_open = &text[start + open.len()..];
    let end = after_open.find(close)?;
    Some(&after_open[..end])
}

fn count_image_open_tags(text: &str) -> usize {
    text.match_indices("<image name=[Image #").count()
}

fn count_image_refs(text: &str) -> usize {
    let mut count = 0usize;
    let mut rest = text;
    while let Some(start) = rest.find("[Image #") {
        let candidate = &rest[start..];
        let Some(end) = candidate.find(']') else {
            break;
        };
        if is_image_ref_token(&candidate[..=end]) {
            count += 1;
            rest = &candidate[end + 1..];
        } else {
            rest = &candidate["[Image #".len()..];
        }
    }
    count
}

fn strip_all_image_refs(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;

    while let Some(start) = rest.find("[Image #") {
        out.push_str(&rest[..start]);
        let candidate = &rest[start..];
        let Some(end) = candidate.find(']') else {
            out.push_str(candidate);
            return out;
        };
        let token = &candidate[..=end];
        if is_image_ref_token(token) {
            rest = &candidate[end + 1..];
        } else {
            out.push_str("[Image #");
            rest = &candidate["[Image #".len()..];
        }
    }

    out.push_str(rest);
    out
}

fn is_image_wrapper_line(line: &str) -> bool {
    line == "</image>" || is_image_open_tag(line)
}

fn is_image_open_tag(text: &str) -> bool {
    let Some(inner) = text
        .strip_prefix("<image name=[Image #")
        .and_then(|value| value.strip_suffix("]>"))
    else {
        return false;
    };

    !inner.is_empty() && inner.chars().all(|ch| ch.is_ascii_digit())
}

fn is_image_ref_token(text: &str) -> bool {
    let Some(inner) = text
        .strip_prefix("[Image #")
        .and_then(|value| value.strip_suffix(']'))
    else {
        return false;
    };

    !inner.is_empty() && inner.chars().all(|ch| ch.is_ascii_digit())
}
