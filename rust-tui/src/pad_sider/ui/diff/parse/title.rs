pub(super) fn diff_title(line: &str) -> String {
    let mut parts = line.split_whitespace().skip(2);
    let left = parts.next().unwrap_or("");
    let right = parts.next().unwrap_or(left);
    let right = right.strip_prefix("b/").unwrap_or(right);
    let left = left.strip_prefix("a/").unwrap_or(left);
    if right == "/dev/null" || right.is_empty() {
        left.to_string()
    } else {
        right.to_string()
    }
}
