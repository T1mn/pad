pub(super) fn normalize_image_refs(
    trimmed: &str,
    image_count_hint: Option<usize>,
) -> Option<String> {
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
        return None;
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
        return Some(trimmed.to_string());
    }

    Some(if body.is_empty() {
        format!("[Image x{}]", image_count)
    } else {
        format!("[Image x{}] {}", image_count, body)
    })
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
