pub(in crate::app::actions) fn normalize_pr_number(text: &str) -> Result<String, &'static str> {
    let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
    let Some(first) = lines.next() else {
        return Err("Clipboard is empty");
    };
    if lines.next().is_some() {
        return Err("Clipboard must contain one PR number or URL");
    }
    let value = trim_wrapping_quotes(first);
    let candidate = value
        .strip_prefix('#')
        .or_else(|| number_after_pull_segment(value))
        .unwrap_or(value);
    if is_positive_integer(candidate) {
        Ok(candidate.to_string())
    } else {
        Err("Clipboard must contain a GitHub PR number or /pull/<number> URL")
    }
}

fn number_after_pull_segment(value: &str) -> Option<&str> {
    let marker = "/pull/";
    let start = value.find(marker)? + marker.len();
    let tail = &value[start..];
    let len = tail
        .char_indices()
        .find_map(|(idx, ch)| (!ch.is_ascii_digit()).then_some(idx))
        .unwrap_or(tail.len());
    (len > 0).then_some(&tail[..len])
}

fn is_positive_integer(value: &str) -> bool {
    !value.is_empty() && value != "0" && value.chars().all(|ch| ch.is_ascii_digit())
}

fn trim_wrapping_quotes(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(value)
}
