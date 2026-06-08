pub(in crate::app::actions) fn normalize_server_url(text: &str) -> Result<String, &'static str> {
    let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
    let Some(first) = lines.next() else {
        return Err("Clipboard is empty");
    };
    if lines.next().is_some() {
        return Err("Clipboard must contain one OpenCode server URL");
    }
    let url = trim_wrapping_quotes(first).trim_end_matches('/');
    if is_http_url(url) && !url.contains(char::is_whitespace) {
        Ok(url.to_string())
    } else {
        Err("Clipboard must contain an http(s) OpenCode server URL")
    }
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

fn is_http_url(value: &str) -> bool {
    let rest = value
        .strip_prefix("http://")
        .or_else(|| value.strip_prefix("https://"));
    rest.is_some_and(|rest| !rest.is_empty() && !rest.starts_with('/'))
}
