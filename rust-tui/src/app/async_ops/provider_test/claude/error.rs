pub(super) fn classify_error(status: u16, body: &str) -> &'static str {
    let lower = body.to_ascii_lowercase();
    if status == 401 || status == 403 {
        "auth"
    } else if status == 404 {
        "not_found"
    } else if status == 408 || lower.contains("timeout") {
        "timeout"
    } else if status == 429 {
        "rate_limit"
    } else if lower.contains("model")
        && (lower.contains("not") || lower.contains("invalid") || lower.contains("unsupported"))
    {
        "model"
    } else if status >= 500 {
        "server_error"
    } else {
        "http_error"
    }
}

pub(super) fn truncate_message(input: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for ch in input.trim().chars().take(max_chars) {
        if ch.is_control() {
            out.push(' ');
        } else {
            out.push(ch);
        }
    }
    out
}
