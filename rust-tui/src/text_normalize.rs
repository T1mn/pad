pub(crate) fn collapse_whitespace(text: &str) -> String {
    let mut collapsed = String::new();
    for part in text.split_whitespace() {
        if !collapsed.is_empty() {
            collapsed.push(' ');
        }
        collapsed.push_str(part);
    }
    collapsed
}

#[cfg(test)]
#[path = "text_normalize_tests.rs"]
mod tests;
