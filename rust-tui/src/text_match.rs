pub(crate) fn contains_ascii_ignore_case(value: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }

    value
        .as_bytes()
        .windows(needle.len())
        .any(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
}

pub(crate) fn contains_ignore_case(value: &str, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }

    if value.is_ascii() && query.is_ascii() {
        return contains_ascii_ignore_case(value, query);
    }

    let query = query.to_lowercase();
    value.to_lowercase().contains(&query)
}

#[cfg(test)]
#[path = "text_match_tests.rs"]
mod tests;
