pub(super) fn split_first_line(content: &str) -> (&str, &str, &str) {
    if let Some(index) = content.find("\r\n") {
        let rest = &content[index + 2..];
        return (&content[..index], "\r\n", rest);
    }
    if let Some(index) = content.find('\n') {
        let rest = &content[index + 1..];
        return (&content[..index], "\n", rest);
    }
    (content, "", "")
}
